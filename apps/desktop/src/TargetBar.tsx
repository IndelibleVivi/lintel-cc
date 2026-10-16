import { useEffect, useId, useRef, useState, type ReactNode } from 'react';
import type { Environment } from './types';
import { transport } from './api';
import { Icon } from './ui';
import { useI18n } from './i18n-provider';

export default function TargetBar({ environments, selected, selectedId, hostAlias, busy, loading,
  onSelect, onRemote, onRefresh, onCopy, children }: {
  environments: Environment[]; selected: Environment | undefined; selectedId: string;
  hostAlias: string | null; busy: boolean; loading: boolean;
  onSelect: (id: string) => void; onRemote: () => void; onRefresh: () => void;
  onCopy: (value: string) => void | Promise<void>; children: ReactNode;
}) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const detailsId = useId();
  const trigger = useRef<HTMLButtonElement>(null);
  useEffect(() => { setOpen(false); }, [hostAlias, selectedId]);
  useEffect(() => {
    if (!open) return;
    function escape(event: KeyboardEvent) {
      if (event.key !== 'Escape' || event.defaultPrevented || document.querySelector('dialog[open]')) return;
      event.preventDefault();
      setOpen(false);
      trigger.current?.focus({ preventScroll: true });
    }
    document.addEventListener('keydown', escape);
    return () => {
      document.removeEventListener('keydown', escape);
    };
  }, [open]);
  const host = hostAlias ? `SSH · ${hostAlias}` : t('app.localLabel');
  function path(value: string) {
    return <div className="target-detail-path"><code>{value}</code><button className="icon-button" aria-label={t('label.copyFullPath')} onClick={() => void onCopy(value)}><Icon name="copy" size={14}/></button></div>;
  }
  return <div className="target-chrome"><header className="target-bar">
    <div className="target-context"><div className="target-line">
      <button className="host-switch" disabled={busy} onClick={onRemote} title={host}>{hostAlias ?? t('app.localLabel')}<Icon name="chevron" size={11}/></button>
      <span className="slash" aria-hidden="true">/</span>
      <div className="target-environment-picker"><select aria-label={t('app.currentEnvLabel')} disabled={busy} value={selectedId} onChange={event => onSelect(event.target.value)}>
        {!environments.length && <option value="">{t('app.noEnvSelected')}</option>}
        {environments.map(env => <option key={env.id} value={env.id}>{env.name}</option>)}
      </select><Icon name="chevron" size={11}/></div>
    </div></div>
    {children}
    <button className="icon-button refresh-button" aria-label={t('app.recheckEnv')} disabled={busy || loading} onClick={onRefresh}><Icon name="refresh" size={16}/></button>
  </header>
      <div className="target-details-container">
        <button ref={trigger} className="target-details-toggle" aria-label={t('app.targetDetails')} title={t('app.targetDetails')} aria-expanded={open} aria-controls={detailsId} onClick={() => setOpen(value => !value)}>
          <svg className="target-terminal-mark" width="22" height="18" viewBox="0 0 22 18" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="m4.5 4.75 5 4.25-5 4.25"/><path className="target-cursor" d="M13 13.25h5"/></svg>
        </button>
        {open && <section id={detailsId} className="target-details-panel" role="region" aria-label={t('app.targetDetails')}>
          <div className="target-details-heading"><strong>{t('app.currentTarget')}</strong><button className="icon-button" aria-label={t('app.close')} onClick={() => { setOpen(false); trigger.current?.focus({ preventScroll: true }); }}><Icon name="close" size={15}/></button></div>
          <dl><div><dt>{t('app.targetHost')}</dt><dd>{host}</dd></div>
            <div><dt>{t('app.currentEnvLabel')}</dt><dd>{selected?.name ?? t('app.noEnvSelected')}</dd></div>
            {selected && <><div><dt>{t('launchPanel.configRoot')}</dt><dd>{path(selected.root)}</dd></div>
              <div><dt>{t('app.targetEnvironmentId')}</dt><dd><code>{selected.id}</code></dd></div>
              {selected.executable && <div><dt>{t('launchPanel.client')}</dt><dd>{path(selected.executable)}</dd></div>}</>}
          </dl>
          <p className="target-runtime"><span className="status-dot"/>{t(hostAlias ? 'app.remoteWorkspace' : transport === 'native' ? 'app.localRunner' : transport === 'synthetic' ? 'app.syntheticRunner' : 'app.desktopUnavailable')}</p>
        </section>}
      </div>
  </div>;
}
