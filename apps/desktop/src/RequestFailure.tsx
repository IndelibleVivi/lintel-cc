import { LocalizedError } from './i18n';
import { useState } from 'react';
import { RequestError } from './api';
import { Icon, Notice } from './ui';
import { useI18n } from './i18n-provider';

// Backend explanation coverage: known error codes get a source-controlled
// explanation resolved through the catalog. `reason`/`summary`/`stderr_excerpt`
// stay raw (data), and an unknown code falls back to the backend message.
const localGuidanceKeys:Record<string,string>={
  stale_plan:'failure.guidance.stalePlan',
  plan_changed:'failure.guidance.planChanged',
  writers_running:'failure.guidance.writersRunning',
  executable_missing:'failure.guidance.executableMissing',
  environment_retired:'failure.guidance.environmentRetired',
  project_missing:'failure.guidance.projectMissing',
  unsupported_resume:'failure.guidance.unsupportedResume',
  terminal_required:'failure.guidance.terminalRequired',
  stale_archive:'failure.guidance.staleArchive',
  browser_component_unavailable:'failure.guidance.browserComponentUnavailable',
};
// UI-owned transport codes: their message was created in api.ts at the locale
// active when thrown, so resolve it from the CURRENT catalog at render. A raw
// backend `message` (unknown code) and any diagnostic stay untouched.
const uiMessageKeys:Record<string,string>={
  NATIVE_REQUIRED:'api.nativeRequired',
  BRIDGE_UNAVAILABLE:'api.bridgeUnavailable',
  stale_archive:'archive.stalePackage',
};
export function asError(error: unknown): Error { return error instanceof Error ? error : new Error(String(error)); }

export default function RequestFailure({ error, context }: { error: Error | string; context?: string }) {
  const { t } = useI18n();
  // Store a finite code, never a translated string, so a language switch after
  // copying re-renders the feedback in the current locale (and the
  // "clipboard unavailable" fall-through keeps working).
  const [copyState, setCopyState] = useState<'' | 'copied' | 'unavailable'>('');
  const diagnostic = error instanceof RequestError ? error.diagnostic : undefined;
  const code = error instanceof RequestError ? error.code : undefined;
  // Resolve a known UI-owned message at the current locale; unknown codes keep
  // the raw backend message.
  const rawMessage = typeof error === 'string' ? error : error.message;
  const message = error instanceof LocalizedError ? t(error.key, error.args)
    : code && Object.hasOwn(uiMessageKeys, code) ? t(uiMessageKeys[code]) : rawMessage;
  const summary = diagnostic?.summary ?? message;
  const stageName = (stage: string) => t(`failure.stage.${stage}` as never);
  const report = [context, summary, code && t('failure.errorCode',{code}), diagnostic && t('failure.stage',{stage:stageName(diagnostic.stage),reason:diagnostic.reason}), diagnostic?.exit_code !== undefined && t('failure.exitCode',{code:diagnostic.exit_code}), diagnostic?.next_steps.join('\n'), diagnostic?.command, diagnostic?.submission_uncertain && t('failure.uncertain')].filter(Boolean).join('\n');
  async function copy() {
    try { await navigator.clipboard.writeText(report); setCopyState('copied'); }
    catch { setCopyState('unavailable'); }
  }
  const copyMessage = copyState === 'copied' ? t('failure.copied') : copyState === 'unavailable' ? t('failure.clipboardUnavailable') : '';
  return <section className="request-failure" role="alert" aria-label={context ? t('failure.titleContext',{context}) : t('failure.title')}>
    <Notice tone="error"><strong>{context && `${context} · `}{summary}</strong>{diagnostic && message !== summary && <p>{message}</p>}{code && <small>{t('failure.errorCode',{code})}{diagnostic && ` · ${stageName(diagnostic.stage)}`}{diagnostic?.exit_code !== undefined && ` · ${t('failure.exitCode',{code:diagnostic.exit_code})}`}</small>}</Notice>
    {!diagnostic && code && Object.hasOwn(localGuidanceKeys, code) && <div className="failure-guidance"><h4>{t('failure.nextSteps')}</h4><p>{t(localGuidanceKeys[code])}</p></div>}
    {diagnostic && <div className="failure-guidance"><h4>{t('failure.howToCheck')}</h4><ol>{diagnostic.next_steps.map((step, i) => <li key={i}>{step}</li>)}</ol>
      {diagnostic.command && <div className="diagnostic-command"><span>{t('failure.ownTerminal')}</span><code>{diagnostic.command}</code></div>}
      {diagnostic.submission_uncertain && <p className="failure-uncertain">{t('failure.uncertainRemote')}</p>}
      {diagnostic.stderr_excerpt && <details><summary>{t('failure.viewOutput',{truncated:diagnostic.stderr_truncated?t('failure.truncated'):''})}</summary><p className="small-print">{t('failure.outputNote')}</p><pre>{diagnostic.stderr_excerpt}</pre></details>}
      <div className="button-row"><button className="text-button" onClick={() => void copy()}><Icon name="copy" size={14}/>{t('failure.copySummary')}</button><span role="status">{copyMessage}</span></div>
      {copyState === 'unavailable' && <pre>{report}</pre>}
    </div>}
  </section>;
}
