import { useEffect, useMemo, useRef, useState } from 'react';
import { requester, transport } from './api';
import type { Plan, Receipt } from './types';
import type { NetworkInspection, NetworkProbe, ProbeOptions } from './networkTypes';
import NetworkResults, { ipv6Mode } from './NetworkResults';
import { Notice } from './ui';

type Props = {
  hostAlias: string | null; proxyUrl: string | null; proxyBinding: string | null; receipts: Receipt[]; disabled: boolean;
  onPlan: (plan: Plan) => void; onReceipt: (receipt: Receipt) => void;
};

export default function IpNetworkPanel({ hostAlias, proxyUrl, proxyBinding, receipts, disabled, onPlan, onReceipt }: Props) {
  const send = useMemo(() => requester(hostAlias), [hostAlias]);
  const [inspection, setInspection] = useState<NetworkInspection | null>(null);
  const [probe, setProbe] = useState<NetworkProbe | null>(null);
  const [ipv4Url, setIpv4Url] = useState('https://api.ipify.org');
  const [ipv6Url, setIpv6Url] = useState('https://api6.ipify.org');
  const [remoteProxy, setRemoteProxy] = useState('');
  const [busy, setBusy] = useState('');
  const [error, setError] = useState('');
  const [observeError, setObserveError] = useState('');
  const mounted = useRef(false);
  const observing = useRef(false);
  const host = hostAlias ?? '本机';
  const selectedProxy = hostAlias ? remoteProxy.trim() || null : proxyUrl;
  const options: ProbeOptions = { ipv4_url: ipv4Url.trim(), ipv6_url: ipv6Url.trim(), ...(selectedProxy ? { proxy_url: selectedProxy } : {}), timeout_seconds: 10 };
  const networkReceipts = receipts.filter(receipt => receipt.network_change);
  const latest = networkReceipts[0];
  const additionalInterfaces = inspection?.interfaces?.filter(item => !inspection.services.some(service => service.interface === item.interface)) ?? [];
  const locked = disabled || !!busy || !inspection;
  const current = !!probe && !probe.stale && !!inspection && inspection.network_revision_complete !== false && !observeError && probe.network_revision === inspection.network_revision
    && probe.endpoints.ipv4 === options.ipv4_url && probe.endpoints.ipv6 === options.ipv6_url
    && probe.timeout_seconds === options.timeout_seconds
    && (probe.proxy_url ?? null) === selectedProxy && (hostAlias || !selectedProxy || probe.proxy_binding === proxyBinding) && Date.now() - Date.parse(probe.executed_at) < 300_000;

  useEffect(() => {
    mounted.current = true;
    let active = true;
    async function observe() {
      if (transport !== 'native' || observing.current || disabled || document.visibilityState === 'hidden') return;
      observing.current = true;
      try { const value = await send('network_inspect', {}); if (active) { setInspection(value); setObserveError(''); } }
      catch (err) { if (active) setObserveError(err instanceof Error ? err.message : String(err)); }
      finally { observing.current = false; }
    }
    if (transport === 'native') void observe();
    const timer = window.setInterval(() => { if (transport === 'native') void observe(); }, 30_000);
    window.addEventListener('focus', observe);
    document.addEventListener('visibilitychange', observe);
    return () => { active = false; mounted.current = false; window.clearInterval(timer); window.removeEventListener('focus', observe); document.removeEventListener('visibilitychange', observe); };
  }, [send, disabled, latest?.id]);
  useEffect(() => { if (latest?.after_probe) setProbe(latest.after_probe); }, [latest?.id]);

  async function act(label: string, action: () => Promise<void>) {
    if (busy || disabled) return;
    setBusy(label); setError('');
    try { await action(); } catch (err) { if (mounted.current) setError(err instanceof Error ? err.message : String(err)); }
    finally { if (mounted.current) setBusy(''); }
  }
  return <section className="ip-network-panel" aria-label="IPv4 / IPv6 网络路径">
    <div className="surface-heading"><div><span className="eyebrow">IPv4 / IPv6</span><h3>实际出口与网络路径</h3></div><span className="small-label">{host} · 宿主共享</span></div>
    <p>先测试实际出口，再在同一份结果旁预览 IPv6 变更。批准后自动复测；切换 Claude 环境不会切换系统网络设置。</p>
    {transport === 'synthetic' && <Notice>合成开发空间没有宿主网络权限；此处不会读取真实接口、访问公网目标或修改系统设置。网络交互使用独立的合成旅程验收。</Notice>}
    <details className="probe-options"><summary>本次测试目标与路径</summary>
      <label className="field">IPv4 HTTPS 回显目标<input value={ipv4Url} disabled={!!busy || disabled} onChange={event => setIpv4Url(event.target.value)} spellCheck={false}/></label>
      <label className="field">IPv6 HTTPS 回显目标<input value={ipv6Url} disabled={!!busy || disabled} onChange={event => setIpv6Url(event.target.value)} spellCheck={false}/></label>
      {hostAlias && <label className="field">远端运行中的 Lintel 通道（可选）<input value={remoteProxy} disabled={!!busy || disabled} onChange={event => setRemoteProxy(event.target.value)} placeholder="http://127.0.0.1:PORT" spellCheck={false}/><small>地址属于这台远端主机。请先用该 runner 启动通道；桌面代理不会覆盖远端。</small></label>}
      <p className="small-print">显式测试会访问这两个端点，向它们暴露本次出口 IP。无后台公网测试；不接受跳转或凭据 URL。当前通道：{selectedProxy ?? '未选择，通道两项显示未测试'}。</p>
    </details>
    <p className="small-print probe-endpoints">本次目标 · <code>{options.ipv4_url}</code> · <code>{options.ipv6_url}</code></p>
    <div className="button-row"><button className="primary" disabled={locked || !options.ipv4_url || !options.ipv6_url} onClick={() => void act('正在测试 IPv4 / IPv6…', async () => { const value = await send('network_probe', options); if (mounted.current) { setProbe(value); const state = await send('network_inspect', {}); if (mounted.current) setInspection(state); } })}>测试实际出口</button><button className="text-button" disabled={!!busy || disabled || transport !== 'native'} onClick={() => void act('读取网络状态…', async () => { const value = await send('network_inspect', {}); if (mounted.current) setInspection(value); })}>刷新网络状态</button></div>
    {busy && <p role="status">{busy}</p>}
    {probe && <>{!current && <Notice tone="warning">网络状态、测试目标或通道已变化，或这份结果超过五分钟／观察不完整。保留历史结果；下一次操作会重新前测。</Notice>}<NetworkResults probe={probe} host={host}/></>}
    {(error || observeError) && <div role="alert"><Notice tone="error">{error || observeError}</Notice></div>}
    {inspection && <section className="network-services" aria-label="宿主共享网络服务"><h4>这台主机的网络服务</h4>
      <p className="small-print">配置与地址观察分别列出。仅有地址或路由，不证明应用已经使用它；新增接口需要重新测试。</p>
      {inspection.services.length ? inspection.services.map(service => <div className="network-service" key={service.service_id}>
        <div className="network-service-heading"><strong>{service.name}</strong><code>{service.interface}</code><span>{ipv6Mode(service.mode)}{!service.enabled ? ' · 服务已停用' : ''}</span></div>
        <div className="network-addresses"><span>IPv4 · <code>{service.ipv4_addresses.join(' · ') || '未观察到地址'}</code></span><span>IPv6 · <code>{service.ipv6_addresses.join(' · ') || '未观察到地址'}</code></span></div>
        {inspection.platform === 'macos' && <div className="button-row"><button disabled={locked || service.mode === 'off'} onClick={() => void act('冻结 IPv6 关闭计划与前测…', async () => { const plan = await send('plan_network_ipv6', { service_id: service.service_id, mode: 'off', probe: options, ...(current ? { baseline_id: probe!.id } : {}) }); if (mounted.current) onPlan(plan); })}>预览关闭 IPv6</button><button disabled={locked || service.mode === 'link_local'} onClick={() => void act('冻结链路本地计划与前测…', async () => { const plan = await send('plan_network_ipv6', { service_id: service.service_id, mode: 'link_local', probe: options, ...(current ? { baseline_id: probe!.id } : {}) }); if (mounted.current) onPlan(plan); })}>预览仅链路本地</button></div>}
      </div>) : <p className="muted">没有可用的网络服务观察；以限制说明为准。</p>}
      {inspection.platform === 'macos' && <p className="small-print">预览会复用仍有效的前测，或测试上方端点。变更影响使用此网络服务的其他应用，可能切断 IPv6-only 网络或 VPN；批准时可能需要 macOS 网络配置授权。保持设置，直到明确恢复。</p>}
      {!!additionalInterfaces.length && <details><summary>其他接口观察（含 VPN / TUN）</summary>{additionalInterfaces.map(item => <div className="network-service" key={item.interface}><div className="network-service-heading"><code>{item.interface}</code><span>{item.up ? '接口已启用' : '接口未启用'}</span></div><div className="network-addresses"><span>IPv4 · <code>{item.ipv4_addresses.join(' · ') || '未观察到地址'}</code></span><span>IPv6 · <code>{item.ipv6_addresses.join(' · ') || '未观察到地址'}</code></span></div></div>)}</details>}
      {inspection.limitations.map((limitation, index) => <p className="small-print" key={index}>{limitation}</p>)}
    </section>}
    {!!networkReceipts.length && <details className="network-history"><summary>共享网络的修改与恢复记录</summary>{networkReceipts.map(receipt => <div className="fact-row" key={receipt.id}><span>{receipt.title}</span><button className="text-button" onClick={() => onReceipt(receipt)}>查看结果{receipt.restorable ? '与恢复' : ''}</button></div>)}</details>}
  </section>;
}
