import test from 'node:test';import assert from 'node:assert/strict';
import {Engine} from '../src/engine.js';import {normalizeAction,describe,dataOptions} from '../src/policy.js';
const chromium={browser:'chromium',sites:[{origin:'https://claude.ai',domain:'claude.ai'}]};const firefox={...chromium,browser:'firefox'};
const twoSites={browser:'chromium',sites:[{origin:'https://claude.ai',domain:'claude.ai'},{origin:'https://console.anthropic.com',domain:'anthropic.com'}]};
function fake(){const db={},calls=[],rules=[];let setting={value:'default',levelOfControl:'controllable_by_this_extension'};const api={storage:{local:{get:async k=>k===null?structuredClone(db):{[k]:structuredClone(db[k])},set:async v=>Object.assign(db,structuredClone(v)),remove:async k=>{delete db[k];}}},permissions:{contains:async()=>true,getAll:async()=>({permissions:[]})},browsingData:{remove:async(o,t)=>calls.push({o,t,journal:structuredClone(db)}),removeCache:async()=>calls.push('cache')},tabs:{query:async()=>[{id:1,url:'https://claude.ai/a'},{id:2,url:'https://unrelated.example'}],remove:async ids=>calls.push({closed:ids})},webNavigation:{getAllFrames:async()=>[]},declarativeNetRequest:{getDynamicRules:async()=>structuredClone(rules),updateDynamicRules:async({addRules=[],removeRuleIds=[]})=>{for(const id of removeRuleIds){const i=rules.findIndex(r=>r.id===id);if(i>=0)rules.splice(i,1);}rules.push(...structuredClone(addRules));}},cookies:{getAll:async()=>[{value:'SYNTHETIC_SECRET'}]},privacy:{network:{webRTCIPHandlingPolicy:{get:async()=>structuredClone(setting),set:async({value})=>{setting={value,levelOfControl:'controlled_by_this_extension'};},clear:async()=>{setting={value:'default',levelOfControl:'controllable_by_this_extension'};}}}},alarms:{create:async()=>{}}};return {api,db,calls,rules,change:v=>setting=v};}
async function finish(e,p){await e.browserStarted();const next=await e.preview({kind:'finishClear',receiptId:p.id});return e.commit(next.id);}
const clear={kind:'clear',origins:['https://claude.ai'],types:['cookies','localStorage','indexedDB','serviceWorkers','cacheStorage']};
test('preview has cookie expansion; no native API mutation before commit',async()=>{const f=fake(),e=new Engine(f.api,chromium),p=await e.preview(clear);assert.equal(f.calls.length,0);assert.deepEqual(p.preview.effectiveCookieScope,['claude.ai']);assert.equal(p.preview.irreversible,true);await e.commit(p.id);assert.deepEqual(f.calls[0],{closed:[1]});const remove=f.calls.find(c=>c.t);assert.equal(remove.journal[`operation:${p.id}`].phase,'running');assert.deepEqual(remove.t,{serviceWorkers:true});assert(!JSON.stringify(f.db).includes('SYNTHETIC_SECRET'));});
test('durable completed ID cannot clear a newly created login after worker restart',async()=>{const f=fake(),e=new Engine(f.api,chromium),p=await e.preview(clear);await e.commit(p.id);const done=await finish(e,p);const calls=f.calls.length;const restarted=new Engine(f.api,chromium);await restarted.recover();assert.equal((await restarted.commit(done.id)).phase,'completed');assert.equal(f.calls.length,calls);});
test('running journal becomes uncertain, never replays',async()=>{const f=fake(),e=new Engine(f.api,chromium),p=await e.preview(clear);f.db[`operation:${p.id}`].phase='running';const restarted=new Engine(f.api,chromium);await restarted.recover();assert.equal((await restarted.commit(p.id)).phase,'uncertain');assert.equal(f.calls.length,0);});
test('Firefox rejects site HTTP cache and cacheStorage without invoking API',()=>{for(const type of ['cache','cacheStorage'])assert.throws(()=>normalizeAction({...clear,types:[type]},firefox));assert.deepEqual(dataOptions({...clear,types:['cookies'],cookieStoreId:'firefox-container-2'},firefox),{hostnames:['claude.ai'],cookieStoreId:'firefox-container-2'});assert.throws(()=>normalizeAction({...clear,cookieStoreId:'firefox-container-2'},firefox),/Firefox browsingData/);assert.throws(()=>normalizeAction({...clear,types:['serviceWorkers'],cookieStoreId:'firefox-container-2'},firefox),/容器限定/);});
test('app-origin action requires current native pairing',async()=>{const f=fake(),e=new Engine(f.api,chromium),p=await e.preview(clear,'app');await assert.rejects(e.commit(p.id),/pairing_required/);assert.equal(f.calls.length,0);});
test('same ID bound to immutable action',async()=>{const f=fake(),e=new Engine(f.api,chromium);await e.preview(clear,'local','operation-one');await assert.rejects(e.preview({kind:'clearProfileCache'},'local','operation-one'),/operation_id_conflict/);});
test('restore reveals underlying policy, but respects outside changes',async()=>{const f=fake(),e=new Engine(f.api,chromium),p=await e.preview({kind:'webrtc',setting:'disable_non_proxied_udp'});await e.commit(p.id);f.change({value:'default_public_interface_only',levelOfControl:'controlled_by_other_extensions'});const restore=await e.preview({kind:'restore',receiptId:p.id});const result=await e.commit(restore.id);assert.equal(result.error.code,'restore_conflict');assert.equal((await f.api.privacy.network.webRTCIPHandlingPolicy.get()).value,'default_public_interface_only');});
test('unapproved sites, irrelevant fields, and missing writer handling rejected',async()=>{assert.throws(()=>normalizeAction({...clear,origins:['https://mail.example']},chromium));assert.throws(()=>normalizeAction({kind:'clearProfileCache',origins:['https://claude.ai']},chromium));const f=fake();await assert.rejects(new Engine(f.api,chromium).preview({...clear,types:['cookies']}),/未选择/);});
test('cleanup isolation persists until explicit release',async()=>{const f=fake(),e=new Engine(f.api,chromium),p=await e.preview(clear);await e.commit(p.id);assert(f.rules.some(r=>r.id===10000));await e.releaseIsolation(p.id);assert.equal(f.rules.length,0);});
test('entire profile cache is a separately named action',()=>{const p=describe(normalizeAction({kind:'clearProfileCache'},firefox),firefox);assert.equal(p.scope,'current-profile');assert.match(p.impact,/整个当前 profile/);});

test('target iframes close their host tab without recording its URL',async()=>{const f=fake();f.api.webNavigation.getAllFrames=async({tabId})=>tabId===2?[{frameId:4,url:'https://claude.ai/embedded'}]:[];const e=new Engine(f.api,chromium),p=await e.preview(clear);assert(p.permissions.permissions.includes('webNavigation'));const r=await e.commit(p.id);assert.deepEqual(f.calls[0],{closed:[1,2]});assert.equal(r.result.embeddedWriterTabCount,1);assert(!JSON.stringify(f.db).includes('unrelated.example'));});
test('released and foreign receipts cannot release another operation isolation',async()=>{const f=fake(),e=new Engine(f.api,chromium),a=await e.preview(clear);await e.commit(a.id);await e.releaseIsolation(a.id);const b=await e.preview(clear);await e.commit(b.id);await e.releaseIsolation(a.id);assert.equal(f.rules.length,1);assert.equal(f.db.cleanupIsolation.operationId,b.id);f.db[`operation:${a.id}`].isolationReleasedAt=undefined;await assert.rejects(e.releaseIsolation(a.id),/不拥有当前隔离/);assert.equal(f.rules.length,1);});
test('failed removal retains owned isolation and never repeats deletion',async()=>{const f=fake();f.api.browsingData.remove=async()=>{throw new Error('synthetic interruption');};const e=new Engine(f.api,chromium),p=await e.preview(clear);assert.equal((await e.commit(p.id)).phase,'uncertain');assert.equal(f.db.cleanupIsolation.operationId,p.id);assert.equal(f.rules.length,1);await e.releaseIsolation(p.id);assert.equal(f.rules.length,0);});
test('Firefox container closes only its frame hosts and validates store before mutations',async()=>{const f=fake();f.api.cookies.getAllCookieStores=async()=>[{id:'firefox-container-2'}];f.api.tabs.query=async()=>[{id:1,url:'https://claude.ai/a',cookieStoreId:'firefox-container-2'},{id:2,url:'https://claude.ai/a',cookieStoreId:'firefox-container-3'}];const e=new Engine(f.api,firefox);const a={...clear,types:['cookies','localStorage','indexedDB'],cookieStoreId:'firefox-container-2'};const p=await e.preview(a);assert(p.permissions.permissions.includes('cookies'));await e.commit(p.id);const r=await finish(e,p);assert.equal(r.phase,'completed');assert.deepEqual(f.calls[0],{closed:[1]});assert.deepEqual(f.calls.find(c=>c.t).o,{hostnames:['claude.ai'],cookieStoreId:'firefox-container-2'});assert.equal(r.result.writerHandling,'container-frames-closed-service-workers-not-verified');await e.releaseIsolation(r.id);const invalid=await e.preview({...a,cookieStoreId:'firefox-container-9'});const count=f.calls.length;assert.equal((await e.commit(invalid.id)).error.code,'unknown_cookie_store');assert.equal(f.calls.length,count);assert.equal(f.rules.length,0);});
test('successful WebRTC restore reads back underlying control instead of asserting a stale value',async()=>{const f=fake(),e=new Engine(f.api,chromium),p=await e.preview({kind:'webrtc',setting:'disable_non_proxied_udp'});await e.commit(p.id);const r=await e.preview({kind:'restore',receiptId:p.id});const restored=await e.commit(r.id);assert.equal(restored.phase,'completed');assert.equal(restored.result.verification,'effective-readback');assert.equal(restored.result.effective.levelOfControl,'controllable_by_this_extension');});

test('cleanup requires a real browser startup event and consumes preparation once',async()=>{const f=fake(),e=new Engine(f.api,chromium),p=await e.preview(clear);const prepared=await e.commit(p.id);assert.equal(prepared.phase,'awaiting-browser-restart');assert.equal(f.calls.filter(c=>c.t).length,1);await assert.rejects(e.preview({kind:'finishClear',receiptId:p.id}),/请完整退出/);await new Engine(f.api,chromium).recover();await assert.rejects(e.preview({kind:'finishClear',receiptId:p.id}),/请完整退出/);const done=await finish(e,p);assert.equal(done.phase,'completed');assert.equal(f.db.cleanupIsolation.operationId,done.id);await assert.rejects(e.preview({kind:'finishClear',receiptId:p.id}),/cleanup_preparation_not_active/);assert.equal((await e.commit(done.id)).phase,'completed');});
test('DNR readback key order does not prevent continuing the same isolation',async()=>{const f=fake(),e=new Engine(f.api,chromium),p=await e.preview(clear);await e.commit(p.id);f.api.declarativeNetRequest.getDynamicRules=async()=>f.rules.map(r=>({condition:r.condition,action:r.action,priority:r.priority,id:r.id}));const done=await finish(e,p);assert.equal(done.phase,'completed');});
test('DNR readback array order does not prevent continuing the same isolation',async()=>{const f=fake(),e=new Engine(f.api,chromium),p=await e.preview(clear);await e.commit(p.id);const stored=structuredClone(f.rules);f.api.declarativeNetRequest.getDynamicRules=async()=>structuredClone(stored).reverse();const done=await finish(e,p);assert.equal(done.phase,'completed');});
test('onRunning fires only after the durable running boundary and not on preflight rejection',async()=>{const f=fake(),e=new Engine(f.api,chromium),p=await e.preview(clear);let observed;await e.commit(p.id,{onRunning:async r=>{observed=structuredClone(f.db[`operation:${p.id}`]);}});assert.equal(observed.phase,'running');const f2=fake();f2.api.permissions.contains=async()=>false;const e2=new Engine(f2.api,chromium),q=await e2.preview(clear);let called=false;await assert.rejects(e2.commit(q.id,{onRunning:async()=>{called=true;}}),/permissions_missing/);assert.equal(called,false);assert.equal(f2.db[`operation:${q.id}`].phase,'preview');});
test('a failed running receipt leaves the op re-deliverable in preview',async()=>{const f=fake(),e=new Engine(f.api,chromium),p=await e.preview(clear);await assert.rejects(e.commit(p.id,{onRunning:async()=>{throw new Error('native unavailable');}}),/native unavailable/);assert.equal(f.db[`operation:${p.id}`].phase,'preview');assert.equal(f.calls.length,0);const done=await e.commit(p.id);assert.equal(done.phase,'awaiting-browser-restart');});

// --- Browser state-continuation fixes -------------------------------------------------

test('ACK updates only metadata via the serial mutation entry, preserving a newer restore',async()=>{
  const f=fake(),e=new Engine(f.api,chromium);
  const clear=await e.preview({kind:'blockSites',origins:['https://claude.ai']});await e.commit(clear.id);
  // Ack while a restore is completing (overlapping native ACK latency).
  const ack=engineAck(e,clear.id);
  const restore=await e.preview({kind:'restore',receiptId:clear.id});const restored=await e.commit(restore.id,{onRunning:async()=>{}});
  await ack;
  assert.equal(restored.phase,'completed');
  const clearAfter=await e.get(`operation:${clear.id}`);
  assert.equal(clearAfter.restoredBy,restore.id,'ACK must not erase restoredBy written by the newer restore');
  assert.equal(clearAfter.nativeAcknowledged,true);
  assert.equal(clearAfter.acknowledgedAt>0,true);
  // A one-shot restore must not be re-armed by the ACK: a second restore is refused.
  await assert.rejects(e.preview({kind:'restore',receiptId:clear.id}),/not_restorable/);
});
function engineAck(e,id){return new Promise(resolve=>setTimeout(()=>resolve(e.acknowledge(id)),0));}

test('ACK refuses a non-terminal preview and never mutates the browser',async()=>{
  const f=fake(),e=new Engine(f.api,chromium);const p=await e.preview(clear);
  await assert.rejects(e.acknowledge(p.id),/nothing to acknowledge/);
  assert.equal((await e.get(`operation:${p.id}`)).phase,'preview');
  assert.equal(f.calls.length,0);
});

test('an expired preview becomes a durable terminal state instead of a repeated failure',async()=>{
  const f=fake(),e=new Engine(f.api,chromium);const p=await e.preview(clear);
  f.db[`operation:${p.id}`].expiresAt=Date.now()-1;
  const expired=await e.commit(p.id);
  assert.equal(expired.phase,'expired');assert.equal(expired.error.code,'preview_expired');
  assert.equal(f.db[`operation:${p.id}`].phase,'expired');
  assert.equal(f.calls.length,0,'expired preview never touched the browser');
  // Once terminal it can never replay.
  assert.equal((await e.commit(p.id)).phase,'expired');
  assert.equal((await e.acknowledge(p.id)).nativeAcknowledged,true);
});

test('cancelling an unexecuted preview persists a terminal state and does not mutate the browser',async()=>{
  const f=fake(),e=new Engine(f.api,chromium);const p=await e.preview(clear);
  const canceled=await e.abandon(p.id,'user-cancelled');
  assert.equal(canceled.phase,'canceled');assert.equal(canceled.cancelReason,'user-cancelled');
  assert.equal(f.calls.length,0);assert.equal(f.rules.length,0);
  assert.equal((await e.commit(p.id)).phase,'canceled','a canceled preview cannot execute');
  assert.equal((await e.acknowledge(p.id)).nativeAcknowledged,true);
  // Abandon is a no-op for an op that already left preview.
  const c2=await e.preview(clear);await e.commit(c2.id);
  await e.abandon(c2.id,'late');assert.equal((await e.get(`operation:${c2.id}`)).phase,'awaiting-browser-restart');
});

test('blockSites verifies DNR readback order-insensitively',async()=>{
  const f=fake(),e=new Engine(f.api,twoSites);
  const p=await e.preview({kind:'blockSites',origins:['https://claude.ai','https://console.anthropic.com']},'local','blocksites-two');
  const clearRules=f.api.declarativeNetRequest.getDynamicRules.bind(f.api.declarativeNetRequest);
  let reads=0;
  f.api.declarativeNetRequest.getDynamicRules=async()=>{reads++;return reads>=2?clearRules().then(rules=>structuredClone(rules).reverse()):clearRules();};
  const done=await e.commit(p.id);
  assert.equal(done.phase,'completed');
  assert.equal(done.result.rules.length,2);
});

test('restore of two reversed block rules reads back by rule identity',async()=>{
  const f=fake(),e=new Engine(f.api,twoSites);
  const p=await e.preview({kind:'blockSites',origins:['https://claude.ai','https://console.anthropic.com']});
  await e.commit(p.id);
  // Every readback from here on returns the stored rules reversed, as DNR may.
  const base=f.api.declarativeNetRequest.getDynamicRules.bind(f.api.declarativeNetRequest);
  f.api.declarativeNetRequest.getDynamicRules=async()=>structuredClone(await base()).reverse();
  const r=await e.preview({kind:'restore',receiptId:p.id});
  const restored=await e.commit(r.id);
  assert.equal(restored.phase,'completed',JSON.stringify(restored.error||restored.result));
  assert.equal(restored.result.verification,'effective-readback');
});


test('popup continuation atomically finishes the original App clear and stale preparation ACK stays unacked',async()=>{
  const f=fake(),e=new Engine(f.api,chromium);await e.set('pairing',{paired:true});
  const p=await e.preview(clear,'app');const prepared=await e.commit(p.id);
  await e.acknowledge(p.id,{sent:prepared});await e.browserStarted();
  const next=await e.preview({kind:'finishClear',receiptId:p.id});const done=await e.commit(next.id);
  await e.acknowledge(p.id,{sent:prepared});
  const original=await e.get(`operation:${p.id}`);
  assert.equal(original.phase,'completed');assert.equal(original.result.continuedBy,done.id);
  assert.equal(original.source,'app');assert.equal(original.nativeAcknowledged,false);
  assert.equal(original.finishedBy,done.id);
  const count=f.calls.length;await e.commit(p.id);await e.commit(done.id);assert.equal(f.calls.length,count);
  await e.acknowledge(p.id,{sent:original});assert.equal((await e.get(`operation:${p.id}`)).nativeAcknowledged,true);
});
test('interrupted consumed popup child makes original App clear uncertain without replay',async()=>{
  const f=fake(),e=new Engine(f.api,chromium);await e.set('pairing',{paired:true});
  const p=await e.preview(clear,'app');await e.commit(p.id);await e.browserStarted();
  const next=await e.preview({kind:'finishClear',receiptId:p.id});
  // Model process loss after consumption is persisted, before API completion.
  f.db[`operation:${p.id}`].finishedBy=next.id;
  f.db[`operation:${next.id}`].phase='running';f.db.cleanupIsolation.operationId=next.id;
  const count=f.calls.length;await new Engine(f.api,chromium).recover();
  assert.equal(f.db[`operation:${p.id}`].phase,'uncertain');assert.equal(f.db[`operation:${p.id}`].nativeAcknowledged,false);
  await e.commit(next.id);assert.equal(f.calls.length,count);
});
test('status sweeps expired previews without requiring a failed execution',async()=>{
  const f=fake(),e=new Engine(f.api,chromium),p=await e.preview(clear,'app');
  f.db[`operation:${p.id}`].expiresAt=Date.now()-1;
  assert.equal((await e.status()).receipts[0].phase,'expired');assert.equal(f.calls.length,0);
});
