import { useState } from 'react';
import type { CustomSettings, CustomValues, PolicyAssessment, PolicyChoice, Setting } from './types';
import { Icon, Status } from './ui';
import { useI18n } from './i18n-provider';
import { present } from './i18n-presentation';

// Human impact lines for the originally verified privacy switches. Newer OTel
// content fields carry their own `impact` from the versioned rule catalog, which
// is rendered when present (fail-open: unknown keys show the rule label only).
const effectKeys: Record<string, string> = {
  DISABLE_TELEMETRY: 'customPolicy.effect.disableTelemetry',
  DISABLE_ERROR_REPORTING: 'customPolicy.effect.disableErrorReporting',
  DISABLE_FEEDBACK_COMMAND: 'customPolicy.effect.disableFeedbackCommand',
  CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY: 'customPolicy.effect.disableFeedbackSurvey',
  DO_NOT_TRACK: 'customPolicy.effect.doNotTrack',
  DISABLE_GROWTHBOOK: 'customPolicy.effect.disableGrowthbook',
  CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: 'customPolicy.effect.disableNonessential',
};
// Visibility of the "value" input per finite semantics family.
const setsValue = new Set(['value', 'raw_api_bodies', 'nonempty']);
const categoryKeys: Record<string, string> = {
  privacy_switch: 'customPolicy.category.privacy', telemetry_core: 'customPolicy.category.telemetryCore',
  content_prompts: 'customPolicy.category.contentPrompts', content_responses: 'customPolicy.category.contentResponses',
  content_tool: 'customPolicy.category.contentTool', content_tool_details: 'customPolicy.category.contentToolDetails',
  content_api_bodies: 'customPolicy.category.contentApiBodies', content_managed_settings: 'customPolicy.category.contentManaged',
  content_limit: 'customPolicy.category.contentLimit', exporter: 'customPolicy.category.exporter',
  exporter_destination: 'customPolicy.category.exporterDestination', identity_attribute: 'customPolicy.category.identityAttribute',
  feedback: 'customPolicy.category.feedback', webfetch: 'customPolicy.category.webfetch',
};
const sourceNameKeys: Record<string, string> = { shell: 'customPolicy.src.shell', user: 'customPolicy.src.user', managed: 'customPolicy.src.managed', project: 'customPolicy.src.project', project_local: 'customPolicy.src.projectLocal' };
const defaultKeys: Record<string, string> = { off: 'customPolicy.defaultOff', on: 'customPolicy.defaultOn', redacted: 'customPolicy.defaultRedacted', unset: 'customPolicy.defaultUnset' };
const choiceKeys: Record<PolicyChoice, string> = { keep: 'customPolicy.choice.keep', disable: 'customPolicy.choice.disable', enable: 'customPolicy.choice.enable', remove: 'customPolicy.choice.remove', set: 'customPolicy.choice.set' };

export default function CustomPolicy({ policy, settings, choices, values, onChoice, onValue }: {
  policy: PolicyAssessment; settings: Setting[]; choices: CustomSettings; values: CustomValues;
  onChoice: (key: string, choice: PolicyChoice) => void; onValue: (key: string, value: string) => void;
}) {
  const { t, locale } = useI18n();
  const [query,setQuery]=useState('');
  const [selected,setSelected]=useState<Set<string>>(new Set());
  const [batch,setBatch]=useState<PolicyChoice>('disable');
  const optionsFor=(rule:PolicyAssessment['rules'][number]):PolicyChoice[]=>rule.key==='OTEL_EXPORTER_OTLP_HEADERS'?['keep','disable','remove']
    :rule.category==='privacy_switch'?['keep','disable','enable','remove','set']
    :rule.semantics==='value'?['keep','disable','set','remove']
    :rule.semantics==='raw_api_bodies'?['keep','disable','set','remove']
    :['keep','disable','enable','remove'];
  const p = (value: string | null | undefined) => present(value, locale);
  const receiver = policy.rules.find(rule => rule.key === 'OTEL_EXPORTER_OTLP_ENDPOINT');
  const semanticsKeys: Record<string, string> = {
    nonempty: 'customPolicy.semanticsNonempty', boolean: 'customPolicy.semanticsBoolean',
    default_on: 'customPolicy.semanticsDefaultOn', value: 'customPolicy.semanticsValue', raw_api_bodies: 'customPolicy.semanticsRawApiBodies',
  };
  return <section className="surface custom-policy" aria-label={t('customPolicy.aria')}>
    <div className="surface-heading"><div><h2>{t('customPolicy.title')}</h2><span className="small-label">{t('customPolicy.keepOthers')}</span></div><button className="text-button" onClick={() => policy.rules.forEach(rule => onChoice(rule.key, 'keep'))}>{t('customPolicy.keepAll')}</button></div>
    <p className="custom-policy-intro">{t('customPolicy.intro')}</p>
    <div className="padded config-controls"><label className="field"><span className="sr-only">{t('customPolicy.search')}</span><input type="search" value={query} onChange={e=>setQuery(e.target.value)} placeholder={t('customPolicy.search')}/></label><label className="field">{t('customPolicy.batchChoice')}<select value={batch} onChange={e=>setBatch(e.target.value as PolicyChoice)}>{(['keep','disable','enable','remove'] as const).map(choice=><option key={choice} value={choice}>{t(choiceKeys[choice])}</option>)}</select></label><button disabled={!selected.size} onClick={()=>policy.rules.filter(rule=>selected.has(rule.key)&&optionsFor(rule).includes(batch)).forEach(rule=>onChoice(rule.key,batch))}>{t('customPolicy.batch')} · {selected.size}</button></div>
    {policy.rules.filter(rule=>`${rule.key} ${rule.label} ${rule.impact??''}`.toLocaleLowerCase().includes(query.toLocaleLowerCase())).map(rule => {
      const choice = choices[rule.key] ?? 'keep';
      const setting = settings.find(item => item.key === rule.key);
      const masked = rule.masked === true || rule.value_masked === true;
      const value = values[rule.key] ?? (typeof rule.value === 'string' ? rule.value : '');
      const effect = effectKeys[rule.key] ? t(effectKeys[rule.key]) : rule.impact;
      // Masked keys never accept enable or set, and their value stays hidden.
      // Action direction is derived by core; positive OTel flags and DISABLE_* flags differ.
      const options = optionsFor(rule);
      const signal=rule.signal_enabled ?? (rule.category==='privacy_switch' && rule.disabled!==null ? !rule.disabled : null);
      const dirty = choice !== 'keep' && !(choice === 'set' && !value);
      return <details className="custom-policy-row" key={rule.key} data-key={rule.key} data-category={rule.category ?? 'other'}>
        <summary>
          <input type="checkbox" checked={selected.has(rule.key)} aria-label={t('customPolicy.selection',{key:rule.key})} onClick={e=>e.stopPropagation()} onChange={e=>setSelected(current=>{const next=new Set(current);e.target.checked?next.add(rule.key):next.delete(rule.key);return next;})}/>
          <span className="custom-policy-copy"><label>{p(rule.label)}</label><small>{categoryKeys[rule.category ?? ''] ? t(categoryKeys[rule.category ?? '']) : (rule.category ?? '')}</small></span>
          <span className="custom-policy-current"><code>{masked ? t('customPolicy.maskedValue') : rule.value === null ? t('app.notSet') : JSON.stringify(rule.value)}</code><span>{signal === null ? t('customPolicy.semanticsUnconfirmed') : signal ? t('customPolicy.switchOn') : t('customPolicy.switchOff')}</span>{dirty && <span className="small-label">{t('customPolicy.changed')}</span>}</span>
          <Status value={rule.disabled === true ? 'configured' : rule.disabled === false ? 'unchanged' : 'uncertain'}/>
          <Icon name="chevron" size={13}/>
        </summary>
        <div className="custom-policy-detail">
          {effect && <p className="custom-policy-impact">{effect}</p>}
          {rule.local_dump && <p className="small-print">{t('customPolicy.localDump', { path: rule.local_dump })}</p>}
          <div className="custom-policy-controls">
            <label className="field">{t('customPolicy.action')}<select aria-label={t('customPolicy.choiceFor', { label: p(rule.label) })} value={choice} onChange={event => onChoice(rule.key, event.target.value as PolicyChoice)}>
              {options.map(option => <option key={option} value={option}>{t(choiceKeys[option])}</option>)}
            </select></label>
            {(setsValue.has(rule.semantics) || rule.category==='privacy_switch') && choice === 'set' && <label className="field">{t('customPolicy.value')}<input value={value} disabled={rule.key==='OTEL_EXPORTER_OTLP_HEADERS'} onChange={event => onValue(rule.key, event.target.value)} placeholder={rule.semantics === 'raw_api_bodies' ? t('customPolicy.rawPlaceholder') : t('customPolicy.valuePlaceholder')} spellCheck={false}/>{!value && <span className="password-error">{t('customPolicy.valueRequired')}</span>}</label>}
          </div>
          <dl className="custom-policy-facts">
            <div><dt>{t('customPolicy.factSource')}</dt><dd>{t('customPolicy.source', { source: setting?.source ?? t('customPolicy.userSettings') })}</dd></div>
            <div><dt>{t('customPolicy.factReceiver')}</dt><dd>{rule.local_dump ? <code>{rule.local_dump}</code>
              : rule.key.startsWith('OTEL_') || rule.category?.startsWith('content_') || rule.category === 'telemetry_core' || rule.category === 'feedback'
                ? <>{receiver?.masked || receiver?.value_masked ? t('customPolicy.maskedValue') : receiver?.value ? <code>{receiver.value}</code> : t('customPolicy.receiverNotSet')}<p className="small-print">{t('customPolicy.receiverScope')}</p></>
                : t('customPolicy.factReceiverUnset')}</dd></div>
            <div><dt>{t('customPolicy.factSemantics')}</dt><dd>{semanticsKeys[rule.semantics] ? t(semanticsKeys[rule.semantics]) : rule.semantics}</dd></div>
            <div><dt>{t('customPolicy.factDefault')}</dt><dd>{rule.default_state && defaultKeys[rule.default_state] ? t(defaultKeys[rule.default_state]) : (rule.default_state ?? t('customPolicy.defaultUnknown'))}</dd></div>
            <div><dt>{t('customPolicy.factSources')}</dt><dd>{rule.sources ? t('customPolicy.sourcesLine', { on: rule.sources.turn_on.map(s => sourceNameKeys[s] ? t(sourceNameKeys[s]) : s).join(t('list.separator')), off: rule.sources.turn_off.map(s => sourceNameKeys[s] ? t(sourceNameKeys[s]) : s).join(t('list.separator')) }) : t('customPolicy.sourcesUnconfirmed')}</dd></div>
            {rule.version_floor && <div><dt>{t('customPolicy.factVersion')}</dt><dd>{t('customPolicy.versionFloor', { version: rule.version_floor })}</dd></div>}
          </dl>
          <details className="custom-policy-evidence"><summary>{t('customPolicy.evidence')}</summary><code>{rule.key}</code><p>{t('customPolicy.evidenceNote', { rule: policy.rule_version })}</p>{rule.depends_on && rule.depends_on.length > 0 && <p>{t('customPolicy.dependsOn', { keys: rule.depends_on.join(t('list.separator')) })}</p>}</details>
          {choice === 'remove' && <p className="custom-policy-impact">{t('customPolicy.removeImpact')}</p>}
          {choice === 'enable' && rule.semantics === 'default_on' && <p className="custom-policy-impact">{t('customPolicy.enableDefaultOn')}</p>}
        </div>
      </details>;
    })}
  </section>;
}
