import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Modal, Notice } from './ui';
import { ResourceLink } from './Resources';
import { useI18n, type Translator } from './i18n-provider';
import './app-updates.css';

type UpdateStatus = {
  current_version: string; channel: 'preview' | 'stable' | null; configured: boolean;
  phase: string; background_check: boolean; checked_at: number | null; failure: string | null;
  candidate: { id: string; version: string; notes: string | null; url: string; signature_verified: boolean } | null;
  downloaded_bytes: number; total_bytes: number | null;
  last_install: { id: string; from_version: string; to_version: string; status: string } | null;
  installation_unresolved: boolean;
};
type Envelope = { ok: true; data: UpdateStatus } | { ok: false; error: { code: string; message: string } };
async function send(payload: Record<string, unknown>, t: Translator) {
  const result = await invoke<Envelope>('app_update_request', { payload });
  if (!result.ok) throw new Error(t('app.diagnosticWithCode', { message: result.error.message, code: result.error.code }));
  return result.data;
}
const phaseKeys: Record<string, string> = {
  idle: 'updates.phase.idle', unconfigured: 'updates.phase.unconfigured', checking: 'updates.phase.checking', latest: 'updates.phase.latest',
  available: 'updates.phase.available', downloading: 'updates.phase.downloading', verified: 'updates.phase.verified',
  installing: 'updates.phase.installing', installed: 'updates.phase.installed', uncertain: 'updates.phase.uncertain', error: 'updates.phase.error',
};

// Always mounted by App: opt-in checks survive closing Settings. Neither check
// nor download installs anything; installation and restart have native gates.
export default function AppUpdates({ visible, disabled, onClose }: { visible: boolean; disabled: boolean; onClose: () => void }) {
  const { t } = useI18n();
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [pending, setPending] = useState('');
  const [failure, setFailure] = useState('');
  const [approve, setApprove] = useState(false);
  const active = useRef(true);
  const statusSequence = useRef(0);
  const inFlight = useRef(false);
  const nextBackgroundCheck = useRef(0);
  const backgroundConsent = useRef(false);
  const read = useCallback(async () => {
    const sequence = ++statusSequence.current;
    try { const value = await send({ op: 'status' }, t); if (active.current && sequence === statusSequence.current) { backgroundConsent.current = value.background_check; setStatus(value); } }
    catch (error) { if (active.current && visible) setFailure(error instanceof Error ? error.message : String(error)); }
  }, [visible, t]);
  useEffect(() => { active.current = true; return () => { active.current = false; }; }, []);
  useEffect(() => { void read(); }, [read]);
  const run = useCallback(async (op: string, fields: Record<string, unknown> = {}) => {
    if (inFlight.current) return;
    inFlight.current = true; ++statusSequence.current; setPending(op); setFailure('');
    if (op === 'preference' && fields.background_check === false) backgroundConsent.current = false;
    try {
      const value = await send({ op, ...fields }, t);
      backgroundConsent.current = value.background_check;
      ++statusSequence.current;
      if (active.current) { setStatus(value); setApprove(false); }
    } catch (error) {
      if (active.current) setFailure(error instanceof Error ? error.message : String(error));
      await read();
    } finally { inFlight.current = false; if (active.current) setPending(''); }
  }, [read]);
  useEffect(() => {
    if (!pending || !['check', 'download', 'install'].includes(pending)) return;
    const timer = setInterval(() => void read(), 1000);
    return () => clearInterval(timer);
  }, [pending, read]);
  useEffect(() => {
    if (!status?.configured || !status.background_check || status.phase === 'installed') return;
    const check = () => {
      if (!backgroundConsent.current || document.visibilityState !== 'visible' || inFlight.current || Date.now() < nextBackgroundCheck.current) return;
      nextBackgroundCheck.current = Date.now() + 60 * 60 * 1000;
      void run('check');
    };
    check();
    const timer = setInterval(check, 60 * 1000);
    window.addEventListener('focus', check); document.addEventListener('visibilitychange', check);
    return () => { clearInterval(timer); window.removeEventListener('focus', check); document.removeEventListener('visibilitychange', check); };
  }, [status?.configured, status?.background_check, status?.phase === 'installed', run]);
  useEffect(() => { setApprove(false); }, [status?.candidate?.id]);
  if (!visible) return null;
  const candidate = status?.candidate;
  const busy = !!pending;
  const progress = status?.total_bytes ? Math.min(100, Math.round(status.downloaded_bytes / status.total_bytes * 100)) : null;
  return <Modal title={t('updates.title')} wide onClose={busy ? () => undefined : onClose}>
    <div className="modal-body app-updates" aria-busy={busy}>
      <div className="update-version"><div><span className="eyebrow">app / version</span><h3>{status?.current_version ?? t('updates.readingVersion')}</h3></div><span className="small-label">{status?.channel === 'stable' ? 'Stable' : status?.channel === 'preview' ? 'Preview' : t('updates.noChannel')}</span></div>
      <p role="status">{status ? (phaseKeys[status.phase] ? t(phaseKeys[status.phase]) : status.phase) : t('updates.readingStatus')}</p>
      {failure && <Notice tone="error">{failure}</Notice>}
      {status?.failure && status.failure !== failure && <Notice tone="error">{status.failure}</Notice>}
      {!status?.configured && <Notice>{t('updates.unconfiguredNote')}</Notice>}
      <div className="button-row"><button disabled={busy} onClick={() => void run('check')}>{pending === 'check' ? t('updates.checking') : t('updates.check')}</button>{status?.checked_at && <span className="small-print">{t('updates.lastCheck',{time:new Date(status.checked_at * 1000).toLocaleString()})}</span>}</div>
      <label className="update-preference"><input type="checkbox" checked={status?.background_check ?? false} disabled={!status || busy || !status.configured} onChange={event => void run('preference', { background_check: event.target.checked })}/><span>{t('updates.backgroundCheck')}<small>{t('updates.backgroundCheckNote')}</small></span></label>
      <p className="small-print">{t('updates.netNote')}</p>
      <ResourceLink resource="app-updates">{t('updates.resource')}</ResourceLink>
      {candidate && <section className="update-candidate" aria-label={t('updates.candidateAria')}>
        <div className="surface-heading"><h3>Lintel {candidate.version}</h3><span className="small-label">{t(candidate.signature_verified ? 'updates.verifiedLabel' : 'updates.pendingVerify')}</span></div>
        {candidate.notes && <p className="update-notes">{candidate.notes}</p>}
        <p className="small-print">{t('updates.source')} <code>{candidate.url}</code></p>
        {status?.phase === 'downloading' && <><progress aria-label={t('updates.progressAria')} max={100} value={progress ?? undefined}/><p className="small-print">{t('updates.downloaded',{miB:(status.downloaded_bytes / 1024 / 1024).toFixed(1),progress:progress !== null ? ` · ${progress}%` : ''})}</p></>}
        {!candidate.signature_verified && <button disabled={busy || status?.installation_unresolved} onClick={() => void run('download', { candidate_id: candidate.id })}>{pending === 'download' ? t('updates.downloading') : t('updates.downloadVerify')}</button>}
        {candidate.signature_verified && status?.phase !== 'installed' && <>
          <Notice>{t('updates.installNote')}</Notice>
          <label className="update-preference"><input type="checkbox" checked={approve} disabled={busy || disabled || status?.installation_unresolved} onChange={event => setApprove(event.target.checked)}/><span>{t('updates.reviewedInstall')}</span></label>
          <button className="primary" disabled={!approve || busy || disabled || status?.installation_unresolved} onClick={() => void run('install', { candidate_id: candidate.id })}>{pending === 'install' ? t('updates.installing') : t('updates.installVerified')}</button>
        </>}
      </section>}
      {disabled && <Notice>{t('updates.busyNote')}</Notice>}
      {status?.last_install && <div className="update-record"><strong>{t('updates.lastInstallTitle',{version:status.last_install.to_version})}</strong><code>{status.last_install.id}</code><span>{status.last_install.status === 'installed' ? t('updates.lastInstallInstalled') : t('updates.lastInstallOther',{status:status.last_install.status})}</span></div>}
      {status?.installation_unresolved && <Notice tone="error">{t('updates.unresolvedNote')}</Notice>}
    </div>
    <div className="modal-footer"><button disabled={busy} onClick={onClose}>{t('app.done')}</button>{status?.phase === 'installed' && status.last_install && <button className="primary" disabled={busy || disabled} onClick={() => void run('restart', { candidate_id: status.last_install!.id })}>{t('updates.restart')}</button>}</div>
  </Modal>;
}
