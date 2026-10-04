// Built App + real core in disposable homes; invoke fixture, not native WebKit.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {copyFile,mkdir,mkdtemp,readFile,realpath,writeFile} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {stripVTControlCharacters} from 'node:util';
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'),desktop=path.join(repo,'apps/desktop');
const {chromium}=await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE||path.join(repo,'extensions/browser/node_modules/playwright/index.mjs')).href);
const fixture=await realpath(await mkdtemp(path.join(os.tmpdir(),'lintel-work-ui-'))),password='synthetic-ui-work-passphrase';
const home=path.join(fixture,'source-home'),root=path.join(home,'.claude');
await mkdir(path.join(root,'projects/synthetic/memory'),{recursive:true});
for(const [name,content] of [['CLAUDE.md','# Synthetic instructions\n'],['projects/synthetic/memory/MEMORY.md','Synthetic memory\n'],['projects/synthetic/session.jsonl','{"synthetic":true}\n'],['settings.json','{"env":{"SYNTHETIC_KEEP":"1"}}'],['.credentials.json','SYNTHETIC_CREDENTIAL']])await writeFile(path.join(root,name),content);
const sourceBytes=await readFile(path.join(root,'settings.json'));
let context={...process.env,HOME:home,LINTEL_TEST_HOME:home,LINTEL_STATE_DIR:path.join(fixture,'source-state')};
const runner=process.env.LINTEL_FIXTURE_RUNNER||path.join(repo,'target/debug/lintel'),calls=[],resources=[];
// Fixture-side assertions and the rendered window share one synthetic actor.
// Serialize their real core requests, as the App transport does, and retain
// the original home's context even if the next journey switches installation.
let coreQueue=Promise.resolve();
function core(payload){calls.push(payload.command);const requestContext=context;
  const result=coreQueue.then(()=>runCore(payload,requestContext));coreQueue=result.catch(()=>undefined);return result;
}
async function runCore(payload,requestContext){return new Promise((resolve,reject)=>{
  const child=spawn(runner,['request'],{env:requestContext,stdio:['pipe','pipe','pipe']});let output='',errors='';
  const timer=setTimeout(()=>{child.kill();reject(new Error('synthetic core timeout'));},55000);
  child.stdout.on('data',d=>output+=d);child.stderr.on('data',d=>errors+=d);child.once('error',reject);
  child.once('close',()=>{clearTimeout(timer);assert.ok(!output.includes(password)&&!errors.includes(password),'Secret in process output');try{resolve(JSON.parse(output));}catch{reject(new Error('Invalid core envelope: '+errors));}});
  child.stdin.end(JSON.stringify(payload));
});}
async function data(payload){const r=await core(payload);assert.ok(r.ok,r.error?.message);return r.data;}
const source=await data({command:'register',root,name:'Synthetic source'}),inventory=await data({command:'discover'});
const preview=spawn(process.execPath,[path.join(desktop,'node_modules/vite/bin/vite.js'),'preview','--host','127.0.0.1','--port','0','--strictPort'],{cwd:desktop,stdio:['ignore','pipe','pipe']});
let browser;
const report={fixture,runtime:'built App + real synthetic core via invoke fixture; not native WebKit',checks:[],passed:false};
try{
  const url=await new Promise((resolve,reject)=>{let output='';const timer=setTimeout(()=>reject(new Error('preview timeout: '+stripVTControlCharacters(output))),15000);preview.once('error',reject);preview.once('exit',code=>reject(new Error('preview exited '+code)));for(const stream of [preview.stdout,preview.stderr])stream.on('data',chunk=>{output+=chunk;const match=stripVTControlCharacters(output).match(/http:\/\/127\.0\.0\.1:\d+\//);if(match){clearTimeout(timer);resolve(match[0]);}});});report.url=url;
  browser=await chromium.launch({headless:true});const page=await browser.newPage({viewport:{width:1120,height:800}});page.setDefaultTimeout(10000);
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.exposeFunction('syntheticInvoke',async(command,args)=>{if(command==='open_resource'){resources.push(args.resource);return null;}assert.equal(command,'request');return core(args.payload);});
  await page.addInitScript(()=>{window.isTauri=true;window.__TAURI_INTERNALS__={invoke:(command,args)=>window.syntheticInvoke(command,args)};});
  await page.goto(url);await page.getByRole('button',{name:'帮助',exact:true}).focus();await page.keyboard.press('Enter');
  const dialog=page.getByRole('dialog');assert.equal(await dialog.locator('.help-tasks section').count(),6);
  await page.screenshot({path:path.join(fixture,'help-day.png')});await dialog.getByRole('link',{name:/完整人类操作指南/}).click();assert.deepEqual(resources,['operator-guide']);
  await dialog.getByRole('button',{name:/工作保全/}).click();await page.getByRole('heading',{name:'正在做的事，好好收着',exact:true}).waitFor();
  for(const [width,height,theme,button] of [[1120,800,'day','浅色 Day'],[900,640,'night','深色 Night']]){await page.setViewportSize({width,height});await page.getByRole('button',{name:button,exact:true}).click();await page.waitForFunction(expected=>document.documentElement.dataset.theme===expected,theme==='day'?'light':'dark');await page.evaluate(()=>Promise.all(document.getAnimations({subtree:true}).filter(a=>Number.isFinite(a.effect.getComputedTiming().iterations)).map(a=>a.finished.catch(()=>{}))));await page.screenshot({path:path.join(fixture,`work-${theme}.png`),fullPage:true});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'Work page horizontal overflow');await page.locator('main').evaluate(el=>el.scrollTop=el.scrollHeight);await page.screenshot({path:path.join(fixture,`work-${theme}-actions.png`)});await page.locator('main').evaluate(el=>el.scrollTop=0);}
  const carried=path.join(home,'carried.age');await page.getByLabel('另存加密包的完整路径（可选）').fill(carried);
  await page.getByRole('button',{name:'预览保全计划',exact:true}).focus();await page.keyboard.press('Enter');await dialog.getByRole('heading',{name:'仅加密归档所选工作内容',exact:true}).waitFor();
  assert.equal(await dialog.locator('.plan-steps .action-description').count(),1);await approve();
  assert.equal((await data({command:'discover'})).environments.length,inventory.environments.length);assert.deepEqual(await readFile(path.join(root,'settings.json')),sourceBytes);assert.equal(await readFile(path.join(root,'.credentials.json'),'utf8'),'SYNTHETIC_CREDENTIAL');
  await dialog.getByRole('button',{name:'查看工作归档',exact:true}).click();await dialog.getByLabel('归档口令',{exact:true}).fill(password);await dialog.getByRole('button',{name:'解锁并查看',exact:true}).click();await dialog.getByRole('button',{name:/CLAUDE.md/}).click();await dialog.locator('.archive-text pre').getByText('# Synthetic instructions',{exact:false}).waitFor();
  await dialog.getByRole('button',{name:'关闭面板',exact:true}).click();assert.ok(!(await page.evaluate(()=>JSON.stringify(localStorage))).includes(password));
  report.checks.push('Keyboard task help → archive-only plan/approve → real package/read; no new root, settings/auth unchanged; secret absent from storage');
  await page.getByLabel(/保全并准备新环境/).check();await page.getByLabel('新环境名称（可选）').fill('Synthetic continuation');await page.getByRole('button',{name:'预览保全计划',exact:true}).click();
  await dialog.locator('.plan-steps .action-description').first().waitFor();assert.equal(await dialog.locator('.plan-steps .action-description').count(),3);await approve();await dialog.getByText(/工作保全已完成/).waitFor();
  await dialog.getByRole('button',{name:'为新环境选择保护方案',exact:true}).click();await page.getByRole('heading',{name:'保护方案',exact:true}).waitFor();assert.notEqual(await page.getByLabel('当前环境',{exact:true}).inputValue(),source.id);assert.deepEqual(await readFile(path.join(root,'settings.json')),sourceBytes);
  report.checks.push('Preserve → completed outcome → new environment policy; old root/login untouched');
  const targetHome=path.join(fixture,'target-home');await mkdir(targetHome);context={...context,HOME:targetHome,LINTEL_TEST_HOME:targetHome,LINTEL_STATE_DIR:path.join(fixture,'target-state')};
  const portable=path.join(targetHome,'portable.age');await copyFile(carried,portable);const destination=await data({command:'create_environment',name:'Synthetic destination'});
  await page.reload();await page.getByRole('button',{name:'工作归档',exact:true}).click();assert.equal(await dialog.getByRole('button',{name:'当前主机的任务归档',exact:true}).isDisabled(),true);
  await dialog.getByLabel('加密包完整路径',{exact:true}).fill(portable);await dialog.getByLabel('归档口令',{exact:true}).fill('wrong-passphrase');await dialog.getByRole('button',{name:'解锁并查看',exact:true}).click();await dialog.getByRole('alert').waitFor();
  await dialog.getByLabel('归档口令',{exact:true}).fill(password);await dialog.getByRole('button',{name:'解锁并查看',exact:true}).click();await dialog.getByRole('heading',{name:'选择性迁入',exact:true}).waitFor();await dialog.getByLabel('会话资料',{exact:true}).uncheck();await dialog.getByRole('button',{name:'预览迁入计划',exact:true}).click();await approve();
  assert.equal(await readFile(path.join(destination.root,'CLAUDE.md'),'utf8'),'# Synthetic instructions\n');assert.equal(await readFile(path.join(destination.root,'lintel-imports/projects/synthetic/memory/MEMORY.md'),'utf8'),'Synthetic memory\n');await page.screenshot({path:path.join(fixture,'portable-import-night.png')});assert.deepEqual(errors,[]);
  report.checks.push('Independent install/empty jobs → wrong-password feedback → portable selective import → real files, no original job state');
  // Explicit synthetic retained-probe receipt, read through the real core job
  // API. This verifies UI recovery presentation, not a real killed worker.
  const modeled=(await data({command:'jobs'})).jobs[0],probe=modeled.migration_probe.path;
  await mkdir(probe);await writeFile(path.join(probe,'placeholder'),'');
  modeled.title='Synthetic retained probe receipt';modeled.status='needs_reconciliation';modeled.migration_probe.status='retained';
  // A recorded create intent is not a registered environment. Preserve both
  // recovery paths, but offer policy editing only for actual inventory entries.
  modeled.new_environment_id='11111111-2222-4333-8444-555555555555';
  modeled.new_root=path.join(fixture,'unregistered-new-root');
  modeled.steps=[{id:'migration_preflight',label:'模拟路径检查中断',status:'executing',message:'Synthetic retained probe fixture.'}];
  await writeFile(path.join(context.LINTEL_STATE_DIR,'jobs',`${modeled.id}.json`),JSON.stringify(modeled));
  const executeCount=calls.filter(command=>command==='execute').length;
  for(const [theme,button] of [['night','深色 Night'],['day','浅色 Day']]){
    await page.reload();await page.getByRole('button',{name:button,exact:true}).click();
    await page.getByRole('button',{name:'记录与恢复',exact:true}).click();
    await page.locator('.job-row').filter({hasText:modeled.title}).getByRole('button',{name:'查看结果',exact:true}).click();
    await dialog.getByText('路径检查临时目录',{exact:true}).scrollIntoViewIfNeeded();
    await dialog.getByText(probe,{exact:true}).waitFor();
    await dialog.getByText('新配置目录（待核对）',{exact:true}).waitFor();
    await dialog.getByText(modeled.new_root,{exact:true}).waitFor();
    assert.equal(await dialog.getByRole('button',{name:'为新环境选择保护方案'}).count(),0);
    await page.screenshot({path:path.join(fixture,`probe-recovery-${theme}.png`)});
  }
  assert.equal(calls.filter(command=>command==='execute').length,executeCount);
  assert.equal(await readFile(path.join(probe,'placeholder'),'utf8'),'');
  report.checks.push('Modeled retained-probe receipt → real original-job query → exact scratch path visible in Day/Night; no replay or cleanup');
  report.calls=[...new Set(calls)];report.passed=true;
  async function approve(){await dialog.getByLabel('归档口令',{exact:true}).fill(password);await dialog.getByLabel('再次输入口令',{exact:true}).fill(password);await dialog.getByRole('button',{name:'批准并执行',exact:true}).click();await dialog.getByRole('heading',{name:'执行结果',exact:true}).waitFor();}
}finally{await browser?.close();preview.kill('SIGTERM');const reportPath=process.env.LINTEL_WORK_UI_REPORT||path.join(fixture,'report.json');await mkdir(path.dirname(reportPath),{recursive:true});await writeFile(reportPath,JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));}
