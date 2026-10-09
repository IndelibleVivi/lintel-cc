import type { NetworkPlan } from './networkTypes';
import type { Receipt } from './types';
import NetworkResults from './NetworkResults';
import { Notice } from './ui';

export function NetworkPlanReview({ network, host }: { network: NetworkPlan; host: string }) {
  return <section className="network-plan-review" aria-label="批准中的共享网络变更">
    <Notice tone="warning"><strong>{host} · {network.service_name} · 宿主共享网络</strong><p>影响使用这个网络服务的其他应用。保持此设置直到明确恢复；IPv6-only 网络和依赖 IPv6 的 VPN 可能失去连接。</p></Notice>
    <div className="fact-row"><span>准确服务与接口</span><code>{network.service_id} · {network.interface}</code></div>
    <p className="small-print">服务本身 · {network.service_enabled === false ? '已停用；本次不会启用服务' : '保持原启用状态'}</p>
    <details><summary>核对完整 IPv6 配置与恢复内容</summary><div className="network-config-review"><div><strong>变更前</strong><pre>{JSON.stringify(network.before, null, 2)}</pre></div><div><strong>批准后</strong><pre>{JSON.stringify(network.after, null, 2)}</pre></div></div></details>
    <p>批准包括配置写入后对相同目标与路径的自动复测。配置读回和请求结果分别呈现；探测失败不会被解释为防泄漏成功。</p>
    <p className="small-print">IPv4 · <code>{network.probe.ipv4_url}</code><br/>IPv6 · <code>{network.probe.ipv6_url}</code><br/>通道 · <code>{network.probe.proxy_url ?? '未选择'}</code></p>
    {network.before_probe && <NetworkResults probe={network.before_probe} host={host}/>}
  </section>;
}

export function NetworkReceiptEvidence({ receipt, host }: { receipt: Receipt; host: string }) {
  if (!receipt.network_change) return null;
  return <section aria-label="共享网络变更与出口结果">
    <Notice><strong>{host} · {receipt.network_change.service_name ?? receipt.network_change.service_id} · 宿主共享</strong><p>配置是否完成，以本任务的写入和读回步骤为准。原任务记录不会在切换 Claude 环境或关闭 App 时消失。</p></Notice>
    {receipt.after_probe ? <NetworkResults probe={receipt.after_probe} before={receipt.before_probe} host={host}/> : <p>未取得后测结果。保留原任务 ID，查询配置步骤；不据此判断实际出口。</p>}
  </section>;
}
