// Built App + real synthetic cores. The SSH/invoke adapter is modeled, never a VPS.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdir,mkdtemp,realpath,writeFile,readFile,chmod,open} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {stripVTControlCharacters} from 'node:util';
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'),desktop=path.join(repo,'apps/desktop');
const {chromium}=await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE||path.join(repo,'extensions/browser/node_modules/playwright/index.mjs')).href);
const fixture=await realpath(await mkdtemp(path.join(os.tmpdir(),'lintel-components-ui-'))),runner=path.join(repo,'target/debug/lintel');
const calls=[],consoleErrors=[],aliases=['synthetic-component-A','synthetic-component-B'];
const original='eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee';
const cores=new Map();let held=null,requested=false;
function raw(actor,args,payload){return new Promise((resolve,reject)=>{const child=spawn(runner,args,{env:actor,stdio:['pipe','pipe','pipe']});let out='',err='';child.stdout.on('data',d=>out+=d);child.stderr.on('data',d=>err+=d);child.once('error',reject);child.once('close',()=>{try{resolve(JSON.parse(out))}catch{reject(new Error(err+out))}});child.stdin.end(payload?JSON.stringify(payload):'');});}
for(const [index,alias] of [null,...aliases].entries()){
 const home=path.join(fixture,'home-'+index),state=path.join(fixture,'state-'+index),root=path.join(home,'config 关联环境 '+index),project=path.join(home,'project 来源');
 await mkdir(root,{recursive:true});await mkdir(path.join(project,'.claude'),{recursive:true});
 await writeFile(path.join(root,'settings.json'),alias===aliases[1]?'malformed':'{"hooks":{"anything":"SYNTHETIC_EXECUTION_SECRET"}}');
 await writeFile(path.join(root,'CLAUDE.md'),'Synthetic instructions');await mkdir(path.join(root,'projects/demo/memory'),{recursive:true});await writeFile(path.join(root,'projects/demo/memory/MEMORY.md'),'Synthetic memory');
 const large=path.join(root,'projects/demo/large-session.jsonl'),file=await open(large,'w');await file.truncate(256*1024*1024+1);await file.close();
 await writeFile(path.join(project,'.claude/settings.local.json'),'{"mcpServers":{"private":"SYNTHETIC_MCP_SECRET"}}');
 const executable=path.join(home,'.local/bin/claude');await mkdir(path.dirname(executable),{recursive:true});await writeFile(executable,'#!/bin/sh\nexit 9\n');await chmod(executable,0o700);
 const actor={...process.env,HOME:home,LINTEL_TEST_HOME:home,LINTEL_STATE_DIR:state,PATH:path.join(home,'.local/bin')};
 const e=await raw(actor,['env','register','--name',alias??'Synthetic local components','--root',root]);assert.ok(e.ok,e.error?.message);
 if(alias===aliases[0])await writeFile(path.join(state,'jobs',original+'.json'),JSON.stringify({id:original,environment_id:e.data.id,status:'completed',title:'Synthetic original service task',created_at:'2026-10-06T12:00:00Z',warnings:[],restorable:false,steps:[],service:{manager:'user',unit:'synthetic.service',root,before:{active_state:'active',restart:'always'},after:{active_state:'inactive',hold:true},hold:{path:path.join(home,'synthetic-units/hold.conf'),persistent:true}},task_result:{outcome:'partial',primary:'Synthetic completed root; independent browser remains',selected_steps:[],coverage:[{scope:'configuration',state:'done',detail:'Root fields processed'},{scope:'browser',state:'not_checked',detail:'Specific profile remains independent'}],next_actions:[]}}));
 cores.set(alias,{actor,environment:e.data,project});
}
const queues=new Map();
async function core(alias,payload){calls.push({alias,payload});if(alias===aliases[0]&&payload.command==='inspect_components'&&held){requested=true;await held.promise;}
 const current=cores.get(alias);const work=(queues.get(alias)||Promise.resolve()).then(()=>raw(current.actor,['request'],payload));queues.set(alias,work.catch(()=>{}));return work;}
const preview=spawn(process.execPath,[path.join(desktop,'node_modules/vite/bin/vite.js'),'preview','--host','127.0.0.1','--port','0','--strictPort'],{cwd:desktop,stdio:['ignore','pipe','pipe']});
const report={fixture,runtime:'built App + real synthetic cores; modeled finite SSH/invoke, not native WebKit/production',checks:[],passed:false};let browser,page;
try{
 const url=await new Promise((resolve,reject)=>{let out='';const timer=setTimeout(()=>reject(new Error('preview startup timeout')),15000);preview.once('error',reject);for(const stream of [preview.stdout,preview.stderr])stream.on('data',data=>{out+=stripVTControlCharacters(data.toString());const match=out.match(/http:\/\/127\.0\.0\.1:\d+\//);if(match){clearTimeout(timer);resolve(match[0]);}});});report.url=url;
 browser=await chromium.launch({headless:true});page=await browser.newPage({viewport:{width:1120,height:760}});page.setDefaultTimeout(15000);page.on('pageerror',e=>consoleErrors.push(e.message));
 await page.exposeFunction('syntheticInvoke',async(command,args)=>{
  if(command==='request')return core(null,args.payload);
  if(command==='remote_request'){
   const p=args.payload;
   if(p.op==='hosts')return {ok:true,data:{hosts:aliases.map(alias=>({alias})),tasks:[],installations:[],launches:[]}};
   if(p.op==='aliases')return {ok:true,data:{aliases,coverage:'synthetic'}};
   if(p.op==='connect')return {ok:true,data:{status:'connected'}};
   if(p.op==='request')return core(p.alias,p.request);
  }
  throw new Error('unexpected invoke '+command+JSON.stringify(args));
 });
 await page.addInitScript(()=>{window.isTauri=true;window.__TAURI_INTERNALS__={invoke:(command,args)=>window.syntheticInvoke(command,args)};});
 await page.goto(url);await page.getByRole('button',{name:'环境详情',exact:true}).click();await page.getByRole('tab',{name:'关联组件',exact:true}).click();
 const panel=page.getByRole('region',{name:'关联组件与处理范围'});
 await panel.locator('[data-component=desktop_ide]').getByText('暂不支持',{exact:true}).waitFor();
 assert.equal(await panel.locator('[data-component=authentication] .component-state').innerText(),'未知');
 await panel.getByLabel('用于检查来源的项目目录（可选）').fill(cores.get(null).project);await panel.getByRole('button',{name:'刷新组件',exact:true}).click();
 await panel.getByText(/项目/, {exact:false}).first().waitFor();await panel.getByText('查看有限配置来源',{exact:true}).click();await panel.getByText('project_local',{exact:true}).waitFor();
 const text=await panel.innerText();assert.ok(!text.includes('SYNTHETIC_EXECUTION_SECRET')&&!text.includes('SYNTHETIC_MCP_SECRET'));
 report.checks.push('finite component facts and explicit cwd sources; account unknown/unsupported adapters labeled; execution-bearing values stay private');
 await page.getByRole('button',{name:'工作保全',exact:true}).click();const capacity=page.getByRole('region',{name:'工作容量预检'});
 await capacity.getByText('projects/demo/large-session.jsonl',{exact:true}).waitFor();await capacity.evaluate(el=>el.scrollIntoView({block:'start'}));await page.screenshot({path:path.join(fixture,'capacity-blocked-light-1120.png')});assert.ok(await page.getByRole('button',{name:'预览保全计划',exact:true}).isDisabled());
 await page.getByLabel('会话资料',{exact:false}).uncheck();await capacity.getByText('当前元数据在容量范围内；计划预览仍会读取并核对全部选中原件。',{exact:true}).waitFor();
 await page.getByRole('button',{name:'预览保全计划',exact:true}).click();await page.getByRole('heading',{name:'确认这份计划',exact:true}).waitFor();await page.getByRole('button',{name:'返回调整',exact:true}).click();
 assert.equal(calls.some(c=>c.payload.command==='execute'),false);report.checks.push('9MiB blocker identifies exact file; category correction admits ordinary plan; no truncation/delete/execute');
 async function selectHost(alias){await page.locator('.host-switch').click();const dialog=page.getByRole('dialog');await dialog.locator('.host-row').filter({hasText:alias}).getByRole('button',{name:'连接并管理',exact:true}).click();await page.locator('.host-switch').getByText(alias,{exact:true}).waitFor();if(await page.getByRole('button',{name:'环境详情',exact:true}).getAttribute('aria-expanded')!=='true')await page.getByRole('button',{name:'环境详情',exact:true}).click();await page.getByRole('tab',{name:'关联组件',exact:true}).click();}
 await selectHost(aliases[0]);await panel.locator('[data-component=services] .component-service').waitFor();await panel.getByRole('button',{name:'核对原服务任务',exact:true}).click();const receipt=page.getByRole('dialog');await receipt.getByRole('heading',{name:'执行结果',exact:true}).waitFor();await receipt.locator('.receipt-id').getByText(original,{exact:true}).waitFor();
 assert.equal(calls.at(-1).alias,aliases[0]);assert.deepEqual(calls.at(-1).payload,{command:'job',job_id:original});await receipt.getByRole('button',{name:'关闭面板',exact:true}).click();
 held={};held.promise=new Promise(resolve=>held.release=resolve);await panel.getByRole('button',{name:'刷新组件',exact:true}).click();await wait(()=>requested);
 await selectHost(aliases[1]);await panel.locator('[data-component=services] .component-state').waitFor();held.release();held=null;await page.waitForTimeout(100);
 assert.ok(!(await panel.innerText()).includes(original));assert.ok((await panel.innerText()).includes(cores.get(aliases[1]).environment.root));assert.equal(await panel.locator('[data-component=configuration] .component-state').innerText(),'未知');
 for(const [width,height,theme] of [[1120,760,'light'],[900,640,'dark']]){
  await page.setViewportSize({width,height});await page.evaluate(theme=>document.documentElement.dataset.theme=theme,theme);
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
  assert.equal(await panel.evaluate(el=>el.scrollWidth>el.clientWidth),false);
  await panel.evaluate(el=>el.scrollIntoView({block:'start'}));await page.screenshot({path:path.join(fixture,`components-${theme}-${width}.png`)});
 }
 assert.deepEqual(consoleErrors,[]);assert.equal(calls.some(c=>c.payload.command==='auth_probe'||c.payload.command==='execute'),false);
 report.checks.push('original service/query retains exact alias/environment; late A report cannot enter B; Day/Night 1120/900 no horizontal overflow or runtime errors');report.passed=true;console.log('PASS: component scope/original task recovery/capacity admission built UI journey');
} catch(error){report.error=error.stack;report.consoleErrors=consoleErrors;report.calls=calls;report.body=await page?.locator('body').innerText().catch(()=>null);await page?.screenshot({path:path.join(fixture,'failure.png')}).catch(()=>{});throw error;} finally{
 held?.release();await browser?.close();if(preview.exitCode===null){const ended=new Promise(resolve=>preview.once('exit',resolve));preview.kill('SIGTERM');await ended;}
 await writeFile(process.env.LINTEL_COMPONENTS_UI_REPORT||path.join(os.tmpdir(),'lintel-components-ui.json'),JSON.stringify(report,null,2));
}
async function wait(check){const deadline=Date.now()+10000;while(Date.now()<deadline){if(await check())return;await new Promise(resolve=>setTimeout(resolve,25));}throw new Error('boundary not observed');}
