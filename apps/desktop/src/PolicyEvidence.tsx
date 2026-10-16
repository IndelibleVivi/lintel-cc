import type { PolicyAssessment, PolicyRule } from './types';
import { Notice } from './ui';
import { useI18n } from './i18n-provider';
import { present } from './i18n-presentation';
import { policyChoiceKey, policyLabelKeys, policySemanticsKeys, policyStateKey } from './policyEditor';

const sourceKeys: Record<string, string> = { native_version_path: 'policy.sourceNative', npm_package: 'policy.sourceNpm', unknown: 'policy.sourceUnknown' };

export default function PolicyEvidence({ policy, projected = false }: { policy: PolicyAssessment; projected?: boolean }) {
  const { t, locale } = useI18n();
  const p = (value: string | null | undefined) => present(value, locale);
  const label = (rule: PolicyRule) => policyLabelKeys[rule.key] ? t(policyLabelKeys[rule.key]) : p(rule.label);
  const remote = policy.remote_control;
  const source = sourceKeys[policy.product.source] ? t(sourceKeys[policy.product.source]) : policy.product.source;
  return <section className="policy-evidence" aria-label={t(projected ? 'policy.projectedAria' : 'policy.currentAria')}>
    <div className="fact-row"><span>{t('policy.productVersion')}</span><span>{policy.product.version ?? t('policy.unidentified')} · {source}</span></div>
    <Notice tone={remote.status === 'configuration_compatible' ? 'neutral' : 'warning'}><strong>{t(projected ? 'policy.projectedConditions' : 'policy.currentConditions')}</strong><p>{p(remote.summary)}</p></Notice>
    {policy.preset === 'custom' && <details className="environment-diagnostics"><summary>{t('policy.customChoices')}</summary><p>{t('policy.customChoicesNote')}</p>{policy.rules.filter(rule => (policy.custom_settings?.[rule.key] ?? 'keep') !== 'keep').map(rule => <div className="fact-row" key={rule.key}><span>{label(rule)}</span><span>{t(policyChoiceKey(rule, policy.custom_settings?.[rule.key] ?? 'keep', policy.custom_values?.[rule.key] ?? rule.value))}</span></div>)}</details>}
    <details className="environment-diagnostics"><summary>{t('policy.semantics')}</summary>
      <p>{t('policy.semanticsNote')}</p>
      {policy.rules.map(rule => <div className="policy-rule" key={rule.key}><code>{rule.key}</code><span>{rule.masked || rule.value_masked ? t('customPolicy.maskedValue') : rule.value === null ? t('policy.notSet') : JSON.stringify(rule.value)} · {t(policyStateKey(rule))}</span><small>{t(policySemanticsKeys[rule.semantics] ?? 'policy.semanticsUnconfirmed')}</small></div>)}
      <p className="muted">{t('policy.rulesLine', { rule: policy.rule_version, version: remote.version_family })}</p>
    </details>
  </section>;
}
