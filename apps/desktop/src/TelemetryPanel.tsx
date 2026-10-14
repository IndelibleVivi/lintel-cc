import { useEffect, useMemo, useState } from 'react';
import { requester, transport } from './api';
import { Notice, Status } from './ui';
import { useI18n } from './i18n-provider';
import { present } from './i18n-presentation';
import { ResourceLink } from './Resources';
import type { Inspection, TelemetryCatalog } from './types';
import './telemetry.css';

type Props = {
  hostAlias: string | null;
  /** Selected catalog ids merged into the stopped-channel blocked draft. */
  selected: string[];
  onChange: (ids: string[]) => void;
  disabled?: boolean;
  /** Observed Claude settings, when the caller has a real `inspect` result. */
  settings?: Inspection['settings'];
  /** Ask the caller to open the existing privacy-settings review (no mutation). */
  onPrivacySettings?: () => void;
  /** Exact running channel binding, when a channel is live. */
  activeBinding?: string | null;
  /** Catalog ids currently explicitly blocked by the running channel. */
  activeBlocked?: string[];
  /** Run a real controlled test on the current running channel. */
  onRuleTest?: (id: string, binding: string) => Promise<ControlledOutcome>;
};

// A controlled test outcome as returned by the native `rule_test` op. It is
// shown separately from observed client traffic and from settings readback.
export type ControlledOutcome = {
  result: string; explicit_block: boolean; connection_attempted: boolean;
  provenance: string; destination_host: string; destination_port: number; test_id: string;
};

// The observed NONESSENTIAL total-switch value is shown separately from the
// fine-grained switches. A missing readback is `not_read`, never "disabled".
const NONESSENTIAL = 'CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC';
const FINE_GRAINED = ['DISABLE_TELEMETRY', 'DISABLE_ERROR_REPORTING', 'CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY', 'DISABLE_FEEDBACK_COMMAND', 'DO_NOT_TRACK', 'DISABLE_GROWTHBOOK'];

// The telemetry catalog is static metadata from the shared contract. This panel
// only reads it and lets the user select blockable destinations into the
// existing stopped-channel draft; it never starts or changes a running channel.
export default function TelemetryPanel({ hostAlias, selected, onChange, disabled, settings, onPrivacySettings, activeBinding, activeBlocked, onRuleTest }: Props) {
  const { t, locale } = useI18n();
  const presentText = (value: string | null | undefined) => present(value, locale);
  const send = useMemo(() => requester(hostAlias), [hostAlias]);
  const [catalog, setCatalog] = useState<TelemetryCatalog | null>(null);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(transport !== 'synthetic');
  // Controlled-test results are keyed by the exact channel binding so a stop or
  // replacement cannot show a stale pass for a new instance.
  const [tested, setTested] = useState<{ binding: string; id: string; outcome: ControlledOutcome | null; error?: string } | null>(null);
  const [testBusy, setTestBusy] = useState('');

  useEffect(() => {
    let active = true;
    setBusy(true); setError('');
    send('telemetry_catalog', {})
      .then(value => { if (active) setCatalog(value); })
      .catch(err => { if (active) setError(err instanceof Error ? err.message : String(err)); })
      .finally(() => { if (active) setBusy(false); });
    return () => { active = false; };
  }, [hostAlias, send]);

  const blockable = catalog?.destinations.filter(entry => entry.blockable) ?? [];
  const informational = catalog?.destinations.filter(entry => !entry.blockable) ?? [];
  const notRead = settings === undefined;
  const observed = (key: string) => settings?.find(setting => setting.key === key);
  const nonessential = observed(NONESSENTIAL);

  function toggle(id: string, checked: boolean) {
    onChange(checked ? [...selected.filter(value => value !== id), id] : selected.filter(value => value !== id));
  }

  async function runTest(id: string) {
    if (!onRuleTest || !activeBinding) return;
    setTestBusy(id);
    try {
      const outcome = await onRuleTest(id, activeBinding);
      setTested({ binding: activeBinding, id, outcome });
    } catch (err) {
      setTested({ binding: activeBinding, id, outcome: null, error: err instanceof Error ? err.message : String(err) });
    } finally { setTestBusy(''); }
  }
  const activeBlockedSet = new Set(activeBlocked ?? []);
  // Only a stale result (different binding) is dropped; a same-instance result
  // remains until the channel changes.
  const currentTest = tested && activeBinding === tested.binding ? tested : null;

  return <section className="telemetry-panel" aria-label={t('telemetry.aria')}>
    <div className="surface-heading"><h3>{t('telemetry.aria')}</h3><Status value={catalog ? 'available' : busy ? 'pending' : 'unknown'}/></div>
    <div className="padded">
      <p>{t('telemetry.intro')}</p>
      {error && <Notice tone="error">{error}</Notice>}
      {!catalog && busy && <p className="small-print">{t('telemetry.readingCatalog')}</p>}
      {catalog && <>
        <p className="small-print"><ResourceLink resource="claude-network-reference">{t('telemetry.officialRef')}</ResourceLink> · {t('telemetry.checkedDate',{date:catalog.checked_date})} · schema {catalog.schema}</p>
        <section className="telemetry-privacy" aria-label={t('telemetry.privacyAria')}>
          <h4>{t('telemetry.privacyTitle')}</h4>
          {notRead ? <p className="small-print" role="status">{t('telemetry.notRead')}</p> : <>
            <div className="fact-row telemetry-master-row"><span>{t('telemetry.nonessentialLabel')}</span><strong><code>{NONESSENTIAL}</code> · {(nonessential?.value ?? t('app.notSet'))} · {nonessential ? settingState(nonessential) : 'unknown'}</strong></div>
            <p className="small-print">{t('telemetry.nonessentialNote')}</p>
            <ul className="compact-list telemetry-info">{FINE_GRAINED.map(key => { const setting = observed(key); return <li key={key}><code>{key}</code> · {setting ? `${setting.value ?? t('app.notSet')} · ${settingState(setting)}` : 'not_read'}</li>; })}</ul>
          </>}
          {onPrivacySettings && <button className="text-button" type="button" onClick={onPrivacySettings}>{t('telemetry.viewPrivacy')}</button>}
        </section>
        <h4>{t('telemetry.blockableTitle')}</h4>
        {blockable.map(entry => <label className="telemetry-row" key={entry.id}>
          <input type="checkbox" disabled={disabled} checked={selected.includes(entry.id)} onChange={event => toggle(entry.id, event.target.checked)}/>
          <span><strong>{presentText(entry.purpose)}</strong>
            <span className="telemetry-detail"><code>{entry.host}:{entry.port}</code> · {entry.provider} · {t('telemetry.clients',{clients:entry.clients.join(t('list.separator'))})}</span>
            <span className="telemetry-detail">{t('telemetry.applicability',{applicability:presentText(entry.applicability),version:presentText(entry.version_applicability),date:entry.checked_date})}</span>
            <span className="telemetry-detail telemetry-impact">{t('telemetry.impact',{impact:presentText(entry.collateral_impact)})}</span>
          </span>
        </label>)}
        {blockable.length === 0 && <p className="small-print">{t('telemetry.noBlockable')}</p>}
        <h4>{t('telemetry.infoTitle')}</h4>
        <ul className="compact-list telemetry-info">{informational.map(entry => <li key={entry.id}>
          <code>{entry.host}:{entry.port}</code> · {presentText(entry.purpose)}
          <span className="telemetry-detail">{t('telemetry.applicabilityShort',{applicability:presentText(entry.applicability)})}</span>
          <span className="telemetry-detail telemetry-impact">{t('telemetry.impact',{impact:presentText(entry.collateral_impact)})}</span>
        </li>)}</ul>
        <Notice>{t('telemetry.notice')}</Notice>
        <section className="telemetry-test" aria-label={t('telemetry.testAria')}>
          <h4>{t('telemetry.testTitle')}</h4>
          {!activeBinding ? <p className="small-print" role="status">{t('telemetry.noActiveChannel')}</p> : <>
            <p className="small-print">{t('telemetry.testNote')}</p>
            {blockable.map(entry => {
              const blockedNow = activeBlockedSet.has(entry.id);
              return <div className="telemetry-test-row" key={entry.id}>
                <code>{entry.host}:{entry.port}</code>
                {blockedNow
                  ? <button className="text-button" type="button" disabled={!!testBusy} onClick={() => void runTest(entry.id)}>{testBusy === entry.id ? t('telemetry.testing') : t('telemetry.testThisBlock')}</button>
                  : <span className="telemetry-detail">{t('telemetry.notBlockedNow')}</span>}
              </div>;
            })}
            {currentTest && <div className="fact-row telemetry-test-result" role="status">
              <span>{t('telemetry.controlledTest',{target:currentTest.outcome?.destination_host || currentTest.id})}</span>
              <strong>{t(currentTest.outcome?.result === 'blocked_explicit' ? 'telemetry.blockConfirmed' : 'telemetry.blockNotPassed')}</strong>
              {currentTest.outcome ? <span className="small-print">{t('telemetry.testDetail',{id:currentTest.outcome.test_id,attempted:currentTest.outcome.connection_attempted ? t('app.yes') : t('app.no')})}</span> : <span className="small-print">{currentTest.error}</span>}
            </div>}
            <p className="small-print">{t('telemetry.testFooter')}</p>
          </>}
        </section>
      </>}
    </div>
  </section>;
}

// A finite, honest label for one observed setting. `status` is the readback's
// own state (`configured`/`unchanged`/`uncertain`); nothing is asserted when the
// value was not read.
function settingState(setting: Inspection['settings'][number]): string {
  return setting.status;
}
