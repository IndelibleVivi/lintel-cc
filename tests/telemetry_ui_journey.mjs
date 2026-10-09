// Built App journey for the telemetry-destination control surface. The catalog is
// the real embedded contract; the invoke bridge, settings and the native rule
// test are explicitly synthetic. No public host, DNS, upstream or system setting
// is touched. Uses the same isolated Chromium + synthetic invoke style as the
// existing network journey; never a personal profile.
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {spawn} from 'node:child_process';
import {writeFile,readFile,mkdir} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {stripVTControlCharacters} from 'node:util';
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const desktop=path.join(repo,'apps/desktop');
const {chromium}=await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE || path.join(repo,'extensions/browser/node_modules/playwright/index.mjs')).href);

const environment={id:randomUUID(),name:'Synthetic telemetry',host:'local',surface:'claude-code',root:'/synthetic/tel',executable:null,ownership:'registered',status:'discovered'};
const calls=[];const channels=new Map();let instance=0;let failRuleTest=false;
const ok=data=>({ok:true,data});
// The real static catalog, echoed by the synthetic bridge.
const catalog=JSON.parse(await readFile(path.join(repo,'contracts/telemetry-destinations.json'),'utf8'));

// Observed settings readback the App renders. The nonempty total switch is shown
// with its documented update impact; fine-grained switches are separate.
const settings=[
 {key:'CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC',label:'非必要流量总开关',value:'0',source:'user_settings',effect_timing:'next_launch',status:'configured'},
 {key:'DISABLE_TELEMETRY',label:'产品指标',value:'',source:'user_settings',effect_timing:'next_launch',status:'unchanged'},
 {key:'DISABLE_ERROR_REPORTING',label:'错误回报',value:null,source:'user_settings',effect_timing:'next_launch',status:'unchanged'}];

async function core(p){
 switch(p.command){
  case 'discover':return {environments:[environment],capabilities:[]};
  case 'jobs':return {jobs:[]};
  case 'inspect':return {environment,settings,assets:[],warnings:[]};
  case 'telemetry_catalog':return catalog;
  default:throw new Error('unexpected core operation: '+p.command);
 }
}
async function invoke(command,args){
 const p=args.payload;calls.push({transport:command,...structuredClone(p)});
 if(command==='app_update_request'){assert.equal(p.op,'status');return ok({current_version:'0.1.0',configured:false,channel:null,phase:'unconfigured',background_check:false,candidate:null,last_install:null});}
 if(command==='network_request'){
  if(p.op==='rule_test'){
   if(failRuleTest)return {ok:false,error:{code:'rule_test_failed',message:'Synthetic test deadline reached'}};
   const ch=channels.get(p.environment_id);assert.ok(ch&&ch.running,'rule_test without active channel');
   const host=catalog.destinations.find(e=>e.blockable&&e.id===p.telemetry_id)?.host;
   if(!host)return {ok:false,error:{code:'rule_test_failed',message:'unknown_telemetry_destination'}};
   if(p.channel_binding!==ch.channel_binding)return {ok:false,error:{code:'channel_changed',message:'channel replaced'}};
   const blocked=(ch.active_config.blocked??[]).some(r=>r.host===host&&(!r.ports.length||r.ports.includes(443)));
   const testId='rt-synthetic-'+calls.length;
   if(!blocked)return {ok:false,error:{code:'rule_test_failed',message:'telemetry_target_not_explicitly_blocked'}};
   // The synthetic outcome is `blocked_explicit` only when the exact target is
   // blocked in the current channel; a broken proxy would be `failed`.
   ch.events.push({timestamp_unix_ms:Date.now(),destination_host:host,destination_port:443,decision:'deny',provenance:'explicit_block',outcome:'blocked',origin:'rule_test',test_id:testId});
   return ok({kind:'rule_test',test_id:testId,owner_origin:'lintel_app_proxy',telemetry_id:p.telemetry_id,environment_id:p.environment_id,channel_binding:ch.channel_binding,outcome:{kind:'rule_test',provenance:'rule_test',destination_host:host,destination_port:443,result:'blocked_explicit',decision:'deny',reason:'explicit_block',explicit_block:true,connection_attempted:false,test_id:testId,origin:'rule_test',coverage:'proxy_connections_only'}});
  }
  if(p.op==='stop'){channels.delete(p.environment_id);return ok({running:false,address:null,active_config:null,events:[],coverage:'synthetic',direct_connections_enforced:false});}
  if(p.op==='start'){
   assert.ok(!channels.get(p.environment_id),'duplicate start');
   channels.set(p.environment_id,{running:true,address:'127.0.0.1:55199',channel_binding:'synthetic-instance-'+(++instance),active_config:{...p.config,environment_id:p.environment_id,bind:'127.0.0.1:55199',max_connections:64,connect_timeout_seconds:10,connection_lifetime_seconds:300},events:[],coverage:'synthetic',direct_connections_enforced:false});
  }
  return ok(channels.get(p.environment_id)??{running:false,address:null,active_config:null,events:[],coverage:'synthetic',direct_connections_enforced:false});
 }
 assert.equal(command,'request');return ok(await core(p));
}
const report={fixture:'real embedded telemetry catalog; synthetic invoke, settings and native rule test',runtime:'built App in isolated Chromium; not native WebKit',checks:[],passed:false};
const preview=spawn(process.execPath,[path.join(desktop,'node_modules/vite/bin/vite.js'),'preview','--host','127.0.0.1','--port','0','--strictPort'],{cwd:desktop,stdio:['ignore','pipe','pipe']});
let browser;
try{
 const url=await new Promise((resolve,reject)=>{let output='';const timer=setTimeout(()=>reject(new Error('preview timeout '+output)),15000);preview.once('error',reject);for(const stream of [preview.stdout,preview.stderr])stream.on('data',chunk=>{output+=chunk;const match=stripVTControlCharacters(output).match(/http:\/\/127\.0\.0\.1:\d+\//);if(match){clearTimeout(timer);resolve(match[0]);}});});
 browser=await chromium.launch({headless:true});report.browser=browser.version();report.url=url;
 const page=await browser.newPage({viewport:{width:1440,height:900}});page.setDefaultTimeout(10000);
 const errors=[],outside=[];page.on('pageerror',e=>errors.push(e.message));page.on('request',request=>{if(!request.url().startsWith(url))outside.push(request.url());});
 report.page_errors=errors;report.outside_requests=outside;
 await page.exposeFunction('syntheticInvoke',invoke);
 await page.addInitScript(id=>{window.isTauri=true;window.__TEL_ID=id;window.__TAURI_INTERNALS__={invoke:(cmd,args)=>window.syntheticInvoke(cmd,args)};Object.defineProperty(navigator,'clipboard',{value:{writeText:async()=>{}}});},environment.id);
 await page.goto(url);
 await page.getByRole('button',{name:'环境详情',exact:true}).click();
 await page.getByRole('tab',{name:'外发与权限',exact:true}).click();
 const options=page.locator('.network-options');await options.locator('summary').click();
 const panel=page.getByRole('region',{name:'Telemetry 目标目录',exact:true});
 await panel.getByText('http-intake.logs.us5.datadoghq.com:443',{exact:false}).waitFor();
 // Catalog: blockable entries offered; mixed/necessary hosts informational only.
 assert.equal(await panel.getByRole('checkbox').count(),2,'only two blockable rows are selectable');
 assert.ok(await panel.getByText('api.anthropic.com:443',{exact:false}).count()>=1,'mixed host shown informational');
 assert.equal(await panel.getByText('仅信息（混合或必要用途，不提供一键阻止）',{exact:false}).count(),1);
 assert.ok(await panel.getByText(/Bedrock、Google Agent Platform 与 Foundry 不使用/).count()>=1,'log intake applicability shown');
 assert.ok(await panel.getByText(/服务端 rollout gate/).count()>=1,'browser intake gate shown');
 report.checks.push('catalog renders blockable vs informational, provider/rollout applicability, exact hosts');
 // Observed settings: NONESSENTIAL shown with its documented update impact.
 assert.equal(await panel.getByText(/非空即生效/).count(),1,'NONESSENTIAL update impact shown');
 assert.ok(await panel.getByText('CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC',{exact:false}).count()>=1);
 report.checks.push('observed settings readback shows NONESSENTIAL update impact and fine-grained switches separately');
 // Select a blockable destination: merges into the stopped-channel draft only.
 const blockedDraft=page.getByRole('textbox',{name:/^显式阻止规则/});
 await blockedDraft.fill('user.invalid 8443\nhttp-intake.logs.us5.datadoghq.com 80');
 const intake=panel.getByRole('checkbox').first();await intake.scrollIntoViewIfNeeded();await intake.check();await intake.uncheck();
 assert.equal(await blockedDraft.inputValue(),'user.invalid 8443\nhttp-intake.logs.us5.datadoghq.com 80','deselect preserves manually entered rules');
 await intake.check();
 await page.getByRole('button',{name:'启动通道',exact:true}).click();
 await page.getByRole('button',{name:'停止通道',exact:true}).waitFor();
 const started=calls.filter(c=>c.transport==='network_request'&&c.op==='start').at(-1);
 assert.ok(started,'channel start dispatched');
 const blockedHosts=(started.config.blocked??[]).map(r=>r.host);
 assert.ok(blockedHosts.includes('http-intake.logs.us5.datadoghq.com'),'selected telemetry host merged into blocked rules');
 assert.ok(!blockedHosts.includes('api.anthropic.com'),'mixed host never auto-selected');
 assert.deepEqual(started.config.blocked.find(r=>r.host==='user.invalid').ports,[8443]);
 assert.equal(started.config.blocked.filter(r=>r.host==='http-intake.logs.us5.datadoghq.com').length,2,'port 80 must not suppress the selected 443 block');
 report.checks.push('selection merges into stopped-channel draft blocked rules; mixed host never selected; start explicit');
 // The UI exposes a real controlled test on the current running channel. It
 // appears only after the channel has the exact target explicitly blocked.
 const testSection=panel.getByRole('region',{name:'受控规则测试',exact:true});
 await testSection.getByRole('button',{name:'测试该阻止',exact:true}).click();
 await testSection.getByText('已确认显式阻止',{exact:true}).waitFor();
 const testedCall=calls.filter(c=>c.transport==='network_request'&&c.op==='rule_test').at(-1);
 assert.equal(testedCall.telemetry_id,'datadog_logs_intake','UI test used a catalog id only');
 assert.deepEqual(Object.keys(testedCall).filter(k=>!['transport','op','environment_id','telemetry_id','channel_binding'].includes(k)),[],'no extra fields');
 report.checks.push('UI controlled test uses catalog id + environment + original channel binding');
 // The result is a controlled, owner-origin outcome shown separately, distinct
 // from real client traffic and from settings readback.
 const resultRow=testSection.locator('.telemetry-test-result');
 assert.ok(await resultRow.count()>=1,'controlled result shown');
 assert.ok((await resultRow.textContent()).includes('连接尝试 否'),'reports no destination connection');
 report.checks.push('controlled result is separated from observed traffic and settings readback');
 failRuleTest=true;await testSection.getByRole('button',{name:'测试该阻止',exact:true}).click();await resultRow.getByText('未通过',{exact:true}).waitFor();
 assert.ok((await resultRow.textContent()).includes('Synthetic test deadline reached'));
 assert.equal((await resultRow.textContent()).includes('test_id Synthetic'),false,'errors are not fabricated test identities');failRuleTest=false;
 // A non-catalog id is refused by the native op.
 const unknown=await page.evaluate(id=>window.__TAURI_INTERNALS__.invoke('network_request',{payload:{op:'rule_test',environment_id:id,telemetry_id:'api.anthropic.com'}}),environment.id);
 assert.equal(unknown.ok,false,'mixed/non-catalog id refused');
 // Stop + restart the channel: the instance changes, so the old pass must not
 // persist and the UI asks for a fresh test on the new instance.
 const bindingBefore=channels.get(environment.id).channel_binding;
 await page.getByRole('button',{name:'停止通道',exact:true}).click();
 await page.getByRole('button',{name:'启动通道',exact:true}).click();
 await testSection.getByRole('button',{name:'测试该阻止',exact:true}).waitFor();
 assert.notEqual(channels.get(environment.id).channel_binding,bindingBefore,'channel instance replaced');
 assert.equal(await testSection.locator('.telemetry-test-result').count(),0,'stale pass cleared after instance replacement');
 report.checks.push('host/environment/instance reset: a replaced channel cannot show a stale controlled pass');
 const stale=await page.evaluate(p=>window.__TAURI_INTERNALS__.invoke('network_request',{payload:p}),{op:'rule_test',environment_id:environment.id,telemetry_id:'datadog_logs_intake',channel_binding:bindingBefore});
 assert.equal(stale.error.code,'channel_changed');
 await testSection.getByRole('button',{name:'测试该阻止',exact:true}).click();
 await page.getByRole('region',{name:'受控请求记录',exact:true}).waitFor();
 assert.ok(await page.getByText('暂无已记录的客户端连接。',{exact:true}).count());
 const artifacts=process.env.LINTEL_TELEMETRY_UI_ARTIFACTS;
 if(artifacts){await mkdir(artifacts,{recursive:true});await panel.scrollIntoViewIfNeeded();await page.screenshot({path:path.join(artifacts,'telemetry-day.png')});}
 await page.setViewportSize({width:900,height:760});
 await page.getByRole('button',{name:/本地工作空间/}).click();
 await page.getByRole('dialog',{name:'设置与模块',exact:true}).getByRole('button',{name:'深色 Night',exact:true}).click();
 await page.getByRole('button',{name:'关闭面板',exact:true}).click();
 await panel.scrollIntoViewIfNeeded();
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'no horizontal document overflow');
 if(artifacts)await page.screenshot({path:path.join(artifacts,'telemetry-night.png')});
 report.checks.push('original binding rejects replacement; controlled events have their own record section; 900 Night layout');
 assert.deepEqual(errors,[]);assert.deepEqual(outside,[]);report.passed=true;console.log('PASS: built App telemetry catalog, draft merge, observed-settings and controlled-test journey');
}catch(e){report.error=e.message;throw e;}finally{
 await browser?.close();if(preview.exitCode===null){const end=new Promise(resolve=>preview.once('exit',resolve));preview.kill('SIGTERM');await end;}
 await writeFile(process.env.LINTEL_TELEMETRY_UI_REPORT||path.join(os.tmpdir(),'lintel-telemetry-ui.json'),JSON.stringify(report,null,2));
}
