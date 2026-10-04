import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { RequestError, transport, type Envelope } from './api';
import Clawd from './Clawd';
import RequestFailure, { asError } from './RequestFailure';
import { ResourceLink } from './Resources';
import { Icon, Notice, Status } from './ui';
import type { Environment, Receipt } from './types';

export async function remoteRequest<T>(payload: Record<string, unknown>): Promise<T> {
  if (transport !== 'native') throw new RequestError('NATIVE_REQUIRED', 'SSH 连接只在桌面应用中可用。合成预览不会连接真实主机。');
  const result = await invoke<Envelope<T>>('remote_request', { payload });
  if (!result.ok) throw new RequestError(result.error.code, result.error.message, result.error.diagnostic);
  return result.data;
}
type InstallPlan = { install_id: string; alias: string; probe: { os: string; architecture: string; uid: string }; bundle: { version: string; target: string; bytes: number; sha256: string }; destination: string; approval: string; status: string; effects: string[]; last_error?: {message: string} | null };
const installStatus: Record<string, string> = { previewed:'等待批准', needs_reconciliation:'结果待核对', ready:'运行器已就绪', not_installed:'未找到匹配的运行器', installed_unverified:'文件已安装，运行能力未通过' };
type Inventory = { installations?: InstallPlan[]; hosts: { alias: string }[]; tasks: { alias: string; plan_id: string; lookup_id: string; status: string }[] };
export default function RemotePanel({ current, onSelect, onRemoved, onOpenReceipt }: { current: string | null; onSelect: (alias: string | null) => void; onRemoved: (alias: string) => void; onOpenReceipt: (alias: string, receipt: Receipt, environment: Environment) => void }) {
  const [inventory, setInventory] = useState<Inventory>({ hosts: [], tasks: [] });
  const [aliases, setAliases] = useState<string[]>([]);
  const [alias, setAlias] = useState('');
  const [coverage, setCoverage] = useState('');
  const [busy, setBusy] = useState('');
  const [failure, setFailure] = useState<{ error: Error; context: string; retry?: () => void } | null>(null);
  const [removed, setRemoved] = useState<string | null>(null);
  const [installPlan, setInstallPlan] = useState<InstallPlan | null>(null);
  const [result, setResult] = useState<{ alias: string; receipt: Receipt } | null>(null);
  const alive = useRef(false);
  const failureRef = useRef<HTMLDivElement>(null);
  const native = transport === 'native';
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
  useEffect(() => { alive.current = true; if (native) void run('refresh', '读取主机列表', refresh); return () => { alive.current = false; }; }, []);
  useEffect(() => { if (failure) failureRef.current?.scrollIntoView({ block: 'nearest' }); }, [failure]);
  async function remove(target: string) {
    await remoteRequest({ op: 'remove_host', alias: target });
    if (!alive.current) return;
    setRemoved(target);
    setInventory(value => ({ ...value, hosts: value.hosts.filter(host => host.alias !== target) }));
    onRemoved(target);
  }
  return <div className="remote-panel"><div className="module-intro"><Clawd small mood="work"/><div><h3>远一点，也能看清每一步</h3><p>先确认 SSH 能登录，再检查远端 Lintel runner。登记主机不会安装软件。</p></div></div>
    {!native && <Notice>合成测试没有真实 SSH 连接。请在桌面应用中选择已配置的 SSH alias。</Notice>}
    <details className="remote-prerequisites"><summary>连接前需要什么？</summary><ol><li>在系统 SSH 中配置 Host alias，并核对主机身份。</li><li>密钥已交给 OpenSSH 或系统 agent；Lintel 的非交互连接不会弹出密码输入框。</li><li>远端需要 <code>lintel</code> runner。Linux x86_64 / arm64 可以在这里检查、预览并批准安装 App 内置版本；只放入目标用户的专用目录。已有 PATH runner 也可直接连接。</li></ol><div className="resource-links"><ResourceLink resource="remote-setup">Lintel 远端准备说明</ResourceLink><ResourceLink resource="vps-basics">VPS 101</ResourceLink><ResourceLink resource="ssh-troubleshooting">SSH 排障手册</ResourceLink></div></details>
    {failure && <div ref={failureRef}><RequestFailure key={`${failure.context}:${failure.error.message}`} error={failure.error} context={failure.context}/>{failure.retry && <button disabled={!!busy} onClick={failure.retry}>重新检查连接</button>}</div>}
    {removed && <div className="remote-removed" role="status"><span>已从 Lintel 移除 <strong>{removed}</strong>。系统 SSH 配置与原任务记录均保留。</span><button disabled={!!busy} onClick={() => void run('undo', `重新登记 ${removed}`, async () => { await remoteRequest({ op: 'add_host', alias: removed }); if (alive.current) { setRemoved(null); await refresh(); } })}>撤销移除</button></div>}
    <div className="host-row"><div><strong>本机</strong><small>当前 Mac 的环境与本地记录</small></div><button disabled={!!busy || current === null} onClick={() => onSelect(null)}>{current === null ? '当前工作空间' : '切换到本机'}</button></div>
    {inventory.hosts.map(host => <div className="host-row" key={host.alias}>
      <div><strong>{host.alias}</strong><small>{current === host.alias ? '当前工作空间 · 重连可重新核验' : '已登记的 SSH 引用 · 尚未在此面板核验'}</small></div>
      <div className="host-actions"><button disabled={!!busy} onClick={() => void run(`prepare:${host.alias}`, `检查 ${host.alias} 的运行器安装条件`, async () => { const plan = await remoteRequest<InstallPlan>({ op:'prepare_runner', alias:host.alias }); if (alive.current) { setInstallPlan(plan); await refresh(); } })}>{busy === `prepare:${host.alias}` ? '正在检查…' : '检查并准备运行器'}</button><button disabled={!!busy} onClick={() => void run(`connect:${host.alias}`, `连接 ${host.alias}`, async () => { await remoteRequest({ op: 'connect', alias: host.alias }); if (alive.current) onSelect(host.alias); })}>{busy === `connect:${host.alias}` ? '正在检查连接…' : current === host.alias ? '重新连接' : '连接并管理'}</button><button className="text-button" aria-label={`从 Lintel 移除 ${host.alias}`} title="仅移除 Lintel 登记；系统 SSH 配置和任务记录保留" disabled={!!busy} onClick={() => void run(`remove:${host.alias}`, `移除 ${host.alias}`, () => remove(host.alias))}>移除</button></div>
    </div>)}
    <form className="remote-add" onSubmit={event => { event.preventDefault(); void run('add', `登记 ${alias.trim()}`, async () => { await remoteRequest({ op: 'add_host', alias: alias.trim() }); if (alive.current) { setAlias(''); await refresh(); } }); }}><label className="field">SSH Host alias<input disabled={!native || !!busy} list="ssh-aliases" value={alias} onChange={event => setAlias(event.target.value)} placeholder="例如：writing-server" spellCheck={false}/><datalist id="ssh-aliases">{aliases.map(value => <option key={value} value={value}/>)}</datalist><small>使用 ~/.ssh/config 已有别名；不在这里收集密码、私钥或 host key。</small></label><button disabled={!native || !!busy || !alias.trim()}>{busy === 'add' ? '正在登记…' : '登记主机'}</button></form><p className="small-print">{coverage || '登记仅保存连接引用。连接会核验已有 host key，未知主机请先用系统 SSH 建立可信记录。'}</p>
    {installPlan && <section className="remote-install" aria-label="运行器安装预览">
      <div className="install-heading"><span className="install-symbol"><Icon name={installPlan.status === 'ready' ? 'check' : 'terminal'} size={22}/></span><div><span className="eyebrow">{installPlan.alias}</span><h3>{installPlan.status === 'ready' ? '远端，已经准备好了' : '给远端备好一个运行器'}</h3></div><button className="text-button" disabled={!!busy} onClick={() => setInstallPlan(null)}>收起</button></div>
      <div className="install-meta"><span>{installStatus[installPlan.status] || installPlan.status}</span><span>{installPlan.probe.os} · {installPlan.probe.architecture === 'x86_64' ? 'x86_64' : 'arm64'}</span><span>{installPlan.probe.uid === '0' ? 'root 用户' : `SSH 用户 · UID ${installPlan.probe.uid}`}</span><span>Lintel {installPlan.bundle.version} · {(installPlan.bundle.bytes / 1024 / 1024).toFixed(2)} MiB</span></div>
      <p className="install-description">运行器放进当前 SSH 用户的 Lintel 专用目录，管理范围也属于这个用户。Claude 由其他用户运行时，请选择那个用户的 SSH alias。</p>
      <div className="install-scope"><Icon name="check" size={15}/><span>现有配置与工作内容保留；已有任务继续使用原运行器。</span></div>
      <details className="install-details"><summary>查看安装与校验详情<Icon name="chevron" size={13}/></summary><dl className="install-facts"><div><dt>目标用户</dt><dd>SSH 用户 · UID {installPlan.probe.uid}</dd></div><div><dt>完整安装位置</dt><dd><code>{installPlan.destination}</code></dd></div><div><dt>SHA-256</dt><dd><code>{installPlan.bundle.sha256}</code></dd></div></dl><ol>{installPlan.effects.map(effect => <li key={effect}>{effect}</li>)}</ol><p>不需要 sudo 或远端编译环境；不修改 PATH、shell 启动文件、sshd 或 Claude 配置。远端需具备基础 Linux 工具与 machine-id。已有版本保留，批准只适用于这次预览的主机、用户和文件。</p></details>
      {installPlan.last_error && <Notice>{installPlan.last_error.message}</Notice>}
      <div className="install-footer"><small>{installPlan.status === 'needs_reconciliation' ? '回包中断时，核对同一份安装即可。' : installPlan.status === 'ready' ? '文件与运行能力均已核验，可以进入工作空间。' : '安装范围仅限这个用户的运行器。'} </small>
      {installPlan.status === 'previewed' && <button className="primary" disabled={!!busy} onClick={() => void run(`install:${installPlan.install_id}`, `安装 ${installPlan.alias} 的运行器`, async () => { try { const result = await remoteRequest<InstallPlan>({op:'install_runner', alias:installPlan.alias, install_id:installPlan.install_id, approval:installPlan.approval}); if (alive.current) setInstallPlan(result); } finally { if (alive.current) await refresh(); } })}>{busy === `install:${installPlan.install_id}` ? '正在上传并核验…' : '批准并安装这个运行器'}<Icon name="arrow" size={15}/></button>}
      {installPlan.status !== 'previewed' && installPlan.status !== 'ready' && <button className="primary" disabled={!!busy} onClick={() => void run(`verify:${installPlan.install_id}`, `核对 ${installPlan.alias} 的原安装`, async () => { const result = await remoteRequest<InstallPlan>({op:'query_install',alias:installPlan.alias,install_id:installPlan.install_id}); if (alive.current) { setInstallPlan(result); await refresh(); } })}>核对原安装<Icon name="refresh" size={15}/></button>}
      {installPlan.status === 'ready' && <button className="primary" disabled={!!busy} onClick={() => void run(`connect:${installPlan.alias}`, `连接 ${installPlan.alias}`, async () => { await remoteRequest({op:'connect',alias:installPlan.alias}); if (alive.current) onSelect(installPlan.alias); })}>连接并管理<Icon name="arrow" size={15}/></button>}</div>
    </section>}
    {!!inventory.installations?.length && <details className="install-records separated" open={inventory.installations.some(record => record.status === 'needs_reconciliation')}><summary>安装记录<span>{inventory.installations.length}</span></summary>{inventory.installations.map(record => <div className="remote-task" key={record.install_id}><div><strong>{record.alias}</strong><small>{installStatus[record.status] || record.status} · Lintel {record.bundle.version}</small><details><summary>查看标识</summary><code>{record.install_id}</code></details></div>{record.status === 'previewed' ? <button disabled={!!busy} onClick={() => setInstallPlan(record)}>查看安装预览</button> : <button disabled={!!busy} onClick={() => void run(`verify:${record.install_id}`, `核对 ${record.alias} 的原安装`, async () => { const result = await remoteRequest<InstallPlan>({op:'query_install',alias:record.alias,install_id:record.install_id}); if (alive.current) { setInstallPlan(result); await refresh(); } })}>核对原安装</button>}</div>)}<p className="small-print">中断后只核对同一份安装，不重新上传。移除 alias 仍保留安装和任务记录。</p></details>}
    <section className="separated"><div className="browser-panel-heading"><h3>持久提交记录</h3><button disabled={!native || !!busy} onClick={() => void run('refresh', '刷新主机与任务', refresh)}><Icon name="refresh" size={14}/>刷新</button></div>{!inventory.tasks.length ? <p className="small-print">还没有从桌面提交的远程任务。连接中断后，可在这里查询原任务。</p> : inventory.tasks.map(task => <div className="remote-task" key={`${task.alias}-${task.plan_id}`}><div><strong>{task.alias}</strong><code>{task.plan_id}</code>{!inventory.hosts.some(host => host.alias === task.alias) && <small>主机已从列表移除 · 原任务仍可查询</small>}</div><Status value={task.status}/><button disabled={!!busy} onClick={() => void run(`query:${task.plan_id}`, `查询 ${task.alias} 的原任务`, async () => { const result = await remoteRequest<Receipt>({ op: 'reconnect', alias: task.alias, plan_id: task.plan_id }); if (alive.current) { setResult({ alias: task.alias, receipt: result }); await refresh(); } })}>查询原任务</button></div>)}</section>
    {result && <section className="remote-receipt"><div className="browser-panel-heading"><div><span className="eyebrow">{result.alias} · 原任务</span><h3>{result.receipt.title}</h3></div><Status value={result.receipt.status}/></div><div className="steps">{result.receipt.steps.map(step => <div className="step" key={step.id}><div><strong>{step.label}</strong><p>{step.message}</p></div><Status value={step.status}/></div>)}</div><Notice>这份结果属于 {result.alias}。查询不会重新提交；恢复仍需单独预览并批准。{!inventory.hosts.some(host => host.alias === result.alias) && ' 主机已移除，请先重新登记。'}</Notice><button disabled={!!busy || !inventory.hosts.some(host => host.alias === result.alias)} onClick={() => void run(`open:${result.receipt.id}`, `打开 ${result.alias} 的原任务`, async () => { const data = await remoteRequest<{ environments: Environment[] }>({ op: 'request', alias: result.alias, request: { command: 'discover' } }); if (!alive.current) return; const environment = data.environments.find(env => env.id === result.receipt.environment_id); if (!environment) throw new Error('原环境不在这台主机的当前清单中。任务记录已保留，请先核对环境。'); onOpenReceipt(result.alias, result.receipt, environment); })}>查看完整回执与恢复<Icon name="arrow" size={15}/></button></section>}
  </div>;
}
