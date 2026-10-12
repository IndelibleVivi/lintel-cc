import { useState } from 'react';
import type { NetworkProbe, ProbeCell, ProbeFamily, ProbePath } from './networkTypes';
import { Icon } from './ui';
import { useI18n } from './i18n-provider';

const outcomeKeys: Record<string, string> = {
  success:'net.outcome.success', ok:'net.outcome.success', not_tested:'net.outcome.notTested', dns_failed:'net.outcome.dnsFailed', no_route:'net.outcome.noRoute',
  refused:'net.outcome.refused', timed_out:'net.outcome.timedOut', tls_failed:'net.outcome.tlsFailed', proxy_denied:'net.outcome.proxyDenied',
  http_error:'net.outcome.httpError', invalid_response:'net.outcome.invalidResponse', family_mismatch:'net.outcome.familyMismatch',
  network_changed:'net.outcome.networkChanged', connection_failed:'net.outcome.connectionFailed', proxy_failed:'net.outcome.proxyFailed',
  ipv4_unavailable:'net.outcome.ipv4Unavailable', ipv6_unavailable:'net.outcome.ipv6Unavailable', permission_denied:'net.outcome.permissionDenied',
};
const modeKeys: Record<string, string> = { automatic:'net.mode.automatic', auto:'net.mode.automatic', manual:'net.mode.manual', off:'net.mode.off', link_local:'net.mode.linkLocal', unknown:'net.mode.unknown' };
export const useIpv6Mode = () => { const { t } = useI18n(); return (mode: string) => modeKeys[mode] ? t(modeKeys[mode]) : mode; };
export const useProbeOutcome = () => { const { t } = useI18n(); return (cell?: ProbeCell) => cell ? (outcomeKeys[cell.status] ? t(outcomeKeys[cell.status]) : cell.status) : t('net.outcome.notTested'); };

function ResultCell({ value, before }: { value?: ProbeCell; before?: ProbeCell }) {
  const { t, formatDateTime } = useI18n();
  const probeOutcome = useProbeOutcome();
  const [copied, setCopied] = useState(false);
  return <td>
    {before && <div className="probe-before"><span>{t('net.before',{outcome:probeOutcome(before)})}</span>{before.public_ip && <code>{before.public_ip}</code>}<Icon name="arrow" size={12}/></div>}
    <div className={`probe-outcome ${value?.status === 'success' || value?.status === 'ok' ? 'reachable' : ''}`}>{probeOutcome(value)}</div>
    {value?.public_ip && <div className="probe-ip"><code>{value.public_ip}</code><button className="icon-button" aria-label={t('net.copyIp',{ip:value.public_ip})} onClick={() => { void navigator.clipboard.writeText(value.public_ip!).then(() => setCopied(true)).catch(() => setCopied(false)); }}><Icon name={copied ? 'check' : 'copy'} size={13}/></button></div>}
    {value && value.status !== 'not_tested' && <span className="small-print">{t('net.elapsed',{ms:value.elapsed_ms,peer:value.peer_family ? t('net.peer',{family:value.peer_family}) : ''})}</span>}
    {value?.message && <p className="small-print">{value.message}</p>}
  </td>;
}

export default function NetworkResults({ probe, before, host }: { probe: NetworkProbe; before?: NetworkProbe; host: string }) {
  const { t, formatDateTime } = useI18n();
  const cell = (report: NetworkProbe | undefined, path: ProbePath, family: ProbeFamily) => report?.cells.find(cell => cell.path === path && cell.family === family);
  return <section className="network-results" aria-label={t(before ? 'net.resultsCompare' : 'net.resultsActual')}>
    <p className="probe-context">{host}{probe.execution_host ? ` · ${probe.execution_host}` : ''} · {probe.platform} · {formatDateTime(probe.executed_at)}{before && <span> · {t('net.after')}</span>}</p>
    <div className="probe-table-wrap"><table className="probe-table"><caption>{t(before ? 'net.captionCompare' : 'net.captionFour')}</caption><thead><tr><th scope="col">{t('net.colPath')}</th><th scope="col">{t('net.colIpv4')}</th><th scope="col">{t('net.colIpv6')}</th></tr></thead><tbody>
      {(['host_default', 'lintel_channel'] as const).map(path => <tr key={path}><th scope="row">{t(path === 'host_default' ? 'net.pathHost' : 'net.pathChannel')}<small>{path === 'host_default' ? t('net.pathHostNote') : probe.proxy_url ?? t('net.noChannel')}</small></th>{(['ipv4', 'ipv6'] as const).map(family => <ResultCell key={family} value={cell(probe, path, family)} before={cell(before, path, family)}/>)}</tr>)}
    </tbody></table></div>
    <p className="small-print">{t('net.disclaimer')}</p>
  </section>;
}
