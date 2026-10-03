import assert from 'node:assert/strict';
import { readFile, mkdir } from 'node:fs/promises';
import { createServer } from 'node:http';
import { resolve, extname, sep } from 'node:path';
import { after, before, test } from 'node:test';
import puppeteer from 'puppeteer-core';

const root = resolve('out/projections');
let server, browser, address;
before(async () => {
  assert.ok(process.env.CHROME_PATH, 'CHROME_PATH must name an installed Chrome executable');
  await readFile(resolve(root, 'pkg/projections_bg.wasm'));
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

async function open(width = 1500) {
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(String(error)));
  await page.setViewport({ width, height: 1100 });
  await page.goto(address);
  await page.waitForSelector('#browser-tasks [data-row-key="task/checks"]');
  return { page, errors };
}

async function settle(page) {
  await page.evaluate(() => new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done))));
}

async function button(page, root, label) {
  for (const element of await page.$$(`${root} button`)) {
    if ((await element.evaluate(element => element.textContent)).trim() === label) return element;
  }
  assert.fail(`No button ${label} in ${root}`);
}

async function click(page, root, label) {
  await (await button(page, root, label)).click();
  await settle(page);
}

const text = (page, selector) => page.$eval(selector, element => element.textContent);
const row = (host, key) => `#${host}-tasks [data-row-key="${key}"]`;
const counts = (page, host) => text(page, `#${host}-tasks .idle-projection-counts`);

test('all destinations show supplied counts, independent freshness, limitations and semantic layouts', async () => {
  const { page, errors } = await open();
  try {
    for (const host of ['browser', 'sidebar']) {
      assert.match(await counts(page, host), /4 shown4 loaded4 total/);
      assert.match(await text(page, `#${host}-triage`), /1 shown1 loaded17 total.*Stale.*Partial results.*bounded read/s);
      assert.match(await text(page, `#${host}-input`), /Freshness unknown.*Choose a provider/s);
      assert.match(await text(page, row(host, 'task/checks')), /Second line <literal text>/);
    }
    assert.deepEqual(await page.$$eval('#browser-tasks .idle-projection-column-heading', elements => elements.map(element => element.textContent)), ['active', 'queued', 'No status', 'Empty status']);
    assert.equal(await page.$$eval('#browser-errors th[scope="col"]', elements => elements.length), 4);
    assert.equal(await page.$$eval('#browser-errors th[scope="row"]', elements => elements.length), 2);
    assert.ok(await page.$('#browser-triage .idle-projection-cards'));
    assert.ok(await page.$('#sidebar-errors .idle-projection-list'));
    assert.match(await text(page, '#browser-triage .idle-projection-gaps'), new RegExp('81'.repeat(32)));
    assert.equal(await page.$eval('#last-action', element => element.textContent), 'No action dispatched', 'mount has no actions');
    const ids = await page.$$eval('[id]', elements => elements.map(element => element.id));
    assert.equal(ids.length, new Set(ids).size, 'every composition has unique IDs');
    assert.equal(await page.$$eval('label[for]', elements => elements.every(element => !!document.getElementById(element.htmlFor))), true);
    assert.equal(await page.$$eval('[aria-describedby]', elements => elements.every(element => element.getAttribute('aria-describedby').split(' ').every(id => !!document.getElementById(id)))), true);
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('native keyboard selection and all layouts share the same selected key', async () => {
  const { page, errors } = await open();
  try {
    await page.focus(`${row('sidebar', 'task/checks')} .idle-projection-title`);
    await page.keyboard.press('Enter');
    await settle(page);
    for (const layout of ['table', 'cards', 'list', 'board']) {
      await page.select('#layout', layout);
      await settle(page);
      assert.ok(await page.$(`#browser-tasks .idle-projection-${layout === 'table' ? 'table-scroll' : layout}`));
      for (const host of ['browser', 'sidebar']) {
        assert.equal(await page.$eval(`${row(host, 'task/checks')} .idle-projection-title`, element => element.getAttribute('aria-pressed')), 'true');
      }
      assert.match(await counts(page, 'browser'), /4 shown4 loaded4 total/);
    }
    await page.focus(`${row('browser', 'task/review')} .idle-projection-title`);
    await page.keyboard.press('Space');
    await settle(page);
    assert.match(await text(page, '#last-action'), /Select.*Task.*task\/review/s);
    await click(page, '#sidebar-tasks', 'Clear selection');
    assert.equal(await page.$$eval('.idle-projection-title[aria-pressed="true"]', elements => elements.length), 0);
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('source and related record actions retain full addresses without changing row selection', async () => {
  const { page, errors } = await open();
  try {
    for (const host of ['browser', 'sidebar']) {
      const source = row(host, 'task/checks');
      await page.click(`${source} .idle-projection-records summary`);
      await click(page, source, 'Inspect source 1');
      const action = await text(page, '#last-action');
      assert.match(action, /Inspect.*Task.*task\/checks/s);
      for (const prefix of ['30', 'a0', 'b0']) assert.ok(action.includes(prefix.repeat(32)));
      const selection = await text(page, '#history-selection');
      for (const prefix of ['30', 'a0']) assert.ok(selection.includes(prefix.repeat(32)));
      assert.equal(await page.$$eval('.idle-projection-title[aria-pressed="true"]', elements => elements.length), 0, 'record buttons do not bubble into row selection');
      await click(page, source, 'Inspect related 1');
      assert.match(await text(page, '#last-action'), /observation: None, item: Some/);
      assert.ok((await text(page, '#history-selection')).includes('c0'.repeat(32)));
      await click(page, source, 'Inspect related 2');
      assert.ok((await text(page, '#last-action')).includes('d0'.repeat(32)));
      assert.match(await text(page, '#last-action'), /item: None/);
      assert.ok((await text(page, '#history-selection')).includes('d0'.repeat(32)));
    }
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('text, exact status and conjunctive labels filter through app-core without changing total or hidden selection', async () => {
  const { page, errors } = await open();
  try {
    await click(page, row('browser', 'task/checks'), 'Run workspace checks');
    await page.type('#sidebar-tasks-filters-text', 'Review');
    await settle(page);
    for (const host of ['browser', 'sidebar']) {
      assert.match(await counts(page, host), /1 shown4 loaded4 total/);
      assert.equal(await page.$eval(`#${host}-tasks-filters-text`, element => element.value), 'Review');
      assert.match(await text(page, `#${host}-tasks`), /Selected item is outside the current filters/);
    }
    await page.select('#browser-tasks-filters-status', 'status:queued');
    await settle(page);
    assert.match(await text(page, '#last-action'), /text: "Review", status: Some\("queued"\)/);
    await click(page, '#sidebar-tasks .idle-projection-label-filters', 'needs review');
    await click(page, '#browser-tasks .idle-projection-label-filters', 'Empty label');
    assert.match(await text(page, '#last-action'), /labels: \["needs review", ""\]/);
    await page.type('#browser-tasks-filters-text', ' does not match');
    await settle(page);
    assert.match(await counts(page, 'sidebar'), /0 shown4 loaded4 total/);
    assert.match(await text(page, '#sidebar-tasks'), /No matching items/);
    assert.equal(await page.$$eval('#sidebar-tasks .idle-projection-label-filter[aria-pressed="true"]', elements => elements.length), 2, 'active labels remain removable with no shown rows');
    await click(page, '#browser-tasks', 'Clear filters');
    assert.match(await counts(page, 'sidebar'), /4 shown4 loaded4 total/);
    assert.equal(await page.$eval(`${row('sidebar', 'task/checks')} .idle-projection-title`, element => element.getAttribute('aria-pressed')), 'true');
    await page.select('#sidebar-tasks-filters-status', 'status:');
    await settle(page);
    assert.match(await text(page, '#last-action'), /status: Some\(""\)/, 'empty status has a distinct option');
    assert.match(await counts(page, 'browser'), /1 shown4 loaded4 total/);
    await click(page, '#sidebar-tasks', 'Clear filters');
    await click(page, '#browser-tasks .idle-projection-label-filters', 'owner,infra');
    assert.match(await counts(page, 'sidebar'), /3 shown4 loaded4 total/);
    assert.match(await text(page, '#last-action'), /labels: \["owner,infra"\]/, 'commas are part of a label, not delimiters');
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('live replacements preserve keyed DOM, disclosure, focus and scroll, then clear a retracted selection', async () => {
  const { page, errors } = await open();
  try {
    await page.$eval('#sidebar-tasks .idle-projection-list', element => { element.style.maxHeight = '260px'; element.style.overflowY = 'auto'; });
    await click(page, row('sidebar', 'task/review'), 'Review retry policy');
    await page.click(`${row('sidebar', 'task/review')} .idle-projection-records summary`);
    await page.$eval(row('sidebar', 'task/review'), element => element.scrollIntoView({ block: 'start' }));
    await page.focus(`${row('sidebar', 'task/review')} .idle-projection-title`);
    await settle(page);
    const top = await page.$eval(row('sidebar', 'task/review'), element => { window.retainedRow = element; window.retainedFocus = document.activeElement; return element.getBoundingClientRect().top; });
    await (await button(page, '.fixture-actions', 'Prepend task')).evaluate(element => element.click());
    await settle(page);
    assert.equal(await page.$eval(row('sidebar', 'task/review'), element => element === window.retainedRow), true, 'stable keys retain DOM across inserted rows');
    assert.equal(await page.evaluate(() => document.activeElement === window.retainedFocus), true, 'refresh does not steal focus');
    assert.equal(await page.$eval(`${row('sidebar', 'task/review')} details`, element => element.open), true);
    assert.ok(Math.abs((await page.$eval(row('sidebar', 'task/review'), element => element.getBoundingClientRect().top)) - top) <= 1, 'native scroll anchoring retains the focused row in a host scroll container');
    assert.equal(await page.$eval('#sidebar-tasks [data-row-key]', element => element.dataset.rowKey), 'task/late', 'provider order wins over observation identity');
    await click(page, '.fixture-actions', 'Retract selected');
    for (const host of ['browser', 'sidebar']) {
      assert.equal(await page.$(row(host, 'task/review')), null);
      assert.equal(await page.$$eval(`#${host}-tasks .idle-projection-title[aria-pressed="true"]`, elements => elements.length), 0);
    }
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('refresh, failure and reconnect expose retained stale rows and block duplicate refreshes', async () => {
  const { page, errors } = await open();
  try {
    await click(page, '#browser-tasks', 'Refresh projections');
    for (const host of ['browser', 'sidebar']) {
      assert.match(await text(page, `#${host}-tasks`), /Loading projections.*Stale.*Run workspace checks/s);
      assert.equal(await page.$eval(`#${host}-tasks .idle-projection-results`, element => element.getAttribute('aria-busy')), 'true');
      assert.equal(await (await button(page, `#${host}-tasks`, 'Refresh projections')).evaluate(element => element.getAttribute('aria-disabled')), 'true');
    }
    await click(page, '.fixture-actions', 'Fail refresh');
    assert.match(await text(page, '#browser-tasks [role="alert"]'), /Provider is offline/);
    assert.ok(await page.$(row('browser', 'task/checks')));
    await click(page, '#sidebar-tasks', 'Refresh projections');
    await click(page, '.fixture-actions', 'Complete refresh');
    assert.match(await text(page, '#browser-tasks .idle-projection-summary'), /Current/);
    await click(page, '.fixture-actions', 'Suspend');
    assert.match(await text(page, '#browser-tasks'), /waiting to reconnect/);
    assert.equal(await (await button(page, '#browser-tasks', 'Refresh projections')).evaluate(element => element.disabled), true);
    await click(page, '.fixture-actions', 'Reconnect');
    await click(page, '.fixture-actions', 'Complete refresh');
    assert.doesNotMatch(await text(page, '#browser-tasks'), /waiting to reconnect|Loading projections|Stale/);
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('known empty, partial, unavailable and disconnected never collapse into the same empty state', async () => {
  const { page, errors } = await open();
  try {
    for (const [command, title] of [['Empty', 'No items'], ['Partial empty', 'No items loaded'], ['Unavailable', 'Projection unavailable']]) {
      await click(page, '.fixture-actions', command);
      for (const host of ['browser', 'sidebar']) {
        assert.equal(await text(page, `#${host}-tasks .idle-state-title`), title);
        assert.match(await text(page, `#${host}-tasks .idle-projection-summary`), /Freshness unknown/);
        assert.match(await counts(page, host), command === 'Empty' ? /0 total/ : /Total unknown/);
      }
    }
    await click(page, '.fixture-actions', 'Reset');
    await click(page, '.fixture-actions', 'Disconnect');
    assert.equal(await page.$$eval('[data-row-key]', elements => elements.length), 0);
    assert.equal(await page.$('.idle-projection-filters'), null);
    assert.equal(await page.$('.idle-state-title'), null);
    assert.match(await text(page, '#sidebar-tasks'), /Connect a workspace/);
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('wide and narrow compositions keep long addresses usable and tables scroll locally', async () => {
  const { page, errors } = await open();
  try {
    await mkdir('out/test-artifacts', { recursive: true });
    await page.screenshot({ path: 'out/test-artifacts/projections-wide.png', fullPage: true });
    for (const width of [760, 320]) {
      await page.setViewport({ width, height: 1000 });
      for (const layout of ['board', 'cards', 'table', 'list']) {
        await page.select('#layout', layout);
        await settle(page);
        await page.click(`${row('browser', 'task/checks')} .idle-projection-records summary`);
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true, `${layout} stays inside a ${width}px page`);
        const table = '#browser-errors .idle-projection-table-scroll';
        assert.equal(await page.$eval(table, element => element.tabIndex), 0, 'table scrolling is keyboard accessible');
        if (width === 320) assert.equal(await page.$eval(table, element => element.scrollWidth > element.clientWidth), true, 'table overflows only its own viewport');
        await click(page, row('browser', 'task/checks'), 'Inspect source 1');
        assert.ok((await text(page, '#last-action')).includes('b0'.repeat(32)));
      }
    }
    await page.screenshot({ path: 'out/test-artifacts/projections-narrow.png', fullPage: true });
    await page.emulateMediaFeatures([{ name: 'prefers-reduced-motion', value: 'reduce' }]);
    await click(page, '#sidebar-tasks', 'Refresh projections');
    assert.equal(await page.$eval('#sidebar-tasks .idle-spinner', element => getComputedStyle(element).animationName), 'none');
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});
