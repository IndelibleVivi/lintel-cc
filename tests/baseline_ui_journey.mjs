// Product baseline UI: built App, real core and disposable source bytes.
// Static CLI/native transport is modeled where Chromium cannot run Tauri.
// Native WebKit and Terminal evidence are independent; this report labels that.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdir,mkdtemp,readFile,realpath,writeFile} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {stripVTControlCharacters} from 'node:util';
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'),desktop=path.join(repo,'apps/desktop');
const {chromium}=await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE||path.join(repo,'extensions/browser/node_modules/playwright/index.mjs')).href);
const fixture=await realpath(await mkdtemp(path.join(os.tmpdir(),'lintel-baseline-ui-')));
const home=path.join(fixture,'home'),state=path.join(fixture,'state'),root=path.join(home,'deep config 配置 with spaces'),project=path.join(home,'different project 项目');
const password='synthetic-archive-passphrase',secret='SYNTHETIC_PRIVATE_SENTINEL_NOT_FOR_OPERATION_PACKET';
await mkdir(home);await mkdir(project);
const versions=path.join(home,'claude/versions');await mkdir(versions,{recursive:true});
const inert=path.join(versions,'2.1.283');await writeFile(inert,'#!/bin/sh\nexit 0\n');await mkdir(path.join(home,'.local/bin'),{recursive:true});
const {symlink}=await import('node:fs/promises');await symlink(inert,path.join(home,'.local/bin/claude'));const {chmod}=await import('node:fs/promises');await chmod(inert,0o700);
const actor={...process.env,HOME:home,LINTEL_TEST_HOME:home,LINTEL_STATE_DIR:state,PATH:path.join(home,'.local/bin')};
const runner=process.env.LINTEL_FIXTURE_RUNNER||path.join(repo,'target/debug/lintel'),calls=[],copied=[],resources=[];
let queue=Promise.resolve(),clipboardFail=false,launchFailure=false;
function processCall(args,payload,env=actor){return new Promise((resolve,reject)=>{const child=spawn(runner,args,{env,stdio:['pipe','pipe','pipe']});let out='',err='';const timeout=setTimeout(()=>{child.kill();reject(new Error('core timeout'))},55000);child.stdout.on('data',d=>out+=d);child.stderr.on('data',d=>err+=d);child.once('error',reject);child.once('close',()=>{clearTimeout(timeout);assert.ok(!out.includes(password)&&!err.includes(password));try{resolve(JSON.parse(out))}catch{reject(new Error('invalid envelope: '+err))}});child.stdin.end(payload?JSON.stringify(payload):'')})}
function core(payload){calls.push(payload);const result=queue.then(()=>processCall(['request'],payload));queue=result.catch(()=>{});return result}
async function data(payload){const result=await core(payload);assert.ok(result.ok,result.error?.message);return result.data}
const preview=spawn(process.execPath,[path.join(desktop,'node_modules/vite/bin/vite.js'),'preview','--host','127.0.0.1','--port','0','--strictPort'],{cwd:desktop,stdio:['ignore','pipe','pipe']});
const report={fixture,runtime:'built App + real core; modeled Tauri static/clipboard/launch-error transport, not native WebKit or actual Claude',checks:[],passed:false};
let browser,page;
try{
 const url=await new Promise((resolve,reject)=>{let out='';const timer=setTimeout(()=>reject(new Error('preview timeout: '+out)),15000);preview.once('error',reject);for(const stream of [preview.stdout,preview.stderr])stream.on('data',d=>{out+=stripVTControlCharacters(d.toString());const match=out.match(/http:\/\/127\.0\.0\.1:\d+\//);if(match){clearTimeout(timer);resolve(match[0])}})});
 report.url=url;browser=await chromium.launch({headless:true});page=await browser.newPage({viewport:{width:1120,height:760}});page.setDefaultTimeout(12000);const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.exposeFunction('syntheticClipboard',async text=>{if(clipboardFail)throw new Error('synthetic clipboard failure');copied.push(text)});
 await page.exposeFunction('syntheticInvoke',async(command,args)=>{
  if(command==='open_resource'){resources.push(args.resource);return null}
  if(command==='inspect_cli'){
   if(!args.executable.startsWith('/'))return {ok:false,error:{code:'cli_absolute_path_required',message:'选择 CLI 的完整路径'}};
   const version=await processCall(['version','--json']),context=await processCall(['context','--json']);
   return {ok:true,data:{executable:runner,version:version.data,context:context.data,candidate:{status:'unknown'},checked_at:1}};
  }
  if(command==='remote_request')return {ok:true,data:{hosts:[],tasks:[],installations:[]}};
  assert.equal(command,'request');if(args.payload.command==='launch_request'&&launchFailure){calls.push(args.payload);return {ok:false,error:{code:'synthetic_connection_lost',message:'合成启动回复丢失；这里没有执行 Terminal launcher'}}}
  return core(args.payload);
 });
 await page.addInitScript(()=>{if(!localStorage.getItem('lintel.selected'))localStorage.setItem('lintel.selected','stale-removed-environment');window.isTauri=true;window.__TAURI_INTERNALS__={invoke:(command,args)=>window.syntheticInvoke(command,args)};Object.defineProperty(navigator,'clipboard',{value:{writeText:text=>window.syntheticClipboard(text)}})});
 await page.goto(url);await page.locator('.home-title').waitFor();
 assert.equal(await page.getByRole('alert').count(),0);assert.equal(calls.some(c=>c.command==='inspect'&&c.environment_id==='stale-removed-environment'),false);assert.equal(await page.locator('.task-home [data-task-id]').count(),0);assert.equal(await page.getByLabel('当前环境').inputValue(),'');
 assert.equal(await page.getByRole('button',{name:'本机',exact:true}).isEnabled(),true);
 await page.screenshot({path:path.join(fixture,'home-empty-day.png')});
 for (const [width,height,theme] of [[1120,760,'light'],[900,640,'dark'],[1440,900,'light']]) {
  await page.setViewportSize({width,height});await page.evaluate(theme=>document.documentElement.dataset.theme=theme,theme);
  assert.ok(await page.locator('.home-content').evaluate(el=>el.scrollHeight<=el.clientHeight+1),'quiet home fits without scrolling');
  const play=await page.locator('.clawd-pull-trigger').boundingBox(),greeting=await page.locator('.home-title').boundingBox();
  assert.ok(play.y+play.height+4<=greeting.y,'Clawd play button stays above the greeting at every window size');
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
  await page.screenshot({path:path.join(fixture,'quiet-home-'+width+'-'+theme+'.png')});
 }
 await page.setViewportSize({width:1120,height:760});
 await page.getByRole('button',{name:'开始一项任务',exact:true}).click();assert.equal(await page.locator('.task-home [data-task-id]').count(),6);
 await page.keyboard.press('Escape');await page.getByRole('button',{name:'开始一项任务',exact:true}).waitFor({state:'visible'});
 assert.equal(await page.getByRole('button',{name:'开始一项任务',exact:true}).evaluate(el=>el===document.activeElement),true);
 await page.getByRole('button',{name:'开始一项任务',exact:true}).click();
 await page.locator('.task-home [data-task-id=browser_profile] button').click();const dialog=page.getByRole('dialog');
 await dialog.locator('.browser-local-scope').getByText('本机浏览器 · 精确 profile',{exact:true}).waitFor();await page.keyboard.press('Escape');
 await page.getByRole('button',{name:'终端与 Agent',exact:true}).click();await page.getByRole('heading',{name:'终端与 Agent',exact:true}).waitFor();
 assert.equal(await page.getByRole('button',{name:'核对 CLI 与上下文',exact:true}).isDisabled(),true);
 await page.getByLabel('Lintel CLI 完整路径').fill('relative');await page.getByRole('button',{name:'核对 CLI 与上下文',exact:true}).click();await page.getByRole('alert').getByText(/cli_absolute_path_required/).waitFor();
 const beforeContext=await processCall(['context','--json']);assert.equal(beforeContext.data.state.exists,true); // App discovery created its own journal; static query itself is independently tested by CLI/Rust.
 report.checks.push('A01/A03: quiet greeting home, six actionable task choices, host selector, local browser scope and CLI entry/error');
 await mkdir(path.join(root,'projects/synthetic/memory'),{recursive:true});
 const transcript=[
  {type:'user',timestamp:'2026-10-05T00:00:00Z',message:{role:'user',content:'继续实现上下文 · '+secret}},
  {type:'assistant',message:{role:'assistant',content:[{type:'text',text:'已决定配置 root 与项目 cwd 分开。'},{type:'thinking',thinking:'opaque content',signature:'SYNTHETIC_SIGNATURE_BYTES'}]}},
  {future_record:true,custom:'未知字段必须保留'},
 ];
 const raw=transcript.map(x=>JSON.stringify(x)).join('\r\n')+'\r\n{malformed record\r\n'+Array.from({length:900},(_,i)=>JSON.stringify({type:'user',message:{role:'user',content:'long record '+i+' '+('正文'.repeat(90))}})).join('\n')+'\n';
 await writeFile(path.join(root,'CLAUDE.md'),'# Synthetic instructions\n');await writeFile(path.join(root,'projects/synthetic/session.jsonl'),raw);await writeFile(path.join(root,'projects/synthetic/memory/MEMORY.md'),'Synthetic reference memory\n');
 const original=await readFile(path.join(root,'projects/synthetic/session.jsonl'));
 const source=await data({command:'register',root,name:'同名环境 / 同名环境 A very long mixed English 中文工作名称'});
 const secondRoot=path.join(home,'another root with same display name');await mkdir(secondRoot);
 const second=await data({command:'register',root:secondRoot,name:source.name});await page.reload();await page.getByLabel('当前环境').selectOption(source.id);
 await page.locator('.sidebar nav').getByRole('button',{name:'保护方案',exact:true}).click();
 const dock=page.locator('.workspace-actions .action-bar');
 await dock.getByRole('button',{name:'预览变更',exact:true}).waitFor();
 assert.equal(await page.locator('main .action-bar').count(),0);
 for (const [width,height] of [[1120,760],[900,640],[1440,900]]) {
  await page.setViewportSize({width,height});
  await page.locator('main').evaluate(el=>el.scrollTop=0);const before=await dock.boundingBox();
  await page.locator('main').evaluate(el=>el.scrollTop=el.scrollHeight);const after=await dock.boundingBox();
  assert.ok(Math.abs(before.y-after.y)<1,'action footer stays at the same position while content scrolls');
  assert.ok(after.y+after.height<=height);
 }
 await page.setViewportSize({width:1120,height:760});
 await page.locator('.sidebar nav').getByRole('button',{name:'工作保全',exact:true}).click();await page.getByLabel(/保全并准备新环境/).check();await page.getByLabel('新环境名称（可选）').fill('Continuation config');
 await page.getByLabel('当前环境').selectOption(second.id);assert.equal(await page.getByLabel('新环境名称（可选）').count(),0);await page.getByLabel('当前环境').selectOption(source.id);assert.equal(await page.getByLabel('新环境名称（可选）').inputValue(),'Continuation config');
 await page.getByRole('button',{name:'预览保全计划',exact:true}).click();await dialog.locator('.approval-root').getByText(root,{exact:true}).waitFor();await dialog.getByRole('region',{name:'最终迁入清单',exact:true}).waitFor();
 const previewPayload=calls.findLast(c=>c.command==='plan_preserve');assert.equal(previewPayload.activate.instructions,false);
 for(const [width,height,theme]of[[1120,760,'light'],[900,640,'dark'],[1440,900,'light']]){await page.setViewportSize({width,height});await page.evaluate(theme=>document.documentElement.dataset.theme=theme,theme);assert.ok(await dialog.evaluate(el=>el.scrollWidth<=el.clientWidth));const body=await dialog.locator('.modal-body').boundingBox();assert.ok(body.height>=180,'approval body must remain readable with a long frozen root');const r=await dialog.getByRole('button',{name:'批准并执行',exact:true}).boundingBox();assert.ok(r.y+r.height<=height);await page.screenshot({path:path.join(fixture,'preserve-review-'+width+'-'+theme+'.png')})}
 await page.keyboard.press('Escape');await page.setViewportSize({width:1120,height:760});
 const archive=await data({command:'plan_archive',environment_id:source.id,categories:['instructions','memory','sessions']});
 const receipt=await data({command:'execute',plan_id:archive.id,approval:archive.hash,archive_passphrase:password});
 await page.getByRole('button',{name:'重新检查环境',exact:true}).click();await page.waitForFunction(()=>!document.querySelector('[aria-label="重新检查环境"]').disabled);await page.getByRole('button',{name:'会话与资料',exact:true}).click();
 const reader=page.locator('.archive-panel');await reader.getByLabel('归档口令',{exact:true}).fill(password);await reader.getByRole('button',{name:'解锁并查看',exact:true}).click();
 await reader.getByRole('button',{name:/session.jsonl/}).click();await reader.getByText('已决定配置 root 与项目 cwd 分开。',{exact:true}).waitFor();await reader.getByText(/不透明 thinking/).waitFor();
 await reader.getByText(/未知字段必须保留/).waitFor();await reader.getByText(/malformed record/).waitFor();
 assert.equal(await reader.locator('.record-meta input[type=checkbox]').count()>0,true);
 const checks=reader.locator('.record-meta input[type=checkbox]');await checks.first().check();await checks.nth(1).check();await reader.getByRole('button',{name:'用选定片段生成交接稿',exact:true}).click();
 const draft=await reader.getByLabel('工作交接稿',{exact:true}).inputValue();assert.ok(draft.includes(secret));assert.ok(!draft.includes('opaque content')&&!draft.includes('SYNTHETIC_SIGNATURE_BYTES'));
 await reader.getByLabel('工作交接稿',{exact:true}).fill(draft+'\n下一步：在正确的项目里接着做。');
 await reader.getByRole('button',{name:'审阅稿件与继续目标',exact:true}).click();const launch=reader.locator('.session-continuation');
 await launch.getByLabel('项目工作目录',{exact:true}).fill(project);await launch.getByRole('button',{name:'核对启动目标',exact:true}).click();
 await launch.locator('.launch-review').getByText(project,{exact:true}).waitFor();await launch.getByLabel(/已审阅当前稿件/).check();
 clipboardFail=true;const launchesBefore=calls.filter(c=>c.command==='launch_request').length;
 await launch.getByRole('button',{name:'复制上下文并打开新会话',exact:true}).click();await launch.getByRole('alert').getByText(/复制失败/).waitFor();
 assert.equal(calls.filter(c=>c.command==='launch_request').length,launchesBefore);
 clipboardFail=false;launchFailure=true;await launch.getByRole('button',{name:'复制上下文并打开新会话',exact:true}).click();await launch.getByText('复制：已复制这份审阅稿',{exact:true}).waitFor();await launch.getByText(/结果待核对；保留原启动请求/).waitFor();
 const requested=calls.findLast(c=>c.command==='launch_request');await launch.getByRole('button',{name:'核对原启动请求',exact:true}).click();await launch.getByText(/尚未请求启动；可批准原计划或继续核对。/).waitFor();assert.deepEqual(calls.findLast(c=>c.command==='launch_query'),{command:'launch_query',request_id:requested.request_id});assert.equal(calls.filter(c=>c.command==='launch_request').length,launchesBefore+1);assert.equal(copied.filter(text=>text.includes(secret)).length,1);
 report.checks.push('A02/A05/A09: same-name draft isolation, frozen root/file purpose preview, responsive reachable approval; A07/A08/A10: real package, bounded structured reader/unknown/thinking exclusion, reviewed text, clipboard refusal and original-request-only launch error');
 await page.getByRole('button',{name:'终端与 Agent',exact:true}).click();assert.ok(!(await page.evaluate(()=>JSON.stringify(localStorage))).includes(password));assert.ok(!(await page.evaluate(()=>JSON.stringify(localStorage))).includes(secret));
 await page.getByLabel('Lintel CLI 完整路径').fill(runner);await page.getByRole('button',{name:'核对 CLI 与上下文',exact:true}).click();await page.getByLabel('Agent 操作交接包').waitFor();
 const packet=JSON.parse(await page.getByLabel('Agent 操作交接包').inputValue());assert.equal(packet.state_context.path,state);assert.equal(packet.state_context.home,home);assert.equal(packet.executor.path,runner);assert.ok(!JSON.stringify(packet).includes(secret)&&!JSON.stringify(packet).includes(password));
 await page.getByLabel(/已核对这一版目标/).check();await page.getByRole('button',{name:'复制交接包',exact:true}).click();await page.getByText(/已复制这一版操作交接包/).waitFor();
 await page.screenshot({path:path.join(fixture,'agent-night.png')});
 const accepted={...receipt,status:'accepted',steps:[]};
 await writeFile(path.join(state,'jobs',receipt.id+'.json'),JSON.stringify(accepted));
 const executions=calls.filter(c=>c.command==='execute').length;
 await page.locator('.sidebar nav').getByRole('button',{name:'记录与恢复',exact:true}).click();
 await page.getByRole('button',{name:'刷新',exact:true}).click();await page.locator('.job-row').filter({hasText:receipt.id}).getByRole('button',{name:'查看结果',exact:true}).click();
 await dialog.getByRole('button',{name:'交给 Agent',exact:true}).click();
 await page.getByRole('button',{name:'核对 CLI 与上下文',exact:true}).click();
 const acceptedPacket=JSON.parse(await page.getByLabel('Agent 操作交接包').inputValue());
 assert.equal(acceptedPacket.job.id,receipt.id);assert.equal(acceptedPacket.next_action.mode,'query_original_only');assert.ok(acceptedPacket.next_action.commands[0].includes('job show'));
 assert.ok(!JSON.stringify(acceptedPacket).includes(secret)&&!JSON.stringify(acceptedPacket).includes(password));assert.equal(calls.filter(c=>c.command==='execute').length,executions);
 report.checks.push('A13: modeled accepted original receipt read through real job API → Agent packet retains exact job ID and query-only command; zero new execute');
 // Model a durable native acceptance with a lost response; re-opening the App
 // reads metadata through the actual core query/list, never another launch.
 await mkdir(path.join(state,'launches'),{recursive:true});
 await writeFile(path.join(state,'launches',requested.request_id+'.json'),JSON.stringify({request_id:requested.request_id,status:'launch_intent',mode:'interactive',root,project_cwd:project,executable:inert,recorded_at:'2026-10-05T12:00:00Z'}));
 await page.locator('.sidebar nav').getByRole('button',{name:'记录与恢复',exact:true}).click();
 const launchRecords=page.locator('.launch-records');await launchRecords.getByRole('button').filter({hasText:requested.request_id}).click();
 await launchRecords.getByRole('button',{name:'把原启动请求交给 Agent',exact:true}).click();
 await page.getByRole('button',{name:'核对 CLI 与上下文',exact:true}).click();
 const launchPacket=JSON.parse(await page.getByLabel('Agent 操作交接包').inputValue());
 assert.equal(launchPacket.launch_request.request_id,requested.request_id);assert.equal(launchPacket.next_action.mode,'query_original_only');assert.ok(launchPacket.next_action.commands[0].includes('launch query'));
 assert.equal(calls.filter(c=>c.command==='launch_request').length,launchesBefore+1);
 report.checks.push('A06/A10/A13: modeled durable launch intent survives closing the view, real core list/query → Agent retains original startup ID; no second launch');


 await page.getByRole('button',{name:'会话与资料',exact:true}).click();assert.equal(await reader.getByLabel('归档口令',{exact:true}).inputValue(),'');
 assert.deepEqual(await readFile(path.join(root,'projects/synthetic/session.jsonl')),original);assert.ok(receipt.archive_path);
 await page.emulateMedia({reducedMotion:'reduce'});await page.getByRole('button',{name:'环境',exact:true}).click();await page.getByRole('button',{name:'收起侧栏',exact:true}).click();await page.screenshot({path:path.join(fixture,'home-collapsed-reduced-motion.png')});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.getByRole('button',{name:'展开侧栏',exact:true}).click();
 await page.getByRole('button',{name:'帮助',exact:true}).focus();await page.keyboard.press('Enter');await dialog.getByRole('heading',{name:'使用 Lintel'}).waitFor();assert.equal(await dialog.locator('[data-task-id]').count(),6);await page.keyboard.press('Escape');assert.equal(await page.getByRole('button',{name:'帮助',exact:true}).evaluate(el=>el===document.activeElement),true);
 assert.deepEqual(errors,[]);report.checks.push('A12/A13 metadata view: explicit CLI/state check and operation packet has no working body/password; independent native checker required; A16/A18: production render, 1120/900/1440, Day/Night, collapse, reduced motion, keyboard dialog return and shared task map');
 report.passed=true;
}finally{if(page && !report.passed)await page.screenshot({path:path.join(fixture,'failure.png')}).catch(()=>{});await browser?.close();preview.kill('SIGTERM');await writeFile(process.env.LINTEL_BASELINE_UI_REPORT||path.join(fixture,'report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2))}
