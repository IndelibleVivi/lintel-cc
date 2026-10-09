// Built App journey: host configuration, public echo results, invoke and SSH
// are explicitly synthetic. No real public endpoint or system setting changes.
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {spawn} from 'node:child_process';
import {mkdir,writeFile} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {stripVTControlCharacters} from 'node:util';
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const desktop=path.join(repo,'apps/desktop');
const {chromium}=await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE || path.join(repo,'extensions/browser/node_modules/playwright/index.mjs')).href);
const environments=[{id:randomUUID(),name:'Synthetic writing A',host:'local',surface:'claude-code',root:'/synthetic/a',executable:null,ownership:'registered',status:'discovered'},{id:randomUUID(),name:'Synthetic writing B',host:'local',surface:'claude-code',root:'/synthetic/b',executable:null,ownership:'registered',status:'discovered'}];
const originalConfig={set_id:'synthetic-set',service_uuid:'synthetic-service',enabled:true,configuration:{ConfigMethod:'Manual',Addresses:['2001:db8::10'],PrefixLength:[64],Router:'2001:db8::1'}};
let config=structuredClone(originalConfig),revision='synthetic-revision-1',nonce=0,conflict=false,loseAck=true,empty=false;
const channels=new Map(),plans=new Map(),probes=new Map(),receipts=[],calls=[];
const alias='synthetic-linux';
const ok=data=>({ok:true,data});
function probe(p,remote=false,after=false){
 const cell=(path,family)=>({path,family,status:path==='lintel_channel'&&!p.proxy_url?'not_tested':after&&family==='ipv6'?'no_route':'ok',public_ip:path==='lintel_channel'&&!p.proxy_url||after&&family==='ipv6'?null:family==='ipv4'?'203.0.113.7':'2001:db8::7',elapsed_ms:17,peer_family:path==='lintel_channel'&&!p.proxy_url||after&&family==='ipv6'?null:path==='lintel_channel'?'ipv4':family,message:after&&family==='ipv6'?'Synthetic no IPv6 route':null});
 const value={schema:'lintel.network-probe/1',id:randomUUID(),executed_at:new Date().toISOString(),platform:remote?'linux':'macos',execution_host:remote?'synthetic-linux-node':'synthetic-mac-node',network_revision:revision,endpoints:{ipv4:p.ipv4_url,ipv6:p.ipv6_url},proxy_url:p.proxy_url??null,proxy_binding:p.proxy_url?channels.values().next().value?.channel_binding??null:null,timeout_seconds:p.timeout_seconds??10,cells:['host_default','lintel_channel'].flatMap(path=>['ipv4','ipv6'].map(family=>cell(path,family)))};
 probes.set(value.id,value);return value;
}
function makePlan(p,restore=false){
 const spec=restore?plans.get(receipts[0].plan_id).network.probe:p.probe;
 const reusable=probes.get(p.baseline_id);
 const before=reusable?.network_revision===revision?reusable:probe(spec);
 const after=restore?originalConfig:{...structuredClone(config),enabled:false};
 const value={id:randomUUID(),hash:'a'.repeat(64),kind:restore?'network_restore':'network_ipv6',title:restore?'恢复原 IPv6 配置':'关闭该网络服务的 IPv6',environment_id:null,changes:[],preserves:['IPv4、DNS、系统代理与其他服务'],warnings:['Synthetic host shared change; no real network operation'],actions:[{id:'network_write',label:'完整写入并读回 IPv6 配置',reversible:!restore},{id:'network_reprobe',label:'同目标与路径自动复测',reversible:false}],created_at:new Date().toISOString(),status:'planned',network:{scope:'host_shared',service_id:'synthetic-set:synthetic-service',service_name:'Synthetic Wi-Fi',interface:'en0',service_enabled:true,before:structuredClone(config),after:structuredClone(after),probe:spec,before_probe:before}};
 plans.set(value.id,value);return value;
}
async function core(p,remote=false){
 switch(p.command){
  case 'discover':return {environments:empty?[]:environments.map(e=>remote?{...e,host:alias}:e),capabilities:[]};
  case 'jobs':return {jobs:remote?[]:receipts};
  case 'inspect':return {environment:environments.find(e=>e.id===p.environment_id),settings:[],assets:[],warnings:[]};
  case 'drift':return {status:'unchanged',changes:[]};
  case 'network_inspect':return {schema:'lintel.network/1',platform:remote?'linux':'macos',execution_host:remote?'synthetic-linux-node':'synthetic-mac-node',network_revision:revision,network_revision_complete:true,interfaces:[{interface:'utun4',flags:1,up:true,ipv4_addresses:[],ipv6_addresses:['fe80::4']}],services:[{service_id:'synthetic-set:synthetic-service',name:remote?'Synthetic eth0':'Synthetic Wi-Fi',interface:remote?'eth0':'en0',mode:config.enabled?'manual':'off',enabled:true,ipv4_addresses:['192.0.2.10'],ipv6_addresses:config.enabled?['2001:db8::10']:[]}],limitations:[remote?'Synthetic Linux readonly; no system mutation':'Synthetic configuration and invoke; no real host change']};
  case 'network_probe':return probe(p,remote);
  case 'plan_network_ipv6':assert.equal(remote,false);assert.equal(p.service_id,'synthetic-set:synthetic-service');return makePlan(p);
  case 'plan_network_restore':
   assert.equal(p.job_id,receipts[0].id);
   if(conflict)throw new Error('Synthetic external IPv6 edit; later configuration preserved');
   return makePlan(p,true);
  case 'execute':{
   const plan=plans.get(p.plan_id);assert.ok(plan);assert.equal(p.approval,plan.hash);assert.ok(!receipts.some(r=>r.id===p.plan_id),'mutation replay');
   config=structuredClone(plan.network.after);revision='synthetic-revision-'+(receipts.length+2);
   const receipt={id:plan.id,plan_id:plan.id,environment_id:null,title:plan.title,status:'completed',created_at:new Date().toISOString(),restorable:plan.kind==='network_ipv6',warnings:[],network_change:{...plan.network,configuration_verified:true},before_probe:plan.network.before_probe,after_probe:probe(plan.network.probe,false,plan.kind==='network_ipv6'),steps:[{id:'network_write',label:'IPv6 配置读回',status:'completed',message:'Synthetic full configuration matches'},{id:'network_reprobe',label:'自动复测',status:'completed',message:'Synthetic outcomes remain independent'}]};
   receipts.unshift(receipt);
   if(loseAck){loseAck=false;throw new Error('Synthetic ACK lost after durable write');}
   return receipt;
  }
  case 'job':{const receipt=receipts.find(r=>r.id===p.job_id);if(!receipt)throw new Error('Synthetic job absent');return receipt;}
  default:throw new Error('unexpected core operation: '+p.command);
 }
}
async function invoke(command,args){
 const p=args.payload;calls.push({transport:command,...structuredClone(p)});
 if(command==='network_request'){
  if(p.op==='stop'){channels.delete(p.environment_id);return ok({running:false,address:null,active_config:null,events:[],coverage:'synthetic',direct_connections_enforced:false});}
  if(p.op==='start')channels.set(p.environment_id,{running:true,address:'127.0.0.1:55123',channel_binding:'synthetic-instance-'+(++nonce),active_config:{...p.config,environment_id:p.environment_id,bind:'127.0.0.1:55123',max_connections:64,connect_timeout_seconds:10,connection_lifetime_seconds:300},events:[],coverage:'synthetic',direct_connections_enforced:false});
  return ok(channels.get(p.environment_id)??{running:false,address:null,active_config:null,events:[],coverage:'synthetic',direct_connections_enforced:false});
 }
 if(command==='remote_request'){
  switch(p.op){
   case 'hosts':return ok({hosts:[{alias}],tasks:[],installations:[],launches:[]});
   case 'aliases':return ok({aliases:[alias],coverage:'Synthetic SSH'});
   case 'connect':return ok({status:'connected'});
   case 'request':assert.equal(p.alias,alias);return ok(await core(p.request,true));
   default:throw new Error('unexpected remote operation '+p.op);
  }
 }
 assert.equal(command,'request');return ok(await core(p));
}
const report={fixture:'synthetic IPv6 configuration, echoes, invoke and SSH',runtime:'built App in isolated Chromium; not native WebKit or real SystemConfiguration',checks:[],passed:false};
const preview=spawn(process.execPath,[path.join(desktop,'node_modules/vite/bin/vite.js'),'preview','--host','127.0.0.1','--port','0','--strictPort'],{cwd:desktop,stdio:['ignore','pipe','pipe']});
let browser;
try{
 const url=await new Promise((resolve,reject)=>{let output='';const timer=setTimeout(()=>reject(new Error('preview timeout '+output)),15000);preview.once('error',reject);for(const stream of [preview.stdout,preview.stderr])stream.on('data',chunk=>{output+=chunk;const match=stripVTControlCharacters(output).match(/http:\/\/127\.0\.0\.1:\d+\//);if(match){clearTimeout(timer);resolve(match[0]);}});});
 browser=await chromium.launch({headless:true});report.browser=browser.version();report.url=url;
 const page=await browser.newPage({viewport:{width:1440,height:900}});page.setDefaultTimeout(10000);
 const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.exposeFunction('syntheticInvoke',invoke);
 await page.addInitScript(()=>{window.isTauri=true;window.__TAURI_INTERNALS__={invoke:(cmd,args)=>window.syntheticInvoke(cmd,args)};Object.defineProperty(navigator,'clipboard',{value:{writeText:async text=>{window.syntheticCopied=text;}}});});
 await page.goto(url);await page.getByRole('button',{name:'环境详情',exact:true}).click();await page.getByRole('tab',{name:'外发与权限',exact:true}).click();
 const panel=page.getByRole('region',{name:'IPv4 / IPv6 网络路径',exact:true});
 await panel.getByText('Synthetic Wi-Fi',{exact:true}).waitFor();
 assert.equal(calls.some(c=>c.command==='network_probe'),false,'public probe happened without explicit action');
 await panel.getByText('其他接口观察（含 VPN / TUN）',{exact:true}).click();await panel.getByText('utun4',{exact:true}).waitFor();await panel.getByText('其他接口观察（含 VPN / TUN）',{exact:true}).click();
 await panel.getByRole('button',{name:'测试实际出口',exact:true}).click();await panel.getByRole('table').waitFor();
 assert.equal(await panel.getByText('未测试',{exact:true}).count(),2);await panel.getByText(/synthetic-mac-node/).waitFor();
 await panel.getByRole('button',{name:'复制出口 IP 203.0.113.7',exact:true}).click();assert.equal(await page.evaluate(()=>window.syntheticCopied),'203.0.113.7');
 const baseline=[...probes.values()].at(-1);
 await panel.getByRole('button',{name:'预览关闭 IPv6',exact:true}).click();
 const dialog=page.getByRole('dialog');await dialog.getByRole('heading',{name:'关闭该网络服务的 IPv6',exact:true}).waitFor();
 assert.equal(calls.filter(c=>c.command==='plan_network_ipv6').at(-1).baseline_id,baseline.id);assert.equal(calls.some(c=>c.command==='execute'),false);
 await dialog.getByText('核对完整 IPv6 配置与恢复内容',{exact:true}).click();assert.match(await dialog.locator('pre').first().textContent(),/PrefixLength/);
 await dialog.getByRole('button',{name:'返回调整',exact:true}).focus();await page.keyboard.press('Enter');assert.equal(config.enabled,true);
 report.checks.push('no automatic public probe; four cells and IP copy; full frozen manual configuration; cancel never mutates');
 const options=page.locator('.network-options');await options.locator('summary').click();await options.getByLabel('Lintel 建立的连接',{exact:true}).selectOption('ipv4_only');
 await page.getByRole('button',{name:'启动通道',exact:true}).click();await page.getByText('严格 IPv4',{exact:true}).waitFor();assert.equal(calls.find(c=>c.transport==='network_request'&&c.op==='start').config.address_family,'ipv4_only');
 await panel.getByText(/网络状态、测试目标或通道已变化/).waitFor();
 await panel.getByRole('button',{name:'测试实际出口',exact:true}).click();assert.equal(await panel.getByText('请求成功',{exact:true}).count(),4);
 const oldBinding=[...probes.values()].at(-1).proxy_binding;
 await page.getByRole('button',{name:'停止通道',exact:true}).click();await page.getByRole('button',{name:'启动通道',exact:true}).click();
 assert.notEqual(channels.get(environments[0].id).channel_binding,oldBinding);await panel.getByText(/网络状态、测试目标或通道已变化/).waitFor();
 report.checks.push('strict IPv4 config readback; channel stop/restart invalidates baseline even at the same port');
 await panel.getByRole('button',{name:'预览关闭 IPv6',exact:true}).click();await dialog.getByRole('button',{name:'批准并执行',exact:true}).focus();await page.keyboard.press('Enter');
 await dialog.getByRole('button',{name:'查询原任务',exact:true}).click();await dialog.getByRole('button',{name:'预览恢复',exact:true}).waitFor();
 assert.equal(calls.filter(c=>c.command==='execute').length,1);assert.equal(config.enabled,false);assert.equal(await dialog.getByRole('button',{name:'打开 Claude',exact:true}).count(),0);
 await dialog.getByRole('region',{name:'出口前后对照',exact:true}).waitFor();assert.equal(await dialog.getByText('无可用路由',{exact:true}).count(),2);
 const artifacts=process.env.LINTEL_NETWORK_UI_ARTIFACTS;if(artifacts){await mkdir(artifacts,{recursive:true});await page.screenshot({path:path.join(artifacts,'network-day-receipt.png')});}
 conflict=true;await dialog.getByRole('button',{name:'预览恢复',exact:true}).click();await dialog.getByText(/Synthetic external IPv6 edit/).waitFor();assert.equal(calls.filter(c=>c.command==='execute').length,1);
 report.checks.push('lost ACK queries original job once; verified configuration plus failed IPv6 probe; external restore conflict preserves later edits');
 await dialog.getByRole('button',{name:'关闭面板',exact:true}).click();await page.getByLabel('当前环境',{exact:true}).selectOption(environments[1].id);await panel.getByText('关闭',{exact:true}).waitFor();
 await panel.getByText('共享网络的修改与恢复记录',{exact:true}).click();await panel.getByRole('button',{name:'查看结果与恢复',exact:true}).click();
 conflict=false;await dialog.getByRole('button',{name:'预览恢复',exact:true}).click();await dialog.getByRole('heading',{name:'恢复原 IPv6 配置',exact:true}).waitFor();assert.equal(calls.filter(c=>c.command==='execute').length,1);
 await dialog.getByRole('button',{name:'批准并执行',exact:true}).click();await dialog.getByRole('heading',{name:'执行结果',exact:true}).waitFor();assert.equal(calls.filter(c=>c.command==='execute').length,2);assert.deepEqual(config,originalConfig);
 report.checks.push('shared service survives environment selection; original history remains available; restoration has separate approval and complete manual target');
 await dialog.getByRole('button',{name:'关闭面板',exact:true}).click();await page.getByRole('button',{name:'深色 Night',exact:true}).click();await page.setViewportSize({width:900,height:640});
 await panel.getByRole('button',{name:'测试实际出口',exact:true}).click();await panel.getByRole('table').waitFor();
 assert.equal(await panel.evaluate(el=>el.scrollWidth>el.clientWidth),false);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
 if(artifacts)await page.screenshot({path:path.join(artifacts,'network-night-panel.png')});
 // A shared host operation remains reachable without any Claude environment.
 empty=true;await page.reload();await page.getByRole('button',{name:'查看这台主机的网络',exact:true}).click();await panel.getByRole('button',{name:'测试实际出口',exact:true}).click();await panel.getByRole('table').waitFor();
 report.checks.push('Day/Night 1440/900 layouts; no-environment host access');
 await page.locator('.host-switch').click();await dialog.locator('.host-row').filter({hasText:alias}).getByRole('button',{name:'连接并管理',exact:true}).click();
 await panel.getByText('Synthetic eth0',{exact:true}).waitFor();assert.equal(await panel.getByRole('button',{name:'预览关闭 IPv6',exact:true}).count(),0);
 const start=calls.length;await panel.getByRole('button',{name:'测试实际出口',exact:true}).click();await panel.getByRole('table').waitFor();
 assert.ok(calls.slice(start).some(c=>c.transport==='remote_request'&&c.op==='request'&&c.alias===alias&&c.request.command==='network_probe'));
 assert.ok(!calls.slice(start).some(c=>c.transport==='request'&&c.command==='network_probe'));
 report.checks.push('remote Linux readonly cells execute through selected SSH host; no local probe or system mutation controls');
 assert.deepEqual(errors,[]);report.passed=true;console.log('PASS: built App IPv4/IPv6 probe, shared approval/query/restore, strict channel and remote journey');
}catch(e){report.error=e.message;throw e;}finally{
 await browser?.close();if(preview.exitCode===null){const end=new Promise(resolve=>preview.once('exit',resolve));preview.kill('SIGTERM');await end;}
 await writeFile(process.env.LINTEL_NETWORK_UI_REPORT||path.join(os.tmpdir(),'lintel-network-ui.json'),JSON.stringify(report,null,2));
}
