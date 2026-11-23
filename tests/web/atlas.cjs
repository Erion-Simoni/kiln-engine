// Relocated green-pixel replay: saved atlas by default, explicit scene with KILN_PIXEL_SCENARIO.
const assert=require('node:assert/strict'),fs=require('node:fs/promises'),http=require('node:http'),path=require('node:path'),os=require('node:os');
const {execFileSync}=require('node:child_process'),{chromium}=require('playwright');
const repo=path.resolve(__dirname,'../..'),runtime=path.resolve(process.argv[2]||'target/debug/web-runtime');
const evidence=path.resolve(process.env.WEB_ATLAS_EVIDENCE||'target/a03-atlas-browser');
(async()=>{let browser,server;try{
 await fs.mkdir(evidence,{recursive:true});await fs.rm(path.join(evidence,'result.json'),{force:true});
 const input=process.env.KILN_ATLAS_PROJECT||JSON.parse(await fs.readFile(path.join(repo,'target/a03-atlas-recipe/result.json'))).project;
 const scenario=process.env.KILN_PIXEL_SCENARIO;
 const work=await fs.mkdtemp(path.join(os.tmpdir(),'kiln-atlas-')),project=path.join(work,'project'),output=path.join(work,'game folder');
 await fs.cp(input,project,{recursive:true});
 if(process.env.WEB_ATLAS_EXPORT){await fs.cp(process.env.WEB_ATLAS_EXPORT,output,{recursive:true});}
 else{const cli=path.resolve(process.env.KILN_CLI_PATH||'target/debug/kiln-cli');const log=execFileSync(cli,['export','--platform','web','--project',path.join(project,'kiln.toml'),'--web-runtime',runtime,'--output',output],{cwd:work,env:{...process.env,PATH:''},encoding:'utf8'});await fs.writeFile(path.join(evidence,'export.log'),log);}
 await fs.rename(project,path.join(work,'source-hidden'));
 const deploy='/nested/atlas/';server=http.createServer(async(req,res)=>{try{
  const url=new URL(req.url,'http://localhost');if(url.pathname==='/favicon.ico'){res.writeHead(204);return res.end();}
  if(!url.pathname.startsWith(deploy)){res.writeHead(404);return res.end();}
  const file=path.resolve(output,decodeURIComponent(url.pathname.slice(deploy.length)||'index.html'));
  if(!file.startsWith(output+path.sep)){res.writeHead(403);return res.end();}
  res.setHeader('Content-Type',({'.js':'text/javascript','.wasm':'application/wasm','.png':'image/png','.html':'text/html'})[path.extname(file)]||'text/plain');res.end(await fs.readFile(file));
 }catch{res.writeHead(404);res.end();}});
 await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
 browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH||undefined,headless:true,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader','--disable-webgpu']});
 const page=await browser.newPage({viewport:{width:800,height:600}}),errors=[];page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
 await page.goto(`http://127.0.0.1:${server.address().port}${deploy}`);
 await page.waitForFunction(()=>['ready','error'].includes(document.querySelector('#gate').dataset.state),null,{timeout:60000});
 assert.equal(await page.locator('#gate').getAttribute('data-state'),'ready',await page.locator('#status').textContent());
 await page.locator('#start').click();await page.waitForFunction(()=>document.querySelector('#gate').hidden);
 const deadline=Date.now()+30000;let green=0;
 while(green<=200&&Date.now()<deadline){
  const png=await page.locator('#game').screenshot({path:path.join(evidence,scenario?'rendered_scene.png':'generated_clip.png')});
  green=await page.evaluate(async data=>{const image=new Image();image.src='data:image/png;base64,'+data;await image.decode();const canvas=document.createElement('canvas');canvas.width=image.width;canvas.height=image.height;const ctx=canvas.getContext('2d',{willReadFrequently:true});ctx.drawImage(image,0,0);const pixels=ctx.getImageData(0,0,image.width,image.height).data;let count=0;for(let at=0;at<pixels.length;at+=4)if(pixels[at+1]>220&&pixels[at]<30&&pixels[at+2]<30)count++;return count;},png.toString('base64'));
 }
 assert.ok(green>200,scenario?'Imported function result must render in actual browser pixels':'Generated green atlas clip must render in actual browser pixels');assert.deepEqual(errors,[]);
 const result={passed:true,nested_relocated_browser_export:true,authoring_copy_hidden:true,[scenario?'rendered_green_pixels':'generated_clip_green_pixels']:green,...(scenario?{scenario}:{}),scope:'Chromium/SwiftShader; no physical-device claim'};
 await fs.writeFile(path.join(evidence,'result.json'),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify(result));
}catch(error){await fs.writeFile(path.join(evidence,'failure.txt'),String(error));console.error(error);process.exitCode=1;}finally{if(browser)await browser.close();if(server)await new Promise(resolve=>server.close(resolve));}})();
