// Built App recovery journey. Invoke, SSH and durable registry are synthetic;
// registry lives in this harness across App reloads. Real PAM/SSH lifetime
// evidence belongs to linux_vm_journey.py, not this rendered UI check.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdir, writeFile} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {stripVTControlCharacters} from 'node:util';

const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const desktop=path.join(repo,'apps/desktop');
const {chromium}=await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE || path.join(repo,'extensions/browser/node_modules/playwright/index.mjs')).href);
const alias='synthetic-writing-server';
const local={id:'11111111-1111-4111-8111-111111111111',name:'Synthetic local workspace',host:'local',surface:'claude-code',root:'/synthetic/local',executable:null,ownership:'registered',status:'discovered'};
const remote={...local,id:'22222222-2222-4222-8222-222222222222',name:'Synthetic remote workspace',host:alias,root:'/synthetic/remote'};
const original='33333333-3333-4333-8333-333333333333',restoreId='44444444-4444-4444-8444-444444444444';
const calls=[],receipts=[],tasks=[];let removed=false,conflict=false;
let heldDiscover,heldJob,discoverRequested=false,jobRequested=false;
function plan(restore=false){return {id:restore?restoreId:original,hash:restore?'synthetic-restore-approval':'synthetic-policy-approval',environment_id:remote.id,title:restore?'恢复原任务配置':'减少外发',changes:[],preserves:['后续编辑与邻居环境'],warnings:[],actions:[{id:'settings',label:restore?'恢复本任务原值':'写入当前环境设置',reversible:!restore}],created_at:new Date().toISOString(),status:'planned'};}
const ok=data=>({ok:true,data});
async function invoke(command,args){
  const p=args.payload;calls.push({command,...p});
  if(command==='remote_request'){
    switch(p.op){
      case 'hosts':return ok({hosts:removed?[]:[{alias}],tasks,installations:[]});
      case 'aliases':return ok({aliases:[alias],coverage:'Synthetic SSH inventory; no real connection'});
      case 'connect':assert.equal(p.alias,alias);return ok({status:'connected'});
      case 'remove_host':removed=true;return ok({status:'removed'});
      case 'reconnect':assert.equal(p.alias,alias);assert.equal(p.plan_id,original);return ok(await core({command:'job',job_id:p.plan_id},true));
      case 'request':assert.equal(p.alias,alias);return ok(await core(p.request,true));
      case 'execute':assert.equal(p.alias,alias);return ok(await core({...p,command:'execute'},true));
      default:throw new Error('unexpected remote operation: '+p.op);
    }
  }
  assert.equal(command,'request');return ok(await core(p,false));
}
async function core(p,isRemote){
  const environment=isRemote?remote:local;
  switch(p.command){
    case 'discover':
      if(isRemote && heldDiscover){discoverRequested=true;await heldDiscover.promise;}
      return {environments:[environment],capabilities:[]};
    case 'jobs':return {jobs:isRemote?receipts.map(r=>({...r})):[]};
    case 'inspect':return {environment,settings:[],assets:[],warnings:[]};
    case 'plan_policy':assert.ok(isRemote);return plan();
    case 'execute':{
      assert.ok(isRemote);const restore=p.plan_id===restoreId;
      assert.equal(p.approval,plan(restore).hash);assert.ok(!receipts.some(r=>r.id===p.plan_id),'duplicate submission');
      const receipt={id:p.plan_id,plan_id:p.plan_id,environment_id:remote.id,title:plan(restore).title,status:restore?'completed':'accepted',created_at:new Date().toISOString(),restorable:false,warnings:[],execution:{mode:'setsid',manager:null,unit:null,continuation:'Synthetic host: logout continuation unverified; reboot interrupts',limitation:'Synthetic existing user manager has no linger',reboot_survival:false},steps:restore?[{id:'settings',label:'恢复本任务原值',status:'completed',message:'Synthetic original values restored; external edits preserved'}]:[]};
      receipts.push(receipt);tasks.push({alias,plan_id:p.plan_id,lookup_id:p.plan_id,status:receipt.status});
      if(!restore)throw new Error('Synthetic ACK lost after durable acceptance');return {...receipt};
    }
    case 'job':
      if(heldJob){jobRequested=true;await heldJob.promise;}
      assert.ok(isRemote);return {...receipts.find(r=>r.id===p.job_id)};
    case 'plan_restore':
      assert.ok(isRemote);assert.equal(p.job_id,original);
      if(conflict)throw new Error('Synthetic external edit conflict; later values preserved');
      return plan(true);
    default:throw new Error('unexpected core operation: '+p.command);
  }
}
function hold(){let release;const promise=new Promise(resolve=>{release=resolve;});return {promise,release};}
const report={fixture:'synthetic SSH/invoke/registry surviving App reload',runtime:'built App in isolated headless Chromium; not native WebKit or real SSH',checks:[],passed:false};
const preview=spawn(process.execPath,[path.join(desktop,'node_modules/vite/bin/vite.js'),'preview','--host','127.0.0.1','--port','0','--strictPort'],{cwd:desktop,stdio:['ignore','pipe','pipe']});
let browser;
try{
  const url=await new Promise((resolve,reject)=>{
    let output='';const timer=setTimeout(()=>reject(new Error('preview timeout: '+stripVTControlCharacters(output))),15000);
    preview.once('error',error=>{clearTimeout(timer);reject(error);});preview.once('exit',code=>{clearTimeout(timer);reject(new Error('preview exited '+code));});
    for(const stream of [preview.stdout,preview.stderr])stream.on('data',chunk=>{output+=chunk;const match=stripVTControlCharacters(output).match(/http:\/\/127\.0\.0\.1:\d+\//);if(match){clearTimeout(timer);resolve(match[0]);}});
  });
  browser=await chromium.launch({headless:true});report.browser=browser.version();report.url=url;
  const page=await browser.newPage({viewport:{width:1120,height:760}});page.setDefaultTimeout(10000);
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await page.exposeFunction('syntheticInvoke',invoke);
  await page.addInitScript(()=>{window.isTauri=true;window.__TAURI_INTERNALS__={invoke:(command,args)=>window.syntheticInvoke(command,args)};});
  await page.goto(url);await page.getByRole('button',{name:'本机',exact:true}).click();
  const dialog=page.getByRole('dialog');await dialog.getByRole('button',{name:'连接并管理',exact:true}).click();
  await page.getByRole('button',{name:'预览变更',exact:true}).click();await dialog.getByRole('button',{name:'批准并执行',exact:true}).click();
  await dialog.getByRole('heading',{name:'结果待核对',exact:true}).waitFor();
  assert.equal(calls.filter(c=>c.op==='execute').length,1);
  await dialog.getByRole('button',{name:'返回记录',exact:true}).click();
  await page.reload();await page.getByRole('button',{name:'本机',exact:true}).click();
  await dialog.getByRole('button',{name:'查询原任务',exact:true}).click();
  await dialog.getByRole('button',{name:'查看完整回执与恢复',exact:true}).focus();await page.keyboard.press('Enter');
  await dialog.getByRole('heading',{name:'执行结果',exact:true}).waitFor();
  await dialog.locator('.receipt-id').getByText(original,{exact:true}).waitFor();
  await dialog.getByText('断线后的继续执行未获保证',{exact:true}).waitFor();
  assert.match(await dialog.locator('.plan-target').innerText(),new RegExp(alias));
  assert.equal(calls.filter(c=>c.op==='execute').length,1);
  report.checks.push('ACK loss then App reload queries same original task; keyboard opens full receipt on exact alias/environment; no resubmit');

  // An outstanding original-job query must not reopen a receipt closed by the user.
  heldJob=hold();await dialog.getByRole('button',{name:'查询最新结果',exact:true}).click();
  await assertWait(()=>jobRequested);assert.equal(calls.some(c=>c.op==='request'&&c.request?.command==='job'),false,'Receipt refresh bypassed the durable pinned-runner reconnect path');await dialog.getByRole('button',{name:'关闭面板',exact:true}).click();
  heldJob.release();heldJob=null;await page.getByRole('button',{name:alias,exact:true}).waitFor({state:'visible'});
  await assertWait(async()=>!await pageBusy());assert.equal(await dialog.count(),0);
  await page.getByRole('button',{name:'记录与恢复',exact:true}).click();
  await page.getByText('执行连接中断？用原任务 ID 找回结果',{exact:true}).click();
  const taskCountBeforeManual=tasks.length;
  await page.getByLabel('原任务 ID',{exact:true}).fill(` ${original} `);
  await page.locator('.query-task').getByRole('button',{name:'查询原任务',exact:true}).click();
  await dialog.getByRole('heading',{name:'执行结果',exact:true}).waitFor();
  assert.deepEqual(calls.filter(c=>c.op==='request'&&c.request?.command==='job').at(-1)?.request,{command:'job',job_id:original},'Manual lookup must use the read-only job operation');
  assert.equal(tasks.length,taskCountBeforeManual,'Manual lookup must not allocate a submission record');
  await dialog.getByRole('button',{name:'关闭面板',exact:true}).click();
  report.checks.push('receipt refresh uses reconnect; manual original-ID lookup uses read-only job without allocating a submission record; late original-job response cannot revive a closed result panel');

  await page.getByRole('button',{name:alias,exact:true}).click();await dialog.getByRole('button',{name:'查询原任务',exact:true}).click();
  heldDiscover=hold();await dialog.getByRole('button',{name:'查看完整回执与恢复',exact:true}).click();
  await assertWait(()=>discoverRequested);await dialog.getByRole('button',{name:'关闭面板',exact:true}).click();
  heldDiscover.release();heldDiscover=null;
  await assertWait(()=>calls.filter(c=>c.request?.command==='discover').length>=3);await page.waitForTimeout(100);
  assert.equal(await dialog.count(),0);report.checks.push('closing SSH panel during read-only discovery suppresses late full-receipt navigation');

  await page.getByRole('button',{name:alias,exact:true}).click();await dialog.getByRole('button',{name:'查询原任务',exact:true}).click();
  await dialog.getByRole('button',{name:'查看完整回执与恢复',exact:true}).click();
  receipts[0]={...receipts[0],execution:{mode:'user_manager',manager:'user',unit:`lintel-${original}.service`,continuation:'Synthetic selected persistent user manager; reboot interrupts',limitation:null,reboot_survival:false},status:'completed',restorable:true,steps:[{id:'settings',label:'当前环境设置',status:'completed',message:'Synthetic write read back; neighbor preserved'}]};tasks[0].status='completed';
  await dialog.getByRole('button',{name:'查询最新结果',exact:true}).click();
  await dialog.getByRole('button',{name:'预览恢复',exact:true}).waitFor();
  await dialog.locator('.receipt-id').getByText(original,{exact:true}).waitFor();
  await dialog.getByText('任务已交给主机后台管理',{exact:true}).waitFor();
  await dialog.getByText('查看任务托管详情',{exact:true}).click();await dialog.getByText(`lintel-${original}.service`,{exact:true}).waitFor();
  assert.equal(await dialog.evaluate(el=>el.scrollWidth>el.clientWidth),false);
  const artifacts=process.env.LINTEL_REMOTE_TASK_UI_ARTIFACTS;
  if(artifacts){await mkdir(artifacts,{recursive:true});await page.screenshot({path:path.join(artifacts,'remote-task-day-managed.png')});}
  report.checks.push('persisted execution facts and limitation are visible on reopened receipt; query latest updates the displayed original accepted receipt to completed without executing');conflict=true;
  await dialog.getByRole('button',{name:'预览恢复',exact:true}).click();await dialog.getByText(/Synthetic external edit conflict/).waitFor();
  assert.equal(calls.filter(c=>c.op==='execute').length,1);
  conflict=false;await dialog.getByRole('button',{name:'预览恢复',exact:true}).click();
  await dialog.getByRole('heading',{name:'确认这份计划',exact:true}).waitFor();assert.equal(calls.filter(c=>c.op==='execute').length,1);
  await dialog.getByRole('button',{name:'批准并执行',exact:true}).click();await dialog.getByText('Synthetic original values restored; external edits preserved').waitFor();
  assert.equal(calls.filter(c=>c.op==='execute').length,2);
  report.checks.push('external-edit conflict never executes; restore requires its own fresh preview and approval');
  if(artifacts){await mkdir(artifacts,{recursive:true});await page.screenshot({path:path.join(artifacts,'remote-task-day-receipt.png')});}
  await dialog.getByRole('button',{name:'查看记录',exact:true}).click();await page.getByRole('button',{name:'深色 Night',exact:true}).click();await page.setViewportSize({width:900,height:640});
  await page.getByRole('button',{name:alias,exact:true}).click();await dialog.getByRole('button',{name:`从 Lintel 移除 ${alias}`,exact:true}).click();
  await dialog.locator('.remote-task').filter({hasText:original}).getByRole('button',{name:'查询原任务',exact:true}).click();
  assert.ok(await dialog.getByRole('button',{name:'查看完整回执与恢复',exact:true}).isDisabled());await dialog.getByText('主机已从列表移除 · 原任务仍可查询',{exact:true}).first().waitFor();
  assert.equal(calls.filter(c=>c.op==='execute').length,2);
  assert.equal(await dialog.evaluate(el=>el.scrollWidth>el.clientWidth),false);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
  await dialog.getByRole('button',{name:'查看完整回执与恢复',exact:true}).scrollIntoViewIfNeeded();
  if(artifacts)await page.screenshot({path:path.join(artifacts,'remote-task-night-removed.png')});
  const cursor=await page.locator('.brand strong .brand-cursor').evaluate(el=>({animation:getComputedStyle(el).animationName,duration:getComputedStyle(el).animationDuration}));assert.deepEqual(cursor,{animation:'brand-blink',duration:'2.6s'});
  await page.emulateMedia({reducedMotion:'reduce'});assert.equal(await page.locator('.brand strong .brand-cursor').evaluate(el=>getComputedStyle(el).animationName),'none');assert.deepEqual(errors,[]);
  report.checks.push('removed alias remains query-only; Day/Night 1120/900 layout; wordmark slow blink/reduced motion preserved');report.passed=true;
  console.log('PASS: built App synthetic remote task reopen/query/conflict/separate restore journey');
  async function pageBusy(){return page.getByRole('button',{name:alias,exact:true}).isDisabled();}
}catch(error){report.error=error.message;throw error;}finally{
  await browser?.close();const exited=new Promise(resolve=>preview.once('exit',resolve));if(preview.exitCode===null){preview.kill('SIGTERM');await exited;}
  await writeFile(process.env.LINTEL_REMOTE_TASK_UI_REPORT || path.join(os.tmpdir(),'lintel-remote-task-ui.json'),JSON.stringify(report,null,2));
}
async function assertWait(condition){const deadline=Date.now()+10000;while(Date.now()<deadline){if(await condition())return;await new Promise(resolve=>setTimeout(resolve,25));}throw new Error('synthetic boundary was not observed');}
