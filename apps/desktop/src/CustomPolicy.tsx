import type { CustomSettings, PolicyAssessment, PolicyChoice, Setting } from './types';

const effects: Record<string, string> = {
  DISABLE_TELEMETRY: '减少产品使用指标；Remote Control 的依赖随版本和组织条件变化。',
  DISABLE_ERROR_REPORTING: '关闭错误报告。你的 OTel 与其他自设诊断保持原样。',
  DISABLE_FEEDBACK_COMMAND: '关闭主动反馈入口，包括 /bug 与 /feedback。',
  CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY: '关闭质量调查提示。',
  DO_NOT_TRACK: '跨工具遥测开关；此处只写当前 Claude 配置根，不设置全局 shell。',
  DISABLE_GROWTHBOOK: '关闭 feature-flag 获取，会与 Remote Control 冲突。',
  CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: '非空即关闭（0/false 也生效）：同时关闭遥测、错误回报、自动更新、release notes 与 feature flags，会与 Remote Control 冲突。',
};

export default function CustomPolicy({ policy, settings, choices, onChange }: {
  policy: PolicyAssessment; settings: Setting[]; choices: CustomSettings; onChange: (choices: CustomSettings) => void;
}) {
  return <section className="surface custom-policy" aria-label="自定义控制">
    <div className="surface-heading"><h2>逐项选择</h2><span className="small-label">其余保持原值</span></div>
    <p className="custom-policy-intro">“关闭”使用此变量的关闭值；“移除覆盖”只删除本环境的字段。shell、项目或组织仍可能影响新启动。</p>
    {policy.rules.map(rule => {
      const choice = choices[rule.key] ?? 'keep';
      const setting = settings.find(item => item.key === rule.key);
      return <div className="custom-policy-row" key={rule.key}>
        <div className="custom-policy-copy"><label htmlFor={`policy-${rule.key}`}>{rule.label}</label><p>{effects[rule.key]}</p><span className="custom-policy-current">当前：<code>{rule.value === null ? '未设置' : JSON.stringify(rule.value)}</code><span>{rule.disabled === null ? '语义未确认' : rule.disabled ? '此开关已关闭' : '此开关未关闭'}</span></span></div>
        <select id={`policy-${rule.key}`} aria-label={`${rule.label}的选择`} value={choice} onChange={event => onChange({ ...choices, [rule.key]: event.target.value as PolicyChoice })}>
          <option value="keep">保持原值</option><option value="disable">关闭这一项</option><option value="remove">移除本环境覆盖</option>
        </select>
        <details className="custom-policy-source"><summary>来源与恢复</summary><code>{rule.key}</code><p>来源：<code>{setting?.source ?? '当前注册配置根的 user settings'}</code></p><p>{rule.semantics === 'nonempty' ? '任意非空字符串都会关闭，包括 "0" 和 "false"。' : '按布尔值语义判断；字符串 "0" / "false" 不关闭。'} 已经关闭时保留原值，否则写入 "1"。</p><p>需新启动；执行后可预览恢复原字段。后续编辑发生冲突时会保留后续编辑。</p></details>
        {choice === 'remove' && <p className="custom-policy-impact">将删除该字段，可能重新开放对应流量；其他来源与实际运行仍需核对。</p>}
      </div>;
    })}
  </section>;
}
