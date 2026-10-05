import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { RequestError, transport } from './api';
import { Icon, Notice, formatDate } from './ui';
import Clawd from './Clawd';
import BrowserHostSetup from './BrowserHostSetup';
import { ResourceLink } from './Resources';
import RequestFailure, {asError} from './RequestFailure';

type Instance = { instance_id: string; label: string; browser: string; extension_id: string; paired: boolean; conflict: boolean; last_seen: number; online: boolean };
type SavedOperation = {id:string;instance_id:string;kind?:string};
function history(): SavedOperation[] { try { const saved = JSON.parse(localStorage.getItem('lintel.browser.history') ?? 'null'); const previous = JSON.parse(localStorage.getItem('lintel.browser.operation') ?? 'null'); return saved ?? (previous ? [previous] : []); } catch { return []; } }
type Pairing = { code: string; expires_at: number };
type Pending = { challenge: string; code: string; instance_id: string; label: string; browser: string; extension_id: string };
type Operation = { instance_id: string; id: string; phase: string; action: { kind: string; setting?: string; receiptId?: string }; created_at: number; receipt?: { id: string; phase: string; result?: Record<string, unknown>; error?: { code: string; message: string } } };
async function browserRequest<T>(payload: Record<string, unknown>): Promise<T> {
  const result = await invoke<{ok:true;data:T}|{ok:false;error:{code:string;message:string}}>('browser_request', { payload });
  if (!result.ok) throw new RequestError(result.error.code, result.error.message);
  return result.data;
}
const phaseName = (phase: string) => ({ 'awaiting-browser-confirmation': '等待在浏览器中确认', 'awaiting-browser-restart': '等待浏览器重启后继续', running: '浏览器正在执行', completed: '浏览器已完成', uncertain: '结果待核对', failed: '未完成', canceled: '已取消（未执行）', expired: '已过期（未执行）' } as Record<string,string>)[phase] ?? phase;
export default function BrowserPanel() {
  const [instances, setInstances] = useState<Instance[]>([]);
  const [pairing, setPairing] = useState<Pairing | null>(null);
  const [pending, setPending] = useState<Pending[]>([]);
  const [operation, setOperation] = useState<Operation | null>(null);
  const [selectedInstance, setSelectedInstance] = useState('');
  const [kind, setKind] = useState('clear');
  const [origins, setOrigins] = useState(['https://claude.ai']);
  const [types, setTypes] = useState(['cookies', 'localStorage', 'indexedDB', 'serviceWorkers']);
  const [permission, setPermission] = useState('location');
  const [port, setPort] = useState('8080');
  const [minutes, setMinutes] = useState('10');
  const [cookieStoreId, setCookieStoreId] = useState('');
  const [uncertain, setUncertain] = useState(false);
  const [setting, setSetting] = useState('disable_non_proxied_udp');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<Error|null>(null);
  const [note, setNote] = useState('');
  const feedback = useRef<HTMLDivElement>(null);
  const [operationHistory, setOperationHistory] = useState<SavedOperation[]>(history);
  const [savedOperation, setSavedOperation] = useState<{id:string;instance_id:string}|null>(() => { try { return JSON.parse(localStorage.getItem('lintel.browser.operation') ?? 'null'); } catch { return null; } });
  async function refresh() {
    const [items, requests] = await Promise.all([browserRequest<Instance[]>({ op: 'instances' }), browserRequest<Pending[]>({ op: 'pair_pending' })]);
    setInstances(items); setPending(requests); setSelectedInstance(current => items.some(item => item.instance_id === current) ? current : items[0]?.instance_id ?? '');
  }
  const actionLock=useRef(false);
  async function run(work: () => Promise<void>) { if(actionLock.current)return;actionLock.current=true;setBusy(true); setError(null); setNote(''); try { await work(); } catch (error) { setError(asError(error)); } finally { actionLock.current=false;setBusy(false); } }
  useEffect(() => { if (transport === 'native') void run(refresh); }, []);
  useEffect(() => { if (error || note) feedback.current?.scrollIntoView({ block: 'nearest' }); }, [error, note]);
  async function query(chosen?: SavedOperation) { const target = chosen ?? operation ?? savedOperation; if (target) { setSavedOperation(target); setOperation(await browserRequest<Operation>({ op: 'query', instance_id: target.instance_id, operation_id: target.id })); setUncertain(false); } }
  async function submit(restore = false, finishClear = false) {
    const instanceId = (restore || finishClear) && operation ? operation.instance_id : selectedInstance;
    const id = crypto.randomUUID();
    const receiptId = operation?.id;
    const saved = { id, instance_id: instanceId, kind:finishClear ? 'finishClear' : restore ? 'restore' : kind }; setSavedOperation(saved); localStorage.setItem('lintel.browser.operation', JSON.stringify(saved)); const records=[saved,...operationHistory]; setOperationHistory(records); localStorage.setItem('lintel.browser.history',JSON.stringify(records)); setOperation(null); setUncertain(true);
    const action = finishClear ? { kind:'finishClear', receiptId } : restore ? { kind: 'restore', receiptId } : kind === 'clear' ? { kind, origins, types, ...(cookieStoreId.trim() && firefox ? { cookieStoreId: cookieStoreId.trim() } : {}) } : kind === 'webrtc' ? { kind, setting } : kind === 'sitePermission' ? { kind, origins, setting: permission } : kind === 'proxy' ? { kind, origins, port: Number(port) } : kind === 'blockSites' ? { kind, origins } : kind === 'pauseRules' ? { kind, minutes: Number(minutes) } : { kind };
    const result = await browserRequest<Operation>({ op: 'submit', instance_id: instanceId, operation_id: id, action }); setOperation(result); setUncertain(false); setNote('已提交给目标 profile。请在浏览器扩展中核对并批准。');
  }
  const selected = instances.find(item => item.instance_id === selectedInstance);
  const native = transport === 'native';
  const firefox = !!selected?.browser.toLowerCase().includes('firefox');
  const needsOrigins = ['clear','sitePermission','proxy','blockSites'].includes(kind);
  const awaiting = uncertain || (!!savedOperation && (!operation || !['completed','failed','canceled','expired','rejected'].includes(operation.phase)));
  const unsupported = firefox && ['sitePermission','proxy'].includes(kind);
  const invalid = (needsOrigins && !origins.length) || (kind === 'clear' && (!types.length || (firefox && types.some(type => ['cacheStorage','cache'].includes(type))))) || (kind === 'proxy' && (!Number.isInteger(Number(port)) || Number(port) < 1 || Number(port) > 65535)) || (kind === 'pauseRules' && (!Number.isInteger(Number(minutes)) || Number(minutes) < 1 || Number(minutes) > 60));
  return <section className="browser-panel"><div className="browser-local-scope"><Icon name="terminal" size={16}/><strong>本机浏览器 · 精确 profile</strong><span>此工作空间始终属于当前 Mac；首页 SSH 选择不改变它的作用对象。</span></div><div className="module-intro"><Clawd small mood="care"/><div><h3>一份 profile，一份明确的授权</h3><p>桌面准备请求；目标扩展展示实际范围，再由你确认。</p></div></div>{!native && <Notice>合成测试空间没有连接真实浏览器。下面展示可用操作；配对与执行只在桌面应用中启用。</Notice>}<div className="browser-panel-heading"><h3>浏览器伴随扩展</h3><button disabled={busy || !native} onClick={() => void run(refresh)}><Icon name="refresh" size={14}/>检查配对</button></div><p>每个 profile 单独配对。先在目标 profile 安装伴随扩展，再由 App 安装本地连接组件。批准安装会授权这个精确扩展 ID；profile 短码需要另行核对。</p>
    {error && <div ref={feedback}><RequestFailure error={error} context="浏览器 profile"/></div>}{note && <div ref={feedback} role="status"><Notice>{note}</Notice></div>}
    <details className="browser-setup"><summary>连接一个新的 profile</summary><p className="small-print">App 已包含扩展与本地连接组件。按下面三步连接目标 profile；各浏览器的开发加载限制见 <ResourceLink resource="browser-setup">浏览器指南</ResourceLink>（当前需仓库权限）。</p><BrowserHostSetup native={native} busy={busy} request={browserRequest} run={run} onInstalled={setNote}/><h4 className="browser-setup-step browser-pairing-heading"><span>3</span>核对这份 profile</h4><div className="button-row section-action"><button disabled={busy || !native} onClick={() => void run(async () => { setPairing(await browserRequest<Pairing>({ op: 'pair_create' })); })}>生成配对请求</button></div>{pairing && <div className="pairing-code"><span>配对短码</span><strong>{pairing.code}</strong><p>在目标 profile 的扩展面板输入这份 12 位短码，提交请求后核对两端短码，再在这里批准。有效期至 {formatDate(String(pairing.expires_at))}。</p><button className="text-button" onClick={() => void run(async () => { await navigator.clipboard.writeText(pairing.code); setNote('已复制配对短码'); })}><Icon name="copy" size={14}/>复制配对短码</button></div>}{pending.map(item => <div className="pending-pair" key={item.challenge}><strong>{item.label} · {item.browser}</strong><p>短码 {item.code} · 实例 {item.instance_id}</p><code>{item.extension_id}</code><button disabled={busy || !native} onClick={() => void run(async () => { await browserRequest({ op: 'pair_approve', challenge: item.challenge }); setPairing(null); await refresh(); setNote('已批准配对。在线状态以实际收到的浏览器轮询为准。'); })}>短码一致，批准此 profile</button></div>)}</details>
    <div className="browser-instances">{instances.length === 0 ? <p className="empty-inline">尚无已配对的浏览器 profile。</p> : instances.map(item => <div className="browser-instance" key={item.instance_id}><div><strong>{item.label}</strong><span>{item.browser} · {item.paired ? '已配对' : '未配对'} · {item.online ? '最近 20 秒收到轮询' : '当前离线'}</span><code>{item.instance_id}</code></div>{item.conflict && <span className="conflict-label">身份冲突，需重新配对</span>}</div>)}</div>
    <div className="browser-actions"><label className="field">目标 profile<select disabled={!instances.length || busy} value={selectedInstance} onChange={event => setSelectedInstance(event.target.value)}>{!instances.length && <option value="">等待连接浏览器 profile</option>}{instances.map(item => <option key={item.instance_id} value={item.instance_id}>{item.label} · {item.browser}</option>)}</select></label>
      <div className="browser-kind-picker" role="group" aria-label="浏览器操作">{[['clear','清理站点数据'],['webrtc','WebRTC'],['sitePermission','站点权限'],['proxy','站点代理'],['blockSites','阻断站点'],['pauseRules','暂停规则'],['clearProfileCache','全 profile 缓存']].map(([value,title]) => <button aria-pressed={kind === value} key={value} onClick={() => setKind(value)}>{title}</button>)}</div>
      {needsOrigins && <fieldset className="compact-options"><legend>目标站点</legend>{['https://claude.ai','https://console.anthropic.com'].map(origin => <label key={origin}><input type="checkbox" checked={origins.includes(origin)} onChange={event => setOrigins(current => event.target.checked ? [...current,origin] : current.filter(item => item !== origin))}/><code>{origin}</code></label>)}</fieldset>}
      {kind === 'clear' && <><fieldset className="compact-options"><legend>将清理的数据类别</legend>{[['cookies','Cookie / 登录'],['localStorage','Local Storage'],['indexedDB','IndexedDB'],['serviceWorkers','Service Worker'],['cacheStorage','Cache Storage'],['cache','HTTP cache']].map(([id,title]) => <label key={id}><input type="checkbox" checked={types.includes(id)} disabled={firefox && ['cacheStorage','cache'].includes(id)} onChange={event => setTypes(current => event.target.checked ? [...current,id] : current.filter(item => item !== id))}/>{title}{firefox && ['cacheStorage','cache'].includes(id) ? ' · 不支持站点清理' : ''}</label>)}</fieldset>{firefox && <label className="field">Firefox store（可选）<input value={cookieStoreId} onChange={event => setCookieStoreId(event.target.value)} placeholder="firefox-default 或 firefox-container-1"/><small>留空处理所选主机全部 store；Service Worker 无法按 store 单独注销。</small></label>}<Notice tone="warning">删除 Cookie 会退出相关登录，数据清理不可恢复。Chromium 可能扩大到完整可注册域；邮箱与第三方 SSO 不在清单中。先隔离站点、关闭相关页面并注销 Service Worker；重启目标浏览器后需第二次批准，才删除其余存储。完成后的隔离仍需另行解除。</Notice></>}
      {kind === 'webrtc' && <><p>整个 profile 的 WebRTC 策略，可能影响通话；不代表设备级 IP 隐藏。</p><label className="field">策略<select value={setting} onChange={event => setSetting(event.target.value)}><option value="disable_non_proxied_udp">限制非代理 UDP</option><option value="default_public_interface_only">仅默认公共接口</option><option value="default">浏览器默认</option></select></label></>}
      {kind === 'sitePermission' && <label className="field">设为阻止的权限<select value={permission} onChange={event => setPermission(event.target.value)}>{[['location','定位'],['camera','摄像头'],['microphone','麦克风'],['notifications','通知']].map(([value,title]) => <option key={value} value={value}>{title}</option>)}</select></label>}
      {kind === 'proxy' && <label className="field">127.0.0.1 上的 HTTP 代理端口<input type="number" min={1} max={65535} value={port} onChange={event => setPort(event.target.value)}/><small>先启动本机代理。仅所选站点，不使用 DIRECT 回退，不覆盖 WebRTC、DNS 或其他应用。</small></label>}
      {kind === 'blockSites' && <Notice tone="warning">会阻止选定站点的全部请求，影响该站点所有功能；不是仅关闭遥测。</Notice>}
      {kind === 'pauseRules' && <label className="field">暂停分钟数<input type="number" min={1} max={60} value={minutes} onChange={event => setMinutes(event.target.value)}/><small>1–60 分钟，只暂停本扩展阻断规则；不解除清理隔离。离线到期时，下次浏览器启动恢复规则。</small></label>}
      {kind === 'clearProfileCache' && <Notice tone="warning">范围是整个选定 profile 的 HTTP cache，包含其他站点。这个独立动作不可恢复，不会称为站点级清理。</Notice>}
      {unsupported && <Notice>Firefox 暂无此原生适配。站点权限请到浏览器中设置；不会静默换成整个 profile 代理。</Notice>}
      {awaiting && <Notice>先查询上一份任务。回复丢失或未完成时，不生成新 ID 重新操作。</Notice>}
      <button className="primary section-action" disabled={busy || !native || !selected?.paired || selected.conflict || !selected.online || unsupported || invalid || awaiting} onClick={() => void run(() => submit())}>送到浏览器预览<Icon name="arrow" size={14}/></button><p className="small-print">请在目标扩展核对范围并批准。桌面提交不表示操作已完成。</p>
    </div>
    {(operation || savedOperation) && <div className="browser-operation"><div className="browser-panel-heading"><h4>{operation ? phaseName(operation.phase) : '有一份已提交任务'}</h4><button disabled={busy || !native} onClick={() => void run(() => query())}>查询原任务</button></div><code>{operation?.id ?? savedOperation?.id}</code>{operationHistory.length > 1 && <label className="field">其他浏览器任务<select value={savedOperation?.id ?? operation?.id} disabled={busy || !native} onChange={event => { const record=operationHistory.find(item=>item.id===event.target.value)!; setOperation(null); void run(() => query(record)); }}>{operationHistory.map(item=><option key={item.id} value={item.id}>{item.kind ?? '操作'} · {item.id}</option>)}</select></label>}{operation?.receipt?.result && <dl className="browser-result">{Object.entries(operation.receipt.result).map(([key,value]) => <div className="browser-result-field" key={key}><dt>{({verification:'验证方式',configured:'配置值',effective:'有效值',controller:'控制来源',scope:'作用范围',restored:'恢复结果'} as Record<string,string>)[key] ?? key}</dt><dd>{typeof value === 'string' ? value : JSON.stringify(value)}</dd></div>)}</dl>}{operation?.receipt?.error && <Notice tone="warning">{operation.receipt.error.message}（{operation.receipt.error.code}）</Notice>}{operation?.phase === 'awaiting-browser-restart' && <Notice tone="warning">准备步骤已完成，其余存储尚未删除。请重启这个浏览器，再到扩展预览并批准继续删除；确认后会把这个原任务更新为最终结果。<button className="section-action" disabled={busy || !native} onClick={() => void run(() => submit(false, true))}>送到浏览器预览重启后继续</button></Notice>}{operation?.phase === 'completed' && ['webrtc','sitePermission','proxy','blockSites'].includes(operation.action.kind) && <button disabled={busy || !native} onClick={() => void run(() => submit(true))}>提交恢复预览</button>}</div>}
  </section>;
}
