// Play only the editor-produced exports. No engine hooks or injected game state.
const assert=require('node:assert/strict');
const fs=require('node:fs/promises');
const http=require('node:http');
const path=require('node:path');
const {chromium}=require('playwright');
const reportFile=path.resolve(process.argv[2]||'target/task9-replay-final/result.json');
const output=path.resolve(process.argv[3]||'target/task9-browser-play');
(async()=>{
 let browser,server;
 try{
  const report=JSON.parse(await fs.readFile(reportFile,'utf8'));
  assert.equal(report.games.length,3,'All three editor exports are required');
  const roots=new Map(report.games.map(item=>[item.export.game,path.resolve(item.export.outputs.web)]));
  server=http.createServer(async(req,res)=>{
   try{
    const url=new URL(req.url,'http://localhost');
    const match=url.pathname.match(/^\/games\/reference\/(emberwatch|switchyard|skybridge)\/(.*)$/);
    if(!match){res.writeHead(404);return res.end();}
    const root=roots.get(match[1]);const file=path.resolve(root,decodeURIComponent(match[2]||'index.html'));
    if(!file.startsWith(root+path.sep)){res.writeHead(403);return res.end();}
    res.setHeader('Content-Type',({'.html':'text/html','.js':'text/javascript','.wasm':'application/wasm','.png':'image/png','.wav':'audio/wav','.ttf':'font/ttf'})[path.extname(file)]||'text/plain');
    res.end(await fs.readFile(file));
   }catch{res.writeHead(404);res.end();}
  });
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  await fs.mkdir(output,{recursive:true});
  browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH||undefined,headless:true,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader','--disable-webgpu']});
  const context=await browser.newContext({viewport:{width:1000,height:650}});
  await context.addInitScript(()=>{
   window.musicStarts=0;
   const create=AudioContext.prototype.createBufferSource;
   AudioContext.prototype.createBufferSource=function(){const source=create.call(this);const start=source.start;source.start=function(...args){if(this.loop)window.musicStarts++;return start.apply(this,args)};return source;};
  });
  const results=[];
  for(const name of ['emberwatch','switchyard','skybridge']){
   const page=await context.newPage();const errors=[];
   page.on('pageerror',error=>errors.push(String(error)));
   page.on('console',message=>{if(message.type()==='error')errors.push(message.text());});
   const evidence=path.join(output,name);await fs.mkdir(evidence,{recursive:true});
   const key=`kiln:kiln-reference-${name}:slot:0`;
   const slot=()=>page.evaluate(key=>{const value=localStorage.getItem(key);return value?JSON.parse(value):null;},key);
   const shot=async label=>page.locator('#game').screenshot({path:path.join(evidence,label+'.png')});
   const click=async(x,y)=>{const box=await page.locator('#game').boundingBox();await page.mouse.click(box.x+x*box.width/640,box.y+y*box.height/360);await page.waitForTimeout(160);};
   const press=async(key,ms=180)=>{await page.keyboard.down(key);await page.waitForTimeout(ms);await page.keyboard.up(key);await page.waitForTimeout(120);};
   async function open(){
    await page.goto(`http://127.0.0.1:${server.address().port}/games/reference/${name}/`);
    await page.waitForFunction(()=>['ready','error'].includes(document.getElementById('gate').dataset.state));
    assert.equal(await page.locator('#gate').getAttribute('data-state'),'ready',await page.locator('#status').textContent());
    assert.equal(await page.evaluate(()=>window.musicStarts),0,'Preparing the export must not play audio');
    await page.locator('#start').click();await page.waitForFunction(()=>document.getElementById('gate').hidden);await page.waitForTimeout(250);
   }
   await open();await shot('title');
   let result={game:name,editor_export:true,nested_http_deployment:true};
   if(name==='switchyard'){
    await click(320,180);
    for(const [stage,solution] of [[1,[0,1,2]],[2,[1,2]],[3,[0,2]]]){
     let selected=0;
     for(const target of solution){while(selected!==target){await press('ArrowRight');selected=(selected+1)%3;}await click(320,276);}
     await page.waitForFunction(({key,stage})=>{const data=JSON.parse(localStorage.getItem(key)||'null');return data?.game_data.stage.Integer===stage&&data?.game_data.outcome.Text==='won';},{key,stage});
     const saved=await slot();assert.equal(saved.game_data.moves.Integer,solution.length);
     for(let index=0;index<5;index++)assert.equal(saved.game_data['l'+index].Integer,1);
     await shot('circuit_'+stage+'_won');
     if(stage===2){await open();await click(320,276);await page.waitForTimeout(300);await shot('reloaded_continue');}
     if(stage<3)await click(320,180);
    }
    const saved=await slot();assert.equal(saved.game_data.stage.Integer,3);assert.equal(saved.game_data.moves.Integer,2);
    result={...result,three_circuits_won:true,reload_continue:true,stage:3,moves:2};
   }else if(name==='emberwatch'){
    await click(320,180);
    for(let index=0;index<14;index++){await click(320,276);await page.waitForTimeout(650);}
    await shot('hostiles_cleared');await press('ArrowRight',2800);
    await page.waitForFunction(key=>JSON.parse(localStorage.getItem(key)||'null')?.game_data.outcome.Text==='won',key);
    const saved=await slot();assert.equal(saved.game_data.cleared.Integer,3);assert.ok(saved.game_data.health.Integer>0);
    await shot('relay_secured');result={...result,all_hostiles_defeated:true,relay_trigger_won:true,victory_saved:true};
   }else{
    await page.keyboard.down('ArrowRight');let second=false;
    for(let index=0;index<12;index++){
     await press('ArrowUp',120);await page.waitForTimeout(300);await press('s',90);
     const saved=await slot();if(saved?.scene.id==='scene_1'){second=true;break;}
    }
    await page.keyboard.up('ArrowRight');assert.ok(second,'Physical gate did not transition to level two');
    const before=await slot();assert.equal(before.game_data.stage.Float,2);await shot('level_two');
    await press('ArrowRight',400);await press('l');await page.waitForTimeout(200);await press('s');
    const resumed=await slot();assert.equal(resumed.scene.id,'scene_1');
    const heroX=data=>data.scene.entities.find(entity=>entity.id==='hero').components.find(component=>component.type==='Transform').x;
    assert.ok(Math.abs(heroX(resumed)-heroX(before))<4,'Load did not restore the hero position');
    await page.keyboard.down('ArrowRight');let won=false;
    for(let index=0;index<16;index++){
     await press('ArrowUp',120);await page.waitForTimeout(300);await press('s',90);
     if((await slot())?.game_data.won.Boolean){won=true;break;}
    }
    await page.keyboard.up('ArrowRight');assert.ok(won,'Second physical gate did not win');await shot('won');
    result={...result,two_physical_levels_won:true,save_load_restores_position:true};
   }
   if(name!=='skybridge')assert.ok(await page.evaluate(()=>window.musicStarts)>0,'Game music was not started by consent');
   assert.deepEqual(errors,[],'Browser game errors');results.push(result);
   await fs.writeFile(path.join(output,'result.json'),JSON.stringify({scope:'Chromium/SwiftShader; no physical-device claim',games:results},null,2)+'\n');
   console.log(name+' browser gameplay passed');await page.close();
  }
 }finally{if(browser)await browser.close();if(server)await new Promise(resolve=>server.close(resolve));}
})().catch(error=>{console.error(error);process.exitCode=1;});
