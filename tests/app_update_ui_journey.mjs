// Built App interface with synthetic invoke/update replies. Official Rust
// updater/signature/install evidence is independent; this never updates an App.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdir,writeFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {stripVTControlCharacters} from 'node:util';
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'),desktop=path.join(repo,'apps/desktop');
const {chromium}=await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE||path.join(repo,'extensions/browser/node_modules/playwright/index.mjs')).href);
const calls=[],report={fixture:'synthetic updater/IPC; no public requests or real installation',checks:[],passed:false};
let configured=false,offline=false,badSignature=false,proxyActive=false,installationCount=0;
let state={current_version:'0.1.0',channel:null,configured:false,phase:'unconfigured',background_check:false,checked_at:null,failure:null,candidate:null,downloaded_bytes:0,total_bytes:null,last_install:null,installation_unresolved:false};
const ok=data=>({ok:true,data:structuredClone(data)}),error=(code,message)=>({ok:false,error:{code,message}});
async function invoke(command,{payload:p}){
 calls.push({command,...p});
 if(command==='request'){
  if(p.command==='discover')return ok({environments:[],capabilities:[]});
  if(p.command==='jobs')return ok({jobs:[]});
  throw Error('Unexpected synthetic core command '+p.command);
 }
 assert.equal(command,'app_update_request');
 if(p.op==='status')return ok(state);
 if(p.op==='preference'){state.background_check=p.background_check;return ok(state);}
 if(!configured)return error('updater_unconfigured','此构建尚未配置发行公钥与更新 feed');
 if(p.op==='check'){
  if(offline){state.phase='error';state.failure='更新检查未完成，当前是否最新尚未确认';return error('update_check_failed',state.failure);}
  state={...state,phase:'available',checked_at:1,failure:null,candidate:{id:'synthetic-frozen-candidate',version:'0.2.0',notes:'Synthetic notes with <script>plain text</script> and a long version explanation.',url:'https://github.com/IndelibleVivi/lintel-cc/releases/download/v0.2.0/Lintel_0.2.0_aarch64.app.tar.gz',signature_verified:false}};return ok(state);
 }
 if(p.op==='download'){
  assert.equal(p.candidate_id,state.candidate.id);
  if(badSignature){state.phase='error';state.failure='下载或签名／版本核验未完成';return error('update_download_failed',state.failure);}
  state.phase='verified';state.candidate.signature_verified=true;state.failure=null;return ok(state);
 }
 if(p.op==='install'){
  assert.equal(p.candidate_id,state.candidate.id);assert.equal(state.candidate.signature_verified,true);
  if(proxyActive)return error('proxy_active','请先停止本 App 的所有受控通道，再安装或重启');
  assert.equal(++installationCount,1,'Repeated install');state.phase='installed';state.last_install={id:p.candidate_id,from_version:'0.1.0',to_version:'0.2.0',status:'installed'};return ok(state);
 }
 if(p.op==='restart'){assert.equal(p.candidate_id,state.last_install.id);return error('synthetic_restart','Synthetic journey does not restart or replace a real App');}
 throw Error('Unexpected update action '+p.op);
}
const preview=spawn(process.execPath,[path.join(desktop,'node_modules/vite/bin/vite.js'),'preview','--host','127.0.0.1','--port','0','--strictPort'],{cwd:desktop,stdio:['ignore','pipe','pipe']});let browser;
try{
 const url=await new Promise((resolve,reject)=>{let output='';const timer=setTimeout(()=>reject(Error('Preview timeout '+output)),15000);preview.once('error',reject);for(const s of [preview.stdout,preview.stderr])s.on('data',c=>{output+=c;const m=stripVTControlCharacters(output).match(/http:\/\/127\.0\.0\.1:\d+\//);if(m){clearTimeout(timer);resolve(m[0]);}});});
 browser=await chromium.launch({headless:true});const page=await browser.newPage({viewport:{width:1120,height:760}});page.setDefaultTimeout(10000);const errors=[],outside=[];
 page.on('pageerror',e=>errors.push(e.message));page.on('request',r=>{if(!r.url().startsWith(url))outside.push(r.url());});
 await page.exposeFunction('syntheticInvoke',invoke);await page.addInitScript(()=>{window.isTauri=true;window.__TAURI_INTERNALS__={invoke:(cmd,args)=>window.syntheticInvoke(cmd,args)};});
 await page.goto(url);await page.getByRole('button',{name:/本地工作空间/}).click();await page.getByRole('button',{name:'Lintel 版本与更新'}).click();
 const dialog=page.getByRole('dialog',{name:'Lintel 更新',exact:true});await dialog.getByText('此构建尚未配置更新发行',{exact:true}).waitFor();
 assert.equal(calls.some(c=>c.op==='check'),false,'Default startup contacted feed');
 await dialog.getByRole('button',{name:'检查更新',exact:true}).click();await dialog.getByText(/updater_unconfigured/).waitFor();
 report.checks.push('visible unconfigured version; no automatic request; manual failure honest');
 configured=true;state={...state,configured:true,channel:'preview',phase:'idle'};await page.reload();
 await page.getByRole('button',{name:/本地工作空间/}).click();await page.getByRole('button',{name:'Lintel 版本与更新'}).click();
 await dialog.getByText('尚未检查',{exact:true}).waitFor();offline=true;await dialog.getByRole('button',{name:'检查更新',exact:true}).click();await dialog.getByText(/update_check_failed/).waitFor();
 assert.equal(await dialog.getByText('本次检查没有发现更高版本',{exact:true}).count(),0);
 offline=false;await dialog.getByRole('button',{name:'检查更新',exact:true}).click();await dialog.getByRole('heading',{name:'Lintel 0.2.0',exact:true}).waitFor();
 assert.match(await dialog.locator('.update-notes').textContent(),/<script>plain text<\/script>/);
 assert.equal(await dialog.locator('script').count(),0);assert.equal(calls.some(c=>c.op==='install'),false);
 badSignature=true;await dialog.getByRole('button',{name:'下载并核验',exact:true}).click();await dialog.getByText(/update_download_failed/).waitFor();
 assert.equal(await dialog.getByRole('button',{name:'安装这份已核验更新',exact:true}).count(),0);
 badSignature=false;await dialog.getByRole('button',{name:'下载并核验',exact:true}).click();await dialog.getByText('签名与版本核验通过',{exact:true}).waitFor();
 const install=dialog.getByRole('button',{name:'安装这份已核验更新',exact:true});assert.equal(await install.isEnabled(),false);
 await dialog.getByRole('checkbox',{name:'我已审阅此版本，准备安装',exact:true}).check();proxyActive=true;await install.click();await dialog.getByText(/proxy_active/).waitFor();assert.equal(installationCount,0);
 report.checks.push('offline not latest; notes plain text; signature error prevents install; explicit approval and proxy rejection');
 const artifacts=process.env.LINTEL_UPDATE_UI_ARTIFACTS;if(artifacts){await mkdir(artifacts,{recursive:true});await page.screenshot({path:path.join(artifacts,'update-day-review.png')});}
 proxyActive=false;await install.click();await dialog.getByText('安装完成，重启后运行新版本',{exact:true}).waitFor();
 assert.equal(calls.filter(c=>c.op==='restart').length,0,'Installed automatically restarted');
 await dialog.getByRole('button',{name:'保存好工作后重启 Lintel',exact:true}).click();await dialog.getByText(/synthetic_restart/).waitFor();
 assert.equal(installationCount,1);await dialog.getByRole('button',{name:'完成',exact:true}).click();
 await page.getByRole('button',{name:'关闭面板',exact:true}).click();await page.getByRole('button',{name:'深色 Night',exact:true}).click();await page.setViewportSize({width:900,height:640});
 state={...state,phase:'idle',candidate:null,last_install:null};await page.reload();
 await page.getByRole('button',{name:/本地工作空间/}).click();await page.getByRole('button',{name:'Lintel 版本与更新'}).click();await dialog.getByText('尚未检查',{exact:true}).waitFor();
 await page.clock.install();const before=calls.filter(c=>c.op==='check').length;
 await page.clock.fastForward(3600000);assert.equal(calls.filter(c=>c.op==='check').length,before);
 await dialog.getByRole('checkbox',{name:/在 App 打开且可见时后台检查/}).click();await dialog.getByText('发现新版本',{exact:true}).waitFor();
 const count=calls.filter(c=>c.op==='check').length;assert.equal(count,before+1);
 await page.clock.fastForward(60000);assert.equal(calls.filter(c=>c.op==='check').length,count,'Background checks exceeded hourly rate');
 await dialog.getByRole('button',{name:'完成',exact:true}).click();await page.clock.fastForward(3600000);
 await assertEventually(()=>assert.equal(calls.filter(c=>c.op==='check').length,count+1));
 await page.getByRole('button',{name:'Lintel 版本与更新'}).click();await dialog.getByRole('checkbox',{name:/在 App 打开且可见时后台检查/}).click();
 await assertEventually(()=>assert.equal(state.background_check,false));await page.clock.fastForward(3600000);assert.equal(calls.filter(c=>c.op==='check').length,count+1);
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);assert.equal(await dialog.evaluate(d=>d.scrollWidth>d.clientWidth),false);
 if(artifacts)await page.screenshot({path:path.join(artifacts,'update-night-status.png')});
 report.checks.push('install separate from explicit restart; original ID retained; Day/Night 1120/900; background offdefault, opt-in hourly, survives Settings close, stops on opt-out');
 assert.deepEqual(errors,[]);assert.deepEqual(outside,[]);report.passed=true;console.log('PASS: built App updater control, errors, approval, original ID and opt-in lifecycle');
}catch(e){report.error=e.message;throw e;}finally{await browser?.close();if(preview.exitCode===null){const end=new Promise(r=>preview.once('exit',r));preview.kill('SIGTERM');await end;}await writeFile(process.env.LINTEL_UPDATE_UI_REPORT||path.join(tmpdir(),'lintel-app-update-ui.json'),JSON.stringify(report,null,2));}
async function assertEventually(assertion){for(let i=0;i<100;i++){try{assertion();return;}catch(error){if(i===99)throw error;await new Promise(r=>setTimeout(r,20));}}}
