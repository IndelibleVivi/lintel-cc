import type { NetworkPlan } from './networkTypes';
import type { Receipt } from './types';
import NetworkResults from './NetworkResults';
import { Notice } from './ui';
import { useI18n } from './i18n-provider';

export function NetworkPlanReview({ network, host }: { network: NetworkPlan; host: string }) {
  const { t } = useI18n();
  return <section className="network-plan-review" aria-label={t('netPlan.aria')}>
    <Notice tone="warning"><strong>{host} · {network.service_name} · {t('app.hostSharedNetwork')}</strong><p>{t('netPlan.warning')}</p></Notice>
    <div className="fact-row"><span>{t('netPlan.exactService')}</span><code>{network.service_id} · {network.interface}</code></div>
    <p className="small-print">{t('netPlan.serviceState',{state:network.service_enabled === false ? t('netPlan.serviceDisabled') : t('netPlan.serviceKept')})}</p>
    <details><summary>{t('netPlan.checkConfig')}</summary><div className="network-config-review"><div><strong>{t('netPlan.before')}</strong><pre>{JSON.stringify(network.before, null, 2)}</pre></div><div><strong>{t('netPlan.after')}</strong><pre>{JSON.stringify(network.after, null, 2)}</pre></div></div></details>
    <p>{t('netPlan.approval')}</p>
    <p className="small-print">IPv4 · <code>{network.probe.ipv4_url}</code><br/>IPv6 · <code>{network.probe.ipv6_url}</code><br/>{t('netPlan.channel')} · <code>{network.probe.proxy_url ?? t('netPlan.notSelected')}</code></p>
    {network.before_probe && <NetworkResults probe={network.before_probe} host={host}/>}
  </section>;
}

export function NetworkReceiptEvidence({ receipt, host }: { receipt: Receipt; host: string }) {
  const { t } = useI18n();
  if (!receipt.network_change) return null;
  return <section aria-label={t('netReceipt.aria')}>
    <Notice><strong>{host} · {receipt.network_change.service_name ?? receipt.network_change.service_id} · {t('netReceipt.shared')}</strong><p>{t('netReceipt.note')}</p></Notice>
    {receipt.after_probe ? <NetworkResults probe={receipt.after_probe} before={receipt.before_probe} host={host}/> : <p>{t('netReceipt.noAfter')}</p>}
  </section>;
}
