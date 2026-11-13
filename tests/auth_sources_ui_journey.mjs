// Built App with production core on disposable synthetic homes. SSH/invoke and
// Claude status are inert adapters; this is not native WebKit or live auth proof.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {chmod,mkdir,mkdtemp,readFile,realpath,writeFile} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {stripVTControlCharacters} from 'node:util';
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'),desktop=path.join(repo,'apps/desktop');
const {chromium}=await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE||path.join(repo,'extensions/browser/node_modules/playwright/index.mjs')).href);
const fixture=await realpath(await mkdtemp(path.join(os.tmpdir(),'lintel-auth-sources-ui-'))),runner=path.join(repo,'target/debug/lintel');
const artifacts=process.env.LINTEL_AUTH_SOURCES_UI_ARTIFACT_DIR||fixture;
await mkdir(artifacts,{recursive:true});
const aliases=['synthetic-auth-A','synthetic-auth-B'],cores=new Map(),calls=[],errors=[];
const secrets=['SYNTHETIC_OAUTH_DO_NOT_PUBLISH','SYNTHETIC_API_DO_NOT_PUBLISH','SYNTHETIC_SETUP_DO_NOT_PUBLISH','SYNTHETIC_MCP_DO_NOT_PUBLISH'];
let held=null,requested=false;
function raw(actor,args,payload){return new Promise((resolve,reject)=>{const child=spawn(runner,args,{env:actor,stdio:['pipe','pipe','pipe']});let out='',err='';child.stdout.on('data',data=>out+=data);child.stderr.on('data',data=>err+=data);child.once('error',reject);child.once('close',()=>{try{resolve(JSON.parse(out));}catch{reject(new Error(err+out));}});child.stdin.end(payload?JSON.stringify(payload):'');});}
for(const [index,alias] of [null,...aliases].entries()){
 const home=path.join(fixture,'home-'+index),state=path.join(fixture,'state-'+index),root=path.join(home,index===0?'.claude':'custom Claude '+index),bin=path.join(home,'.local/bin');
 await mkdir(root,{recursive:true});await mkdir(bin,{recursive:true});
 await writeFile(path.join(root,'.credentials.json'),JSON.stringify({claudeAiOauth:{accessToken:secrets[0],refreshToken:secrets[0]},mcpOAuth:{synthetic:secrets[3]}}));
 await writeFile(path.join(root,'settings.json'),JSON.stringify({env:{ANTHROPIC_API_KEY:secrets[1],KEEP_ME:'yes'},apiKeyHelper:'printf '+secrets[1],mcpServers:{preserve:{command:'inert'}}}));
 await writeFile(path.join(home,'.zshrc'),`# Synthetic startup only\nexport CLAUDE_CODE_OAUTH_TOKEN='${secrets[2]}'\nexport KEEP_ME='unchanged'\n`);
 await writeFile(path.join(home,'.bashrc'),'export ANTHROPIC_AUTH_TOKEN="$(printf synthetic-dynamic)"\n');
 await writeFile(path.join(root,'CLAUDE.md'),'Synthetic work remains byte-for-byte.\n');
 const executable=path.join(bin,'claude');
 await writeFile(executable,'#!/bin/sh\nif [ "$1" = "auth" ] && [ "$2" = "status" ]; then\n  printf \'{"authMethod":"oauth_token","configDirectory":"%s"}\\n\' "$CLAUDE_CONFIG_DIR"\n  exit 0\nfi\nexit 8\n');await chmod(executable,0o700);
 // Deliberate allowlist: no actual process credential or HOME reaches fixtures.
 const actor={HOME:home,LINTEL_TEST_HOME:home,LINTEL_STATE_DIR:state,PATH:bin,USER:'synthetic',CLAUDE_CODE_OAUTH_TOKEN:'SYNTHETIC_INHERITED_DO_NOT_PUBLISH'};
 const registered=await raw(actor,['env','register','--name',alias||'Synthetic local authentication','--root',root]);assert.ok(registered.ok,registered.error?.message);
 cores.set(alias,{actor,root,home,state,environment:registered.data});
}
const queues=new Map();
async function core(alias,payload){calls.push({alias,payload});if(alias===aliases[0]&&payload.command==='auth_inventory'&&held){requested=true;await held.promise;}
 const actor=cores.get(alias).actor;const work=(queues.get(alias)||Promise.resolve()).then(()=>raw(actor,['request'],payload));queues.set(alias,work.catch(()=>{}));return work;}
const preview=spawn(process.execPath,[path.join(desktop,'node_modules/vite/bin/vite.js'),'preview','--host','127.0.0.1','--port','0','--strictPort'],{cwd:desktop,stdio:['ignore','pipe','pipe']});
const report={fixture,runtime:'built App + real synthetic core; inert Claude status and modeled SSH/invoke; not native WebKit',checks:[],passed:false};let browser,page;
try{
 const url=await new Promise((resolve,reject)=>{let out='';const timer=setTimeout(()=>reject(new Error('preview startup timeout')),15000);preview.once('error',reject);for(const stream of [preview.stdout,preview.stderr])stream.on('data',data=>{out+=stripVTControlCharacters(data.toString());const match=out.match(/http:\/\/127\.0\.0\.1:\d+\//);if(match){clearTimeout(timer);resolve(match[0]);}});});
 browser=await chromium.launch({headless:true,...(process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE?{executablePath:process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE}:{})});page=await browser.newPage({viewport:{width:1120,height:820}});page.setDefaultTimeout(20000);page.on('pageerror',error=>errors.push(error.message));
 await page.exposeFunction('syntheticInvoke',async(command,args)=>{
  if(command==='request')return core(null,args.payload);
  if(command==='remote_request'){
   const payload=args.payload;
   if(payload.op==='hosts')return {ok:true,data:{hosts:aliases.map(alias=>({alias})),tasks:[],installations:[],launches:[]}};
   if(payload.op==='aliases')return {ok:true,data:{aliases,coverage:'synthetic'}};
   if(payload.op==='connect')return {ok:true,data:{status:'connected'}};
   if(payload.op==='request')return core(payload.alias,payload.request);
  }
  throw new Error('unexpected synthetic invoke '+command);
 });
 await page.addInitScript(()=>{try{localStorage.setItem('lintel.language','zh-CN');}catch{}window.isTauri=true;window.__TAURI_INTERNALS__={invoke:(command,args)=>command==='app_set_language'?null:command==='app_preferred_languages'?['zh-CN']:window.syntheticInvoke(command,args)};});
 await page.goto(url);
 // Environment / Components only reads credential declarations after a click.
 await page.getByRole('button',{name:'环境详情',exact:true}).click();await page.getByRole('tab',{name:'关联组件',exact:true}).click();
 const component=page.locator('[data-component=authentication]');await component.getByRole('button',{name:'检查认证来源',exact:true}).waitFor();
 assert.equal(calls.some(call=>call.payload.command==='auth_inventory'),false);
 await component.getByRole('button',{name:'检查认证来源',exact:true}).click();await component.locator('[data-auth-kind=credential_field]').waitFor();
 assert.equal(calls.some(call=>call.payload.command==='auth_probe'),false);report.checks.push('environment components require explicit bounded source read and never start Claude automatically');
 await page.getByRole('button',{name:'清理与重建',exact:true}).click();
 let inventory=page.getByRole('region',{name:'认证从哪里来',exact:true});await inventory.locator('[data-auth-kind=credential_field] input').waitFor();
 assert.equal(await inventory.locator('input[type=checkbox]:checked').count(),1);
 assert.equal(await inventory.locator('[data-auth-kind=shell_assignment] input:checked').count(),0);
 assert.equal(await inventory.locator('[data-auth-kind=process_env] input').count(),0);
 assert.equal(await page.getByRole('checkbox',{name:'同时请求官方注销',exact:true}).count(),0);
 await inventory.getByRole('button',{name:'选择可精确移除的来源',exact:true}).click();
 assert.ok(await inventory.locator('input[type=checkbox]:checked').count()>=4);
 assert.ok((await inventory.innerText()).includes('此主机用户共享'));
 await page.getByRole('checkbox',{name:'已关闭目标的终端、IDE 与自动重启来源',exact:false}).check();
 await page.getByRole('checkbox',{name:'清理后显式检查此执行上下文的认证',exact:false}).check();
 const chosen=await inventory.locator('input[type=checkbox]:checked').count();
 await page.getByRole('combobox',{name:'界面语言',exact:true}).selectOption('en');
 inventory=page.getByRole('region',{name:'Where authentication comes from',exact:true});
 assert.equal(await inventory.locator('input[type=checkbox]:checked').count(),chosen);
 assert.equal(calls.some(call=>call.payload.command==='plan_cleanup'),false);
 const visible=await page.locator('body').innerText();for(const secret of [...secrets,'SYNTHETIC_INHERITED_DO_NOT_PUBLISH'])assert.equal(visible.includes(secret),false);
 await page.getByRole('button',{name:'Preview this plan',exact:true}).click();
 const dialog=page.getByRole('dialog');await dialog.getByRole('heading',{name:'Confirm this plan',exact:true}).waitFor();
 const planCall=calls.findLast(call=>call.payload.command==='plan_cleanup');assert.equal(planCall.payload.auth_source_ids.length,chosen);assert.equal(planCall.payload.official_logout,false);assert.equal(planCall.payload.verify_auth_status,true);
 await dialog.getByRole('region',{name:'Authentication sources selected for exact removal',exact:true}).waitFor();
 await page.screenshot({path:path.join(artifacts,'auth-plan-en-light-1120.png')});
 const plans=calls.filter(call=>call.payload.command==='plan_cleanup').length;assert.equal(plans,1);
 await dialog.getByRole('button',{name:'Approve and run',exact:true}).click();
 await dialog.getByRole('region',{name:'Authentication cleanup verification',exact:true}).waitFor();
 const verification=dialog.getByRole('region',{name:'Authentication cleanup verification',exact:true});
 await verification.getByText('Authentication is still available · Injected OAuth token',{exact:true}).waitFor();
 assert.equal(calls.filter(call=>call.payload.command==='execute').length,1);
 const root=cores.get(null).root;
 assert.deepEqual(JSON.parse(await readFile(path.join(root,'.credentials.json'),'utf8')),{mcpOAuth:{synthetic:secrets[3]}});
 assert.deepEqual(JSON.parse(await readFile(path.join(root,'settings.json'),'utf8')),{env:{KEEP_ME:'yes'},mcpServers:{preserve:{command:'inert'}}});
 assert.equal(await readFile(path.join(cores.get(null).home,'.zshrc'),'utf8'),"# Synthetic startup only\nexport KEEP_ME='unchanged'\n");
 assert.equal(await readFile(path.join(root,'CLAUDE.md'),'utf8'),'Synthetic work remains byte-for-byte.\n');
 await verification.getByRole('button',{name:'Recheck local sources',exact:true}).click();await verification.getByText('Fresh verification',{exact:true}).waitFor();
 assert.equal(calls.findLast(call=>call.payload.command==='auth_verify').payload.observe_runtime,false);
 assert.equal(calls.filter(call=>call.payload.command==='execute').length,1);
 await verification.getByRole('button',{name:'Also check authentication in this process',exact:true}).click();await verification.getByText('Authentication is still available · Injected OAuth token',{exact:true}).waitFor();
 assert.equal(calls.findLast(call=>call.payload.command==='auth_verify').payload.observe_runtime,true);
 assert.equal(calls.filter(call=>call.payload.command==='execute').length,1);
 await verification.getByRole('button',{name:'Show original task verification',exact:true}).click();await verification.getByText('Original task record',{exact:true}).waitFor();
 assert.equal(calls.filter(call=>call.payload.command==='auth_verify').length,2);
 const storage=await page.evaluate(()=>JSON.stringify(localStorage));for(const id of planCall.payload.auth_source_ids)assert.equal(storage.includes(id),false);
 report.checks.push('explicit selection, one exact plan and execution; bilingual switch preserves choices; no credential values or source IDs persisted; MCP/work and neighboring shell fields retained');
 report.checks.push('fresh post-cleanup sources and still-authenticated runner are separate; static/runtime recheck retains original receipt and never executes again');
 for(const [width,height,theme] of [[1120,820,'light'],[900,720,'dark']]){await page.setViewportSize({width,height});await page.evaluate(theme=>document.documentElement.dataset.theme=theme,theme);await verification.evaluate(element=>element.scrollIntoView({block:'start'}));assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);assert.equal(await verification.evaluate(element=>element.scrollWidth>element.clientWidth),false);await page.screenshot({path:path.join(artifacts,`auth-result-en-${theme}-${width}.png`)});}
 await dialog.getByRole('button',{name:'Close panel',exact:true}).click();await page.getByRole('combobox',{name:'Interface language',exact:true}).selectOption('zh-CN');
 async function selectHost(alias){await page.locator('.host-switch').click();await page.getByRole('dialog').locator('.host-row').filter({hasText:alias}).getByRole('button',{name:'连接并管理',exact:true}).click();await page.locator('.host-switch').getByText(alias,{exact:true}).waitFor();await page.getByRole('button',{name:'清理与重建',exact:true}).click();}
 await selectHost(aliases[0]);inventory=page.getByRole('region',{name:'认证从哪里来',exact:true});await inventory.locator('[data-auth-kind=credential_field]').waitFor();
 held={};held.promise=new Promise(resolve=>held.release=resolve);await inventory.getByRole('button',{name:'刷新范围',exact:true}).click();await wait(()=>requested);
 await selectHost(aliases[1]);await inventory.locator('[data-auth-kind=credential_field]').waitFor();held.release();held=null;await page.waitForTimeout(100);
 const finalText=await inventory.innerText();assert.ok(finalText.includes(cores.get(aliases[1]).root));assert.equal(finalText.includes(cores.get(aliases[0]).root),false);
 assert.equal(await inventory.locator('input[type=checkbox]:checked').count(),1);assert.equal(await page.getByRole('checkbox',{name:'已关闭目标的终端、IDE 与自动重启来源',exact:false}).isChecked(),false);
 await page.screenshot({path:path.join(artifacts,'auth-inventory-zh-dark-900.png')});
 for(const secret of [...secrets,'SYNTHETIC_INHERITED_DO_NOT_PUBLISH'])assert.equal((await page.locator('body').innerText()).includes(secret),false);
 assert.deepEqual(errors,[]);report.checks.push('late A inventory cannot enter B; confirmations/selection isolated; 1120/900 Day/Night no overflow or runtime error');report.passed=true;console.log('PASS: authentication provenance, exact selection, verified cleanup and host-isolation built-App journey');
} catch(error){report.error=error.stack;report.calls=calls;report.errors=errors;report.body=await page?.locator('body').innerText().catch(()=>null);await page?.screenshot({path:path.join(artifacts,'failure.png')}).catch(()=>{});throw error;}
finally{held?.release();await browser?.close();if(preview.exitCode===null){const ended=new Promise(resolve=>preview.once('exit',resolve));preview.kill('SIGTERM');await ended;}await writeFile(process.env.LINTEL_AUTH_SOURCES_UI_REPORT||path.join(os.tmpdir(),'lintel-auth-sources-ui.json'),JSON.stringify(report,null,2));}
async function wait(check){const deadline=Date.now()+15000;while(Date.now()<deadline){if(await check())return;await new Promise(resolve=>setTimeout(resolve,25));}throw new Error('boundary not observed');}
