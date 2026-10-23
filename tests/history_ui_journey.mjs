// Built App + finite synthetic invoke, no native WebKit or real SSH.
import assert from 'node:assert/strict';
import {spawn,spawnSync} from 'node:child_process';
import {mkdtemp,writeFile,mkdir} from 'node:fs/promises';
import os from 'node:os';import path from 'node:path';import {fileURLToPath,pathToFileURL} from 'node:url';import {stripVTControlCharacters} from 'node:util';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'),desktop=path.join(root,'apps/desktop');
const {chromium}=await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE||path.join(root,'extensions/browser/node_modules/playwright/index.mjs')).href);
const output=await mkdtemp(path.join(os.tmpdir(),'lintel-history-ui-'));
const report={runtime:'built App + real synthetic core, injected errors/late-host transport; no native WebKit/SSH',checks:[],passed:false};
const preview=spawn(process.execPath,[path.join(desktop,'node_modules/vite/bin/vite.js'),'preview','--host','127.0.0.1','--port','0','--strictPort'],{cwd:desktop,stdio:['ignore','pipe','pipe']});let browser;
const env={id:'00000000-0000-4000-8000-000000000001',name:'SYNTHETIC_DISCOVERY',root:'/synthetic/claude',status:'active',executable:null};
const digest='a'.repeat(64),job='00000000-0000-4000-8000-000000000002';
let realMode=false,realEnvironment,realPolicy,realInvoke;
let historyFail=true,discoverFail=false,hold=false,release,requestedResolve;
let requested=new Promise(r=>requestedResolve=r);const calls=[];
function summary(title){return {id:job,title,created_at:'2026-01-01',updated_at_ns:'1760000000000000000',environment_id:env.id,status:'failed'}}
async function core(p,remote=false){calls.push({...p,remote});
 if(p.command==='discover'){if(discoverFail)throw new Error('SYNTHETIC_DISCOVERY_FAILURE');return {environments:[{...env,name:remote?'REMOTE_DISCOVERY':env.name}],capabilities:[]}}
 if(p.command==='inspect')return {settings:[],assets:[],warnings:[]};
 if(p.command==='jobs_page'){
  if(hold&&!remote){requestedResolve();await new Promise(r=>release=r);return {jobs:[summary('STALE_LOCAL_ROW')],record_errors:[],digest,next_offset:null,scan_complete:true}}
  if(historyFail)throw new Error('SYNTHETIC_HISTORY_FAILURE');
  if(p.offset){assert.equal(p.expected_digest,digest);return {jobs:[summary('SECOND_PAGE')],record_errors:[],digest,next_offset:null,scan_complete:true}}
  return {jobs:remote?[summary('REMOTE_ROW')]:[],record_errors:[{job_id:job,code:'invalid_json'}],digest,next_offset:50,scan_complete:false};
 }
 if(p.command==='job_detail_page'){assert.equal(p.job_id,job);if(p.section==='steps')return {job_id:job,section:'steps',items:[{id:'step',status:'done'}],digest,next_offset:null};return {job_id:job,section:'header',items:[{id:job,status:'failed',environment_id:env.id}],digest,next_offset:null}}
 throw new Error('unexpected '+p.command);
}
try{
 const url=await new Promise((resolve,reject)=>{let text='';const timer=setTimeout(()=>reject(new Error(text)),15000);for(const stream of [preview.stdout,preview.stderr])stream.on('data',chunk=>{text+=stripVTControlCharacters(chunk.toString());const m=text.match(/http:\/\/127\.0\.0\.1:\d+\//);if(m){clearTimeout(timer);resolve(m[0])}})});
 browser=await chromium.launch({headless:true});const page=await browser.newPage({viewport:{width:1120,height:760}});page.setDefaultTimeout(10000);
 await page.exposeFunction('historyInvoke',async(command,args)=>{
  if(command==='app_preferred_languages')return ['en'];if(command==='app_set_language')return;
  if(command==='request'){if(realMode)return realInvoke(args.payload);if(args.payload.command==='job_detail_page'&&args.payload.section==='receipt')return {ok:false,error:{code:'detail_item_too_large',message:'Synthetic large receipt; use sections'}};return {ok:true,data:await core(args.payload)}};
  if(command==='remote_request'){const p=args.payload;if(p.op==='hosts')return {ok:true,data:{hosts:[{alias:'synthetic-host'}],tasks:[],installations:[],launches:[]}};if(p.op==='connect')return {ok:true,data:{status:'connected'}};if(p.op==='request')return {ok:true,data:await core(p.request,true)}}
  throw new Error('unexpected invoke '+command);
 });
 await page.addInitScript(()=>{localStorage.setItem('lintel.language','en');window.isTauri=true;window.__TAURI_INTERNALS__={invoke:(c,a)=>window.historyInvoke(c,a)}});
 await page.goto(url);await page.getByRole('button',{name:'SYNTHETIC_DISCOVERY',exact:true}).waitFor();
 await page.locator('.sidebar nav').getByRole('button',{name:'Records & recovery',exact:true}).click();await page.getByText(/SYNTHETIC_HISTORY_FAILURE/).waitFor();assert.equal(calls.filter(c=>c.command==='jobs').length,0);
 await page.locator('.sidebar nav').getByRole('button',{name:'Environments',exact:true}).click();await page.locator('.home-title').waitFor();
 historyFail=false;await page.locator('.sidebar nav').getByRole('button',{name:'Records & recovery',exact:true}).click();await page.getByText(/More candidates remain/).waitFor();assert.equal(await page.locator('.job-row').count(),0);await page.getByRole('button',{name:'Read next page',exact:true}).click();await page.getByText('SECOND_PAGE',{exact:true}).waitFor();
 await page.getByRole('button',{name:'View result',exact:true}).click();await page.getByRole('button',{name:'Execution steps',exact:true}).waitFor();await page.getByRole('button',{name:'Execution steps',exact:true}).click();await page.getByText('step',{exact:true}).waitFor();assert.equal(calls.filter(c=>c.command==='job').length,0);
 report.checks.push('discovery survives history failure; retry, empty partial page, digest-bound more, original-ID bounded section details; no unbounded fallback');
 hold=true;requested=new Promise(r=>requestedResolve=r);await page.getByRole('button',{name:'Refresh records',exact:true}).last().click();await requested;
 await page.getByRole('button',{name:'This Mac',exact:true}).click();const dialog=page.getByRole('dialog');await dialog.locator('.host-row').filter({hasText:'synthetic-host'}).getByRole('button',{name:'Connect and manage',exact:true}).click();await page.getByRole('button',{name:'REMOTE_DISCOVERY',exact:true}).waitFor();release();await page.locator('.sidebar nav').getByRole('button',{name:'Records & recovery',exact:true}).click();await page.getByText('REMOTE_ROW',{exact:true}).waitFor();assert.equal(await page.getByText('STALE_LOCAL_ROW',{exact:true}).count(),0);report.checks.push('host switch drops late old-host history');
 discoverFail=true;hold=false;await page.reload();await page.locator('.sidebar nav').getByRole('button',{name:'Records & recovery',exact:true}).click();hold=false;await page.getByText(/More candidates remain/).waitFor();report.checks.push('history remains usable when discovery fails');
 // One real synthetic core segment joins actual schemas, UI detail rendering and restore planning.
 const realHome=path.join(output,'real-home'),realState=path.join(output,'real-state'),realRoot=path.join(realHome,'claude');await mkdir(realRoot,{recursive:true});
 const realEnv={HOME:realHome,LINTEL_TEST_HOME:realHome,LINTEL_STATE_DIR:realState,PATH:'/usr/bin:/bin',TMPDIR:output};
 const binary=process.env.LINTEL_TEST_BINARY||path.join(root,'target/debug/lintel');
 realInvoke=p=>{calls.push({...p,real:true});const result=spawnSync(binary,['request'],{input:JSON.stringify(p),env:realEnv,cwd:output,encoding:'utf8',timeout:300000});if(result.error)throw result.error;return JSON.parse(result.stdout)};
 const data=p=>{const envelope=realInvoke(p);assert.equal(envelope.ok,true,JSON.stringify(envelope));return envelope.data};
 await writeFile(path.join(realRoot,'settings.json'),'{}');realEnvironment=data({command:'register',name:'REAL_HISTORY_ENV',root:realRoot});
 realPolicy=data({command:'plan_policy',environment_id:realEnvironment.id,preset:'reduce',keep_remote_control:false});
 const receipt=data({command:'execute',plan_id:realPolicy.id,approval:realPolicy.hash});assert.equal(receipt.status,'completed');
 const largeId='00000000-0000-4000-8000-000000000003';await writeFile(path.join(realState,'jobs',largeId+'.json'),JSON.stringify({id:largeId,plan_id:largeId,title:'REAL_LARGE_RECORD',created_at:'2026-01-01',status:'completed',environment_id:realEnvironment.id,warnings:[],restorable:false,steps:[{id:'large',status:'done',label:'Synthetic large step',message:'x'.repeat(3*1024*1024)}]}));
 realMode=true;await page.reload();await page.locator('.sidebar nav').getByRole('button',{name:'Records & recovery',exact:true}).click();await page.getByText('REAL_LARGE_RECORD',{exact:true}).waitFor();
 await page.locator('input[type="search"]').fill('REAL_LARGE_RECORD');await page.locator('.job-row').filter({hasText:largeId}).getByRole('button',{name:'View result',exact:true}).click();await page.getByRole('button',{name:'Execution steps',exact:true}).click();await page.getByText(/detail_item_too_large/).waitFor();
 await page.locator('input[type="search"]').fill('');await page.locator('.query-task summary').click();await page.getByLabel('Original task ID',{exact:true}).fill(realPolicy.id);await page.getByRole('button',{name:'Query original task',exact:true}).click();
 await page.getByRole('dialog').getByRole('heading',{name:'Execution result',exact:true}).waitFor();await page.getByRole('dialog').getByRole('button',{name:'Preview restore',exact:true}).click();await page.getByRole('dialog').getByRole('heading',{name:'Confirm this plan',exact:true}).waitFor();assert.ok(calls.some(c=>c.real&&c.command==='plan_restore'&&c.job_id===realPolicy.id));assert.equal(calls.filter(c=>c.real&&c.command==='execute').length,1);
 await page.getByRole('dialog').getByRole('button',{name:'Back to adjust',exact:true}).click();
 report.checks.push('actual synthetic-root core: server-side search/pages, 3 MiB receipt header and explicit oversized section, genuine small receipt TaskOutcome and original-ID restore preview without replay');
 await writeFile(path.join(realRoot,'CLAUDE.md'),'Synthetic single archive source.\n');
 const archivePlan=data({command:'plan_archive',environment_id:realEnvironment.id,categories:['instructions'],output_path:path.join(output,'single-real.age')});
 const archiveJob=data({command:'execute',plan_id:archivePlan.id,approval:archivePlan.hash,archive_passphrase:'synthetic-history-only'});assert.equal(archiveJob.status,'completed');
 await page.locator('.sidebar nav').getByRole('button',{name:'Sessions & data',exact:true}).click();
 const reader=page.locator('.archive-panel');await reader.getByRole('button',{name:'This host’s task archives',exact:true}).waitFor();
 await reader.locator('.source-open select').first().waitFor();assert.equal(await reader.locator('.source-open select').first().inputValue(),archivePlan.id,'single actual archive without initialJob selects its real original ID');
 await reader.locator('input[type="password"]').first().fill('synthetic-history-only');await reader.getByRole('button',{name:'Unlock and view',exact:true}).click();
 await reader.locator('.archive-files button').waitFor();assert.ok(calls.some(c=>c.real&&c.command==='archive_inspect'&&c.job_id===archivePlan.id));report.checks.push('single real archive: ordinary no-initialJob entry defaults to task source, selects actual ID, unlocks through real core');
 await page.locator('.sidebar nav').getByRole('button',{name:'Records & recovery',exact:true}).click();

 await page.getByLabel('All environments',{exact:true}).selectOption('attention');await page.locator('.sidebar nav').getByRole('button',{name:'Environments',exact:true}).click();await page.locator('.sidebar nav').getByRole('button',{name:'Records & recovery',exact:true}).click();assert.equal(await page.getByLabel('All environments',{exact:true}).inputValue(),'attention','history filter survives ordinary page navigation');await page.getByLabel('All environments',{exact:true}).selectOption('all');await page.getByText('REAL_LARGE_RECORD',{exact:true}).waitFor();report.checks.push('history filter survives ordinary page navigation');

 await page.locator('#main-content').evaluate(node=>{node.scrollTop=0});await page.screenshot({path:path.join(output,'history-en-1120.png'),fullPage:true});await page.locator('.language-switch select').selectOption('zh-CN');await page.setViewportSize({width:760,height:900});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'narrow history has no horizontal document overflow');await page.screenshot({path:path.join(output,'history-zh-760.png'),fullPage:true});report.passed=true;
}finally{await browser?.close();preview.kill();await writeFile(process.env.LINTEL_HISTORY_UI_REPORT||path.join(output,'report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify({...report,output}));}
