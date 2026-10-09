import { useState } from 'react';
import type { NetworkProbe, ProbeCell, ProbeFamily, ProbePath } from './networkTypes';
import { formatDate, Icon } from './ui';

const outcomes: Record<string, string> = {
  success: '请求成功', ok: '请求成功', not_tested: '未测试', dns_failed: 'DNS 解析失败', no_route: '无可用路由',
  refused: '连接被拒绝', timed_out: '请求超时', tls_failed: 'TLS 核验失败', proxy_denied: '通道拒绝请求',
  http_error: 'HTTP 请求失败', invalid_response: '回显无效', family_mismatch: '回显地址族不符',
  network_changed: '测试期间网络变化', connection_failed: '连接失败', proxy_failed: '通道连接失败',
  ipv4_unavailable: '没有可用 IPv4 地址', ipv6_unavailable: '没有可用 IPv6 地址', permission_denied: '系统拒绝连接',
};
export const probeOutcome = (cell?: ProbeCell) => cell ? outcomes[cell.status] ?? cell.status : '未测试';
export const ipv6Mode = (mode: string) => ({ automatic: '自动配置', auto: '自动配置', manual: '手动配置', off: '关闭', link_local: '仅链路本地', unknown: '未识别' })[mode] ?? mode;

function ResultCell({ value, before }: { value?: ProbeCell; before?: ProbeCell }) {
  const [copied, setCopied] = useState(false);
  return <td>
    {before && <div className="probe-before"><span>变更前 · {probeOutcome(before)}</span>{before.public_ip && <code>{before.public_ip}</code>}<Icon name="arrow" size={12}/></div>}
    <div className={`probe-outcome ${value?.status === 'success' || value?.status === 'ok' ? 'reachable' : ''}`}>{probeOutcome(value)}</div>
    {value?.public_ip && <div className="probe-ip"><code>{value.public_ip}</code><button className="icon-button" aria-label={`复制出口 IP ${value.public_ip}`} onClick={() => { void navigator.clipboard.writeText(value.public_ip!).then(() => setCopied(true)).catch(() => setCopied(false)); }}><Icon name={copied ? 'check' : 'copy'} size={13}/></button></div>}
    {value && value.status !== 'not_tested' && <span className="small-print">{value.elapsed_ms} ms{value.peer_family ? ` · 建立连接 ${value.peer_family}` : ''}</span>}
    {value?.message && <p className="small-print">{value.message}</p>}
  </td>;
}

export default function NetworkResults({ probe, before, host }: { probe: NetworkProbe; before?: NetworkProbe; host: string }) {
  const cell = (report: NetworkProbe | undefined, path: ProbePath, family: ProbeFamily) => report?.cells.find(cell => cell.path === path && cell.family === family);
  return <section className="network-results" aria-label={before ? '出口前后对照' : '实际出口结果'}>
    <p className="probe-context">{host}{probe.execution_host ? ` · ${probe.execution_host}` : ''} · {probe.platform} · {formatDate(probe.executed_at)}{before && <span> · 变更后</span>}</p>
    <div className="probe-table-wrap"><table className="probe-table"><caption>{before ? '相同目标与路径的前后结果' : '从所选主机发出的四项请求'}</caption><thead><tr><th scope="col">测试路径</th><th scope="col">IPv4 目标</th><th scope="col">IPv6 目标</th></tr></thead><tbody>
      {(['host_default', 'lintel_channel'] as const).map(path => <tr key={path}><th scope="row">{path === 'host_default' ? '主机默认网络' : '当前 Lintel 通道'}<small>{path === 'host_default' ? '保留系统 VPN / TUN 的作用' : probe.proxy_url ?? '未选择运行中的通道'}</small></th>{(['ipv4', 'ipv6'] as const).map(family => <ResultCell key={family} value={cell(probe, path, family)} before={cell(before, path, family)}/>)}</tr>)}
    </tbody></table></div>
    <p className="small-print">IP 来自服务对这一次请求的回显。连接失败、超时或 IPv4 成功，都不证明其他应用的 IPv6 流量已被阻止。</p>
  </section>;
}
