// Actual production worker/Engine and real popup DOM, with memory browser
// mutation APIs and persistent framed stdio to the real native host.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {readFile} from 'node:fs/promises';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
export async function extensionRecoveryJourney({repo,browser,host,env,control,extensionId,report}) {
  const db={},rules=[],requests=[],processes=new Set();let removeFailure=false,settingWrites=0,clearWrites=0,dropAck=false;
  const event=()=>({addListener(fn){this.fn=fn;}});
  let setting={value:'default',levelOfControl:'controllable_by_this_extension'};
  const api={storage:{local:{get:async k=>k===null?structuredClone(db):{[k]:structuredClone(db[k])},set:async v=>Object.assign(db,structuredClone(v)),remove:async k=>{if(k==='cleanupIsolation'&&removeFailure)throw Error('synthetic owner removal fault');delete db[k];}}},
    permissions:{contains:async()=>true,getAll:async()=>({permissions:[]}),onRemoved:event()},alarms:{create:async()=>{},onAlarm:event()},tabs:{query:async()=>[],remove:async()=>{}},webNavigation:{getAllFrames:async()=>[]},browsingData:{remove:async()=>{clearWrites++;}},
    declarativeNetRequest:{getDynamicRules:async()=>structuredClone(rules),updateDynamicRules:async({addRules=[],removeRuleIds=[]})=>{for(const id of removeRuleIds){const i=rules.findIndex(r=>r.id===id);if(i>=0)rules.splice(i,1);}rules.push(...structuredClone(addRules));}},
    privacy:{network:{webRTCIPHandlingPolicy:{get:async()=>structuredClone(setting),set:async({value})=>{settingWrites++;setting={value,levelOfControl:'controlled_by_this_extension'};}}}}};
  function connectNative(){
    const child=spawn(host,[`chrome-extension://${extensionId}/`],{env,stdio:['pipe','pipe','pipe']});processes.add(child);
    let bytes=Buffer.alloc(0),stderr='';child.stderr.on('data',b=>stderr+=b);
    const port={onMessage:event(),onDisconnect:event(),disconnect(){child.stdin.end();},postMessage(msg){requests.push(structuredClone(msg));const payload=Buffer.from(JSON.stringify(msg)),frame=Buffer.alloc(payload.length+4);frame.writeUInt32LE(payload.length);payload.copy(frame,4);child.stdin.write(frame);}};
    child.stdout.on('data',chunk=>{bytes=Buffer.concat([bytes,chunk]);while(bytes.length>=4&&bytes.length>=4+bytes.readUInt32LE()){const len=bytes.readUInt32LE(),reply=JSON.parse(bytes.subarray(4,4+len));bytes=bytes.subarray(4+len);if(dropAck&&reply.ok&&requests.find(r=>r.request_id===reply.request_id)?.receipt?.phase==='completed'){dropAck=false;port.onMessage.fn({...reply,ok:false,error:{code:'synthetic_lost_ack',message:'Host persisted terminal receipt; reply deliberately withheld'}});}else port.onMessage.fn(reply);}});
    child.on('exit',code=>{processes.delete(child);if(code)throw Error(`synthetic host exit ${code}: ${stderr}`);port.onDisconnect.fn?.();});return port;
  }
  let send;
  async function openWorker(){
    // Simulated worker destruction closes its private native pipe; no startup event.
    for(const child of [...processes]){const exited=new Promise(resolve=>child.once('exit',resolve));child.stdin.end();await exited;}
    api.runtime={id:extensionId,getURL:p=>`chrome-extension://${extensionId}/${p}`,connectNative,onMessage:event(),onStartup:event()};
    const prior=globalThis.browser,interval=globalThis.setInterval;globalThis.browser=api;globalThis.setInterval=()=>0;
    try{await import(pathToFileURL(path.join(repo,'extensions/browser/src/background.js')).href+`?recovery=${crypto.randomUUID()}`);}finally{globalThis.browser=prior;globalThis.setInterval=interval;}
    send=message=>new Promise(resolve=>api.runtime.onMessage.fn(message,{id:extensionId,url:api.runtime.getURL('popup.html')},resolve));await send({type:'status'});
  }
  async function ok(message){const result=await send(message);assert.equal(result.ok,true,JSON.stringify(result.error));return result.data;}
  async function pair(){const p=control({op:'pair_create'}).data;await ok({type:'pair',code:p.code,label:'Synthetic recovery profile'});const pending=control({op:'pair_pending'}).data.find(r=>r.instance_id===db.identity.instanceId);assert(pending);assert.equal(control({op:'pair_approve',challenge:pending.challenge}).ok,true);await ok({type:'refreshPairing'});}
  async function popup(){const page=await browser.newPage();await page.exposeFunction('syntheticPopupSend',message=>send(message));await page.addInitScript(()=>{window.browser={runtime:{sendMessage:message=>window.syntheticPopupSend(message)}};});
    await page.route('http://lintel-extension.test/**',async route=>{const file=new URL(route.request().url()).pathname.slice(1);assert(['popup.html','popup.css','popup.js','config.js','policy.js'].includes(file));await route.fulfill({body:await readFile(path.join(repo,'extensions/browser/src',file)),contentType:file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':'text/html'});});
    await page.goto('http://lintel-extension.test/popup.html');await page.locator('#instance').filter({hasText:db.identity.instanceId}).waitFor();return page;}
  const action={kind:'webrtc',setting:'disable_non_proxied_udp'};
  try{
    await openWorker();await pair();const originalInstance=db.identity.instanceId;
    assert.equal(control({op:'submit',instance_id:originalInstance,operation_id:'recovery-old-operation',action}).ok,true);await ok({type:'refreshPairing'});dropAck=true;
    assert.equal((await ok({type:'commit',id:'recovery-old-operation'})).phase,'completed');assert.equal(db['operation:recovery-old-operation'].nativeAcknowledged,undefined);
    assert.equal(control({op:'query',instance_id:originalInstance,operation_id:'recovery-old-operation'}).data.phase,'completed','real host durably accepted receipt before ACK loss');
    await ok({type:'resetIdentity'});const newInstance=db.identity.instanceId;assert.notEqual(newInstance,originalInstance);await pair();await openWorker();
    let page=await popup();await page.close();page=await popup();await page.close();
    assert.equal(control({op:'submit',instance_id:newInstance,operation_id:'recovery-new-operation',action}).ok,true);await ok({type:'refreshPairing'});assert.equal((await ok({type:'commit',id:'recovery-new-operation'})).phase,'completed');
    assert.equal(control({op:'query',instance_id:newInstance,operation_id:'recovery-new-operation'}).data.phase,'completed');assert.equal(settingWrites,2);
    assert.equal(db['operation:recovery-old-operation'].nativeInstanceId,originalInstance);assert(!db['operation:recovery-old-operation'].nativeAcknowledged);assert(requests.filter(r=>r.receipt?.id==='recovery-old-operation').every(r=>r.instance_id===originalInstance));
    report.checks.push('production background/Engine + real framed host: lost ACK, reset, new pairing, worker/popup reopen and fresh commit retain original instance/ID');
    assert.equal(control({op:'submit',instance_id:newInstance,operation_id:'legacy-real-known',action}).ok,true);
    await ok({type:'refreshPairing'});await ok({type:'commit',id:'legacy-real-known'});
    // A terminal host operation is absent from poll proposals: only the real
    // authenticated operation_query can establish this legacy record's owner.
    delete db['operation:legacy-real-known'].nativeInstanceId;delete db['operation:legacy-real-known'].nativeAcknowledged;
    db['operation:legacy-real-unknown']={id:'legacy-real-unknown',action,source:'app',phase:'completed',createdAt:Date.now(),result:{verification:'synthetic unknown legacy fact'}};
    await ok({type:'refreshPairing'});assert.equal(db['operation:legacy-real-known'].nativeInstanceId,newInstance);assert.equal(db['operation:legacy-real-known'].nativeAcknowledged,true);assert(requests.some(r=>r.op==='operation_query'&&r.operation_id==='legacy-real-known'));
    assert.equal(db['operation:legacy-real-unknown'].nativeInstanceId,undefined);assert(db['operation:legacy-real-unknown'].nativeIdentityLimitation);assert(!requests.some(r=>r.receipt?.id==='legacy-real-unknown'));assert.equal(control({op:'query',instance_id:newInstance,operation_id:'legacy-real-unknown'}).ok,false);
    report.checks.push('real authenticated operation_query binds exact legacy operation; unknown legacy stays unbound, retained and unsent');
    const clear=await ok({type:'preview',action:{kind:'clear',origins:['https://claude.ai'],types:['cookies','serviceWorkers']}});await ok({type:'commit',id:clear.id});removeFailure=true;assert.equal((await send({type:'releaseIsolation',id:clear.id})).ok,false);assert(db[`operation:${clear.id}`].isolationReleasedAt);assert.equal(rules.length,0);const writes=clearWrites;
    removeFailure=false;await openWorker();page=await popup();await page.getByRole('button',{name:'完成隔离释放恢复',exact:true}).waitFor();await page.close();page=await popup();await page.getByRole('button',{name:'完成隔离释放恢复',exact:true}).click();await page.getByRole('button',{name:'完成隔离释放恢复',exact:true}).waitFor({state:'detached'});assert.equal(db.cleanupIsolation,undefined);assert.equal(clearWrites,writes);await page.close();
    report.checks.push('real popup open/reopen reaches exact pending release; synthetic storage fault and browser APIs only, no fabricated runtime.onStartup');
  }finally{for(const child of [...processes]){const exited=new Promise(resolve=>child.once('exit',resolve));child.stdin.end();await exited;}}
}
