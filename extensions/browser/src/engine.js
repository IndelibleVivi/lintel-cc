import {CONFIG} from './config.js';
import {fail,normalizeAction,describe,requiredPermissions,dataOptions,sitesFor} from './policy.js';
const equal = (a,b) => JSON.stringify(a) === JSON.stringify(b);
const PERMISSION_API = {location:'location',camera:'camera',microphone:'microphone',notifications:'notifications'};
const CLEANUP_RULE_START = 10000;
const NETWORK_RULE_START = 20000;
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
    const record={id:operationId,action,source,phase:'preview',createdAt:Date.now(),expiresAt:Date.now()+300000,preview:describe(action,this.config),permissions:requiredPermissions(action,this.config)};
    if (action.kind === 'clear') {
      record.preview.writerHandling=action.cookieStoreId ? '容器删除不注销共享 Service Worker；只保证所选 store 的浏览器 API 确认，无法验证后台本地回写停止。' : action.types.includes('serviceWorkers') ? '关闭范围内标签并注销目标 Service Worker；隔离保留到单独解除。' : '未选择 serviceWorkers，无法可靠停止后台本地写入；执行前须选择该类别。';
      if (!action.cookieStoreId && !action.types.includes('serviceWorkers')) fail('service_worker_quiescence_required',record.preview.writerHandling);
    }
    if (action.kind === 'restore') {
      const prior=await this.get(`operation:${action.receiptId}`);
      if (!prior?.undo || prior.restoredBy) fail('not_restorable');
      record.preview.restoring=prior.preview;
    }
    await this.set(`operation:${operationId}`,record);
    return record;
  }
  commit(id) {return this.serial(async()=>{
    const key=`operation:${id}`,r=await this.get(key);
    if (!r) fail('unknown_operation');
    if (r.phase !== 'preview') return r; // running/uncertain/completed never replay
    if (r.expiresAt < Date.now()) fail('preview_expired');
    if (r.source === 'app' && !(await this.get('pairing'))?.paired) fail('pairing_required');
    if (!await this.api.permissions.contains(r.permissions)) fail('permissions_missing');
    const running={...r,phase:'running',startedAt:Date.now()};
    await this.set(key,running); // durable boundary BEFORE any browser mutation
    try {
      const result=await this.execute(running);
      const done={...running,...result,phase:'completed',completedAt:Date.now()};
      await this.set(key,done);return done;
    } catch (error) {
      // API failure may follow a side effect. Keep uncertainty; never retry the ID.
      const saved=await this.get(key);
      const uncertain={...saved,phase:'uncertain',error:{code:error.code||'browser_api_error',message:error.message},completedAt:Date.now()};
      await this.set(key,uncertain);return uncertain;
    }
  });}
  async recover() {
    const all=await this.api.storage.local.get(null);
    for (const [key,r] of Object.entries(all)) if (key.startsWith('operation:') && r.phase==='running') await this.set(key,{...r,phase:'uncertain',error:{code:'worker_restarted',message:'执行期间扩展重启；请核对持久结果，不自动重复删除。'}});
  }
  async checkpoint(r,extra) {Object.assign(r,extra);await this.set(`operation:${r.id}`,r);}
  async execute(r) {
    const a=r.action;
    if (a.kind === 'clear') return this.clear(r);
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
      await this.set('networkRules',rules);
      return {result:{verification:'effective-readback',rules:await this.api.declarativeNetRequest.getDynamicRules()}};
    }
    if (a.kind === 'pauseRules') {
      const rules=(await this.api.declarativeNetRequest.getDynamicRules()).filter(v=>v.id>=NETWORK_RULE_START && v.id<NETWORK_RULE_START+100);
      if (!rules.length) fail('no_active_rules');
      const resumeAt=Date.now()+a.minutes*60000;
      await this.set('pausedRules',{rules,resumeAt});
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
    // Cookie deletion on Chromium expands to the registrable domain, including sibling tabs.
    if (this.config.browser!=='firefox' && a.types.includes('cookies')) rules.forEach((rule,i)=>{rule.condition.urlFilter=`||${sitesFor(a.origins,this.config)[i].domain}^`;});
    const existing=(await this.api.declarativeNetRequest.getDynamicRules()).filter(v=>v.id>=CLEANUP_RULE_START && v.id<CLEANUP_RULE_START+100);
    if (existing.length) fail('cleanup_isolation_active','请先核对上次结果并解除旧清理隔离。');
    await this.checkpoint(r,{isolationRuleIds:rules.map(v=>v.id)});
    await this.api.declarativeNetRequest.updateDynamicRules({addRules:rules});
    const sites=sitesFor(a.origins,this.config);
    const tabs=await this.api.tabs.query({});
    const targets=tabs.filter(tab=>{
      if (a.cookieStoreId && tab.cookieStoreId!==a.cookieStoreId) return false;
      try {const u=new URL(tab.url);return sites.some(s=>this.config.browser!=='firefox' && a.types.includes('cookies') ? u.hostname===s.domain || u.hostname.endsWith('.'+s.domain) : (this.config.browser==='firefox' ? u.hostname===new URL(s.origin).hostname : u.origin===s.origin));} catch {return false;}
    });
    await this.checkpoint(r,{closedTabCount:targets.length});
    if (targets.length) await this.api.tabs.remove(targets.map(t=>t.id));
    const options=dataOptions(a,this.config);
    // Unregister writers first, then clear stores; do not navigate to the sites for verification.
    if (a.types.includes('serviceWorkers')) await this.api.browsingData.remove(options,{serviceWorkers:true});
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
    return {result:{verification:'browser-acknowledged',categories:a.types,cookieObservation,closedTabCount:targets.length,
      writerHandling:a.cookieStoreId?'container-writers-not-verified':'tabs-closed-service-workers-unregistered',
      isolation:'active-until-explicit-release',storageEnumeration:'unavailable',remoteSessionRevocation:'not-performed',relogin:'not-started'}};
  }
  releaseIsolation(id) {return this.serial(async()=>{
    const r=await this.get(`operation:${id}`);
    if (!r?.isolationRuleIds || r.phase==='running') fail('no_releasable_isolation');
    await this.api.declarativeNetRequest.updateDynamicRules({removeRuleIds:r.isolationRuleIds});
    r.isolationReleasedAt=Date.now();await this.set(`operation:${id}`,r);return r;
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
    if (['webrtc','proxy'].includes(u.type)) {
      const setting=u.type==='webrtc' ? this.api.privacy.network.webRTCIPHandlingPolicy : this.api.proxy.settings;
      const current=await setting.get({});
      if (current.levelOfControl!=='controlled_by_this_extension' || !equal(current.value,u.applied)) fail('restore_conflict','当前值或控制者已变化，未覆盖。');
      if (u.before.levelOfControl==='controlled_by_this_extension') await setting.set({value:u.before.value,scope:'regular'});
      else await setting.clear({scope:'regular'}); // reveal current underlying setting, not stale snapshot
    } else if (u.type==='rules') {
      const current=(await this.api.declarativeNetRequest.getDynamicRules()).filter(v=>v.id>=NETWORK_RULE_START && v.id<NETWORK_RULE_START+100);
      if (!equal(current,u.applied) || await this.get('pausedRules')) fail('restore_conflict');
      await this.api.declarativeNetRequest.updateDynamicRules({removeRuleIds:current.map(v=>v.id),addRules:u.before});
      await this.set('networkRules',u.before);
    } else if (u.type==='content') {
      if (!equal(await this.get(`contentRules:${u.name}`),u.applied)) fail('restore_conflict');
      const api=this.api.contentSettings[u.name];
      for (const rule of u.applied) if ((await api.get({primaryUrl:rule.primaryPattern.slice(0,-2)})).setting!==rule.setting) fail('restore_conflict');
      await api.clear({scope:'regular'}); // clear only this extension's rules of this content type
      for (const rule of u.before) await api.set(rule);
      await this.set(`contentRules:${u.name}`,u.before);
    }
    prior.restoredBy=r.id;await this.set(`operation:${prior.id}`,prior);
    return {result:{verification:'browser-acknowledged',restored:r.action.receiptId}};
  }
  async resumeRules() {
    const paused=await this.get('pausedRules');
    if (!paused || paused.resumeAt>Date.now()) return;
    const current=(await this.api.declarativeNetRequest.getDynamicRules()).filter(v=>v.id>=NETWORK_RULE_START && v.id<NETWORK_RULE_START+100);
    if (current.length) {await this.set('ruleConflict','暂停期间规则已变化；未覆盖。');return;}
    await this.api.declarativeNetRequest.updateDynamicRules({addRules:paused.rules});
    await this.api.storage.local.remove('pausedRules');
  }
  async status() {
    const all=await this.api.storage.local.get(null);
    const webrtc=await this.api.privacy.network.webRTCIPHandlingPolicy.get({});
    return {browser:this.config.browser,synthetic:this.config.fixture,pairing:all.pairing || {paired:false},bridge:all.bridge || {connected:false},
      instanceId:all.identity?.instanceId,webrtc,permissions:await this.api.permissions.getAll(),
      receipts:Object.entries(all).filter(([k])=>k.startsWith('operation:')).map(([,v])=>v).sort((a,b)=>b.createdAt-a.createdAt),
      pausedRules:all.pausedRules,ruleConflict:all.ruleConflict,
      capabilities:{siteData:true,cacheStorage:this.config.browser!=='firefox',siteHttpCache:this.config.browser!=='firefox',containerStore:this.config.browser==='firefox',sitePermissions:this.config.browser!=='firefox',scopedProxy:this.config.browser!=='firefox'}};
  }
}
