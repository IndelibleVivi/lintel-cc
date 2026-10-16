// Bilingual built-App UI journey. The App-invoked native layer is a synthetic
// mock (no native WebKit, no core process): `app_preferred_languages` and
// `app_set_language` are finite App-commanded presentation commands; `request`
// is a canned core envelope whose `plan_policy`/`execute` arguments are recorded
// exactly. The point is exactly how the rendered App localizes, switches and
// preserves state — not native behavior.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdtemp,writeFile} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {stripVTControlCharacters} from 'node:util';

const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'),desktop=path.join(repo,'apps/desktop');
const {en}=await import(pathToFileURL(path.join(desktop,'src/i18n.catalog.en.ts')).href);
const {zhCN}=await import(pathToFileURL(path.join(desktop,'src/i18n.catalog.zh-CN.ts')).href);
const {chromium}=await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE||path.join(repo,'extensions/browser/node_modules/playwright/index.mjs')).href);
const output=await mkdtemp(path.join(os.tmpdir(),'lintel-i18n-ui-'));

const environment={id:'11111111-1111-4111-8111-111111111111',name:'Synthetic workspace',host:'local',surface:'claude-code',root:'/synthetic/config',executable:'/synthetic/bin/claude',ownership:'registered',status:'discovered'};
const plan={id:'plan-raw-1234',hash:'a'.repeat(64),environment_id:environment.id,title:'减少外发',changes:[{key:'DISABLE_TELEMETRY',label:'产品指标',before:null,after:'1',path:'/synthetic/config/settings.json'}],preserves:['所有原始工作内容与项目文件'],warnings:[],actions:[{id:'settings',label:'精确字段写入与读回',reversible:true}],created_at:'2026-10-06T00:00:00Z',status:'planned',policy:{rule_version:'v',product:{version:'2.1.283',source:'native_version_path',executable:'/synthetic/bin/claude'},rules:[{key:'DISABLE_TELEMETRY',label:'产品指标',value:null,disabled:false,semantics:'nonempty',scope:'registered_root_user_settings',effect_timing:'next_launch'}],remote_control:{status:'configuration_compatible',summary:'此版本条件下 user settings 未发现 Remote Control 开关冲突；账号、组织权限及实际运行仍未验证。',trusted_devices:'unknown',trusted_devices_source:'user_declared',version_family:'2.1.x',runtime_verified:false,blockers:[],unverified:[]}}};
const receipt={id:'job-raw-5678',plan_id:plan.id,environment_id:environment.id,title:'减少外发',status:'accepted',steps:[{id:'settings',label:'精确字段写入与读回',status:'accepted',message:'写入意图已保存；中断后只查询原任务'}],created_at:'2026-10-06T00:00:00Z',restorable:false,warnings:[],task_result:{outcome:'query_only',selected_steps:[],coverage:[{scope:'configuration',state:'unverified',detail:'服务端 token 撤销与 Keychain 状态未独立核验'}],next_actions:[{label:'查询原任务',entry:'records'}]}};
const capabilities=[{name:'Claude Code 配置',status:'available',reason:'精确字段预览、读回与恢复；运行时效果另行验证'},{name:'网络强约束',status:'not_delivered',reason:'未安装或验证平台高权限组件'}];

const report={fixture:'synthetic native language commands + canned core envelopes',runtime:'built App in isolated Chromium; not native WebKit',checks:[],passed:false};
const preview=spawn(process.execPath,[path.join(desktop,'node_modules/vite/bin/vite.js'),'preview','--host','127.0.0.1','--port','0','--strictPort'],{cwd:desktop,stdio:['ignore','pipe','pipe']});
let browser=null,page=null,devPreview=null;
try{
  const url=await new Promise((resolve,reject)=>{let text='';const timer=setTimeout(()=>reject(new Error('preview timeout')),15000);preview.once('error',reject);preview.once('exit',code=>reject(new Error(`preview exited ${code}`)));for(const stream of [preview.stdout,preview.stderr])stream.on('data',chunk=>{text+=chunk;const match=stripVTControlCharacters(text).match(/http:\/\/127\.0\.0\.1:\d+\//);if(match){clearTimeout(timer);resolve(match[0]);}});});
  browser=await chromium.launch({headless:true});
  page=await browser.newPage({viewport:{width:1120,height:800},colorScheme:'light'});
  page.setDefaultTimeout(12000);
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  const menuCalls=[];const coreCalls=[];const execCalls=[];let nodePreferred=['en-US'];let planCount=0;let failPreferred=false;let delayPreferred=false;const pendingPreferred=[];let pendingReady=null;
  const waitPending=async count=>{if(pendingPreferred.length<count)await new Promise(resolve=>{pendingReady={count,resolve};});};
  await page.exposeFunction('nativeInvoke',async (command,args)=>{
    if(command==='app_preferred_languages'){ if(failPreferred) throw new Error('command unavailable');const value=[...nodePreferred];if(delayPreferred)return new Promise(resolve=>{pendingPreferred.push({resolve,value});if(pendingReady&&pendingPreferred.length>=pendingReady.count){pendingReady.resolve();pendingReady=null;}});return value; }
    if(command==='app_set_language'){menuCalls.push(args.language);return null;}
    if(command==='request'){const c=args.payload;coreCalls.push(c.command);
      if(c.command==='discover')return {ok:true,data:{environments:[environment],capabilities}};
      if(c.command==='jobs')return {ok:true,data:{jobs:[receipt]}};
      if(c.command==='inspect')return {ok:true,data:{environment,settings:[],assets:[],warnings:[]}};
      if(c.command==='plan_policy'){planCount++;return {ok:true,data:plan};}
      if(c.command==='job')return {ok:true,data:receipt};
      if(c.command==='execute'){execCalls.push({plan_id:c.plan_id,approval:c.approval});return {ok:true,data:receipt};}
      throw new Error('unexpected core command '+c.command);}
    if(command==='remote_request')return {ok:true,data:{hosts:[],tasks:[],installations:[]}};
    throw new Error('unexpected native command '+command);
  });
  await page.addInitScript(()=>{window.__firstHome=[];new MutationObserver(()=>{const home=document.querySelector('.home-title');if(home&&window.__firstHome.length===0)window.__firstHome.push({text:home.textContent,lang:document.documentElement.lang});}).observe(document,{childList:true,subtree:true});window.isTauri=true;window.__TAURI_INTERNALS__={invoke:(c,a)=>window.nativeInvoke(c,a)};Object.defineProperty(navigator,'clipboard',{value:{writeText:async()=>{}}});});
  await page.goto(url);
  await page.locator('.home-title').waitFor();
  const switchSel='.language-switch select';
  const currentLanguage=page.locator('.target-bar .language-current');
  // (1) First paint follows the native preferred language (en), no zh flash.
  assert.equal(await page.locator(switchSel).inputValue(),'system');
  assert.equal(await currentLanguage.innerText(),'EN','System face shows the effective App language, not the preference mode');
  assert.deepEqual(await page.locator(switchSel).locator('option').allTextContents(),['Follow system','简体中文','English'],'all three preference choices retain full names');
  const descriptionId=await page.locator(switchSel).getAttribute('aria-describedby');
  assert.equal(await page.locator(`[id="${descriptionId}"]`).innerText(),'Current interface: English','effective language is available to assistive technology');
  assert.equal(await page.getByRole('button',{name:'Start a task',exact:true}).count(),1,'first paint is English from native en-US');
  assert.equal(await page.locator('html').getAttribute('lang'),'en');
  assert.equal(menuCalls.at(-1),'en','native menu language pushed on first resolve');
  assert.equal(await page.evaluate(()=>window.__firstHome[0].lang),'en','document language precedes the first App content');
  report.checks.push('first paint follows native en-US preference; html lang=en; menu push recorded');
  // The quiet toolbar keeps identity visible and reveals exact paths only on demand.
  const targetToggle=page.getByRole('button',{name:'Show target details',exact:true});
  assert.equal(await page.locator('.target-cursor').evaluate(node=>getComputedStyle(node).animationName),'none','the terminal cursor is still by default');
  assert.equal(await page.locator('.brand-cursor').evaluate(node=>getComputedStyle(node).animationName),'brand-blink','the existing logo keeps its own default');
  assert.equal(await page.locator('.target-details-panel').count(),0);
  const beforeTargetCalls=coreCalls.length;
  await targetToggle.click();
  const targetPanel=page.getByRole('region',{name:'Show target details',exact:true});
  assert.ok((await targetPanel.innerText()).includes(environment.root));
  await page.keyboard.press('Escape');
  assert.equal(await page.locator('.target-details-panel').count(),0);
  assert.equal(await targetToggle.evaluate(node=>node===document.activeElement),true,'Escape returns focus to the terminal cursor');
  await targetToggle.click();
  await targetPanel.getByRole('button',{name:'Copy full path',exact:true}).last().focus();
  await page.keyboard.press('Tab');
  assert.equal(await page.locator('.target-details-panel').count(),1,'inline details stay available while tabbing into the workspace');
  await targetToggle.focus();
  await page.locator('.sidebar .brand').click();
  assert.equal(await page.locator('.target-details-panel').count(),1,'inline details do not reflow the workspace on an outside pointer');
  await page.getByRole('button',{name:'Start a task',exact:true}).click();
  await page.getByRole('dialog').waitFor();
  await page.keyboard.press('Escape');
  await page.getByRole('dialog').waitFor({state:'hidden'});
  assert.equal(await page.locator('.target-details-panel').count(),1,'Escape belongs to an open modal before the inline disclosure');
  await targetToggle.focus();
  await page.keyboard.press('Enter');
  assert.equal(await page.locator('.target-details-panel').count(),0,'the prompt trigger collapses its details');
  assert.equal(coreCalls.length,beforeTargetCalls,'target disclosure is local UI only');
  await page.emulateMedia({reducedMotion:'reduce'});
  assert.equal(await page.locator('.target-cursor').evaluate(node=>getComputedStyle(node).animationName),'none');
  await page.emulateMedia({reducedMotion:'no-preference'});
  report.checks.push('one-line toolbar with orange prompt before the host: inline exact paths below, explicit/Escape collapse and focus return, no core request, default terminal cursor still');
  // (2) Manual override to zh-CN: instant rerender, menu push, no core side effects; greeting identity stable.
  const titleBefore=await page.locator('.home-title').innerText();
  const coreBefore=coreCalls.length;
  await page.locator(switchSel).selectOption('zh-CN');
  await page.getByRole('button',{name:'开始一项任务',exact:true}).waitFor();
  assert.equal(await page.locator('html').getAttribute('lang'),'zh-CN');
  assert.equal(await currentLanguage.innerText(),'中文');
  assert.equal(menuCalls.at(-1),'zh-CN','menu follows manual override');
  assert.equal(coreCalls.length,coreBefore,'language switch issues no core request');
  const titleAfter=await page.locator('.home-title').innerText();
  const greetingKeys=Object.keys(en).filter(key=>key.startsWith('greeting.')&&en[key]===titleBefore);
  assert.ok(greetingKeys.length>0,'first greeting comes from the English catalog');
  assert.ok(greetingKeys.some(key=>zhCN[key]===titleAfter),'same greeting resolves through the Chinese catalog, including shared English wording');
  // Same identity: switching back to en restores the same first-paint greeting text.
  await page.locator(switchSel).selectOption('en');await page.getByRole('button',{name:'Start a task',exact:true}).waitFor();
  assert.equal(await page.locator('.home-title').innerText(),titleBefore,'same greeting identity across a switch');
  report.checks.push('manual override rerenders with no core call; menu follows; same greeting identity across switch');
  // (3) Reload persists the override (en).
  await page.reload();await page.locator('.home-title').waitFor();
  assert.equal(await page.locator(switchSel).inputValue(),'en');
  assert.equal(await page.getByRole('button',{name:'Start a task',exact:true}).count(),1);
  report.checks.push('manual override persists across reload');
  // Delayed and interleaved reads: retain the rendered manual locale while
  // returning to System, discard a late older response, and let manual choice win.
  await page.locator(switchSel).selectOption('zh-CN');
  delayPreferred=true;nodePreferred=['en-US'];
  await page.locator(switchSel).selectOption('system');await waitPending(1);
  assert.equal(await page.locator('html').getAttribute('lang'),'zh-CN','pending System read retains current locale');
  assert.equal(await currentLanguage.innerText(),'中文','pending System read retains the displayed effective language');
  assert.equal(await page.locator(switchSel).inputValue(),'zh-CN','preference and System setting commit together');
  pendingPreferred.shift().resolve(['en-US']);await page.getByRole('button',{name:'Start a task',exact:true}).waitFor();
  nodePreferred=['zh-Hans'];await page.evaluate(()=>window.dispatchEvent(new Event('languagechange')));await waitPending(1);
  nodePreferred=['en-GB'];await page.evaluate(()=>window.dispatchEvent(new Event('languagechange')));await waitPending(2);
  pendingPreferred[1].resolve(pendingPreferred[1].value);await page.waitForTimeout(50);
  pendingPreferred[0].resolve(pendingPreferred[0].value);pendingPreferred.splice(0);await page.waitForTimeout(50);
  assert.equal(await page.locator('html').getAttribute('lang'),'en','older response cannot overwrite newer preference');
  assert.equal(await currentLanguage.innerText(),'EN','late reads cannot replace the effective language face');
  nodePreferred=['zh-Hans'];await page.evaluate(()=>window.dispatchEvent(new Event('languagechange')));await waitPending(1);
  await page.locator(switchSel).selectOption('en');pendingPreferred.shift().resolve(['zh-Hans']);await page.waitForTimeout(50);
  assert.equal(await page.locator(switchSel).inputValue(),'en','manual selection wins while native preference read is pending');
  assert.equal(await page.locator('html').getAttribute('lang'),'en');delayPreferred=false;
  report.checks.push('delayed System read is coherent; newest read wins out of order; manual override supersedes pending reads');
  // (4) Back to System uses fresh native preferences (zh-Hans), no fallback flash.
  nodePreferred=['zh-Hans-CN'];
  await page.locator(switchSel).selectOption('system');
  await page.getByRole('button',{name:'开始一项任务',exact:true}).waitFor();
  assert.equal(await page.locator('html').getAttribute('lang'),'zh-CN','System resolves to native zh-Hans without falling back');
  assert.equal(await page.locator(switchSel).inputValue(),'system');
  assert.equal(await currentLanguage.innerText(),'中文','Chinese System mode still displays 中文');
  assert.deepEqual(await page.locator(switchSel).locator('option').allTextContents(),['跟随系统','简体中文','English']);
  report.checks.push('returning to System re-reads native preferences (zh-Hans) with no fallback flash');
  // (5) System mode follows a languagechange; manual mode ignores it.
  nodePreferred=['en-GB'];await page.evaluate(()=>window.dispatchEvent(new Event('languagechange')));
  await page.getByRole('button',{name:'Start a task',exact:true}).waitFor();
  assert.equal(await page.locator('html').getAttribute('lang'),'en');
  await page.locator(switchSel).selectOption('zh-CN');
  nodePreferred=['en-GB'];await page.evaluate(()=>window.dispatchEvent(new Event('languagechange')));
  await page.waitForTimeout(200);
  assert.equal(await page.locator('html').getAttribute('lang'),'zh-CN','manual override ignores languagechange');
  report.checks.push('languagechange updates only in System mode; manual override is not silently replaced');
  // (5b) Native read failure keeps the fallback and never crashes; the manual
  // override still resolves without the native command.
  await page.locator(switchSel).selectOption('en');
  nodePreferred=[];failPreferred=true;await page.locator(switchSel).selectOption('system');
  await page.waitForTimeout(150);
  assert.equal(await page.getByRole('button',{name:'Start a task',exact:true}).count(),1,'native read failure falls back without crashing');
  failPreferred=false;
  report.checks.push('native preferred-language read failure keeps fallback and does not crash');
  // An unsubmitted form keeps exact user text while labels change under System.
  nodePreferred=['en-US'];await page.locator(switchSel).selectOption('system');
  await page.getByRole('button',{name:'New environment',exact:true}).click();
  const draftDialog=page.getByRole('dialog');
  await draftDialog.getByLabel('Environment name',{exact:true}).fill('草稿 raw-name 🐈');
  const draftCalls=coreCalls.length;
  nodePreferred=['zh-Hans'];await page.evaluate(()=>window.dispatchEvent(new Event('languagechange')));
  await draftDialog.getByRole('heading',{name:'建立专用环境',exact:true}).waitFor();
  assert.equal(await draftDialog.getByLabel('环境名称',{exact:true}).inputValue(),'草稿 raw-name 🐈');
  nodePreferred=['en-US'];await page.evaluate(()=>window.dispatchEvent(new Event('languagechange')));
  await draftDialog.getByRole('heading',{name:'Create a dedicated environment',exact:true}).waitFor();
  assert.equal(await draftDialog.getByLabel('Environment name',{exact:true}).inputValue(),'草稿 raw-name 🐈');
  assert.equal(coreCalls.length,draftCalls,'relabeling an open draft makes no core calls');
  await page.keyboard.press('Escape');
  report.checks.push('open form draft preserves exact user input without submission or extra core calls');
  // (6) Switch to en while the protection-plan page holds a live draft/approval-ready preview.
  nodePreferred=['en-US'];await page.locator(switchSel).selectOption('system');await page.getByRole('button',{name:'Start a task',exact:true}).waitFor();
  await page.getByRole('button',{name:'Protection plan',exact:true}).click();
  await page.getByRole('button',{name:'Preview changes',exact:true}).click();
  const dialog=page.getByRole('dialog');
  await dialog.getByRole('heading',{name:'Reduce outbound',exact:true}).waitFor();
  const planCallsAfterPreview=planCount;
  // Known backend Chinese fields render English.
  await dialog.getByText('Exact field write and readback',{exact:false}).first().waitFor();
  await dialog.getByText('Product metrics',{exact:false}).first().waitFor();
  // Raw frozen identity fields are intact and never translated (id visible in the
  // collapsed "Target and plan identifiers" details, so open it).
  await dialog.getByText('Target and plan identifiers',{exact:true}).click();
  assert.ok((await dialog.innerText()).includes(plan.id),'frozen plan id preserved');
  // The hash is carried by approval; exact execute arguments below verify it.
  // Switch language while the approval dialog is open: same plan/hash, no extra plan request.
  nodePreferred=['zh-Hans'];await page.evaluate(()=>window.dispatchEvent(new Event('languagechange')));
  await dialog.getByRole('button',{name:'批准并执行',exact:true}).waitFor();
  assert.equal(planCount,planCallsAfterPreview,'language switch while approval open issues no new plan');
  nodePreferred=['en-US'];await page.evaluate(()=>window.dispatchEvent(new Event('languagechange')));
  await dialog.getByRole('button',{name:'Approve and run',exact:true}).waitFor();
  assert.equal(planCount,planCallsAfterPreview,'no new plan on second switch');
  report.checks.push('switch during open approval preserves plan identity and issues no extra plan');
  // (7) Approve sends EXACT frozen plan id/hash; recorded args prove it.
  await dialog.getByRole('button',{name:'Approve and run',exact:true}).click();
  await page.waitForTimeout(200);
  assert.equal(execCalls.length,1,'exactly one execute issued');
  assert.deepEqual(execCalls[0],{plan_id:plan.id,approval:plan.hash},'execute carries the exact frozen id/hash');
  report.checks.push('approval executes the exact frozen plan id/hash');
  // (7b) Representative panels/dialogs render English (task help + settings + help).
  await page.keyboard.press('Escape');await page.waitForTimeout(150); // close the flow dialog if still open
  await page.getByRole('button',{name:'Environments',exact:true}).click();
  await page.getByRole('button',{name:'Start a task',exact:true}).click();
  const tasks=page.getByRole('dialog');
  await tasks.getByRole('heading',{name:'Start a task',exact:true}).waitFor();
  assert.equal(await tasks.locator('[data-task-id]').count(),6);
  // The six task labels come from the additive task-catalog English metadata.
  await tasks.getByText('Reduce outbound while keeping what you need',{exact:true}).first().waitFor();
  await page.keyboard.press('Escape');
  // Settings dialog: appearance + capabilities in English.
  await page.getByRole('button',{name:/Local workspace/}).click();
  const settings=page.getByRole('dialog');
  await settings.getByRole('heading',{name:'Settings & modules',exact:true}).waitFor();
  await settings.getByText('Claude Code configuration',{exact:true}).waitFor();
  await settings.getByText('Network enforcement',{exact:true}).waitFor();
  // Language selector lives in settings too; it is accessible.
  assert.equal(await settings.locator('.language-switch select').count()>=1,true);
  assert.equal(await settings.locator('.language-switch select').inputValue(),'system');
  assert.equal(await settings.locator('.language-switch select option:checked').innerText(),'Follow system','settings retains the full selected preference mode');
  const animationSwitch=settings.getByRole('checkbox',{name:'Interface animation',exact:true});
  const logoSwitch=settings.getByRole('checkbox',{name:'Logo cursor blink',exact:true});
  const terminalSwitch=settings.getByRole('checkbox',{name:'Terminal cursor blink',exact:true});
  assert.equal(await animationSwitch.isChecked(),true);
  assert.equal(await logoSwitch.isChecked(),true);
  assert.equal(await terminalSwitch.isChecked(),false);
  const beforeMotionCalls=coreCalls.length;
  await terminalSwitch.check();
  await logoSwitch.uncheck();
  assert.equal(await page.locator('.target-cursor').evaluate(node=>getComputedStyle(node).animationName),'brand-blink');
  assert.equal(await page.locator('.brand-cursor').evaluate(node=>getComputedStyle(node).animationName),'none');
  await animationSwitch.uncheck();
  assert.equal(await settings.evaluate(node=>getComputedStyle(node).animationName),'none','interface transitions stop independently');
  assert.equal(await page.locator('.target-cursor').evaluate(node=>getComputedStyle(node).animationName),'brand-blink','turning off interface effects retains an independently enabled cursor');
  await page.emulateMedia({reducedMotion:'reduce'});
  assert.equal(await page.locator('.target-cursor').evaluate(node=>getComputedStyle(node).animationName),'none','system reduction overrides enabled blink');
  await page.emulateMedia({reducedMotion:'no-preference'});
  assert.equal(coreCalls.length,beforeMotionCalls,'motion preferences issue no core operation');
  await page.reload();await page.locator('.home-title').waitFor();
  assert.deepEqual(await page.evaluate(()=>JSON.parse(localStorage.getItem('lintel.motion'))),{animations:false,logoBlink:false,terminalBlink:true});
  assert.equal(await page.locator('html').getAttribute('data-motion'),'off','quiet choice applies after reload');
  await page.getByRole('button',{name:/Local workspace/}).click();
  await settings.getByRole('heading',{name:'Settings & modules',exact:true}).waitFor();
  assert.equal(await animationSwitch.isChecked(),false);assert.equal(await logoSwitch.isChecked(),false);assert.equal(await terminalSwitch.isChecked(),true);
  await page.screenshot({path:path.join(output,'motion-settings.png')});
  await animationSwitch.check();await logoSwitch.check();await terminalSwitch.uncheck();
  report.checks.push('three independent local motion preferences persist across reload; interface effects and both cursors remain separate; system reduced motion wins; no core call');
  await page.keyboard.press('Escape');
  // Help dialog.
  await page.getByRole('button',{name:'Help',exact:true}).click();
  const help=page.getByRole('dialog');
  await help.getByRole('heading',{name:'Using Lintel',exact:true}).waitFor();
  await help.getByText('Know exactly what changed',{exact:true}).waitFor();
  await page.keyboard.press('Escape');
  report.checks.push('task help, settings capabilities and help dialog render English from additive metadata/catalog');

  // (8) 900px layout + accessible html lang; no horizontal overflow.
  await page.setViewportSize({width:900,height:640});
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),true,'900px no horizontal overflow');
  assert.equal(['en','zh-CN'].includes(await page.locator('html').getAttribute('lang')),true);
  for(const theme of ['light','dark']){
    await page.getByRole('button',{name:theme==='light'?'Light Day':'Dark Night',exact:true}).click();
    const geometry=await page.locator('.target-bar').evaluate(node=>({height:node.getBoundingClientRect().height,select:node.querySelector('.language-switch').getBoundingClientRect().width,context:node.querySelector('.target-context').getBoundingClientRect().width}));
    assert.ok(geometry.height<=60,'language control must not turn the toolbar into a tall form');
    assert.ok(geometry.select>=44&&geometry.select<=80,'minimal language face retains a usable hit area');
    assert.ok(geometry.context>200,'target context remains readable at 900px');
    await page.screenshot({path:path.join(output,`i18n-en-900-${theme}.png`)});
  }
  await page.locator(switchSel).focus();assert.equal(await page.locator(switchSel).evaluate(node=>node===document.activeElement),true,'selector remains keyboard focusable');
  assert.equal(await page.locator('.target-bar .language-control').evaluate(node=>getComputedStyle(node).boxShadow!=='none'),true,'keyboard focus is visible on the compact face');
  // Chromium on macOS does not drive the OS-owned select popup with Playwright
  // key events, including an ordinary visible select. Focus/visible feedback is
  // checked here; native popup clicking/keyboard selection needs native evidence.
  nodePreferred=['zh-Hans'];await page.evaluate(()=>window.dispatchEvent(new Event('languagechange')));
  await page.getByRole('button',{name:'开始一项任务',exact:true}).waitFor();
  await page.screenshot({path:path.join(output,'i18n-zh-system-900-dark.png')});
  nodePreferred=['en-US'];await page.evaluate(()=>window.dispatchEvent(new Event('languagechange')));
  await page.getByRole('button',{name:'Start a task',exact:true}).waitFor();
  assert.deepEqual(errors,[]);
  report.checks.push('minimal EN/中文 face tracks effective locale; full native choices and visible keyboard focus remain accessible at 900px in Day/Night');
  // The development build runs the real main.tsx StrictMode setup/cleanup/setup.
  // A preference refresh after this mount must still apply (production React
  // does not exercise this lifecycle).
  devPreview=spawn(process.execPath,[path.join(desktop,'node_modules/vite/bin/vite.js'),'--host','127.0.0.1','--port','0','--strictPort'],{cwd:desktop,stdio:['ignore','pipe','pipe']});
  const devUrl=await new Promise((resolve,reject)=>{let text='';const timer=setTimeout(()=>reject(new Error('dev preview timeout')),15000);devPreview.once('error',reject);devPreview.once('exit',code=>reject(new Error(`dev preview exited ${code}`)));for(const stream of [devPreview.stdout,devPreview.stderr])stream.on('data',chunk=>{text+=chunk;const match=stripVTControlCharacters(text).match(/http:\/\/127\.0\.0\.1:\d+\//);if(match){clearTimeout(timer);resolve(match[0]);}});});
  // The cold dev module graph is transformed on demand, unlike the built App.
  // Give that startup its own bound; keep the language interaction waits at 12s.
  nodePreferred=['en-US'];await page.goto(devUrl,{timeout:60000});await page.getByRole('button',{name:'Start a task',exact:true}).waitFor();
  nodePreferred=['zh-Hans'];await page.evaluate(()=>window.dispatchEvent(new Event('languagechange')));
  await page.getByRole('button',{name:'开始一项任务',exact:true}).waitFor();
  await page.locator(switchSel).selectOption('en');
  await page.getByRole('button',{name:'Start a task',exact:true}).waitFor();
  assert.equal(menuCalls.at(-1),'en');assert.deepEqual(errors,[]);
  report.checks.push('actual development StrictMode remount keeps provider active; System refresh and manual override still work');
  report.passed=true;
  await page.screenshot({path:path.join(output,'i18n-en-900.png')});
}catch(e){
  report.error=e&&e.message?e.message:String(e);
  if(page){try{await page.screenshot({path:path.join(output,'failure.png')});}catch{}}
  throw e;
}finally{
  try{await browser?.close();}catch{}
  if(devPreview&&devPreview.exitCode===null){try{const end=new Promise(r=>devPreview.once('exit',r));devPreview.kill('SIGTERM');await end;}catch{}}
  if(preview.exitCode===null){try{const end=new Promise(r=>preview.once('exit',r));preview.kill('SIGTERM');await end;}catch{}}
  await writeFile(process.env.LINTEL_I18N_UI_REPORT||path.join(output,'report.json'),JSON.stringify(report,null,2));
  console.log(JSON.stringify({status:report.passed?'passed':'failed',checks:report.checks,error:report.error}));
}
