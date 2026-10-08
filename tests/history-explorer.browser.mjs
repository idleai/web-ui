import assert from 'node:assert/strict';
import { readFile, mkdir } from 'node:fs/promises';
import { createServer } from 'node:http';
import { resolve, extname, sep } from 'node:path';
import { after, before, test } from 'node:test';
import puppeteer from 'puppeteer-core';

const root = resolve('out/history-explorer');
const screenshots = 'out/history-explorer-tests';
const rows = '#idle-history .idle-timeline-row';
let server, browser, address;

before(async () => {
  assert.ok(process.env.CHROME_PATH, 'CHROME_PATH must name an installed Chromium executable');
  await readFile(resolve(root, 'pkg/history_explorer_bg.wasm'));
  server = createServer(async (request, response) => {
    const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
    const path = resolve(root, `.${pathname === '/' ? '/index.html' : pathname}`);
    if (!path.startsWith(root + sep)) { response.writeHead(403).end(); return; }
    try {
      const data = await readFile(path);
      response.writeHead(200, { 'Content-Type': { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm' }[extname(path)] || 'application/octet-stream' }).end(data);
    } catch { response.writeHead(404).end(); }
  });
  await new Promise(done => server.listen(0, '127.0.0.1', done));
  address = `http://127.0.0.1:${server.address().port}/`;
  browser = await puppeteer.launch({ executablePath: process.env.CHROME_PATH, headless: true, args: ['--no-sandbox'] });
  await mkdir(screenshots, { recursive: true });
});
after(async () => {
  await browser?.close();
  if (server) await new Promise(done => server.close(done));
});
async function settle(page) {
  await page.evaluate(() => new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done))));
  await page.waitForFunction(() => {
    const boxes = [...document.querySelectorAll('#idle-history .idle-timeline-row')].map(row => row.getBoundingClientRect()).sort((a, b) => a.top - b.top);
    return boxes.every((box, index) => box.height === 34 && (!index || box.top >= boxes[index - 1].bottom - 1));
  });
}
async function open(width = 1600, height = 1000) {
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(String(error)));
  await page.setViewport({ width, height });
  await page.emulateMediaFeatures([{ name: 'prefers-reduced-motion', value: 'reduce' }]);
  await page.goto(address);
  await page.waitForSelector(rows);
  await settle(page);
  return { page, errors };
}
async function button(page, label) {
  const handle = await page.evaluateHandle(label => [...document.querySelectorAll('button')].find(button => button.getAttribute('aria-label') === label || button.textContent.trim() === label), label);
  assert.ok(handle.asElement(), `Button ${label} exists`);
  await handle.asElement().click();
  await settle(page);
}
async function range(page, label, value) {
  await page.click('.idle-timeline-settings > summary');
  await page.$eval(`input[aria-label="${label}"]`, (input, value) => { input.value = value; input.dispatchEvent(new Event('input', { bubbles: true })); }, value);
  await page.click('.idle-timeline-settings > summary');
  await settle(page);
}
const opens = page => page.$eval('#native-opens', element => Number(element.textContent));
const selected = page => page.$eval(`${rows}[aria-selected=true]`, row => row.dataset.occurrence);

test('small native recordings keep distinct streams and smooth metro connections in both surfaces', async () => {
  const recordings = JSON.parse(await readFile('crates/web-ui/examples/history_explorer/simple-cases.json', 'utf8'));
  const { page, errors } = await open(1440, 700);
  try {
    for (const [name, recording] of Object.entries(recordings)) {
      await page.select('select[aria-label="Graph case"]', name);
      await settle(page);
      for (const selector of [rows, '.idle-history-mini .idle-history-mini-row']) {
        const actual = await page.$$eval(selector, elements => elements.map(element => {
          const svg = element.querySelector('.idle-timeline-graph');
          const dot = svg.querySelector('circle');
          const paths = [...svg.querySelectorAll('.idle-timeline-rail, .idle-timeline-bend')].map(path => {
            const length = path.getTotalLength();
            const first = path.getPointAtLength(0), next = path.getPointAtLength(0.005);
            const last = path.getPointAtLength(length), previous = path.getPointAtLength(length - 0.005);
            let clearance = Infinity;
            for (let sample = 1; sample < 200; sample++) {
              const point = path.getPointAtLength(length * sample / 200);
              clearance = Math.min(clearance, Math.hypot(point.x - Number(dot.getAttribute('cx')), point.y - Number(dot.getAttribute('cy'))));
            }
            return { d: path.getAttribute('d'), style: path.getAttribute('style'), bend: path.classList.contains('idle-timeline-bend'),
              first: { x: first.x, y: first.y }, last: { x: last.x, y: last.y }, clearance,
              firstDx: Math.abs(next.x - first.x), lastDx: Math.abs(last.x - previous.x) };
          });
          return { occurrence: element.dataset.occurrence, x: Number(dot.getAttribute('cx')), y: Number(dot.getAttribute('cy')), height: Number(svg.getAttribute('height')), paths };
        }));
        assert.equal(actual.length, recording.rows.length, `${name}: all small-case rows render`);
        const pitch = name === 'linear' ? 18 : Math.min(...actual.map(row => row.x).filter(x => x > 14)) - 14;
        for (const [index, row] of actual.entries()) {
          const native = recording.rows.find(candidate => candidate.occurrence === row.occurrence);
          const expected = native.preview.startsWith('Main:') || native.preview.startsWith('Stream A:') ? 0
            : native.preview.startsWith('Grandchild:') || native.preview.startsWith('Child B:') || (name === 'passing' && native.preview.startsWith('Child:')) ? 2 : 1;
          assert.equal(row.x, 14 + expected * pitch, `${name}: ${native.preview} has its own column`);
          if (name === 'linear' || name === 'independent') {
            assert(row.paths.every(path => !path.bend), `${name}: no invented joins`);
          }
          for (const path of row.paths.filter(path => path.bend)) {
            assert.match(path.d, /^M [-\d.]+ [-\d.]+ C /, `${name}: each turn is a cubic curve`);
            assert(path.firstDx < 0.001 && path.lastDx < 0.001, `${name}: turns meet adjoining rails vertically: ${path.d}`);
            const attached = (path.first.x === row.x && path.first.y === row.y) || (path.last.x === row.x && path.last.y === row.y);
            if (!attached) assert(path.clearance >= 5, `${name}: a passing curve clears the unrelated dot: ${path.d}`);
            if (['fork', 'join', 'nested', 'siblings'].includes(name)) {
              const branch = Math.round((Math.max(path.first.x, path.last.x) - 14) / pitch);
              assert(path.style.includes(`--idle-history-lane-${branch}`), `${name}: a branch keeps its color through its turn`);
            }
          }
          if (index) {
            const boundary = (paths, y) => [...new Set(paths.flatMap(path => [path.first, path.last]).filter(point => point.y === y).map(point => point.x))].sort((a, b) => a - b);
            assert.deepEqual(boundary(actual[index - 1].paths, row.height), boundary(row.paths, 0), `${name}: adjoining rows have no broken rails`);
          }
        }
      }
      await page.screenshot({ path: `${screenshots}/metro-${name}.png`, clip: { x: 0, y: 0, width: 1440, height: 440 } });
    }
    await button(page, 'Switch theme');
    await page.screenshot({ path: `${screenshots}/metro-siblings-light.png`, clip: { x: 0, y: 0, width: 1440, height: 440 } });
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('mini uses compact native routes and reveals the exact editor occurrence without opening a document', async () => {
  const { page, errors } = await open(1180, 820);
  try {
    const mini = '.idle-history-mini';
    await page.waitForSelector(`${mini} .idle-history-mini-row`);
    const shape = await page.$eval(mini, element => {
      const row = element.querySelector('.idle-history-mini-row');
      return { count: element.querySelectorAll('.idle-history-mini-row').length, height: row.getBoundingClientRect().height,
        width: element.getBoundingClientRect().width, scroll: element.scrollWidth,
        continuation: element.textContent.includes('History continues'),
        paths: element.querySelectorAll('.idle-timeline-continuation').length };
    });
    assert.equal(shape.height, 28);
    assert(shape.count <= 40 && shape.scroll <= shape.width + 1, JSON.stringify(shape));
    assert(shape.continuation && shape.paths > 0, 'dense branches expose continuations and the full editor action');
    const selector = `${mini} .idle-history-mini-row:nth-child(3)`;
    const exact = await page.$eval(selector, row => ({ occurrence: row.dataset.occurrence, operation: row.dataset.operation, hash: row.dataset.recordHash }));
    await page.click(`${selector} .idle-mini-title`);
    await settle(page);
    assert.equal(await opens(page), 0, 'mini selection reveals the editor without activating a native document');
    assert.equal(await selected(page), exact.occurrence);
    assert.deepEqual(await page.$eval(`${rows}[aria-selected=true]`, row => ({ occurrence: row.dataset.occurrence, operation: row.dataset.operation, hash: row.dataset.recordHash })), exact);
    await page.screenshot({ path: `${screenshots}/mini-handoff.png` });
    await button(page, 'Switch theme');
    await page.screenshot({ path: `${screenshots}/mini-light.png` });
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('full editor uses aligned 34px columns, native branch coverage, and bounded mounted rows', async () => {
  const { page, errors } = await open();
  try {
    const dimensions = await page.evaluate(() => {
      const grid = document.querySelector('#idle-history');
      const row = grid.querySelector('.idle-timeline-row');
      const header = [...grid.querySelectorAll('[role=columnheader]')];
      return { height: grid.clientHeight, row: row.getBoundingClientRect().height,
        columns: [...row.children].map((cell, index) => Math.abs(cell.getBoundingClientRect().left - header[index].getBoundingClientRect().left)),
        pageHeight: document.documentElement.scrollHeight, viewport: innerHeight,
        rows: grid.querySelectorAll('.idle-timeline-row').length,
        bends: grid.querySelectorAll('.idle-timeline-bend').length, rails: grid.querySelectorAll('.idle-timeline-rail').length };
    });
    assert.ok(dimensions.height > 750, JSON.stringify(dimensions));
    assert.equal(dimensions.row, 34);
    assert.ok(dimensions.columns.every(delta => delta <= 1), JSON.stringify(dimensions));
    assert.ok(dimensions.rows < 50, 'only a viewport and small overscan are mounted');
    assert.ok(dimensions.rails > 100 && dimensions.bends >= 10, JSON.stringify(dimensions));
    assert.ok(dimensions.pageHeight <= dimensions.viewport, 'only the table scrolls vertically');
    assert.equal(await page.$('.idle-history-inspector'), null);
    assert.match(await page.$eval('.idle-timeline-date', element => element.textContent), /Oct|2026/);
    await page.screenshot({ path: `${screenshots}/editor-dark.png` });
    await range(page, 'Graph column width', 360);
    await page.setViewport({ width: 1600, height: 1160 });
    await settle(page);
    await page.screenshot({ path: `${screenshots}/editor-dense.png` });
    await button(page, 'Switch theme');
    await page.screenshot({ path: `${screenshots}/editor-light.png` });
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('mini disclosure retains its own viewport and does not alter editor disclosure', async () => {
  const { page, errors } = await open(1180, 820);
  try {
    const control = await page.$('.idle-history-mini .idle-timeline-disclosure');
    assert.ok(control, 'the mini includes a native group header');
    const occurrence = await control.evaluate(button => button.closest('[data-occurrence]').dataset.occurrence);
    const editor = `${rows}[data-occurrence="${occurrence}"] .idle-timeline-disclosure`;
    const mini = `.idle-history-mini-row[data-occurrence="${occurrence}"] .idle-timeline-disclosure`;
    assert.equal(await page.$eval(editor, button => button.getAttribute('aria-expanded')), 'false');
    await control.click();
    await settle(page);
    assert.equal(await page.$eval(mini, button => button.getAttribute('aria-expanded')), 'true');
    assert.equal(await page.$eval(editor, button => button.getAttribute('aria-expanded')), 'false');
    assert.equal(await opens(page), 0, 'disclosure never activates the row');
    assert.ok(await page.$$eval('.idle-history-mini-row', rows => rows.length <= 40));
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('single clicks and Enter activate exactly once; arrows do not activate', async () => {
  const { page, errors } = await open();
  try {
    await page.click(`${rows}:first-child .idle-timeline-content`);
    await settle(page);
    const occurrence = await selected(page);
    assert.equal(await opens(page), 1);
    assert.match(occurrence, /^current:[a-f0-9]{64}$/);
    assert.match(await page.$eval(`${rows}[aria-selected=true]`, row => row.dataset.recordHash), /^[a-f0-9]{64}$/);
    await page.$eval(`${rows}[aria-selected=true]`, row => row.dispatchEvent(new MouseEvent('click', { bubbles: true, detail: 2 })));
    assert.equal(await opens(page), 1, 'the second click of a double-click does not open again');
    await page.focus('#idle-history');
    await page.keyboard.press('ArrowDown');
    await settle(page);
    assert.notEqual(await selected(page), occurrence);
    assert.equal(await opens(page), 1);
    await page.keyboard.press('Enter');
    await settle(page);
    assert.equal(await opens(page), 2);
    assert.equal(await page.$('.idle-history-inspector'), null);
    await page.screenshot({ path: `${screenshots}/selected.png` });
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('folded task disclosure reveals compact members without activating the row', async () => {
  const { page, errors } = await open();
  try {
    const control = await page.$(`${rows} .idle-timeline-disclosure`);
    assert.ok(control, 'the native fixture contains a folded safe task path');
    const original = await control.evaluate(button => ({ label: button.getAttribute('aria-label'), id: button.closest('[data-occurrence]').dataset.occurrence }));
    assert.match(original.label, /^Expand (8|10) activities$/);
    const row = `${rows}[data-occurrence="${original.id}"]`;
    await control.click();
    await settle(page);
    assert.equal(await opens(page), 0);
    assert.equal(await page.$eval(`${row} button`, element => element.getAttribute('aria-expanded')), 'true');
    assert.equal(await page.$eval(row, element => element.getBoundingClientRect().height), 34);
    await page.click(`${row} .idle-timeline-content`);
    assert.equal(await opens(page), 1, 'the group representative uses normal row activation');
    await page.click(`${row} .idle-timeline-disclosure`);
    await settle(page);
    assert.equal(await opens(page), 1);
    assert.equal(await page.$eval(`${row} button`, element => element.getAttribute('aria-expanded')), 'false');
    await page.screenshot({ path: `${screenshots}/folded.png` });
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('Find seeks across pages and folded groups with next, previous, and clear', async () => {
  const { page, errors } = await open();
  try {
    await page.focus('#idle-history');
    await page.keyboard.down('Control'); await page.keyboard.press('f'); await page.keyboard.up('Control');
    assert.equal(await page.evaluate(() => document.activeElement.id), 'idle-history-search');
    await page.type('#idle-history-search', 'Parent execution continues');
    await page.keyboard.press('Enter');
    await settle(page);
    assert.equal(await page.$eval('.idle-history-match-count', element => element.textContent), '1 of 18');
    const first = await selected(page);
    assert.ok(await page.$(`${rows}[aria-selected=true][data-match=true]`));
    await button(page, 'Next match'); assert.notEqual(await selected(page), first);
    await button(page, 'Previous match'); assert.equal(await selected(page), first);
    assert.equal(await opens(page), 0, 'Find only selects and seeks');
    await page.focus('#idle-history-search'); await page.keyboard.press('Escape'); await settle(page);
    assert.equal(await page.$eval('#idle-history-search', element => element.value), '');
    assert.equal(await page.$('.idle-history-match-count'), null);
    assert.equal(await page.evaluate(() => document.activeElement.id), 'idle-history-search');
    await page.type('#idle-history-search', 'activity 601'); await page.keyboard.press('Enter'); await settle(page);
    assert.match(await page.$eval(`${rows}[aria-selected=true]`, element => element.textContent), /activity 601/);
    assert.ok(Number(await page.$eval('#idle-history', element => element.getAttribute('aria-rowcount'))) > 500, 'Find retains full timeline context');
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('narrow layout scrolls horizontally; graph width, pan, and zoom preserve topology', async () => {
  const { page, errors } = await open(440, 780);
  try {
    assert.ok(await page.$eval('.idle-timeline-horizontal', element => element.scrollWidth > element.clientWidth));
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'no document overflow');
    const initial = await page.$eval('#idle-history', element => element.clientHeight);
    const paths = await page.$$eval(`${rows} .idle-timeline-bend`, paths => paths.length);
    await range(page, 'Graph column width', '280'); await range(page, 'Graph lane spacing', '19');
    const boundaryNodes = await page.$$eval(`${rows} .idle-timeline-graph`, graphs => graphs.filter(graph =>
      Number(graph.querySelector('circle').getAttribute('cx')) === Number(graph.getAttribute('width'))
    ).map(graph => [...graph.querySelectorAll('.idle-timeline-continuation title')].some(title => title.textContent.includes('right'))));
    assert(boundaryNodes.length > 0, 'the recording includes a node centered exactly on the graph boundary');
    assert(boundaryNodes.every(Boolean), 'a partially clipped node always shows a continuation marker');
    await range(page, 'Graph column width', '130'); await range(page, 'Graph lane spacing', '30'); await range(page, 'Pan graph horizontally', '100');
    assert.equal(await page.$$eval(`${rows} .idle-timeline-bend`, paths => paths.length), paths, 'pixel controls leave returned topology intact');
    await page.setViewport({ width: 640, height: 1080 }); await settle(page);
    assert.ok(await page.$eval('#idle-history', element => element.clientHeight) > initial + 200, 'height tracks the host');
    await page.screenshot({ path: `${screenshots}/editor-narrow.png` });
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('keyboard navigation crosses windows; local retry retains readable rows', async () => {
  const { page, errors } = await open();
  try {
    await button(page, 'Latest'); await page.focus('#idle-history');
    for (let index = 0; index < 9; index += 1) { await page.keyboard.press('PageDown'); await settle(page); }
    assert.ok(await page.$eval('#idle-history', grid => {
      const selected = grid.querySelector('[aria-selected=true]').getBoundingClientRect();
      const viewport = grid.getBoundingClientRect();
      return selected.top >= viewport.top + 27 && selected.bottom <= viewport.bottom + 1;
    }), 'keyboard selection stays visible across a window boundary');
    assert.equal(await opens(page), 0);
    const anchor = await selected(page);
    await button(page, 'Fail history page');
    assert.match(await page.$eval('.idle-timeline-footer', element => element.textContent), /Fixture history is offline/);
    assert.equal(await selected(page), anchor);
    await button(page, 'Retry');
    assert.doesNotMatch(await page.$eval('.idle-timeline-footer', element => element.textContent), /offline/);
    assert.ok(await page.$(rows));
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});
