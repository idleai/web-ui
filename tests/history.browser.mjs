import assert from 'node:assert/strict';
import { readFile, mkdir } from 'node:fs/promises';
import { createServer } from 'node:http';
import { resolve, extname, sep } from 'node:path';
import { after, before, test } from 'node:test';
import puppeteer from 'puppeteer-core';
import { assertSmoothPaths } from './history-paths.mjs';

const root = resolve('out/history');
let server, browser, address;
before(async () => {
  assert.ok(process.env.CHROME_PATH, 'CHROME_PATH must name an installed Chrome executable');
  await readFile(resolve(root, 'pkg/history_bg.wasm'));
  server = createServer(async (request, response) => {
    const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
    const path = resolve(root, `.${pathname === '/' ? '/index.html' : pathname}`);
    if (!path.startsWith(root + sep)) { response.writeHead(403).end(); return; }
    try {
      const data = await readFile(path);
      const type = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm' }[extname(path)] || 'application/octet-stream';
      response.writeHead(200, { 'Content-Type': type }).end(data);
    } catch { response.writeHead(404).end(); }
  });
  await new Promise(done => server.listen(0, '127.0.0.1', done));
  address = `http://127.0.0.1:${server.address().port}/`;
  browser = await puppeteer.launch({ executablePath: process.env.CHROME_PATH, headless: true, args: ['--no-sandbox'] });
});
after(async () => {
  await browser?.close();
  if (server) await new Promise(done => server.close(done));
});

async function open() {
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(String(error)));
  await page.setViewport({ width: 1360, height: 1000 });
  await page.goto(address);
  await page.waitForSelector('#browser-history [role=treeitem]');
  await settle(page);
  return { page, errors };
}

async function settle(page) {
  await page.evaluate(() => new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done))));
  await page.waitForFunction(() => [...document.querySelectorAll('[role=tree]')].every(tree => {
    const rows = [...tree.querySelectorAll('[role=treeitem]')].map(row => row.getBoundingClientRect()).sort((a, b) => a.top - b.top);
    return rows.every((row, index) => !index || row.top >= rows[index - 1].bottom - 1);
  }));
}

async function anchor(page, treeId = 'browser-history') {
  return page.evaluate(id => {
    const tree = document.getElementById(id), box = tree.getBoundingClientRect();
    const rows = [...tree.querySelectorAll('[role=treeitem]')].sort((a, b) => a.getBoundingClientRect().top - b.getBoundingClientRect().top);
    const row = rows.find(row => row.getBoundingClientRect().bottom > box.top + tree.clientTop);
    return { key: row.dataset.itemKey, offset: row.getBoundingClientRect().top - box.top, top: tree.scrollTop };
  }, treeId);
}

async function clickButton(page, label) {
  await page.locator(`button::-p-text(${label})`).click();
  await settle(page);
}

test('both host compositions virtualize, retain records, and distinguish relationship types', async () => {
  const { page, errors } = await open();
  try {
    const trees = await page.evaluate(() => [...document.querySelectorAll('[role=tree]')].map(tree => ({
      rows: tree.querySelectorAll('[role=treeitem]').length,
      total: tree.querySelector('[role=treeitem]').getAttribute('aria-setsize'),
      kinds: [...new Set([...tree.querySelectorAll('path')].map(path => path.dataset.kind))],
      records: [...tree.querySelectorAll('path')].every(path => path.dataset.operation.length === 64 && path.dataset.recordHash.length === 64),
      active: !!document.getElementById(tree.getAttribute('aria-activedescendant')),
    })));
    assert.equal(trees.length, 2);
    for (const tree of trees) {
      assert.ok(tree.rows > 5 && tree.rows < 30, 'mounted rows follow viewport size');
      assert.equal(tree.total, '300');
      assert.deepEqual(tree.kinds.sort(), ['causal', 'cause', 'link']);
      assert.ok(tree.records && tree.active, 'full records and the active descendant stay mounted');
    }
    assert.deepEqual(errors, []);
    await mkdir('out/history-tests', { recursive: true });
    await page.screenshot({ path: 'out/history-tests/desktop.png', fullPage: true });
  } finally { await page.close(); }
});

test('late inserts and endpoints keep a scrolled item at the same screen position', async () => {
  const { page, errors } = await open();
  try {
    await page.$eval('#browser-history', tree => { tree.scrollTop = 7007; });
    await page.waitForFunction(() => document.querySelector('#browser-history').scrollTop > 6000);
    await settle(page);
    const before = await anchor(page);
    await clickButton(page, 'Insert late record');
    let after = await anchor(page);
    assert.equal(after.key, before.key);
    assert.ok(Math.abs(after.offset - before.offset) <= 1, 'late insertion preserves the pixel offset');
    await clickButton(page, 'Resolve endpoints');
    after = await anchor(page);
    assert.equal(after.key, before.key);
    assert.ok(Math.abs(after.offset - before.offset) <= 1, 'endpoint repair preserves the pixel offset');
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('branch tangents stay smooth through disclosure, late arrivals and narrow layout', async () => {
  const { page, errors } = await open();
  try {
    await assertSmoothPaths(page);
    await page.focus('#browser-history');
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('ArrowRight');
    await settle(page);
    await assertSmoothPaths(page);
    await clickButton(page, 'Insert late record');
    await clickButton(page, 'Resolve endpoints');
    await assertSmoothPaths(page);
    await clickButton(page, 'Toggle narrow layout');
    await settle(page);
    await assertSmoothPaths(page);
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('keyboard focus survives disclosure and item removal without taking focus from controls', async () => {
  const { page, errors } = await open();
  try {
    await page.focus('#browser-history');
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('Enter');
    await settle(page);
    const focused = await page.$eval('#browser-history', tree => tree.getAttribute('aria-activedescendant'));
    await page.keyboard.press('ArrowRight');
    await settle(page);
    assert.equal(await page.$eval(`#${focused}`, row => row.getAttribute('aria-expanded')), 'true');
    assert.equal(await page.evaluate(() => document.activeElement.id), 'browser-history');
    await page.keyboard.press('End');
    await settle(page);
    assert.equal(await page.$eval('#browser-history [data-focused=true]', row => row.getAttribute('aria-posinset')), '300');
    await page.keyboard.press('Enter');
    await settle(page);
    await clickButton(page, 'Retract selected item');
    assert.equal(await page.evaluate(() => document.activeElement.textContent), 'Retract selected item', 'reconciliation must not steal focus back from a control');
    assert.ok(await page.$eval('#browser-history', tree => !!document.getElementById(tree.getAttribute('aria-activedescendant'))));
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('narrow layout, zoom and reduced motion keep the viewport usable', async () => {
  const { page, errors } = await open();
  try {
    await page.emulateMediaFeatures([{ name: 'prefers-reduced-motion', value: 'reduce' }]);
    await clickButton(page, 'Toggle narrow layout');
    await page.evaluate(() => { document.body.style.zoom = '1.5'; });
    await settle(page);
    await clickButton(page, 'Resolve endpoints');
    assert.ok(await page.evaluate(() => [...document.querySelectorAll('.idle-history-item,.idle-history-path')].every(element => getComputedStyle(element).animationName === 'none')));
    await page.focus('#browser-history');
    await page.keyboard.press('PageDown');
    await settle(page);
    assert.ok(await page.$eval('#browser-history', tree => tree.scrollTop > 0));
    await page.screenshot({ path: 'out/history-tests/narrow.png', fullPage: true });
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});
