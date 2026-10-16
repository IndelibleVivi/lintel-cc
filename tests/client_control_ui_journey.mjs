// Client-control UI: built App + real core, disposable synthetic roots.
// Real core answers every product request; only the native-only transports
// Chromium cannot provide are modeled here — locale commands, the clipboard and
// the Finder reveal. The modeled Finder invoke only records the call; it never
// opens Finder, and the report labels it as modeled native, not a mock core.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {chmod,mkdir,mkdtemp,readFile,readdir,realpath,symlink,writeFile} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {stripVTControlCharacters} from 'node:util';

const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'),desktop=path.join(repo,'apps/desktop');
const {chromium}=await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE||path.join(repo,'extensions/browser/node_modules/playwright/index.mjs')).href);
const fixture=await realpath(await mkdtemp(path.join(os.tmpdir(),'lintel-client-control-ui-')));
const home=path.join(fixture,'home'),state=path.join(fixture,'state');
// Environment A is the default config root (home/.claude); environment B is a
// separate root. All mutations stay inside these synthetic roots.
const rootA=path.join(home,'.claude'),rootB=path.join(home,'second config 第二环境'),project=path.join(home,'selected project 项目');
const rawDirectory=path.join(home,'api-records',...Array.from({length:10},(_,index)=>`synthetic API 正文目录 ${index}`));
const password='synthetic-client-control-passphrase';
const userId='SYNTHETIC_USER_ID_MUST_NOT_PERSIST_OR_LEAK_01';
await mkdir(home);await mkdir(rootA,{recursive:true});await mkdir(rootB,{recursive:true});await mkdir(project,{recursive:true});
// Static inert Claude at the order's pinned version; never executed as a client.
const versions=path.join(home,'claude/versions'),inert=path.join(versions,'2.1.285');
await mkdir(versions,{recursive:true});await writeFile(inert,'#!/bin/sh\nexit 0\n');await chmod(inert,0o700);
await mkdir(path.join(home,'.local/bin'),{recursive:true});await symlink(inert,path.join(home,'.local/bin/claude'));
// Seeded residue + settings + identity in environment A.
await mkdir(path.join(rootA,'debug'),{recursive:true});
const debugFile=path.join(rootA,'debug','synthetic-debug.log');
await writeFile(debugFile,'synthetic debug line\n');
await writeFile(path.join(rootA,'settings.json'),JSON.stringify({env:{OTEL_LOG_USER_PROMPTS:'1',OTEL_EXPORTER_OTLP_ENDPOINT:'https://collector.invalid/v1',OTEL_LOG_RAW_API_BODIES:`file:${rawDirectory}`} ,synthetic_unrelated:'keep',synthetic_note:'before'},null,2));
await writeFile(path.join(home,'.claude.json'),JSON.stringify({userID:userId,oauthAccount:{synthetic:true}},null,2));

const actor={...process.env,HOME:home,LINTEL_TEST_HOME:home,LINTEL_STATE_DIR:state,PATH:path.join(home,'.local/bin')};
const runner=process.env.LINTEL_FIXTURE_RUNNER||path.join(repo,'target/debug/lintel');
const calls=[],copied=[],reveals=[],resources=[];
let queue=Promise.resolve(),clipboardFail=false;
function processCall(args,payload,env=actor){return new Promise((resolve,reject)=>{
 const child=spawn(runner,args,{env,stdio:['pipe','pipe','pipe']});let out='',err='',inputError;
 const composite=payload?.command==='execute'&&coreResponses.some(item=>item.envelope.data?.id===payload.plan_id&&item.envelope.data?.kind==='client_workflow');
 const budget=composite?120000:55000;
 const timeout=setTimeout(()=>{child.kill();reject(new Error('core timeout: '+payload?.command))},budget);
 child.stdout.on('data',d=>out+=d);child.stderr.on('data',d=>err+=d);
 child.stdin.once('error',error=>{inputError=error});
 child.once('error',error=>{clearTimeout(timeout);reject(error)});
 child.once('close',(code,signal)=>{clearTimeout(timeout);try{
  assert.ok(!out.includes(password)&&!err.includes(password),'passphrase must never appear in core output');
  if(code!==0||signal)throw new Error(`core exited ${code??signal}: ${err}`);
  if(inputError)throw new Error(`core stdin failed: ${inputError.code}`);
  try{resolve(JSON.parse(out))}catch{throw new Error('invalid envelope: '+err)}
 }catch(error){reject(error)}});
 child.stdin.end(payload?JSON.stringify(payload):'');
})}
function core(payload){calls.push(payload);const result=queue.then(()=>processCall(['request'],payload));queue=result.catch(()=>{});return result}
async function data(payload){const result=await core(payload);assert.ok(result.ok,`${payload.command}: ${result.error?.message}`);return result.data}
// Captured real-core envelopes so the journey can bind to a persisted plan/job
// ID without depending on a collapsed <details> for its innerText.
const coreResponses=[];
function coreTracked(payload){const result=core(payload);result.then(envelope=>coreResponses.push({command:payload.command,envelope})).catch(()=>{});return result}
async function dataTracked(payload){const result=await coreTracked(payload);assert.ok(result.ok,`${payload.command}: ${result.error?.message}`);return result.data}
function lastPlanId(command){const entry=[...coreResponses].reverse().find(item=>item.command===command&&item.envelope.ok);return entry?.envelope.data?.id}
async function waitForResult(command,previous){const deadline=Date.now()+(command==='execute'?125000:60000);while(Date.now()<deadline){const completed=coreResponses.filter(item=>item.command===command);if(completed.length>previous){assert.ok(completed.at(-1).envelope.ok,JSON.stringify(completed.at(-1).envelope.error));return completed.at(-1).envelope.data}await new Promise(resolve=>setTimeout(resolve,20))}throw new Error(`no completed ${command}`)}

const L={envDetails:'环境详情',tabState:'本地状态',tabCompare:'变化与对照',reveal:'显示原始值',hide:'隐藏原始值',locate:'在 Finder 中定位',combineReduce:'同时减少正文记录',combineReset:'同时重置本地客户端标识',combinePreview:'预览合成计划',writersStopped:'已关闭目标的终端、IDE 与自动重启来源',approve:'批准并执行',previewPolicy:'预览变更',configRead:'读取',configReinspect:'重新观察',configPreviewEdit:'预览并应用',restoreInspect:'读取可恢复字段',restorePreview:'预览恢复',snapshotSave:'保存快照',snapshotCompare:'比较',snapshotPair:'并排对照',applyNormal:'将普通配置应用到目标环境',recipeApply:'应用配方',recipeSave:'保存为个人默认',recipeReduce:'减少正文记录'};

async function revealPolicyRecipes(){const sheet=page.locator('.policy-alternatives');if(await sheet.count()&&!await sheet.evaluate(element=>element.open))await sheet.locator(':scope > summary').click()}

const preview=spawn(process.execPath,[path.join(desktop,'node_modules/vite/bin/vite.js'),'preview','--host','127.0.0.1','--port','0','--strictPort'],{cwd:desktop,stdio:['ignore','pipe','pipe']});
const report={fixture,runtime:'built App + real core; modeled native locale/clipboard/Finder only — not native WebKit, not a mock core',checks:[],passed:false};
let browser,page;
try{
 const url=await new Promise((resolve,reject)=>{let out='';const timer=setTimeout(()=>reject(new Error('preview timeout: '+out)),15000);preview.once('error',reject);for(const stream of [preview.stdout,preview.stderr])stream.on('data',d=>{out+=stripVTControlCharacters(d.toString());const match=out.match(/http:\/\/127\.0\.0\.1:\d+\//);if(match){clearTimeout(timer);resolve(match[0])}})});
 report.url=url;browser=await chromium.launch({headless:true});page=await browser.newPage({viewport:{width:1120,height:760}});page.setDefaultTimeout(12000);const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.exposeFunction('syntheticClipboard',async text=>{if(clipboardFail)throw new Error('synthetic clipboard failure');copied.push(text)});
 // The modeled native surface: locale commands, Finder reveal, resources.
 // Every product request is answered by the REAL core runner below.
 await page.exposeFunction('syntheticInvoke',async(command,args)=>{
  if(command==='app_set_language'||command==='app_preferred_languages')return ['zh-CN'];
  if(command==='open_resource'){resources.push(args.resource);return null}
  if(command==='reveal_local_item'){reveals.push(args.payload);return {status:'revealed',environment_id:args.payload.environment_id,path:args.payload.path}}
  if(command==='remote_request')return {ok:true,data:{hosts:[],tasks:[],installations:[],aliases:[],coverage:'synthetic'}};
  if(command==='network_request')return {ok:false,error:{code:'synthetic_network_unmodeled',message:'客户端控制旅程不驱动网络通道'}};
  assert.equal(command,'request','only the product request channel may reach real core');
  return await coreTracked(args.payload);
 });
 await page.addInitScript(()=>{try{localStorage.setItem('lintel.language','zh-CN')}catch{};window.isTauri=true;window.__TAURI_INTERNALS__={invoke:(command,args)=>window.syntheticInvoke(command,args)};Object.defineProperty(navigator,'clipboard',{value:{writeText:text=>window.syntheticClipboard(text)}})});

 // Register the two synthetic environments through the real core.
 const envA=await data({command:'register',root:rootA,name:'Synthetic A'});
 const envB=await data({command:'register',root:rootB,name:'Synthetic B'});

 await page.goto(url);
 await page.locator('.home-title').waitFor();
 assert.equal(await page.locator('.clawd-pull-trigger').count()>0,true,'the original home Clawd stays');
 await page.getByLabel('当前环境',{exact:true}).selectOption(envA.id);
 if(await page.getByRole('tab').count()===0)await page.getByRole('button',{name:L.envDetails,exact:true}).click();
 await page.getByRole('tab',{name:L.tabState,exact:true}).click();
 const debugRow=page.locator('.residue-location[data-category="debug_logs"]');
 await debugRow.first().waitFor();
 // Open the known-location row so its bounded entry list is visible before selecting.
 await debugRow.first().locator('summary').click();
 await page.getByText('调试日志',{exact:true}).first().waitFor({state:'attached'});
 report.checks.push('home Clawd kept; env details opened; local-data tab renders the real residue inventory');

 // --- Identity: explicit reveal -> copy -> hide; raw must never reach storage ---
 const identity=page.locator('.identity-panel');
 await identity.getByRole('button',{name:L.reveal,exact:true}).click();
 const copyButton=identity.locator('.identity-value .icon-button').first();
 await copyButton.waitFor();
 await copyButton.click();
 assert.ok(copied.some(text=>text===userId),'explicit reveal + copy places the raw identifier on the clipboard');
 await identity.getByRole('button',{name:L.hide,exact:true}).click();
 const storage=await page.evaluate(()=>JSON.stringify(localStorage));
 assert.ok(!storage.includes(userId),'the raw identifier must never enter localStorage');
 assert.ok(!storage.includes(password),'the passphrase must never enter localStorage');
 report.checks.push('identity reveal/copy/hide: raw identifier copied in memory, never written to localStorage');

 // --- Modeled Finder reveal: recorded call only, never a real Finder open ---
 await debugRow.first().locator('.residue-entry-list input[type=checkbox]').first().check();
 await page.locator('.residue-tools-row').getByRole('button',{name:L.locate,exact:true}).click();
 assert.deepEqual(reveals.at(-1),{environment_id:envA.id,path:debugFile,extra_roots:[]},'Finder reveal carries the exact environment and path');
 report.checks.push(`modeled native reveal_local_item call recorded: ${JSON.stringify(reveals.at(-1))}`);

 // --- Combined single plan: reduce content recording + reset userID + clear residue ---
 await page.getByLabel(L.combineReduce).check();
 await page.getByLabel(L.combineReset).check();
 await page.getByLabel(L.writersStopped).check();
 const previousCombined=coreResponses.filter(item=>item.command==='plan_client_workflow').length;
 await page.getByRole('button',{name:L.combinePreview,exact:true}).click();
 await waitForResult('plan_client_workflow',previousCombined);
 const dialog=page.getByRole('dialog');
 async function approveCurrent(){const previous=coreResponses.filter(item=>item.command==='execute').length;await dialog.getByRole('button',{name:L.approve,exact:true}).click();await waitForResult('execute',previous);}
 await dialog.locator('.approval-root, .plan-target').first().waitFor();
 // Bind to the real persisted plan ID rather than a collapsed <details> text.
 const planId=lastPlanId('plan_client_workflow');
 assert.equal(typeof planId,'string','the real core persisted a combined plan ID');
 const planPayloads=calls.filter(c=>c.command==='plan_client_workflow');
 assert.equal(planPayloads.length,1,'the combined journey is ONE preview');
 assert.ok(Array.isArray(planPayloads[0].residue_paths)&&planPayloads[0].residue_paths.includes(debugFile),'the frozen plan names the selected residue original');
 assert.equal(planPayloads[0].reset_identity,true);
 assert.equal(planPayloads[0].archive_before_clear,true);
 // One approval: passphrase fields only appear when the frozen plan requires them.
 const passFields=dialog.locator('.archive-password input[type=password]');
 if(await passFields.count()>=1){await passFields.nth(0).fill(password);if(await passFields.count()>1)await passFields.nth(1).fill(password)}
 await approveCurrent();
 await dialog.locator('.receipt-id code').waitFor();
 const receiptId=await dialog.locator('.receipt-id code').innerText();
 assert.equal(calls.filter(c=>c.command==='execute').length,1,'exactly one approval executes the combined plan');
 // Real core byte verification: userID moved, residue original cleared, settings read back.
 const clientState=JSON.parse(await readFile(path.join(home,'.claude.json'),'utf8'));
 assert.ok(!('userID' in clientState),'identity reset removes userID');
 assert.ok(clientState.oauthAccount,'identity reset does NOT log out (oauthAccount preserved)');
 const debugEntries=await readdir(path.join(rootA,'debug')).catch(()=>[]);
 assert.equal(debugEntries.includes('synthetic-debug.log'),false,'the selected residue original is cleared');
 const settingsAfter=JSON.parse(await readFile(path.join(rootA,'settings.json'),'utf8'));
 assert.ok(['0','false'].includes(String(settingsAfter.env?.OTEL_LOG_USER_PROMPTS)),'content recording is disabled in the read-back settings');
 assert.equal(settingsAfter.synthetic_unrelated,'keep','unrelated settings stay untouched');
 // Core-truth confirmation through the real inspect command, not just file bytes.
 const inspectAfter=await data({command:'inspect',environment_id:envA.id});
 const promptSetting=inspectAfter.settings.find(setting=>setting.key==='OTEL_LOG_USER_PROMPTS');
 assert.ok(promptSetting&&['0','false'].includes(promptSetting.value),'real core reports the configured content flag as false');
 assert.equal(inspectAfter.policy.rules.find(rule=>rule.key==='OTEL_LOG_USER_PROMPTS').signal_enabled,false);
 assert.ok(!JSON.stringify(inspectAfter).includes(userId),'identity raw value never appears in the environment inspection');
 report.checks.push('real-core bytes: single approval removed userID (no logout), cleared the selected residue original, disabled content recording, kept unrelated settings');

 // --- Repeat the original task ID: query, never replay ---
 await dialog.getByRole('button',{name:'查看记录',exact:true}).click().catch(()=>{});
 const original=await data({command:'job',job_id:receiptId});
 assert.equal(original.id,receiptId);
 assert.equal(calls.filter(c=>c.command==='execute').length,1,'reopening the original task never replays the execution');
 assert.equal(original.plan_id,planId,'the queried original task still names the frozen combined plan');
 report.checks.push(`repeat original ID ${receiptId}: query-only, zero additional execute`);

 // Open the separately encrypted residue manifest and restore the selected
 // absent original. The identity bundle is not substituted for work content.
 await page.locator('.sidebar nav').getByRole('button',{name:'环境',exact:true}).click();
 if(await page.getByRole('tab').count()===0)await page.getByRole('button',{name:L.envDetails,exact:true}).click();
 await page.getByRole('tab',{name:L.tabState,exact:true}).click();
 const archiveReader=page.locator('.residue-archive-panel');
 await archiveReader.locator('summary').click();
 await archiveReader.getByLabel('原任务 ID',{exact:true}).fill(receiptId);
 await archiveReader.getByLabel('归档口令（仅本次交互）',{exact:true}).fill(password);
 const previousManifest=coreResponses.filter(item=>item.command==='residue_archive_inspect').length;
 await archiveReader.getByRole('button',{name:'打开加密清单',exact:true}).click();
 await waitForResult('residue_archive_inspect',previousManifest);
 await archiveReader.getByRole('checkbox',{name:`恢复 ${debugFile}`,exact:true}).check();
 const previousResidueRestore=coreResponses.filter(item=>item.command==='plan_residue_restore').length;
 await archiveReader.getByRole('button',{name:'预览恢复 1 项',exact:true}).click();
 await waitForResult('plan_residue_restore',previousResidueRestore);
 await dialog.locator('.plan-target').waitFor();
 assert.equal(await dialog.locator('.archive-password input').first().inputValue(),password,'reader passphrase reaches only the current approval');
 await approveCurrent();
 await dialog.locator('.receipt-id code').waitFor();
 assert.equal(await readFile(debugFile,'utf8'),'synthetic debug line\n');
 await dialog.getByRole('button',{name:'查看记录',exact:true}).click();
 report.checks.push('independent encrypted residue manifest opened and selected absent original restored through one new approval');

 // --- Advanced config: project_cwd inspect, read, edit, restore one selected field ---
 await page.locator('.sidebar nav').getByRole('button',{name:'环境',exact:true}).click();
 await page.getByLabel('当前环境',{exact:true}).selectOption(envA.id);
 if(await page.getByRole('tab').count()===0)await page.getByRole('button',{name:L.envDetails,exact:true}).click();
 await page.getByRole('tab',{name:L.tabCompare,exact:true}).click();
 const config=page.locator('.config-editor');
 await config.getByLabel('项目 cwd（可选）').fill(project);
 await config.getByRole('button',{name:L.configReinspect,exact:true}).click();
 await config.getByRole('button',{name:L.configRead,exact:true}).click();
 const editor=config.locator('.config-textarea');
 await editor.waitFor();
 const beforeDoc=JSON.parse(await editor.inputValue());
 const edited={...beforeDoc,synthetic_note:'after'};
 await editor.fill(JSON.stringify(edited,null,2));
 await config.getByRole('button',{name:L.configPreviewEdit,exact:true}).click();
 await dialog.locator('.plan-identity code.full-path').waitFor({state:'attached'});
 const editPlanId=lastPlanId('plan_config_edit');
 assert.ok(editPlanId,'advanced edit is bound to its real persisted plan');
 const editPass=dialog.locator('.archive-password input[type=password]');
 if(await editPass.count()>=1){await editPass.nth(0).fill(password);if(await editPass.count()>1)await editPass.nth(1).fill(password)}
 await approveCurrent();
 const editReceipt=await dialog.locator('.receipt-id code').innerText();
 const editedDoc=JSON.parse(await readFile(path.join(rootA,'settings.json'),'utf8'));
 assert.equal(editedDoc.synthetic_note,'after','the edited document was written and read back');
 assert.equal(editedDoc.synthetic_unrelated,'keep','the unrelated field survives the edit');
 await dialog.getByRole('button',{name:'查看记录',exact:true}).click();
 await page.locator('.sidebar nav').getByRole('button',{name:'环境',exact:true}).click();
 if(await page.getByRole('tab').count()===0)await page.getByRole('button',{name:L.envDetails,exact:true}).click();
 await page.getByRole('tab',{name:L.tabCompare,exact:true}).click();
 await config.getByRole('button',{name:L.configRead,exact:true}).click();
 // Restore only ONE field from the original edit task.
 await config.locator('.config-restore > summary').click();
 await config.getByLabel('原任务 ID').fill(editReceipt);
 await config.getByRole('button',{name:L.restoreInspect,exact:true}).click();
 const restoreFields=config.locator('.config-restore-fields li');
 await restoreFields.first().waitFor();
 const noteField=restoreFields.filter({hasText:'synthetic_note'}).first();
 await noteField.getByRole('checkbox').check();
 await config.getByRole('button',{name:L.restorePreview,exact:true}).click();
 await dialog.locator('.plan-identity code.full-path').waitFor({state:'attached'});
 const restorePass=dialog.locator('.archive-password input[type=password]');await restorePass.nth(0).fill(password);await restorePass.nth(1).fill(password);
 await approveCurrent();
 await dialog.locator('.receipt-id code').waitFor().catch(()=>{});
 const restoredDoc=JSON.parse(await readFile(path.join(rootA,'settings.json'),'utf8'));
 assert.equal(restoredDoc.synthetic_note,'before','restore returned the selected field to its original value');
 assert.equal(restoredDoc.synthetic_unrelated,'keep','restore preserves the unrelated field');
 assert.notEqual(editPlanId,'');
 await dialog.getByRole('button',{name:'查看记录',exact:true}).click().catch(()=>{});
 report.checks.push('config: project_cwd inspect + read + edit read-back; selected-field restore reverted only that field, unrelated edits preserved');

 // --- Snapshot save x2 -> compare; cross-environment apply to target ---
 await page.locator('.sidebar nav').getByRole('button',{name:'环境',exact:true}).click();
 if(await page.getByRole('tab').count()===0)await page.getByRole('button',{name:L.envDetails,exact:true}).click();
 await page.getByRole('tab',{name:L.tabCompare,exact:true}).click();
 const snapshot=page.locator('.snapshot-panel');
 const previousSaves=coreResponses.filter(item=>item.command==='snapshot_save').length;
 await snapshot.getByRole('button',{name:L.snapshotSave,exact:true}).click();
 await waitForResult('snapshot_save',previousSaves);
 await snapshot.locator('[role=status]').waitFor();
 await snapshot.getByRole('button',{name:L.snapshotSave,exact:true}).click();
 await waitForResult('snapshot_save',previousSaves+1);
 await snapshot.locator('[role=status]').waitFor();
 const compareButtons=snapshot.getByRole('button',{name:L.snapshotCompare,exact:true});
 assert.ok(await compareButtons.count()>=1,'snapshot compare is reachable after two saves');
 await compareButtons.first().click();
 // A real external config change is accepted one semantic item at a time.
 const savedFirst=coreResponses.filter(item=>item.command==='snapshot_save').at(-2).envelope.data.snapshot.snapshot_id;
 const external=JSON.parse(await readFile(path.join(rootA,'settings.json'),'utf8'));external.env.OTEL_LOG_USER_PROMPTS='1';
 await writeFile(path.join(rootA,'settings.json'),JSON.stringify(external));
 const previousThird=coreResponses.filter(item=>item.command==='snapshot_save').length;
 await snapshot.getByRole('button',{name:L.snapshotSave,exact:true}).click();
 const third=await waitForResult('snapshot_save',previousThird);await snapshot.locator('[role=status]').waitFor();
 await snapshot.getByLabel('较早快照',{exact:true}).selectOption(savedFirst);
 await snapshot.getByLabel('较新快照',{exact:true}).selectOption(third.snapshot.snapshot_id);
 await snapshot.getByRole('button',{name:L.snapshotCompare,exact:true}).click();
 await snapshot.getByRole('checkbox',{name:'接受这一项：OTEL_LOG_USER_PROMPTS',exact:true}).check();
 const previousAccept=coreResponses.filter(item=>item.command==='snapshot_accept').length;
 await snapshot.getByRole('button',{name:'接受所选 1 项当前值',exact:true}).click();
 const accepted=await waitForResult('snapshot_accept',previousAccept);assert.equal(accepted.status,'accepted');
 assert.equal(JSON.parse(await readFile(path.join(rootA,'settings.json'),'utf8')).env.OTEL_LOG_USER_PROMPTS,'1','accept updates only the comparison baseline');
 report.checks.push('one selected semantic change accepted into a baseline without mutating configuration');
 // Cross-environment apply: choose env B, produce a plan whose target is B.
 await page.getByLabel('当前环境',{exact:true}).selectOption(envB.id);
 await snapshot.getByRole('button',{name:L.snapshotSave,exact:true}).click().catch(()=>{});
 await page.getByLabel('当前环境',{exact:true}).selectOption(envA.id);
 await snapshot.getByLabel('另一环境').selectOption(envB.id);
 const previousPairs=coreResponses.filter(item=>item.command==='env_compare').length;
 await snapshot.getByRole('button',{name:L.snapshotPair,exact:true}).click();
 await waitForResult('env_compare',previousPairs);
 const applyButton=snapshot.getByRole('button',{name:L.applyNormal,exact:true});
 await applyButton.waitFor();
 await snapshot.getByRole('checkbox',{name:'复制普通设置 OTEL_LOG_USER_PROMPTS',exact:true}).check();
 await applyButton.click();
 await dialog.locator('.plan-target').getByText(envB.name,{exact:true}).waitFor();
 const applyPlan=calls.filter(c=>c.command==='plan_config_apply').at(-1);
 assert.equal(applyPlan.environment_id,envB.id,'the cross-environment plan targets the chosen environment');
 const applyPass=dialog.locator('.archive-password input[type=password]');
 if(await applyPass.count()){await applyPass.nth(0).fill(password);await applyPass.nth(1).fill(password);}
 await approveCurrent();
 await dialog.locator('.receipt-id code').waitFor();
 assert.equal(JSON.parse(await readFile(path.join(rootB,'settings.json'),'utf8')).env.OTEL_LOG_USER_PROMPTS,'1');
 await dialog.getByRole('button',{name:'查看记录',exact:true}).click();
 report.checks.push('snapshots saved twice and compared; cross-environment apply targets the chosen environment');

 // --- Recipe: save as personal default, then Run produces a FRESH plan id ---
 await page.locator('.sidebar nav').getByRole('button',{name:'保护方案',exact:true}).click();
 await revealPolicyRecipes();
 const recipes=page.locator('.recipes');
 const reduceRow=recipes.locator('.recipe-row').filter({hasText:L.recipeReduce}).first();
 await reduceRow.getByRole('button',{name:L.recipeSave,exact:true}).click();
 const policyDock=page.locator('.workspace-actions .action-bar');
 async function previewPolicyId(check){
  const previous=coreResponses.filter(item=>item.command==='plan_policy').length;
  await policyDock.getByRole('button',{name:L.previewPolicy,exact:true}).click();
  const plan=await waitForResult('plan_policy',previous);
  await dialog.locator('.plan-identity code.full-path').waitFor({state:'attached'});assert.ok(plan.id);
  if(check)await check(plan);
  await dialog.getByRole('button',{name:'返回调整',exact:true}).click();await dialog.waitFor({state:'hidden'});
  return plan.id;
 }
 await reduceRow.getByRole('button',{name:L.recipeApply,exact:true}).click();
 const firstPlan=await previewPolicyId();
 await reduceRow.getByRole('button',{name:L.recipeApply,exact:true}).click();
 const secondPlan=await previewPolicyId();
 assert.notEqual(firstPlan,secondPlan,'a saved recipe never replays an old approval; each run is a new plan ID');
 report.checks.push(`recipe re-run produced fresh plan IDs (${firstPlan} -> ${secondPlan})`);

 // --- Responsive + Day/Night with no horizontal overflow ---
 await page.locator('.sidebar nav').getByRole('button',{name:'环境',exact:true}).click();
 if(await page.getByRole('tab').count()===0)await page.getByRole('button',{name:L.envDetails,exact:true}).click();
 for(const [width,height,theme] of [[900,640,'dark'],[900,640,'light'],[1120,760,'light'],[1120,760,'dark']]){
  await page.setViewportSize({width,height});
  await page.evaluate(theme=>document.documentElement.dataset.theme=theme,theme);
  await page.getByRole('tab',{name:L.tabCompare,exact:true}).click();
  await page.locator('.snapshot-panel').scrollIntoViewIfNeeded();
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),`no horizontal overflow at ${width}x${height}`);
  await page.screenshot({path:path.join(fixture,`client-control-${width}-${theme}.png`)});
  await page.locator('.config-editor').scrollIntoViewIfNeeded();
  await page.screenshot({path:path.join(fixture,`client-control-config-${width}-${theme}.png`)});
  await page.getByRole('tab',{name:L.tabState,exact:true}).click();
  await page.locator('.client-data-panel').evaluate(element=>element.scrollIntoView({block:'start'}));
  await page.screenshot({path:path.join(fixture,`client-control-data-${width}-${theme}.png`)});
 }
 report.checks.push('900x640 and 1120x760 in Day/Night: DOM has no horizontal overflow, screenshots captured');
 // Expand every newly introduced form family. A collapsed form cannot prove
 // that its password/number/select fields share the actual App treatment.
 await page.setViewportSize({width:900,height:640});
 async function inspectFields(scope){
  const metrics=await scope.locator('select,input[type=password],input[type=number]').evaluateAll(elements=>elements.filter(element=>element.getClientRects().length).map(element=>{const style=getComputedStyle(element);return {kind:element.tagName+':'+element.type,radius:parseFloat(style.borderRadius),height:element.getBoundingClientRect().height,background:style.backgroundColor};}));
  assert.ok(metrics.length,'the expanded form exposes its controls');
  for(const field of metrics){assert.ok(field.radius>=5&&field.height>=36,JSON.stringify(field));}
  return metrics;
 }
 for(const theme of ['light','dark']){
  await page.evaluate(theme=>document.documentElement.dataset.theme=theme,theme);
  const filters=page.locator('.client-data-panel>details.surface').filter({hasText:'扫描目录与完整清单筛选'});
  if(!await filters.evaluate(element=>element.open))await filters.locator('summary').click();
  await inspectFields(filters);
  await filters.evaluate(element=>element.scrollIntoView({block:'start'}));
  await page.screenshot({path:path.join(fixture,`controls-filters-${theme}.png`)});
  const reader=page.locator('.residue-archive-panel');
  if(!await reader.evaluate(element=>element.open))await reader.locator('summary').click();
  await inspectFields(reader);
  await reader.evaluate(element=>element.scrollIntoView({block:'start'}));
  await page.screenshot({path:path.join(fixture,`controls-archive-${theme}.png`)});
  await page.getByRole('tab',{name:L.tabCompare,exact:true}).click();
  const privateEditor=page.locator('.config-editor');
  await privateEditor.getByRole('button',{name:L.configRead,exact:true}).click();
  await privateEditor.locator('.config-textarea').waitFor();
  await privateEditor.locator('.config-textarea').evaluate(element=>element.scrollIntoView({block:'center'}));
  await page.screenshot({path:path.join(fixture,`controls-json-${theme}.png`)});
  await page.locator('.sidebar nav').getByRole('button',{name:'保护方案',exact:true}).click();
  await revealPolicyRecipes();
  const recipe=page.locator('.recipe-row').filter({hasText:L.recipeReduce}).first();
  await recipe.getByRole('button',{name:'编辑',exact:true}).click();
  await inspectFields(recipe);
  await recipe.evaluate(element=>element.scrollIntoView({block:'start'}));
  const frame=await page.evaluate(()=>({scrollY,rootHeight:innerHeight,shell:[...document.querySelectorAll('#root,.app-shell,.workspace,.main-content,.sidebar')].map(element=>({class:element.id||element.className,height:element.clientHeight,scrollHeight:element.scrollHeight,scrollTop:element.scrollTop,top:element.getBoundingClientRect().top})),headerTop:document.querySelector('.target-bar').getBoundingClientRect().top}));
  report.frameMetrics??=[];report.frameMetrics.push({theme,...frame});
  assert.equal(frame.headerTop,0,'scrolling an editor must keep the workspace header fixed');
  await page.screenshot({path:path.join(fixture,`controls-recipe-${theme}.png`)});
  await page.locator('.preset').filter({hasText:'自定义'}).getByRole('radio').check();
  const rule=page.locator('.custom-policy-row[data-key="DISABLE_TELEMETRY"]');
  await rule.locator('.custom-policy-detail > summary').click();
  await inspectFields(rule);
  await rule.evaluate(element=>element.scrollIntoView({block:'start'}));
  await page.screenshot({path:path.join(fixture,`controls-policy-${theme}.png`)});
  const contentRule=page.locator('.custom-policy-row[data-key="OTEL_LOG_USER_PROMPTS"]');
  await contentRule.locator('.custom-policy-detail > summary').click();
  assert.ok((await contentRule.locator('.policy-rule-information > .custom-policy-facts').innerText()).includes('https://collector.invalid/v1'),'content controls show the configured receiver rather than duplicating the impact text');
  await page.locator('.sidebar nav').getByRole('button',{name:'环境',exact:true}).click();
  if(await page.getByRole('tab').count()===0)await page.getByRole('button',{name:L.envDetails,exact:true}).click();
  await page.getByRole('tab',{name:L.tabState,exact:true}).click();
 }
 report.checks.push('expanded configuration, recipe, custom policy, residue filters and archive forms: themed controls inspected in Day/Night at 900px, including numeric/password fields');
 for(const [width,height,theme] of [[900,640,'light'],[900,640,'dark'],[1120,760,'light'],[1120,760,'dark']]){
  await page.setViewportSize({width,height});
  await page.evaluate(theme=>document.documentElement.dataset.theme=theme,theme);
  await page.locator('.sidebar nav').getByRole('button',{name:'记录与恢复',exact:true}).click();
  await page.locator('.launch-records').scrollIntoViewIfNeeded();
  await page.screenshot({path:path.join(fixture,`polish-records-${width}-${theme}.png`)});
  const previousScope=coreResponses.filter(item=>item.command==='cleanup_inspect').length;
  await page.locator('.sidebar nav').getByRole('button',{name:'清理与重建',exact:true}).click();
  await waitForResult('cleanup_inspect',previousScope);
  const scope=page.locator('.cleanup-scope');
  await scope.locator('.cleanup-file-path').first().waitFor();
  await scope.scrollIntoViewIfNeeded();
  assert.equal(await scope.locator('.cleanup-file-path').first().evaluate(element=>element.open),false,'full paths are initially quiet');
  await page.screenshot({path:path.join(fixture,`polish-scope-${width}-${theme}.png`)});
  await scope.locator('.cleanup-file-path > summary').first().click();
  assert.ok((await scope.locator('.cleanup-file-full-path').first().innerText()).includes(home),'the exact path remains available on demand');
  await page.locator('.sidebar nav').getByRole('button',{name:'保护方案',exact:true}).click();
  await revealPolicyRecipes();
 const recipes=page.locator('.recipes');
  await recipes.locator('.surface-heading').scrollIntoViewIfNeeded();
  await page.screenshot({path:path.join(fixture,`polish-recipes-${width}-${theme}.png`)});
  for(let row=0;row<3;row++){
   const recipe=recipes.locator('.recipe-row').nth(row);
   await recipe.scrollIntoViewIfNeeded();
   await recipe.screenshot({path:path.join(fixture,`polish-recipe-${row+1}-${width}-${theme}.png`)});
  }
  await page.locator('.sidebar nav').getByRole('button',{name:'环境',exact:true}).click();
  assert.ok(await page.locator('.target-bar').evaluate(element=>element.getBoundingClientRect().height<=60),'header stays one line');
  const target=page.getByRole('button',{name:'查看目标详情',exact:true});
  await target.click();
  const targetPanel=page.locator('.target-details-panel');
  assert.ok((await targetPanel.innerText()).includes(rootA));
  await targetPanel.getByRole('button',{name:'复制完整路径',exact:true}).first().click();
  assert.equal(copied.at(-1),rootA,'copy uses the complete exact root, not its shortened label');
  const rect=await targetPanel.boundingBox();assert.ok(rect.x>=0&&rect.x+rect.width<=width,'target details fit the workspace');
  const inlineDetails=await targetPanel.evaluate(panel=>{
   const header=document.querySelector('.target-bar').getBoundingClientRect(),trigger=document.querySelector('.target-details-toggle').getBoundingClientRect();
   const host=document.querySelector('.host-switch').getBoundingClientRect();
   const box=panel.getBoundingClientRect(),main=document.querySelector('main').getBoundingClientRect();
   return trigger.right<=host.left&&trigger.top>=header.top&&trigger.bottom<=header.bottom&&box.top>=header.bottom&&main.top>=box.bottom&&getComputedStyle(panel).position==='static';
  });
  assert.ok(inlineDetails,'terminal prompt precedes the host; details expand below the toolbar in normal flow without covering the workspace');
  await page.waitForFunction(()=>getComputedStyle(document.querySelector('.target-cursor')).opacity==='1');
  await page.screenshot({path:path.join(fixture,`polish-target-${width}-${theme}.png`)});
  await page.keyboard.press('Escape');
  assert.equal(await page.locator('.target-details-panel').count(),0);
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'no horizontal overflow after disclosure');
 }
 report.checks.push('polish: compact target disclosure and exact root, cleanup path expansion, launch records and all recipe rows captured at 900/1120 in Day/Night');

 {
 // --- A person can edit one setting directly or explicitly select a batch. ---
 await page.locator('.toast').waitFor({state:'hidden'});
 await page.locator('.sidebar nav').getByRole('button',{name:'保护方案',exact:true}).click();
 await page.locator('.preset').filter({hasText:'自定义'}).getByRole('radio').check();
 // Re-observe a representative external configuration edit, confined to this fixture.
 const reseeded=JSON.parse(await readFile(path.join(rootA,'settings.json'),'utf8'));
 reseeded.env.OTEL_LOG_RAW_API_BODIES=`file:${rawDirectory}`;
 await writeFile(path.join(rootA,'settings.json'),JSON.stringify(reseeded,null,2));
 const beforeInspect=coreResponses.filter(item=>item.command==='inspect').length;
 await page.getByRole('button',{name:'重新检查环境',exact:true}).click();
 await waitForResult('inspect',beforeInspect);
 const editor=page.locator('.custom-policy');
 const row=key=>editor.locator(`.custom-policy-row[data-key="${key}"]`);
 const choice=key=>row(key).locator('.policy-rule-choice select');
 const previewButton=policyDock.getByRole('button',{name:L.previewPolicy,exact:true});
 if(await editor.getByRole('button',{name:'撤销全部改动',exact:true}).count())await editor.getByRole('button',{name:'撤销全部改动',exact:true}).click();
 assert.equal(await editor.locator('.policy-rule-selection').count(),0,'selection checkboxes are absent until the explicit bulk mode');
 assert.equal(await editor.locator('.custom-policy-row').count(),27,'all current finite policy fields remain reachable');
 assert.equal(await previewButton.isDisabled(),true,'an empty custom draft cannot produce an unexplained empty preview');
 const originalSettings=await readFile(path.join(rootA,'settings.json'),'utf8');
 await editor.getByRole('button',{name:'批量编辑',exact:true}).click();
 const selectedKeys=['DISABLE_TELEMETRY','DISABLE_ERROR_REPORTING','OTEL_LOG_USER_PROMPTS'];
 for(const key of selectedKeys)await row(key).locator('.policy-rule-selection').check();
 const bulk=editor.locator('.policy-batch');
 await bulk.getByRole('combobox').selectOption('disable');
 await bulk.getByRole('button',{name:'更新 3 项草案',exact:true}).click();
 for(const key of selectedKeys)assert.equal(await choice(key).inputValue(),'disable',`one bulk action retains ${key}, including early updates in the React batch`);
 assert.equal(await editor.locator('.custom-policy-row.is-changed').count(),3);
 for(const [width,height,theme] of [[1120,760,'light'],[900,640,'dark']]){
  await page.setViewportSize({width,height});await page.evaluate(theme=>document.documentElement.dataset.theme=theme,theme);
  await bulk.evaluate(element=>element.scrollIntoView({block:'start'}));
  await page.screenshot({path:path.join(fixture,`policy-batch-${width}-${theme}.png`)});
 }
 await page.setViewportSize({width:1120,height:760});await page.evaluate(()=>document.documentElement.dataset.theme='light');

 await previewPolicyId();
 const batchRequest=calls.filter(call=>call.command==='plan_policy').at(-1);
 for(const key of selectedKeys)assert.equal(batchRequest.custom_settings[key],'disable','the same frozen plan receives every selected action');
 assert.equal(batchRequest.environment_id,envA.id);
 assert.equal(await readFile(path.join(rootA,'settings.json'),'utf8'),originalSettings,'a draft and plan preview cannot mutate the environment');
 await row('OTEL_EXPORTER_OTLP_ENDPOINT').locator('.policy-rule-selection').check();
 assert.equal(await bulk.getByRole('combobox').locator('option[value=enable]').count(),0,'a mixed selection offers only actions every selected rule supports');
 await editor.getByRole('searchbox').fill('ERROR_REPORTING');
 assert.ok((await bulk.innerText()).includes('其中 3 项在其他结果中'),'hidden selections stay explicit when filtering');
 await bulk.getByRole('button',{name:'清空选择',exact:true}).click();
 assert.equal(await bulk.getByRole('combobox').isDisabled(),true);
 await editor.getByRole('searchbox').fill('');
 await editor.getByRole('button',{name:'结束批量选择',exact:true}).click();
 assert.equal(await editor.locator('.policy-rule-selection').count(),0);
 await row('DISABLE_ERROR_REPORTING').getByRole('button',{name:'撤销此项',exact:true}).click();
 assert.equal(await choice('DISABLE_ERROR_REPORTING').inputValue(),'keep');
 assert.equal(await choice('DISABLE_TELEMETRY').inputValue(),'disable','undoing one item retains its neighboring draft');
 await editor.getByRole('button',{name:'撤销全部改动',exact:true}).click();
 assert.equal(await editor.locator('.custom-policy-row.is-changed').count(),0,'discarding all pending actions updates every row atomically');

 for(const [key,invalidValue,validValue] of [['CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH','many','4096'],['OTEL_LOGS_EXPORTER','none,otlp','otlp,console']]){
  await choice(key).selectOption('set');
  const input=row(key).locator('.policy-value-editor input');await input.fill(invalidValue);
  assert.equal(await input.getAttribute('aria-invalid'),'true');assert.equal(await previewButton.isDisabled(),true);
  await input.fill(validValue);assert.equal(await input.getAttribute('aria-invalid'),'false');
  await row(key).getByRole('button',{name:'撤销此项',exact:true}).click();
 }
 await choice('DISABLE_ERROR_REPORTING').selectOption('enable');
 await previewPolicyId(async()=>{
  const evidence=dialog.locator('.policy-evidence details').filter({has:page.getByText('本次自定义选择',{exact:true})});
  await evidence.locator(':scope > summary').click();
  const result=evidence.locator('.fact-row').filter({hasText:'错误回报'});
  assert.equal(await result.locator('span').last().innerText(),'开启','the approval summary cannot mislabel enable as keep');
 });
 await row('DISABLE_ERROR_REPORTING').getByRole('button',{name:'撤销此项',exact:true}).click();

 // An explicit custom value must be real request data, not a visual fallback.
 const raw=row('OTEL_LOG_RAW_API_BODIES');
 await choice('OTEL_LOG_RAW_API_BODIES').selectOption('file');
 const rawInput=raw.locator('.policy-value-editor input');
 assert.equal(await rawInput.inputValue(),rawDirectory,'directory mode asks for a path, not a file: command');
 await previewPolicyId();
 assert.equal(calls.filter(call=>call.command==='plan_policy').at(-1).custom_values.OTEL_LOG_RAW_API_BODIES,`file:${rawDirectory}`,'selecting the shown original value also explicitly submits it');
 await rawInput.fill('');
 assert.equal(await previewButton.isDisabled(),true,'missing custom values explain and block preview before a core error');
 await rawInput.fill(` ${rawDirectory}`);
 assert.equal(await previewButton.isDisabled(),true,'directory whitespace is explained before the core rejects it');
 await rawInput.fill(`${rawDirectory}/next`);
 await previewPolicyId();
 assert.equal(calls.filter(call=>call.command==='plan_policy').at(-1).custom_values.OTEL_LOG_RAW_API_BODIES,`file:${rawDirectory}/next`);
 assert.ok(!(await page.evaluate(()=>localStorage.getItem('lintel.drafts'))).includes(rawDirectory),'raw paths remain outside persisted drafts');
 await choice('OTEL_LOG_RAW_API_BODIES').selectOption('enable');
 assert.equal(await raw.locator('.policy-value-editor input').count(),0,'inline mode needs no implementation string or directory');
 await previewPolicyId(async()=>{
  const evidence=dialog.locator('.policy-evidence details').filter({has:page.getByText('本次自定义选择',{exact:true})});
  await evidence.locator(':scope > summary').click();
  const result=evidence.locator('.fact-row').filter({hasText:'原始 API 正文记录'});
  assert.equal(await result.locator('span').last().innerText(),'内联记录');
 });
 assert.equal(calls.filter(call=>call.command==='plan_policy').at(-1).custom_settings.OTEL_LOG_RAW_API_BODIES,'enable');
 await choice('OTEL_LOG_RAW_API_BODIES').selectOption('file');
 await rawInput.fill(`${rawDirectory}/next`);
 await revealPolicyRecipes();
 await page.getByRole('button',{name:'把当前方案存为个人默认',exact:true}).click();
 assert.ok((await page.locator('.recipe-save-feedback').innerText()).includes('原始 API 正文记录'),'saving explains the exact path-setting intent that cannot become a default');
 assert.equal(await rawInput.inputValue(),`${rawDirectory}/next`,'saving a portable default never replaces the live draft');
 await editor.getByRole('button',{name:'只看待改 · 1',exact:true}).click();
 assert.equal(await editor.locator('.custom-policy-row').count(),1,'changes-only review retains the exact edited setting');
 await editor.getByRole('button',{name:'只看待改 · 1',exact:true}).click();
 await raw.locator('.custom-policy-detail > summary').click();
 assert.ok((await raw.locator('.custom-policy-facts').first().innerText()).includes(rawDirectory),'full original API destination remains readable on demand');
 for(const [width,height,theme] of [[900,640,'light'],[900,640,'dark'],[1120,760,'light'],[1120,760,'dark']]){
  await page.setViewportSize({width,height});await page.evaluate(theme=>document.documentElement.dataset.theme=theme,theme);
  await page.locator('.main-content').evaluate(element=>element.scrollTo({top:0}));
  await page.screenshot({path:path.join(fixture,`policy-overview-${width}-${theme}.png`)});
  await editor.locator('.policy-editor-heading').scrollIntoViewIfNeeded();
  await page.screenshot({path:path.join(fixture,`policy-editor-${width}-${theme}.png`)});
  await raw.locator('.policy-rule-main').evaluate(element=>element.scrollIntoView({block:'start'}));
  const bounds=await raw.locator('h4').boundingBox();
  assert.ok(bounds.width>=180&&bounds.height<65,`API title keeps a readable width and normal lines at ${width}/${theme}`);
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'long API values do not enlarge the workspace');
  await page.screenshot({path:path.join(fixture,`policy-api-${width}-${theme}.png`)});
 }
 await page.locator('.target-bar .language-control select').selectOption('en');
 assert.equal(await rawInput.inputValue(),`${rawDirectory}/next`,'language switching preserves the exact custom value');
 await page.screenshot({path:path.join(fixture,'policy-api-english.png')});
 await page.locator('.target-bar .language-control select').selectOption('zh-CN');
 await page.locator('.policy-current-config > summary').click();
 const rawReadback=page.locator('.policy-current-config .setting-row[data-key="OTEL_LOG_RAW_API_BODIES"]');
 await rawReadback.locator(':scope > summary').click();
 assert.ok((await rawReadback.locator('.setting-detail').innerText()).includes(rawDirectory),'the separate readback keeps the full value behind its bounded summary');
 const readbackTitle=await rawReadback.locator(':scope > summary > span').first().boundingBox();
 assert.ok(readbackTitle.width>=140&&readbackTitle.height<75,'readback title cannot collapse into a vertical strip');
 assert.equal(await readFile(path.join(rootA,'settings.json'),'utf8'),originalSettings);
 // A credential-bearing existing endpoint stays masked, but may be replaced
 // with a plain endpoint through the same approved core path.
 const maskedDocument=JSON.parse(originalSettings);
 const credentialEndpoint='https://synthetic-private:credential@collector.invalid/v1';
 maskedDocument.env.OTEL_EXPORTER_OTLP_ENDPOINT=credentialEndpoint;
 await writeFile(path.join(rootA,'settings.json'),JSON.stringify(maskedDocument,null,2));
 const maskedOriginal=await readFile(path.join(rootA,'settings.json'),'utf8');
 const inspected=coreResponses.filter(item=>item.command==='inspect').length;
 await page.getByRole('button',{name:'重新检查环境',exact:true}).click();await waitForResult('inspect',inspected);
 await editor.getByRole('button',{name:'撤销全部改动',exact:true}).click();
 const endpoint=row('OTEL_EXPORTER_OTLP_ENDPOINT');
 assert.equal(await endpoint.locator('option[value=set]').count(),1,'masking the old endpoint must not remove the supported plain replacement action');
 assert.equal(await endpoint.locator('option[value=disable]').count(),0,'a free value has one explicit removal operation, not two misleading outcomes');
 await choice('OTEL_EXPORTER_OTLP_ENDPOINT').selectOption('set');
 const endpointInput=endpoint.locator('.policy-value-editor input');
 assert.equal(await endpointInput.inputValue(),'','a masked old value is never used as an editable fallback');
 assert.equal(await endpointInput.isDisabled(),false);
 assert.ok(!(await page.locator('body').textContent()).includes(credentialEndpoint));
 await endpointInput.fill('https://synthetic-private:do-not-persist@collector.invalid/?token=synthetic');
 assert.equal(await previewButton.isDisabled(),true,'credential-bearing replacement stays in the private editor, not an ordinary plan');
 await endpointInput.fill('https://replacement.invalid:4318');
 await previewPolicyId();
 assert.equal(calls.filter(call=>call.command==='plan_policy').at(-1).custom_values.OTEL_EXPORTER_OTLP_ENDPOINT,'https://replacement.invalid:4318');
 assert.equal(await readFile(path.join(rootA,'settings.json'),'utf8'),maskedOriginal,'plain replacement preview does not mutate the masked original');
 assert.ok(!(await page.evaluate(()=>localStorage.getItem('lintel.drafts'))).includes('replacement.invalid'));
 report.checks.push('policy redesign: direct editing, explicit bulk mode with common supported actions, all selected fields in one real-core plan, atomic reset, single-item undo, visible hidden-selection scope, missing-value guard, custom-value request binding, non-persistence, full long API paths and readable titles in both themes/sizes and English; draft and preview leave original settings unchanged');
 }
 await page.setViewportSize({width:1120,height:760});
 assert.deepEqual(errors,[]);
 report.passed=true;
}finally{if(!report.passed){report.coreResults=coreResponses.slice(-12).map(item=>({command:item.command,ok:item.envelope.ok,errorCode:item.envelope.error?.code}));report.lastCommands=calls.slice(-12).map(item=>item.command);if(page)report.currentPage=await page.locator('.sidebar nav [aria-current=page]').innerText().catch(()=>'unavailable');}if(page&&!report.passed)await page.screenshot({path:path.join(fixture,'failure.png')}).catch(()=>{});await browser?.close();if(preview.exitCode===null){const ended=new Promise(resolve=>preview.once('exit',resolve));preview.kill('SIGTERM');await ended}await writeFile(process.env.LINTEL_CLIENT_CONTROL_UI_REPORT||path.join(fixture,'report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2))}
