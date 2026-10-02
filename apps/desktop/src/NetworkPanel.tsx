import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { transport } from './api';
import { Icon, Notice, Status } from './ui';

type Connection = { timestamp_unix_ms: number; destination_host: string | null; destination_port: number | null; outcome: string; decision: string };
type Channel = { running: boolean; address: string | null; events: Connection[]; coverage: string; direct_connections_enforced: false; message?: string };
type Envelope<T> = { ok: true; data: T } | { ok: false; error: { code: string; message: string } };
async function network<T>(payload: object): Promise<T> {
  const result = await invoke<Envelope<T>>('network_request', { payload });
  if (!result.ok) throw new Error(result.error.message);
  return result.data;
}
export default function NetworkPanel({ environmentId, executable }: { environmentId: string; executable: boolean }) {
  const [channel, setChannel] = useState<Channel | null>(null);
  const [blocked, setBlocked] = useState('');
  const [upstream, setUpstream] = useState('');
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');
  const [error, setError] = useState('');
  useEffect(() => {
    let active = true;
    if (transport === 'native') network<Channel>({ op: 'status', environment_id: environmentId }).then(value => { if (active) setChannel(value); }).catch(err => { if (active) setError(String(err)); });
    return () => { active = false; };
  }, [environmentId]);
  async function act(op: 'start' | 'stop' | 'status' | 'launch') {
    setBusy(true); setError(''); setMessage('');
    try {
      const payload = { op, environment_id: environmentId, ...(op === 'start' ? { config: { default_action: 'allow', blocked: blocked.split(/\s+/).filter(Boolean).map(host => ({ host, ports: [] })), upstream: upstream.trim() || null } } : {}) };
      if (op === 'launch') { const result = await network<{ message: string }>(payload); setMessage(result.message); }
      else { const result = await network<Channel>(payload); setChannel(result); setMessage(result.message ?? ''); }
    } catch (err) { setError(err instanceof Error ? err.message : String(err)); } finally { setBusy(false); }
  }
  return <section className="network-panel"><div className="surface-heading"><h3>受控通道</h3><Status value={channel?.running ? 'available' : 'not_run'}/></div><div className="padded"><p>通过本地代理观察连接，按确切域名阻止外发。不解密 TLS，也不修改系统代理。</p>{transport !== 'native' ? <Notice>此测试空间未连接原生网络模块。请在桌面应用中启动真实通道。</Notice> : <><details className="network-options"><summary>通道规则与上游代理</summary><label className="field">阻止的域名<textarea rows={3} disabled={!!channel?.address} value={blocked} onChange={e => setBlocked(e.target.value)} placeholder="每行一个确切域名；留空只观察" spellCheck={false}/><small>每条规则适用于该域名的全部端口，不匹配子域。其余连接放行。混用 API 域名可能同时承载必要功能。</small></label><label className="field">上游 HTTP / HTTPS 代理（可选）<input disabled={!!channel?.address} value={upstream} onChange={e => setUpstream(e.target.value)} placeholder="http://127.0.0.1:7890" spellCheck={false}/><small>不支持 SOCKS 或含账号口令的代理 URL。改变规则前先停止通道。</small></label></details>{channel?.address && <div className="fact-row"><span>本地地址</span><code>http://{channel.address}</code></div>}<div className="button-row network-controls">{channel?.address ? <button disabled={busy} onClick={() => void act('stop')}>停止通道</button> : <button disabled={busy} onClick={() => void act('start')}>启动通道</button>}<button className="primary" disabled={busy || !channel?.running || !executable} onClick={() => void act('launch')}>通过通道打开 Claude<Icon name="arrow" size={15}/></button><button className="text-button" disabled={busy} onClick={() => void act('status')}>刷新连接</button></div>{channel?.events.length ? <div className="connection-log">{channel.events.slice(-12).reverse().map((event,index) => <div key={`${event.timestamp_unix_ms}-${index}`}><code>{event.destination_host ?? '未知目标'}{event.destination_port ? `:${event.destination_port}` : ''}</code><span>{event.outcome === 'blocked' ? '已阻止' : event.outcome}</span></div>)}</div> : <p className="small-print">暂无已记录的通道连接。</p>}</>}{message && <p role="status" className="network-message">{message}</p>}{error && <div role="alert"><Notice tone="error">{error}</Notice></div>}<p className="small-print">覆盖范围仅限经过此通道的连接。客户端可能绕过代理；没有进程级强约束。关闭 Lintel 会停止通道，已有客户端不会自动退出。</p></div></section>;
}
