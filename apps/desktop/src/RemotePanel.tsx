import { present } from './i18n-presentation';
import { LocalizedError } from './i18n';
import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { RequestError, transport, type Envelope } from './api';
import Clawd from './Clawd';
import ExecutionEvidence from './ExecutionEvidence';
import RequestFailure, { asError } from './RequestFailure';
import { ResourceLink } from './Resources';
import { Icon, Notice, Status } from './ui';
import { useI18n } from './i18n-provider';
import type { Environment, Receipt, LaunchRecord } from './types';

export async function remoteRequest<T>(payload: Record<string, unknown>): Promise<T> {
  if (transport !== 'native') throw new RequestError('NATIVE_REQUIRED', 'remote.sshnative');
  const result = await invoke<Envelope<T>>('remote_request', { payload });
  if (!result.ok) throw new RequestError(result.error.code, result.error.message, result.error.diagnostic);
  return result.data;
}
type InstallPlan = { install_id: string; alias: string; probe: { os: string; architecture: string; uid: string }; bundle: { version: string; target: string; bytes: number; sha256: string }; destination: string; approval: string; status: string; effects: string[]; last_error?: {message: string} | null };
const installStatusKeys: Record<string, string> = { previewed:'remote.install.previewed', needs_reconciliation:'remote.install.needsReconciliation', ready:'remote.install.ready', superseded:'remote.install.superseded', verified:'remote.install.verified', not_installed:'remote.install.notInstalled', installed_unverified:'remote.install.installedUnverified' };
type Inventory = { installations?: InstallPlan[]; launches?: (LaunchRecord & {alias:string})[]; hosts: { alias: string }[]; tasks: { alias: string; plan_id: string; lookup_id: string; status: string }[] };
export default function RemotePanel({ current, onSelect, onRemoved, onOpenReceipt, onOpenLaunch }: { current: string | null; onSelect: (alias: string | null) => void; onRemoved: (alias: string) => void; onOpenReceipt: (alias: string, receipt: Receipt, environment: Environment) => void; onOpenLaunch: (alias:string, record:LaunchRecord)=>void }) {
  const { t, locale } = useI18n();
  const p = (value:string|undefined) => present(value,locale);
  const installStatus = (status: string) => installStatusKeys[status] ? t(installStatusKeys[status]) : status;
  const [inventory, setInventory] = useState<Inventory>({ hosts: [], tasks: [] });
  const [aliases, setAliases] = useState<string[]>([]);
  const [alias, setAlias] = useState('');
  const [coverage, setCoverage] = useState('');
  const [busy, setBusy] = useState('');
  const [failure, setFailure] = useState<{ error: Error; context: string; retry?: () => void } | null>(null);
  const [removed, setRemoved] = useState<string | null>(null);
  const [installPlan, setInstallPlan] = useState<InstallPlan | null>(null);
  const [result, setResult] = useState<{ alias: string; receipt: Receipt } | null>(null);
  const [launchResult,setLaunchResult] = useState<{alias:string;record:LaunchRecord}|null>(null);
  const alive = useRef(false);
  const failureRef = useRef<HTMLDivElement>(null);
  const native = transport === 'native';
  const pendingInstall = failure?.error instanceof RequestError && failure.error.code === 'install_reconciliation_required' ? failure.error.diagnostic : undefined;
  async function run(name: string, context: string, action: () => Promise<void>) {
    setBusy(name); setFailure(null);
    try { await action(); }
    catch (err) { if (alive.current) setFailure({ error: asError(err), context, retry: name.startsWith('connect:') ? () => void run(name, context, action) : undefined }); }
    finally { if (alive.current) setBusy(''); }
  }
  async function refresh() {
    const hosts = await remoteRequest<Inventory>({ op: 'hosts' });
    if (!alive.current) return;
    setInventory(hosts);
    setInstallPlan(current => current ? hosts.installations?.find(record => record.install_id === current.install_id) ?? current : null);
    const config = await remoteRequest<{ aliases: string[]; coverage: string }>({ op: 'aliases' });
    if (!alive.current) return;
    setAliases(config.aliases); setCoverage(config.coverage);
  }
  useEffect(() => { alive.current = true; if (native) void run('refresh', t('remote.listHosts'), refresh); return () => { alive.current = false; }; }, []);
  useEffect(() => { if (failure) failureRef.current?.scrollIntoView({ block: 'nearest' }); }, [failure]);
  async function remove(target: string) {
    await remoteRequest({ op: 'remove_host', alias: target });
    if (!alive.current) return;
    setRemoved(target);
    setInventory(value => ({ ...value, hosts: value.hosts.filter(host => host.alias !== target) }));
    onRemoved(target);
  }
  return <div className="remote-panel"><div className="module-intro"><Clawd small mood="work"/><div><h3>{t('remote.introTitle')}</h3><p>{t('remote.introNote')}</p></div></div>
    {!native && <Notice>{t('remote.syntheticNote')}</Notice>}
    <details className="remote-prerequisites"><summary>{t('remote.prereqTitle')}</summary><ol><li>{t('remote.prereq1')}</li><li>{t('remote.prereq2')}</li><li>{t('remote.prereq3a')} <code>lintel</code> {t('remote.prereq3b')}</li></ol><div className="resource-links"><ResourceLink resource="remote-setup">{t('remote.resources.setup')}</ResourceLink><ResourceLink resource="vps-basics">VPS 101</ResourceLink><ResourceLink resource="ssh-troubleshooting">{t('remote.resources.troubleshoot')}</ResourceLink></div></details>
    {failure && <div ref={failureRef}><RequestFailure key={`${failure.context}:${failure.error.message}`} error={failure.error} context={failure.context}/>{pendingInstall?.install_id && pendingInstall.alias && (() => { const alias = pendingInstall.alias!; const installId = pendingInstall.install_id!; return <button disabled={!!busy} onClick={() => void run(`verify:${installId}`, t('remote.verifyInstallCtx',{alias}), async () => { const result = await remoteRequest<InstallPlan>({ op: 'query_install', alias, install_id: installId }); if (alive.current) { setInstallPlan(result); await refresh(); } })}>{t('remote.checkPendingInstall')}<Icon name="refresh" size={14}/></button>; })()}{failure.retry && <button disabled={!!busy} onClick={failure.retry}>{t('remote.recheckConnection')}</button>}</div>}
    {removed && <div className="remote-removed" role="status"><span>{t('remote.removedPrefix')} <strong>{removed}</strong>{t('remote.removedSuffix')}</span><button disabled={!!busy} onClick={() => void run('undo', t('remote.reregisterCtx',{alias:removed}), async () => { await remoteRequest({ op: 'add_host', alias: removed }); if (alive.current) { setRemoved(null); await refresh(); } })}>{t('remote.undoRemove')}</button></div>}
    <div className="host-row"><div><strong>{t('remote.local')}</strong><small>{t('remote.localNote')}</small></div><button disabled={!!busy || current === null} onClick={() => onSelect(null)}>{t(current === null ? 'remote.currentWorkspace' : 'remote.switchLocal')}</button></div>
    {inventory.hosts.map(host => <div className="host-row" key={host.alias}>
      <div><strong>{host.alias}</strong><small>{t(current === host.alias ? 'remote.currentReconnect' : 'remote.registeredUnchecked')}</small></div>
      <div className="host-actions"><button disabled={!!busy} onClick={() => void run(`prepare:${host.alias}`, t('remote.prepareCtx',{alias:host.alias}), async () => { const plan = await remoteRequest<InstallPlan>({ op:'prepare_runner', alias:host.alias }); if (alive.current) { setInstallPlan(plan); await refresh(); } })}>{busy === `prepare:${host.alias}` ? t('remote.checking') : t('remote.checkPrepareRunner')}</button><button disabled={!!busy} onClick={() => void run(`connect:${host.alias}`, t('remote.connectCtx',{alias:host.alias}), async () => { await remoteRequest({ op: 'connect', alias: host.alias }); if (alive.current) onSelect(host.alias); })}>{busy === `connect:${host.alias}` ? t('remote.checkingConnection') : current === host.alias ? t('remote.reconnect') : t('remote.connectManage')}</button><button className="text-button" aria-label={t('remote.removeAria',{alias:host.alias})} title={t('remote.removeTitle')} disabled={!!busy} onClick={() => void run(`remove:${host.alias}`, t('remote.removeCtx',{alias:host.alias}), () => remove(host.alias))}>{t('remote.remove')}</button></div>
    </div>)}
    <form className="remote-add" onSubmit={event => { event.preventDefault(); void run('add', t('remote.registerCtx',{alias:alias.trim()}), async () => { await remoteRequest({ op: 'add_host', alias: alias.trim() }); if (alive.current) { setAlias(''); await refresh(); } }); }}><label className="field">{t('remote.aliasLabel')}<input disabled={!native || !!busy} list="ssh-aliases" value={alias} onChange={event => setAlias(event.target.value)} placeholder={t('remote.aliasPlaceholder')} spellCheck={false}/><datalist id="ssh-aliases">{aliases.map(value => <option key={value} value={value}/>)}</datalist><small>{t('remote.aliasNote')}</small></label><button disabled={!native || !!busy || !alias.trim()}>{busy === 'add' ? t('remote.registering') : t('remote.registerHost')}</button></form><p className="small-print">{coverage || t('remote.coverageNote')}</p>
    {installPlan && <section className="remote-install" aria-label={t('remote.installAria')}>
      <div className="install-heading"><span className="install-symbol"><Icon name={installPlan.status === 'ready' ? 'check' : 'terminal'} size={22}/></span><div><span className="eyebrow">{installPlan.alias}</span><h3>{t(installPlan.status === 'ready' ? 'remote.installReadyTitle' : 'remote.installPrepareTitle')}</h3></div><button className="text-button" disabled={!!busy} onClick={() => setInstallPlan(null)}>{t('remote.collapse')}</button></div>
      <div className="install-meta"><span>{installStatus(installPlan.status)}</span><span>{installPlan.probe.os} · {installPlan.probe.architecture === 'x86_64' ? 'x86_64' : 'arm64'}</span><span>{installPlan.probe.uid === '0' ? t('remote.rootUser') : t('remote.sshUser',{uid:installPlan.probe.uid})}</span><span>Lintel {installPlan.bundle.version} · {(installPlan.bundle.bytes / 1024 / 1024).toFixed(2)} MiB</span></div>
      <p className="install-description">{t('remote.installDescription')}</p>
      <div className="install-scope"><Icon name="check" size={15}/><span>{t('remote.installScope')}</span></div>
      <details className="install-details"><summary>{t('remote.installDetails')}<Icon name="chevron" size={13}/></summary><dl className="install-facts"><div><dt>{t('remote.targetUser')}</dt><dd>{t('remote.sshUser',{uid:installPlan.probe.uid})}</dd></div><div><dt>{t('remote.installLocation')}</dt><dd><code>{installPlan.destination}</code></dd></div><div><dt>SHA-256</dt><dd><code>{installPlan.bundle.sha256}</code></dd></div></dl><ol>{installPlan.effects.map(effect => <li key={effect}>{effect}</li>)}</ol><p>{t('remote.installNote')}</p></details>
      {installPlan.status === 'superseded' && <Notice>{t('remote.supersededNote')}</Notice>}{installPlan.status === 'verified' && <Notice>{t('remote.verifiedNote')}</Notice>}{installPlan.last_error && <Notice>{installPlan.last_error.message}</Notice>}
      <div className="install-footer"><small>{t(installPlan.status === 'needs_reconciliation' ? 'remote.footerReconcile' : installPlan.status === 'ready' ? 'remote.footerReady' : 'remote.footerScope')} </small>
      {installPlan.status === 'previewed' && <button className="primary" disabled={!!busy} onClick={() => void run(`install:${installPlan.install_id}`, t('remote.installCtx',{alias:installPlan.alias}), async () => { try { const result = await remoteRequest<InstallPlan>({op:'install_runner', alias:installPlan.alias, install_id:installPlan.install_id, approval:installPlan.approval}); if (alive.current) setInstallPlan(result); } finally { if (alive.current) await refresh(); } })}>{busy === `install:${installPlan.install_id}` ? t('remote.uploading') : t('remote.approveInstall')}<Icon name="arrow" size={15}/></button>}
      {installPlan.status !== 'previewed' && installPlan.status !== 'ready' && <button className="primary" disabled={!!busy} onClick={() => void run(`verify:${installPlan.install_id}`, t('remote.verifyInstallCtx',{alias:installPlan.alias}), async () => { const result = await remoteRequest<InstallPlan>({op:'query_install',alias:installPlan.alias,install_id:installPlan.install_id}); if (alive.current) { setInstallPlan(result); await refresh(); } })}>{t('remote.checkOriginalInstall')}<Icon name="refresh" size={15}/></button>}
      {installPlan.status === 'ready' && <button className="primary" disabled={!!busy} onClick={() => void run(`connect:${installPlan.alias}`, t('remote.connectCtx',{alias:installPlan.alias}), async () => { await remoteRequest({op:'connect',alias:installPlan.alias}); if (alive.current) onSelect(installPlan.alias); })}>{t('remote.connectManage')}<Icon name="arrow" size={15}/></button>}</div>
    </section>}
    {!!inventory.installations?.length && <details className="install-records separated" open={inventory.installations.some(record => record.status === 'needs_reconciliation')}><summary>{t('remote.installRecords')}<span>{inventory.installations.length}</span></summary>{inventory.installations.map(record => <div className="remote-task" key={record.install_id}><div><strong>{record.alias}</strong><small>{installStatus(record.status)} · Lintel {record.bundle.version}</small><details><summary>{t('remote.viewId')}</summary><code>{record.install_id}</code></details></div>{record.status === 'previewed' ? <button disabled={!!busy} onClick={() => setInstallPlan(record)}>{t('remote.viewInstallPreview')}</button> : <button disabled={!!busy} onClick={() => void run(`verify:${record.install_id}`, t('remote.verifyInstallCtx',{alias:record.alias}), async () => { const result = await remoteRequest<InstallPlan>({op:'query_install',alias:record.alias,install_id:record.install_id}); if (alive.current) { setInstallPlan(result); await refresh(); } })}>{t('remote.checkOriginalInstall')}</button>}</div>)}<p className="small-print">{t('remote.installRecordsNote')}</p></details>}
    <section className="separated"><div className="browser-panel-heading"><h3>{t('remote.tasksTitle')}</h3><button disabled={!native || !!busy} onClick={() => void run('refresh', t('remote.refreshHostsTasks'), refresh)}><Icon name="refresh" size={14}/>{t('app.refresh')}</button></div>{!inventory.tasks.length ? <p className="small-print">{t('remote.noTasks')}</p> : inventory.tasks.map(task => <div className="remote-task" key={`${task.alias}-${task.plan_id}`}><div><strong>{task.alias}</strong><code>{task.plan_id}</code>{!inventory.hosts.some(host => host.alias === task.alias) && <small>{t('remote.hostRemovedQueryable')}</small>}</div><Status value={task.status}/><button disabled={!!busy} onClick={() => void run(`query:${task.plan_id}`, t('remote.queryCtx',{alias:task.alias}), async () => { const result = await remoteRequest<Receipt>({ op: 'reconnect', alias: task.alias, plan_id: task.plan_id }); if (alive.current) { setResult({ alias: task.alias, receipt: result }); await refresh(); } })}>{t('app.queryOriginalTask')}</button></div>)}</section>
    {!!inventory.launches?.length && <section className="separated remote-launch-records"><h3>{t('remote.launchRecordsTitle')}</h3><p className="small-print">{t('remote.launchRecordsNote')}</p>{inventory.launches.map(record=><div className="remote-task" key={`${record.alias}:${record.request_id}`}><div><strong>{record.alias}</strong><code>{record.request_id}</code>{!inventory.hosts.some(host=>host.alias===record.alias) && <small>{t('remote.hostRemovedLaunchQueryable')}</small>}</div><Status value={record.status}/><div className="button-row"><button disabled={!!busy} onClick={()=>void run(`launch-query:${record.request_id}`,t('remote.checkOriginalLaunchCtx',{alias:record.alias}),async()=>{const found=await remoteRequest<LaunchRecord>({op:'launch_query',alias:record.alias,request_id:record.request_id});if(alive.current)setLaunchResult({alias:record.alias,record:found})})}>{t('remote.queryOriginalLaunch')}</button><button disabled={!!busy} onClick={()=>onOpenLaunch(record.alias,record)}>{t('remote.toAgentVerify')}</button></div></div>)}</section>}
    {launchResult && <section className="remote-receipt"><div className="browser-panel-heading"><h3>{t('remote.originalLaunchTitle',{alias:launchResult.alias})}</h3><Status value={launchResult.record.status}/></div><code className="full-path">{launchResult.record.request_id}</code><p>{launchResult.record.message ?? t('remote.launchReadBack')}</p><div className="fact-row"><span>{t('launchRecords.configRoot')}</span><code>{launchResult.record.root ?? launchResult.record.config_root ?? t('remote.notRead')}</code></div><div className="fact-row"><span>{t('launchRecords.projectCwd')}</span><code>{launchResult.record.project_cwd ?? t('remote.notRead')}</code></div><button onClick={()=>onOpenLaunch(launchResult.alias,launchResult.record)}>{t('launchRecords.toAgent')}<Icon name="arrow" size={14}/></button></section>}
    {result && <section className="remote-receipt"><div className="browser-panel-heading"><div><span className="eyebrow">{t('remote.originalTaskEyebrow',{alias:result.alias})}</span><h3>{p(result.receipt.title)}</h3></div><Status value={result.receipt.status}/></div><div className="steps">{result.receipt.steps.map(step => <div className="step" key={step.id}><div><strong>{p(step.label)}</strong><p>{p(step.message)}</p></div><Status value={step.status}/></div>)}</div><ExecutionEvidence execution={result.receipt.execution}/><Notice>{t('remote.resultNote',{alias:result.alias})}{!inventory.hosts.some(host => host.alias === result.alias) && t('remote.resultReregister')}</Notice><button disabled={!!busy || !inventory.hosts.some(host => host.alias === result.alias)} onClick={() => void run(`open:${result.receipt.id}`, t('remote.openOriginalCtx',{alias:result.alias}), async () => { const data = await remoteRequest<{ environments: Environment[] }>({ op: 'request', alias: result.alias, request: { command: 'discover' } }); if (!alive.current) return; const environment = data.environments.find(env => env.id === result.receipt.environment_id); if (!environment) throw new LocalizedError('remote.envMissing'); onOpenReceipt(result.alias, result.receipt, environment); })}>{t('remote.viewFullReceipt')}<Icon name="arrow" size={15}/></button></section>}
  </div>;
}
