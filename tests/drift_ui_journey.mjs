// Built App, controlled synthetic invoke timing. Never changes a real baseline.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdtemp, writeFile} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {stripVTControlCharacters} from 'node:util';
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'), desktop=path.join(repo,'apps/desktop');
const {chromium}=await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE||path.join(repo,'extensions/browser/node_modules/playwright/index.mjs')).href);
const fixture=await mkdtemp(path.join(os.tmpdir(),'lintel-drift-ui-'));
const environments=['A','B'].map(id=>({id,name:`Synthetic ${id}`,root:`/synthetic/${id}`,status:'active',executable:null}));
const calls=[],errors=[],waiting=[];
let holdAccept=false, pendingAccept;
const preview=spawn(process.execPath,[path.join(desktop,'node_modules/vite/bin/vite.js'),'preview','--host','127.0.0.1','--port','0','--strictPort'],{cwd:desktop,stdio:['ignore','pipe','pipe']});
const report={fixture,runtime:'built App + controlled synthetic invoke; not native WebKit',checks:[],passed:false};
let browser,page;
function drift(label){return {status:'drift',changes:[{key:'DO_NOT_TRACK',label,value:label,status:'drift',source:'synthetic',effect_timing:'next_launch'}]};}
async function core(alias,payload){
  calls.push({alias,...payload});
  if(payload.command==='drift')return new Promise(resolve=>waiting.push({alias,payload,resolve}));
  if(payload.command==='accept_drift'&&holdAccept)await new Promise(resolve=>pendingAccept=resolve);
  const data={discover:{environments,capabilities:[]},jobs:{jobs:[]},inspect:{settings:[],assets:[],warnings:[]},accept_drift:{status:'accepted'}}[payload.command];
  assert.ok(data,`unexpected ${payload.command}`);return {ok:true,data};
}
async function wait(check){const end=Date.now()+8000;while(Date.now()<end){if(await check())return;await new Promise(resolve=>setTimeout(resolve,20));}throw new Error('controlled boundary not observed');}
async function release(label, fail=false){await wait(()=>waiting.length);const pending=waiting.shift();pending.resolve(fail?{ok:false,error:{code:'synthetic_late_failure',message:label}}:{ok:true,data:drift(label)});await page.waitForTimeout(100);return pending;}
async function choose(id){await page.locator('.sidebar-environment').filter({hasText:`Synthetic ${id}`}).click();await page.getByLabel('当前环境',{exact:true}).waitFor();assert.equal(await page.getByLabel('当前环境',{exact:true}).inputValue(),id);}
async function check(){await page.getByRole('button',{name:'检查变化',exact:true}).click();await wait(()=>waiting.length);}
try{
  const url=await new Promise((resolve,reject)=>{let out='';const timer=setTimeout(()=>reject(new Error('preview timeout '+out)),15000);preview.once('error',reject);for(const stream of [preview.stdout,preview.stderr])stream.on('data',data=>{out+=stripVTControlCharacters(data.toString());const match=out.match(/http:\/\/127\.0\.0\.1:\d+\//);if(match){clearTimeout(timer);resolve(match[0]);}});});report.url=url;
  browser=await chromium.launch({headless:true});page=await browser.newPage({viewport:{width:1120,height:760}});page.setDefaultTimeout(8000);page.on('pageerror',e=>errors.push(e.message));
  await page.exposeFunction('syntheticInvoke',async(command,args)=>{
    if(command==='request')return core(null,args.payload);
    if(command==='remote_request'){
      const p=args.payload;
      if(p.op==='hosts')return {ok:true,data:{hosts:[{alias:'synthetic-host'}],tasks:[],installations:[],launches:[]}};
      if(p.op==='aliases')return {ok:true,data:{aliases:['synthetic-host'],coverage:'synthetic'}};
      if(p.op==='connect')return {ok:true,data:{status:'connected'}};
      if(p.op==='request')return core(p.alias,p.request);
    }
    throw new Error('unexpected invoke '+command);
  });
  await page.addInitScript(()=>{window.isTauri=true;window.__TAURI_INTERNALS__={invoke:(command,args)=>window.syntheticInvoke(command,args)};});
  await page.goto(url);await page.getByRole('button',{name:'环境详情',exact:true}).click();
  await check();await choose('B');await release('STALE_A');
  assert.equal(await page.locator('.drift-result').count(),0,'A result must not appear under B');
  assert.equal(await page.getByRole('button',{name:'接受当前值',exact:true}).count(),0);
  report.checks.push('late A result discarded after sidebar switch to B');
  await choose('A');await check();await choose('B');await choose('A');await release('STALE_A_ROUNDTRIP');
  assert.equal(await page.locator('.drift-result').count(),0,'returning to A must not revive the older generation');
  await check();await choose('B');await release('STALE_FAILURE',true);
  assert.ok(!(await page.locator('body').innerText()).includes('STALE_FAILURE'));
  assert.ok(await page.getByRole('button',{name:'检查变化',exact:true}).isEnabled());
  report.checks.push('A-B-A generation and stale failure/busy completion remain isolated');
  await check();const current=await release('CURRENT_B');assert.equal(current.payload.environment_id,'B');
  await page.getByRole('button',{name:'接受当前值',exact:true}).click();await release('ACCEPTED_B');
  const accepted=calls.filter(c=>c.command==='accept_drift');assert.deepEqual(accepted,[{alias:null,command:'accept_drift',environment_id:'B'}]);
  await page.getByText('已将当前值记为此环境的基线',{exact:true}).waitFor();
  report.checks.push('accept uses exactly the displayed target and rechecks that same target');
  holdAccept=true;await page.getByRole('button',{name:'接受当前值',exact:true}).click();await wait(()=>pendingAccept);await choose('A');pendingAccept();pendingAccept=null;holdAccept=false;await page.waitForTimeout(120);
  assert.equal(waiting.length,0,'departed accept must not enqueue a stale follow-up drift read');
  assert.equal(await page.locator('.drift-result').count(),0);
  await check();await page.locator('.host-switch').click();const dialog=page.getByRole('dialog');await dialog.locator('.host-row').filter({hasText:'synthetic-host'}).getByRole('button',{name:'连接并管理',exact:true}).click();
  await page.locator('.host-switch').getByText('synthetic-host',{exact:true}).waitFor();await release('STALE_LOCAL_A');
  assert.equal(await page.locator('.drift-result').count(),0,'same environment id on another host must not inherit result');
  await check();const remote=await release('CURRENT_REMOTE_A');assert.equal(remote.alias,'synthetic-host');
  await page.getByRole('button',{name:'接受当前值',exact:true}).click();await release('ACCEPTED_REMOTE_A');
  assert.deepEqual(calls.filter(c=>c.command==='accept_drift').at(-1),{alias:'synthetic-host',command:'accept_drift',environment_id:'A'});
  report.checks.push('accept in flight is target-owned; same-id host switch discards local result and remote accept retains alias');
  assert.deepEqual(errors,[]);await page.screenshot({path:path.join(fixture,'drift-target-bound.png')});report.passed=true;console.log('PASS: drift result, accept, pending/error and host/environment generations');
}catch(error){report.error=error.stack;report.body=await page?.locator('body').innerText().catch(()=>null);await page?.screenshot({path:path.join(fixture,'failure.png')}).catch(()=>{});throw error;}
finally{for(const pending of waiting)pending.resolve({ok:false,error:{code:'test_disposed',message:'synthetic disposed'}});pendingAccept?.();await browser?.close();if(preview.exitCode===null){const ended=new Promise(resolve=>preview.once('exit',resolve));preview.kill('SIGTERM');await ended;}report.calls=calls;report.errors=errors;await writeFile(process.env.LINTEL_DRIFT_UI_REPORT||path.join(os.tmpdir(),'lintel-drift-ui.json'),JSON.stringify(report,null,2));}
