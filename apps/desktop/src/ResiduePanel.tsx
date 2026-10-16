import ResidueArchivePanel from './ResidueArchivePanel';
import { useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { request, transport } from './api';
import { LocalizedError } from './i18n';
import { readTaskDraft, saveTaskDraft } from './taskDrafts';
import type { Environment, IdentityInspection, Plan, ResidueInventory, ResidueLocation } from './types';
import { Icon, Notice, WorkspaceActions, formatBytes } from './ui';
import { contentRecordingOff } from './recipeIntent';
import RequestFailure, { asError } from './RequestFailure';
import { useI18n } from './i18n-provider';

// The finite residue category vocabulary from core's known-location table.
const categoryKeys: Record<string, string> = {
  debug_logs: 'residue.category.debugLogs', shell_snapshots: 'residue.category.shellSnapshots',
  todo_state: 'residue.category.todoState', config_backups: 'residue.category.configBackups',
  session_env: 'residue.category.sessionEnv', sessions: 'residue.category.sessions',
  plugins: 'residue.category.plugins', credentials: 'residue.category.credentials',
  client_state: 'residue.category.clientState', client_state_backup: 'residue.category.clientStateBackup',
  shared_profile: 'residue.category.sharedProfile',
  file_history:'residue.category.fileHistory',tool_results:'residue.category.toolResults',image_cache:'residue.category.imageCache',feature_flags_cache:'residue.category.featureFlags',plan_state:'residue.category.planState',ide_state:'residue.category.ideState',raw_body_dump:'residue.category.rawBodies',explicit_root:'residue.category.explicitRoot',
};
const classKeys: Record<string, string> = {
  work: 'residue.class.work', residue: 'residue.class.residue', identity: 'residue.class.identity', config: 'residue.class.config',
  unclassified:'residue.class.unclassified',
};
const stateKeys: Record<string, string> = {
  present: 'residue.state.present', not_found: 'residue.state.notFound', access_limited: 'residue.state.accessLimited',
};
const identityStateKeys: Record<string, string> = {
  observed: 'identity.stateObserved', not_found: 'identity.stateNotFound', too_large: 'identity.stateTooLarge',
  unreadable: 'identity.stateUnreadable', unknown: 'identity.stateUnknown',
};
const persistenceKeys: Record<string, string> = {
  persistent_across_sessions: 'identity.persistence.persistent', session_account: 'identity.persistence.session',
};
// Default selection: clearable runtime residue only. Work content, identity and
// config stay user-chosen; identity material is never a clear target here.
const clearableClass = 'residue';

function mtime(value: number | null | undefined, format: (v: number) => string) {
  return value === null || value === undefined ? '' : format(value);
}

// One row is one known location. Presence-only locations (a file, an identity
// location) expose a single entry; a scanned directory exposes its bounded list.
function LocationRow({ location, selected, disabled, onToggle, format,deleteWork,onCopy,onLocate }: {
  location: ResidueLocation; selected: Set<string>; disabled: boolean;
  onToggle: (paths: string[], checked: boolean) => void; format: (v: number) => string;
  deleteWork:boolean;onCopy:(path:string)=>void;onLocate:(path:string)=>void;
}) {
  const { t } = useI18n();
  const clearable = ['residue','unclassified',...(deleteWork?['work']:[])].includes(location.class) && location.state === 'present';
  const entries = location.entries;
  const rowPaths = clearable ? entries.map(entry => entry.path) : [];
  const chosen = rowPaths.filter(path => selected.has(path)).length;
  const indeterminate = chosen > 0 && chosen < rowPaths.length;
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => { if (ref.current) ref.current.indeterminate = indeterminate; }, [indeterminate]);
  return <details className="residue-location" data-category={location.category} data-class={location.class}>
    <summary>
      <span className="residue-location-copy">
        <strong>{categoryKeys[location.category] ? t(categoryKeys[location.category]) : location.category}</strong>
        <span className="small-label">{classKeys[location.class] ? t(classKeys[location.class]) : location.class}</span>
        <code title={location.path}>{location.path}</code>
      </span>
      <span className="residue-location-facts">
        {location.state === 'present' && <span>{t('residue.filesBytes', { count: location.file_count, bytes: formatBytes(location.total_bytes) })}</span>}
        {location.truncated && <span className="small-label">{t('residue.truncated')}</span>}
        <span className={`status ${location.state === 'present' ? 'good' : ''}`}><span className="status-dot"/>{stateKeys[location.state] ? t(stateKeys[location.state]) : location.state}</span>
      </span>
      <Icon name="chevron" size={13}/>
    </summary>
    <div className="residue-location-body">
      <code className="residue-full-path">{location.path}</code>
      {location.newest_mtime !== null && <p className="small-print">{t('residue.modified', { time: mtime(location.newest_mtime, format) })}</p>}
      <div className="button-row"><button className="text-button" onClick={()=>onCopy(location.path)}>{t('residue.copyPath')}</button><button className="text-button" disabled={disabled||location.state!=='present'} onClick={()=>onLocate(location.path)}>{t('residue.locate')}</button></div>
      {(location.skipped_special > 0 || location.skipped_unsafe > 0) && <p className="small-print">{t('residue.skipped', { special: location.skipped_special, unsafe: location.skipped_unsafe })}</p>}
      {clearable && rowPaths.length > 0 && <label className="residue-select-all"><input ref={ref} type="checkbox" disabled={disabled} checked={chosen === rowPaths.length && rowPaths.length > 0} aria-label={t('residue.selectAll', { category: categoryKeys[location.category] ? t(categoryKeys[location.category]) : location.category })} onChange={event => onToggle(rowPaths, event.target.checked)}/><span>{t('residue.selectAllNote')}</span></label>}
      {location.class === 'identity' && <Notice>{t('residue.identityPresenceOnly')}</Notice>}
      {location.class === 'work' && <Notice>{t('residue.workHandledByWork')}</Notice>}
      {entries.length > 0 ? <ul className="residue-entry-list">{entries.map(entry => <li key={entry.path}>
        {clearable && <input type="checkbox" disabled={disabled} checked={selected.has(entry.path)} aria-label={t('residue.selectFile', { path: entry.path })} onChange={event => onToggle([entry.path], event.target.checked)}/>}
        <code>{entry.path}</code>
        <small>{formatBytes(entry.bytes)} · {mtime(entry.mtime, format)}</small>
        <span className="residue-entry-actions"><button className="icon-button" aria-label={t('residue.copyItem',{path:entry.path})} title={t('residue.copyPath')} onClick={()=>onCopy(entry.path)}><Icon name="copy" size={14}/></button><button className="icon-button" aria-label={t('residue.locateItem',{path:entry.path})} title={t('residue.locate')} disabled={disabled} onClick={()=>onLocate(entry.path)}><Icon name="environments" size={14}/></button></span>
      </li>)}</ul> : location.state === 'present' && <p className="muted">{t('residue.noEntries')}</p>}
      {location.truncated && <p className="small-print">{t('residue.truncatedNote')}</p>}
    </div>
  </details>;
}

function IdentitySection({ send, environment, disabled, onPlan, reveal, setReveal, onCopy,backupPaths,refreshKey,backups,onToggleBackup }: {
  send: typeof request; environment: Environment; disabled: boolean;
  onPlan: (plan: Plan) => void; reveal: boolean; setReveal: (value: boolean) => void; onCopy: (value: string) => void;
  backupPaths:string[];refreshKey?:string;backups:Set<string>;onToggleBackup:(path:string,checked:boolean)=>void;
}) {
  const { t } = useI18n();
  const [inspection, setInspection] = useState<IdentityInspection | null>(null);
  const [error, setError] = useState<Error | null>(null);
  const [busy, setBusy] = useState(false);
  const [stopped, setStopped] = useState(false);
  const alive = useRef(true);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  useEffect(() => {
    let current = true; setError(null);
    void send('identity_inspect', { environment_id: environment.id, ...(reveal ? { reveal: true as const } : {}) })
      .then(data => { if (current && alive.current) setInspection(data); })
      .catch(err => { if (current && alive.current) setError(asError(err)); });
    return () => { current = false; };
  }, [send, environment.id, environment.root, reveal,refreshKey]);
  async function reset() {
    if (busy || !stopped) return;
    setBusy(true); setError(null);
    try { const plan = await send('plan_identity_reset', { environment_id: environment.id, writers_confirmed_stopped: true,identity_backup_paths:[...backups] }); if (alive.current) onPlan(plan); }
    catch (err) { if (alive.current) setError(asError(err)); }
    finally { if (alive.current) setBusy(false); }
  }
  const revealed = new Map((inspection?.revealed ?? []).map(item => [item.key, item.value]));
  return <section className="surface identity-panel" aria-label={t('identity.aria')}>
    <div className="surface-heading"><div><h2>{t('identity.title')}</h2><span className="small-label">{t('identity.maskedNote')}</span></div><button className="text-button" disabled={disabled} onClick={() => setReveal(!reveal)} aria-pressed={reveal}><Icon name={reveal ? 'close' : 'copy'} size={14}/>{t(reveal ? 'identity.hideRaw' : 'identity.revealRaw')}</button></div>
    <p className="padded small-print">{t('identity.intro')}</p>
    {error && <div className="padded"><RequestFailure error={error} context={t('identity.title')}/></div>}
    <div className="padded">
      <div className="fact-row"><span>{t('identity.state')}</span><span>{inspection ? (identityStateKeys[inspection.state] ? t(identityStateKeys[inspection.state]) : inspection.state) : t('identity.reading')}</span></div>
      <div className="fact-row"><span>{t('identity.storage')}</span><code>{inspection?.path ?? t('identity.unknownPath')}</code></div>
      {inspection && inspection.fields.length > 0 ? <ul className="identity-fields">{inspection.fields.map(field => <li key={field.key}>
        <div><strong>{field.key}</strong><span className="small-label">{persistenceKeys[field.persistence] ? t(persistenceKeys[field.persistence]) : field.persistence}</span></div>
        <div className="identity-value">
          <code>{revealed.has(field.key) ? revealed.get(field.key) : (field.present ? '••••••••' : t('identity.absent'))}</code>
          {field.present && <button className="icon-button" disabled={!revealed.has(field.key)} aria-label={t('identity.copyValue', { key: field.key })} onClick={() => { const value = revealed.get(field.key); if (value) onCopy(value); }}><Icon name="copy" size={14}/></button>}
        </div>
      </li>)}</ul> : !error && <p className="muted">{t('identity.none')}</p>}
      {reveal && <Notice tone="warning">{t('identity.revealWarning')}</Notice>}
    </div>
    <div className="surface-heading separated"><h2>{t('identity.resetTitle')}</h2></div>
    <p className="padded small-print">{t('identity.resetNote')}</p>
    {backupPaths.length>0&&<fieldset className="padded recipe-fields-group"><legend>{t('residue.identityBackups')}</legend>{backupPaths.map(path=><label className="checkbox-inline" key={path}><input type="checkbox" disabled={disabled||busy} checked={backups.has(path)} onChange={e=>onToggleBackup(path,e.target.checked)}/><code>{path}</code></label>)}</fieldset>}
    <label className="checkbox-row"><span><strong>{t('identity.resetConfirm')}</strong><span>{t('identity.resetConfirmDetail')}</span></span><input type="checkbox" checked={stopped} onChange={event => setStopped(event.target.checked)}/></label>
    <Notice>{t('identity.resetScope')}</Notice>
    <div className="padded"><button className="primary" disabled={disabled || busy || !stopped} onClick={() => void reset()}>{busy ? t('identity.planning') : t('identity.resetPlan')}<Icon name="arrow" size={15}/></button></div>
  </section>;
}

export default function ResiduePanel({ send = request, environment, hostAlias = null, disabled = false, onPlan, onWork, seed,refreshKey }: {
  send?: typeof request; environment: Environment; hostAlias?: string | null; disabled?: boolean;
  onPlan: (plan: Plan,passphrase?:string) => void; onWork: () => void;
  refreshKey?:number;
  seed?: { key: string; reduceContent: boolean; resetIdentity: boolean } | null;
}) {
  const { t, formatDateTime } = useI18n();
  const draftKey = 'lintel.residue-draft:' + JSON.stringify([environment.id]);
  const initial = readTaskDraft(draftKey, { archiveBeforeClear: true });
  const [inventory, setInventory] = useState<ResidueInventory | null>(null);
  const [error, setError] = useState<Error | null>(null);
  const [busy, setBusy] = useState(false);
  const [query, setQuery] = useState('');
  const [classFilter, setClassFilter] = useState<'all' | 'residue' | 'work' | 'identity' | 'config'|'unclassified'>('all');
  const [extraRoots,setExtraRoots]=useState('');const [projectFilter,setProjectFilter]=useState('');const [days,setDays]=useState('');const [minKiB,setMinKiB]=useState('');const [deleteWork,setDeleteWork]=useState(false);
  const [onlyClearable, setOnlyClearable] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [archiveBeforeClear, setArchiveBeforeClear] = useState(initial.archiveBeforeClear);
  const [stopped, setStopped] = useState(false);
  const [reveal, setReveal] = useState(false);
  const [notification, setNotification] = useState('');
  // The Finder locate action is 本地-only: it needs the native transport and no
  // remote host. Server-side re-validates the path against a fresh
  // residue_inventory, so a stale selection fails closed.
  const [locating, setLocating] = useState(false);
  const [locateError, setLocateError] = useState<Error | null>(null);
  // Optional combined client workflow: one synthesized plan covering content
  // recording, an optional identity reset and the selected residue clear. This
  // never loops several approved plans — it is a single plan_client_workflow
  // call. The recipe seed pre-fills the two independent choices.
  const [combineReduce, setCombineReduce] = useState(seed?.reduceContent ?? false);
  const [combineResetIdentity, setCombineResetIdentity] = useState(seed?.resetIdentity ?? false);
  const [identityBackups,setIdentityBackups]=useState<Set<string>>(new Set());
  useEffect(() => { if (seed) { setCombineReduce(seed.reduceContent); setCombineResetIdentity(seed.resetIdentity); } }, [seed?.key]);
  const alive = useRef(true); const lock = useRef(false);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  useEffect(() => saveTaskDraft(draftKey, { archiveBeforeClear }), [draftKey, archiveBeforeClear]);
  async function load(more=false) {
    if(lock.current)return;lock.current=true;setBusy(true);setError(null);
    const previous=inventory;if(!more){setInventory(null);setSelected(new Set());}
    try {
      const data=await send('residue_inventory',{environment_id:environment.id,limit:200,extra_roots:extraRoots.split('\n').map(s=>s.trim()).filter(Boolean),...(projectFilter.trim()?{project:projectFilter.trim()}:{}),...(classFilter!=='all'?{class:classFilter}:{}),...(days?{modified_after:Math.floor(Date.now()/1000)-Number(days)*86400}:{}),...(minKiB?{min_bytes:Number(minKiB)*1024}:{}),...(more&&previous?.next_offset!=null?{offset:previous.next_offset,expected_digest:previous.digest}:{})});
      if(alive.current){
        if(more&&previous){setInventory({...data,locations:data.locations.map(location=>({...location,entries:[...(previous.locations.find(old=>old.path===location.path)?.entries??[]),...location.entries].filter((entry,index,all)=>all.findIndex(item=>item.path===entry.path)===index)}))});}
        else {setInventory(data);setSelected(new Set(data.locations.filter(location=>location.class==='residue'&&location.state==='present').flatMap(location=>location.entries.map(entry=>entry.path))));}
      }
    }catch(e){if(alive.current)setError(asError(e));}finally{lock.current=false;if(alive.current)setBusy(false);}
  }
  useEffect(() => { void load(); }, [send, environment.id, environment.root,refreshKey]);
  useEffect(()=>{setReveal(false);},[refreshKey]);
  // Leaving the environment disposes the selection (paths are host-scoped).
  useEffect(() => { setSelected(new Set()); }, [environment.id]);
  async function copy(value: string) {
    try { await navigator.clipboard.writeText(value); setNotification(t('residue.copied')); }
    catch { setNotification(t('residue.copyFailed')); }
  }
  function toggle(paths: string[], checked: boolean) {
    setSelected(current => { const next = new Set(current); for (const path of paths) checked ? next.add(path) : next.delete(path); return next; });
  }
  // Reveal exactly one selected residue path in Finder. The native side
  // re-reads the current residue inventory, matches this path against a listed
  // location or entry, and only then opens Finder. Remote hosts and arbitrary
  // paths are refused by the same boundary; here we also fail closed early.
  function locateablePath(path: string | undefined): boolean {
    if (!path || !inventory) return false;
    return inventory.locations.some(location => location.path === path || location.entries.some(entry => entry.path === path));
  }
  async function locate(path: string) {
    if (locating) return;
    if (transport !== 'native' || hostAlias) { setLocateError(new LocalizedError('residue.locateLocalOnly')); return; }
    if (!locateablePath(path)) { setLocateError(new LocalizedError('residue.locateStale')); return; }
    setLocating(true); setLocateError(null);
    try { await invoke('reveal_local_item', { payload: { environment_id: environment.id, path, extra_roots:inventory?.coverage?.additional_sources.extra_roots??[] } }); if (alive.current) setNotification(t('residue.located')); }
    catch (err) { if (alive.current) setLocateError(err instanceof Error ? new LocalizedError('residue.locateFailed', { message: err.message }) : new LocalizedError('residue.locateFailed', { message: String(err) })); }
    finally { if (alive.current) setLocating(false); }
  }
  async function clear(archive: boolean,archiveOnly=false) {
    if (disabled || lock.current || selected.size === 0 || !stopped) return;
    lock.current = true; setBusy(true); setError(null);
    try {
      const plan = await send('plan_cleanup', {
        environment_id: environment.id, recipe: 'clear_residue', writers_confirmed_stopped: true,
        official_logout: false, categories: [], residue_paths: [...selected].sort(), archive_before_clear: archive,archive_only:archiveOnly,extra_roots:inventory?.coverage?.additional_sources.extra_roots??[],delete_work:deleteWork,
      });
      if (alive.current) onPlan(plan);
    } catch (err) { if (alive.current) setError(asError(err)); }
    finally { lock.current = false; if (alive.current) setBusy(false); }
  }
  // One synthesized plan for the combined journey: reduce content recording
  // (real content fields off), optionally reset the local identifier, and clear
  // the selected residue after an encrypted archive. A single onPlan/approval;
  // never a loop of executes.
  async function combined() {
    if (disabled || lock.current || (!selected.size&&!combineReduce&&!combineResetIdentity) || !stopped) return;
    lock.current = true; setBusy(true); setError(null);
    try {
      const plan = await send('plan_client_workflow', {
        environment_id: environment.id,
        ...(combineReduce ? { policy: { preset: 'custom', keep_remote_control: true, custom_settings: contentRecordingOff() } } : {}),
        ...(selected.size?{residue_paths:[...selected].sort()}:{}), archive_before_clear: archiveBeforeClear,extra_roots:inventory?.coverage?.additional_sources.extra_roots??[],delete_work:deleteWork,
        writers_confirmed_stopped: true, reset_identity: combineResetIdentity,
        ...(combineResetIdentity ? { identity_backup_paths:[...identityBackups].filter(path=>inventory?.locations.some(location=>location.category==='client_state_backup'&&location.entries.some(entry=>entry.path===path))) } : {}),
      });
      if (alive.current) onPlan(plan);
    } catch (err) { if (alive.current) setError(asError(err)); }
    finally { lock.current = false; if (alive.current) setBusy(false); }
  }
  const visible = useMemo(() => {
    const needle = query.trim().toLocaleLowerCase();
    return (inventory?.locations ?? []).filter(location => {
      if (classFilter !== 'all' && location.class !== classFilter) return false;
      if (onlyClearable && !(location.class === clearableClass && location.state === 'present')) return false;
      if (!needle) return true;
      return `${location.path} ${location.category} ${location.entries.map(entry => entry.path).join(' ')}`.toLocaleLowerCase().includes(needle);
    });
  }, [inventory, classFilter, onlyClearable, query]);
  const residueLocations = (inventory?.locations ?? []).filter(location => location.class === clearableClass && location.state === 'present');
  const observedLocations=visible.filter(location=>location.state!=='not_found');
  const absentLocations=visible.filter(location=>location.state==='not_found');
  const residuePaths = residueLocations.flatMap(location => location.entries.map(entry => entry.path));
  const classCounts: Record<string, number> = { residue: 0, work: 0, identity: 0, config: 0 };
  for (const location of inventory?.locations ?? []) if (location.state === 'present') classCounts[location.class] = (classCounts[location.class] ?? 0) + 1;
  return <div className="client-data-panel">
    <div className="residue-toolbar">
      <label className="history-search"><span className="sr-only">{t('residue.searchSr')}</span><input type="search" value={query} onChange={event => setQuery(event.target.value)} placeholder={t('residue.searchPlaceholder')}/></label>
      <div className="segmented" role="group" aria-label={t('residue.classFilter')}>
        {(['all', 'residue', 'work', 'identity', 'config','unclassified'] as const).map(id => <button key={id} aria-pressed={classFilter === id} onClick={() => setClassFilter(id)}>{id === 'all' ? t('residue.class.all') : t(classKeys[id])}{id !== 'all' && classCounts[id] ? <span className="filter-count">{classCounts[id]}</span> : null}</button>)}
      </div>
      <label className="checkbox-inline"><input type="checkbox" checked={onlyClearable} onChange={event => setOnlyClearable(event.target.checked)}/><span>{t('residue.onlyClearable')}</span></label>
      <button className="text-button" disabled={busy || disabled} onClick={() => void load()}><Icon name="refresh" size={14}/>{t('residue.refresh')}</button>
    </div>
    <details className="surface"><summary className="padded">{t('residue.filters')}</summary><div className="padded config-controls"><label className="field">{t('residue.extraRoots')}<textarea value={extraRoots} disabled={busy||disabled} onChange={e=>setExtraRoots(e.target.value)} spellCheck={false}/></label><label className="field">{t('residue.projectFilter')}<input value={projectFilter} disabled={busy||disabled} onChange={e=>setProjectFilter(e.target.value)}/></label><label className="field">{t('residue.recent')}<select value={days} disabled={busy||disabled} onChange={e=>setDays(e.target.value)}><option value="">{t('residue.anyTime')}</option><option value="7">{t('residue.days7')}</option><option value="30">{t('residue.days30')}</option></select></label><label className="field">{t('residue.minSize')}<input type="number" min="0" value={minKiB} disabled={busy||disabled} onChange={e=>setMinKiB(e.target.value)}/></label><button disabled={busy||disabled} onClick={()=>void load()}>{t('residue.applyFilters')}</button></div></details>
    <Notice>{t('residue.scopeNote')}</Notice>
    {error && <RequestFailure error={error} context={t('residue.title')}/>}
    {busy && !inventory && <p className="muted">{t('residue.reading')}</p>}
    {inventory && <>
      <div className="residue-summary"><span>{t('residue.totals', { count: inventory.totals.file_count, bytes: formatBytes(inventory.totals.total_bytes) })}</span><span className="small-label">{t('residue.checkedAt', { time: formatDateTime(inventory.checked_at) })}</span>{!inventory.scan_complete && <span className="small-label warning-text">{t('residue.incomplete')}</span>}</div>
      {observedLocations.length === 0 ? <div className="empty-state bordered residue-empty"><Icon name="environments" size={24}/><h2>{t('residue.empty')}</h2><p>{t('residue.emptyNote')}</p></div>
        : <div className="residue-list">{observedLocations.map(location => <LocationRow key={location.path} location={location} selected={selected} disabled={disabled || busy} onToggle={toggle} format={formatDateTime} deleteWork={deleteWork} onCopy={value=>void copy(value)} onLocate={value=>void locate(value)}/>)}</div>}
      {absentLocations.length>0&&<details className="residue-absent"><summary>{t('residue.absentLocations',{count:absentLocations.length})}<Icon name="chevron" size={13}/></summary><div>{absentLocations.map(location=><LocationRow key={location.path} location={location} selected={selected} disabled={disabled||busy} onToggle={toggle} format={formatDateTime} deleteWork={deleteWork} onCopy={value=>void copy(value)} onLocate={value=>void locate(value)}/>)}</div></details>}
      {inventory.next_offset!=null&&<button disabled={busy||disabled} onClick={()=>void load(true)}>{t('residue.loadMore')}</button>}
      <label className="checkbox-row"><span><strong>{t('residue.deleteWork')}</strong><span>{t('residue.deleteWorkNote')}</span></span><input type="checkbox" checked={deleteWork} onChange={e=>{setDeleteWork(e.target.checked);if(!e.target.checked)setSelected(current=>new Set([...current].filter(path=>!inventory.locations.some(location=>location.class==='work'&&location.entries.some(entry=>entry.path===path)))));}}/></label>
      <div className="residue-tools"><button className="text-button" disabled={disabled || busy || residuePaths.length === 0} onClick={() => toggle(residuePaths, true)}>{t('residue.selectAllResidue')}</button><button className="text-button" disabled={disabled || busy || selected.size === 0} onClick={() => setSelected(new Set())}>{t('residue.clearSelection')}</button><span>{t('residue.selectedCount', { count: selected.size })}</span></div>
      <label className="checkbox-row residue-archive-choice"><span><strong>{t('residue.archiveBeforeClear')}</strong><span>{t('residue.archiveBeforeClearDetail')}</span></span><input type="checkbox" checked={archiveBeforeClear} onChange={event => setArchiveBeforeClear(event.target.checked)}/></label>
      <section className="surface residue-combine"><div className="surface-heading"><h2>{t('residue.combineTitle')}</h2><span className="small-label">{t('residue.combineOneApproval')}</span></div><p className="padded small-print">{t('residue.combineNote')}</p>
        <label className="checkbox-row"><span><strong>{t('residue.combineReduce')}</strong><span>{t('residue.combineReduceDetail')}</span></span><input type="checkbox" checked={combineReduce} onChange={event => setCombineReduce(event.target.checked)}/></label>
        <label className="checkbox-row"><span><strong>{t('residue.combineResetIdentity')}</strong><span>{t('residue.combineResetIdentityDetail')}</span></span><input type="checkbox" checked={combineResetIdentity} onChange={event => setCombineResetIdentity(event.target.checked)}/></label>
      </section>
      <label className="checkbox-row"><span><strong>{t('residue.writersStopped')}</strong><span>{t('residue.writersStoppedDetail')}</span></span><input type="checkbox" checked={stopped} onChange={event => setStopped(event.target.checked)}/></label>
      <Notice tone="warning">{t('residue.clearWarning')}</Notice>
      <div className="inline-route"><span>{t('residue.workLink')}</span><button className="text-button" onClick={onWork}>{t('residue.goWork')}<Icon name="arrow" size={14}/></button></div>
      <section className="surface"><div className="surface-heading"><h2>{t('residue.pathTools')}</h2></div><div className="padded residue-tools-row"><button className="text-button" disabled={selected.size === 0} onClick={() => void copy([...selected].sort().join('\n'))}><Icon name="copy" size={14}/>{t('residue.copySelected')}</button><button className="text-button" disabled={disabled || locating || selected.size !== 1} onClick={() => { const only = [...selected][0]; if (only) void locate(only); }}><Icon name="environments" size={14}/>{locating ? t('residue.locating') : t('residue.locate')}</button><span className="small-print">{t('residue.copyNote')}</span></div>{locateError && <div className="padded"><RequestFailure error={locateError} context={t('residue.pathTools')}/></div>}{hostAlias && <Notice>{t('residue.locateRemoteUnsupported')}</Notice>}</section>
    </>}
    <ResidueArchivePanel send={send} environment={environment} disabled={disabled||busy} onPlan={onPlan}/>
    <IdentitySection backups={identityBackups} onToggleBackup={(path,checked)=>{setIdentityBackups(current=>{const next=new Set(current);checked?next.add(path):next.delete(path);return next;});if(checked)setSelected(current=>new Set([...current].filter(item=>item!==path)));}} send={send} environment={environment} disabled={disabled || busy} onPlan={onPlan} reveal={reveal} setReveal={setReveal} onCopy={value => void copy(value)} refreshKey={inventory?.checked_at} backupPaths={(inventory?.locations??[]).filter(location=>location.category==='client_state_backup').flatMap(location=>location.entries.map(entry=>entry.path))}/>
    <WorkspaceActions><div><strong>{t('residue.dockTitle')}</strong><span>{t('residue.dockScope', { count: selected.size, name: environment.name })}{combineReduce || combineResetIdentity ? ' · ' + t('residue.dockCombined') : ''}</span></div>{combineReduce || combineResetIdentity ? <button disabled={disabled || busy || !stopped} onClick={() => void combined()}>{t('residue.combinePreview')}<Icon name="arrow" size={16}/></button> : <><div className="residue-secondary-actions"><button disabled={disabled||busy||selected.size===0||!stopped} onClick={()=>void clear(true,true)}>{t('residue.archiveOnly')}</button><button disabled={disabled || busy || selected.size === 0 || !stopped} onClick={() => void clear(false)}>{t('residue.clearOnly')}</button></div><button className="primary" disabled={disabled || busy || selected.size === 0 || !stopped} onClick={() => void clear(archiveBeforeClear)}>{archiveBeforeClear ? t('residue.archiveAndClear') : t('residue.clearPreview')}<Icon name="arrow" size={16}/></button></>}</WorkspaceActions>
    {notification && <div className="toast" role="status"><Icon name="check" size={16}/>{notification}</div>}
  </div>;
}
