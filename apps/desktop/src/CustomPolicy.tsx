import type { CustomSettings, PolicyAssessment, PolicyChoice, Setting } from './types';
import { useI18n } from './i18n-provider';
import { present } from './i18n-presentation';

const effectKeys: Record<string, string> = {
  DISABLE_TELEMETRY: 'customPolicy.effect.disableTelemetry',
  DISABLE_ERROR_REPORTING: 'customPolicy.effect.disableErrorReporting',
  DISABLE_FEEDBACK_COMMAND: 'customPolicy.effect.disableFeedbackCommand',
  CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY: 'customPolicy.effect.disableFeedbackSurvey',
  DO_NOT_TRACK: 'customPolicy.effect.doNotTrack',
  DISABLE_GROWTHBOOK: 'customPolicy.effect.disableGrowthbook',
  CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: 'customPolicy.effect.disableNonessential',
};

export default function CustomPolicy({ policy, settings, choices, onChange }: {
  policy: PolicyAssessment; settings: Setting[]; choices: CustomSettings; onChange: (choices: CustomSettings) => void;
}) {
  const { t, locale } = useI18n();
  const p = (value: string | null | undefined) => present(value, locale);
  return <section className="surface custom-policy" aria-label={t('customPolicy.aria')}>
    <div className="surface-heading"><h2>{t('customPolicy.title')}</h2><span className="small-label">{t('customPolicy.keepOthers')}</span></div>
    <p className="custom-policy-intro">{t('customPolicy.intro')}</p>
    {policy.rules.map(rule => {
      const choice = choices[rule.key] ?? 'keep';
      const setting = settings.find(item => item.key === rule.key);
      return <div className="custom-policy-row" key={rule.key}>
        <div className="custom-policy-copy"><label htmlFor={`policy-${rule.key}`}>{p(rule.label)}</label><p>{effectKeys[rule.key] ? t(effectKeys[rule.key]) : rule.label}</p><span className="custom-policy-current">{t('customPolicy.current')}<code>{rule.value === null ? t('app.notSet') : JSON.stringify(rule.value)}</code><span>{rule.disabled === null ? t('customPolicy.semanticsUnconfirmed') : rule.disabled ? t('customPolicy.switchOff') : t('customPolicy.switchOn')}</span></span></div>
        <select id={`policy-${rule.key}`} aria-label={t('customPolicy.choiceFor',{label:p(rule.label)})} value={choice} onChange={event => onChange({ ...choices, [rule.key]: event.target.value as PolicyChoice })}>
          <option value="keep">{t('policy.choiceKeep')}</option><option value="disable">{t('policy.choiceDisable')}</option><option value="remove">{t('policy.choiceRemove')}</option>
        </select>
        <details className="custom-policy-source"><summary>{t('customPolicy.sourceAndRestore')}</summary><code>{rule.key}</code><p>{t('customPolicy.source',{source:setting?.source ?? t('customPolicy.userSettings')})}</p><p>{t(rule.semantics === 'nonempty' ? 'customPolicy.semanticsNonempty' : 'customPolicy.semanticsBoolean')}</p><p>{t('customPolicy.newLaunchNote')}</p></details>
        {choice === 'remove' && <p className="custom-policy-impact">{t('customPolicy.removeImpact')}</p>}
      </div>;
    })}
  </section>;
}
