import assert from 'node:assert/strict';
import { readFile, mkdir } from 'node:fs/promises';
import { createServer } from 'node:http';
import { resolve, extname, sep } from 'node:path';
import { after, before, test } from 'node:test';
import puppeteer from 'puppeteer-core';

const root = resolve('out/sessions');
let server, browser, address;
const id = value => value.toString(16).padStart(64, '0');
before(async () => {
  assert.ok(process.env.CHROME_PATH, 'CHROME_PATH must name an installed Chrome executable');
  await readFile(resolve(root, 'pkg/sessions_bg.wasm'));
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
  await page.waitForSelector('#browser [data-request-id="remote-bob"]');
  return { page, errors };
}

async function settle(page) {
  await page.evaluate(() => new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done))));
}

async function clickButton(page, root, label) {
  for (const button of await page.$$(`${root} button`)) {
    if ((await button.evaluate(element => element.textContent)).trim() === label) {
      await button.click();
      await settle(page);
      return;
    }
  }
  assert.fail(`No button ${label} in ${root}`);
}

const text = (page, selector) => page.$eval(selector, element => element.textContent);
const prompt = (host, request) => `#${host} [data-request-id="${request}"]`;

async function submit(page, host, value = '  Exact prompt\n🙂  ') {
  await page.type(`#${host}-composer-text`, value);
  await clickButton(page, `#${host}-composer`, 'Send prompt');
  await page.waitForSelector(prompt(host, `${host}-prompt-0`));
}

for (const host of ['browser', 'extension']) {
  test(`${host}: shared reducer supplies pending, receipt, order and execution without local ordering`, async () => {
    const { page, errors } = await open();
    try {
      const request = `${host}-prompt-0`;
      await submit(page, host);
      for (const surface of ['browser', 'extension']) {
        const local = prompt(surface, request);
        assert.match(await text(page, local), /Pending delivery/);
        assert.match(await text(page, local), /Awaiting runtime confirmation/);
        assert.equal(await page.$eval(local, element => element.getAttribute('data-runtime-order')), null);
        assert.equal(await text(page, `${local} .idle-session-prompt-text`), '  Exact prompt\n🙂  ');
        assert.match(await text(page, `${local} .idle-session-attribution`), /contributor-alice.*You.*Session owner/s);
        assert.doesNotMatch(await text(page, `${prompt(surface, 'remote-bob')} .idle-session-attribution`), /Session owner/);
      }
      assert.equal(await page.$eval(`#${host}-composer-text`, element => element.value), '', 'host clears only after app-core stores the prompt');
      await clickButton(page, '.fixture-actions', 'Receive prompt');
      assert.match(await text(page, prompt(host, request)), /Received by coordination.*Awaiting runtime confirmation/s);
      assert.equal(await page.$eval(prompt(host, request), element => element.getAttribute('data-runtime-order')), null);
      await clickButton(page, '.fixture-actions', 'Accept prompt');
      assert.match(await text(page, prompt(host, request)), /Accepted · awaiting order/);
      assert.equal(await page.$eval(prompt(host, request), element => element.getAttribute('data-runtime-order')), null);
      await clickButton(page, '.fixture-actions', 'Assign runtime order 9');
      await clickButton(page, '.fixture-actions', 'Run prompt');
      assert.match(await text(page, prompt(host, request)), /Running.*Runtime order #9/s);
      await clickButton(page, '.fixture-actions', 'Complete prompt');
      for (const surface of ['browser', 'extension']) {
        assert.match(await text(page, prompt(surface, request)), /Completed.*Runtime order #9/s);
        const orders = await page.$$eval(`#${surface} [data-runtime-order]`, elements => elements.map(element => element.dataset.runtimeOrder));
        assert.deepEqual(orders, ['42', '9'], 'view position remains independent from runtime order');
      }
      await clickButton(page, `#${host}`, 'Refresh sessions');
      assert.equal(await page.$$eval(`#${host} .idle-session-prompt`, elements => elements.length), 2, 'recovery does not duplicate prompts');
      assert.match(await text(page, prompt(host, request)), /Completed.*Runtime order #9/s);
      assert.deepEqual(errors, []);
    } finally { await page.close(); }
  });
}

test('composer preserves newlines, composition and keyboard focus in both mounts', async () => {
  const { page, errors } = await open();
  try {
    for (const host of ['browser', 'extension']) {
      const field = `#${host}-composer-text`;
      await page.type(field, 'first');
      await page.keyboard.press('Enter');
      await page.type(field, 'second');
      assert.equal(await page.$eval(field, element => element.value), 'first\nsecond');
      await page.$eval(field, element => {
        element.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
        element.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', code: 'Enter', ctrlKey: true, isComposing: true, bubbles: true }));
      });
      await clickButton(page, `#${host}-composer`, 'Send prompt');
      assert.equal(await page.$(prompt(host, `${host}-prompt-0`)), null, 'IME cannot send through a key or button');
      await page.$eval(field, element => element.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true })));
      await page.focus(field);
      await page.keyboard.down('Control');
      await page.keyboard.press('Enter');
      await page.keyboard.up('Control');
      await page.waitForSelector(prompt(host, `${host}-prompt-0`));
      assert.equal(await page.evaluate(() => document.activeElement.id), `${host}-composer-text`);
    }
    const ids = await page.$$eval('[id]', elements => elements.map(element => element.id));
    assert.equal(ids.length, new Set(ids).size, 'both mounts have unique form/disclosure IDs');
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('invitations and revocation wait for recorded grants and distinguish ownership from prompt permission', async () => {
  const { page, errors } = await open();
  try {
    await submit(page, 'browser');
    await clickButton(page, '.fixture-actions', 'Uncertain delivery');
    await page.select('#extension-sharing-recipient', 'contributor-bob');
    await page.select('#extension-sharing-access', 'contribute');
    await clickButton(page, '#extension-sharing', 'Invite participant');
    assert.match(await text(page, '.fixture-action'), /Invite.*grant_id: "extension-grant-0".*Observe, SubmitInput/s);
    assert.equal(await page.$('[data-grant-id="extension-grant-0"]'), null, 'click is not a grant');
    await clickButton(page, '.fixture-actions', 'Commit sharing');
    assert.match(await text(page, '#extension-sharing'), /Sharing change committed/);
    assert.equal(await page.$('[data-grant-id="extension-grant-0"]'), null, 'receipt waits for the stream');
    await clickButton(page, '.fixture-actions', 'Publish sharing');
    for (const host of ['browser', 'extension']) {
      assert.match(await text(page, `#${host} [data-grant-id="extension-grant-0"]`), /contributor-bob.*Issued.*Read · Send prompts/s);
    }
    await clickButton(page, '#browser [data-grant-id="extension-grant-0"]', 'Revoke invitation');
    const revocation = await text(page, '.fixture-action');
    for (const host of ['browser', 'extension']) {
      const grant = `#${host} [data-grant-id="extension-grant-0"]`;
      assert.equal(await page.$eval(`${grant} button`, element => element.getAttribute('aria-busy')), 'true');
      await clickButton(page, grant, 'Revoke invitation');
      assert.equal(await text(page, '.fixture-action'), revocation, 'a fresh host token cannot duplicate a pending revocation');
    }
    await clickButton(page, '.fixture-actions', 'Commit sharing');
    assert.doesNotMatch(await text(page, '#browser [data-grant-id="extension-grant-0"] .idle-badge'), /Revoked/);
    for (const host of ['browser', 'extension']) {
      const grant = `#${host} [data-grant-id="extension-grant-0"]`;
      assert.equal(await page.$eval(`${grant} button`, element => element.getAttribute('aria-busy')), 'true', 'a commit still waits for the grant update');
      await clickButton(page, grant, 'Revoke invitation');
      assert.equal(await text(page, '.fixture-action'), revocation);
    }
    await clickButton(page, '.fixture-actions', 'Publish sharing');
    assert.match(await text(page, '#extension [data-grant-id="extension-grant-0"] .idle-badge'), /Revoked/);
    assert.equal(await page.$eval('#extension [data-grant-id="extension-grant-0"] button', element => element.disabled), true);
    await page.type('#browser-composer-text', 'Retain after revocation');
    await clickButton(page, '#browser [data-grant-id="grant-session-alice"]', 'Revoke invitation');
    await clickButton(page, '.fixture-actions', 'Commit sharing');
    await clickButton(page, '.fixture-actions', 'Publish sharing');
    assert.match(await text(page, '#browser-composer'), /do not have permission to send/);
    assert.equal(await page.$eval('#browser-composer button', element => element.disabled), true);
    assert.equal(await page.$eval('#browser-composer-text', element => element.value), 'Retain after revocation');
    assert.equal(await page.$eval('#browser-sharing-recipient', element => element.disabled), false, 'ownership still permits sharing, independently of input grants');
    await clickButton(page, prompt('browser', 'browser-prompt-0'), 'Check delivery');
    assert.match(await text(page, '.fixture-action'), /Recover\("browser-prompt-0"\)/, 'an observer can still look up an uncertain request after input access is revoked');
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('failed and uncertain delivery retain original retry identities and require the provider deadline', async () => {
  for (const uncertain of [false, true]) {
    const { page, errors } = await open();
    try {
      await submit(page, 'browser');
      await clickButton(page, '.fixture-actions', uncertain ? 'Uncertain delivery' : 'Fail delivery');
      const local = prompt('extension', 'browser-prompt-0');
      assert.match(await text(page, local), /Delivery failed/);
      if (uncertain) {
        await clickButton(page, local, 'Check delivery');
        assert.match(await text(page, '.fixture-action'), /Recover\("browser-prompt-0"\)/);
        assert.match(await text(page, local), /Delivery outcome unknown/);
        assert.doesNotMatch(await text(page, local), /Retry delivery/);
      } else {
        assert.doesNotMatch(await text(page, local), /Retry delivery/);
        await clickButton(page, '.fixture-actions', 'Advance provider clock');
        await clickButton(page, local, 'Retry delivery');
        assert.match(await text(page, '.fixture-action'), /Retry\("browser-prompt-0"\)/);
        assert.match(await text(page, local), /Pending delivery/);
        await clickButton(page, '.fixture-actions', 'Receive prompt');
        assert.match(await text(page, local), /Received by coordination/);
      }
      assert.equal(await page.$$eval('#browser .idle-session-prompt', elements => elements.length), 2);
      assert.deepEqual(errors, []);
    } finally { await page.close(); }
  }
});

test('reconnect, selection changes and unavailable adapters retain drafts without authorizing stale sends', async () => {
  const { page, errors } = await open();
  try {
    await page.type('#extension-composer-text', 'Unsent session-specific draft');
    await clickButton(page, '.fixture-actions', 'Disconnect updates');
    assert.match(await text(page, '#extension'), /Session updates disconnected/);
    assert.equal(await page.$eval('#extension-composer button', element => element.disabled), true);
    await clickButton(page, '#browser', 'Refresh sessions');
    assert.equal(await page.$eval('#extension-composer button', element => element.disabled), false);
    await clickButton(page, '.fixture-actions', 'Select control session');
    assert.equal(await page.$eval('#extension-composer-text', element => element.value), '');
    assert.equal(await page.$eval('#extension-composer button', element => element.disabled), true);
    assert.equal(await page.$('#extension [data-output-item]'), null, 'history binding changes cannot expose the previous session');
    await clickButton(page, '.fixture-actions', 'Select shared session');
    assert.equal(await page.$eval('#extension-composer-text', element => element.value), 'Unsent session-specific draft');
    await clickButton(page, '.fixture-actions', 'Remove adapters');
    for (const host of ['browser', 'extension']) {
      assert.match(await text(page, `#${host}-composer`), /Prompt delivery is unavailable/);
      assert.match(await text(page, `#${host}-sharing`), /Sharing is unavailable/);
      assert.equal(await page.$eval(`#${host}-sharing-recipient`, element => element.disabled), true);
    }
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('recorded output expands through shared history actions and stays escaped and attributed', async () => {
  const { page, errors } = await open();
  try {
    const row = `#browser [data-output-item="${id(1)}"]`;
    assert.match(await text(page, row), /Preview truncated/);
    await clickButton(page, row, 'Expand content');
    for (const host of ['browser', 'extension']) {
      const output = `#${host} [data-output-item="${id(1)}"]`;
      assert.match(await text(page, output), /MESSAGE_END/);
      assert.match(await text(page, output), /Author: agent:runner/);
      assert.match(await text(page, output), /runtime-recorder/);
    }
    assert.equal(await page.evaluate(() => window.sessionMarkupExecuted), undefined);
    const tool = `#extension [data-output-item="${id(2)}"]`;
    await clickButton(page, tool, 'Expand content');
    assert.equal(await page.$$eval(`${tool} [data-attempt]`, elements => elements.length), 3);
    for (const value of ['Stdout', 'Stderr', 'Partial content', 'Content unavailable']) assert.ok((await text(page, tool)).includes(value), value);
    await clickButton(page, row, 'Inspect records');
    assert.match(await text(page, '#browser-conversation-history-details'), /agent:runner/);
    await mkdir('out/sessions-tests', { recursive: true });
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.screenshot({ path: 'out/sessions-tests/desktop.png' });
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});

test('narrow layouts keep conversation, composer and sharing usable inside the viewport', async () => {
  const { page, errors } = await open(390);
  try {
    await submit(page, 'browser', 'A'.repeat(500));
    await clickButton(page, `#browser [data-output-item="${id(2)}"]`, 'Expand content');
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'the page has no horizontal overflow');
    for (const host of ['browser', 'extension']) {
      const field = `#${host}-composer-text`;
      await page.focus(field);
      assert.equal(await page.evaluate(() => document.activeElement.tagName), 'TEXTAREA');
      assert.ok(await page.$eval(field, element => element.getBoundingClientRect().width > 250), 'composer uses the available width');
      await page.focus(`#${host}-sharing-recipient`);
      await page.keyboard.press('ArrowDown');
      await page.keyboard.press('Enter');
    }
    await mkdir('out/sessions-tests', { recursive: true });
    await page.$eval('#browser-composer', element => element.scrollIntoView({ block: 'start' }));
    await page.screenshot({ path: 'out/sessions-tests/narrow.png' });
    assert.deepEqual(errors, []);
  } finally { await page.close(); }
});
