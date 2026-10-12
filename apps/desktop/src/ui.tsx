import { createContext, useContext, useEffect, useRef, useId, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import type { Receipt } from './types';
import { useI18n } from './i18n-provider';

export const WorkspaceActionsContext = createContext<HTMLElement | null>(null);
export function WorkspaceActions({ children }: { children: ReactNode }) {
  const target = useContext(WorkspaceActionsContext);
  const bar = <div className="action-bar">{children}</div>;
  return target ? createPortal(bar, target) : bar;
}

export function hasConfirmedWorkArchive(receipt: Receipt): boolean {
  return !!receipt.archive_path && (!!receipt.archive_digest || receipt.steps.some(step => step.id === 'archive' && step.status === 'completed'));
}

export type IconName = 'sidebar' | 'environments' | 'policy' | 'rebuild' | 'history' | 'settings' | 'help' | 'plus' | 'arrow' | 'check' | 'refresh' | 'copy' | 'close' | 'terminal' | 'chevron' | 'warning' | 'sun' | 'moon' | 'download' | 'language';
const paths: Record<IconName, ReactNode> = {
  sidebar: <><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16"/></>,
  environments: <><rect x="3" y="4" width="18" height="13" rx="2"/><path d="M8 21h8m-4-4v4"/></>,
  policy: <><path d="M4 6h16M4 12h16M4 18h16"/><circle cx="8" cy="6" r="2"/><circle cx="16" cy="12" r="2"/><circle cx="10" cy="18" r="2"/></>,
  rebuild: <><path d="M5 9a8 8 0 1 1-1 7M5 3v6h6"/><path d="M10 16v-5h5v5"/></>,
  history: <><path d="M3 11a9 9 0 1 1 3 8M3 5v6h6"/><path d="M12 7v5l3 2"/></>,
  settings: <><path d="m10 3-1 3-3 1-2-1-2 4 2 2v3l-1 2 3 3 3-1 3 1 1 2h4l1-3 2-2 2-1-1-4-2-1-1-3-3-2-2 1Z"/><circle cx="12" cy="12" r="3"/></>,
  help: <><circle cx="12" cy="12" r="9"/><path d="M9.5 9a2.5 2.5 0 0 1 5 0c0 2-2.5 2-2.5 4m0 3h.01"/></>,
  plus: <path d="M12 5v14M5 12h14"/>, arrow: <path d="M4 12h16m-6-6 6 6-6 6"/>,
  check: <path d="m5 12 4 4L19 6"/>, refresh: <><path d="M20 7v5h-5M4 17v-5h5"/><path d="M6 6a8 8 0 0 1 14 6M4 12a8 8 0 0 0 14 6"/></>,
  copy: <><rect x="8" y="8" width="12" height="13" rx="2"/><path d="M16 8V3H3v13h5"/></>, close: <path d="m6 6 12 12M6 18 18 6"/>,
  terminal: <><rect x="2" y="4" width="20" height="16" rx="3"/><path d="m6 9 3 3-3 3m6 0h5"/></>, chevron: <path d="m9 5 7 7-7 7"/>,
  warning: <><path d="m12 3 10 18H2Z"/><path d="M12 9v5m0 3h.01"/></>,
  sun: <><circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1 1m12 12 1 1M5 19l1-1M18 6l1-1"/></>,
  moon: <path d="M20 14a8 8 0 0 1-10-10A9 9 0 1 0 20 14Z"/>,
  download: <><path d="M12 3v12m-5-5 5 5 5-5M4 16v5h16v-5"/></>,
  language: <><circle cx="12" cy="12" r="9"/><ellipse cx="12" cy="12" rx="4" ry="9"/><path d="M3 12h18"/></>,
};
export function Icon({ name, size = 18 }: { name: IconName; size?: number }) { return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.65" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">{paths[name]}</svg>; }
export function Brand() { return <svg className="brand-mark" viewBox="0 0 32 32" fill="none" aria-hidden="true"><rect className="brand-beam" x="4" y="8.5" width="23" height="4"/><rect className="brand-support" x="6.5" y="12.5" width="4" height="10.5"/><rect className="brand-support" x="20.5" y="12.5" width="3.5" height="10.5"/><rect className="brand-mark-cursor" x="24.75" y="21" width="4" height="2"/></svg>; }

// Finite status vocabulary shared by App, panels and the CLI capability map.
// Codes are stable; the label text is resolved through the active catalog so a
// raw backend code is never shown when a translation exists. Unmapped codes
// fall back to the code itself (raw data preserved).
const statusKeys: Record<string, string> = {
  launch_attempt: 'status.launchAttempt', resume_attempt: 'status.resumeAttempt', launch_intent: 'status.launchIntent', resume_intent: 'status.resumeIntent',
  resume_prepared: 'status.resumePrepared', launch_failed: 'status.launchFailed', uncertain: 'status.uncertain', verifying: 'status.verifying',
  interrupted: 'status.interrupted', expired: 'status.expired', cancelled: 'status.cancelled', superseded: 'status.superseded',
  requires_explicit_target: 'status.requiresExplicitTarget', linux_runner_only: 'status.linuxRunnerOnly', needs_reconciliation: 'status.needsReconciliation',
  launch_requested: 'status.launchRequested', terminal_required: 'status.terminalRequired', submission_unknown: 'status.submissionUnknown',
  query_only: 'status.queryOnly', response_received: 'status.responseReceived', separate_module: 'status.separateModule', not_delivered: 'status.notDelivered',
  awaiting_browser_restart: 'status.awaitingBrowserRestart', not_completed: 'status.notCompleted', not_present: 'status.notPresent',
  retired: 'status.retired', reactivated: 'status.reactivated', completed: 'status.completed', partially_completed: 'status.partiallyCompleted',
  failed: 'status.failed', configured: 'status.configured', discovered: 'status.discovered', registered: 'status.registered', ready: 'status.ready',
  available: 'status.available', limited: 'status.limited', unsupported: 'status.unsupported', unverified: 'status.unverified', pending: 'status.pending',
  executing: 'status.executing', accepted: 'status.accepted', verified: 'status.verified', unchanged: 'status.unchanged', clean: 'status.clean',
  drifted: 'status.drifted', changed: 'status.changed', unknown: 'status.unknown', preserved: 'status.preserved', skipped: 'status.skipped',
  needs_restart: 'status.needsRestart', launched: 'status.launched', not_configured: 'status.notConfigured', owned: 'status.owned', external: 'status.external',
  user: 'status.user', local: 'status.local', immediate: 'status.immediate', not_enforced: 'status.notEnforced', planned: 'status.planned', not_run: 'status.notRun',
};

// Codes that render as fixed proper nouns rather than a translated phrase.
const statusLiterals: Record<string, string> = { 'claude-code': 'Claude Code', 'next_launch': 'status.nextLaunch', 'next-launch': 'status.nextLaunch' };

export function useLabel(): (value: string) => string {
  const { t } = useI18n();
  return (value: string) => {
    const literal = Object.hasOwn(statusLiterals, value) ? statusLiterals[value] : undefined;
    if (literal) return literal.startsWith('status.') ? t(literal) : literal;
    const key = Object.hasOwn(statusKeys, value) ? statusKeys[value] : undefined;
    return key ? t(key) : value;
  };
}


// Accessible System / 简体中文 / English selector. The control only changes the
// interface language; it never issues a core operation or reopens the App.
export function LanguageSwitch({ compact = false }: { compact?: boolean }) {
  const { t, setting, setLanguage } = useI18n();
  const options: { value: 'system' | 'zh-CN' | 'en'; label: string }[] = [
    { value: 'system', label: t(compact ? 'language.systemShort' : 'language.system') },
    { value: 'zh-CN', label: '简体中文' },
    { value: 'en', label: 'English' },
  ];
  return <label className="field language-switch"><span className="language-switch-label">{t('language.select')}</span><span className="language-control"><Icon name="language" size={14}/><select aria-label={t('language.select')} value={setting} onChange={event => setLanguage(event.target.value as 'system' | 'zh-CN' | 'en')}>{options.map(option => <option key={option.value} value={option.value}>{option.label}</option>)}</select><Icon name="chevron" size={11}/></span><small>{t('language.hint')}</small></label>;
}

export function Status({ value }: { value: string }) {
  const labelText = useLabel();
  const good = ['completed','configured','ready','available','verified','unchanged','clean','preserved'].includes(value);
  const tone = good ? 'good' : ['failed', 'uncertain', 'launch_failed'].includes(value) ? 'error' : ['needs_reconciliation', 'submission_unknown', 'interrupted', 'partially_completed','launch_intent','resume_intent','resume_prepared','launch_attempt','resume_attempt'].includes(value) ? 'attention' : '';
  return <span className={`status ${tone}`}><span className="status-dot"/>{labelText(value)}</span>;
}
export function Notice({ children, tone = 'neutral' }: { children: ReactNode; tone?: 'neutral'|'warning'|'error' }) { return <div className={`notice ${tone}`}><Icon name={tone === 'neutral' ? 'help' : 'warning'} size={17}/><div>{children}</div></div>; }
export function Modal({ title, children, onClose, wide = false }: { title: string; children: ReactNode; onClose: () => void; wide?: boolean }) {
  const { t } = useI18n();
  const ref = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  useEffect(() => { const previous = document.activeElement as HTMLElement | null; ref.current?.showModal(); return () => { ref.current?.close(); if (previous?.isConnected) previous.focus(); else requestAnimationFrame(()=>{if(document.querySelector('dialog[open]'))return;const target=document.querySelector<HTMLElement>('main h1,main h2');if(target){target.tabIndex=-1;target.focus({preventScroll:true})}}); }; }, []);
  // Reset scroll only when the dialog opens, not on a language-only title
  // change (which would jump a scrolled approval/dialog back to the top).
  useEffect(() => { ref.current?.querySelector('.modal-body')?.scrollTo({ top: 0 }); }, []);
  return <dialog ref={ref} className={wide ? 'modal wide' : 'modal'} onCancel={event => { event.preventDefault(); onClose(); }} aria-labelledby={titleId}><div className="modal-heading"><h2 id={titleId}>{title}</h2><button autoFocus className="icon-button" aria-label={t('label.closePanel')} onClick={onClose}><Icon name="close"/></button></div>{children}</dialog>;
}
export function formatBytes(value: number) { return value < 1024 ? `${value} B` : value < 1024 ** 2 ? `${(value / 1024).toFixed(1)} KB` : `${(value / 1024 ** 2).toFixed(1)} MB`; }
