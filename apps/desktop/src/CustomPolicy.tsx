import { useState } from 'react';
import type { CustomSettings, CustomValues, PolicyAssessment, PolicyChoice, PolicyRule, Setting } from './types';
import { Icon } from './ui';
import { useI18n } from './i18n-provider';
import { present } from './i18n-presentation';
import { commonPolicyOptions, pendingPolicyEdits, policyGroup, policyOptions, policyStateKey, policyChoiceKeys, policyChoiceKey, policySemanticsKeys, policyLabelKeys, policyValueError, removesValue, visiblePolicyOptions, type PolicyGroup } from './policyEditor';
import './CustomPolicy.css';

const effectKeys: Record<string, string> = {
  DISABLE_TELEMETRY: 'customPolicy.effect.disableTelemetry',
  DISABLE_ERROR_REPORTING: 'customPolicy.effect.disableErrorReporting',
  DISABLE_FEEDBACK_COMMAND: 'customPolicy.effect.disableFeedbackCommand',
  CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY: 'customPolicy.effect.disableFeedbackSurvey',
  DO_NOT_TRACK: 'customPolicy.effect.doNotTrack',
  DISABLE_GROWTHBOOK: 'customPolicy.effect.disableGrowthbook',
  CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: 'customPolicy.effect.disableNonessential',
  OTEL_LOG_RAW_API_BODIES: 'customPolicy.effect.rawApiBodies',
};
const groupKeys: Record<PolicyGroup, string> = {
  privacy: 'customPolicy.group.privacy', content: 'customPolicy.group.content',
  telemetry: 'customPolicy.group.telemetry', identity: 'customPolicy.group.identity', other: 'customPolicy.group.other',
};
const groups: PolicyGroup[] = ['privacy', 'content', 'telemetry', 'identity', 'other'];
const sourceKeys: Record<string, string> = { shell: 'customPolicy.src.shell', user: 'customPolicy.src.user', managed: 'customPolicy.src.managed', project: 'customPolicy.src.project', project_local: 'customPolicy.src.projectLocal' };
const defaultKeys: Record<string, string> = { off: 'customPolicy.defaultOff', on: 'customPolicy.defaultOn', redacted: 'customPolicy.defaultRedacted', unset: 'customPolicy.defaultUnset' };

export default function CustomPolicy({ policy, settings, choices, values, disabled, onChange }: {
  policy: PolicyAssessment; settings: Setting[]; choices: CustomSettings; values: CustomValues; disabled: boolean;
  onChange: (choices: CustomSettings, values?: CustomValues) => void;
}) {
  const { t, locale } = useI18n();
  const p = (value: string | null | undefined) => present(value, locale);
  const label = (rule: PolicyRule) => policyLabelKeys[rule.key] ? t(policyLabelKeys[rule.key]) : p(rule.label);
  const [query, setQuery] = useState('');
  const [group, setGroup] = useState<PolicyGroup | 'all'>('all');
  const [onlyChanged, setOnlyChanged] = useState(false);
  const [batchMode, setBatchMode] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [batch, setBatch] = useState<PolicyChoice | ''>('');
  const [applied, setApplied] = useState(0);
  const receiver = policy.rules.find(rule => rule.key === 'OTEL_EXPORTER_OTLP_ENDPOINT');
  const pending = pendingPolicyEdits(policy.rules, choices, values);
  const visible = policy.rules.filter(rule => (group === 'all' || policyGroup(rule) === group)
    && (!onlyChanged || (choices[rule.key] ?? 'keep') !== 'keep')
    && `${rule.key} ${label(rule)} ${p(rule.impact)}`.toLocaleLowerCase().includes(query.toLocaleLowerCase()));
  const selectedRules = policy.rules.filter(rule => selected.has(rule.key));
  const batchOptions = commonPolicyOptions(selectedRules);
  const hiddenSelected = selectedRules.filter(rule => !visible.includes(rule)).length;
  const allVisibleSelected = visible.length > 0 && visible.every(rule => selected.has(rule.key));

  function choose(rule: PolicyRule, choice: PolicyChoice) {
    setApplied(0);
    // Choosing a custom value explicitly seeds that field. A value displayed
    // only as a fallback must never masquerade as a submitted custom_values entry.
    const initial = values[rule.key] ?? (rule.masked || rule.value_masked ? '' : rule.value ?? '');
    onChange({ [rule.key]: choice }, choice === 'set' ? { [rule.key]: initial } : undefined);
  }
  function chooseDisplayed(rule: PolicyRule, choice: string) {
    if (choice !== 'file') { choose(rule, choice as PolicyChoice); return; }
    const current = values[rule.key] ?? rule.value ?? '';
    setApplied(0);
    onChange({ [rule.key]: 'set' }, { [rule.key]: current.startsWith('file:') ? current : 'file:' });
  }
  function toggleBatch() { setBatchMode(!batchMode); setSelected(new Set()); setBatch(''); setApplied(0); }
  function applyBatch() {
    if (!batch || !batchOptions.includes(batch) || !selectedRules.length) return;
    onChange(Object.fromEntries(selectedRules.map(rule => [rule.key, batch])));
    setApplied(selectedRules.length);
  }


  return <section className="surface custom-policy" aria-label={t('customPolicy.aria')}>
    <header className="policy-editor-heading">
      <div><h2>{t('customPolicy.title')}</h2><p>{t('customPolicy.editorNote')}</p></div>
      <button disabled={disabled} aria-pressed={batchMode} onClick={toggleBatch}>{t(batchMode ? 'customPolicy.endBatch' : 'customPolicy.startBatch')}</button>
    </header>
    <div className="policy-editor-tools">
      <label className="field policy-search"><span className="sr-only">{t('customPolicy.search')}</span><input type="search" value={query} onChange={event => setQuery(event.target.value)} placeholder={t('customPolicy.search')}/></label>
      <button className="text-button" aria-pressed={onlyChanged} onClick={() => setOnlyChanged(!onlyChanged)}>{t('customPolicy.onlyChanged', { count: pending.pending.length })}</button>
      {pending.pending.length > 0 && <button className="text-button" disabled={disabled} onClick={() => { onChange(Object.fromEntries(policy.rules.map(rule => [rule.key, 'keep']))); setApplied(0); }}>{t('customPolicy.resetDraft')}</button>}
    </div>
    <nav className="policy-group-filter" aria-label={t('customPolicy.groupFilter')}>
      <button aria-pressed={group === 'all'} onClick={() => setGroup('all')}>{t('customPolicy.allGroups')}</button>
      {groups.filter(item => policy.rules.some(rule => policyGroup(rule) === item)).map(item => <button key={item} aria-pressed={group === item} onClick={() => setGroup(item)}>{t(groupKeys[item])}</button>)}
    </nav>
    {batchMode && <div className="policy-batch" aria-label={t('customPolicy.batchPanel')}>
      <div className="policy-batch-selection"><label><input type="checkbox" aria-label={t('customPolicy.selectVisible')} checked={allVisibleSelected} disabled={!visible.length || disabled} onChange={event => setSelected(current => { const next = new Set(current); visible.forEach(rule => event.target.checked ? next.add(rule.key) : next.delete(rule.key)); return next; })}/>{t('customPolicy.selectVisible')}</label><span>{t('customPolicy.selectedCount', { count: selectedRules.length })}{hiddenSelected > 0 && t('customPolicy.hiddenSelected', { count: hiddenSelected })}</span>{selected.size > 0 && <button className="text-button" onClick={() => { setSelected(new Set()); setApplied(0); }}>{t('customPolicy.clearSelection')}</button>}</div>
      <div className="policy-batch-actions"><div className="field"><label className="sr-only" htmlFor="policy-batch-choice">{t('customPolicy.batchChoice')}</label><select id="policy-batch-choice" value={batchOptions.includes(batch as PolicyChoice) ? batch : ''} disabled={!selectedRules.length || disabled} onChange={event => { setBatch(event.target.value as PolicyChoice); setApplied(0); }}><option value="">{t('customPolicy.chooseBatch')}</option>{batchOptions.map(choice => <option key={choice} value={choice}>{t(choice === 'enable' && selectedRules.some(rule => rule.semantics === 'raw_api_bodies') ? 'customPolicy.batchEnableWithRaw' : policyChoiceKeys[choice])}</option>)}</select></div><button disabled={disabled || !batchOptions.includes(batch as PolicyChoice)} onClick={applyBatch}>{t('customPolicy.applyBatch', { count: selectedRules.length })}</button></div>
      <p className="small-print">{t('customPolicy.batchNote')}</p>
      <p className="policy-editor-feedback" role="status">{applied > 0 ? t('customPolicy.batchApplied', { count: applied }) : t('customPolicy.batchScope')}</p>
    </div>}
    {visible.length === 0 && <p className="policy-editor-empty">{t(onlyChanged && !query && group === 'all' ? 'customPolicy.noPending' : 'customPolicy.noResults')}</p>}
    {groups.map(item => {
      const rules = visible.filter(rule => policyGroup(rule) === item);
      if (!rules.length) return null;
      return <section className="policy-rule-group" key={item} aria-label={t(groupKeys[item])}>
        <h3>{t(groupKeys[item])}<span>{rules.length}</span></h3>
        {rules.map(rule => {
          const choice = choices[rule.key] ?? 'keep';
          const masked = rule.masked === true || rule.value_masked === true;
          const setting = settings.find(value => value.key === rule.key);
          const effect = effectKeys[rule.key] ? t(effectKeys[rule.key]) : p(rule.impact);
          const dirty = choice !== 'keep';
          const raw = rule.semantics === 'raw_api_bodies';
          const rawMode = choice === 'set' ? values[rule.key]?.startsWith('file:') ? 'file' : values[rule.key] === '1' ? 'enable' : values[rule.key] === '0' ? 'disable' : 'set' : choice;
          const supported = policyOptions(rule).includes(choice);
          const displayChoice = !supported ? 'unsupported' : raw ? rawMode : choice === 'disable' && removesValue(rule) ? 'remove' : choice;
          const valueError = choice === 'set' ? policyValueError(rule, values[rule.key] ?? '') : null;

          return <div className={`custom-policy-row${dirty ? ' is-changed' : ''}${batchMode ? ' is-selecting' : ''}`} key={rule.key} data-key={rule.key} data-category={rule.category ?? 'other'}>
            <div className="policy-rule-main">
              {batchMode && <input className="policy-rule-selection" type="checkbox" disabled={disabled} checked={selected.has(rule.key)} aria-label={t('customPolicy.selection', { key: label(rule) })} onChange={event => { setApplied(0); setSelected(current => { const next = new Set(current); event.target.checked ? next.add(rule.key) : next.delete(rule.key); return next; }); }}/>}
              <div className="policy-rule-copy"><h4>{label(rule)}</h4><p>{t('customPolicy.configuredState', { state: t(policyStateKey(rule)) })}{dirty && <span className="policy-pending">{t('customPolicy.pending')}</span>}</p></div>
              <label className="field policy-rule-choice"><span className="sr-only">{t('customPolicy.choiceFor', { label: label(rule) })}</span><select disabled={disabled} aria-label={t('customPolicy.choiceFor', { label: label(rule) })} value={displayChoice} onChange={event => chooseDisplayed(rule, event.target.value)}>{!supported && <option value="unsupported" disabled>{t('customPolicy.chooseAgain')}</option>}{raw ? <><option value="keep">{t('customPolicy.choice.keep')}</option><option value="disable">{t('customPolicy.choice.disable')}</option><option value="enable">{t('customPolicy.rawInline')}</option><option value="file">{t('customPolicy.rawFile')}</option><option value="remove">{t('customPolicy.choice.remove')}</option>{rawMode === 'set' && <option value="set" disabled>{t('customPolicy.chooseRawMode')}</option>}</> : visiblePolicyOptions(rule).map(option => <option key={option} value={option}>{t(policyChoiceKey(rule, option))}</option>)}</select></label>
            </div>
            {choice === 'set' && supported && (!raw || rawMode === 'file') && <label className="field policy-value-editor"><span>{t(raw ? 'customPolicy.directory' : rule.key === 'CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH' ? 'customPolicy.limitBytes' : rule.key === 'OTEL_EXPORTER_OTLP_ENDPOINT' ? 'customPolicy.endpoint' : 'customPolicy.value')}</span><input disabled={disabled || rule.key === 'OTEL_EXPORTER_OTLP_HEADERS'} aria-label={t('customPolicy.valueFor', { label: label(rule) })} aria-invalid={!!valueError} inputMode={rule.key === 'CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH' ? 'numeric' : rule.key === 'OTEL_EXPORTER_OTLP_ENDPOINT' ? 'url' : 'text'} value={raw ? (values[rule.key] ?? '').slice(5) : values[rule.key] ?? ''} onChange={event => onChange({}, { [rule.key]: raw ? `file:${event.target.value}` : event.target.value })} placeholder={raw ? t('customPolicy.directoryPlaceholder') : t('customPolicy.valuePlaceholder')} spellCheck={false}/>{valueError && <span className="password-error">{t(valueError)}</span>}{rule.key === 'OTEL_EXPORTER_OTLP_ENDPOINT' && <span className="small-print">{t('customPolicy.plainEndpointNote')}</span>}</label>}
            {choice === 'set' && rule.category === 'privacy_switch' && <p className="policy-choice-note">{t('customPolicy.rawPrivacyNote')}</p>}
            {!supported && <p className="password-error">{t('customPolicy.unsupportedChoice')}</p>}
            {raw && choice === 'set' && rawMode === 'set' && <p className="password-error">{t('customPolicy.chooseRawMode')}</p>}
            {(choice === 'remove' || (choice === 'disable' && removesValue(rule)) || (choice === 'enable' && rule.semantics === 'default_on')) && <p className="policy-choice-note">{t(choice === 'remove' || (choice === 'disable' && removesValue(rule)) ? 'customPolicy.removeImpact' : 'customPolicy.enableDefaultOn')}</p>}
            <div className="policy-rule-secondary">
              <details className="custom-policy-detail"><summary>{t('customPolicy.details')}<Icon name="chevron" size={11}/></summary>
                <div className="policy-rule-information">
                  {effect && <p className="custom-policy-impact">{effect}</p>}
                  <dl className="custom-policy-facts">
                    <div><dt>{t('customPolicy.rawCurrent')}</dt><dd><code>{masked ? t('customPolicy.maskedValue') : rule.value === null ? t('app.notSet') : JSON.stringify(rule.value)}</code></dd></div>
                    <div><dt>{t('customPolicy.factSource')}</dt><dd>{setting?.source ?? t('customPolicy.userSettings')}</dd></div>
                    {(rule.local_dump || rule.key.startsWith('OTEL_') || rule.category?.startsWith('content_') || rule.category === 'telemetry_core' || rule.category === 'feedback') && <div><dt>{t('customPolicy.factReceiver')}</dt><dd>{rule.local_dump ? <>{rule.value !== `file:${rule.local_dump}` && <code>{rule.local_dump}</code>}<p>{t('customPolicy.localDumpNote')}</p></> : <>{receiver?.masked || receiver?.value_masked ? t('customPolicy.maskedValue') : receiver?.value ? <code>{receiver.value}</code> : t('customPolicy.receiverNotSet')}<p>{t('customPolicy.receiverScope')}</p></>}</dd></div>}
                  </dl>
                  <details className="custom-policy-evidence"><summary>{t('customPolicy.evidence')}</summary><code>{rule.key}</code>{rule.key === 'OTEL_LOG_RAW_API_BODIES' && rule.impact && <p>{p(rule.impact)}</p>}<dl className="custom-policy-facts"><div><dt>{t('customPolicy.factSemantics')}</dt><dd>{policySemanticsKeys[rule.semantics] ? t(policySemanticsKeys[rule.semantics]) : rule.semantics}</dd></div><div><dt>{t('customPolicy.factDefault')}</dt><dd>{rule.default_state && defaultKeys[rule.default_state] ? t(defaultKeys[rule.default_state]) : (rule.default_state ?? t('customPolicy.defaultUnknown'))}</dd></div><div><dt>{t('customPolicy.factSources')}</dt><dd>{rule.sources ? t('customPolicy.sourcesLine', { on: rule.sources.turn_on.map(source => sourceKeys[source] ? t(sourceKeys[source]) : source).join(t('list.separator')), off: rule.sources.turn_off.map(source => sourceKeys[source] ? t(sourceKeys[source]) : source).join(t('list.separator')) }) : t('customPolicy.sourcesUnconfirmed')}</dd></div>{rule.version_floor && <div><dt>{t('customPolicy.factVersion')}</dt><dd>{t('customPolicy.versionFloor', { version: rule.version_floor })}</dd></div>}</dl><p>{t('customPolicy.evidenceNote', { rule: policy.rule_version })}</p>{rule.depends_on?.length ? <p>{t('customPolicy.dependsOn', { keys: rule.depends_on.join(t('list.separator')) })}</p> : null}</details>
                </div>
              </details>
              {dirty && <button className="text-button policy-rule-undo" disabled={disabled} onClick={() => choose(rule, 'keep')}>{t('customPolicy.undoItem')}</button>}
            </div>
          </div>;
        })}
      </section>;
    })}
  </section>;
}
