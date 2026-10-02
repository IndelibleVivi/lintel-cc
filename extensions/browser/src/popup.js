import {CONFIG} from './config.js';
import {TYPES} from './policy.js';
const api=globalThis.browser||globalThis.chrome;
const $=id=>document.getElementById(id);
let selected;
const names={cookies:'Cookie / 登录',localStorage:'localStorage',indexedDB:'IndexedDB',serviceWorkers:'Service Worker',cacheStorage:'Cache Storage',cache:'HTTP cache'};
function checkbox(value,label,checked,parent){const l=document.createElement('label');l.className='check';const c=document.createElement('input');c.type='checkbox';c.value=value;c.checked=checked;l.append(c,document.createTextNode(label));$(parent).append(l);}
for(const s of CONFIG.sites)checkbox(s.origin,s.label,s.default,'sites');
for(const t of TYPES)checkbox(t,names[t],t!=='cache' && !(CONFIG.browser==='firefox'&&t==='cacheStorage'),'types');
$('synthetic').hidden=!CONFIG.fixture;$('container-wrap').hidden=CONFIG.browser!=='firefox';$('chromium-controls').hidden=CONFIG.browser==='firefox';$('firefox-limit').hidden=CONFIG.browser!=='firefox';
const origins=()=>[...$('sites').querySelectorAll('input:checked')].map(c=>c.value);
async function send(type,values={}){const r=await api.runtime.sendMessage({type,...values});if(!r?.ok)throw new Error(`${r?.error?.code}: ${r?.error?.message}`);return r.data;}
function run(fn){return async event=>{event?.preventDefault();$('error').textContent='';try{await fn();}catch(e){$('error').textContent=e.message;}};}
function showPreview(r){selected=r;$('preview-text').textContent=JSON.stringify(r.preview,null,2);$('preview-panel').hidden=false;$('confirm').checked=false;$('commit').disabled=true;$('preview-panel').scrollIntoView({behavior:'instant'});}
async function preview(action){showPreview(await send('preview',{action}));}
async function refresh(){const s=await send('status');$('pair-status').textContent=s.pairing.paired?'已配对 · 桌面连接有效':`未连接 / 未配对 · ${s.bridge.reason||s.pairing.reason||'可使用本地独立操作'}`;$('instance').textContent=`实例 ${s.instanceId}`;$('webrtc-status').textContent=`有效值：${s.webrtc.value} · 控制权：${s.webrtc.levelOfControl}`;$('receipts').replaceChildren();for(const r of s.receipts){const box=document.createElement('div');box.className='receipt';const strong=document.createElement('strong');strong.textContent=`${r.action.kind} · ${r.phase}`;box.append(strong);const small=document.createElement('small');small.textContent=r.id;box.append(small);const details=document.createElement('details'),summary=document.createElement('summary'),pre=document.createElement('pre');summary.textContent='查看范围与结果';pre.textContent=JSON.stringify({preview:r.preview,result:r.result,error:r.error},null,2);details.append(summary,pre);box.append(details);const button=(text,fn)=>{const b=document.createElement('button');b.className='quiet';b.textContent=text;b.onclick=run(fn);box.append(b);};if(r.phase==='preview')button(r.source==='app'?'查看来自桌面的待确认操作':'重新查看预览',()=>showPreview(r));if(r.undo&&!r.restoredBy&&r.phase!=='running')button('预览恢复此项设置',()=>preview({kind:'restore',receiptId:r.id}));if(r.isolationRuleIds&&!r.isolationReleasedAt&&r.phase!=='running')button('已核对结果，解除站点隔离',async()=>{await send('releaseIsolation',{id:r.id});await refresh();});$('receipts').append(box);}}
$('refresh').onclick=run(async()=>{await send('refreshPairing');await refresh();});
$('pair-form').onsubmit=run(async()=>{const r=await send('pair',{code:$('code').value,label:$('label').value});$('pair-status').textContent=`请求已提交 · 请在桌面批准短码 ${r.code}`;});
$('reset-identity').onclick=run(async()=>{await send('resetIdentity');await refresh();});
$('preview-clear').onclick=run(()=>preview({kind:'clear',origins:origins(),types:[...$('types').querySelectorAll('input:checked')].map(c=>c.value),...($('store').value?{cookieStoreId:$('store').value.trim()}:{})}));
$('profile-cache').onclick=run(()=>preview({kind:'clearProfileCache'}));
$('preview-webrtc').onclick=run(()=>preview({kind:'webrtc',setting:$('webrtc').value}));
$('preview-permission').onclick=run(()=>preview({kind:'sitePermission',origins:origins(),setting:$('site-permission').value}));
$('preview-proxy').onclick=run(()=>preview({kind:'proxy',origins:origins(),port:Number($('proxy-port').value)}));
$('preview-block').onclick=run(()=>preview({kind:'blockSites',origins:origins()}));
$('pause').onclick=run(()=>preview({kind:'pauseRules',minutes:10}));
$('confirm').onchange=()=>{$('commit').disabled=!$('confirm').checked;};
$('commit').onclick=run(async()=>{
  if(!selected||!$('confirm').checked)return;
  const r=selected;$('commit').disabled=true;
  // Must be the first asynchronous browser call in the user gesture for Firefox.
  const granted=await api.permissions.request(r.permissions);
  if(!granted)throw new Error('必要权限未授予；未执行。');
  const result=await send('commit',{id:r.id});$('preview-panel').hidden=true;selected=undefined;await refresh();
  if(result.phase==='uncertain')throw new Error(`结果需核对：${result.error.message}`);
});
$('cancel').onclick=()=>{$('preview-panel').hidden=true;selected=undefined;};
await run(refresh)();
