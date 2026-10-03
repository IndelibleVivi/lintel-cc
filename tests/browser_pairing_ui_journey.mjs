// Render the built App with a synthetic invoke/clipboard transport, but use
// actual core and Native Messaging host processes for pairing values/approval.
// Never reads or registers a personal browser profile.
import assert from 'node:assert/strict';
import {execFileSync, spawn, spawnSync} from 'node:child_process';
import {mkdir, mkdtemp, rm, writeFile} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {stripVTControlCharacters} from 'node:util';

const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const desktop=path.join(repo,'apps/desktop');
const modulePath=process.env.PLAYWRIGHT_MODULE || path.join(repo,'extensions/browser/node_modules/playwright/index.mjs');
const {chromium}=await import(pathToFileURL(modulePath).href);
execFileSync('cargo',['build','-p','lintel-runner'],{cwd:repo,stdio:'inherit'});
execFileSync('cargo',['build','--manifest-path','extensions/browser/native-host/Cargo.toml'],{cwd:repo,stdio:'inherit'});
const base=await mkdtemp(path.join(os.tmpdir(),'lintel-pairing-ui-'));
const home=path.join(base,'home');
await mkdir(path.join(home,'.claude'),{recursive:true});
await writeFile(path.join(home,'.claude/settings.json'),'{}\n');
const state=path.join(home,process.platform==='darwin'?'Library/Application Support/Lintel/browser-bridge':'.local/state/lintel/browser-bridge');
const host=path.join(repo,'extensions/browser/native-host/target/debug/lintel-browser-host');
const runner=path.join(repo,'target/debug/lintel');
const extensionId='a'.repeat(32),token=crypto.randomUUID().replaceAll('-','')+crypto.randomUUID().replaceAll('-',''),instanceId='synthetic-pairing-copy';
const env={...process.env,PATH:'/usr/bin:/bin',LINTEL_TEST_HOME:home,LINTEL_STATE_DIR:path.join(base,'core-state'),LINTEL_BROWSER_STATE:state};
function execJson(executable,args,input){const result=spawnSync(executable,args,{env,input,encoding:'utf8'});assert.equal(result.status,0,result.stderr);return JSON.parse(result.stdout);}
const control=payload=>execJson(host,['control'],JSON.stringify(payload));
function native(payload){
  const bytes=Buffer.from(JSON.stringify({...payload,request_id:crypto.randomUUID(),instance_id:instanceId,token}));
  const frame=Buffer.alloc(4+bytes.length);frame.writeUInt32LE(bytes.length);bytes.copy(frame,4);
  const result=spawnSync(host,[`chrome-extension://${extensionId}/`],{env,input:frame});
  assert.equal(result.status,0,result.stderr.toString());const size=result.stdout.readUInt32LE();
  return JSON.parse(result.stdout.subarray(4,4+size));
}
const report={fixture:'fresh synthetic home and headless browser',transport:'rendered App invoke shim to real core/host; synthetic clipboard',checks:[],passed:false};
const preview=spawn(process.execPath,[path.join(desktop,'node_modules/vite/bin/vite.js'),'preview','--host','127.0.0.1','--port','0','--strictPort'],{cwd:desktop,stdio:['ignore','pipe','pipe']});
let browser,pair;
try{
  const url=await new Promise((resolve,reject)=>{
    let output='';const timer=setTimeout(()=>reject(new Error('isolated Vite preview did not start within 15 seconds: '+stripVTControlCharacters(output))),15000);
    preview.once('error',error=>{clearTimeout(timer);reject(error);});
    preview.once('exit',code=>{clearTimeout(timer);reject(new Error(`preview exited ${code}: ${output}`));});
    for(const stream of [preview.stdout,preview.stderr])stream.on('data',chunk=>{output+=chunk;const match=stripVTControlCharacters(output).match(/http:\/\/127\.0\.0\.1:\d+\//);if(match){clearTimeout(timer);resolve(match[0]);}});
  });
  assert.equal(execJson(host,['register','chrome',extensionId,host,'--home',home,'--apply']).status,'registered');
  browser=await chromium.launch({headless:true});
  const page=await browser.newPage({viewport:{width:1120,height:800}});page.setDefaultTimeout(10000);
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await page.exposeFunction('actualInvoke',(command,args)=>{
    if(command==='request')return execJson(runner,['request'],JSON.stringify(args.payload)+'\n');
    if(command==='remote_request'&&args.payload.op==='hosts')return {ok:true,data:{hosts:[],tasks:[],installations:[]}};
    assert.equal(command,'browser_request');const response=control(args.payload);assert.equal(response.ok,true,JSON.stringify(response.error));
    if(args.payload.op==='pair_create')pair=response.data;return response;
  });
  await page.addInitScript(()=>{
    window.isTauri=true;window.clipboardWrites=[];
    Object.defineProperty(navigator,'clipboard',{value:{writeText:async text=>window.clipboardWrites.push(text)}});
    window.__TAURI_INTERNALS__={invoke:(command,args)=>window.actualInvoke(command,args)};
  });
  await page.goto(url);await page.getByRole('button',{name:'浏览器',exact:true}).click();
  const dialog=page.getByRole('dialog');await dialog.getByText('连接一个新的 profile',{exact:true}).click();
  await dialog.getByRole('button',{name:'生成配对请求',exact:true}).click();await page.locator('.pairing-code strong').waitFor();
  assert.match(pair.code,/^[A-F0-9]{12}$/);assert.notEqual(pair.challenge,pair.code);
  // Keep the old accessible name in the matcher so reverting the bug fails at
  // the real host boundary, rather than only because a button was renamed.
  await dialog.getByRole('button',{name:/复制(?:挑战值|配对短码)/}).click();
  const copied=await page.evaluate(()=>window.clipboardWrites.at(-1));
  const request=native({op:'pair_request',code:copied,label:'Synthetic copy-to-pair'});
  assert.equal(request.ok,true,JSON.stringify(request.error));assert.equal(copied,pair.code);
  assert.equal(await dialog.getByLabel('配对挑战值').count(),0);
  report.checks.push('App copied value accepted by framed native pair_request');
  await dialog.getByRole('button',{name:'检查配对',exact:true}).click();
  await dialog.getByText(`短码 ${pair.code} · 实例 ${instanceId}`,{exact:true}).waitFor();
  const pending=control({op:'pair_pending'}).data;assert.equal(pending.length,1);assert.equal(pending[0].challenge,pair.challenge);
  await dialog.getByRole('button',{name:'短码一致，批准此 profile',exact:true}).click();
  await dialog.getByText(/chromium · 已配对 · 当前离线/).waitFor();assert.equal(control({op:'instances'}).data[0].paired,true);
  report.checks.push('App approves internal pending challenge; no fabricated online state');
  const firstCode=pair.code;
  await dialog.getByRole('button',{name:'生成配对请求',exact:true}).click();await page.locator('.pairing-code strong').filter({hasText:pair.code}).waitFor();
  assert.notEqual(pair.code,firstCode);await page.locator('.pairing-code button').focus();await page.keyboard.press('Enter');
  assert.equal(await page.evaluate(()=>window.clipboardWrites.at(-1)),pair.code);
  await dialog.getByRole('button',{name:'完成',exact:true}).click();await page.getByRole('button',{name:'深色 Night',exact:true}).click();
  await page.setViewportSize({width:900,height:640});await page.getByRole('button',{name:'浏览器',exact:true}).click();
  await dialog.getByText('连接一个新的 profile',{exact:true}).click();await dialog.getByRole('button',{name:'生成配对请求',exact:true}).click();
  await page.locator('.pairing-code strong').filter({hasText:pair.code}).waitFor();await page.locator('.pairing-code').scrollIntoViewIfNeeded();
  assert.equal(await dialog.evaluate(element=>element.scrollWidth>element.clientWidth),false);assert.deepEqual(errors,[]);
  report.checks.push('fresh code replacement, keyboard Enter, Day/Night layout');report.passed=true;
  console.log('PASS: rendered App copy/real native pairing/explicit approval journey');
}catch(error){report.error=error.message;report.retainedFixture=base;throw error;}finally{
  await browser?.close();
  const exited=new Promise(resolve=>preview.once('exit',resolve));if(preview.exitCode===null){preview.kill('SIGTERM');await exited;}
  await writeFile(process.env.LINTEL_PAIRING_UI_REPORT || path.join(os.tmpdir(),'lintel-pairing-ui.json'),JSON.stringify(report,null,2));
  if(report.passed)await rm(base,{recursive:true,force:true});
}
