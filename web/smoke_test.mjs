// Atari Font Maker — WASM Phase 5 browser smoke test.
//
// Drives a real headless Chrome over the Chrome DevTools Protocol (no npm
// dependencies; uses Node's built-in `fetch` + `WebSocket`). Proves:
//   1. the Slint app boots and renders a canvas (not an empty shell);
//   2. Font 1..4 + default Atari glyphs are present (model + rendered pixels);
//   3. Character Editor: select character -> paint pixel -> glyph mutates ->
//      undo/redo restores/reapplies;
//   4. View 40x26: placing the selected character visibly changes the view;
//   5. Open (file picker -> setFileInputFiles) loads a real .atrview fixture;
//   6. Save triggers a real browser download matching the in-memory snapshot;
//   7. Re-open reproduces the same snapshot (lossless round-trip);
//   8. Cancel leaves state untouched;
//   9. a screenshot is captured and verified non-blank;
//  10. there are no console errors / uncaught exceptions.
//
// Exit code 0 == PASS, non-zero == FAIL (with a diagnostic).

import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import http from 'node:http';
import zlib from 'node:zlib';

const CHROME = process.argv[4] || process.env.CHROME || 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe';
const DIST_WIN = process.argv[2];      // Windows path to web/dist
const FIXTURE_WIN = process.argv[3];   // Windows path to sample.atrview
const SCREENSHOT_OUT = process.argv[5] || null; // Windows path for the screenshot PNG
const PORT = 9333;   // debug port
const HTTP_PORT = 9334;

if (!DIST_WIN || !FIXTURE_WIN) {
  console.error('FAIL: usage: node smoke_test.mjs <dist_win_path> <fixture_win_path> [chrome_exe] [screenshot_out]');
  process.exit(1);
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

let chromeProc = null;
let stageTmp = null;
let chromeProfile = null;

function log(msg) { console.log(`[smoke] ${msg}`); }
function fail(msg) { throw new Error(msg); }

// ---------------------------------------------------------------------------
// Minimal PNG decoder (8-bit RGB/RGBA, filters 0-4) — dependency-free, enough
// to assert a screenshot is not a blank single-colour image.
// ---------------------------------------------------------------------------
function decodePng(buf) {
  if (buf.readUInt32BE(0) !== 0x89504e47) throw new Error('not a PNG');
  let pos = 8;
  let width = 0, height = 0, bitDepth = 0, colorType = 0;
  const idat = [];
  while (pos < buf.length) {
    const len = buf.readUInt32BE(pos);
    const type = buf.toString('ascii', pos + 4, pos + 8);
    const data = buf.subarray(pos + 8, pos + 8 + len);
    if (type === 'IHDR') {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      bitDepth = data[8];
      colorType = data[9];
    } else if (type === 'IDAT') {
      idat.push(data);
    } else if (type === 'IEND') {
      break;
    }
    pos += 12 + len;
  }
  if (bitDepth !== 8) throw new Error(`unsupported bit depth ${bitDepth}`);
  const bpp = colorType === 6 ? 4 : colorType === 2 ? 3 : null;
  if (bpp === null) throw new Error(`unsupported color type ${colorType}`);

  const raw = zlib.inflateSync(Buffer.concat(idat));
  const stride = width * bpp;
  const out = Buffer.alloc(height * stride);
  let prev = Buffer.alloc(stride);

  let rp = 0;
  for (let y = 0; y < height; y++) {
    const filter = raw[rp++];
    const row = raw.subarray(rp, rp + stride);
    rp += stride;
    const outRow = out.subarray(y * stride, (y + 1) * stride);
    for (let i = 0; i < stride; i++) {
      const a = i >= bpp ? outRow[i - bpp] : 0;
      const b = prev[i];
      const c = i >= bpp ? prev[i - bpp] : 0;
      let val = row[i];
      switch (filter) {
        case 0: break;
        case 1: val = (val + a) & 0xff; break;
        case 2: val = (val + b) & 0xff; break;
        case 3: val = (val + ((a + b) >> 1)) & 0xff; break;
        case 4: {
          const p = a + b - c;
          const pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c);
          val = (val + (pa <= pb && pa <= pc ? a : pb <= pc ? b : c)) & 0xff;
          break;
        }
        default: throw new Error(`bad filter ${filter}`);
      }
      outRow[i] = val;
    }
    prev = Buffer.from(outRow);
  }

  // Quantize and count distinct colour buckets (non-blank proof).
  const buckets = new Set();
  let nonZero = 0;
  for (let i = 0; i < out.length; i += bpp) {
    const r = out[i], g = out[i + 1], b = out[i + 2];
    if (r || g || b) nonZero++;
    buckets.add(((r >> 4) << 8) | ((g >> 4) << 4) | (b >> 4));
  }
  return { width, height, uniqueColors: buckets.size, nonZeroPixels: nonZero };
}

// ---------------------------------------------------------------------------
// Minimal static file server (serves the wasm-bindgen dist/ directory).
// ---------------------------------------------------------------------------
const MIME = {
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.mjs': 'text/javascript',
  '.wasm': 'application/wasm',
  '.d.ts': 'text/plain',
  '.json': 'application/json',
};
function serveDist() {
  const server = http.createServer((req, res) => {
    let p = decodeURIComponent(new URL(req.url, 'http://x').pathname);
    if (p === '/' ) p = '/index.html';
    const file = path.join(DIST_WIN, p);
    try {
      const data = fs.readFileSync(file);
      const ext = path.extname(file).toLowerCase();
      res.writeHead(200, { 'Content-Type': MIME[ext] || 'application/octet-stream' });
      res.end(data);
    } catch {
      res.writeHead(404); res.end('not found');
    }
  });
  return new Promise((resolve) => server.listen(HTTP_PORT, '127.0.0.1', () => resolve(server)));
}

// ---------------------------------------------------------------------------
// Tiny CDP client.
// ---------------------------------------------------------------------------
class CDP {
  constructor(ws) {
    this.ws = ws;
    this.id = 0;
    this.pending = new Map();
    this.handlers = new Map();
    this.ws.addEventListener('message', (e) => {
      const msg = JSON.parse(e.data);
      if (msg.id && this.pending.has(msg.id)) {
        const { resolve, reject } = this.pending.get(msg.id);
        this.pending.delete(msg.id);
        if (msg.error) reject(new Error(`${msg.error.code}: ${msg.error.message}`));
        else resolve(msg.result);
      } else if (msg.method) {
        const hs = this.handlers.get(msg.method) || [];
        for (const h of hs) h(msg.params);
      }
    });
  }
  send(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = ++this.id;
      this.pending.set(id, { resolve, reject });
      this.ws.send(JSON.stringify({ id, method, params }));
    });
  }
  on(method, handler) {
    if (!this.handlers.has(method)) this.handlers.set(method, []);
    this.handlers.get(method).push(handler);
  }
}

async function connectWS(url) {
  const ws = new WebSocket(url);
  await new Promise((res, rej) => {
    ws.addEventListener('open', res);
    ws.addEventListener('error', () => rej(new Error('websocket error')));
  });
  return ws;
}

async function getJson(url) {
  const r = await fetch(url);
  const text = await r.text();
  try {
    return JSON.parse(text);
  } catch {
    throw new Error(`non-JSON response from ${url}: ${text.slice(0, 200)}`);
  }
}

async function newTab(url) {
  const r = await fetch(`http://127.0.0.1:${PORT}/json/new?${encodeURIComponent(url)}`, { method: 'PUT' });
  const text = await r.text();
  try {
    return JSON.parse(text);
  } catch {
    throw new Error(`non-JSON response from /json/new: ${text.slice(0, 200)}`);
  }
}

async function launchChrome() {
  const chromeExe = path.normalize(CHROME);
  const profile = path.join(os.tmpdir(), `afm_wasm_profile_${Date.now()}`);
  fs.mkdirSync(profile, { recursive: true });
  chromeProfile = profile;
  const args = [
    '--headless=new',
    `--remote-debugging-port=${PORT}`,
    `--user-data-dir=${profile}`,
    '--no-first-run',
    '--no-default-browser-check',
    '--disable-background-networking',
    '--disable-component-update',
    '--disable-dev-shm-usage',
    '--window-size=1280,900',
    'about:blank',
  ];
  chromeProc = spawn(chromeExe, args, { stdio: 'ignore' });
  // Poll for the DevTools endpoint.
  for (let i = 0; i < 100; i++) {
    await sleep(200);
    try {
      const v = await getJson(`http://127.0.0.1:${PORT}/json/version`);
      return v;
    } catch { /* keep polling */ }
  }
  throw new Error('Chrome DevTools endpoint never came up');
}

// ---------------------------------------------------------------------------
// Main test flow.
// ---------------------------------------------------------------------------
async function main() {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'afm_wasm_smoke_'));
  stageTmp = tmp;
  const downloadDir = path.join(tmp, 'downloads');
  fs.mkdirSync(downloadDir, { recursive: true });

  // Copy the fixture to a plain Windows temp path (Chrome-friendly).
  const fixtureLocal = path.join(tmp, 'sample.atrview');
  fs.copyFileSync(FIXTURE_WIN, fixtureLocal);
  log(`fixture copied to ${fixtureLocal}`);

  const server = await serveDist();
  log(`static server on http://127.0.0.1:${HTTP_PORT}`);

  const version = await launchChrome();
  log('chrome launched');

  const browserWs = await connectWS(version.webSocketDebuggerUrl);
  const browser = new CDP(browserWs);
  await browser.send('Browser.setDownloadBehavior', {
    behavior: 'allow',
    downloadPath: downloadDir,
    eventsEnabled: true,
  });

  // New tab.
  const target = await newTab('about:blank');
  const pageWs = await connectWS(target.webSocketDebuggerUrl);
  const page = new CDP(pageWs);

  const consoleErrors = [];
  const exceptions = [];
  let downloads = [];

  page.on('Runtime.consoleAPICalled', (p) => {
    const type = p.type;
    const text = (p.args || []).map((a) => a.value ?? a.description ?? '').join(' ');
    if (type === 'error' || type === 'assert') consoleErrors.push(text);
    log(`[console.${type}] ${text}`);
  });
  page.on('Runtime.exceptionThrown', (p) => {
    const d = p.exceptionDetails?.exception?.description || JSON.stringify(p.exceptionDetails);
    exceptions.push(d);
    log(`[exception] ${d}`);
  });
  page.on('Page.loadEventFired', () => log('[page] load event fired'));
  page.on('Page.javascriptDialogOpening', () => log('[page] javascript dialog opening'));

  browser.on('Browser.downloadWillBegin', (p) => {
    downloads.push({ guid: p.guid, filename: p.suggestedFilename, state: 'begin', dir: downloadDir });
    log(`downloadWillBegin: ${p.suggestedFilename}`);
  });
  browser.on('Browser.downloadProgress', (p) => {
    const d = downloads.find((x) => x.guid === p.guid);
    if (d) { d.state = p.state; }
  });

  await page.send('Page.enable');
  await page.send('Runtime.enable');
  await page.send('DOM.enable');

  await page.send('Page.navigate', { url: `http://127.0.0.1:${HTTP_PORT}/index.html` });

  // Wait until the app reports 'ready'.
  let ready = false;
  for (let i = 0; i < 150; i++) {
    await sleep(200);
    const r = await page.send('Runtime.evaluate', {
      expression: "document.getElementById('status')?.textContent",
      returnByValue: true,
    });
    const status = r.result?.value;
    if (status === 'ready') { ready = true; break; }
    if (status && status.startsWith('failed')) fail(`page init failed: ${status}`);
  }
  if (!ready) {
    const diag = await page.send('Runtime.evaluate', {
      expression: "JSON.stringify({status: document.getElementById('status')?.textContent, hasAfm: typeof window.__afm, scripts: [...document.scripts].map(s=>s.src), body: document.body?.innerHTML?.slice(0,300)})",
      returnByValue: true,
    });
    fail(`app never became ready (timed out). diag=${diag.result?.value}`);
  }

  // 1. Slint rendered a canvas (may appear on the first event-loop tick).
  let canvas = null;
  for (let i = 0; i < 40; i++) {
    const r = await page.send('Runtime.evaluate', {
      expression: "(() => { const c = document.querySelector('canvas'); return c ? {w: c.width, h: c.height} : null; })()",
      returnByValue: true,
    });
    if (r.result?.value) { canvas = r.result.value; break; }
    await sleep(250);
  }
  if (!canvas) {
    const diag = await page.send('Runtime.evaluate', {
      expression: "JSON.stringify({status: document.getElementById('status')?.textContent, canvases: document.querySelectorAll('canvas').length, children: [...document.body.children].map(c=>c.tagName), body: document.body.innerHTML.slice(0,600)})",
      returnByValue: true,
    });
    fail(`no <canvas> element found — Slint did not render. diag=${diag.result?.value}`);
  }
  log(`canvas rendered (${canvas.w}x${canvas.h})`);

  // Helper: evaluate a harness expression synchronously (string/void).
  async function evalAfm(expr) {
    await page.send('Runtime.evaluate', { expression: expr, returnByValue: true });
  }
  // Helper: read the domain-state JSON from the harness.
  async function domain() {
    const r = await page.send('Runtime.evaluate', { expression: 'window.__afm.domainState()', returnByValue: true });
    return JSON.parse(r.result?.value ?? '{}');
  }

  // 2. Font banks + view dimensions present in the model (not an empty shell).
  const d0 = await domain();
  if (d0.font_count !== 4) fail(`expected 4 font banks, got ${d0.font_count}`);
  if (d0.view_width !== 40 || d0.view_height !== 26) fail(`expected 40x26 view, got ${d0.view_width}x${d0.view_height}`);
  log(`domain state: ${d0.font_count} font banks, view ${d0.view_width}x${d0.view_height}`);

  // 3. New Project -> default Atari glyphs visible in all 4 banks.
  await evalAfm('window.__afm.newProject()');
  const d1 = await domain();
  if (d1.glyph_nonzero < 100) fail(`default Atari glyphs missing after New Project (glyph_nonzero=${d1.glyph_nonzero})`);
  if (d1.selected_char !== 0) fail(`selected character not reset (${d1.selected_char})`);
  log(`new project: ${d1.glyph_nonzero} non-zero glyphs across 4 banks (default Atari font present)`);

  // 4. Character Editor: select character -> paint -> glyph mutates -> undo/redo.
  await evalAfm('window.__afm.selectCharacter(65)');
  let d4 = await domain();
  if (d4.selected_char !== 65) fail(`selectCharacter(65) selected ${d4.selected_char}`);
  const glyphHashBefore = d4.font_hash;
  await evalAfm('window.__afm.pixelClick(0, 0)');
  d4 = await domain();
  if (d4.font_hash === glyphHashBefore) fail('pixel edit did not mutate the glyph (Character Editor broken)');
  log(`character editor: pixel edit changed font_hash ${glyphHashBefore} -> ${d4.font_hash}`);
  await evalAfm('window.__afm.undo()');
  d4 = await domain();
  if (d4.font_hash !== glyphHashBefore) fail('undo did not restore the glyph');
  await evalAfm('window.__afm.redo()');
  d4 = await domain();
  if (d4.font_hash === glyphHashBefore) fail('redo did not reapply the glyph edit');
  log('character editor: undo/redo round-trip correct');

  // 5. View 40x26: placing the selected character mutates the view buffer.
  await evalAfm('window.__afm.selectCharacter(1)');
  let d5 = await domain();
  const viewHashBefore = d5.view_hash;
  await evalAfm('window.__afm.placeChar(0, 0)');
  d5 = await domain();
  if (d5.view_hash === viewHashBefore) fail('placing a character did not change the 40x26 view');
  if (d5.view_nonzero < 1) fail('view has no non-zero cells after placing a character');
  log(`view 40x26: placing char changed view_hash (${d5.view_nonzero} non-zero cells)`);

  // 6. Screenshot: capture the rendered UI and verify it is not blank.
  if (SCREENSHOT_OUT) {
    const shot = await page.send('Page.captureScreenshot', { format: 'png' });
    fs.writeFileSync(SCREENSHOT_OUT, Buffer.from(shot.data, 'base64'));
    const info = decodePng(fs.readFileSync(SCREENSHOT_OUT));
    if (info.uniqueColors < 8) fail(`screenshot looks blank (${info.uniqueColors} unique colours)`);
    log(`screenshot saved (${info.width}x${info.height}, ${info.uniqueColors} unique colours, ${info.nonZeroPixels} non-zero px)`);
  }

  // Helper: start an Open via picker (don't await the promise yet).
  async function beginOpen(accept) {
    const r = await page.send('Runtime.evaluate', {
      expression: `window.__afm.open(${JSON.stringify(accept)})`,
      awaitPromise: false,
    });
    if (!r.result?.objectId) fail('harness_open returned no promise object');
    return r.result.objectId;
  }
  async function awaitOpen(promiseId) {
    const r = await page.send('Runtime.awaitPromise', { promiseObjectId: promiseId, returnByValue: true });
    return r.result?.value;
  }
  async function setFiles(nodeId, files) {
    await page.send('DOM.setFileInputFiles', { nodeId, files });
  }
  async function fileInputNode() {
    const doc = await page.send('DOM.getDocument');
    const q = await page.send('DOM.querySelector', { nodeId: doc.root.nodeId, selector: 'input[type=file]' });
    return q.nodeId;
  }
  async function snapshot() {
    const r = await page.send('Runtime.evaluate', { expression: 'window.__afm.snapshot()', returnByValue: true });
    return r.result?.value;
  }

  // 2. Open the fixture.
  log('opening fixture via file picker (setInputFiles)');
  const p1 = await beginOpen('.atrview,.vf2,.vfn,.dat');
  await sleep(100);
  const node1 = await fileInputNode();
  await setFiles(node1, [fixtureLocal]);
  const opened = await awaitOpen(p1);
  if (opened !== 'opened') fail(`open did not resolve to "opened": ${opened}`);
  log('open resolved: opened');

  const snap0 = await snapshot();
  if (!snap0 || snap0.startsWith('{"error"')) fail(`snapshot after open is invalid: ${String(snap0).slice(0, 120)}`);
  const parsed0 = JSON.parse(snap0);
  const pages0 = parsed0?.View?.ViewLines?.length ?? parsed0?.View?.ViewPages?.length ?? null;
  log(`snapshot0 ok (JSON len=${snap0.length}, pages=${pages0})`);

  // 3. Save and capture the real browser download.
  log('saving via browser download');
  const beforeDownloads = downloads.length;
  await page.send('Runtime.evaluate', { expression: 'window.__afm.save()', returnByValue: true });
  // Wait for a completed download.
  let downloadedPath = null;
  for (let i = 0; i < 150; i++) {
    await sleep(200);
    const done = downloads.slice(beforeDownloads).find((d) => d.state === 'completed');
    if (done) {
      const candidate = path.join(downloadDir, done.filename);
      if (fs.existsSync(candidate)) { downloadedPath = candidate; break; }
      // Chrome may still hold the GUID-named file; find newest non-.crdownload file.
      const files = fs.readdirSync(downloadDir).filter((f) => !f.endsWith('.crdownload'));
      if (files.length) { downloadedPath = path.join(downloadDir, files[files.length - 1]); break; }
    }
  }
  if (!downloadedPath) fail('no completed browser download captured');
  const downloaded = fs.readFileSync(downloadedPath, 'utf8');
  log(`download captured: ${path.basename(downloadedPath)} (${downloaded.length} bytes)`);
  if (downloaded !== snap0) fail('downloaded bytes differ from in-memory snapshot');
  log('download bytes match snapshot exactly');

  // 4. Re-open the downloaded file; snapshot must be identical.
  log('re-opening the downloaded file');
  const p2 = await beginOpen('.atrview,.vf2,.vfn,.dat');
  await sleep(100);
  const node2 = await fileInputNode();
  await setFiles(node2, [downloadedPath]);
  const opened2 = await awaitOpen(p2);
  if (opened2 !== 'opened') fail(`re-open did not resolve to "opened": ${opened2}`);
  const snap1 = await snapshot();
  if (snap1 !== snap0) fail('round-trip snapshot mismatch (Open->Save->Open not lossless)');
  log('round-trip snapshot identical (lossless)');

  // 5. Cancel leaves state untouched.
  log('testing cancel');
  const p3 = await beginOpen('.atrview');
  await sleep(100);
  await page.send('Runtime.evaluate', {
    expression: "document.querySelector('input[type=file]').dispatchEvent(new Event('cancel'))",
    returnByValue: true,
  });
  const cancelled = await awaitOpen(p3);
  if (cancelled !== 'cancelled') fail(`cancel did not resolve to "cancelled": ${cancelled}`);
  const snap2 = await snapshot();
  if (snap2 !== snap1) fail('cancel mutated project state');
  log('cancel leaves state untouched');

  // 7. Resize: shrink the window; the app must stay responsive and not lose
  //    project state (Section 10).
  const beforeResize = await domain();
  const win = await browser.send('Browser.getWindowForTarget', { targetId: target.id });
  await browser.send('Browser.setWindowBounds', {
    windowId: win.windowId,
    bounds: { width: 820, height: 560 },
  });
  await sleep(800);
  const afterResize = await domain();
  if (afterResize.font_hash !== beforeResize.font_hash || afterResize.view_hash !== beforeResize.view_hash) {
    fail('resize lost project state');
  }
  const canvasStillThere = await page.send('Runtime.evaluate', {
    expression: "!!document.querySelector('canvas')",
    returnByValue: true,
  });
  if (!canvasStillThere.result?.value) fail('canvas disappeared after resize');
  log('resize: window resized without crash or state loss');

  // 8. No console errors / exceptions (ignore benign control-flow note if any).
  const realErrors = consoleErrors.filter((t) => !/Using exceptions for control flow/.test(t));
  const realEx = exceptions.filter((t) => !/Using exceptions for control flow/.test(t));
  if (realErrors.length) fail(`console errors: ${realErrors.join(' | ')}`);
  if (realEx.length) fail(`uncaught exceptions: ${realEx.join(' | ')}`);
  log('no console errors or uncaught exceptions');

  // Cleanup.
  try { pageWs.close(); browserWs.close(); } catch {}
  server.close();

  console.log('\n[smoke] ===============================');
  console.log('[smoke] PASS — real browser Open→Save→Open round-trip verified');
  console.log('[smoke] ===============================');
}

main()
  .then(() => process.exit(0))
  .catch((e) => {
    console.error(`[smoke] FAIL: ${e.message || e}`);
    process.exit(1);
  });

process.on('exit', () => {
  if (chromeProc) { try { chromeProc.kill(); } catch {} }
  if (stageTmp) { try { fs.rmSync(stageTmp, { recursive: true, force: true }); } catch {} }
  if (chromeProfile) { try { fs.rmSync(chromeProfile, { recursive: true, force: true }); } catch {} }
});
