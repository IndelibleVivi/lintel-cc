// Built App UI with synthetic browser IPC; no native registration/profile writes.
import assert from 'node:assert/strict';
import {spawn,spawnSync} from 'node:child_process';
import {mkdtemp,mkdir,writeFile,rm} from 'node:fs/promises';
import os from 'node:os';import path from 'node:path';import {fileURLToPath,pathToFileURL} from 'node:url';import {stripVTControlCharacters} from 'node:util';
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'),desktop=path.join(repo,'apps/desktop');
const {chromium}=await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE||path.join(repo,'extensions/browser/node_modules/playwright/index.mjs')).href);
const root=await mkdtemp(path.join(os.tmpdir(),'lintel-host-ui-')),home=path.join(root,'home');await mkdir(path.join(home,'.claude'),{recursive:true});await writeFile(path.join(home,'.claude/settings.json'),'{}');
const runner=process.env.LINTEL_TEST_RUNNER||path.join(repo,'target/debug/lintel');
const plan={browser:'chrome',extension_id:'a'.repeat(32),version:'0.2.0',sha256:'a'.repeat(64),bytes:24,host_path:'/synthetic/host',manifest_path:'/synthetic/manifest',manifest:{name:'synthetic'},state_path:'/synthetic/state',effect:'synthetic recovery',existing_manifest:null,status:'ready',install_action:'install'};
const preview=spawn(process.execPath,[path.join(desktop,'node_modules/vite/bin/vite.js'),'preview','--host','127.0.0.1','--port','0'],{cwd:desktop,stdio:['ignore','pipe','pipe']});let browser;
try{
 const url=await new Promise((resolve,reject)=>{let out='';const timer=setTimeout(()=>reject(Error(out)),20000);for(const stream of [preview.stdout,preview.stderr])stream.on('data',chunk=>{out+=chunk;const match=stripVTControlCharacters(out).match(/http:\/\/127\.0\.0\.1:\d+\//);if(match){clearTimeout(timer);resolve(match[0]);}});});
 browser=await chromium.launch({headless:true});const page=await browser.newPage();page.on('pageerror',e=>console.error(e));let mode='pending',installed=0;
 await page.exposeFunction('actualInvoke',(command,args)=>{
  if(command==='request'){const r=spawnSync(runner,['request'],{env:{...process.env,LINTEL_TEST_HOME:home,LINTEL_STATE_DIR:path.join(root,'state')},input:JSON.stringify(args.payload)+'\n',encoding:'utf8'});assert.equal(r.status,0,r.stderr);return JSON.parse(r.stdout);}
  if(command==='remote_request')return {ok:true,data:{hosts:[],tasks:[],installations:[]}};
  const op=args?.payload?.op;if(op==='bundled_host_pending')return mode==='error'?{ok:false,error:{code:'storage_error',message:'synthetic invalid pending intent'}}:{ok:true,data:plan};
  if(op==='install_bundled_host'){assert.deepEqual(args.payload.approved_plan,plan);installed++;return {ok:true,data:{status:'registered',plan}};}
  return {ok:true,data:['instances','pair_pending'].includes(op)?[]:{}};
 });
 await page.addInitScript(()=>{localStorage.setItem('lintel.language','zh-CN');window.isTauri=true;window.__TAURI_INTERNALS__={invoke:(command,args)=>command==='app_preferred_languages'?['zh-CN']:command==='app_set_language'?null:window.actualInvoke(command,args)};});
 await page.goto(url);await page.getByRole('button',{name:'浏览器',exact:true}).click();await page.locator('.browser-setup > summary').click();await page.getByRole('button',{name:'继续原安装',exact:true}).waitFor();await page.getByRole('button',{name:'继续原安装',exact:true}).click();assert.equal(installed,1);
 mode='error';await page.reload();await page.getByRole('button',{name:'浏览器',exact:true}).click();await page.locator('.browser-setup > summary').click();await page.getByText(/无法查询未完成的安装/).waitFor();await page.getByText(/synthetic invalid pending intent/).waitFor();
 console.log(JSON.stringify({passed:true,transport:'built App synthetic browser IPC; real synthetic-root core',checks:['pending discovered after mount and original approval continued','pending query error visible after reopen']}));
}finally{await browser?.close();preview.kill();await rm(root,{recursive:true,force:true});}
