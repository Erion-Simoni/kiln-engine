// Export and relocate a real authored physics game, then execute only its artifact.
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const http = require('node:http');
const os = require('node:os');
const path = require('node:path');
const {execFileSync} = require('node:child_process');
const {chromium} = require('playwright');
const repo = path.resolve(__dirname, '../..');
const runtime = path.resolve(process.argv[2] || path.join(repo, 'target/debug/web-runtime'));
const cli = path.resolve(process.env.KILN_CLI_PATH || path.join(repo, 'target/debug/kiln-cli'));
const evidence = path.resolve(process.env.WEB_PHYSICS_EVIDENCE || path.join(repo, 'target/task8-physics-evidence'));
const deploy = '/nested/physics/';
const slotKey = slot => `kiln:kiln-browser-physics:slot:${slot}`;
const component = (saved, id, type) => {
  const entity = saved.scene.entities.find(entity => entity.id === id);
  assert.ok(entity, `Missing ${id}`);
  const result = entity.components.find(component => component.type === type);
  assert.ok(result, `Missing ${type} on ${id}`);
  return result;
};
const pose = (saved, id) => component(saved, id, 'Transform');
const velocity = (saved, id) => component(saved, id, 'Physics').body.linear_velocity;
const distance = saved => Math.hypot(pose(saved, 'bob').x - pose(saved, 'anchor').x,
                                    pose(saved, 'bob').y - pose(saved, 'anchor').y);

function assertJoints(saved) {
  assert.ok(Math.abs(distance(saved) - 40) < 3, `Distance joint: ${distance(saved)}`);
  const anchor = pose(saved, 'hinge_anchor'), bob = pose(saved, 'hinge_bob');
  const error = Math.hypot(bob.x - Math.sin(bob.rotation)*30 - anchor.x,
                           bob.y + Math.cos(bob.rotation)*30 - anchor.y);
  assert.ok(error < 3, `Revolute joint: ${error}`);
  const slider = pose(saved, 'slider');
  assert.ok(Math.abs(slider.y-120) < 3 && slider.x >= 557 && slider.x <= 603, `Prismatic joint: ${JSON.stringify(slider)}`);
}

(async () => {
  let browser, server;
  await fs.mkdir(evidence, {recursive:true});
  await fs.rm(path.join(evidence, 'result.json'), {force:true});
  try {
    const work = await fs.mkdtemp(path.join(os.tmpdir(), 'kiln-browser-physics-'));
    const project = path.join(work, 'authoring project');
    await fs.cp(path.join(repo, 'samples/browser_physics2d'), project, {recursive:true});
    const output = path.join(work, 'exported');
    const exportLog = execFileSync(cli, ['export', '--platform', 'web', '--project', path.join(project, 'kiln.toml'),
      '--web-runtime', runtime, '--output', output], {env:{...process.env, PATH:''}, cwd:work, encoding:'utf8'});
    await fs.writeFile(path.join(evidence, 'export.log'), exportLog);
    const receipt = JSON.parse(await fs.readFile(path.join(output, 'export.json')));
    assert.equal(receipt.runtime.physics, true);
    const relocated = path.join(await fs.mkdtemp(path.join(os.tmpdir(), 'kiln-relocated-physics-')), 'game folder');
    await fs.rename(output, relocated);
    await fs.rename(project, path.join(work, 'source-unavailable'));
    server = http.createServer(async (request, response) => {
      try {
        const url = new URL(request.url, 'http://localhost');
        if (!url.pathname.startsWith(deploy)) { response.writeHead(404); return response.end(); }
        const file = path.resolve(relocated, decodeURIComponent(url.pathname.slice(deploy.length) || 'index.html'));
        if (!file.startsWith(relocated + path.sep)) { response.writeHead(403); return response.end(); }
        response.setHeader('Content-Type', ({'.html':'text/html', '.js':'text/javascript', '.wasm':'application/wasm', '.png':'image/png'})[path.extname(file)] || 'text/plain');
        response.end(await fs.readFile(file));
      } catch { response.writeHead(404); response.end(); }
    });
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    browser = await chromium.launch({executablePath:process.env.CHROMIUM_PATH || undefined, headless:true,
      args:['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--disable-webgpu']});
    const page = await browser.newPage({viewport:{width:800, height:500}});
    const errors = [], requests = [];
    page.on('pageerror', error => errors.push(String(error)));
    page.on('console', message => {
      if (message.type() === 'error') errors.push(message.text());
      if (process.env.WEB_PHYSICS_DEBUG) console.error('[browser]', message.type(), message.text());
    });
    page.on('response', response => { if (response.status() >= 400) requests.push(response.url()); });
    const url = `http://127.0.0.1:${server.address().port}${deploy}`;
    async function start() {
      await page.goto(url);
      await page.waitForFunction(() => ['ready','error'].includes(document.querySelector('#gate').dataset.state), null, {timeout:60000});
      assert.equal(await page.locator('#gate').getAttribute('data-state'), 'ready', await page.locator('#status').textContent());
      await page.locator('#start').click();
      await page.waitForFunction(() => document.querySelector('#gate').hidden);
    }
    async function hold(key, milliseconds = 70) {
      await page.keyboard.down(key);
      try { await page.waitForTimeout(milliseconds); } finally { await page.keyboard.up(key); }
      await page.waitForTimeout(40);
    }
    async function capture(key = 's', slot = 0) {
      const old = await page.evaluate(key => localStorage.getItem(key), slotKey(slot));
      await hold(key);
      await page.waitForFunction(({key, old}) => { const value=localStorage.getItem(key); return value !== null && value !== old; },
        {key:slotKey(slot), old}, {timeout:10000});
      return page.evaluate(key => JSON.parse(localStorage.getItem(key)), slotKey(slot));
    }
    async function heroPixels(name) {
      const png = await page.locator('#game').screenshot({path:path.join(evidence,name+'.png')});
      return page.evaluate(async base64 => {
        const image = new Image(); image.src = 'data:image/png;base64,'+base64; await image.decode();
        const canvas = document.createElement('canvas'); canvas.width=image.width; canvas.height=image.height;
        const ctx=canvas.getContext('2d'); ctx.drawImage(image,0,0);
        const data=ctx.getImageData(0,0,image.width,image.height).data;
        let count=0,x=0,y=0;
        for(let i=0;i<data.length;i+=4) {
          const [r,g,b]=data.subarray(i,i+3);
          if(b>200 && r<120 && g>130 && g<240 && b>g*1.1) {
            count++; x+=(i/4)%image.width; y+=Math.floor(i/4/image.width);
          }
        }
        return {count,x:x/count,y:y/count};
      }, png.toString('base64'));
    }
    await start();
    await page.waitForTimeout(1100);
    const settled = await capture();
    await fs.writeFile(path.join(evidence, 'settled.json'), JSON.stringify(settled, null, 2));
    assert.equal(settled.scene.id, 'main');
    assert.ok(Math.abs(pose(settled, 'hero').y - 30) < 2, `Gravity/floor/filtering: ${pose(settled,'hero').y}`);
    assert.ok(Math.abs(pose(settled, 'tile_ball').y - 48) < 2, `Solid tile collision: ${pose(settled,'tile_ball').y}`);
    assert.ok(Math.abs(velocity(settled, 'hero')[1]) < 2);
    assert.ok(settled.game_data.sensor_enters.Integer >= 1);
    assert.ok(settled.game_data.sensor_exits.Integer >= 1);
    assert.equal(settled.game_data.ground_hit.Array[0].Text, 'floor');
    assert.equal(settled.game_data.tile_hit.Array[0].Text, 'tiles');
    assert.ok(pose(settled, 'mover').x > 310, 'Kinematic body must advance');
    assertJoints(settled);
    assert.ok(pose(settled,"slider").x > 581, "Prismatic motor must advance");
    const settledPixels = await heroPixels('settled');
    assert.ok(settledPixels.count > 150 && Math.abs(settledPixels.x-100)<3 && Math.abs(settledPixels.y-330)<3, `Visible grounded hero: ${JSON.stringify(settledPixels)}`);
    await hold('j', 35);
    const checkpoint = await capture('c', 1);
    await fs.writeFile(path.join(evidence, 'checkpoint.json'), JSON.stringify(checkpoint, null, 2));
    assert.ok(velocity(checkpoint, 'hero')[1] > 100, 'Checkpoint must contain upward body velocity');
    assert.ok(pose(checkpoint, 'hero').y > 35);
    assert.equal(checkpoint.game_data.hero_starts.Integer, 1);
    await page.reload();
    await page.waitForFunction(() => document.querySelector('#gate').dataset.state === 'ready');
    await page.locator('#start').click();
    await page.waitForFunction(() => document.querySelector('#gate').hidden);
    await hold('l', 35);
    const restored = await capture();
    await fs.writeFile(path.join(evidence, 'restored.json'), JSON.stringify(restored, null, 2));
    assert.equal(restored.scene.id, 'main');
    assert.equal(restored.game_data.hero_starts.Integer, 1, 'Attached Start must not replay on load');
    assert.ok(pose(restored, 'hero').y >= pose(checkpoint, 'hero').y - 5);
    assert.ok(pose(restored, 'hero').y < pose(checkpoint, 'hero').y + 65, 'Restore must use saved transform/velocity');
    assert.ok(velocity(restored, 'hero')[1] > 0, 'Restored physics body must retain upward motion');
    assertJoints(restored);
    await page.locator('#game').screenshot({path:path.join(evidence, 'restored.png')});
    await page.waitForTimeout(1100);
    const landed = await capture();
    assert.ok(Math.abs(pose(landed,'hero').y - 30) < 2, 'Restored world must rebuild collision geometry');
    assert.ok(Math.abs(pose(landed,'tile_ball').y - 48) < 2);
    await hold('n');
    await page.waitForTimeout(700);
    const next = await capture();
    await fs.writeFile(path.join(evidence,'next-scene.json'), JSON.stringify(next,null,2));
    assert.equal(next.scene.id, 'next');
    assert.ok(Math.abs(pose(next,'hero').y - 130) < 2);
    assert.equal(next.game_data.ground_hit.Boolean, false, 'Old floor collider must not survive transition');
    assert.ok(!next.scene.entities.some(entity => entity.id === 'bob'), 'Old joint/body must be gone');
    const nextPixels = await heroPixels('next-scene');
    assert.ok(nextPixels.count>150 && Math.abs(nextPixels.y-230)<4, `Visible next-scene hero: ${JSON.stringify(nextPixels)}`);
    await hold('l', 35);
    const crossScene = await capture();
    assert.equal(crossScene.scene.id, 'main', 'Loading must restore saved physics scene');
    assert.ok(crossScene.scene.entities.some(entity => entity.id === 'rope'));
    assertJoints(crossScene);
    if (process.env.WEB_PHYSICS_MINIMAL_RUNTIME) {
      const minimal = path.resolve(process.env.WEB_PHYSICS_MINIMAL_RUNTIME);
      const disabledOutput = path.join(work, 'disabled-export');
      const source = path.join(work, 'source-unavailable/kiln.toml');
      let rejected = false;
      try {
        execFileSync(cli, ['export','--platform','web','--project',source,'--web-runtime',minimal,'--output',disabledOutput],
          {env:{...process.env,PATH:''},cwd:work,stdio:'pipe'});
      } catch (error) {
        rejected = String(error.stderr).includes('lacks physics support');
      }
      assert.ok(rejected, 'No-physics prepared runtime must reject authored physics');
      await assert.rejects(fs.stat(disabledOutput), {code:'ENOENT'});
      // Direct API/bootstrap users also get a useful error if they bypass export validation.
      const unsupported = await browser.newPage();
      await unsupported.route('**/runtime/*', async route => {
        const name = path.basename(new URL(route.request().url()).pathname);
        if (['kiln_web.js','kiln_web_bg.wasm'].includes(name)) {
          await route.fulfill({path:path.join(minimal,name),contentType:name.endsWith('.js')?'text/javascript':'application/wasm'});
        } else await route.continue();
      });
      await unsupported.goto(url);
      await unsupported.waitForFunction(() => document.querySelector('#gate').dataset.state === 'error');
      assert.match(await unsupported.locator('#status').textContent(), /lacks physics/);
      await unsupported.screenshot({path:path.join(evidence,'missing-physics.png')});
      await unsupported.close();
    }
    assert.deepEqual(requests, []);
    assert.deepEqual(errors, []);
    const result = {passed:true, runtime_physics:receipt.runtime.physics, source_project_unavailable:true,
      export_tools_path_empty:true, relocated_nested_http:true, gravity_collision_filter_sensor_raycast:true,
      tilemap_collisions:true, kinematic_body:true, distance_revolute_prismatic_joints:true, minimal_runtime_rejected:!!process.env.WEB_PHYSICS_MINIMAL_RUNTIME, checkpoint_reload_velocity_and_start_once:true,
      restored_colliders:true, scene_transition_clears_bodies_joints:true, cross_scene_load:true,
      rendered_hero_pixels:{settled:settledPixels,next:nextPixels}, settled_hero_y:pose(settled,'hero').y, checkpoint_velocity:velocity(checkpoint,'hero'),
      restored_velocity:velocity(restored,'hero'), next_scene_hero_y:pose(next,'hero').y, errors};
    await fs.writeFile(path.join(evidence,'result.json'), JSON.stringify(result,null,2)+'\n');
    console.log(JSON.stringify(result));
  } finally {
    if (browser) await browser.close();
    if (server) await new Promise(resolve => server.close(resolve));
  }
})().catch(error => { console.error(error); process.exitCode=1; });
