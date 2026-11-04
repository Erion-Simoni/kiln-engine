// Browser regression: load real WASM/assets, render the hero, and move it with input.
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const http = require('node:http');
const path = require('node:path');
const { chromium } = require('playwright');

const pkg = process.argv[2] ? path.resolve(process.argv[2]) : path.resolve(__dirname, '../../target/web-smoke');
const game = path.resolve(__dirname, '../../samples/vertical_slice');
const html = `<!doctype html><html><head><link rel="icon" href="data:,"></head>
<body style="margin:0"><canvas id="game" style="width:640px;height:480px"></canvas>
<script type="module">
import init, {run} from '/pkg/kiln_web.js';
window.ready = (async () => {await init(); console.log('WASM initialized'); await run('game', '/game/kiln.toml'); console.log('Game initialized');})();
</script></body></html>`;

const server = http.createServer(async (req, res) => {
  try {
    const url = new URL(req.url, 'http://localhost');
    if (url.pathname === '/') { res.setHeader('Content-Type', 'text/html'); return res.end(html); }
    const match = url.pathname.match(/^\/(pkg|game)\/(.+)$/);
    if (!match) { res.writeHead(404); return res.end(); }
    const base = match[1] === 'pkg' ? pkg : game;
    const file = path.resolve(base, decodeURIComponent(match[2]));
    if (!file.startsWith(base + path.sep)) { res.writeHead(403); return res.end(); }
    const data = await fs.readFile(file);
    const mime = {'.js':'text/javascript', '.wasm':'application/wasm', '.png':'image/png'};
    res.setHeader('Content-Type', mime[path.extname(file)] || 'text/plain');
    res.end(data);
  } catch { res.writeHead(404); res.end(); }
});

// Decode a browser screenshot through a separate 2D canvas, not the game's GPU canvas.
async function heroPosition(page) {
  const png = await page.locator('#game').screenshot();
  if (process.env.WEB_SMOKE_SCREENSHOT) await fs.writeFile(process.env.WEB_SMOKE_SCREENSHOT, png);
  return page.evaluate(async (base64) => {
    const image = new Image(); image.src = 'data:image/png;base64,' + base64; await image.decode();
    const canvas = document.createElement('canvas'); canvas.width = image.width; canvas.height = image.height;
    const ctx = canvas.getContext('2d'); ctx.drawImage(image, 0, 0);
    const data = ctx.getImageData(0, 0, image.width, image.height).data;
    let count = 0, x = 0;
    for (let i = 0; i < data.length; i += 4) {
      const [r, g, b] = data.subarray(i, i + 3);
      if (b > 100 && b > r * 1.3 && b > g * 1.1) { count++; x += (i / 4) % image.width; }
    }
    return {count, x: x / count};
  }, png.toString('base64'));
}

(async () => {
  let browser;
  try {
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    browser = await chromium.launch({
      ...(process.env.CHROMIUM_PATH ? {executablePath: process.env.CHROMIUM_PATH} : {}),
      headless: true,
      args: ['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--disable-webgpu'],
    });
    const page = await browser.newPage({viewport:{width:640,height:480}});
    const errors = [], failedRequests = [];
    page.on('pageerror', error => { errors.push(String(error)); console.error('[page error]', String(error)); });
    page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); if (process.env.WEB_SMOKE_DEBUG) console.error('[browser]', message.type(), message.text()); });
    page.on('response', response => { if (response.status() >= 400) failedRequests.push(response.url()); });
    await page.goto(`http://127.0.0.1:${server.address().port}/`);
    await page.waitForFunction(() => window.ready !== undefined);
    let initTimer;
    try {
      await Promise.race([
        page.evaluate(() => window.ready),
        new Promise((_, reject) => { initTimer = setTimeout(() => reject(new Error('Browser initialization timed out: ' + errors.join('\n'))), 30000); }),
      ]);
    } finally { clearTimeout(initTimer); }
    const rejectsMissingCanvas = await page.evaluate(async () => {
      const {run} = await import('/pkg/kiln_web.js');
      try { await run('missing-canvas', '/game/kiln.toml'); return false; }
      catch (error) { return String(error).includes('Canvas element not found'); }
    });
    assert.ok(rejectsMissingCanvas, 'Invalid canvas must reject startup');
    await page.waitForTimeout(300);
    const before = await heroPosition(page);
    assert.ok(before.count > 20, `Hero pixels not rendered: ${JSON.stringify({before, failedRequests, errors})}`);
    await page.keyboard.down('d');
    let after;
    try {
      const deadline = Date.now() + 10000;
      do {
        await page.waitForTimeout(50);
        after = await heroPosition(page);
      } while (after.x <= before.x + 10 && Date.now() < deadline);
    } finally { await page.keyboard.up('d'); }
    assert.ok(after.count > 20, 'Hero disappeared after input');
    assert.ok(after.x > before.x + 10, `Keyboard did not move the hero: ${before.x} -> ${after.x}`);
    // Establish a post-keyup frame before measuring whether motion stopped;
    // the last held-key screenshot may finish several frames before keyup.
    await page.waitForTimeout(100);
    const released = await heroPosition(page);
    await page.waitForTimeout(150);
    const stopped = await heroPosition(page);
    assert.ok(Math.abs(stopped.x - released.x) <= 2, `Hero kept moving after key release: ${released.x} -> ${stopped.x}`);
    assert.deepEqual(failedRequests, [], 'Assets failed to load');
    assert.deepEqual(errors, [], 'Browser runtime errors');
    console.log(JSON.stringify({passed:true, before, after, stopped}));
  } finally {
    if (browser) await browser.close();
    await new Promise(resolve => server.close(resolve));
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
