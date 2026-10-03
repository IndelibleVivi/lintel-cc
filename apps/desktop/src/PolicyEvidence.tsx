import type { PolicyAssessment } from './types';
import { Notice } from './ui';

export default function PolicyEvidence({ policy, projected = false }: { policy: PolicyAssessment; projected?: boolean }) {
  const remote = policy.remote_control;
  const source = { native_version_path: '原生安装版本目录', npm_package: 'npm package metadata', unknown: '未找到可识别的安装 metadata' }[policy.product.source];
  return <section className="policy-evidence" aria-label={projected ? '执行后预计功能条件' : '当前功能条件'}>
    <div className="fact-row"><span>产品版本</span><span>{policy.product.version ?? '未识别'} · {source}</span></div>
    <Notice tone={remote.status === 'configuration_compatible' ? 'neutral' : 'warning'}><strong>{projected ? '执行后预计条件' : '当前配置条件'}</strong><p>{remote.summary}</p></Notice>
    <details className="environment-diagnostics"><summary>变量语义与验证范围</summary>
      <p>仅核对当前环境 user settings。组织条件为用户声明；shell、项目、组织配置、账号资格和实际 Remote Control 运行仍未验证。移除字段后需新启动。</p>
      {policy.rules.map(rule => <div className="policy-rule" key={rule.key}><code>{rule.key}</code><span>{rule.value === null ? '未设置' : JSON.stringify(rule.value)} · {rule.disabled === null ? '取值语义未确认' : rule.disabled ? '配置为关闭' : '此开关未关闭'}</span><small>{rule.semantics === 'nonempty' ? '任意非空值生效，包括 0 / false' : '按布尔值解析，0 / false 不生效'}</small></div>)}
      <p className="muted">规则：{policy.rule_version} · 版本条件：{remote.version_family} · 运行效果未验证</p>
    </details>
  </section>;
}
