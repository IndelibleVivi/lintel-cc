// Synthetic profiles only. Set PLAYWRIGHT_MODULE to an installed playwright index.mjs,
// or install the development dependency. Never uses a personal browser profile.
import assert from 'node:assert/strict';import http from 'node:http';import {mkdtemp,rm,mkdir,writeFile} from 'node:fs/promises';import os from 'node:os';import path from 'node:path';import {fileURLToPath,pathToFileURL} from 'node:url';import {execFileSync} from 'node:child_process';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const {chromium}=await import(process.env.PLAYWRIGHT_MODULE?pathToFileURL(process.env.PLAYWRIGHT_MODULE).href:'playwright');
execFileSync(process.execPath,[path.join(root,'scripts/build.mjs'),'--fixture']);
await mkdir(path.join(root,'artifacts'),{recursive:true});
const profile=await mkdtemp(path.join(os.tmpdir(),'lintel-synthetic-browser-'));
const ext=path.join(root,'dist/chromium-fixture');
const server=http.createServer((req,res)=>{res.setHeader('Cache-Control','no-store');if(req.url==='/sw.js'){res.setHeader('Content-Type','text/javascript');res.end("self.addEventListener('install',()=>self.skipWaiting());self.addEventListener('activate',e=>e.waitUntil(self.clients.claim()));");}else{res.setHeader('Content-Type','text/html');res.end('<!doctype html><title>Synthetic storage fixture</title><h1>Synthetic storage fixture</h1>');}});
await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(18765,'127.0.0.1',resolve);});
let context;const launch=()=>chromium.launchPersistentContext(profile,{headless:true,channel:'chromium',args:[`--disable-extensions-except=${ext}`,`--load-extension=${ext}`]});
async function bounded(promise){let timer;try{return await Promise.race([promise,new Promise((_,reject)=>{timer=setTimeout(()=>reject(new Error('Synthetic browser stage exceeded 30 seconds')),30000);})]);}finally{clearTimeout(timer);}}
async function seed(page){await bounded(page.evaluate(async()=>{localStorage.setItem('lintel-fixture','SYNTHETIC');document.cookie='lintel_fixture=SYNTHETIC; Path=/; SameSite=Lax; Max-Age=3600';await new Promise((resolve,reject)=>{const q=indexedDB.open('lintel-fixture',1);q.onupgradeneeded=()=>q.result.createObjectStore('data');q.onsuccess=()=>{const db=q.result,tx=db.transaction('data','readwrite');tx.objectStore('data').put('SYNTHETIC','key');tx.oncomplete=()=>{db.close();resolve();};};q.onerror=()=>reject(q.error);});const c=await caches.open('lintel-fixture');await c.put('/cached',new Response('SYNTHETIC'));await navigator.serviceWorker.register('/sw.js');await navigator.serviceWorker.ready;}));}
async function observe(page){return bounded(page.evaluate(async()=>({localStorage:localStorage.getItem('lintel-fixture'),cookie:document.cookie.includes('lintel_fixture='),indexedDB:(await indexedDB.databases()).map(v=>v.name),cacheStorage:await caches.keys(),serviceWorkers:(await navigator.serviceWorker.getRegistrations()).length})));}
try{
 console.log('launch synthetic Chromium');context=await launch();let worker=context.serviceWorkers()[0]||await context.waitForEvent('serviceworker');const extensionId=new URL(worker.url()).host;
 const target=await context.newPage(),neighbor=await context.newPage();await target.goto('http://localhost:18765');await neighbor.goto('http://127.0.0.1:18765');await seed(target);await seed(neighbor);assert.equal((await observe(target)).serviceWorkers,1);const beforeNeighbor=await observe(neighbor);console.log('fixtures seeded');
 const popup=await context.newPage();await popup.goto(`chrome-extension://${extensionId}/popup.html`);
 await popup.getByRole('button',{name:'预览站点数据清理',exact:true}).click();await popup.waitForFunction(()=>!document.getElementById('preview-panel').hidden || document.getElementById('error').textContent);assert.equal(await popup.locator('#error').textContent(),'');await popup.getByLabel('我确认以上范围与功能影响').check();await popup.getByRole('button',{name:'授予必要权限并执行一次'}).click();
 await popup.locator('#receipts').getByText('clear · completed',{exact:true}).waitFor({timeout:30000});assert(target.isClosed());assert.deepEqual(await observe(neighbor),beforeNeighbor);
 let receipts=await worker.evaluate(async()=>Object.values(await chrome.storage.local.get(null)).filter(v=>v?.action?.kind==='clear'));const record=receipts[0];assert.equal(record.result.cookieObservation.remaining,0);assert.equal(record.phase,'completed');
 const blocked=await context.newPage();await assert.rejects(blocked.goto('http://localhost:18765'),/ERR_BLOCKED_BY_CLIENT/);await blocked.close();
 await popup.getByRole('button',{name:'已核对结果，解除站点隔离'}).click();
 const clean=await context.newPage();await clean.goto('http://localhost:18765');const after=await observe(clean);assert.deepEqual(after,{localStorage:null,cookie:false,indexedDB:[],cacheStorage:[],serviceWorkers:0});console.log('target cleared; neighbor preserved; navigation isolation verified');
 await seed(clean); // new login/storage after clear; duplicate commit must not remove it
 await context.close();console.log('restart synthetic browser');context=await launch();worker=context.serviceWorkers()[0]||await context.waitForEvent('serviceworker');
 const page=await context.newPage();await page.goto(`chrome-extension://${extensionId}/popup.html`);const duplicate=await page.evaluate(id=>chrome.runtime.sendMessage({type:'commit',id}),record.id);assert.equal(duplicate.data.phase,'completed');
 const newLogin=await context.newPage();await newLogin.goto('http://localhost:18765');assert.equal((await observe(newLogin)).cookie,true);

 const screenshot='not-run (functional storage test)';
 const report={screenshot,browser:context.browser()?.version()||'Chromium persistent context',platform:`${os.platform()} ${os.arch()}`,fixture:'localhost target, 127.0.0.1 neighbor; independent temp profile',checks:['real popup preview-confirm-execute','Cookie/localStorage/IndexedDB/ServiceWorker/CacheStorage deleted','neighbor stores preserved','target tabs closed','cookie count verified without values in receipt','durable result after full browser/worker restart','new synthetic login preserved on duplicate operation ID'],receipt:record};
 await writeFile(path.join(root,'artifacts/browser-smoke.json'),JSON.stringify(report,null,2));console.log(JSON.stringify({ok:true,checks:report.checks},null,2));
}finally{await context?.close();await new Promise(r=>server.close(r));await rm(profile,{recursive:true,force:true});}
