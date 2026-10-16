import { useEffect, useMemo, useRef, useState } from 'react';
import { request } from './api';
import type { Environment, EnvComparison, Plan, SnapshotComparison, SnapshotListResult, SnapshotSummary } from './types';
import { Icon, Notice, Status, formatBytes } from './ui';
import RequestFailure, { asError } from './RequestFailure';
import { useI18n } from './i18n-provider';
import {present} from './i18n-presentation';

// Finite state vocabulary from core `snapshot.rs` compare/compare_pair.
const changeStateKeys: Record<string, string> = {
  added: 'snapshot.change.added', removed: 'snapshot.change.removed', changed: 'snapshot.change.changed',
  unchanged: 'snapshot.change.unchanged', uncertain: 'snapshot.change.uncertain', incomplete: 'snapshot.change.incomplete',
  appeared: 'snapshot.change.appeared', disappeared: 'snapshot.change.disappeared', flags_changed: 'snapshot.change.flagsChanged',
  same: 'snapshot.change.same', different: 'snapshot.change.different', unknown: 'snapshot.change.unknown',
};
const areaKeys: Record<string, string> = {
  settings: 'snapshot.area.settings', declarations: 'snapshot.area.declarations', identity: 'snapshot.area.identity', residue: 'snapshot.area.residue',
};
const identityKeys: Record<string, string> = { same: 'snapshot.identity.same', different: 'snapshot.identity.different', unknown: 'snapshot.identity.unknown' };
const statusKeys: Record<string, string> = {
  changed: 'snapshot.status.changed', rules_updated: 'snapshot.status.rulesUpdated', incomplete: 'snapshot.status.incomplete', unchanged: 'snapshot.status.unchanged',
};
const filterKeys: Record<string, string> = { all: 'snapshot.filter.all', added: 'snapshot.filter.added', removed: 'snapshot.filter.removed', changed: 'snapshot.filter.changed' };

function ChangeRow({ change }: { change: SnapshotComparison['changes'][number] }) {
  const { t,locale } = useI18n();
  const p = (value: unknown) => change.masked ? '••••••' : (value === null || value === undefined ? t('snapshot.notSet') : typeof value === 'string' ? value : JSON.stringify(value));
  const label = present(change.label || (change.key ? change.key : change.category || change.path || change.subject || ''),locale);
  const tone = ['added', 'appeared'].includes(change.state) ? 'attention' : ['removed', 'disappeared', 'changed', 'flags_changed'].includes(change.state) ? 'attention' : 'good';
  return <div className="compare-change" data-area={change.area} data-state={change.state}>
    <div className="compare-change-head"><span className="compare-area">{areaKeys[change.area] ? t(areaKeys[change.area]) : change.area}</span><strong>{label}</strong><span className={`status ${tone}`}>{changeStateKeys[change.state] ? t(changeStateKeys[change.state]) : change.state}</span></div>
    {change.area === 'settings' && <div className="compare-values"><code>{p(change.old)}</code><Icon name="arrow" size={13}/><code>{p(change.new)}</code></div>}
    {change.area === 'declarations' && <div className="compare-values"><code>{change.path ?? change.scope ?? ''}</code>{change.flags?.length ? <span className="small-print">{change.flags.map(flag => `${flag.flag}: ${String(flag.from)} → ${String(flag.to)}`).join(' · ')}</span> : null}</div>}
    {change.area === 'residue' && <div className="compare-values"><span>{t('snapshot.residueDelta', { count: change.count_delta ?? 0, bytes: formatBytes(Math.abs(change.bytes_delta ?? 0)) })}</span></div>}
    {change.reinterpreted_under_new_rules && <p className="small-print">{t('snapshot.reinterpreted')}</p>}
    {change.area === 'identity' && <p className="small-print">{identityKeys[change.state] ? t(identityKeys[change.state]) : change.state}</p>}
  </div>;
}

export default function SnapshotPanel({ send = request, environment, environments, disabled = false, onPlan }: {
  send?: typeof request; environment: Environment; environments: Environment[]; disabled?: boolean;
  onPlan: (plan: Plan, environment: Environment) => void;
}) {
  const { t, formatDateTime } = useI18n();
  const [snapshots, setSnapshots] = useState<SnapshotSummary[]>([]);
  const [listMeta, setListMeta] = useState<SnapshotListResult | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<Error | null>(null);
  const [busy, setBusy] = useState('');
  const [oldId, setOldId] = useState('');
  const [newId, setNewId] = useState('');
  const [comparison, setComparison] = useState<SnapshotComparison | null>(null);
  const [other, setOther] = useState('');
  const [pair, setPair] = useState<EnvComparison | null>(null);
  const [stateFilter, setStateFilter] = useState<'all' | 'added' | 'removed' | 'changed'>('all');
  const [selectedChanges,setSelectedChanges]=useState<Set<string>>(new Set());
  const [applyKeys,setApplyKeys]=useState<Set<string>>(new Set());
  const [projectCwd,setProjectCwd]=useState('');
  const alive = useRef(true); const lock = useRef(false);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  useEffect(() => {
    let current = true; setError(null); setComparison(null); setPair(null);setSelectedChanges(new Set());setApplyKeys(new Set());
    void send('snapshot_list', { environment_id: environment.id })
      .then(data => { if (current && alive.current) { setSnapshots(data.snapshots); setListMeta(data); setOldId(data.accepted_snapshot_id ?? data.snapshots[1]?.snapshot_id ?? ''); setNewId(data.snapshots[0]?.snapshot_id ?? ''); } })
      .catch(err => { if (current && alive.current) setError(asError(err)); });
    return () => { current = false; };
  }, [send, environment.id, environment.root]);
  async function run(name: string, action: () => Promise<void>) {
    if (lock.current) return; lock.current = true; setBusy(name); setError(null); setNotice(null);
    try { await action(); } catch (err) { if (alive.current) setError(asError(err)); } finally { lock.current = false; if (alive.current) setBusy(''); }
  }
  const save = () => run('save', async () => {
    const result = await send('snapshot_save', { environment_id: environment.id,...(projectCwd.trim()?{project_cwd:projectCwd.trim()}:{}) });
    const list = await send('snapshot_list', { environment_id: environment.id });
    if (alive.current) { setSnapshots(list.snapshots); setListMeta(list); setNewId(result.snapshot.snapshot_id); setOldId(current=>current||list.accepted_snapshot_id||list.snapshots.find(item=>item.snapshot_id!==result.snapshot.snapshot_id)?.snapshot_id||''); setNotice(t('snapshot.saved', { time: formatDateTime(result.snapshot.created_at) })); }
  });
  const compare = () => run('compare', async () => {
    if (!oldId || !newId) return;
    const result = await send('snapshot_compare', { environment_id: environment.id, old_snapshot_id: oldId, new_snapshot_id: newId });
    if (alive.current) {setComparison(result);setSelectedChanges(new Set());}
  });
  const compareEnvs = () => run('compareEnvs', async () => {
    if (!other || other === environment.id) return;
    const result = await send('env_compare', { environment_id: environment.id, other_environment_id: other,...(projectCwd.trim()?{project_cwd:projectCwd.trim()}:{}) });
    if (alive.current) {setPair(result);setApplyKeys(new Set());}
  });
  const accept=()=>run('accept',async()=>{
    const result=await send('snapshot_accept',{environment_id:environment.id,old_snapshot_id:oldId,new_snapshot_id:newId,selected_changes:[...selectedChanges]});
    const list=await send('snapshot_list',{environment_id:environment.id});
    const comparison=await send('snapshot_compare',{environment_id:environment.id,old_snapshot_id:result.baseline.snapshot_id,new_snapshot_id:newId});
    if(alive.current){setSnapshots(list.snapshots);setListMeta(list);setOldId(result.baseline.snapshot_id);setComparison(comparison);setSelectedChanges(new Set());setNotice(t('snapshot.accepted'));}
  });
  // Only ordinary (non-identity) configuration may be applied to another
  // environment: login credentials and local identity are excluded by core.
  const applyNormal = (keys: string[]) => run('apply', async () => {
    const chosen = keys.filter(key => key !== 'OTEL_EXPORTER_OTLP_HEADERS');
    if (!chosen.length || !other) return;
    const plan = await send('plan_config_apply', { environment_id: other, source_environment_id: environment.id, selected_keys: chosen });
    const target = environments.find(item => item.id === other);
    if (alive.current && target) onPlan(plan, target);
  });
  const visibleChanges = useMemo(() => (comparison?.changes ?? []).filter(change => stateFilter === 'all' || change.state === stateFilter || (stateFilter === 'added' && change.state === 'appeared') || (stateFilter === 'removed' && change.state === 'disappeared')), [comparison, stateFilter]);
  const otherEnvironments = environments.filter(item => item.id !== environment.id);
  return <section className="surface snapshot-panel" aria-label={t('snapshot.aria')}>
    <div className="surface-heading"><div><h2>{t('snapshot.title')}</h2><span className="small-label">{t('snapshot.desensitized')}</span></div><button disabled={disabled || busy === 'save'} onClick={() => void save()}><Icon name="check" size={14}/>{busy === 'save' ? t('snapshot.saving') : t('snapshot.save')}</button></div>
    <p className="padded small-print">{t('snapshot.intro')}</p>
    <label className="field padded">{t('config.projectCwd')}<input value={projectCwd} onChange={e=>setProjectCwd(e.target.value)} placeholder={t('config.projectCwdPlaceholder')} disabled={disabled||busy!==''}/></label>
    {notice && <p className="padded" role="status">{notice}</p>}
    {error && <div className="padded"><RequestFailure error={error} context={t('snapshot.title')}/></div>}
    <div className="padded compare-pickers">
      <label className="field">{t('snapshot.old')}<select aria-label={t('snapshot.old')} value={oldId} onChange={event => setOldId(event.target.value)} disabled={disabled || busy !== ''}>{snapshots.map(item => <option key={item.snapshot_id} value={item.snapshot_id}>{item.baseline?t('snapshot.baselineLabel')+' · ':''}{formatDateTime(item.created_at)} · {item.snapshot_id.slice(0,8)}</option>)}</select></label>
      <label className="field">{t('snapshot.new')}<select aria-label={t('snapshot.new')} value={newId} onChange={event => setNewId(event.target.value)} disabled={disabled || busy !== ''}>{snapshots.map(item => <option key={item.snapshot_id} value={item.snapshot_id}>{item.baseline?t('snapshot.baselineLabel')+' · ':''}{formatDateTime(item.created_at)} · {item.snapshot_id.slice(0,8)}</option>)}</select></label>
      <button className="primary" disabled={disabled || busy !== '' || !oldId || !newId || oldId === newId} onClick={() => void compare()}>{busy === 'compare' ? t('snapshot.comparing') : t('snapshot.compare')}</button>
    </div>
    {snapshots.length < 2 && <Notice>{t('snapshot.needTwo')}</Notice>}
    {listMeta && (!listMeta.scan_complete || listMeta.errors.length > 0 || listMeta.listed < listMeta.total) && <Notice tone="warning">{t('snapshot.listPartial', { listed: listMeta.listed, total: listMeta.total })}</Notice>}
    {comparison && <>
      <div className="padded compare-head"><Status value={comparison.status === 'changed' ? 'drifted' : comparison.status === 'unchanged' ? 'unchanged' : 'unknown'}/><strong>{statusKeys[comparison.status] ? t(statusKeys[comparison.status]) : comparison.status}</strong><p>{comparison.summary}</p></div>
      {comparison.rule_change && <Notice tone="warning">{t('snapshot.ruleChange', { old: comparison.rule_change.old, new: comparison.rule_change.new })}</Notice>}
      {comparison.status !== 'unchanged' && <>
        <div className="segmented compare-filters" role="group" aria-label={t('snapshot.filterAria')}>{(['all', 'added', 'removed', 'changed'] as const).map(id => <button key={id} aria-pressed={stateFilter === id} onClick={() => setStateFilter(id)}>{t(filterKeys[id])}</button>)}</div>
        <div className="compare-changes">{visibleChanges.length ? visibleChanges.map((change, index) => <div key={change.id??index}>{change.id&&!change.masked&&['added','removed','changed','appeared','disappeared','flags_changed'].includes(change.state)&&<label className="checkbox-inline padded"><input type="checkbox" checked={selectedChanges.has(change.id)} disabled={disabled||busy!==''} onChange={e=>setSelectedChanges(current=>{const next=new Set(current);e.target.checked?next.add(change.id!):next.delete(change.id!);return next;})}/><span>{t('snapshot.selectChange',{key:change.key??change.path??change.category??change.subject??''})}</span></label>}<ChangeRow change={change}/></div>) : <p className="muted padded">{t('snapshot.noMatch')}</p>}</div>
        <div className="padded"><button disabled={disabled||busy!==''||!selectedChanges.size} onClick={()=>void accept()}>{t('snapshot.acceptSelected',{count:selectedChanges.size})}</button><p className="small-print">{t('snapshot.acceptNote')}</p></div>
      </>}
    </>}
    <div className="surface-heading separated"><h2>{t('snapshot.pairTitle')}</h2></div>
    <p className="padded small-print">{t('snapshot.pairIntro')}</p>
    {otherEnvironments.length === 0 ? <Notice>{t('snapshot.pairNone')}</Notice> : <div className="padded compare-pickers"><label className="field">{t('snapshot.otherEnv')}<select value={other} onChange={event => setOther(event.target.value)} disabled={disabled || busy !== ''}><option value="">{t('snapshot.chooseEnv')}</option>{otherEnvironments.map(item => <option key={item.id} value={item.id}>{item.name}</option>)}</select></label><button className="primary" disabled={disabled || busy !== '' || !other} onClick={() => void compareEnvs()}>{busy === 'compareEnvs' ? t('snapshot.comparing') : t('snapshot.compareEnvs')}</button></div>}
    {pair && <>{(() => { const c = pair.comparison; return <>
      <div className="padded compare-head"><span className="compare-area">{t('snapshot.identityLabel')}</span><strong>{identityKeys[c.identity.userID] ? t(identityKeys[c.identity.userID]) : c.identity.userID}</strong><span className="small-label">{t('snapshot.pairSides', { a: pair.a.name, b: pair.b.name })}</span></div>
      <details className="environment-diagnostics padded" open><summary>{t('snapshot.pairSettings', { count: Object.keys(c.settings).length })}</summary><ul className="compare-pair-list">{Object.entries(c.settings).map(([key, value]) => <li key={key}>{!value.masked&&<input type="checkbox" aria-label={t('snapshot.selectApply',{key})} checked={applyKeys.has(key)} onChange={e=>setApplyKeys(current=>{const next=new Set(current);e.target.checked?next.add(key):next.delete(key);return next;})}/>}<code>{key}</code>{value.same === null ? <span className="small-label">{value.masked ? t('snapshot.maskedUnknown') : t('snapshot.sameUnknown')}</span> : <span className={value.same ? 'status good' : 'status attention'}>{value.same ? t('snapshot.same') : <><code>{value.a ?? t('snapshot.notSet')}</code> ≠ <code>{value.b ?? t('snapshot.notSet')}</code></>}</span>}</li>)}</ul><div className="button-row"><button className="text-button" disabled={disabled||busy!==''||!applyKeys.size} onClick={()=>void applyNormal([...applyKeys])}>{t('snapshot.applyNormal')}<Icon name="arrow" size={14}/></button><span className="small-print">{t('snapshot.applyNote')}</span></div></details>
      <details className="environment-diagnostics padded"><summary>{t('snapshot.pairDeclarations')}</summary><ul className="compare-pair-list">{c.declarations.only_a.map(item => <li key={'a' + item.path}><span className="small-label">{t('snapshot.onlyThis')}</span><code>{item.path}</code></li>)}{c.declarations.only_b.map(item => <li key={'b' + item.path}><span className="small-label">{t('snapshot.onlyOther')}</span><code>{item.path}</code></li>)}{c.declarations.both.map(item => <li key={'c' + item.path}><span className="small-label">{t('snapshot.both')}</span><code>{item.path}</code></li>)}</ul></details>
      <p className="padded small-print">{c.note}</p>
    </>; })()}</>}
  </section>;
}
