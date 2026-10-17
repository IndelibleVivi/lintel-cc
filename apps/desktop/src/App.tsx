import { LocalizedError } from './i18n';
import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore, type FormEvent } from 'react';
import { requester, RequestError, transport, subscribePackageActivity, getPackageActivity } from './api';
import BrowserPanel from './BrowserPanel';
import Clawd from './Clawd';
import type { ClawdDestination } from './ClawdFlightMenu';
import ClawdPlayroom from './ClawdPlayroom';
import ArchivePanel from './ArchivePanel';
import AgentPanel, { type HandoffContext } from './AgentPanel';
import LaunchPanel from './LaunchPanel';
import LaunchRecords from './LaunchRecords';
import { buildIdentity } from './buildIdentity';
import CleanupPanel from './CleanupPanel';
import WorkPanel from './WorkPanel';
import ComponentPanel from './ComponentPanel';
import { ServiceEvidence } from './ServiceControl';
import RemotePanel, { remoteRequest } from './RemotePanel';
import NetworkPanel from './NetworkPanel';
import AppUpdates from './AppUpdates';
import { NetworkPlanReview, NetworkReceiptEvidence } from './NetworkEvidence';
import DeveloperResources, { ResourceLink, TaskHelp, productTasks, useTaskHelp } from './Resources';
import PolicyEvidence from './PolicyEvidence';
import PlanReview from './PlanReview';
import ExecutionEvidence from './ExecutionEvidence';
import TaskOutcome from './TaskOutcome';
import CustomPolicy from './CustomPolicy';
import { editPolicyDraft, pendingPolicyEdits } from './policyEditor';
import ResiduePanel from './ResiduePanel';
import SnapshotPanel from './SnapshotPanel';
import ConfigEditor from './ConfigEditor';
import Recipes from './Recipes';
import TargetBar from './TargetBar';
import { applyMotionPreferences, readMotionPreferences, type MotionPreferences } from './motion';
import {saveRecipe} from './recipeIntent';
import type { CleanupRecipeIntent, PolicyRecipeIntent, ResidueRecipeIntent } from './recipeIntent';
import RequestFailure, { asError } from './RequestFailure';
import type { Capability, Draft, Drift, Environment, Inspection, Plan, Receipt } from './types';
import { Brand, Icon, Modal, Notice, Status, WorkspaceActions, WorkspaceActionsContext, formatBytes, hasConfirmedWorkArchive, type IconName } from './ui';
import { LanguageSwitch, useLabel } from './ui';
import { useI18n } from './i18n-provider';
import { present } from './i18n-presentation';
import type { Greeting } from './homeGreetings';

type Page = 'environments' | 'policy' | 'work' | 'rebuild' | 'history' | 'sessions' | 'agent';
type Tab = 'overview' | 'outbound' | 'state' | 'compare' | 'launch' | 'components';
type Theme = 'light' | 'dark' | 'system';
type Flow = { hostAlias?: string | null; environment: Environment | null; plan?: Plan; receipt?: Receipt; uncertain?: boolean; initialPassphrase?: string };
// Navigation labels resolve through the catalog at render; ids stay stable so
// page/task state is never rebuilt on a language switch.
const nav: { id: Page; key: string; icon: IconName }[] = [ { id: 'environments', key: 'nav.environments', icon: 'environments' }, { id: 'policy', key: 'nav.policy', icon: 'policy' }, { id: 'work', key: 'nav.work', icon: 'environments' }, { id: 'rebuild', key: 'nav.rebuild', icon: 'rebuild' }, { id: 'history', key: 'nav.history', icon: 'history' }, { id: 'sessions', key: 'nav.sessions', icon: 'environments' }, { id: 'agent', key: 'nav.agent', icon: 'terminal' } ];
const defaultDraft: Draft = { preset: 'reduce', keepRemoteControl: false };
function safeDrafts(value:Record<string,Draft>):Record<string,Draft> {
  return Object.fromEntries(Object.entries(value).filter(([,draft])=>draft&&typeof draft==='object').map(([key,draft])=>[key,{preset:['preserve','reduce','custom'].includes(draft.preset)?draft.preset:'reduce',keepRemoteControl:draft.keepRemoteControl===true,trustedDevices:draft.trustedDevices,releaseSettings:draft.releaseSettings,customSettings:draft.customSettings}]));
}
function loadDrafts(): Record<string, Draft> { try { const value=JSON.parse(localStorage.getItem('lintel.drafts') ?? '{}');return value&&typeof value==='object'&&!Array.isArray(value)?safeDrafts(value):{}; } catch { return {}; } }
function useErrorText() { const { t } = useI18n(); return (error: unknown) => error instanceof LocalizedError ? t(error.key,error.args) : typeof error === 'string' ? error : error instanceof RequestError ? t('app.diagnosticWithCode',{message:error.message,code:error.code}) : error instanceof Error ? error.message : t('app.operationIncomplete'); }

export default function App({ greeting }: { greeting: Greeting }) {
  const { t, locale, formatDateTime: fmtDate, formatTime: fmtTime } = useI18n();
  // Projection of finite, source-controlled backend display strings; unknown
  // values pass through raw (see i18n-presentation.ts).
  const p = (value: string | null | undefined) => present(value, locale);
  const errorText = useErrorText();
  // Finite task metadata: labels/summaries project through the additive
  // task-catalog English metadata (never re-translated on the fly).
  const taskHelp = useTaskHelp();
  // Render the same chosen greeting identity through the active catalog; a
  // language switch translates it rather than drawing a new one.
  const greetingText = t(greeting.key);
  const [hostAlias, setHostAlias] = useState<string | null>(null);
  const request = useMemo(() => requester(hostAlias), [hostAlias]);
  const packageActivity = useSyncExternalStore(subscribePackageActivity, getPackageActivity);
  const activeHost = useRef(hostAlias); activeHost.current = hostAlias;
  const mainRef = useRef<HTMLElement>(null);
  const [actionsMount, setActionsMount] = useState<HTMLElement | null>(null);
  const [playroom, setPlayroom] = useState<'landscapes' | 'runner' | null>(null);
  const [page, setPage] = useState<Page>('environments');
  const [activeTask,setActiveTask]=useState<string|null>(()=>{const saved=localStorage.getItem('lintel.goal');return productTasks.some(t=>t.id===saved)?saved:null});
  const [tab, setTab] = useState<Tab>('overview');
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [showInspector, setShowInspector] = useState(false);
  const [environments, setEnvironments] = useState<Environment[]>([]);
  const [capabilities, setCapabilities] = useState<Capability[]>([]);
  const [selectedId, setSelectedId] = useState(localStorage.getItem('lintel.selected') ?? '');
  const [inspection, setInspection] = useState<Inspection | null>(null);
  const [jobs, setJobs] = useState<Receipt[]>([]);
  const [drafts, setDrafts] = useState<Record<string, Draft>>(loadDrafts);
  const [busy, setBusy] = useState('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | string>('');
  // Finite notification code (never a translated string) so it re-renders in the
  // current locale after a language switch.
  const [notification, setNotification] = useState<'' | 'copied' | 'envCreated' | 'envRegistered' | 'supportSaved' | 'reactivated' | 'policyRecipeSaved'>('');
  const [flow, setFlowState] = useState<Flow | null>(null);
  const [handoff, setHandoff] = useState<HandoffContext | null>(null);
  const [launchTarget, setLaunchTarget] = useState<{environment: Environment; hostAlias: string|null;proxyUrl?:string} | null>(null);
  const [cleanupRecipeSeed, setCleanupRecipeSeed] = useState<{ key: string; intent: CleanupRecipeIntent } | null>(null);
  // A residue recipe seeds the residue panel: whether it should also turn off
  // content recording and/or reset the local identifier in ONE combined plan.
  const [residueRecipe, setResidueRecipe] = useState<{ key: string; intent: ResidueRecipeIntent } | null>(null);
  function setFlow(value: Flow | null | ((old: Flow|null) => Flow|null)) {
    setFlowState(old => { const next = typeof value === 'function' ? value(old) : value; if(!next || next===old)return next;return { ...next, hostAlias: 'hostAlias' in next ? next.hostAlias : hostAlias, environment: next.environment ? {...next.environment} : null }; });
  }
  function openAgent(context?: Flow) {
    setHandoff({ task: context?.receipt || context?.uncertain ? 'recover_results' : page==='work' || page==='sessions' ? 'preserve_work' : page==='rebuild' ? 'repair_cleanup_retire' : activeTask ?? 'reduce_egress', hostAlias: context ? context.hostAlias ?? null : hostAlias, environment: context ? context.environment ?? undefined : selected, plan: context?.plan, receipt: context?.receipt, uncertain: context?.uncertain, observedAt: new Date().toISOString() });
    setFlow(null); setDialog(null); setPage('agent');
  }
  const [archivePassphrase, setArchivePassphrase] = useState('');
  const [archiveConfirmation, setArchiveConfirmation] = useState('');
  const [dialog, setDialog] = useState<'add' | 'settings' | 'help' | 'support' | 'browser' | 'remote' | 'tasks' | 'updates' | null>(null);
  const [addMode, setAddMode] = useState<'register'|'create'>('create');
  const [name, setName] = useState('');
  const [root, setRoot] = useState('');
  const [formError, setFormError] = useState<Error | string>('');
  const [archiveJobId, setArchiveJobId] = useState<string | undefined>();
  const [support, setSupport] = useState('');
  const [theme, setTheme] = useState<Theme>((localStorage.getItem('lintel.theme') as Theme) ?? 'system');
  const [motion, setMotion] = useState<MotionPreferences>(readMotionPreferences);
  const [queryId, setQueryId] = useState('');
  const [recipeSaveResult, setRecipeSaveResult] = useState<{ target: string; omitted: string[]; saved: boolean } | null>(null);
  const [historySearch, setHistorySearch] = useState('');
  const [lastCheck, setLastCheck] = useState<Date | null>(null);
  // A dismissed quiet drift tip stays dismissed for this host/environment/root
  // generation; a new generation (host or root switch) may show it again.
  const [driftTipDismissed, setDriftTipDismissed] = useState('');
  const [receiptFilter, setReceiptFilter] = useState<'all'|'target'|'attention'>('all');
  const executionLock = useRef(false);
  useEffect(() => { setArchivePassphrase(flow?.receipt ? '' : flow?.initialPassphrase ?? ''); setArchiveConfirmation(flow?.receipt ? '' : flow?.initialPassphrase ?? ''); }, [flow?.plan?.id, flow?.receipt?.id]);
  const activeGoal=productTasks.find(task=>task.id===activeTask);
  useEffect(()=>{if(activeTask)localStorage.setItem('lintel.goal',activeTask);else localStorage.removeItem('lintel.goal')},[activeTask]);
  const selected = environments.find(env => env.id === selectedId);
  const currentHandoff=useMemo<HandoffContext>(()=>({task:activeTask ?? 'recover_results',hostAlias,environment:selected,observedAt:new Date().toISOString()}),[activeTask,hostAlias,selected?.id,selected?.root]);
  const draftKey = hostAlias ? `ssh:${hostAlias}:${selectedId}` : selectedId;
  const draft = drafts[draftKey] ?? defaultDraft;
  const policyEdits = pendingPolicyEdits(inspection?.policy?.rules ?? [], draft.customSettings ?? {}, draft.customValues ?? {});
  const targetJobs = jobs.filter(job => job.environment_id === selectedId || !!job.network_change);
  const attentionJobs = jobs.filter(job => ['accepted', 'executing', 'verifying', 'pending', 'interrupted', 'needs_reconciliation', 'failed', 'partially_completed'].includes(job.status));
  const visibleJobs = (receiptFilter === 'target' ? targetJobs : receiptFilter === 'attention' ? attentionJobs : jobs).filter(job => `${job.title} ${job.id} ${environments.find(env => env.id === job.environment_id)?.name ?? ''}`.toLowerCase().includes(historySearch.toLowerCase()));

  useEffect(()=>{mainRef.current?.scrollTo({top:0});requestAnimationFrame(()=>{if(document.querySelector('dialog[open]'))return;const heading=mainRef.current?.querySelector<HTMLElement>('h1,h2');if(heading){heading.tabIndex=-1;heading.focus({preventScroll:true})}})},[page]);
  useEffect(() => { document.documentElement.dataset.theme = theme; localStorage.setItem('lintel.theme', theme); }, [theme]);
  useEffect(() => { applyMotionPreferences(motion); }, [motion]);
  useEffect(() => { localStorage.setItem('lintel.drafts', JSON.stringify(safeDrafts(drafts))); }, [drafts]);
  useEffect(() => { localStorage.setItem('lintel.selected', selectedId); }, [selectedId]);
  useEffect(() => { if (!notification) return; const timeout = setTimeout(() => setNotification(''), 6000); return () => clearTimeout(timeout); }, [notification]);

  useEffect(() => {
    if (error && !flow) mainRef.current?.querySelector('.error-wrap')?.scrollIntoView({ block: 'start', behavior: 'instant' });
  }, [error, flow]);

  const refresh = useCallback(async () => {
    const [inventory, history] = await Promise.all([request('discover', {}), request('jobs', {})]);
    if (activeHost.current !== hostAlias) return;
    setEnvironments(inventory.environments); setCapabilities(inventory.capabilities); setJobs(history.jobs);
    setSelectedId(current => inventory.environments.some(env => env.id === current) ? current : inventory.environments[0]?.id ?? '');
    setLastCheck(new Date());
  }, [request, hostAlias]);
  useEffect(() => { let active = true; setLoading(true); setInspection(null); setEnvironments([]); setJobs([]); refresh().catch(error => { if (active) setError(asError(error)); }).finally(() => { if (active) setLoading(false); }); return () => { active = false; }; }, [refresh]);
  useEffect(() => {
    let active = true; setInspection(null);
    if (selected) request('inspect', { environment_id: selected.id, trusted_devices: draft.trustedDevices ?? 'unknown' }).then(data => { if (active) setInspection(data); }).catch(error => { if (active) setError(asError(error)); });
    return () => { active = false; };
  }, [selected?.id, selected?.root, lastCheck, request, draft.trustedDevices]);

  async function perform(action: string, work: () => Promise<void>) { const source = hostAlias; setBusy(action); setError(''); try { await work(); } catch (error) { if (activeHost.current === source) setError(asError(error)); } finally { if (activeHost.current === source) setBusy(''); } }
  function updateDraft(value: Partial<Draft>) { setRecipeSaveResult(null); setDrafts(current => ({ ...current, [draftKey]: { ...(current[draftKey] ?? defaultDraft), ...value } })); }
  function updatePolicyDraft(choices: NonNullable<Draft['customSettings']>, values: NonNullable<Draft['customValues']> = {}) {
    setRecipeSaveResult(null);
    setDrafts(current => ({ ...current, [draftKey]: editPolicyDraft(current[draftKey] ?? defaultDraft, choices, values) }));
  }
  // A recipe seeds today's draft for the current host/environment only. It
  // stores no plan and replays no approval — preview regenerates it.
  function applyPolicyRecipe(intent: PolicyRecipeIntent) {
    if (!selected) return;
    updateDraft({ preset: intent.preset, keepRemoteControl: intent.keepRemoteControl, trustedDevices: intent.trustedDevices, releaseSettings: intent.releaseSettings ?? [], customSettings: intent.customSettings ?? {}, customValues: intent.customValues ?? {} });
  }
  function applyCleanupRecipe(intent: CleanupRecipeIntent) {
    if (!selected) return;
    setCleanupRecipeSeed({ key: JSON.stringify([hostAlias, selected.id]), intent });
    setResidueRecipe(null);
  }
  function applyResidueRecipe(intent: ResidueRecipeIntent) {
    if (!selected) return;
    setResidueRecipe({ key: JSON.stringify([hostAlias, selected.id]), intent });
    setCleanupRecipeSeed(null);
    // The residue clear lives in the environment's local-data view; open it so
    // the seeded combined-workflow choice is immediately visible.
    setPage('environments'); setShowInspector(true); setTab('state');
  }
  function openClawd(destination: ClawdDestination) { setPlayroom(destination); }
  function closeClawd() { setPlayroom(null); requestAnimationFrame(() => mainRef.current?.querySelector<HTMLButtonElement>('.clawd-pull-trigger')?.focus({ preventScroll: true })); }
  function selectEnvironment(id: string) { setSelectedId(id); setError(''); }
  async function copy(value: string) { try { await navigator.clipboard.writeText(value); setNotification('copied'); } catch { setError(new LocalizedError('app.clipboardUnavailable')); } }
  const openAdd = (mode: 'register'|'create') => { setAddMode(mode); setFormError(''); setName(''); setRoot(''); setDialog('add'); };
  async function addEnvironment(event: FormEvent) {
    event.preventDefault(); if (!name.trim() || (addMode === 'register' && !root.trim())) return;
    setBusy('add'); setFormError('');
    try { const environment = addMode === 'create' ? await request('create_environment', { name: name.trim() }) : await request('register', { name: name.trim(), root: root.trim() }); await refresh(); setSelectedId(environment.id); setPage('environments'); setDialog(null); setNotification(addMode === 'create' ? 'envCreated' : 'envRegistered'); }
    catch (error) { setFormError(asError(error)); } finally { setBusy(''); }
  }
  function preview(_kind: 'policy') {
    if (!selected) return; const target = selected;
    void perform('plan', async () => { const plan = await request('plan_policy', { environment_id: target.id, preset: draft.preset, keep_remote_control: draft.keepRemoteControl, trusted_devices: draft.trustedDevices ?? 'unknown', ...(draft.preset === 'custom' ? { custom_settings: draft.customSettings ?? {}, ...(Object.keys(draft.customValues ?? {}).length ? { custom_values: draft.customValues } : {}) } : { release_settings: draft.keepRemoteControl ? draft.releaseSettings ?? [] : [] }) }); if (activeHost.current === hostAlias) setFlow({ environment: target, plan }); });
  }
  async function executePlan() {
    if (!flow?.plan || executionLock.current || flow.uncertain || flow.receipt) return;
    if (flow.plan.archive_passphrase_required && (archivePassphrase.length < 12 || archivePassphrase !== archiveConfirmation)) return;
    const current = flow; executionLock.current = true; setBusy('execute'); setError('');
    try { const receipt = await requester(current.hostAlias ?? null)('execute', { plan_id: current.plan!.id, approval: current.plan!.hash, ...(current.plan!.archive_passphrase_required ? { archive_passphrase: archivePassphrase } : {}) }); if (activeHost.current !== hostAlias) return; setFlow(value => value === current ? { ...current, receipt, initialPassphrase: undefined } : value); await refresh(); }
    catch (error) { if (activeHost.current === hostAlias) { setFlow(value => value === current ? { ...current, uncertain: true } : value); setError(asError(error)); } }
    finally { executionLock.current = false; if (activeHost.current === hostAlias) setBusy(''); }
  }
  function reconcile() {
    if (!flow?.plan) return; const source = flow;
    void perform('reconcile', async () => { const found = source.hostAlias ? await remoteRequest<Receipt>({op:'reconnect',alias:source.hostAlias,plan_id:source.plan!.id}) : await requester(null)('job', { job_id: source.plan!.id }); if (activeHost.current !== hostAlias) return; setFlow(value => value === source ? { ...source, receipt: found, uncertain: false, initialPassphrase: undefined } : value); await refresh(); });
  }
  function refreshReceipt() { if (!flow?.receipt) return; const source = flow; void perform('query', async () => { const receipt = source.hostAlias ? await remoteRequest<Receipt>({ op:'reconnect', alias:source.hostAlias, plan_id:source.receipt!.plan_id }) : await requester(null)('job', { job_id: source.receipt!.id }); if (activeHost.current !== hostAlias) return; setFlow(value => value === source ? { ...source, receipt, initialPassphrase: undefined } : value); await refresh(); }); }
  function restore(receipt: Receipt) {
    const source = flow; const target = environments.find(env => env.id === receipt.environment_id) ?? null;
    if (!target && !receipt.network_change) { setError(new LocalizedError('app.restoreEnvMissing')); return; }
    void perform('restore', async () => { const plan = await request(receipt.network_change ? 'plan_network_restore' : 'plan_restore', { job_id: receipt.id, ...(receipt.network_change && receipt.after_probe ? {baseline_id:receipt.after_probe.id} : {}) }); if (activeHost.current === hostAlias) setFlow(value => value === source ? { environment: target, plan } : value); });
  }
  function resumeService(receipt: Receipt) {
    const source = flow; const target = environments.find(env => env.id === receipt.environment_id);
    if (!target) { setError(new LocalizedError('app.restoreServiceMissing')); return; }
    void perform('restore-service', async () => { const plan = await request('plan_service_resume', { job_id: receipt.id }); if (activeHost.current === hostAlias) setFlow(value => value === source ? { environment: target, plan } : value); });
  }
  function launch(environment: Environment, sourceHost=hostAlias,proxyUrl?:string) { setLaunchTarget({ environment: {...environment}, hostAlias: sourceHost,proxyUrl }); }
  function downloadSupport() { const url = URL.createObjectURL(new Blob([support], { type: 'application/json' })); const anchor = document.createElement('a'); anchor.href = url; anchor.download = 'lintel-support-redacted.json'; anchor.click(); setTimeout(() => URL.revokeObjectURL(url), 1000); setNotification('supportSaved'); }
  function queryOriginalJob() { void perform('query', async () => { const receipt = await request('job', { job_id: queryId.trim() }); if (activeHost.current !== hostAlias) return; const target = environments.find(env => env.id === receipt.environment_id) ?? null; if (!target && !receipt.network_change) throw new LocalizedError('app.jobEnvMissing'); setFlow({ environment: target, receipt }); }); }
  function supportPreview() { void perform('support', async () => { setSupport(JSON.stringify(await request('export_support', {}), null, 2)); setDialog('support'); }); }
  function Path({ value }: { value: string }) { return <div className="path"><code title={value}>{value}</code><button className="icon-button" aria-label={t('label.copyFullPath')} onClick={() => void copy(value)}><Icon name="copy" size={14}/></button></div>; }
  function Warnings({ items }: { items: string[] }) { return <>{items.length > 0 && <Notice tone="warning"><ul className="compact-list">{items.map((item, index) => <li key={index}>{p(item)}</li>)}</ul></Notice>}</>; }

  function continueGoal() {
    const destinations: Record<string, () => void> = {
      reduce_egress: () => setPage('policy'), preserve_work: () => setPage('work'),
      repair_cleanup_retire: () => setPage('rebuild'), recover_results: () => setPage('history'),
      browser_profile: () => setDialog('browser'), ssh_remote: () => setDialog('remote'),
    };
    if (activeTask) destinations[activeTask]?.();
  }
  return <WorkspaceActionsContext.Provider value={actionsMount}><div className={`app-shell ${sidebarCollapsed ? 'sidebar-collapsed' : ''}`}>
    <aside className="sidebar">
      <div className="brand"><strong>lintel<span className="brand-cursor">_</span></strong><button className="icon-button" aria-label={t(sidebarCollapsed ? 'app.expandSidebar' : 'app.collapseSidebar')} onClick={() => setSidebarCollapsed(value => !value)}><Icon name="sidebar" size={18}/></button></div>
      <button className="new-space" onClick={() => openAdd('create')}><span><Icon name="plus" size={15}/></span><span>{t('app.newEnv')}</span></button>
      <nav aria-label={t('app.mainNav')}>{nav.map(item => <button key={item.id} className={`nav-item ${page === item.id ? 'active' : ''}`} title={t(item.key)} aria-current={page === item.id ? 'page' : undefined} onClick={() => {setPage(item.id);const task=({policy:'reduce_egress',work:'preserve_work',rebuild:'repair_cleanup_retire',history:'recover_results',sessions:'preserve_work'} as Partial<Record<Page,string>>)[item.id];if(task)setActiveTask(task);}}><Icon name={item.icon}/><span>{t(item.key)}</span></button>)}</nav>
      <div className="sidebar-environments"><div className="nav-caption">{t('app.yourEnvironments')}<button className="icon-button" aria-label={t('app.addExistingEnv')} onClick={() => openAdd('register')}><Icon name="plus" size={13}/></button></div>{environments.map(env => <button key={env.id} className={`sidebar-environment ${env.id === selectedId ? 'chosen' : ''}`} onClick={() => { selectEnvironment(env.id); setPage('environments'); }} title={env.name}>{env.name.replace('Claude Code · ', '')}</button>)}</div>
      <div className="sidebar-bottom"><button className="nav-item" onClick={() => setDialog('help')} title={t('app.helpNav')}><Icon name="help"/><span>{t('app.helpNav')}</span></button><button className="profile-settings" onClick={() => setDialog('settings')} title={t('app.settingsModulesTitle')}><span className="profile-symbol">L</span><span><strong>{t('app.workspaceLabel')}</strong><small>{t('app.devPreviewLabel')}</small></span><Icon name="chevron" size={14}/></button></div>
    </aside>
    <div className="workspace">
      <TargetBar environments={environments} selected={selected} selectedId={selectedId} hostAlias={hostAlias} busy={!!busy} loading={loading} onSelect={selectEnvironment} onRemote={() => setDialog('remote')} onRefresh={() => void perform('refresh', refresh)} onCopy={copy}><LanguageSwitch compact/><ThemeSwitch theme={theme} onChange={setTheme}/></TargetBar>
      {hostAlias && <div className="remote-banner"><Icon name="terminal" size={13}/>{t('app.remoteWorkspace')} · {hostAlias}<span>{t('app.remoteWorkspaceNote')}</span></div>}{transport === 'synthetic' && <div className="fixture-banner"><Icon name="terminal" size={15}/><span>{t('app.fixtureSpace')}</span></div>}
      {packageActivity.length>0 && <aside className="package-activity" role="status" aria-live="polite" aria-label={t('app.packageActivity')}>{packageActivity.map(item=><div key={item.id}><strong>{item.hostAlias?`SSH · ${item.hostAlias}`:t('app.localLabel')} · {t(item.running?'app.activityRunning':'app.activityWaiting',{label:t(item.labelKey)})}</strong><span>{t('app.activityNote')}{item.waiting>0?t('app.activityQueued',{count:item.waiting}):''}</span></div>)}</aside>}
      <main ref={mainRef} className={`main-content ${page === 'environments' ? 'home-content' : page === 'sessions' ? 'reading-content' : ''}`} id="main-content">
        {activeGoal && page!=='environments' && <aside className="current-goal" aria-label={t('app.currentGoal')}><div><span className="eyebrow">{t('app.nowToFinish')}</span><strong>{taskHelp.label(activeGoal)}</strong><details><summary>{t('app.taskScopeSummary')}</summary><p>{taskHelp.summary(activeGoal)}</p></details></div><button className="text-button" onClick={()=>setPage('environments')}>{t('app.chooseOtherTask')}</button></aside>}
        {error && !flow && <div className="error-wrap"><RequestFailure error={error}/><button className="text-button" onClick={() => setError('')}>{t('app.dismiss')}</button></div>}
        {loading ? <div className="empty-state"><span className="loading-mark"/><h2>{t('app.checkingEnvs')}</h2><p>{t('app.checkingHint')}</p></div> : transport === 'unavailable' ? <div className="empty-state"><Brand/><h1>{t('app.startDesktop')}</h1><p>{t('app.noNativePermission')}<br/>{t('app.devPreviewRunPrefix')} <code>npm run dev:synthetic</code>{t('sentence.end')}</p><button onClick={() => setDialog('help')}>{t('app.viewGuide')}</button></div> : <>
          {page === 'environments' && <>
            <section className="home-stage"><div className="home-greeting"><Clawd interactive scrollRef={mainRef} onChoose={openClawd}/><div><h1 className="home-title">{greetingText}</h1><p>{t('app.homeSubtitle')}</p></div></div>
            <section className="home-composer" aria-label={t('app.homeComposerAria')}><button className="home-task-prompt" onClick={() => setDialog('tasks')}>{t('app.startTaskHint')}<span>{t('app.startTaskSubhint')}</span></button><div className="home-composer-footer"><button className="text-button" onClick={() => openAdd('register')}><Icon name="plus" size={15}/>{t('app.addExistingEnv')}</button><button className="primary" onClick={() => setDialog('tasks')}>{t('app.startTask')}<Icon name="arrow" size={16}/></button></div></section>
            {activeGoal && <button className="home-continue" onClick={continueGoal}><span>{t('app.continueLastGoal')}</span><strong>{taskHelp.label(activeGoal)}</strong><Icon name="arrow" size={14}/></button>}
            {attentionJobs.length > 0 && <button className="home-pending" onClick={() => {setReceiptFilter('attention');setPage('history');}}><Icon name="history" size={14}/>{t('app.pendingReview',{count:attentionJobs.length})}<Icon name="arrow" size={14}/></button>}
            {environments.length === 0 ? <><p className="home-empty-note">{t('app.noEnvNote')}</p><button className="text-button" aria-expanded={showInspector} onClick={()=>setShowInspector(value=>!value)}>{t('app.viewHostNetwork')}</button>{showInspector && <section className="detail-panel"><NetworkPanel key={hostAlias ?? "local"} environmentId="" hostAlias={hostAlias} receipts={jobs} disabled={!!busy} onPlan={plan=>setFlow({environment:null,plan})} onReceipt={receipt=>setFlow({environment:null,receipt})} executable={false} onLaunch={()=>{}}/></section>}</> : <>
              {selected?.status === 'retired' && <Notice>{t('app.envRetired')}<button className="text-button" disabled={!!busy} onClick={() => void perform('reactivate', async () => { await request('reactivate_environment', { environment_id: selected.id }); await refresh(); setNotification('reactivated'); })}>{t('app.reactivateEnv')}</button></Notice>}              {selected && driftTipDismissed !== JSON.stringify([hostAlias, selected.id, selected.root]) && <DriftTip key={JSON.stringify([hostAlias, selected.id, selected.root, lastCheck?.getTime()])} send={request} environmentId={selected.id} environmentName={selected.name} canCompare={selected.status !== 'retired'} onCompare={() => { setShowInspector(true); setTab('compare'); }} onDismiss={() => setDriftTipDismissed(JSON.stringify([hostAlias, selected.id, selected.root]))}/>}{selected && <div className="home-tools"><div className="home-work-links"><button onClick={() => setDialog('browser')}>{t('app.browserTools')}<Icon name="chevron" size={13}/></button><button onClick={() => { setArchiveJobId(undefined); setPage('sessions'); }}>{t('app.sessionsTools')}<Icon name="chevron" size={13}/></button><button onClick={() => openAgent()}>{t('app.handToAgent')}<Icon name="chevron" size={13}/></button></div><button className="inspector-toggle" aria-expanded={showInspector} onClick={() => setShowInspector(value => !value)}>{t('app.envDetails')}<Icon name="chevron" size={13}/></button></div>}
              {selected && showInspector && <section className="detail-panel"><div className="inspector-status"><span>{lastCheck ? t('app.checkedAt',{time:fmtTime(lastCheck)}) : t('app.notCheckedYet')}</span><button className="text-button" disabled={!!busy} onClick={() => void perform('refresh', refresh)}><Icon name="refresh" size={13}/>{t('app.recheck')}</button></div><div className="tabs" role="tablist" aria-label={t('app.envDetails')} onKeyDown={event => { const ids: Tab[] = ['overview','outbound','state','compare','launch','components']; let next = ids.indexOf(tab); if (event.key === 'ArrowRight') next = (next + 1) % ids.length; else if (event.key === 'ArrowLeft') next = (next + ids.length - 1) % ids.length; else if (event.key === 'Home') next = 0; else if (event.key === 'End') next = ids.length - 1; else return; event.preventDefault(); setTab(ids[next]); (event.currentTarget.querySelectorAll('button')[next] as HTMLButtonElement).focus(); }}>{([['overview','tab.overview'],['outbound','tab.outbound'],['state','tab.state'],['compare','tab.compare'],['launch','tab.launch'],['components','tab.components']] as const).map(([id,key]) => <button role="tab" tabIndex={tab === id ? 0 : -1} aria-selected={tab === id} aria-controls="environment-tab-content" key={id} onClick={() => setTab(id)}>{t(key)}</button>)}</div><div className="detail-body" id="environment-tab-content" role="tabpanel">
                {tab === 'components' ? <ComponentPanel key={JSON.stringify([hostAlias,selected.id])} send={request} environment={selected} onEntry={entry=>{if(entry==='browser')setDialog('browser');else if(entry==='launch')launch(selected);else if(entry==='policy')setPage('policy');else if(entry==='cleanup'||entry==='service')setPage('rebuild');}} onJob={id=>void perform('query',async()=>{const receipt=await request('job',{job_id:id});if(activeHost.current!==hostAlias)return;setFlow({hostAlias,environment:selected,receipt});})}/> : !inspection ? <p className="muted">{t('app.readingEnv')}</p> : <>
                  {tab === 'overview' && <><div className="overview-grid"><button className="overview-item" onClick={() => setPage('policy')}><span className="feature-icon blue"><Icon name="policy" size={20}/></span><span><strong>{t('overview.outbound')}</strong><small>{t('overview.outboundCount',{configured:inspection.settings.filter(setting => setting.status === 'configured').length,total:inspection.settings.length})}</small></span><Icon name="chevron" size={14}/></button><button className="overview-item" onClick={() => setTab('state')}><span className="feature-icon violet"><Icon name="environments" size={20}/></span><span><strong>{t('overview.content')}</strong><small>{t('overview.contentCount',{count:inspection.assets.filter(asset => ['instructions','memory','sessions'].includes(asset.category)).reduce((sum,asset) => sum + asset.count,0)})}</small></span><Icon name="chevron" size={14}/></button><button className="overview-item" onClick={() => setPage('history')}><span className="feature-icon amber"><Icon name="history" size={20}/></span><span><strong>{t('overview.history')}</strong><small>{targetJobs.length ? t('overview.historyCount',{count:targetJobs.length}) : t('overview.historyEmpty')}</small></span><Icon name="chevron" size={14}/></button></div><details className="environment-diagnostics"><summary><span><Icon name="help" size={15}/>{t('overview.sources')}</span><Icon name="chevron" size={14}/></summary><div className="fact-row"><span>{t('overview.configDir')}</span><Path value={selected.root}/></div><div className="fact-row"><span>{t('overview.verifyScope')}</span><span>{t('overview.verifyScopeValue')}</span></div><Warnings items={inspection.warnings}/></details></>}
                  {tab === 'outbound' && <><SettingsRows settings={inspection.settings}/><NetworkPanel key={`${hostAlias ?? 'local'}:${selected.id}`} environmentId={selected.id} hostAlias={hostAlias} settings={inspection.settings} onPrivacySettings={()=>setPage('policy')} receipts={jobs} disabled={!!busy} onPlan={plan=>setFlow({environment:null,plan})} onReceipt={receipt=>setFlow({environment:null,receipt})} executable={!!selected.executable} onLaunch={proxy=>launch(selected,null,proxy)}/> <div className="separated"><h3>{t('browser.permissions')}</h3><p className="muted">{t('browser.permissionsNote')}</p><button className="text-button" onClick={() => setDialog('browser')}>{t('browser.manageProfiles')}<Icon name="arrow" size={14}/></button></div></>}
                  {tab === 'compare' && <><SnapshotPanel key={JSON.stringify([hostAlias, selected.id, selected.root])} send={request} environment={selected} environments={environments} disabled={!!busy} onPlan={(plan, environment) => { if (activeHost.current === hostAlias) setFlow({ environment, hostAlias, plan }); }}/><ConfigEditor key={`cfg:${JSON.stringify([hostAlias, selected.id, selected.root])}`} send={request} environment={selected} disabled={!!busy} onPlan={plan => { if (activeHost.current === hostAlias) setFlow({ environment: selected, hostAlias, plan }); }}/></>}
                  {tab === 'state' && <><div className="asset-grid">{inspection.assets.map(asset => <div className="asset-row" key={asset.category}><span>{t(categoryKey(asset.category))}</span><strong>{t('app.items',{count:asset.count})}</strong><span className="muted">{formatBytes(asset.bytes)}</span></div>)}</div>{inspection.assets.some(asset => asset.complete === false) && <Notice tone="warning">{t('state.incomplete')}</Notice>}<Notice>{t('state.notIsolated')}</Notice><button className="text-button section-action" onClick={() => setPage('work')}>{t('state.chooseWork')}<Icon name="arrow" size={15}/></button><ResiduePanel key={JSON.stringify([hostAlias, selected.id, selected.root])} send={request} environment={selected} hostAlias={hostAlias} disabled={!!busy} seed={residueRecipe && residueRecipe.key === JSON.stringify([hostAlias, selected.id]) ? { key: residueRecipe.key, reduceContent: residueRecipe.intent.reduceContent, resetIdentity: residueRecipe.intent.resetIdentity } : null} refreshKey={lastCheck?.getTime()} onPlan={(plan, initialPassphrase) => { if (activeHost.current === hostAlias) setFlow({ environment: selected, hostAlias, plan, initialPassphrase }); }} onWork={() => setPage('work')}/></>}
                  {tab === 'launch' && <><div className="fact-row"><span>{t('launch.exe')}</span>{selected.executable ? <Path value={selected.executable}/> : <span>{t('launch.exeMissing')}</span>}</div><div className="fact-row"><span>{t('launch.configRoot')}</span><Path value={selected.root}/></div><div className="fact-row"><span>{t('launch.existingSessions')}</span><span>{t('launch.noAutoRestart')}</span></div><Notice>{t('launch.newLaunchNote')}</Notice><div className="button-row"><button className="primary" disabled={!!busy || !selected.executable || selected.status === 'retired'} onClick={() => launch(selected)}>{t('launch.openClaude')}<Icon name="arrow" size={15}/></button>{hostAlias && <span className="muted">{t('launch.connectTerminal')}</span>}</div></>}
                </>}
              </div></section>}
              {selected && showInspector && <DriftCheck key={JSON.stringify([hostAlias, selected.id, selected.root, lastCheck?.getTime(), draft.trustedDevices])} send={request} environmentId={selected.id} disabled={!!busy} onPolicy={() => setPage('policy')} onCompare={()=>setTab('compare')}/>}
            </>}
            </section>
          </>}
          {page === 'policy' && <>
            <div className="page-heading"><div><h1>{t('policy.title')}</h1><p>{t('policy.subtitle')}</p></div><Clawd small mood="plan"/></div>
            {!selected ? <SelectTarget onAdd={() => openAdd('register')}/> : <>
              <fieldset className="preset-grid"><legend className="sr-only">{t('policy.presetLegend')}</legend><label className={`preset ${draft.preset === 'preserve' ? 'chosen' : ''}`}><input type="radio" name="preset" value="preserve" checked={draft.preset === 'preserve'} onChange={() => updateDraft({ preset: 'preserve', releaseSettings: [] })}/><span className="preset-copy"><strong>{t('policy.preserve')}</strong><span>{t('app.preserveOffSurvey')}<br/>{t('app.keepMetricsFeedback')}</span></span></label><label className={`preset ${draft.preset === 'reduce' ? 'chosen' : ''}`}><input type="radio" name="preset" value="reduce" checked={draft.preset === 'reduce'} onChange={() => updateDraft({ preset: 'reduce', releaseSettings: [] })}/><span className="preset-copy"><strong>{t('policy.reduce')}</strong><span>{t('app.reduceFeedbackSurvey')}<br/>{t('app.feedbackAndSurvey')}</span></span></label><label className={`preset ${draft.preset === 'custom' ? 'chosen' : ''} ${!inspection?.policy?.supported_presets?.includes('custom') ? 'unavailable' : ''}`}><input type="radio" name="preset" value="custom" checked={draft.preset === 'custom'} disabled={!inspection?.policy?.supported_presets?.includes('custom')} onChange={() => updateDraft({ preset: 'custom', releaseSettings: [] })}/><span className="preset-copy"><strong>{t('policy.custom')}</strong><span>{t('app.customKeepOthers')}<br/>{t('app.customOptionNote')}</span></span></label></fieldset>
              <details className="policy-compatibility"><summary><span>{t('policy.compatibility')}</span><small>{t(draft.keepRemoteControl ? 'app.keepRemoteControl' : 'policy.compatibilityOptional')}</small><Icon name="chevron" size={13}/></summary><div className="policy-compatibility-body"><label className="checkbox-row"><span><strong>{t('app.keepRemoteControl')}</strong><span>{t('app.keepRemoteControlNote')}</span></span><input type="checkbox" aria-label={t('app.keepRemoteControl')} checked={draft.keepRemoteControl} onChange={event => updateDraft({ keepRemoteControl: event.target.checked, releaseSettings: [] })}/></label><label className="policy-condition"><span>{t('app.trustedDevicesQ')}</span><select aria-label={t('app.trustedDevicesAria')} value={draft.trustedDevices ?? 'unknown'} onChange={event => updateDraft({ trustedDevices: event.target.value as NonNullable<Draft['trustedDevices']>, releaseSettings: [] })}><option value="unknown">{t('app.trustedUnknown')}</option><option value="required">{t('app.trustedRequired')}</option><option value="not_required">{t('app.trustedNotRequired')}</option></select><small>{t('app.trustedDevicesNote')}</small></label>{inspection?.policy && <PolicyEvidence policy={inspection.policy}/>}{draft.keepRemoteControl && draft.preset !== 'custom' && inspection?.policy?.remote_control.blockers.map(blocker => <label className="checkbox-row policy-release" key={blocker.key}><span><strong>{t('app.releaseKeyLabel',{key:blocker.key})}</strong><span>{t('app.releaseKeyNote',{value:JSON.stringify(blocker.value),reason:p(blocker.reason)})}</span></span><input type="checkbox" aria-label={t('app.releaseKeyAria',{key:blocker.key})} checked={(draft.releaseSettings ?? []).includes(blocker.key)} onChange={event => updateDraft({ releaseSettings: event.target.checked ? [...draft.releaseSettings ?? [], blocker.key] : (draft.releaseSettings ?? []).filter(key => key !== blocker.key) })}/></label>)}{inspection && !inspection.policy && <Notice tone="warning">{t('app.noPolicyNotice')}</Notice>}<div className="preserve-note"><Icon name="check" size={16}/><span>{t('app.preserveNote')}</span></div></div></details>
              {draft.preset === 'custom' && inspection?.policy?.supported_presets?.includes('custom') && <CustomPolicy key={JSON.stringify([hostAlias, selected.id, selected.root])} policy={inspection.policy} settings={inspection.settings} choices={draft.customSettings ?? {}} values={draft.customValues ?? {}} disabled={!!busy} onChange={updatePolicyDraft}/>}
              <details className="policy-alternatives"><summary><span>{t('policy.savedRecipes')}</span><Icon name="chevron" size={13}/></summary><Recipes key={JSON.stringify([hostAlias,selected.id])} hostAlias={hostAlias} environmentId={selected.id} disabled={!!busy} scope="policy" onPolicy={() => {}} onRebuild={() => setPage('rebuild')} onApplyPolicy={applyPolicyRecipe} onApplyResidue={applyResidueRecipe} onApplyCleanup={applyCleanupRecipe}/><div className="button-row"><button className="text-button" onClick={()=>{const saved=saveRecipe('reduce_content',hostAlias,selected.id,{kind:'policy',preset:draft.preset,keepRemoteControl:draft.keepRemoteControl,trustedDevices:draft.trustedDevices,releaseSettings:draft.releaseSettings,customSettings:draft.customSettings,customValues:draft.customValues});setRecipeSaveResult({ target: draftKey, omitted: saved.omittedKeys, saved: saved.saved });if(saved.saved)setNotification('policyRecipeSaved');}}>{t('recipes.saveCurrent')}</button><span className="small-print">{t('recipes.saveCurrentNote')}</span></div>{recipeSaveResult?.target === draftKey && <p className="recipe-save-feedback" role="status">{!recipeSaveResult.saved ? t('recipes.saveFailed') : recipeSaveResult.omitted.length ? t('recipes.savedPartial', { fields: recipeSaveResult.omitted.map(key => p(inspection?.policy?.rules.find(rule => rule.key === key)?.label ?? key)).join(t('list.separator')) }) : t('recipes.savedComplete')}</p>}</details>
              <details className="policy-current-config"><summary><span>{t('app.currentConfig')}</span><small>{t('app.readbackNotProof')}</small><Icon name="chevron" size={13}/></summary>{inspection ? <SettingsRows settings={inspection.settings}/> : <p className="padded muted">{t('app.readingSettings')}</p>}</details>
              {inspection && !inspection.policy?.supported_presets?.includes('custom') && <Notice tone="warning">{t('app.customUnsupportedNotice')}</Notice>}
              <Notice>{t('app.policyFooterNote')}</Notice>
              <WorkspaceActions><div><strong>{draft.preset === 'custom' ? t(policyEdits.invalid.length ? 'policy.checkEdits' : policyEdits.pending.length ? 'policy.pendingEdits' : 'policy.noEdits', { count: policyEdits.invalid.length || policyEdits.pending.length }) : t('app.draftKept')}</strong><span>{t('app.draftScope',{name:selected.name})}</span></div><button className="primary" disabled={!!busy || !inspection || (draft.preset === 'custom' && (!inspection.policy?.supported_presets?.includes('custom') || !policyEdits.pending.length || !!policyEdits.invalid.length))} onClick={() => preview('policy')}>{busy === 'plan' ? t('app.generatingPlan') : t('app.previewChanges')}<Icon name="arrow" size={16}/></button></WorkspaceActions>
            </>}
          </>}
          {page === 'work' && (selected ? <WorkPanel key={`${hostAlias ?? 'local'}:${selected.id}`} send={request} hostAlias={hostAlias} environment={selected} inspection={inspection} onPlan={plan=>{if(activeHost.current===hostAlias)setFlow({environment:selected,hostAlias,plan});}} onArchives={()=>{setArchiveJobId(undefined);setPage('sessions');}}/> : <SelectTarget onAdd={() => openAdd('register')}/>)}
          {page === 'rebuild' && (selected ? <><Recipes key={JSON.stringify([hostAlias,selected.id])} hostAlias={hostAlias} environmentId={selected.id} disabled={!!busy} scope="rebuild" onPolicy={() => { setPage('policy'); }} onRebuild={() => {}} onApplyPolicy={applyPolicyRecipe} onApplyResidue={applyResidueRecipe} onApplyCleanup={applyCleanupRecipe}/><CleanupPanel key={`${hostAlias ?? 'local'}-${selected.id}`} send={request} hostAlias={hostAlias} environment={selected} refreshKey={lastCheck?.getTime()} seed={cleanupRecipeSeed && cleanupRecipeSeed.key === JSON.stringify([hostAlias, selected.id]) ? { key: cleanupRecipeSeed.key, intent: { recipe: cleanupRecipeSeed.intent.recipe, categories: cleanupRecipeSeed.intent.categories, activateInstructions: cleanupRecipeSeed.intent.activateInstructions } } : null} onPlan={plan => { if (activeHost.current === hostAlias) setFlow({environment:selected,plan}); }} onBrowser={() => setDialog('browser')} onWork={() => setPage('work')}/></> : <SelectTarget onAdd={() => openAdd('register')}/>)}
          {page === 'sessions' && <><div className="page-heading"><div><span className="eyebrow">work / read, select, continue</span><h1>{t('sessions.title')}</h1><p>{t('sessions.subtitle')}</p></div><Clawd small mood="pack"/></div><ArchivePanel key={hostAlias ?? 'local'} send={request} jobs={jobs} environments={environments} initialJob={archiveJobId} selectedId={selectedId} hostAlias={hostAlias} onImport={(environment,plan,passphrase)=>setFlow({environment,plan,initialPassphrase:passphrase})}/></>}
          {page === 'agent' && <AgentPanel context={handoff ?? currentHandoff}/>}
          {page === 'history' && <>
            <div className="page-heading"><div><span className="eyebrow">03 / nothing lost in the shuffle</span><h1>{t('history.title')}</h1><p>{t('history.subtitle')}</p></div><button disabled={!!busy} onClick={() => void perform('refresh', refresh)}><Icon name="refresh" size={16}/>{t('app.refresh')}</button></div>
            <div className="history-toolbar"><div className="segmented"><button aria-pressed={receiptFilter === 'all'} onClick={() => setReceiptFilter('all')}>{t('app.filterAll')}</button><button aria-pressed={receiptFilter === 'target'} onClick={() => setReceiptFilter('target')}>{t('app.filterTarget')}</button><button aria-pressed={receiptFilter === 'attention'} onClick={() => setReceiptFilter('attention')}>{t('app.filterAttention')}{attentionJobs.length > 0 && <span className="filter-count">{attentionJobs.length}</span>}</button></div><label className="history-search"><span className="sr-only">{t('app.searchRecordsSr')}</span><input type="search" value={historySearch} onChange={event => setHistorySearch(event.target.value)} placeholder={t('app.searchPlaceholder')}/></label></div><details className="query-task"><summary>{t('app.querySummary')}</summary><form onSubmit={event => { event.preventDefault(); queryOriginalJob(); }}><label className="field">{t('app.originalTaskId')}<input value={queryId} onChange={event => setQueryId(event.target.value)} placeholder={t('app.queryPlaceholder')} spellCheck={false}/></label><button disabled={!!busy || !queryId.trim()}>{t('app.queryOriginalTask')}</button></form><p className="small-print">{t('app.queryNote')}</p></details>
            {visibleJobs.length === 0 ? <div className="empty-state bordered"><Clawd mood="rest"/><h2>{t(historySearch ? 'app.emptyNoMatch' : receiptFilter === 'attention' ? 'app.emptyAttention' : 'app.emptyNone')}</h2><p>{t(receiptFilter === 'attention' ? 'app.emptyAttentionNote' : 'app.emptyNoneNote')}</p>{receiptFilter === 'attention' ? <button onClick={() => setReceiptFilter('all')}>{t('app.viewAll')}</button> : <button onClick={() => setPage('policy')}>{t('app.choosePolicyPlan')}</button>}</div> : <div className="job-list">{visibleJobs.map(job => { const env = environments.find(item => item.id === job.environment_id); return <div className="job-row" key={job.id}><span className="job-icon"><Icon name="history" size={19}/></span><div className="job-copy"><strong>{p(job.title)}</strong><span>{job.network_change ? t('app.hostSharedNetwork') : env?.name ?? job.environment_id} · {fmtDate(job.created_at)}</span><code>{job.id}</code></div><Status value={job.status}/><button disabled={!!busy} onClick={() => { if (env || job.network_change) void perform('query', async () => { const receipt = await request('job', { job_id: job.id }); if (activeHost.current !== hostAlias) return; setFlow({ environment: env ?? null, receipt }); }); else setError(new LocalizedError('app.envMissingForRecord')); }}>{t('app.viewResult')}</button></div>; })}</div>}
            <LaunchRecords send={request} hostAlias={hostAlias} onHandoff={record=>{setHandoff({task:"recover_results",hostAlias,launch:record,environment:environments.find(e=>e.root===(record.root ?? record.config_root)),observedAt:new Date().toISOString()});setPage("agent");}}/><div className="inline-route"><ResourceLink resource="recovery-guide">{t('app.recoveryGuide')}</ResourceLink></div><div className="archive-shortcut"><Clawd small mood="pack"/><div><h3>{t('app.archiveTitle')}</h3><p>{t('app.archiveNote')}</p></div><button onClick={() => { setArchiveJobId(undefined); setPage('sessions'); }}>{t('app.viewArchive')}<Icon name="arrow" size={14}/></button></div><div className="support-footer"><span>{t('app.supportNote')}</span><button className="text-button" disabled={!!busy} onClick={supportPreview}>{t('app.supportPreview')}<Icon name="arrow" size={15}/></button></div>
          </>}
        </>}
      </main>
      <footer className="workspace-actions" aria-label={t('app.operationTarget')} ref={setActionsMount}/>
    </div>
    {notification && <div className="toast" role="status"><Icon name="check" size={17}/>{t(`app.notice.${notification}`)}<button className="icon-button" aria-label={t('app.dismiss')} onClick={() => setNotification('')}><Icon name="close" size={14}/></button></div>}
    {dialog === 'tasks' && <Modal title={t('app.startTask')} wide onClose={() => setDialog(null)}><div className="modal-body task-home"><TaskHelp onChooseTask={id => {setActiveTask(id);setDialog(null);}} onProtect={() => setPage('policy')} onWork={() => setPage('work')} onCleanup={() => setPage('rebuild')} onBrowser={() => setDialog('browser')} onRemote={() => setDialog('remote')} onRecovery={() => {setReceiptFilter('attention');setPage('history');}}/></div><div className="modal-footer"><button onClick={() => setDialog(null)}>{t('app.waitAWhile')}</button></div></Modal>}
    {dialog === 'add' && <Modal title={t(addMode === 'create' ? 'app.createEnvTitle' : 'app.addEnvTitle')} onClose={() => { if (!busy) setDialog(null); }}><form onSubmit={addEnvironment}><div className="modal-body"><div className="segmented add-tabs"><button type="button" aria-pressed={addMode === 'create'} onClick={() => setAddMode('create')}>{t('app.createEnvTab')}</button><button type="button" aria-pressed={addMode === 'register'} onClick={() => setAddMode('register')}>{t('app.registerDirTab')}</button></div><p>{t(addMode === 'create' ? 'app.createEnvNote' : 'app.registerEnvNote')}</p><label className="field">{t('app.envNameLabel')}<input required maxLength={100} value={name} onChange={event => setName(event.target.value)} placeholder={t('app.envNamePlaceholder')}/></label>{addMode === 'register' && <label className="field">{t('app.configDirLabel')}<input required value={root} onChange={event => setRoot(event.target.value)} placeholder="/path/to/claude-config" spellCheck={false}/><small>{t('app.fontFamilyNote')}</small></label>}{formError && <div role="alert"><Notice tone="error">{errorText(formError)}</Notice></div>}</div><div className="modal-footer"><button type="button" disabled={!!busy} onClick={() => setDialog(null)}>{t('app.cancel')}</button><button className="primary" disabled={!!busy || !name.trim() || (addMode === 'register' && !root.trim())} type="submit">{busy === 'add' ? t('app.processing') : t(addMode === 'create' ? 'app.createEnv' : 'app.addEnv')}</button></div></form></Modal>}
    {playroom && <Modal title={t('app.playroomTitle')} wide onClose={closeClawd}><div className="modal-body playroom-body"><ClawdPlayroom reducedMotion={!motion.animations} initialTab={playroom}/></div></Modal>}
    {dialog === 'remote' && <Modal title={t('app.hostSsh')} wide onClose={() => setDialog(null)}><div className="modal-body"><RemotePanel current={hostAlias} onOpenLaunch={(alias,record)=>{setHandoff({task:"recover_results",hostAlias:alias,launch:record,observedAt:new Date().toISOString()});setDialog(null);setPage("agent");}} onRemoved={alias => { if (hostAlias === alias) { activeHost.current = null; setHostAlias(null); setSelectedId(''); setFlow(null); setError(''); setPage('environments'); } }} onOpenReceipt={(alias, receipt, environment) => { activeHost.current = alias; setHostAlias(alias); setSelectedId(environment.id); setDialog(null); setFlow({ hostAlias: alias, environment, receipt }); setError(''); setBusy(''); setPage('history'); }} onSelect={alias => { activeHost.current = alias; setHostAlias(alias); setSelectedId(''); setDialog(null); setFlow(null); setError(''); setBusy(''); setPage('environments'); }}/></div><div className="modal-footer"><button onClick={() => setDialog(null)}>{t('app.done')}</button></div></Modal>}
    {dialog === 'browser' && <Modal title={t('app.browserSpace')} wide onClose={() => setDialog(null)}><div className="modal-body"><BrowserPanel/></div><div className="modal-footer"><button onClick={() => setDialog(null)}>{t('app.done')}</button></div></Modal>}
    {dialog === 'settings' && <Modal title={t('app.settingsModulesTitle')} wide onClose={() => setDialog(null)}><div className="modal-body settings-body"><h3>{t('app.appearance')}</h3><LanguageSwitch/><ThemeSwitch theme={theme} onChange={setTheme}/><p className="small-print">{t('app.appearanceNote')}</p><div className="motion-settings"><h4>{t('app.motionTitle')}</h4>{([['animations','app.motionAnimations'],['logoBlink','app.motionLogo'],['terminalBlink','app.motionTerminal']] as const).map(([key,label])=><label className="motion-setting" key={key}><span>{t(label)}</span><input type="checkbox" aria-label={t(label)} checked={motion[key]} onChange={event=>setMotion(current=>({...current,[key]:event.target.checked}))}/></label>)}<p className="small-print">{t('app.motionNote')}</p></div><div className="settings-links"><button className="section-action" onClick={() => setDialog('remote')}>{t('app.hostSsh')}<Icon name="arrow" size={14}/></button><button className="section-action" onClick={() => setDialog('updates')}>{t('app.versionUpdates')}<Icon name="arrow" size={14}/></button></div><h3 className="separated">{t('app.capabilitiesTitle')}</h3><p className="muted">{t('app.capabilitiesNote')}</p><div className="capability-list">{capabilities.map(capability => <div className="capability" key={capability.name}><div><strong>{p(capability.name)}</strong><p>{p(capability.reason)}</p></div><Status value={capability.status}/></div>)}</div><button className="section-action" onClick={() => setDialog('browser')}>{t('app.browserPairingCleanup')}<Icon name="arrow" size={15}/></button><Notice>{t('app.settingsNote')}</Notice><div className="fact-row"><span>{t('app.dataPrivacy')}</span><span>{t('app.dataPrivacyNote')}</span></div></div><div className="modal-footer"><button onClick={() => setDialog(null)}>{t('app.done')}</button></div></Modal>}
    <AppUpdates visible={dialog === 'updates'} disabled={!!busy || packageActivity.length > 0 || !!flow || !!launchTarget} onClose={() => setDialog('settings')}/>
    {dialog === 'help' && <Modal title={t('app.helpTitle')} wide onClose={() => setDialog(null)}>
      <div className="modal-body help-content">
        <div className="help-brand"><Brand/><div><h3>{t('app.helpBrandTitle')}</h3><p>{t('app.helpBrandNote')}</p></div></div>
        <section className="help-section"><h3>{t('app.helpScopeTitle')}</h3><p>{t('app.helpScopeIntro')}</p><div className="help-scope"><div><h4>{t('app.helpScopeCanTitle')}</h4><ul><li>{t('app.helpScopeCan1')}</li><li>{t('app.helpScopeCan2')}</li><li>{t('app.helpScopeCan3')}</li><li>{t('app.helpScopeCan4')}</li></ul></div><div><h4>{t('app.helpScopeCannotTitle')}</h4><ul><li>{t('app.helpScopeCannot1')}</li><li>{t('app.helpScopeCannot2')}</li><li>{t('app.helpScopeCannot3')}</li><li>{t('app.helpScopeCannot4')}</li></ul></div></div><Notice>{t('app.helpScopeNotice')}</Notice></section>
        <TaskHelp showGuides={false} onChooseTask={setActiveTask} onProtect={() => {setDialog(null);setPage('policy');}} onWork={() => {setDialog(null);setPage('work');}} onCleanup={() => {setDialog(null);setPage('rebuild');}} onBrowser={() => setDialog('browser')} onRemote={() => setDialog('remote')} onRecovery={() => {setDialog(null);setPage('history');}}/>
        <section className="help-section"><h3>{t('app.helpPathsTitle')}</h3><p>{t('app.helpPathsIntro')}</p><div className="help-paths"><div className="help-path"><strong>{t('app.helpPathAppTitle')}</strong><p>{t('app.helpPathAppNote')}</p><ResourceLink resource="operator-guide">{t('resources.operatorGuide')}</ResourceLink></div><div className="help-path"><strong>{t('app.helpPathCliTitle')}</strong><p>{t('app.helpPathCliNote')}</p><ResourceLink resource="agent-guide">{t('resources.agentGuide')}</ResourceLink></div><div className="help-path"><strong>{t('app.helpPathSshTitle')}</strong><p>{t('app.helpPathSshNote')}</p><ResourceLink resource="remote-setup">{t('resources.remoteGuide')}</ResourceLink></div></div></section>
        <section className="offline-help help-section"><h3>{t('app.helpSteps')}</h3><ol><li><strong>{t('app.helpStep1Title')}</strong>{t('app.helpStep1')}</li><li><strong>{t('app.helpStep2Title')}</strong>{t('app.helpStep2')}</li><li><strong>{t('app.helpStep3Title')}</strong>{t('app.helpStep3')}</li><li><strong>{t('app.helpStep4Title')}</strong>{t('app.helpStep4')}</li></ol><h4>{t('app.helpContinueTitle')}</h4><p>{t('app.helpContinue')}</p><h4>{t('app.helpCliTitle')}</h4><p>{t('app.helpCli')}</p></section>
        <section className="help-section"><h3>{t('app.helpErrorsTitle')}</h3><p>{t('app.helpErrors')}</p><ul className="help-errors"><li><strong>{t('app.helpError1Title')}</strong>{t('app.helpError1')}</li><li><strong>{t('app.helpError2Title')}</strong>{t('app.helpError2')}</li><li><strong>{t('app.helpError3Title')}</strong>{t('app.helpError3')}</li></ul><div className="inline-route"><button className="text-button" onClick={() => openAgent()}>{t('app.helpTerminalAgent')}</button></div><Notice>{t('app.helpNotice')}</Notice></section>
        <DeveloperResources/>
        <p className="help-revision">{t('app.helpRevision',{version:buildIdentity.version,revision:buildIdentity.docs_revision,dirty:buildIdentity.source_dirty ? t('app.revisionDirty') : ''})}</p>
      </div>
      <div className="modal-footer"><button onClick={() => setDialog('settings')}>{t('app.viewModules')}</button><button className="primary" onClick={() => setDialog(null)}>{t('app.gotIt')}</button></div>
    </Modal>}
    {dialog === 'support' && <Modal title={t('app.supportTitle')} wide onClose={() => setDialog(null)}><div className="modal-body"><Notice>{t('app.supportDialogNote')}</Notice><textarea className="support-preview" aria-label={t('app.supportAria')} value={support} readOnly spellCheck={false}/></div><div className="modal-footer"><button onClick={() => setDialog(null)}>{t('app.close')}</button><button onClick={downloadSupport}><Icon name="download" size={15}/>{t('app.saveJson')}</button><button className="primary" onClick={() => void copy(support)}><Icon name="copy" size={15}/>{t('app.copyData')}</button></div></Modal>}
    {launchTarget && <Modal title={t('app.launchInProjectTitle')} wide onClose={() => setLaunchTarget(null)}><div className="modal-body"><LaunchPanel environment={launchTarget.environment} hostAlias={launchTarget.hostAlias} proxyUrl={launchTarget.proxyUrl}/></div></Modal>}
    {flow && <Modal title={t(flow.receipt ? 'app.executionResult' : flow.uncertain ? 'app.resultToVerify' : 'app.confirmPlan')} wide onClose={() => { if (busy !== 'execute') { setFlow(null); setError(''); } }}><div className="plan-target">{(flow.plan?.network || flow.receipt?.network_change) ? <><Icon name="environments" size={21}/><div><strong>{flow.hostAlias ?? t('app.localLabel')} · {t('app.hostSharedNetwork')}</strong><span>{flow.plan?.network?.service_name ?? flow.receipt?.network_change?.service_name ?? t('app.originalService')}</span></div><span className="small-label">{t('app.affectsAllApps')}</span></> : flow.environment && <><Icon name="terminal" size={21}/><div><strong>{flow.environment.name}</strong><span>{flow.hostAlias ?? t(flow.environment.host==='local'?'app.localLabel':flow.environment.host)} · {flow.environment.surface==='claude-code'?'Claude Code':flow.environment.surface}</span></div><span className="small-label">{t('app.approvalTargetOnly')}</span><code className="approval-root">{flow.environment.root}</code><button className="icon-button" aria-label={t('label.copyFrozenTarget')} onClick={() => { if (flow.environment) void copy(flow.environment.root); }}><Icon name="copy" size={15}/></button></>}</div><div className="modal-body flow-body"><div className="flow-intro"><Clawd small mood={busy === 'execute' ? 'work' : flow.receipt ? flow.receipt.status === 'completed' ? 'done' : 'care' : 'plan'}/><div><span className="eyebrow">{flow.receipt ? 'the actual result' : 'take a look together'}</span><h3 className="flow-title">{p(flow.receipt?.title ?? flow.plan?.title)}</h3></div></div>{error && <RequestFailure error={error}/>}
      {(flow.receipt?.service ?? flow.plan?.service) && <ServiceEvidence service={(flow.receipt?.service ?? flow.plan?.service)!}/>}{flow.receipt ? <><TaskOutcome receipt={flow.receipt}/><NetworkReceiptEvidence receipt={flow.receipt} host={flow.hostAlias ?? t('app.localLabel')}/><p className="small-print">{t('app.recordTime',{time:fmtDate(flow.receipt.created_at)})}</p><div className="steps">{flow.receipt.steps.map((step, index) => <div className="step" key={step.id}><span className="step-number">{index + 1}</span><div><strong>{p(step.label)}</strong><p>{p(step.message)}</p></div><Status value={step.status}/></div>)}</div>{flow.receipt.error && <Notice tone="error"><strong>{flow.receipt.error.message}</strong><p><code>{flow.receipt.error.code}</code> · {flow.receipt.error.phase}</p>{flow.receipt.error.uncertain_side_effects && <p>{t('app.uncertainSideEffects')}</p>}{flow.receipt.error.recovery && <p>{flow.receipt.error.recovery}</p>}</Notice>}<Warnings items={flow.receipt.warnings}/>{flow.receipt.settings_recovery && <Notice tone={flow.receipt.settings_recovery.state === 'written' ? 'neutral' : 'warning'}><strong>{t('app.settingsWriteVerify')}</strong><p>{p(flow.receipt.settings_recovery.message)}</p>{flow.receipt.restorable && <p>{t('app.settingsRecoveryNote')}</p>}</Notice>}{!!flow.receipt.next_steps?.length && !flow.receipt.task_result?.next_actions.length && <section className="receipt-next"><h4>{t('app.nextUp')}</h4><ul>{flow.receipt.next_steps.map((step, index) => <li key={index}>{p(step)}</li>)}</ul></section>}<ExecutionEvidence execution={flow.receipt.execution}/>{flow.receipt.policy && <PolicyEvidence policy={flow.receipt.policy} projected/>}{flow.receipt.local_cleanup && <div className="fact-row"><span>{t('app.localCleanup')}</span><Status value={flow.receipt.local_cleanup}/></div>}{flow.receipt.remote_revocation && <div className="fact-row"><span>{t('app.remoteRevocation')}</span><Status value={flow.receipt.remote_revocation}/></div>}{flow.receipt.outcome === 'preserved' && <Notice>{t('app.preservedNotice')}</Notice>}{flow.receipt.migration_probe && flow.receipt.migration_probe.status !== 'removed' && <><div className="fact-row"><span>{t('app.migrationProbeDir')}</span><Path value={flow.receipt.migration_probe.path}/></div><Notice tone="warning">{t('app.migrationProbeNote')}</Notice></>}{flow.receipt.new_root && <div className="fact-row"><span>{t(flow.receipt.steps.some(step => step.id === 'create' && step.status === 'completed') ? 'app.newConfigRoot' : 'app.newConfigRootPending')}</span><Path value={flow.receipt.new_root}/></div>}{(['config_archive_path','identity_archive_path','residue_archive_path'] as const).map(key=>flow.receipt?.[key]?<div className="fact-row" key={key}><span>{t(`app.${key}`)}</span><Path value={flow.receipt[key]!}/></div>:null)}{flow.receipt.identity_reset&&<Notice>{t(flow.receipt.identity_reset.removal_verified?'identity.removalVerified':'identity.removalUnverified')}</Notice>}{flow.receipt.state_archive_path && <div className="fact-row"><span>{t(flow.receipt.steps.some(step => step.id === 'state_backup' && step.status === 'completed') ? 'app.stateBackup' : 'app.stateBackupPending')}</span><Path value={flow.receipt.state_archive_path}/></div>}{flow.receipt.archive_path && <div className="fact-row"><span>{t(hasConfirmedWorkArchive(flow.receipt) ? 'app.archiveTitle' : 'app.workArchivePending')}</span><Path value={flow.receipt.archive_path}/></div>}<div className="receipt-id"><span>{t('app.taskId')}</span><code>{flow.receipt.id}</code><button className="icon-button" aria-label={t('label.copyTaskId')} onClick={() => void copy(flow.receipt!.id)}><Icon name="copy" size={14}/></button></div>{flow.hostAlias && !flow.receipt.network_change && <Notice>{t('app.remoteLaunchNote')}</Notice>}{!flow.receipt.restorable && <p className="muted">{t('app.noRestorableAction')}</p>}</> : flow.plan && <>{flow.hostAlias && <Notice>{t('app.remoteSubmitNote')}</Notice>}<Warnings items={flow.plan.warnings}/>{flow.plan.network ? <NetworkPlanReview network={flow.plan.network} host={flow.hostAlias ?? t('app.localLabel')}/> : <PlanReview plan={flow.plan}/>}{flow.plan.archive_passphrase_required && <section className="archive-password"><h4>{t('app.archivePasswordTitle')}</h4><p>{flow.initialPassphrase ? t('app.archivePasswordImport') : t('app.archiveFileCount',{count:flow.plan.file_count ?? t('app.selectedCount')})}</p><div className="password-fields"><label className="field">{t('app.archivePassphrase')}<input type="password" autoComplete="new-password" minLength={12} value={archivePassphrase} onChange={event => setArchivePassphrase(event.target.value)}/></label><label className="field">{t('app.archivePassphraseConfirm')}<input type="password" autoComplete="new-password" value={archiveConfirmation} onChange={event => setArchiveConfirmation(event.target.value)}/></label></div>{archiveConfirmation && archiveConfirmation !== archivePassphrase && <p className="password-error">{t('app.archivePassphraseMismatch')}</p>}</section>}{flow.plan.policy && <PolicyEvidence policy={flow.plan.policy} projected/>}<section className="plan-steps"><h4>{t('app.actualSteps')}</h4>{flow.plan.actions.map(action => <div className="action-description" key={action.id}><span>{p(action.label)}</span><span className="small-label">{t(action.reversible ? 'app.reversible' : 'app.noAutoRecovery')}</span></div>)}</section><details className="plan-identity"><summary>{t('app.planIdentity')}</summary><span>{flow.plan.network ? t('app.hostSharedNetworkService') : flow.environment?.root}</span><code className="full-path">{flow.plan.id}</code></details>{flow.uncertain && <Notice tone="warning">{t('app.uncertainNote')}</Notice>}</>}
      </div><div className="modal-footer flow-footer"><span>{flow.environment?.name ?? t('app.hostSharedNetwork')}</span><div className="button-row"><button disabled={!!busy} onClick={() => openAgent(flow)}>{t('app.handToAgent')}</button>{flow.receipt ? <><button onClick={() => { setFlow(null); setPage('history'); }}>{t('app.viewRecords')}</button>{['accepted','executing','verifying','pending','interrupted','needs_reconciliation'].includes(flow.receipt.status) && <button disabled={!!busy} onClick={refreshReceipt}>{t('app.queryLatest')}</button>}{hasConfirmedWorkArchive(flow.receipt) && <button onClick={() => { setArchiveJobId(flow.receipt!.id); setFlow(null); setPage('sessions'); }}>{t('app.viewWorkArchive')}</button>}{flow.receipt.new_environment_id && environments.some(environment => environment.id === flow.receipt!.new_environment_id) && <button onClick={() => {selectEnvironment(flow.receipt!.new_environment_id!);setFlow(null);setPage('policy');}}>{t('app.choosePolicyForNewEnv')}</button>}{flow.receipt.restorable && <button disabled={!!busy} onClick={() => restore(flow.receipt!)}>{t('app.previewRestore')}</button>}{flow.receipt.service_restorable && <button disabled={!!busy} onClick={() => resumeService(flow.receipt!)}>{t('app.previewRestoreService')}</button>}{flow.receipt.service ? <button className="primary" onClick={() => { setFlow(null); setPage('rebuild'); }}>{t('app.backToRebuild')}</button> : flow.environment && !flow.receipt.network_change && flow.receipt.outcome !== 'archived' && flow.environment.status !== 'retired' && <button className="primary" title={!['completed','partially_completed','failed'].includes(flow.receipt.status) ? t('app.queryCompletionFirst') : hostAlias ? t('app.openRemoteTerminal') : undefined} disabled={!!busy || !['completed','partially_completed','failed'].includes(flow.receipt.status) || !(flow.receipt.new_environment_id ? environments.find(env => env.id === flow.receipt!.new_environment_id)?.executable : flow.environment?.executable)} onClick={() => { const target = environments.find(env => env.id === flow.receipt?.new_environment_id) ?? flow.environment; if(target)launch(target,flow.hostAlias ?? null);setFlow(null); }}>{t('app.openClaude')}<Icon name="arrow" size={15}/></button>}</> : flow.uncertain ? <><button onClick={() => { setFlow(null); setPage('history'); setError(''); }}>{t('app.backToRecords')}</button><button className="primary" disabled={!!busy} onClick={reconcile}>{t('app.queryOriginalTask')}</button></> : <><button disabled={!!busy} onClick={() => { setFlow(null); setError(''); }}>{t('app.backToAdjust')}</button><button className="primary" disabled={!!busy || !flow.plan?.actions.length || (!!flow.plan?.archive_passphrase_required && (archivePassphrase.length < 12 || archivePassphrase !== archiveConfirmation))} onClick={() => void executePlan()}>{busy === 'execute' ? t('app.runningPleaseWait') : t('app.approveAndRun')}<Icon name="arrow" size={15}/></button></>}</div></div></Modal>}
  </div></WorkspaceActionsContext.Provider>;
}

function SettingsRows({ settings }: { settings: Inspection['settings'] }) { const { t, locale } = useI18n(); const lbl = useLabel(); return <div className="settings-list">{settings.length === 0 ? <p className="padded muted">{t('app.noSettingsInEnv')}</p> : settings.map(setting => <details className="setting-row" key={setting.key} data-key={setting.key}><summary><span><strong>{present(setting.label, locale)}</strong><span className="setting-timing">{lbl(setting.effect_timing)}</span></span><code>{setting.value ?? t('app.notSet')}</code><Status value={setting.status}/><Icon name="chevron" size={13}/></summary><div className="setting-detail"><code>{setting.key}</code><span>{t('customPolicy.rawCurrent')}：<code>{setting.value ?? t('app.notSet')}</code></span><span>{t('app.source',{source:setting.source})}</span><span>{t('app.recordState',{status:lbl(setting.status),timing:lbl(setting.effect_timing)})}</span></div></details>)}</div>; }
const categoryKeys: Record<string,string> = { instructions: 'category.instructions', memory: 'category.memory', sessions: 'category.sessions', settings: 'category.settings', projects: 'category.projects', credentials: 'category.credentials' };
function categoryKey(value: string) { return categoryKeys[value] ?? value; }
function SelectTarget({ onAdd }: { onAdd: () => void }) { const { t } = useI18n(); return <div className="empty-state bordered"><Icon name="environments" size={32}/><h2>{t('app.chooseEnvFirst')}</h2><p>{t('app.chooseEnvNote')}</p><button onClick={onAdd}>{t('app.addExistingEnv')}</button></div>; }

function ThemeSwitch({ theme, onChange }: { theme: Theme; onChange: (theme: Theme) => void }) { const { t } = useI18n(); return <div className="theme-switch" role="group" aria-label={t('app.themeLabel')}>{(([['light','app.themeShortLight','sun'],['dark','app.themeShortDark','moon'],['system','app.themeShortSystem','environments']]) as const).map(([value,title,icon]) => <button key={value} title={t(value === 'light' ? 'app.themeLight' : value === 'dark' ? 'app.themeDark' : 'app.themeSystem')} aria-label={t(value === 'light' ? 'app.themeLight' : value === 'dark' ? 'app.themeDark' : 'app.themeSystem')} aria-pressed={theme === value} onClick={() => onChange(value)}><Icon name={icon} size={13}/><span>{t(title)}</span></button>)}</div>; }

// A single quiet change check when an environment opens. It runs once per
// host/environment/root generation (never per render, and never again on a
// language switch, since those deps are excluded), reports unchanged silently,
// and leaves only a lightweight tip for a real change or an incomplete read.
// It never scans metadata repeatedly and never touches launch approval identity.
function DriftTip({ send, environmentId, environmentName, canCompare, onCompare, onDismiss }: {
  send: ReturnType<typeof requester>; environmentId: string; environmentName: string;
  canCompare: boolean; onCompare: () => void; onDismiss: () => void;
}) {
  const { t } = useI18n();
  const [state, setState] = useState<'idle' | 'changed' | 'incomplete' | 'rules_updated'>('idle');
  const generation = useRef(0);
  useEffect(() => {
    const current = ++generation.current;
    void send('drift', { environment_id: environmentId, lightweight:true })
      .then(data => {
        if (current !== generation.current) return;
        if (data.status === 'unchanged' || data.status === 'no_baseline') return;
        if (data.status === 'rules_updated') {setState('rules_updated');return;}
        setState(data.changes.length > 0 ? 'changed' : 'incomplete');
      })
      .catch(() => { /* A quiet check never surfaces an error or blocks the workspace. */ });
    return () => { generation.current += 1; };
  }, [send, environmentId, environmentName]);
  if (state === 'idle') return null;
  return <div className="workspace-tip" role="status">
    <Icon name="warning" size={15}/>
    <span>{t(state === 'changed' ? 'app.driftTipChanged' : state==='rules_updated'?'app.driftTipRulesUpdated':'app.driftTipIncomplete', { name: environmentName })}</span>
    {canCompare && <button className="text-button" onClick={onCompare}>{t('app.driftTipCompare')}<Icon name="arrow" size={13}/></button>}
    <button className="icon-button" aria-label={t('app.dismiss')} onClick={onDismiss}><Icon name="close" size={13}/></button>
  </div>;
}

// A mounted check owns one exact host/environment generation. Leaving that
// target (including A -> B -> A) disposes its results, errors and accept action.
function DriftCheck({ send, environmentId, disabled, onPolicy,onCompare }: {
  send: ReturnType<typeof requester>; environmentId: string; disabled: boolean; onPolicy: () => void;onCompare:()=>void;
}) {
  const { t } = useI18n();
  const [result, setResult] = useState<Drift | null>(null);
  const [pending, setPending] = useState(false);
  const [failure, setFailure] = useState<Error | null>(null);
  const [message, setMessage] = useState(false);
  const generation = useRef(0);
  useEffect(() => () => { generation.current += 1; }, []);
  async function run(accept = false) {
    if (disabled || pending || (accept && !result?.changes.length)) return;
    const current = ++generation.current;
    setPending(true); setFailure(null); setMessage(false);
    try {
      if (accept) await send('accept_drift', { environment_id: environmentId });
      if (current !== generation.current) return;
      const data = await send('drift', { environment_id: environmentId });
      if (current !== generation.current) return;
      setResult(data);
      if (accept) setMessage(true);
    } catch (error) {
      if (current === generation.current) setFailure(asError(error));
    } finally {
      if (current === generation.current) setPending(false);
    }
  }
  return <div className="drift-block" aria-busy={pending}>
    <div><h3>{t('app.driftTitle')}</h3><p>{t('app.driftNote')}</p></div>
    <button disabled={disabled || pending} onClick={() => void run()}><Icon name="refresh" size={15}/>{pending ? t('app.driftChecking') : t('app.driftCheck')}</button>
    {failure && <RequestFailure error={failure}/>}
    {message && <span role="status">{t('app.driftAccepted')}</span>}
    {result && <div className="drift-result"><Status value={result.status}/>{result.changes.length > 0 ? <>
      {result.scope!=='configuration_sources_only'&&<SettingsRows settings={result.changes}/>}<div className="button-row"><button onClick={onPolicy}>{t('app.driftReapply')}</button>
      {result.scope==='configuration_sources_only'?<button disabled={disabled||pending} onClick={onCompare}>{t('app.driftTipCompare')}</button>:<button disabled={disabled || pending} onClick={() => void run(true)}>{t('app.driftAccept')}</button>}</div>
    </> : <span>{t('app.driftNone')}</span>}</div>}
  </div>;
}
