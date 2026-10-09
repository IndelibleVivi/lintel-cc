import {CONFIG} from './config.js';
import {fail,normalizeAction,describe,requiredPermissions,dataOptions,sitesFor} from './policy.js';
// Browser readback may reorder object fields (notably DNR rules).
const ordered=value=>Array.isArray(value)?value.map(ordered):value && typeof value==='object'?Object.fromEntries(Object.keys(value).sort().map(key=>[key,ordered(value[key])])):value;
const equal = (a,b) => JSON.stringify(ordered(a)) === JSON.stringify(ordered(b));
const ruleKey=rule=>rule && typeof rule==='object' && rule.id!==undefined ? rule.id : 0;
// DNR may return the same rule set in a different array order than it was stored; compare by rule id.
const equalRules = (a,b) => JSON.stringify([...(a||[])].sort((x,y)=>ruleKey(x)-ruleKey(y)).map(ordered)) === JSON.stringify([...(b||[])].sort((x,y)=>ruleKey(x)-ruleKey(y)).map(ordered));
const PERMISSION_API = {location:'location',camera:'camera',microphone:'microphone',notifications:'notifications'};
const CLEANUP_RULE_START = 10000;
const NETWORK_RULE_START = 20000;
const PREVIEW_TTL = 300000;
const TERMINAL_ACK_PHASES = ['completed','uncertain','rejected','awaiting-browser-restart','canceled','expired'];
export class Engine {
  constructor(api,config=CONFIG) {this.api=api;this.config=config;this.tail=Promise.resolve();}
  serial(fn) {const run=this.tail.then(fn,fn);this.tail=run.catch(()=>{});return run;}
  async get(key) {return (await this.api.storage.local.get(key))[key];}
  async set(key,value) {await this.api.storage.local.set({[key]:value});}
  async preview(input,source='local',operationId=crypto.randomUUID()) {
    const action=normalizeAction(input,this.config);
    if (!/^[A-Za-z0-9_-]{8,80}$/.test(operationId)) fail('invalid_operation_id');
    const existing=await this.get(`operation:${operationId}`);
    if (existing) {
      if (!equal(existing.action,action)) fail('operation_id_conflict');
      return existing;
    }
    const record={id:operationId,action,source,phase:'preview',createdAt:Date.now(),expiresAt:Date.now()+PREVIEW_TTL,preview:describe(action,this.config),permissions:requiredPermissions(action,this.config)};
    if (action.kind === 'clear') {
      record.preview.writerHandling=action.cookieStoreId ? '容器删除不注销共享 Service Worker；浏览器重启后仍仅保证所选 store 的 API 确认，无法验证后台本地回写停止。' : action.types.includes('serviceWorkers') ? '关闭目标 frame 并注销 Service Worker；重启浏览器后再次确认删除，隔离保留到单独解除。' : '未选择 serviceWorkers，无法可靠停止后台本地写入；执行前须选择该类别。';
      if (!action.cookieStoreId && !action.types.includes('serviceWorkers')) fail('service_worker_quiescence_required',record.preview.writerHandling);
    }
    if (action.kind === 'finishClear') {
      const prior=await this.clearPreparation(action.receiptId);
      record.preview={...prior.preview,preparationId:prior.id,impact:'已观察到准备后的浏览器启动。确认删除原范围的数据；隔离保留到单独解除。'};
      record.permissions=prior.permissions;
    }
    if (action.kind === 'restore') {
      const prior=await this.get(`operation:${action.receiptId}`);
      if (!prior?.undo || prior.restoredBy) fail('not_restorable');
      record.preview.restoring=prior.preview;
      record.permissions=prior.permissions;
    }
    await this.set(`operation:${operationId}`,record);
    return record;
  }
  // `onRunning` runs after preflight succeeds and the durable `running` journal
  // boundary is crossed, but before any browser mutation. Callers use it to
  // report `running` to the host only once the op can no longer be rejected
  // without a side effect. A preflight rejection leaves the op in `preview`
  // (re-deliverable) and never invokes the callback.
  commit(id,{onRunning}={}) {return this.serial(async()=>{
    const key=`operation:${id}`,r=await this.get(key);
    if (!r) fail('unknown_operation');
    if (r.phase !== 'preview') return r; // running/uncertain/completed never replay
    if (r.expiresAt < Date.now()) {
      // A preview that was never executed must not stay re-deliverable forever.
      // Make the terminal state durable under the serial lock (no browser change).
      const expired={...r,phase:'expired',expiredAt:Date.now(),error:{code:'preview_expired',message:'预览已过期，未执行；请重新预览以获得新的操作 ID，不会改变浏览器。'}};
      await this.set(key,expired);
      return expired;
    }
    if (r.source === 'app' && !(await this.get('pairing'))?.paired) fail('pairing_required');
    if (!await this.api.permissions.contains(r.permissions)) fail('permissions_missing');
    const running={...r,phase:'running',startedAt:Date.now()};
    await this.set(key,running); // durable boundary BEFORE any browser mutation
    if (onRunning) {
      try {
        await onRunning(running);
      } catch (error) {
        // The host was not told `running`, so no phantom op exists. Restore the
        // preview so the operation stays re-deliverable; no browser mutation ran.
        await this.set(key,r);
        throw error;
      }
    }
    try {
      const result=await this.execute(running);
      const phase=result.phase||'completed';
      const done={...running,...result,phase,...(phase==='completed'?{completedAt:Date.now()}:{preparedAt:Date.now()})};
      await this.saveOutcome(done);return done;
    } catch (error) {
      // API failure may follow a side effect. Keep uncertainty; never retry the ID.
      const saved=await this.get(key);
      const uncertain={...saved,phase:'uncertain',error:{code:error.code||'browser_api_error',message:error.message},completedAt:Date.now()};
      await this.saveOutcome(uncertain);return uncertain;
    }
  });}
  // Single serialized entry point for ACK metadata. The host acknowledges a
  // durable receipt by id; the extension must re-read the *latest* record under
  // the same lock and only stamp acknowledgement fields. Writing back a stale
  // snapshot (e.g. a `restore` completion) would erase newer Engine state such
  // as `restoredBy`, silently re-arming a one-shot restore. No browser mutation.
  acknowledge(id,{sent,acknowledgedAt=Date.now()}={}) {return this.serial(async()=>{
    const key=`operation:${id}`,r=await this.get(key);
    if (!r) fail('unknown_operation');
    if (!TERMINAL_ACK_PHASES.includes(r.phase)) fail('operation_not_terminal',`operation ${id} is ${r.phase}; nothing to acknowledge`);
    // A preparation ACK may arrive after finishClear changed the final fact.
    if (sent && !equal([sent.phase,sent.result,sent.error,sent.completedAt],[r.phase,r.result,r.error,r.completedAt])) return r;
    r.acknowledgedAt=acknowledgedAt;r.nativeAcknowledged=true;
    await this.set(key,r);return r;
  });}
  // Persist a terminal state for a preview that was never executed (user cancel
  // or a consumed/expired preview). Never mutates the browser. Uses the same
  // serialized mutation path so it cannot clobber a concurrent commit.
  abandon(id,reason) {return this.serial(async()=>{
    const key=`operation:${id}`;
    const r=await this.get(key);
    if (!r || r.phase!=='preview') return r;
    const terminal={...r,phase:'canceled',canceledAt:Date.now(),cancelReason:reason};
    await this.set(key,terminal);
    return terminal;
  });}
  expirePreviews() {return this.serial(async()=>{
    const all=await this.api.storage.local.get(null);
    for (const [key,r] of Object.entries(all)) if (key.startsWith('operation:') && r.phase==='preview' && r.expiresAt<Date.now()) {
      await this.set(key,{...r,phase:'expired',expiredAt:Date.now(),error:{code:'preview_expired',message:'预览已过期，未执行；请重新预览。'}});
    }
  });}
  // Called under the Engine lock. The child and original clear publish their
  // final fact together; popup continuation therefore uses the original App ID.
  async saveOutcome(r) {
    const records={[`operation:${r.id}`]:r};
    if (r.action.kind==='finishClear') {
      const prior=await this.get(`operation:${r.action.receiptId}`);
      if (prior?.finishedBy===r.id) records[`operation:${prior.id}`]={...prior,phase:r.phase,
        result:{...r.result,continuedBy:r.id},...(r.error?{error:r.error}:{}),completedAt:r.completedAt,nativeAcknowledged:false};
    }
    await this.api.storage.local.set(records);
  }
  recover() {return this.serial(async()=>{
    const all=await this.api.storage.local.get(null);
    for (const [key,r] of Object.entries(all)) if (key.startsWith('operation:') && r.phase==='running') await this.saveOutcome({...r,phase:'uncertain',error:{code:'worker_restarted',message:'执行期间扩展重启；请核对持久结果，不自动重复删除。'}});
  });}
  async checkpoint(r,extra) {Object.assign(r,extra);await this.set(`operation:${r.id}`,r);}
  async execute(r) {
    const a=r.action;
    if (a.kind === 'clear') return this.clear(r);
    if (a.kind === 'finishClear') return this.finishClear(r);
    if (a.kind === 'clearProfileCache') {await this.api.browsingData.removeCache({});return {result:{verification:'browser-acknowledged',scope:'entire-current-profile-http-cache'}};}
    if (a.kind === 'webrtc') return this.applySetting(r,this.api.privacy.network.webRTCIPHandlingPolicy,a.setting,'webrtc');
    if (a.kind === 'proxy') {
      const hosts=a.origins.map(o=>new URL(o).hostname);
      // Literal hosts/port come exclusively from normalized allowlisted inputs.
      const data=`function FindProxyForURL(url,host){return ${JSON.stringify(hosts)}.indexOf(host)!==-1 ? "PROXY 127.0.0.1:${a.port}" : "DIRECT";}`;
      return this.applySetting(r,this.api.proxy.settings,{mode:'pac_script',pacScript:{data,mandatory:true}},'proxy');
    }
    if (a.kind === 'sitePermission') return this.sitePermission(r);
    if (a.kind === 'blockSites') {
      const current=await this.api.declarativeNetRequest.getDynamicRules();
      const prior=current.filter(v=>v.id>=NETWORK_RULE_START && v.id<NETWORK_RULE_START+100);
      const rules=this.rules(a,NETWORK_RULE_START);
      await this.checkpoint(r,{undo:{type:'rules',before:prior,applied:rules}});
      await this.api.declarativeNetRequest.updateDynamicRules({removeRuleIds:prior.map(v=>v.id),addRules:rules});
      const effective=(await this.api.declarativeNetRequest.getDynamicRules()).filter(v=>v.id>=NETWORK_RULE_START && v.id<NETWORK_RULE_START+100);
      if (!equalRules(effective,rules)) fail('rules_not_effective');
      await this.set('networkRules',rules);
      return {result:{verification:'effective-readback',rules:effective}};
    }
    if (a.kind === 'pauseRules') {
      const rules=(await this.api.declarativeNetRequest.getDynamicRules()).filter(v=>v.id>=NETWORK_RULE_START && v.id<NETWORK_RULE_START+100);
      if (!rules.length) fail('no_active_rules');
      const resumeAt=Date.now()+a.minutes*60000;
      await this.set('pausedRules',{rules,resumeAt,operationId:r.id});
      await this.api.alarms.create('resume-rules',{when:resumeAt});
      await this.api.declarativeNetRequest.updateDynamicRules({removeRuleIds:rules.map(v=>v.id)});
      return {result:{verification:'effective-readback',resumeAt}};
    }
    if (a.kind === 'restore') return this.restore(r);
    fail('unsupported_action');
  }
  rules(a,start) {
    return sitesFor(a.origins,this.config).map((s,i)=>({id:start+i,priority:100,action:{type:'block'},condition:{urlFilter:`||${new URL(s.origin).hostname}^`,resourceTypes:['main_frame','sub_frame','stylesheet','script','image','font','object','xmlhttprequest','ping','csp_report','media','websocket','other']}}));
  }
  async clear(r) {
    const a=r.action, rules=this.rules(a,CLEANUP_RULE_START);
    if (a.cookieStoreId && !(await this.api.cookies.getAllCookieStores()).some(store=>store.id===a.cookieStoreId)) fail('unknown_cookie_store','所选 Firefox cookie store 不存在；未清理。');
    // Cookie deletion on Chromium expands to the registrable domain, including sibling tabs.
    if (this.config.browser!=='firefox' && a.types.includes('cookies')) rules.forEach((rule,i)=>{rule.condition.urlFilter=`||${sitesFor(a.origins,this.config)[i].domain}^`;});
    const existing=(await this.api.declarativeNetRequest.getDynamicRules()).filter(v=>v.id>=CLEANUP_RULE_START && v.id<CLEANUP_RULE_START+100);
    if (existing.length) fail('cleanup_isolation_active','请先核对上次结果并解除旧清理隔离。');
    if (await this.get('cleanupIsolation')) fail('cleanup_isolation_active','请先核对上次结果并解除旧清理隔离。');
    await this.checkpoint(r,{isolationRuleIds:rules.map(v=>v.id)});
    await this.set('cleanupIsolation',{operationId:r.id,rules});
    await this.api.declarativeNetRequest.updateDynamicRules({addRules:rules});
    const sites=sitesFor(a.origins,this.config);
    const tabs=await this.api.tabs.query({});
    const matches=url=>{
      try {const u=new URL(url);return sites.some(s=>this.config.browser!=='firefox' && a.types.includes('cookies') ? u.hostname===s.domain || u.hostname.endsWith('.'+s.domain) : (this.config.browser==='firefox' ? u.hostname===new URL(s.origin).hostname : u.origin===s.origin));} catch {return false;}
    };
    const targets=[];let embeddedWriterTabCount=0;
    // Read URLs only for this approved cleanup. Never persist frame URLs/history.
    // Closing the host tab also terminates local writes in already loaded iframes.
    for (const tab of tabs) {
      if (a.cookieStoreId && tab.cookieStoreId!==a.cookieStoreId) continue;
      const frames=await this.api.webNavigation.getAllFrames({tabId:tab.id});
      if (matches(tab.url) || frames?.some(frame=>matches(frame.url))) {
        targets.push(tab);
        if (!matches(tab.url)) embeddedWriterTabCount++;
      }
    }
    await this.checkpoint(r,{closedTabCount:targets.length,embeddedWriterTabCount});
    if (targets.length) await this.api.tabs.remove(targets.map(t=>t.id));
    const options=dataOptions(a,this.config);
    // Unregister writers first, then clear stores; do not navigate to the sites for verification.
    if (a.types.includes('serviceWorkers')) await this.api.browsingData.remove(options,{serviceWorkers:true});
    await this.checkpoint(r,{preparedGeneration:await this.get('browserStartupGeneration')||'initial'});
    return {phase:'awaiting-browser-restart',result:{verification:'preparation-only',storageDeletion:'not-started',closedTabCount:targets.length,embeddedWriterTabCount,isolation:'active-until-explicit-release',nextAction:'restart-browser-then-confirm-finishClear',writerHandling:'unregistration-does-not-stop-active-events'}};
  }
  browserStarted() {return this.serial(()=>this.set('browserStartupGeneration',crypto.randomUUID()));}
  async clearPreparation(id) {
    const prior=await this.get(`operation:${id}`),owner=await this.get('cleanupIsolation');
    if (prior?.phase!=='awaiting-browser-restart' || prior.finishedBy || prior.isolationReleasedAt || owner?.operationId!==id) fail('cleanup_preparation_not_active');
    if ((await this.get('browserStartupGeneration')||'initial')===prior.preparedGeneration) fail('browser_restart_required','请完整退出并重启此浏览器，再确认继续删除；扩展 worker 重启不算浏览器重启。');
    const active=(await this.api.declarativeNetRequest.getDynamicRules()).filter(rule=>prior.isolationRuleIds.includes(rule.id));
    if (!equalRules(active,owner.rules)) fail('cleanup_isolation_changed','清理隔离已变化，未继续删除。');
    return prior;
  }
  async finishClear(r) {
    const prior=await this.clearPreparation(r.action.receiptId),a=prior.action,sites=sitesFor(a.origins,this.config),options=dataOptions(a,this.config);
    // Consume the preparation durably before deletion; a failed new operation is
    // uncertain and cannot turn the preparation into a second deletion attempt.
    prior.finishedBy=r.id;r.isolationRuleIds=prior.isolationRuleIds;
    const owner=await this.get('cleanupIsolation');owner.operationId=r.id;
    await this.api.storage.local.set({[`operation:${prior.id}`]:prior,[`operation:${r.id}`]:r,cleanupIsolation:owner});
    const types=Object.fromEntries(a.types.filter(t=>t!=='serviceWorkers').map(t=>[t,true]));
    if (Object.keys(types).length) await this.api.browsingData.remove(options,types);
    let cookieObservation={verification:'not-enumerated',reason:'optional cookies permission not granted'};
    if (a.types.includes('cookies') && await this.api.permissions.contains({permissions:['cookies'],origins:r.permissions.origins})) {
      let count=0;
      for (const s of sites) {
        const cookies=await this.api.cookies.getAll({domain:this.config.browser==='firefox' ? new URL(s.origin).hostname : s.domain,...(a.cookieStoreId?{storeId:a.cookieStoreId}:{})});
        count+=cookies.length; // values never leave this execution context or enter the journal
      }
      cookieObservation={verification:'enumerated-cookie-count',remaining:count};
    }
    return {result:{verification:'browser-acknowledged',categories:a.types,cookieObservation,closedTabCount:prior.closedTabCount,
      embeddedWriterTabCount:prior.embeddedWriterTabCount,writerHandling:a.cookieStoreId?'container-frames-closed-service-workers-not-verified':'browser-restarted-after-service-workers-unregistered',
      isolation:'active-until-explicit-release',storageEnumeration:'unavailable',remoteSessionRevocation:'not-performed',relogin:'not-started'}};
  }
  releaseIsolation(id) {return this.serial(async()=>{
    const r=await this.get(`operation:${id}`);
    if (!r?.isolationRuleIds || r.phase==='running') fail('no_releasable_isolation');
    if (r.isolationReleasedAt) return r;
    const owner=await this.get('cleanupIsolation');
    if (owner?.operationId!==id) fail('isolation_owner_conflict','此回执不拥有当前隔离，未移除任何规则。');
    await this.api.declarativeNetRequest.updateDynamicRules({removeRuleIds:r.isolationRuleIds});
    r.isolationReleasedAt=Date.now();await this.set(`operation:${id}`,r);
    await this.api.storage.local.remove('cleanupIsolation');return r;
  });}
  async applySetting(r,setting,value,type) {
    if (!setting) fail('setting_unsupported');
    const before=await setting.get({});
    if (!['controllable_by_this_extension','controlled_by_this_extension'].includes(before.levelOfControl)) fail('setting_control_conflict',before.levelOfControl);
    await this.checkpoint(r,{undo:{type,before,applied:value}});
    await setting.set({value,scope:'regular'});
    const after=await setting.get({});
    if (after.levelOfControl!=='controlled_by_this_extension' || !equal(after.value,value)) fail('setting_not_effective');
    return {result:{verification:'effective-readback',configured:value,effective:after.value,controller:after.levelOfControl,scope:'current-profile'}};
  }
  async sitePermission(r) {
    const name=PERMISSION_API[r.action.setting],api=this.api.contentSettings?.[name];
    if (!api) fail('permission_api_unsupported');
    const owned=await this.get(`contentRules:${name}`)||[];
    const before=[];
    for (const origin of r.action.origins) before.push({origin,...await api.get({primaryUrl:origin})});
    const newRules=r.action.origins.map(origin=>({primaryPattern:`${origin}/*`,setting:'block',scope:'regular'}));
    const applied=[...owned.filter(v=>!newRules.some(n=>n.primaryPattern===v.primaryPattern)),...newRules];
    await this.checkpoint(r,{undo:{type:'content',name,before:owned,applied,effectiveBefore:before}});
    // Save extension-owned rules before setting, to permit explicit recovery of partial writes.
    await this.set(`contentRules:${name}`,applied);
    for (const rule of newRules) await api.set(rule);
    const effective=[];
    for (const origin of r.action.origins) effective.push({origin,...await api.get({primaryUrl:origin})});
    if (effective.some(v=>v.setting!=='block')) fail('permission_not_effective');
    return {result:{verification:'effective-readback',effective,controller:'not-exposed-by-contentSettings-api'}};
  }
  async restore(r) {
    const prior=await this.get(`operation:${r.action.receiptId}`),u=prior?.undo;
    if (!u || prior.restoredBy) fail('not_restorable');
    let effective;
    if (['webrtc','proxy'].includes(u.type)) {
      const setting=u.type==='webrtc' ? this.api.privacy.network.webRTCIPHandlingPolicy : this.api.proxy.settings;
      const current=await setting.get({});
      if (current.levelOfControl!=='controlled_by_this_extension' || !equal(current.value,u.applied)) fail('restore_conflict','当前值或控制者已变化，未覆盖。');
      if (u.before.levelOfControl==='controlled_by_this_extension') await setting.set({value:u.before.value,scope:'regular'});
      else await setting.clear({scope:'regular'}); // reveal current underlying setting, not stale snapshot
      effective=await setting.get({});
      if (u.before.levelOfControl==='controlled_by_this_extension' ? effective.levelOfControl!=='controlled_by_this_extension' || !equal(effective.value,u.before.value) : effective.levelOfControl==='controlled_by_this_extension') fail('restore_not_effective');
    } else if (u.type==='rules') {
      const current=(await this.api.declarativeNetRequest.getDynamicRules()).filter(v=>v.id>=NETWORK_RULE_START && v.id<NETWORK_RULE_START+100);
      if (!equalRules(current,u.applied) || await this.get('pausedRules')) fail('restore_conflict');
      await this.api.declarativeNetRequest.updateDynamicRules({removeRuleIds:current.map(v=>v.id),addRules:u.before});
      await this.set('networkRules',u.before);
      effective=(await this.api.declarativeNetRequest.getDynamicRules()).filter(v=>v.id>=NETWORK_RULE_START && v.id<NETWORK_RULE_START+100);
      if (!equalRules(effective,u.before)) fail('restore_not_effective');
    } else if (u.type==='content') {
      if (!equal(await this.get(`contentRules:${u.name}`),u.applied)) fail('restore_conflict');
      const api=this.api.contentSettings[u.name];
      for (const rule of u.applied) if ((await api.get({primaryUrl:rule.primaryPattern.slice(0,-2)})).setting!==rule.setting) fail('restore_conflict');
      await api.clear({scope:'regular'}); // clear only this extension's rules of this content type
      for (const rule of u.before) await api.set(rule);
      await this.set(`contentRules:${u.name}`,u.before);
      effective=[];
      for (const {origin} of u.effectiveBefore) effective.push({origin,...await api.get({primaryUrl:origin})});
      for (const rule of u.before) if ((await api.get({primaryUrl:rule.primaryPattern.slice(0,-2)})).setting!==rule.setting) fail('restore_not_effective');
    }
    prior.restoredBy=r.id;await this.set(`operation:${prior.id}`,prior);
    return {result:{verification:'effective-readback',restored:r.action.receiptId,effective,...(u.type==='content'?{controller:'not-exposed-by-contentSettings-api'}:{})}};
  }
  // Alarms/startup share the mutation queue with an approved pause. Consume
  // only the record read by this reconciliation, never a newer pause generation.
  resumeRules() {return this.serial(async()=>{
    const paused=await this.get('pausedRules');
    if (!paused || paused.resumeAt>Date.now()) return;
    const current=(await this.api.declarativeNetRequest.getDynamicRules()).filter(v=>v.id>=NETWORK_RULE_START && v.id<NETWORK_RULE_START+100);
    // A prior DNR call may have applied before its Promise/storage failed.
    // Exact readback completes that same restoration without adding twice.
    if (current.length && !equalRules(current,paused.rules)) {await this.set('ruleConflict','暂停期间规则已变化；未覆盖。');return;}
    if (!current.length) {
      await this.api.declarativeNetRequest.updateDynamicRules({addRules:paused.rules});
      const effective=(await this.api.declarativeNetRequest.getDynamicRules()).filter(v=>v.id>=NETWORK_RULE_START && v.id<NETWORK_RULE_START+100);
      if (!equalRules(effective,paused.rules)) fail('resume_rules_not_effective');
    }
    // Snapshot matching also supports legacy records without operationId.
    if (equal(await this.get('pausedRules'),paused)) await this.api.storage.local.remove('pausedRules');
  });}
  async status() {
    await this.expirePreviews();
    const all=await this.api.storage.local.get(null);
    const webrtc=await this.api.privacy.network.webRTCIPHandlingPolicy.get({});
    return {browser:this.config.browser,synthetic:this.config.fixture,pairing:all.pairing || {paired:false},bridge:all.bridge || {connected:false},
      instanceId:all.identity?.instanceId,webrtc,permissions:await this.api.permissions.getAll(),
      receipts:Object.entries(all).filter(([k])=>k.startsWith('operation:')).map(([,v])=>v).sort((a,b)=>b.createdAt-a.createdAt),
      pausedRules:all.pausedRules,ruleConflict:all.ruleConflict,
      capabilities:{siteData:true,cacheStorage:this.config.browser!=='firefox',siteHttpCache:this.config.browser!=='firefox',containerStore:this.config.browser==='firefox',sitePermissions:this.config.browser!=='firefox',scopedProxy:this.config.browser!=='firefox'}};
  }
}
