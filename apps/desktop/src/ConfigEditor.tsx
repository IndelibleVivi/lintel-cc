import { useEffect, useMemo, useRef, useState } from 'react';
import { request } from './api';
import type { ConfigDocument, ConfigInspection, ConfigRestoreInspection, Environment, Plan } from './types';
import { Icon, Notice, Status } from './ui';
import PolicyEvidence from './PolicyEvidence';
import RequestFailure, { asError } from './RequestFailure';
import { useI18n } from './i18n-provider';
import {present} from './i18n-presentation';

// A tiny, dependency-free structural diff over the two parsed JSON documents.
// It is a display aid only: the plan sent to core always carries the complete
// edited document, and core re-validates against the frozen expected digest.
type DiffLine = { kind: 'same' | 'add' | 'remove'; text: string };
function jsonDiff(before: string, after: string): DiffLine[] {
  const left = before.length ? JSON.stringify(JSON.parse(before), null, 2).split('\n') : [];
  const right = after.length ? JSON.stringify(JSON.parse(after), null, 2).split('\n') : [];
  const lines: DiffLine[] = [];
  let i = 0, j = 0;
  while (i < left.length || j < right.length) {
    if (i < left.length && j < right.length && left[i] === right[j]) { lines.push({ kind: 'same', text: left[i] }); i++; j++; continue; }
    // Find the next shared line to keep the diff readable without a full LCS.
    const lookahead = right.slice(j + 1, j + 12).indexOf(left[i] ?? '\u0000');
    if (i < left.length && (lookahead === -1 || right[j] === undefined)) { lines.push({ kind: 'remove', text: left[i] }); i++; continue; }
    if (j < right.length) { lines.push({ kind: 'add', text: right[j] }); j++; continue; }
  }
  return lines;
}

export default function ConfigEditor({ send = request, environment, disabled = false, onPlan }: {
  send?: typeof request; environment: Environment; disabled?: boolean; onPlan: (plan: Plan) => void;
}) {
  const { t,locale } = useI18n();
  const [inspection, setInspection] = useState<ConfigInspection | null>(null);
  // Optional project cwd selects the project/project-local source for the
  // inspection; empty keeps the user/root default the environment already has.
  const [projectCwd, setProjectCwd] = useState('');
  const [path, setPath] = useState('');
  const [document, setDocument] = useState<ConfigDocument | null>(null);
  const [draftText, setDraftText] = useState('');
  const [error, setError] = useState<Error | null>(null);
  const [busy, setBusy] = useState('');
  const [showDiff, setShowDiff] = useState(false);
  const [restoreJobId, setRestoreJobId] = useState('');
  const [restore, setRestore] = useState<ConfigRestoreInspection | null>(null);
  const [restoreSelected, setRestoreSelected] = useState<Set<string>>(new Set());
  const alive = useRef(true); const lock = useRef(false);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  async function run(name: string, action: () => Promise<void>) {
    if (lock.current) return; lock.current = true; setBusy(name); setError(null);
    try { await action(); } catch (err) { if (alive.current) setError(asError(err)); } finally { lock.current = false; if (alive.current) setBusy(''); }
  }
  // Inspect re-reads the finite writable file set for this environment and the
  // explicitly chosen project cwd. On a target change the previous document and
  // its un-saved draft are dropped so nothing leaks across environments.
  const inspect = () => run('inspect', async () => {
    const data = await send('config_inspect', { environment_id: environment.id, ...(projectCwd.trim() ? { project_cwd: projectCwd.trim() } : {}) });
    if (!alive.current) return;
    setInspection(data); setDocument(null); setDraftText('');
    setPath(current => data.files.some(file => file.writable && file.path === current) ? current : data.files.find(file => file.writable)?.path || '');
  });
  useEffect(() => { setInspection(null); setDocument(null); setDraftText(''); setPath(''); void inspect(); return () => { alive.current = false; }; }, [send, environment.id, environment.root]);
  const read = () => run('read', async () => {
    if (!path) return;
    const file = await send('config_read', { environment_id: environment.id, config_path: path });
    if (alive.current) { setDocument(file); setDraftText(file.document); setShowDiff(false); }
  });
  // Validation is real JSON parse of the edited text; core still re-validates.
  const validation = useMemo(() => {
    if (!document || draftText === document.document) return { state: 'clean' as const, message: '' };
    try { JSON.parse(draftText); return { state: 'valid' as const, message: '' }; }
    catch (err) { return { state: 'invalid' as const, message: err instanceof Error ? err.message : String(err) }; }
  }, [document, draftText]);
  const diff = useMemo(() => (document && showDiff && validation.state === 'valid' ? jsonDiff(document.document, draftText) : []), [document, draftText, showDiff, validation.state]);
  const edit = () => run('edit', async () => {
    if (!document || validation.state !== 'valid') return;
    const plan = await send('plan_config_edit', { environment_id: environment.id, config_path: document.path, document: draftText, expected_digest: document.digest });
    if (alive.current) onPlan(plan);
  });
  // Inspect one original config-edit task's recoverable fields (read-only), then
  // offer a real multi-select. plan_config_restore receives a NON-empty
  // selected_keys; unavailable fields are disabled with their reason.
  const inspectRestore = () => run('restoreInspect', async () => {
    const job = restoreJobId.trim();
    if (!job) return;
    const result = await send('config_restore_inspect', { environment_id: environment.id, job_id: job });
    if (!alive.current) return;
    setRestore(result);
    setRestoreSelected(new Set(result.fields.filter(field => field.available).map(field => field.key)));
  });
  const restorePlan = () => run('restore', async () => {
    const job = restoreJobId.trim();
    const keys = [...restoreSelected].filter(key => restore?.fields.some(field => field.key === key && field.available)).sort();
    if (!job || keys.length === 0) return;
    const plan = await send('plan_config_restore', { environment_id: environment.id, job_id: job, selected_keys: keys });
    if (alive.current) onPlan(plan);
  });
  const writableFiles = inspection?.files.filter(file => file.writable) ?? [];
  return <section className="surface config-editor" aria-label={t('config.aria')}>
    <div className="surface-heading"><div><h2>{t('config.title')}</h2><span className="small-label">{t('config.advanced')}</span></div></div>
    <p className="padded small-print">{t('config.intro')}</p>
    {error && <div className="padded"><RequestFailure error={error} context={t('config.title')}/></div>}
    <div className="padded config-controls config-project-controls">
      <label className="field">{t('config.projectCwd')}<input value={projectCwd} disabled={disabled || busy !== ''} onChange={event => setProjectCwd(event.target.value)} placeholder={t('config.projectCwdPlaceholder')} spellCheck={false}/><small>{t('config.projectCwdNote')}</small></label>
      <button disabled={disabled || busy !== ''} onClick={() => void inspect()}>{busy === 'inspect' ? t('config.reading') : t('config.reinspect')}</button>
    </div>
    {inspection?.conflicts?.map(conflict=><Notice key={conflict.key}>{t('config.conflict',{key:conflict.key,count:conflict.sources.length})}<ul>{conflict.sources.map(source=><li key={source}><code>{source}</code></li>)}</ul></Notice>)}
    {inspection?.policy && <div className="padded"><PolicyEvidence policy={inspection.policy}/></div>}
    {inspection?.settings && inspection.settings.length > 0 && <details className="padded environment-diagnostics"><summary>{t('config.sources', { count: inspection.settings.length })}</summary><ul className="compare-pair-list">{inspection.settings.map(setting => <li key={`${setting.source}:${setting.key}`}><code>{setting.key}</code><span className="small-label">{setting.source}</span><span>{setting.masked?'••••••':setting.value ?? t('snapshot.notSet')}</span>{setting.impact&&<small>{present(setting.impact,locale)}</small>}</li>)}</ul><p className="small-print">{t('config.sourcesNote')}</p></details>}
    {writableFiles.length === 0 ? <Notice>{t('config.noneWritable')}</Notice> : <div className="padded config-controls config-file-picker">
      <label className="field">{t('config.file')}<select aria-label={t('config.file')} value={writableFiles.some(file => file.path === path) ? path : ''} onChange={event => setPath(event.target.value)} disabled={disabled || busy !== ''}><option value="">{t('config.chooseFile')}</option>{writableFiles.map(file => <option key={file.path} value={file.path}>{file.scope} · {file.path.startsWith(environment.root+'/')?file.path.slice(environment.root.length+1):file.path.split('/').slice(-2).join('/')}</option>)}</select></label>
      <button disabled={disabled || busy !== '' || !path.trim()} onClick={() => void read()}>{busy === 'read' ? t('config.reading') : t('config.read')}</button>
      <code className="config-selected-path">{path}</code>
      <details className="config-manual-path"><summary>{t('config.filePath')}</summary><label className="field">{t('config.filePath')}<input value={path} disabled={disabled || busy !== ''} onChange={event => setPath(event.target.value)} placeholder={t('config.filePlaceholder')} spellCheck={false}/><small>{t('config.filePathNote')}</small></label></details>
    </div>}
    {inspection && inspection.files.some(file => !file.writable) && <details className="padded environment-diagnostics"><summary>{t('config.readOnlyFiles', { count: inspection.files.filter(file => !file.writable).length })}</summary><ul className="compare-pair-list">{inspection.files.filter(file => !file.writable).map(file => <li key={file.path}><Status value={file.state}/><code>{file.path}</code></li>)}</ul><p className="small-print">{t('config.readOnlyNote')}</p></details>}
    {document && <div className="padded">
      <div className="config-document-head"><div><span className="small-label">{t('config.digest')}</span><code className="full-path">{document.digest}</code></div>{validation.state === 'valid' && <Status value="verified"/>}{validation.state === 'invalid' && <Status value="failed"/>}{validation.state === 'clean' && <Status value="unchanged"/>}</div>
      <label className="field">{t('config.document')}<textarea className="config-textarea" spellCheck={false} value={draftText} disabled={disabled || busy !== ''} onChange={event => setDraftText(event.target.value)} aria-describedby="config-validation"/></label>
      <p id="config-validation" className={`small-print ${validation.state === 'invalid' ? 'password-error' : ''}`} role={validation.state === 'invalid' ? 'alert' : undefined}>{validation.state === 'invalid' ? t('config.invalid', { message: validation.message }) : validation.state === 'valid' ? t('config.valid') : t('config.unchanged')}</p>
      <div className="button-row"><button className="text-button" disabled={validation.state === 'invalid' || draftText === document.document} onClick={() => setShowDiff(value => !value)} aria-expanded={showDiff}><Icon name="chevron" size={13}/>{showDiff ? t('config.hideDiff') : t('config.showDiff')}</button><button className="primary" disabled={disabled || busy !== '' || validation.state !== 'valid'} onClick={() => void edit()}>{busy === 'edit' ? t('config.planning') : t('config.previewEdit')}<Icon name="arrow" size={14}/></button></div>
      {showDiff && diff.length > 0 && <pre className="config-diff" aria-label={t('config.diffAria')}>{diff.map((line, index) => <div key={index} className={`diff-${line.kind}`}>{line.kind === 'add' ? '+ ' : line.kind === 'remove' ? '- ' : '  '}{line.text}</div>)}</pre>}
      <Notice>{t('config.editNote')}</Notice>
      <details className="config-restore"><summary>{t('config.restoreTitle')}</summary><p className="small-print">{t('config.restoreNote')}</p><div className="config-restore-job"><label className="field">{t('config.restoreJob')}<input value={restoreJobId} disabled={disabled || busy !== ''} onChange={event => { setRestoreJobId(event.target.value); setRestore(null); setRestoreSelected(new Set()); }} placeholder={t('config.restorePlaceholder')} spellCheck={false}/></label><button disabled={disabled || busy !== '' || !restoreJobId.trim()} onClick={() => void inspectRestore()}>{busy === 'restoreInspect' ? t('config.restoring') : t('config.restoreInspect')}</button></div>
        {restore && (restore.fields.length === 0 ? <Notice>{t('config.restoreNone')}</Notice> : <>
          <p className="small-print">{restore.note ?? t('config.restoreFieldsNote')}</p>
          <ul className="config-restore-fields">{restore.fields.map(field => <li key={field.key}>
            <label className="checkbox-inline"><input type="checkbox" disabled={disabled || busy !== '' || !field.available} checked={restoreSelected.has(field.key)} onChange={event => setRestoreSelected(current => { const next = new Set(current); event.target.checked ? next.add(field.key) : next.delete(field.key); return next; })}/><span><strong>{field.label ?? field.key}</strong><code>{field.key}</code>{!field.available && <em className="password-error">{t('config.restoreUnavailable', { reason: field.reason ?? t('config.restoreUnknownReason') })}</em>}</span></label>
            <div className="compare-values"><code>{field.masked?'••••••':field.before ?? t('snapshot.notSet')}</code><Icon name="arrow" size={13}/><code>{field.masked?'••••••':field.after ?? t('snapshot.notSet')}</code></div>
          </li>)}</ul>
          <button className="primary" disabled={disabled || busy !== '' || restoreSelected.size === 0} onClick={() => void restorePlan()}>{busy === 'restore' ? t('config.restoring') : t('config.restorePreview')}<Icon name="arrow" size={14}/></button>
        </>)}
      </details>
    </div>}
  </section>;
}
