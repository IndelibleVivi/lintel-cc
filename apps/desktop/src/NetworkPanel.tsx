import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { transport } from './api';
import { Icon, Notice, Status } from './ui';
import IpNetworkPanel from './IpNetworkPanel';
import TelemetryPanel from './TelemetryPanel';
import telemetryCatalog from '../../../contracts/telemetry-destinations.json';
import type { Inspection, Plan, Receipt } from './types';

type Rule = { host: string; ports: number[] };
type Config = {
  environment_id: string; bind: string; default_action: 'allow' | 'deny';
  blocked: Rule[]; allowed: Rule[]; upstream: string | null;
  address_family: 'system' | 'ipv4_only';
  max_connections: number; connect_timeout_seconds: number; connection_lifetime_seconds: number;
};
type Connection = { timestamp_unix_ms: number; destination_host: string | null; destination_port: number | null; outcome: string; decision: string; peer_family?: string; origin?: 'rule_test' | null; test_id?: string | null };
type Channel = { running: boolean; address: string | null; channel_binding?: string; active_config: Config | null; events: Connection[]; coverage: string; direct_connections_enforced: false; message?: string };
type Envelope<T> = { ok: true; data: T } | { ok: false; error: { code: string; message: string } };
type Draft = { defaultAction: Config['default_action']; blocked: string; allowed: string; upstream: string; addressFamily: Config['address_family']; limits?: Pick<Config, 'max_connections' | 'connect_timeout_seconds' | 'connection_lifetime_seconds'>; telemetry?: string[] };
// Keep edits for each environment while this webview is open, without persisting connection details.
const drafts = new Map<string, Draft>();
const emptyDraft = (): Draft => ({ defaultAction: 'allow', blocked: '', allowed: '', upstream: '', addressFamily: 'system', telemetry: [] });
const TELEMETRY_RULES = telemetryCatalog.destinations.filter(entry => entry.blockable);
// Merge selected telemetry destinations into the blocked draft text without
// dropping any user rule and without duplicating a host already present.
function withTelemetryRules(blockedText: string, ids: string[]): string {
  const lines = blockedText.split('\n').map(line => line.trim()).filter(Boolean);
  const rules = parseRules(blockedText);
  for (const id of ids) {
    const entry = TELEMETRY_RULES.find(value => value.id === id);
    if (entry && !rules.some(rule => rule.host.toLowerCase().replace(/\.$/, '') === entry.host && (!rule.ports.length || rule.ports.includes(entry.port)))) {
      lines.push(`${entry.host} ${entry.port}`);
      rules.push({host: entry.host, ports: [entry.port]});
    }
  }
  return lines.join('\n');
}
const ruleText = (rules: Rule[]) => rules.map(rule => `${rule.host}${rule.ports.length ? ` ${rule.ports.join(',')}` : ''}`).join('\n');
function fromConfig(config: Config): Draft {
  return {
    defaultAction: config.default_action, blocked: ruleText(config.blocked), allowed: ruleText(config.allowed), upstream: config.upstream ?? '', addressFamily: config.address_family ?? 'system',
    limits: { max_connections: config.max_connections, connect_timeout_seconds: config.connect_timeout_seconds, connection_lifetime_seconds: config.connection_lifetime_seconds },
  };
}
function parseRules(value: string): Rule[] {
  return value.split('\n').map(line => line.trim()).filter(Boolean).map(line => {
    const [host, portsText, extra] = line.split(/\s+/);
    if (extra || (portsText && !/^\d+(,\d+)*$/.test(portsText))) throw new Error('每行填写主机名，可在空格后指定端口，例如 example.invalid 443,8443。');
    const ports = portsText ? portsText.split(',').map(Number) : [];
    if (ports.some(port => !Number.isInteger(port) || port < 1 || port > 65535)) throw new Error('规则端口必须为 1–65535。');
    return { host, ports };
  });
}
async function network<T>(payload: object): Promise<T> {
  const result = await invoke<Envelope<T>>('network_request', { payload });
  if (!result.ok) throw new Error(result.error.message);
  return result.data;
}
function RuleReadback({ rules }: { rules: Rule[] }) {
  return rules.length ? <ul className="compact-list">{rules.map((rule, index) => <li key={index}><code>{rule.host}</code> · {rule.ports.length ? `端口 ${rule.ports.join('、')}` : '全部端口'}</li>)}</ul> : <span>无</span>;
}
// The key owns the whole request lifetime, so even start/stop replies from the
// previous environment cannot update the newly selected environment's panel.
type Props = { environmentId: string; executable: boolean; onLaunch:(proxyUrl:string)=>void; hostAlias:string|null; receipts:Receipt[]; disabled:boolean; onPlan:(plan:Plan)=>void;onReceipt:(receipt:Receipt)=>void; settings?:Inspection['settings']; onPrivacySettings?:()=>void };
export default function NetworkPanel(props: Props) {
  return (props.hostAlias || !props.environmentId) ? <><IpNetworkPanel key={`${props.hostAlias}:${props.environmentId}`} {...props} proxyUrl={null} proxyBinding={null}/><Notice>此上下文只读遥测目录；受控通道需在本地明确选择一个环境。</Notice><TelemetryPanel key={`telemetry:${props.hostAlias}:${props.environmentId}`} hostAlias={props.hostAlias} selected={[]} onChange={()=>{}} disabled settings={props.settings} onPrivacySettings={props.onPrivacySettings}/></> : <EnvironmentNetworkPanel key={props.environmentId} {...props}/>;
}
function EnvironmentNetworkPanel({ environmentId, executable,onLaunch, ...props }: Props) {
  const [channel, setChannel] = useState<Channel | null>(null);
  const [draft, setDraft] = useState<Draft>(() => drafts.get(environmentId) ?? emptyDraft());
  const [busy, setBusy] = useState(transport === 'native');
  const [message, setMessage] = useState('');
  const [error, setError] = useState('');
  const mounted = useRef(false);
  function updateDraft(value: Partial<Draft>) {
    setDraft(current => { const next = { ...current, ...value }; drafts.set(environmentId, next); return next; });
  }
  function receive(value: Channel) {
    setChannel(value); setMessage(value.message ?? '');
    if (value.active_config) {
      const adopted = fromConfig(value.active_config);
      drafts.set(environmentId, adopted); setDraft(adopted);
    }
  }
  useEffect(() => {
    mounted.current = true;
    let active = true;
    if (transport === 'native') network<Channel>({ op: 'status', environment_id: environmentId })
      .then(value => { if (active) receive(value); })
      .catch(err => { if (active) setError(err instanceof Error ? err.message : String(err)); })
      .finally(() => { if (active) setBusy(false); });
    return () => { active = false; mounted.current = false; };
  }, [environmentId]);
  async function act(op: 'start' | 'stop' | 'status') {
    setBusy(true); setError(''); setMessage('');
    try {
      const payload = { op, environment_id: environmentId, ...(op === 'start' ? { config: { ...draft.limits, default_action: draft.defaultAction, blocked: parseRules(withTelemetryRules(draft.blocked, draft.telemetry ?? [])), allowed: parseRules(draft.allowed), upstream: draft.upstream.trim() || null, address_family: draft.addressFamily } } : {}) };
      const result = await network<Channel>(payload); if (mounted.current) receive(result);
    } catch (err) { if (mounted.current) setError(err instanceof Error ? err.message : String(err)); }
    finally { if (mounted.current) setBusy(false); }
  }
  // Real controlled test on the current running channel. The native op returns
  // the verified outcome; a failure is surfaced, never treated as a pass.
  async function ruleTest(id: string, binding: string) {
    const result = await network<{ outcome: { result: string; explicit_block: boolean; connection_attempted: boolean; provenance: string; destination_host: string; destination_port: number; test_id: string } }>({ op: 'rule_test', environment_id: environmentId, channel_binding: binding, telemetry_id: id });
    const status = await network<Channel>({ op: 'status', environment_id: environmentId });
    if (mounted.current) receive(status);
    return result.outcome;
  }
  const active = channel?.running ? channel.active_config : null;
  // Catalog ids currently explicitly blocked by the *running* channel.
  const activeBlocked = active
    ? TELEMETRY_RULES.filter(entry => active.blocked.some(rule => rule.host === entry.host && (!rule.ports.length || rule.ports.includes(entry.port)))).map(entry => entry.id)
    : [];
  const locked = busy || props.disabled || !channel || !!channel.address;
  return <><IpNetworkPanel {...props} proxyBinding={channel?.running ? channel.channel_binding ?? null : null} disabled={props.disabled || busy} proxyUrl={channel?.running && channel.address ? `http://${channel.address}` : null}/><section className="network-panel">
    <div className="surface-heading"><h3>受控通道</h3><Status value={channel?.running ? 'available' : channel ? 'not_run' : 'unknown'}/></div>
    <div className="padded"><p>通过本地代理观察连接，按确切主机名和端口控制外发。不解密 TLS，也不修改系统代理。</p>
      {transport !== 'native' ? <Notice>此测试空间未连接原生网络模块。请在桌面应用中启动真实通道。</Notice> : <>
        {active && <section aria-label="当前生效的通道配置">
          <h4>当前生效的通道配置</h4><p className="small-print">以下由正在运行的代理读回，仅作用于经过此通道的连接。</p>
          <div className="fact-row"><span>默认动作</span><strong>{active.default_action === 'deny' ? '阻止未匹配的连接' : '放行未匹配的连接'}</strong></div>
          <div className="fact-row"><span>显式阻止</span><RuleReadback rules={active.blocked}/></div>
          <div className="fact-row"><span>显式允许</span><RuleReadback rules={active.allowed}/></div>
          <div className="fact-row"><span>上游代理</span><code className="full-path">{active.upstream ?? '无 · 由本代理直接连接目标'}</code></div>
          <div className="fact-row"><span>{active.upstream ? 'Lintel → 上游' : 'Lintel → 目标'}的连接地址族</span><strong>{active.address_family === 'ipv4_only' ? '严格 IPv4' : '跟随系统'}</strong></div>
          <p className="small-print">显式阻止优先于允许规则；规则只匹配确切主机，不自动匹配子域。</p>
        </section>}
        {channel?.address && !channel.running && <Notice tone="warning">原通道任务已结束，没有正在生效的规则。请先清除旧通道，再重新启动。</Notice>}
        <details className="network-options"><summary>下次启动的配置草案</summary>
          <p className="small-print">{channel?.address ? '停止通道后可基于读回的配置编辑，再显式启动。修改草案不会影响当前连接。' : '这里只编辑草案，点击“启动通道”并收到读回结果后才会生效。草案按环境保留至关闭 Lintel。'}</p>
          <label className="field">默认动作<select disabled={locked} value={draft.defaultAction} onChange={e => updateDraft({ defaultAction: e.target.value as Config['default_action'] })}><option value="allow">放行未匹配的连接</option><option value="deny">阻止未匹配的连接</option></select></label>
          <label className="field">显式阻止规则<textarea rows={3} disabled={locked} value={draft.blocked} onChange={e => updateDraft({ blocked: e.target.value })} placeholder="example.invalid 或 example.invalid 443,8443" spellCheck={false}/><small>每行一个确切主机；空格后可列端口，以英文逗号分隔。省略端口适用全部端口。阻止规则优先，混用 API 域名可能同时承载必要功能。</small></label>
          <label className="field">显式允许规则<textarea rows={3} disabled={locked} value={draft.allowed} onChange={e => updateDraft({ allowed: e.target.value })} placeholder="example.invalid 443" spellCheck={false}/><small>格式同上。默认阻止时，可用允许规则保留所需目标。</small></label>
          <label className="field">上游 HTTP / HTTPS 代理（可选）<input disabled={locked} value={draft.upstream} onChange={e => updateDraft({ upstream: e.target.value })} placeholder="http://127.0.0.1:7890" spellCheck={false}/><small>不支持 SOCKS 或含账号口令的代理 URL。改变规则前先停止通道。</small></label>
          <label className="field">Lintel 建立的连接<select aria-label="Lintel 建立的连接" disabled={locked} value={draft.addressFamily} onChange={event => updateDraft({ addressFamily: event.target.value as Config['address_family'] })}><option value="system">跟随系统地址族选择</option><option value="ipv4_only">严格仅使用 IPv4</option></select><small>连接目标或上游时只使用 IPv4；没有可用 IPv4 就失败。上游之后的公网出口由上方实际请求验证。</small></label>
          {draft.limits && <p className="small-print">沿用已读回的资源限制：最多 {draft.limits.max_connections} 个连接，连接超时 {draft.limits.connect_timeout_seconds} 秒，连接寿命 {draft.limits.connection_lifetime_seconds} 秒。</p>}
          <TelemetryPanel hostAlias={props.hostAlias} selected={draft.telemetry ?? []} disabled={locked} settings={props.settings} onPrivacySettings={props.onPrivacySettings} activeBinding={channel?.running ? channel.channel_binding ?? null : null} activeBlocked={activeBlocked} onRuleTest={ruleTest} onChange={ids => updateDraft({ telemetry: ids })}/>
        </details>
        {channel?.address && <div className="fact-row"><span>{channel.running ? '本地地址' : '原监听地址（已停止）'}</span><code>http://{channel.address}</code></div>}
        <div className="button-row network-controls">{channel?.address ? <button disabled={busy || props.disabled} onClick={() => void act('stop')}>{channel.running ? '停止通道' : '清除旧通道'}</button> : <button disabled={busy || props.disabled || !channel} onClick={() => void act('start')}>启动通道</button>}<button className="primary" disabled={busy || props.disabled || !channel?.running || !channel.address || !executable} onClick={() => onLaunch('http://' + channel!.address)}>通过通道打开 Claude<Icon name="arrow" size={15}/></button><button className="text-button" disabled={busy || props.disabled} onClick={() => void act('status')}>刷新连接</button></div>
        <h4>客户端连接记录</h4>
        {channel?.events.some(event => event.origin !== 'rule_test') ? <div className="connection-log">{channel.events.filter(event => event.origin !== 'rule_test').slice(-12).reverse().map((event,index) => <div key={`${event.timestamp_unix_ms}-${index}`}><code>{event.destination_host ?? '未知目标'}{event.destination_port ? `:${event.destination_port}` : ''}</code><span>{event.outcome === 'blocked' ? '已阻止' : event.outcome}{event.peer_family ? ` · 实际连接 ${event.peer_family === 'ipv4' ? 'IPv4' : 'IPv6'}` : ''}</span></div>)}</div> : <p className="small-print">暂无已记录的客户端连接。</p>}
        {channel?.events.some(event => event.origin === 'rule_test') && <section aria-label="受控请求记录"><h4>受控请求记录</h4><div className="connection-log">{channel.events.filter(event => event.origin === 'rule_test').slice(-12).reverse().map(event => <div key={event.test_id}><code>{event.destination_host}:{event.destination_port}</code><span>{event.outcome} · {event.test_id}</span></div>)}</div></section>}
      </>}
      {message && <p role="status" className="network-message">{message}</p>}{error && <div role="alert"><Notice tone="error">{error}</Notice></div>}
      <p className="small-print">覆盖范围仅限经过此通道的连接。客户端可能绕过代理；没有进程级强约束。关闭 Lintel 会停止通道，已有客户端不会自动退出。</p>
    </div>
  </section></>;
}
