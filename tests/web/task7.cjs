// Real browser Task 7 menus, font/audio integration, controller input and save/resume.
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const http = require('node:http');
const path = require('node:path');
const {chromium} = require('playwright');
const pkg = path.resolve(process.argv[2] || 'target/web-smoke');
const game = path.resolve('samples/ui2d_editor');
const output = path.resolve(process.env.WEB_TASK7_EVIDENCE || 'target/task7-web-evidence');
const html = `<!doctype html><html><head><link rel="icon" href="data:,"></head><body style="margin:0;padding:30px 0 0 70px;background:#161616"><canvas id="game" style="width:800px;height:450px;touch-action:none"></canvas><script type="module">import init,{run} from '/pkg/kiln_web.js';window.ready=(async()=>{await init();await run('game','/game/kiln.toml')})();</script></body></html>`;
const server = http.createServer(async(req,res)=>{
  try {const url=new URL(req.url,'http://localhost');if(url.pathname==='/'){res.setHeader('Content-Type','text/html');return res.end(html);}
    const match=url.pathname.match(/^\/(pkg|game)\/(.+)$/);if(!match){res.writeHead(404);return res.end();}
    const base=match[1]==='pkg'?pkg:game;const file=path.resolve(base,decodeURIComponent(match[2]));if(!file.startsWith(base+path.sep)){res.writeHead(403);return res.end();}
    res.setHeader('Content-Type',({'.js':'text/javascript','.wasm':'application/wasm','.png':'image/png','.wav':'audio/wav','.ttf':'font/ttf'})[path.extname(file)]||'text/plain');res.end(await fs.readFile(file));
  }catch{res.writeHead(404);res.end();}
});
(async()=>{let browser;try{
  await fs.mkdir(output,{recursive:true});await fs.rm(path.join(output,'result.json'),{force:true});
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH || undefined,headless:true,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader','--disable-webgpu']});
  const context=await browser.newContext({viewport:{width:1000,height:650},hasTouch:true});
  await context.addInitScript(()=>{
    window.audioAudit={starts:[],gains:[],contexts:[]};
    const createSource=AudioContext.prototype.createBufferSource;
    AudioContext.prototype.createBufferSource=function(){window.audioAudit.contexts.push(this);const source=createSource.call(this);const start=source.start;source.start=function(...args){window.audioAudit.starts.push({loop:this.loop,time:this.context.currentTime,offset:args[1]||0});return start.apply(this,args)};return source;};
    const createGain=AudioContext.prototype.createGain;
    AudioContext.prototype.createGain=function(){const node=createGain.call(this);window.audioAudit.gains.push(node);return node;};
    window.padButtons=Array.from({length:17},()=>false);
    const pad=Object.create(Gamepad.prototype);Object.defineProperties(pad,{connected:{value:true},axes:{get:()=>[0,0,0,0]},buttons:{get:()=>window.padButtons.map(pressed=>{const b=Object.create(GamepadButton.prototype);Object.defineProperties(b,{pressed:{value:pressed},value:{value:pressed?1:0},touched:{value:pressed}});return b;})}});
    navigator.getGamepads=()=>[pad];
  });
  const page=await context.newPage();const errors=[],requests=[],warnings=[];
  page.on('pageerror',error=>errors.push(String(error)));page.on('console',message=>{if(message.type()==='error')errors.push(message.text());if(message.type()==='warning')warnings.push(message.text());if(process.env.WEB_TASK7_DEBUG)console.error('[browser]',message.type(),message.text());});
  page.on('response',r=>{if(r.status()>=400)requests.push(r.url());});
  async function open(){await page.goto(`http://127.0.0.1:${server.address().port}/`);await page.waitForFunction(()=>window.ready!==undefined);await page.evaluate(()=>window.ready);await page.waitForTimeout(250);}
  async function shot(name){await page.waitForTimeout(120);await page.locator('#game').screenshot({path:path.join(output,`${name}.png`)});}
  async function click(x,y){const box=await page.locator('#game').boundingBox();await page.mouse.click(box.x+x*box.width/640,box.y+y*box.height/360);await page.waitForTimeout(120);}
  async function pad(index){await page.evaluate(index=>window.padButtons[index]=true,index);await page.waitForTimeout(90);await page.evaluate(index=>window.padButtons[index]=false,index);await page.waitForTimeout(90);}
  const slotKey='kiln:saved-ui-2d:slot:0';const prefsKey='kiln:saved-ui-2d:preferences';
  async function slot(){return page.evaluate(key=>JSON.parse(localStorage.getItem(key)),slotKey);}
  async function waitSaved(){await page.waitForFunction(key=>localStorage.getItem(key)!==null,slotKey);}
  await open();await shot('title');
  // D-pad and A use the actual browser Gamepad API, not injected engine events.
  await pad(13);await pad(0);await shot('controller_settings');
  const beforeSound=await page.evaluate(()=>audioAudit.starts.filter(s=>!s.loop).length);
  const box=await page.locator('#game').boundingBox();await page.touchscreen.tap(box.x+320*1.25,box.y+180*1.25);await page.waitForTimeout(160);await shot('settings_changed');
  const audio=await page.evaluate(()=>({music:audioAudit.starts.some(s=>s.loop),sounds:audioAudit.starts.filter(s=>!s.loop).length,contexts:audioAudit.contexts.map(c=>c.state),gains:audioAudit.gains.map(g=>g.gain.value)}));
  assert.equal(audio.sounds,beforeSound+1,'One touch must generate exactly one button event');assert.ok(audio.music);assert.ok(audio.contexts.includes('running'));
  const preferences=await page.evaluate(key=>JSON.parse(localStorage.getItem(key)),prefsKey);
  assert.equal(preferences.audio.master_volume,.25);assert.equal(preferences.values.menu_setting,'saved');
  await page.keyboard.press('Escape');await page.keyboard.press('ArrowUp');await page.keyboard.press('Enter');await page.waitForTimeout(200);await shot('gameplay');
  await click(320,276); // Damage: two simultaneous effects, custom health is now 66.
  await page.keyboard.press('Escape');await page.waitForTimeout(80);await click(320,304);await waitSaved();await shot('saved_pause');
  const saved=await slot();assert.equal(saved.version,1);assert.equal(saved.game_data.health.Integer,66);assert.equal(saved.game_data.inventory.Array[0].Text,'brass_key');assert.equal(saved.ui.active,'pause');assert.equal(saved.attached.ui.memory.values.starts.Integer,1);
  const savedActor=saved.scene.entities.find(e=>e.id==='player').components.find(c=>c.type==='Transform').x;
  assert.ok(savedActor>40,'Real gameplay must advance from the authored x=40 before saving');
  const simultaneous=await page.evaluate(()=>audioAudit.starts.filter(s=>!s.loop).slice(-2));assert.equal(simultaneous.length,2);assert.ok(Math.abs(simultaneous[0].time-simultaneous[1].time)<.02);
  // New player settings must survive loading an older gameplay snapshot.
  await page.evaluate(key=>{const p=JSON.parse(localStorage.getItem(key));p.audio.master_volume=.6;localStorage.setItem(key,JSON.stringify(p));},prefsKey);
  await open();await click(320,276);await page.waitForTimeout(120);await page.keyboard.press('Escape');await page.waitForTimeout(120);await page.keyboard.press('F5');await page.waitForTimeout(120);await shot('resumed_pause');
  const resumed=await slot();assert.equal(resumed.game_data.health.Integer,66);assert.equal(resumed.attached.ui.memory.values.starts.Integer,1);assert.ok(resumed.attached.ui.memory.values.ticks.Integer>=saved.attached.ui.memory.values.ticks.Integer);
  const resumedActor=resumed.scene.entities.find(e=>e.id==='player').components.find(c=>c.type==='Transform').x;assert.ok(resumedActor>=savedActor&&resumedActor-savedActor<100);
  assert.equal(await page.evaluate(key=>JSON.parse(localStorage.getItem(key)).audio.master_volume,prefsKey),.6);
  // Quota failures preserve both the old slot and the currently running game.
  const beforeQuota=await page.evaluate(key=>localStorage.getItem(key),slotKey);
  await page.evaluate(()=>{window.originalSetItem=Storage.prototype.setItem;Storage.prototype.setItem=function(key,value){if(key.includes(':slot:'))throw new DOMException('Test quota exhausted','QuotaExceededError');return window.originalSetItem.call(this,key,value)};});
  await click(320,304);await page.waitForTimeout(160);await shot('save_error');assert.equal(await page.evaluate(key=>localStorage.getItem(key),slotKey),beforeQuota);assert.ok(warnings.some(w=>w.includes('quota')));
  await page.evaluate(()=>Storage.prototype.setItem=window.originalSetItem);
  // Corrupt saves report failure without clearing or rewriting the slot.
  await page.evaluate(key=>localStorage.setItem(key,'broken'),slotKey);await open();await page.keyboard.press('F9');await page.waitForTimeout(160);await shot('load_error');assert.equal(await page.evaluate(key=>localStorage.getItem(key),slotKey),'broken');
  assert.deepEqual(errors,[]);assert.deepEqual(requests,[]);
  const result={passed:true,controller_navigation:true,touch_once:true,audio,health:saved.game_data.health.Integer,saved_actor_x:savedActor,resumed_actor_x:resumedActor,script_starts:resumed.attached.ui.memory.values.starts.Integer,preferences_independent:true,quota_preserves_slot:true,corrupt_load_preserves_slot:true};
  await fs.writeFile(path.join(output,'result.json'),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify(result));
}catch(error){console.error(error);process.exitCode=1;}finally{if(browser)await browser.close();server.close();}})();
