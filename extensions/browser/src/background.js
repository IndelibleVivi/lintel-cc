import {CONFIG} from './config.js';
import {Engine} from './engine.js';
import {fail} from './policy.js';
const api=globalThis.browser || globalThis.chrome;
const engine=new Engine(api);
let port,identity,polling=false;
const pending=new Map();
const ready=(async()=>{
  identity=await engine.get('identity');
  if (!identity) {identity=newIdentity();await engine.set('identity',identity);}
  await engine.recover();
  await api.alarms.create('bridge-status',{periodInMinutes:0.5});
  await engine.resumeRules().catch(error=>engine.set('ruleConflict',error.message));
})();
function newIdentity(){return {instanceId:crypto.randomUUID(),token:Array.from(crypto.getRandomValues(new Uint8Array(32)),b=>b.toString(16).padStart(2,'0')).join('')};}
function connect(){
  if (port) return port;
  port=api.runtime.connectNative('app.lintel.browser');
  port.onMessage.addListener(msg=>{
    const waiter=pending.get(msg.request_id);
    if (waiter){clearTimeout(waiter.timer);pending.delete(msg.request_id);msg.ok ? waiter.resolve(msg.data) : waiter.reject(Object.assign(new Error(msg.error?.message || 'native error'),{code:msg.error?.code}));}
  });
  port.onDisconnect.addListener(()=>{
    const message=api.runtime.lastError?.message || 'Native host disconnected';port=undefined;
    for (const waiter of pending.values()){clearTimeout(waiter.timer);waiter.reject(new Error(message));}pending.clear();
    engine.set('bridge',{connected:false,reason:message,at:Date.now()});
    engine.set('pairing',{paired:false,reason:'bridge-disconnected'});
  });
  return port;
}
async function native(op,extra={}){
  await ready;const request_id=crypto.randomUUID();
  return new Promise((resolve,reject)=>{
    const timer=setTimeout(()=>{pending.delete(request_id);reject(new Error('native request timeout; query durable status before retry'));},8000);
    pending.set(request_id,{resolve,reject,timer});
    try {connect().postMessage({op,request_id,instance_id:identity.instanceId,token:identity.token,...extra});}catch(e){clearTimeout(timer);pending.delete(request_id);reject(e);}
  });
}
function receipt(r){return {id:r.id,phase:r.phase,...(r.result?{result:r.result}:{}),...(r.error?{error:r.error}:{}),...(r.completedAt?{completedAt:r.completedAt}:{})};}
async function poll(){
  if (polling) return;polling=true;
  try {
    await engine.expirePreviews();
    const data=await native('poll');
    await engine.set('pairing',{paired:true});await engine.set('bridge',{connected:true,lastSeen:Date.now()});
    for (const proposal of data.proposals){
      try {await engine.preview(proposal.action,'app',proposal.id);}catch(e){await native('receipt',{receipt:{id:proposal.id,phase:'rejected',error:{code:e.code || 'invalid_proposal',message:e.message}}});}
    }
    // Re-send only durable receipts, never browser mutations. Lost ACK is safe.
    const all=await api.storage.local.get(null);
    for (const [key,r] of Object.entries(all)) if (key.startsWith('operation:')&&r.source==='app'&&['completed','uncertain','rejected','awaiting-browser-restart','canceled','expired'].includes(r.phase)&&!r.nativeAcknowledged){
      await native('receipt',{receipt:receipt(r)});await engine.acknowledge(r.id,{sent:r});
    }
  }catch(error){
    await engine.set('pairing',{paired:false,reason:error.code || 'native-unavailable'});
    await engine.set('bridge',{connected:false,reason:error.message,at:Date.now()});
  }finally {polling=false;}
}
async function handle(message,sender){
  await ready;
  // Only bundled popup/options pages, never web pages or content scripts.
  if (sender.id!==api.runtime.id || sender.url!==api.runtime.getURL('popup.html')) fail('untrusted_sender');
  switch(message.type){
    case 'status': return engine.status();
    case 'pair': return native('pair_request',{code:String(message.code).trim().toUpperCase(),label:String(message.label||'My browser profile'),browser:CONFIG.browser});
    case 'refreshPairing': await poll();return engine.status();
    case 'resetIdentity':
      if(port)port.disconnect();identity=newIdentity();await engine.set('identity',identity);await engine.set('pairing',{paired:false,reason:'identity-reset-repair-required'});return {instanceId:identity.instanceId};
    case 'preview': return engine.preview(message.action);
    case 'cancel': {const result=await engine.abandon(message.id,'user-cancelled');await pollIfConnected();return result;}
    case 'commit': {
      const r=await engine.get(`operation:${message.id}`);
      if (r?.source==='app') {await poll();if (!(await engine.get('pairing'))?.paired) fail('pairing_required');}
      // Report `running` only after commit has durably crossed its running
      // journal boundary. A preflight rejection (permissions_missing,
      // preview_expired, ...) throws before onRunning, so the host is never told
      // `running` for an op that stayed re-deliverable in preview.
      const result=await engine.commit(message.id,{onRunning:r?.source==='app' ? async()=>{await native('receipt',{receipt:{id:r.id,phase:'running'}});} : undefined});
      await pollIfConnected();return result;
    }
    case 'releaseIsolation': return engine.releaseIsolation(message.id);
    default: fail('unknown_popup_message');
  }
}
async function pollIfConnected(){if(port) await poll();}
api.runtime.onMessage.addListener((message,sender,sendResponse)=>{handle(message,sender).then(data=>sendResponse({ok:true,data}),e=>sendResponse({ok:false,error:{code:e.code||'extension_error',message:e.message}}));return true;});
api.alarms.onAlarm.addListener(alarm=>{if(alarm.name==='bridge-status')poll();if(alarm.name==='resume-rules')engine.resumeRules().catch(e=>engine.set('ruleConflict',e.message));});
api.runtime.onStartup.addListener(()=>engine.browserStarted());
api.permissions.onRemoved.addListener(()=>engine.set('bridge',{connected:false,reason:'permission-removed; refresh effective status',at:Date.now()}));
// A connected native port keeps a Chromium MV3 worker alive. Heartbeats also bind
// concurrent copied installations; alarms recover after an actual worker restart.
setInterval(()=>{if(port)poll();},5000);
