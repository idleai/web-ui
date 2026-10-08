import assert from 'node:assert/strict';
import { readFile, mkdir } from 'node:fs/promises';
import { createServer } from 'node:http';
import { resolve, extname, sep } from 'node:path';
import { after, before, test } from 'node:test';
import puppeteer from 'puppeteer-core';

const root = resolve('out/navigation');
let server, browser, address;
before(async () => {
  assert.ok(process.env.CHROME_PATH, 'CHROME_PATH must name an installed Chrome executable');
  await readFile(resolve(root, 'pkg/navigation_bg.wasm'));
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

async function open(width = 1160) {
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(String(error)));
  await page.setViewport({ width, height: 1200 });
  await page.goto(address);
  await page.waitForSelector('#sidebar [data-identity="session-control"]');
  return { page, errors };
}

async function settle(page) {
  await page.evaluate(() => new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done))));
}

async function toolbar(page, name) {
  const buttons = await page.$$('.fixture-actions button');
  for (const button of buttons) {
    if ((await button.evaluate(element => element.textContent)).trim() === name) {
      await button.click(); await settle(page); return;
    }
  }
  assert.fail(`Missing fixture control: ${name}`);
}

const text = (page, selector) => page.$eval(selector, element => element.textContent);

test('reference order, control first, supplied counts, graph and separate configuration entries', async () => {
  const { page, errors } = await open();
  try {
    for (const host of ['sidebar', 'browser']) {
      assert.deepEqual(await page.$$eval(`#${host} .idle-navigation-heading h2`, elements => elements.map(element => element.textContent)), ['Workspace', 'Users', 'Sessions', 'Projections', 'Compute hosts', 'Model providers', 'Activity']);
      assert.deepEqual(await page.$$eval(`#${host} .idle-navigation-configuration .idle-navigation-name`, elements => elements.map(element => element.textContent)), ['Settings', 'Agent Rules']);
      assert.deepEqual(await page.$$eval(`#${host} .idle-navigation-count`, elements => elements.map(element => element.textContent)), ['4', '7', '1', '2', '2']);
      const sessions = await page.$$eval(`#${host} [data-section="Sessions"] [data-identity]`, elements => elements.map(element => element.dataset.identity));
      assert.equal(sessions[0], 'session-control');
      assert.equal(sessions.length, 7);
      assert.match(await text(page, `#${host} [data-section="Projections"]`), /24/);
      assert.equal(await page.$eval(`#${host}-activity`, element => element.clientHeight), 176);
      assert.ok(await page.$(`#${host}-activity .idle-timeline-rail`));
      assert.deepEqual(await page.$$eval(`#${host} [data-contributor] .idle-navigation-name`, elements => elements.map(element => element.textContent)), ['Alice', 'Bob', 'Alex', 'Dev']);
      assert.equal(await page.$eval(`#${host} [aria-label="Add compute host — registration unavailable"]`, element => element.disabled), true);
    }
    const ids = await page.$$eval('[id]', elements => elements.map(element => element.id));
    assert.equal(ids.length, new Set(ids).size);
    assert.match(await text(page, '#last-action'), /Timeline\((Load|Visible)/, 'recent activity loads its native window when mounted');
    await mkdir(resolve(root, 'screenshots'), { recursive: true });
    await page.screenshot({ path: resolve(root, 'screenshots/navigation.png'), fullPage: true });
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('keyboard selections and creation share app-core state between both hosts', async () => {
  const { page, errors } = await open();
  try {
    for (const [selector, expected] of [
      ['[data-identity="session-shared"]', /Section: Sessions.*Session: Some\("session-shared"\)/],
      ['[data-identity="host-cluster"]', /Section: ComputeHosts.*Host: Some\("host-cluster"\)/],
      ['[data-identity="provider-external"]', /Section: ModelProviders.*Provider: Some\("provider-external"\)/],
      ['[title="Settings"]', /Section: Settings/],
      ['[title="Agent Rules"]', /Section: AgentRules/],
    ]) {
      await page.focus(`#sidebar ${selector}`);
      await page.keyboard.press('Enter'); await settle(page);
      assert.match(await text(page, '#selection'), expected);
      assert.equal(await page.$eval(`#browser ${selector}`, element => element.getAttribute('aria-pressed')), 'true');
    }
    await page.focus('#sidebar [aria-label="Add session"]');
    await page.keyboard.press('Space'); await settle(page);
    assert.equal(await text(page, '#pending-creations'), '1 pending creations');
    for (const host of ['sidebar', 'browser']) {
      assert.equal(await page.$eval(`#${host} [aria-label="Add session"]`, element => element.disabled), true);
    }
    assert.equal(await page.$$eval('#sidebar [data-section="Sessions"] [data-identity]', elements => elements.length), 7, 'pending creation is not an optimistic session');
    await page.focus('#sidebar-activity'); await page.keyboard.press('ArrowDown'); await page.keyboard.press('Enter'); await settle(page);
    assert.match(await text(page, '#selection'), /Section: Activity/);
    assert.ok(await page.$('#sidebar-activity [aria-selected="true"]'));
    assert.ok(await page.$('#browser-activity [aria-selected="true"]'));
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('dense sections collapse by keyboard and keep their state through updates', async () => {
  const { page, errors } = await open();
  try {
    assert.equal(await page.$eval('#sidebar [data-contributor] .idle-navigation-row', element => element.getBoundingClientRect().height), 22);
    assert.equal(await page.$$eval('#sidebar .idle-navigation-connections', elements => elements.length), 0, 'member activity fits in one row');
    const section = '#sidebar [data-section="Sessions"]';
    const selection = await text(page, '#selection');
    await page.focus(`${section} > summary`);
    await page.keyboard.press('Space'); await settle(page);
    assert.equal(await page.$eval(section, element => element.open), false);
    assert.equal(await text(page, '#selection'), selection, 'collapsing does not navigate');
    await toolbar(page, 'Fail resource refresh');
    assert.equal(await page.$eval(section, element => element.open), false, 'live updates retain the collapsed state');
    await page.focus(`${section} > summary`);
    await page.keyboard.press('Enter'); await settle(page);
    await page.focus(`${section} [aria-label="Open Sessions"]`);
    await page.keyboard.press('Enter'); await settle(page);
    assert.equal(await page.$eval(section, element => element.open), true, 'toolbar actions do not collapse the section');
    assert.match(await text(page, '#selection'), /Section: Sessions/);
    await page.focus('#sidebar [data-section="Workspace"] > summary');
    await page.keyboard.press('Tab');
    await page.keyboard.press('Tab');
    await page.keyboard.press('Enter'); await settle(page);
    assert.match(await text(page, '#selection'), /Section: Workspace/, 'the workspace toolbar returns to the overview');
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('workspace switches, expiry and refresh failure never reuse old status or actions', async () => {
  const { page, errors } = await open();
  try {
    await page.select('#sidebar-repository', 'repository-two'); await settle(page);
    assert.equal(await page.$eval('#browser-repository', element => element.value), 'repository-two');
    await toolbar(page, 'Expire member status and host health');
    assert.match(await text(page, '#sidebar [data-contributor="contributor-alice"]'), /Online status unknown/);
    assert.match(await text(page, '#sidebar [data-identity="session-control"]'), /Lease expired/);
    await toolbar(page, 'Fail resource refresh');
    assert.match(await text(page, '#sidebar [data-section="ComputeHosts"]'), /MacBook Pro.*Resource provider is offline/s);
    assert.equal(await page.$$eval('#sidebar [data-section="ComputeHosts"] [data-tone="success"]', elements => elements.length), 0);
    await page.select('#sidebar-workspace', 'workspace-two'); await settle(page);
    assert.equal(await page.$$eval('#sidebar [data-identity]', elements => elements.length), 0);
    assert.equal(await page.$eval('#sidebar [aria-label="Add session"]', element => element.disabled), true);
    assert.match(await text(page, '#browser'), /Sessions not connected/);
    await toolbar(page, 'Disconnect');
    assert.equal(await page.$eval('#sidebar [title="Settings"]', element => element.disabled), true);
    await toolbar(page, 'Reset standalone');
    for (const host of ['sidebar', 'browser']) {
      assert.equal(await page.$$eval(`#${host} .idle-navigation-selectors select`, elements => elements.length), 1, 'standalone exposes one workspace choice');
      assert.equal(await page.$(`#${host}-repository`), null, 'the automatically bound repository has no duplicate picker');
    }
    await page.select('#sidebar-workspace', 'workspace-two'); await settle(page);
    assert.equal(await page.$eval('#browser-workspace', element => element.value), 'workspace-two');
    await page.select('#sidebar-workspace', 'workspace-one'); await settle(page);
    assert.equal(await page.$('#sidebar-repository'), null);
    assert.ok(await page.$('#sidebar [data-identity="session-control"]'), 'returning to a standalone workspace restores its bound data');
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('narrow layouts, larger text and coarse pointers keep controls accessible', async () => {
  const { page, errors } = await open(320);
  try {
    await page.addStyleTag({ content: 'html { font-size: 20px; }' });
    await settle(page);
    const overflow = await page.$eval('#sidebar', element => element.scrollWidth > element.clientWidth + 1);
    assert.equal(overflow, false, 'sidebar has no horizontal overflow at increased text size');
    await page.setViewport({ width: 390, height: 844, isMobile: true, hasTouch: true });
    // Mobile emulation reloads the page and restarts the WASM application.
    await page.waitForSelector('#sidebar [title="Settings"]');
    await page.addStyleTag({ content: 'html { font-size: 20px; }' });
    await settle(page);
    assert.ok(await page.$eval('#sidebar [title="Settings"]', element => element.getBoundingClientRect().height) >= 44);
    assert.equal(await page.$$eval('label[for]', elements => elements.every(element => !!document.getElementById(element.htmlFor))), true);
    await page.screenshot({ path: resolve(root, 'screenshots/navigation-narrow.png'), fullPage: true });
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});
