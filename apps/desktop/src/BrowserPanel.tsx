import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { RequestError, transport } from './api';
import { Icon, Notice, formatDate } from './ui';

type Instance = { instance_id: string; label: string; browser: string; extension_id: string; paired: boolean; conflict: boolean; last_seen: number; online: boolean };
type Pairing = { challenge: string; code: string; expires_at: number };
type Pending = { challenge: string; code: string; instance_id: string; label: string; browser: string; extension_id: string };
type Operation = { instance_id: string; id: string; phase: string; action: { kind: string; setting?: string; receiptId?: string }; created_at: number; receipt?: { id: string; phase: string; result?: { verification?: string; configured?: string; effective?: string; controller?: string; scope?: string; restored?: string }; error?: { code: string; message: string } } };
async function browserRequest<T>(payload: Record<string, unknown>): Promise<T> {
  const result = await invoke<{ok:true;data:T}|{ok:false;error:{code:string;message:string}}>('browser_request', { payload });
  if (!result.ok) throw new RequestError(result.error.code, result.error.message);
  return result.data;
}
const phaseName = (phase: string) => ({ 'awaiting-browser-confirmation': '等待在浏览器中确认', running: '浏览器正在执行', completed: '浏览器已完成', uncertain: '结果待核对', failed: '未完成', cancelled: '已取消' } as Record<string,string>)[phase] ?? phase;
export default function BrowserPanel() {
  const [instances, setInstances] = useState<Instance[]>([]);
  const [pairing, setPairing] = useState<Pairing | null>(null);
  const [pending, setPending] = useState<Pending[]>([]);
  const [extensionId, setExtensionId] = useState('');
  const [operation, setOperation] = useState<Operation | null>(null);
  const [selectedInstance, setSelectedInstance] = useState('');
  const [setting, setSetting] = useState('disable_non_proxied_udp');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [note, setNote] = useState('');
  const [savedOperation, setSavedOperation] = useState<{id:string;instance_id:string}|null>(() => { try { return JSON.parse(localStorage.getItem('lintel.browser.operation') ?? 'null'); } catch { return null; } });
  async function refresh() {
    const [items, requests] = await Promise.all([browserRequest<Instance[]>({ op: 'instances' }), browserRequest<Pending[]>({ op: 'pair_pending' })]);
    setInstances(items); setPending(requests); setSelectedInstance(current => items.some(item => item.instance_id === current) ? current : items[0]?.instance_id ?? '');
  }
  async function run(work: () => Promise<void>) { setBusy(true); setError(''); setNote(''); try { await work(); } catch (error) { setError(error instanceof Error ? error.message : '浏览器模块请求未完成'); } finally { setBusy(false); } }
  useEffect(() => { if (transport === 'native') void run(refresh); }, []);
  async function query() { const target = operation ?? savedOperation; if (target) setOperation(await browserRequest<Operation>({ op: 'query', instance_id: target.instance_id, operation_id: target.id })); }
  async function submit(restore = false) {
    const instanceId = restore && operation ? operation.instance_id : selectedInstance;
    const id = crypto.randomUUID();
    const result = await browserRequest<Operation>({ op: 'submit', instance_id: instanceId, operation_id: id, action: restore ? { kind: 'restore', receiptId: operation!.id } : { kind: 'webrtc', setting } });
    const saved = { id, instance_id: instanceId }; setOperation(result); setSavedOperation(saved); localStorage.setItem('lintel.browser.operation', JSON.stringify(saved));
  }
  if (transport !== 'native') return <section className="browser-panel"><h3>浏览器伴随扩展</h3><Notice>配对与浏览器操作通过桌面 Native Messaging host 完成。此合成预览没有连接真实浏览器，不能执行配对或修改 profile。</Notice></section>;
  const selected = instances.find(item => item.instance_id === selectedInstance);
  return <section className="browser-panel"><div className="browser-panel-heading"><h3>浏览器伴随扩展</h3><button disabled={busy} onClick={() => void run(refresh)}><Icon name="refresh" size={14}/>检查配对</button></div><p>每个 profile 单独配对。开发候选仍需先安装扩展与 Native Messaging host；允许扩展 ID 本身不会建立连接。</p>
    {error && <div role="alert"><Notice tone="error">{error}</Notice></div>}{note && <div role="status"><Notice>{note}</Notice></div>}
    <details className="browser-setup"><summary>连接一个新的 profile</summary><label className="field">已安装扩展的 ID<input value={extensionId} onChange={event => setExtensionId(event.target.value)} spellCheck={false} placeholder="扩展管理页中的 ID"/></label><div className="button-row"><button disabled={busy || !extensionId.trim()} onClick={() => void run(async () => { await browserRequest({ op: 'allow_extension', extension_id: extensionId.trim() }); setNote('已允许此扩展 ID。请继续核对具体 profile 的配对短码。'); })}>允许此扩展 ID</button><button disabled={busy} onClick={() => void run(async () => { setPairing(await browserRequest<Pairing>({ op: 'pair_create' })); })}>生成配对请求</button></div>{pairing && <div className="pairing-code"><span>配对短码</span><strong>{pairing.code}</strong><p>在目标 profile 的扩展面板输入下方挑战值，核对两端短码。有效期至 {formatDate(String(pairing.expires_at))}。</p><textarea aria-label="配对挑战值" readOnly value={pairing.challenge}/><button className="text-button" onClick={() => void run(async () => { await navigator.clipboard.writeText(pairing.challenge); setNote('已复制配对挑战值'); })}><Icon name="copy" size={14}/>复制挑战值</button></div>}{pending.map(item => <div className="pending-pair" key={item.challenge}><strong>{item.label} · {item.browser}</strong><p>短码 {item.code} · 实例 {item.instance_id}</p><code>{item.extension_id}</code><button disabled={busy} onClick={() => void run(async () => { await browserRequest({ op: 'pair_approve', challenge: item.challenge }); setPairing(null); await refresh(); setNote('已批准配对。在线状态以实际收到的浏览器轮询为准。'); })}>短码一致，批准此 profile</button></div>)}</details>
    <div className="browser-instances">{instances.length === 0 ? <p className="empty-inline">尚无已配对的浏览器 profile。</p> : instances.map(item => <div className="browser-instance" key={item.instance_id}><div><strong>{item.label}</strong><span>{item.browser} · {item.paired ? '已配对' : '未配对'} · {item.online ? '最近 20 秒收到轮询' : '当前离线'}</span><code>{item.instance_id}</code></div>{item.conflict && <span className="conflict-label">身份冲突，需重新配对</span>}</div>)}</div>
    {instances.length > 0 && <div className="browser-actions"><h4>WebRTC profile 策略</h4><p>作用于整个选定 profile。限制非代理 UDP 可能影响通话；不是全设备 IP 隐藏或网络强约束。</p><label className="field">目标 profile<select value={selectedInstance} onChange={event => setSelectedInstance(event.target.value)}>{instances.map(item => <option key={item.instance_id} value={item.instance_id}>{item.label}</option>)}</select></label><label className="field">策略<select value={setting} onChange={event => setSetting(event.target.value)}><option value="disable_non_proxied_udp">限制非代理 UDP</option><option value="default_public_interface_only">仅默认公共接口</option><option value="default">浏览器默认</option></select></label><button disabled={busy || !selected?.paired || selected.conflict || !selected.online} onClick={() => void run(() => submit())}>提交到此 profile 确认<Icon name="arrow" size={14}/></button><p className="small-print">提交后请到扩展面板查看范围并批准。桌面提交不会直接宣称设置已生效。</p></div>}
    {(operation || savedOperation) && <div className="browser-operation"><div className="browser-panel-heading"><h4>{operation ? phaseName(operation.phase) : '有一份已提交任务'}</h4><button disabled={busy} onClick={() => void run(query)}>查询原任务</button></div><code>{operation?.id ?? savedOperation?.id}</code>{operation?.receipt?.result && <dl className="browser-result"><dt>验证方式</dt><dd>{operation.receipt.result.verification ?? '以浏览器返回结果为准'}</dd><dt>配置值</dt><dd>{operation.receipt.result.configured ?? '无此字段'}</dd><dt>有效值</dt><dd>{operation.receipt.result.effective ?? '无此字段'}</dd><dt>控制来源</dt><dd>{operation.receipt.result.controller ?? '无此字段'}</dd><dt>作用范围</dt><dd>{operation.receipt.result.scope ?? '无此字段'}</dd></dl>}{operation?.receipt?.error && <Notice tone="warning">{operation.receipt.error.message}（{operation.receipt.error.code}）</Notice>}{operation?.phase === 'completed' && operation.action.kind !== 'restore' && <button disabled={busy} onClick={() => void run(() => submit(true))}>提交恢复预览</button>}</div>}
  </section>;
}
