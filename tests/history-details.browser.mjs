import assert from 'node:assert/strict';
import { readFile, mkdir } from 'node:fs/promises';
import { createServer } from 'node:http';
import { resolve, extname, sep } from 'node:path';
import { after, before, test } from 'node:test';
import puppeteer from 'puppeteer-core';

const root = resolve('out/history-details');
let server, browser, address;
const id = value => value.toString(16).padStart(64, '0');
before(async () => {
  assert.ok(process.env.CHROME_PATH, 'CHROME_PATH must name an installed Chrome executable');
  await readFile(resolve(root, 'pkg/history_details_bg.wasm'));
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

async function open(width = 1380) {
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(String(error)));
  await page.setViewport({ width, height: 1000 });
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

async function clickButton(page, root, label) {
  const buttons = await page.$$(`${root} button`);
  for (const button of buttons) {
    if ((await button.evaluate(element => element.textContent)).trim() === label) {
      await button.click();
      await settle(page);
      return;
    }
  }
  assert.fail(`No button ${label} in ${root}`);
}

async function select(page, position, host = 'browser') {
  await page.focus(`#${host}-history`);
  await page.keyboard.press('Home');
  for (let index = 0; index < position; index += 1) await page.keyboard.press('ArrowDown');
  await page.keyboard.press('Enter');
  await settle(page);
  await clickButton(page, `#${host}-history-details`, 'Inspect record');
}

test('both hosts expand all blocks through typed actions without executing captured markup', async () => {
  const { page, errors } = await open();
  try {
    const row = `#browser-history [data-item="${id(10)}"]`;
    assert.match(await page.$eval(row, element => element.textContent), /Preview truncated/);
    assert.doesNotMatch(await page.$eval(row, element => element.textContent), /MESSAGE_TAIL/);
    await clickButton(page, row, 'Expand content');
    assert.match(await page.$eval('.fixture-action', element => element.textContent), /ToggleDisclosure/);
    for (const host of ['browser', 'extension']) {
      const selector = `#${host}-history [data-item="${id(10)}"]`;
      const text = await page.$eval(selector, element => element.textContent);
      assert.match(text, /MESSAGE_TAIL/);
      assert.match(text, /second message block remains available/);
      assert.match(text, /human:ada/);
      assert.match(text, /fixture-importer/);
    }
    assert.equal(await page.evaluate(() => window.recordExecuted), undefined);
    assert.match(await page.evaluate(() => document.activeElement.textContent), /Collapse content/, 'row action retains focus');
    const counts = await page.evaluate(() => {
      const ids = [...document.querySelectorAll('[id]')].map(element => element.id);
      return [ids.length, new Set(ids).size];
    });
    assert.equal(counts[0], counts[1], 'disclosure IDs are unique across both mounts');
    await clickButton(page, row, 'Collapse content');
    assert.doesNotMatch(await page.$eval(row, element => element.textContent), /MESSAGE_TAIL/);
    assert.deepEqual(errors, []);
    await mkdir('out/history-details-tests', { recursive: true });
    await page.screenshot({ path: 'out/history-details-tests/desktop.png', fullPage: true });
  } finally { await page.close(); }
});

test('tool attempts and channels remain separate and item paging is explicit', async () => {
  const { page, errors } = await open();
  try {
    await select(page, 1);
    await page.focus('#browser-history');
    await page.keyboard.press('ArrowRight');
    await settle(page);
    const row = `#browser-history [data-item="${id(20)}"]`;
    assert.equal(await page.$$eval(`${row} [data-block]`, blocks => blocks.length), 3);
    const text = await page.$eval(row, element => element.textContent);
    for (const value of ['Stdout', 'Stderr', id(200), id(201), 'Partial content', 'Terminal lifecycle recorded', 'output blob has not arrived']) assert.ok(text.includes(value), value);
    await clickButton(page, '#browser-history-details', 'Load more observations');
    assert.match(await page.$eval('.fixture-action', element => element.textContent), /LoadItem/);
    assert.match(await page.$eval('#browser-history-details', element => element.textContent), /Item scan complete/);
    assert.match(await page.$eval(row, element => element.textContent), /Content unavailable/, 'scan completion does not claim content completion');
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('Original drill-down keeps exact binary bytes and the operation identity', async () => {
  const { page, errors } = await open();
  try {
    await select(page, 0);
    await clickButton(page, '#browser-history-details-original', 'Load exact records and content');
    const action = await page.$eval('.fixture-action', element => element.textContent);
    assert.ok(action.includes('LoadOperationDetails') && action.includes(id(7)) && action.includes('refresh: false'));
    const expected = Buffer.from([123, 34, 122, 34, 58, 32, 49, 44, 32, 34, 97, 34, 58, 50, 125, 13, 10, 0, 255]);
    const shown = await page.$eval('#browser-history-details-original .idle-history-field pre', element => element.textContent);
    assert.deepEqual(Buffer.from(shown.replace(/\s/g, ''), 'hex'), expected, 'Original bytes include recorded order, spaces, CRLF, NUL and non-UTF-8 bytes');
    const disabled = await page.$$eval('#browser-history-details-original .idle-history-open button', buttons => buttons.every(button => button.disabled));
    assert.ok(disabled, 'browser without installed native adapters retains the full inline content');
    await clickButton(page, '#extension-history-details-original', 'Open Original');
    const native = await page.$eval('.fixture-action', element => element.textContent);
    assert.ok(native.includes(id(7)) && native.includes(id(1007)) && native.includes('target: Original'));
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('file snapshots refresh late bytes and native actions obey revocation and pending state', async () => {
  const { page, errors } = await open();
  try {
    await select(page, 2);
    const inspector = '#browser-history-details';
    assert.match(await page.$eval(inspector, element => element.textContent), /Comparison unavailable/);
    assert.match(await page.$eval(inspector, element => element.textContent), /Content missing/);
    await clickButton(page, '.fixture-actions', 'Make late bytes available');
    assert.match(await page.$eval(inspector, element => element.textContent), /Content missing/, 'late content must be requested');
    await clickButton(page, inspector, 'Refresh records and content');
    assert.match(await page.$eval(inspector, element => element.textContent), /Offsets are bytes/);
    assert.doesNotMatch(await page.$eval(inspector, element => element.textContent), /Content missing/);
    await clickButton(page, '#extension-history-details', 'Open recorded comparison');
    const action = await page.$eval('.fixture-action', element => element.textContent);
    assert.ok(action.includes(id(3)) && action.includes(id(1003)) && action.includes('target: Diff'));
    await clickButton(page, '#extension-history-details', 'Open recorded revision');
    assert.equal(await page.$eval('.fixture-action', element => element.textContent), action, 'pending native opens cannot double-dispatch');
    await clickButton(page, '.fixture-actions', 'Revoke native adapters');
    assert.ok(await page.$$eval('#extension-history-details .idle-history-open button', buttons => buttons.every(button => button.disabled)));
    await clickButton(page, '.fixture-actions', 'Report native failure');
    assert.match(await page.$eval('#extension-history-details [role=alert]', element => element.textContent), /editor is offline/);
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('conflicted variants are individually inspectable and native-open retains the chosen digest', async () => {
  const { page, errors } = await open();
  try {
    await select(page, 3);
    const inspector = '#browser-history-details';
    assert.match(await page.$eval(inspector, element => element.textContent), /Conflicted observation/);
    assert.equal(await page.$$eval(`${inspector} .idle-history-raw-record`, records => records.length), 2);
    assert.equal(await page.$$eval(`${inspector} .idle-history-field`, fields => fields.length), 0);
    await clickButton(page, `#extension-history-details .idle-history-raw-record[data-record-hash="${id(9004)}"]`, 'Open stored record');
    const action = await page.$eval('.fixture-action', element => element.textContent);
    assert.ok(action.includes(id(4)) && action.includes(id(9004)) && action.includes('target: Record'));
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('narrow layouts keep bytes and keyboard-accessible scroll regions inside the page', async () => {
  const { page, errors } = await open(390);
  try {
    const row = `#browser-history [data-item="${id(10)}"]`;
    await clickButton(page, row, 'Expand content');
    const block = `${row} pre`;
    await page.click(block);
    assert.equal(await page.evaluate(() => document.activeElement.tagName), 'PRE', 'clicking content must not return focus to the tree');
    const top = await page.$eval('#browser-history', tree => tree.scrollTop);
    await page.$eval(block, element => { element.scrollTop = 100; });
    await settle(page);
    assert.ok(await page.$eval(block, element => element.scrollTop > 0), 'the full block scrolls within its panel');
    assert.equal(await page.$eval('#browser-history', tree => tree.scrollTop), top, 'content scroll events must not move the graph');
    await page.keyboard.press('ArrowDown');
    assert.equal(await page.evaluate(() => document.activeElement.tagName), 'PRE');
    assert.match(await page.$eval('.fixture-action', element => element.textContent), /ToggleDisclosure/, 'block interaction must not select the enclosing item');
    await clickButton(page, row, 'Collapse content');
    await select(page, 0);
    await clickButton(page, '#browser-history-details-original', 'Load exact records and content');
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), 'only the graph and content regions may scroll horizontally');
    const region = '#browser-history-details-original pre';
    await page.focus(region);
    assert.equal(await page.$eval(region, element => element.getAttribute('role')), 'region');
    assert.ok(await page.$eval(region, element => !!element.getAttribute('aria-label')));
    await page.keyboard.press('ArrowDown');
    assert.equal(await page.evaluate(() => document.activeElement.tagName), 'PRE', 'graph navigation does not consume content scrolling');
    assert.deepEqual(errors, []);
    await mkdir('out/history-details-tests', { recursive: true });
    await page.screenshot({ path: 'out/history-details-tests/narrow.png', fullPage: true });
  } finally { await page.close(); }
});
