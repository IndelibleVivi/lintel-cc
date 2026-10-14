import { present } from './i18n-presentation';
import { useEffect, useMemo, useRef, useState } from 'react';
import { requester, transport } from './api';
import type { Plan, Receipt } from './types';
import type { NetworkInspection, NetworkProbe, ProbeOptions } from './networkTypes';
import NetworkResults, { useIpv6Mode } from './NetworkResults';
import { useI18n } from './i18n-provider';
import { Notice } from './ui';

type Props = {
  hostAlias: string | null; proxyUrl: string | null; proxyBinding: string | null; receipts: Receipt[]; disabled: boolean;
  onPlan: (plan: Plan) => void; onReceipt: (receipt: Receipt) => void;
};

export default function IpNetworkPanel({ hostAlias, proxyUrl, proxyBinding, receipts, disabled, onPlan, onReceipt }: Props) {
  const { t, locale } = useI18n();
  const ipv6Mode = useIpv6Mode();
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
  const host = hostAlias ?? t('app.localLabel');
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
  return <section className="ip-network-panel" aria-label={t('ipnet.aria')}>
    <div className="surface-heading"><div><span className="eyebrow">IPv4 / IPv6</span><h3>{t('ipnet.title')}</h3></div><span className="small-label">{t('ipnet.hostShared',{host})}</span></div>
    <p>{t('ipnet.intro')}</p>
    {transport === 'synthetic' && <Notice>{t('ipnet.syntheticNote')}</Notice>}
    <details className="probe-options"><summary>{t('ipnet.testTargets')}</summary>
      <label className="field">{t('ipnet.ipv4Target')}<input value={ipv4Url} disabled={!!busy || disabled} onChange={event => setIpv4Url(event.target.value)} spellCheck={false}/></label>
      <label className="field">{t('ipnet.ipv6Target')}<input value={ipv6Url} disabled={!!busy || disabled} onChange={event => setIpv6Url(event.target.value)} spellCheck={false}/></label>
      {hostAlias && <label className="field">{t('ipnet.remoteChannel')}<input value={remoteProxy} disabled={!!busy || disabled} onChange={event => setRemoteProxy(event.target.value)} placeholder="http://127.0.0.1:PORT" spellCheck={false}/><small>{t('ipnet.remoteChannelNote')}</small></label>}
      <p className="small-print">{t('ipnet.explicitNote',{channel:selectedProxy ?? t('ipnet.noChannel')})}</p>
    </details>
    <p className="small-print probe-endpoints">{t('ipnet.currentTargets')} · <code>{options.ipv4_url}</code> · <code>{options.ipv6_url}</code></p>
    <div className="button-row"><button className="primary" disabled={locked || !options.ipv4_url || !options.ipv6_url} onClick={() => void act('ipnet.testing', async () => { const value = await send('network_probe', options); if (mounted.current) { setProbe(value); const state = await send('network_inspect', {}); if (mounted.current) setInspection(state); } })}>{t('ipnet.testEgress')}</button><button className="text-button" disabled={!!busy || disabled || transport !== 'native'} onClick={() => void act('ipnet.readingStatus', async () => { const value = await send('network_inspect', {}); if (mounted.current) setInspection(value); })}>{t('ipnet.refreshStatus')}</button></div>
    {busy && <p role="status">{t(busy)}</p>}
    {probe && <>{!current && <Notice tone="warning">{t('ipnet.stale')}</Notice>}<NetworkResults probe={probe} host={host}/></>}
    {(error || observeError) && <div role="alert"><Notice tone="error">{error || observeError}</Notice></div>}
    {inspection && <section className="network-services" aria-label={t('ipnet.servicesAria')}><h4>{t('ipnet.hostServices')}</h4>
      <p className="small-print">{t('ipnet.servicesNote')}</p>
      {inspection.services.length ? inspection.services.map(service => <div className="network-service" key={service.service_id}>
        <div className="network-service-heading"><strong>{service.name}</strong><code>{service.interface}</code><span>{ipv6Mode(service.mode)}{!service.enabled ? t('ipnet.serviceDisabled') : ''}</span></div>
        <div className="network-addresses"><span>IPv4 · <code>{service.ipv4_addresses.join(' · ') || t('ipnet.noAddress')}</code></span><span>IPv6 · <code>{service.ipv6_addresses.join(' · ') || t('ipnet.noAddress')}</code></span></div>
        {inspection.platform === 'macos' && <div className="button-row"><button disabled={locked || service.mode === 'off'} onClick={() => void act('ipnet.freezing', async () => { const plan = await send('plan_network_ipv6', { service_id: service.service_id, mode: 'off', probe: options, ...(current ? { baseline_id: probe!.id } : {}) }); if (mounted.current) onPlan(plan); })}>{t('ipnet.previewOff')}</button><button disabled={locked || service.mode === 'link_local'} onClick={() => void act('ipnet.freezing', async () => { const plan = await send('plan_network_ipv6', { service_id: service.service_id, mode: 'link_local', probe: options, ...(current ? { baseline_id: probe!.id } : {}) }); if (mounted.current) onPlan(plan); })}>{t('ipnet.previewLinkLocal')}</button></div>}
      </div>) : <p className="muted">{t('ipnet.noServices')}</p>}
      {inspection.platform === 'macos' && <p className="small-print">{t('ipnet.previewNote')}</p>}
      {!!additionalInterfaces.length && <details><summary>{t('ipnet.otherInterfaces')}</summary>{additionalInterfaces.map(item => <div className="network-service" key={item.interface}><div className="network-service-heading"><code>{item.interface}</code><span>{t(item.up ? 'ipnet.ifaceUp' : 'ipnet.ifaceDown')}</span></div><div className="network-addresses"><span>IPv4 · <code>{item.ipv4_addresses.join(' · ') || t('ipnet.noAddress')}</code></span><span>IPv6 · <code>{item.ipv6_addresses.join(' · ') || t('ipnet.noAddress')}</code></span></div></div>)}</details>}
      {inspection.limitations.map((limitation, index) => <p className="small-print" key={index}>{present(limitation,locale)}</p>)}
    </section>}
    {!!networkReceipts.length && <details className="network-history"><summary>{t('ipnet.history')}</summary>{networkReceipts.map(receipt => <div className="fact-row network-history-row" key={receipt.id}><span>{present(receipt.title,locale)}</span><button className="text-button" onClick={() => onReceipt(receipt)}>{t(receipt.restorable ? 'ipnet.viewResultRestore' : 'app.viewResult')}</button></div>)}</details>}
  </section>;
}
