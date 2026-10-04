// Built App journey. The service manager and invoke transport are explicitly
// synthetic; real systemd/PAM/reboot acceptance lives in linux_vm_journey.py.
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
const environment={id:'11111111-1111-4111-8111-111111111111',name:'Synthetic service workspace',host:'local',surface:'claude-code',root:'/synthetic/config',executable:null,ownership:'registered',status:'discovered'};
const held={manager:'user',unit:'claude-work.service',root:environment.root,before:{active_state:'active',sub_state:'running',unit_file_state:'enabled',restart:'always'},after:{active_state:'inactive',hold:true},hold:{path:'/synthetic/units/claude-work.service.d/90-lintel.conf',persistent:true},original_job:null};
const quiesceId='22222222-2222-4222-8222-222222222222',resumeId='33333333-3333-4333-8333-333333333333';
const calls=[],receipts=[];let quiesced=false,resumeConflict=false,loseAck=true;
function makePlan(resume){return {id:resume?resumeId:quiesceId,hash:resume?'synthetic-resume-approval':'synthetic-quiesce-approval',environment_id:environment.id,title:resume?'恢复目标服务':'暂停目标服务',changes:[],preserves:['其他服务与原始 unit'],warnings:[],actions:[{id:'service',label:resume?'核对暂停项并按原状态恢复':'添加暂停项并停止目标',reversible:!resume}],created_at:new Date().toISOString(),status:'planned',service:resume?{...held,original_job:quiesceId,before:{...held.before,active_state:'inactive',sub_state:'dead'},after:{active_state:'active',hold:false}}:held};}
function invoke(command,args){
  assert.equal(command,'request');const p=args.payload;calls.push(p);
  let data;
  switch(p.command){
    case 'discover':data={environments:[environment],capabilities:[]};break;
    case 'jobs':data={jobs:receipts};break;
    case 'inspect':data={environment,settings:[],assets:[],warnings:[]};break;
    case 'cleanup_inspect':data={environment_id:environment.id,files:[],writers:quiesced?[]:[{pid:123,name:'claude',scope:'not_attributed'}],shared_profile_present:false,official_logout_available:false,coverage:'Synthetic files only'};break;
    case 'service_inspect':
      assert.equal(p.environment_id,environment.id);assert.equal(p.unit,held.unit);
      data={environment_id:environment.id,manager:p.manager,unit:p.unit,root:environment.root,active_state:quiesced?'inactive':'active',sub_state:quiesced?'dead':'running',unit_file_state:'enabled',restart:'always',main_pid:quiesced?0:123,control_group:'/synthetic.slice/target',triggered_by:['claude-work.timer'],bound:true,quiesced,quiesce_job_id:quiesced?quiesceId:null,hold:quiesced?held.hold:null,limitations:['Synthetic manager; no real host operation']};break;
    case 'plan_service_quiesce':assert.equal(p.unit,held.unit);data=makePlan(false);break;
    case 'plan_service_resume':
      assert.equal(p.job_id,quiesceId);
      if(resumeConflict)return {ok:false,error:{code:'service_conflict',message:'目标 unit 在任务后被编辑；保留后续修改，未恢复。'}};
      data=makePlan(true);break;
    case 'execute':{
      const resume=p.plan_id===resumeId;assert.equal(p.approval,makePlan(resume).hash);assert.equal(receipts.some(r=>r.id===p.plan_id),false,'duplicate mutation');quiesced=!resume;
      data={id:p.plan_id,plan_id:p.plan_id,environment_id:environment.id,title:makePlan(resume).title,status:'completed',created_at:new Date().toISOString(),restorable:false,service_restorable:!resume,warnings:[],steps:[{id:'service',label:'目标服务读回',status:'completed',message:resume?'目标恢复运行；邻居保持运行':'目标停止且禁止启动；邻居保持运行'}],service:{...makePlan(resume).service,observed:{active_state:quiesced?'inactive':'active',main_pid:quiesced?0:124,quiesced}}};receipts.push(data);if(!resume && loseAck){loseAck=false;throw new Error('Synthetic accepted response lost');}break;
    }
    case 'job':data=receipts.find(r=>r.id===p.job_id);assert.ok(data);break;
    default:throw new Error('unexpected synthetic operation: '+p.command);
  }
  return {ok:true,data};
}
const report={fixture:'synthetic service manager and invoke transport',runtime:'built App in headless Chromium; not native WebKit or systemd',checks:[],passed:false};
const preview=spawn(process.execPath,[path.join(desktop,'node_modules/vite/bin/vite.js'),'preview','--host','127.0.0.1','--port','0','--strictPort'],{cwd:desktop,stdio:['ignore','pipe','pipe']});
let browser;
try{
  const url=await new Promise((resolve,reject)=>{
    let output='';const timer=setTimeout(()=>reject(new Error('preview timeout: '+stripVTControlCharacters(output))),15000);
    preview.once('error',e=>{clearTimeout(timer);reject(e);});preview.once('exit',code=>{clearTimeout(timer);reject(new Error('preview exited '+code));});
    for(const stream of [preview.stdout,preview.stderr])stream.on('data',chunk=>{output+=chunk;const match=stripVTControlCharacters(output).match(/http:\/\/127\.0\.0\.1:\d+\//);if(match){clearTimeout(timer);resolve(match[0]);}});
  });
  browser=await chromium.launch({headless:true});report.browser=browser.version();
  const page=await browser.newPage({viewport:{width:1120,height:760}});page.setDefaultTimeout(10000);
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await page.exposeFunction('syntheticInvoke',invoke);
  await page.addInitScript(()=>{window.isTauri=true;window.__TAURI_INTERNALS__={invoke:(command,args)=>window.syntheticInvoke(command,args)};});
  await page.goto(url);await page.getByRole('button',{name:'清理与重建',exact:true}).click();
  await page.getByRole('radio',{name:/修复本地登录/}).check();
  const control=page.locator('.service-control');await control.locator('summary').click();
  await control.getByLabel('服务 unit').fill(held.unit);await control.getByRole('button',{name:'检查服务',exact:true}).click();
  await control.getByText('运行中',{exact:true}).waitFor();assert.equal(calls.some(c=>c.command==='execute'),false);
  await control.getByRole('button',{name:'预览暂停服务',exact:true}).click();
  const dialog=page.getByRole('dialog');await dialog.getByRole('heading',{name:'暂停这一个服务',exact:true}).waitFor();
  assert.equal(calls.some(c=>c.command==='execute'),false);
  await dialog.getByRole('button',{name:'返回调整',exact:true}).click();assert.equal(calls.some(c=>c.command==='execute'),false);
  report.checks.push('inspect/preview/cancel never mutates; target and persistent hold visible');
  await control.getByRole('button',{name:'检查服务',exact:true}).click();await control.getByRole('button',{name:'预览暂停服务',exact:true}).click();
  await dialog.getByRole('button',{name:'批准并执行',exact:true}).click();await dialog.getByRole('button',{name:'查询原任务',exact:true}).click();await dialog.getByRole('button',{name:'预览恢复服务',exact:true}).waitFor();
  assert.equal(calls.filter(c=>c.command==='execute').length,1);assert.ok(calls.filter(c=>c.command==='cleanup_inspect').length>=2,'service receipt refreshes cleanup writers');assert.equal(await dialog.getByRole('button',{name:'打开 Claude',exact:true}).count(),0);
  resumeConflict=true;await dialog.getByRole('button',{name:'预览恢复服务',exact:true}).click();await dialog.getByText(/目标 unit 在任务后被编辑/).waitFor();assert.equal(calls.filter(c=>c.command==='execute').length,1);
  report.checks.push('lost ACK queries original job; one approved mutation; external-edit conflict preserves hold');
  resumeConflict=false;await dialog.getByRole('button',{name:'预览恢复服务',exact:true}).click();await dialog.getByRole('heading',{name:'恢复这一个服务',exact:true}).waitFor();
  assert.equal(calls.filter(c=>c.command==='execute').length,1);await dialog.getByRole('button',{name:'批准并执行',exact:true}).focus();await page.keyboard.press('Enter');
  await dialog.getByRole('button',{name:'返回清理与重建',exact:true}).waitFor();assert.equal(calls.filter(c=>c.command==='execute').length,2);assert.equal(quiesced,false);
  report.checks.push('resume has separate preview/approval and keyboard execution');
  const artifacts=process.env.LINTEL_SERVICE_UI_ARTIFACTS;if(artifacts){await mkdir(artifacts,{recursive:true});await page.screenshot({path:path.join(artifacts,'service-day-receipt.png')});}
  await dialog.getByRole('button',{name:'返回清理与重建',exact:true}).click();await page.getByRole('button',{name:'深色 Night',exact:true}).click();await page.setViewportSize({width:900,height:640});
  await page.getByRole('radio',{name:/修复本地登录/}).check();if(!await control.evaluate(el=>el.open))await control.locator('summary').click();
  await control.getByRole('button',{name:'检查服务',exact:true}).click();await control.getByText('运行中',{exact:true}).waitFor();
  await control.getByRole('button',{name:'预览暂停服务',exact:true}).click({trial:true});assert.equal(await control.evaluate(el=>el.scrollWidth>el.clientWidth),false);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
  if(artifacts)await page.screenshot({path:path.join(artifacts,'service-night-control.png')});
  const cursor=await page.locator('.brand strong .brand-cursor').evaluate(el=>({animation:getComputedStyle(el).animationName,duration:getComputedStyle(el).animationDuration}));assert.deepEqual(cursor,{animation:'brand-blink',duration:'2.6s'});
  await page.emulateMedia({reducedMotion:'reduce'});assert.equal(await page.locator('.brand strong .brand-cursor').evaluate(el=>getComputedStyle(el).animationName),'none');assert.deepEqual(errors,[]);
  report.checks.push('Day/Night 1120/900 layout, preserved wordmark blink and reduced motion');report.passed=true;
  console.log('PASS: built App synthetic service inspect/approval/conflict/resume journey');
}catch(error){report.error=error.message;throw error;}finally{
  await browser?.close();const exited=new Promise(resolve=>preview.once('exit',resolve));if(preview.exitCode===null){preview.kill('SIGTERM');await exited;}
  await writeFile(process.env.LINTEL_SERVICE_UI_REPORT || path.join(os.tmpdir(),'lintel-service-ui.json'),JSON.stringify(report,null,2));
}
