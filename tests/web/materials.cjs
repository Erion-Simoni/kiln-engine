// Real relocated browser material pixels; named constants/textures, order and restoration.
const assert=require('node:assert/strict'),fs=require('node:fs/promises'),http=require('node:http'),path=require('node:path'),os=require('node:os');
const {execFileSync}=require('node:child_process');const {chromium}=require('playwright');
const repo=path.resolve(__dirname,'../..'),runtime=path.resolve(process.argv[2]||'target/debug/web-runtime');
const evidence=path.resolve(process.env.WEB_MATERIAL_EVIDENCE||'target/task8-material-evidence');
(async()=>{let browser,server;try {
 await fs.mkdir(evidence,{recursive:true});await fs.rm(path.join(evidence,'result.json'),{force:true});
 const work=await fs.mkdtemp(path.join(os.tmpdir(),'kiln-materials-')),project=path.join(work,'project'),output=path.join(work,'export');
 await fs.cp(path.join(repo,'samples/materials2d'),project,{recursive:true});
 const cli=path.resolve(process.env.KILN_CLI_PATH||'target/debug/kiln-cli');
 const log=execFileSync(cli,['export','--platform','web','--project',path.join(project,'kiln.toml'),'--web-runtime',runtime,'--output',output],{cwd:work,env:{...process.env,PATH:''},encoding:'utf8'});
 await fs.writeFile(path.join(evidence,'export.log'),log);
 const receipt=JSON.parse(await fs.readFile(path.join(output,'export.json')));assert.equal(receipt.runtime.materials,true);
 const moved=path.join(await fs.mkdtemp(path.join(os.tmpdir(),'kiln-relocated-materials-')),'game folder');await fs.rename(output,moved);await fs.rename(project,path.join(work,'source-hidden'));
 const deploy='/nested/materials/';server=http.createServer(async(req,res)=>{try {const url=new URL(req.url,'http://localhost');if(!url.pathname.startsWith(deploy)){res.writeHead(404);return res.end();}const file=path.resolve(moved,decodeURIComponent(url.pathname.slice(deploy.length)||'index.html'));if(!file.startsWith(moved+path.sep)){res.writeHead(403);return res.end();}res.setHeader('Content-Type',({'.js':'text/javascript','.wasm':'application/wasm','.png':'image/png','.html':'text/html'})[path.extname(file)]||'text/plain');res.end(await fs.readFile(file));}catch{res.writeHead(404);res.end();}});
 await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
 browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH||undefined,headless:true,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader','--disable-webgpu']});
 const page=await browser.newPage({viewport:{width:800,height:500}}),errors=[];page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());if(process.env.WEB_MATERIAL_DEBUG)console.error('[browser]',m.type(),m.text());});
 await page.goto(`http://127.0.0.1:${server.address().port}${deploy}`);
 await page.waitForFunction(()=>['ready','error'].includes(document.querySelector('#gate').dataset.state),null,{timeout:60000});
 assert.equal(await page.locator('#gate').getAttribute('data-state'),'ready',await page.locator('#status').textContent());await page.locator('#start').click();await page.waitForFunction(()=>document.querySelector('#gate').hidden);await page.waitForTimeout(200);
 async function pixels(name,points){const png=await page.locator('#game').screenshot({path:path.join(evidence,name+'.png')});return page.evaluate(async({data,points})=>{const image=new Image();image.src='data:image/png;base64,'+data;await image.decode();const canvas=document.createElement('canvas');canvas.width=image.width;canvas.height=image.height;const ctx=canvas.getContext('2d',{willReadFrequently:true});ctx.drawImage(image,0,0);return points.map(([x,y])=>[...ctx.getImageData(x,y,1,1).data]);},{data:png.toString('base64'),points});}
 const points=[[80,100],[180,100],[280,100],[380,100],[480,100],[80,190],[180,190],[100,280],[364,280],[396,280],[480,264],[480,296]];
 const colors=await pixels('materials',points);
 const blue=c=>c[2]>200&&c[0]<40&&c[1]<40,red=c=>c[0]>200&&c[1]<40&&c[2]<40;
 assert.ok(blue(colors[0])&&blue(colors[4]),'Default/custom/default must reset pipeline');assert.ok(colors[1][1]>200&&colors[1][0]<40,'Shader default green');assert.ok(red(colors[2]),'Color override red');assert.ok(colors[3][0]>90&&colors[3][0]<190&&colors[3][1]<40,'Float brightness override');
 assert.ok(red(colors[5])&&blue(colors[6]),'Named texture defaults and overrides');
 assert.ok(colors[7][1]>colors[7][0]&&Math.abs(colors[7][0]-colors[7][2])<10&&colors[7][0]>90,'Transparent painter order across pipelines');
 assert.ok(blue(colors[8])&&red(colors[9]),'Custom fragment respects flip UVs');assert.ok(blue(colors[10])&&red(colors[11]),'Custom fragment uses normal rotation/quad vertex layout');
 async function key(k){await page.keyboard.down(k);await page.waitForTimeout(70);await page.keyboard.up(k);await page.waitForTimeout(80);}
 await key('s');await page.waitForFunction(()=>!!localStorage.getItem('kiln:kiln-materials2d:slot:0'));
 const saved=await page.evaluate(()=>JSON.parse(localStorage.getItem('kiln:kiln-materials2d:slot:0')));assert.equal(saved.scene.entities.find(e=>e.id==='overridden_texture').components.find(c=>c.type==='Sprite').material,'texture_blue');
 await key('n');const next=await pixels('next-scene',[[80,100]]);assert.ok(blue(next[0]));await key('l');const loaded=await pixels('restored',points);assert.deepEqual(loaded,colors,'Material references and pixels survive cross-scene load');
 if(process.env.WEB_MATERIAL_LEGACY_RUNTIME){let rejected=false;try{execFileSync(cli,['export','--platform','web','--project',path.join(work,'source-hidden/kiln.toml'),'--web-runtime',path.resolve(process.env.WEB_MATERIAL_LEGACY_RUNTIME),'--output',path.join(work,'legacy')],{cwd:work,env:{...process.env,PATH:''},stdio:'pipe'});}catch(error){rejected=String(error.stderr).includes('lacks sprite materials');}assert.ok(rejected);}
 assert.deepEqual(errors,[]);
 const manifestFile=path.join(moved,'project/kiln.toml'),manifest=await fs.readFile(manifestFile,'utf8');
 await fs.writeFile(manifestFile,manifest.replace('@fragment fn paint(', '@fragment INVALID SOURCE fn paint('));
 const invalid=await browser.newPage();await invalid.goto(`http://127.0.0.1:${server.address().port}${deploy}`);
 await invalid.waitForFunction(()=>document.querySelector('#gate').dataset.state==='error');
 assert.match(await invalid.locator('#status').textContent(),/Material/);await invalid.screenshot({path:path.join(evidence,'invalid-shader.png')});
 await invalid.close();await fs.writeFile(manifestFile,manifest);
 const result={passed:true,runtime_materials:true,relocated_source_unavailable:true,empty_tools_path:true,numeric_defaults_overrides:true,texture_defaults_overrides:true,transparent_order:true,flip_rotation:true,cross_scene_restore:true,invalid_shader_feedback:true,points,colors};await fs.writeFile(path.join(evidence,'result.json'),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify(result));
 }finally{if(browser)await browser.close();if(server)await new Promise(resolve=>server.close(resolve));}
})().catch(error=>{console.error(error);process.exitCode=1;});
