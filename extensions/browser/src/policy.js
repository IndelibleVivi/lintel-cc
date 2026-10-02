import {CONFIG} from './config.js';
export const TYPES = ['cookies','localStorage','indexedDB','serviceWorkers','cacheStorage','cache'];
export function fail(code, detail) { const e = new Error(detail || code); e.code = code; throw e; }
export function exactKeys(value, allowed) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) fail('invalid_request');
  for (const key of Object.keys(value)) if (!allowed.includes(key)) fail('unknown_field',key);
}
export function sitesFor(origins, config = CONFIG) {
  if (!Array.isArray(origins) || !origins.length || origins.length > config.sites.length || new Set(origins).size !== origins.length) fail('invalid_origins');
  return origins.map(origin => config.sites.find(s => s.origin === origin) || fail('site_not_allowlisted',origin));
}
export function normalizeAction(input, config = CONFIG) {
  exactKeys(input,['kind','origins','types','cookieStoreId','setting','port','receiptId','minutes']);
  const a = structuredClone(input);
  switch (a.kind) {
    case 'clear':
      sitesFor(a.origins,config);
      if (!Array.isArray(a.types) || !a.types.length || new Set(a.types).size !== a.types.length || a.types.some(t => !TYPES.includes(t))) fail('invalid_data_types');
      if (config.browser === 'firefox') {
        if (a.types.includes('cache')) fail('firefox_site_cache_unsupported','Firefox 不能按站点清理 HTTP cache。请单独预览整个 profile 缓存。');
        if (a.types.includes('cacheStorage')) fail('firefox_cache_storage_unsupported','Firefox browsingData 没有独立的按站点 CacheStorage 清理接口；此类别尚未交付。');
        if (a.cookieStoreId && a.types.some(t => !['cookies','localStorage','indexedDB'].includes(t))) fail('container_scope_unsupported','容器限定仅支持 Cookie、localStorage 和 IndexedDB。');
        if (a.cookieStoreId && !/^firefox-(default|container-\d+)$/.test(a.cookieStoreId)) fail('invalid_cookie_store');
      } else if (a.cookieStoreId) fail('cookie_store_unsupported');
      break;
    case 'clearProfileCache': break;
    case 'webrtc':
      if (!['default','default_public_interface_only','disable_non_proxied_udp'].includes(a.setting)) fail('invalid_webrtc_policy');
      break;
    case 'sitePermission':
      sitesFor(a.origins,config);
      if (!['location','camera','microphone','notifications'].includes(a.setting)) fail('unsupported_permission');
      if (config.browser === 'firefox') fail('firefox_site_permission_unsupported','Firefox 不提供 Chromium contentSettings；请使用浏览器站点权限面板。');
      break;
    case 'proxy':
      sitesFor(a.origins,config);
      if (!Number.isInteger(a.port) || a.port < 1024 || a.port > 65535) fail('invalid_proxy_port');
      if (config.browser === 'firefox') fail('firefox_scoped_proxy_unsupported','Firefox 按站点代理适配尚未交付；不会改成全 profile 代理。');
      break;
    case 'blockSites': sitesFor(a.origins,config); break;
    case 'pauseRules':
      if (!Number.isInteger(a.minutes) || a.minutes < 1 || a.minutes > 60) fail('invalid_pause_duration');
      break;
    case 'finishClear':
    case 'restore':
      if (typeof a.receiptId !== 'string' || !/^[A-Za-z0-9_-]{8,80}$/.test(a.receiptId)) fail('invalid_receipt_id');
      break;
    default: fail('unknown_action');
  }
  // Reject fields which have no meaning for the selected action.
  const fields = {clear:['origins','types','cookieStoreId'],finishClear:['receiptId'],clearProfileCache:[],webrtc:['setting'],sitePermission:['origins','setting'],proxy:['origins','port'],blockSites:['origins'],pauseRules:['minutes'],restore:['receiptId']}[a.kind];
  for (const key of Object.keys(a)) if (key !== 'kind' && !fields.includes(key)) fail('irrelevant_field',key);
  return a;
}
export function describe(a,config = CONFIG) {
  if (a.kind === 'clear') {
    const sites = sitesFor(a.origins,config);
    return {scope:'current-profile',requestedOrigins:a.origins,effectiveStorageScope:config.browser === 'firefox' ? sites.map(s=>new URL(s.origin).hostname) : a.origins,
      effectiveCookieScope:a.types.includes('cookies') ? sites.map(s=>config.browser === 'firefox' ? new URL(s.origin).hostname : s.domain) : [],
      isolationScope:'目标主机的新请求暂时阻止；DNR 作用于整个当前 profile（含其他 Firefox 容器），仅关闭选定 store 的标签。',cookieStoreId:a.cookieStoreId || 'all-stores',types:a.types,irreversible:true,
      impact:'先关闭目标标签及嵌入目标 iframe 的宿主标签并注销 Service Worker。已运行的 worker 事件仍可能回写，因此保留隔离，要求用户重启此浏览器后再次预览确认删除。webNavigation 仅用于 frame 匹配，不保存浏览记录。不会自动重启或删除；Cookie 删除会退出登录。',
      observation:'browser-acknowledged；有可选 cookies 权限时只返回剩余数量；没有通用存储枚举能力。'};
  }
  const descriptions = {
    clearProfileCache:'不可撤销：清除整个当前 profile 的 HTTP cache，包括无关站点。不会把它计为站点级清理。',
    webrtc:'整个当前 profile 的 WebRTC 策略；可能影响通话和媒体连接，不代表设备无 IP 泄漏。',
    sitePermission:'仅选定站点的权限改为阻止；读取有效值。API 不提供完整控制者身份。',
    proxy:'仅所选站点 HTTPS/HTTP 请求使用本机 HTTP proxy；其他站点直连。未覆盖 WebRTC、DNS 或全部后台连接；代理未启动会使目标请求失败。',
    blockSites:'阻止所选站点及子域的请求；所有该站点功能都会受影响，不将此规则称为遥测分类。',
    pauseRules:'限时暂停本扩展的持久站点阻断规则；不自动解除清理任务的隔离。',
    restore:'只恢复仍与本扩展写入值一致且仍归本扩展控制的设置；删除的数据不可恢复。',
    finishClear:'浏览器已在准备后重新启动；再次确认原范围后删除数据。隔离保留到单独解除，旧任务 ID 不会再次删除。'
  };
  return {scope:a.kind === 'webrtc' || a.kind === 'clearProfileCache' ? 'current-profile' : 'selected-sites',...a,impact:descriptions[a.kind]};
}
export function hostPermissions(a,config=CONFIG) {
  if (!a.origins) return [];
  return [...new Set(sitesFor(a.origins,config).flatMap(s=> {
    const u = new URL(s.origin);
    return config.fixture ? [`${u.protocol}//${u.hostname}/*`] : [`https://${s.domain}/*`,`https://*.${s.domain}/*`];
  }))];
}
export function requiredPermissions(a,config=CONFIG) {
  const permissions=[];
  if (['clear','blockSites','pauseRules'].includes(a.kind)) permissions.push('declarativeNetRequest');
  if (a.kind === 'clear') permissions.push('webNavigation');
  if (a.kind === 'clear' && a.cookieStoreId) permissions.push('cookies');
  if (a.kind === 'sitePermission') permissions.push('contentSettings');
  if (a.kind === 'proxy') permissions.push('proxy');
  return {permissions,origins:hostPermissions(a,config)};
}
export function dataOptions(a,config=CONFIG) {
  if (a.kind === 'clearProfileCache') return {};
  if (config.browser === 'firefox') return {hostnames:sitesFor(a.origins,config).map(s=>new URL(s.origin).hostname),...(a.cookieStoreId ? {cookieStoreId:a.cookieStoreId} : {})};
  return {origins:a.origins};
}
