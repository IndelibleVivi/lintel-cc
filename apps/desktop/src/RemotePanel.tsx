import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { RequestError, transport, type Envelope } from './api';
import Clawd from './Clawd';
import RequestFailure, { asError } from './RequestFailure';
import { ResourceLink } from './Resources';
import { Icon, Notice, Status } from './ui';
import type { Receipt } from './types';

export async function remoteRequest<T>(payload: Record<string, unknown>): Promise<T> {
  if (transport !== 'native') throw new RequestError('NATIVE_REQUIRED', 'SSH 连接只在桌面应用中可用。合成预览不会连接真实主机。');
  const result = await invoke<Envelope<T>>('remote_request', { payload });
  if (!result.ok) throw new RequestError(result.error.code, result.error.message, result.error.diagnostic);
  return result.data;
}
type Inventory = { hosts: { alias: string }[]; tasks: { alias: string; plan_id: string; lookup_id: string; status: string }[] };
export default function RemotePanel({ current, onSelect, onRemoved }: { current: string | null; onSelect: (alias: string | null) => void; onRemoved: (alias: string) => void }) {
  const [inventory, setInventory] = useState<Inventory>({ hosts: [], tasks: [] });
  const [aliases, setAliases] = useState<string[]>([]);
  const [alias, setAlias] = useState('');
  const [coverage, setCoverage] = useState('');
  const [busy, setBusy] = useState('');
  const [failure, setFailure] = useState<{ error: Error; context: string; retry?: () => void } | null>(null);
  const [removed, setRemoved] = useState<string | null>(null);
  const [receipt, setReceipt] = useState<Receipt | null>(null);
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
    <details className="remote-prerequisites"><summary>连接前需要什么？</summary><ol><li>在系统 SSH 中配置 Host alias，并核对主机身份。</li><li>密钥已交给 OpenSSH 或系统 agent；Lintel 的非交互连接不会弹出密码输入框。</li><li>远端需要单独安装 <code>lintel</code> runner，并放在 SSH 非交互命令的 PATH 中。装了 Claude Code 不等于装了 Lintel runner。</li></ol><div className="resource-links"><ResourceLink resource="remote-setup">Lintel 远端准备说明</ResourceLink><ResourceLink resource="vps-basics">VPS 101</ResourceLink><ResourceLink resource="ssh-troubleshooting">SSH 排障手册</ResourceLink></div></details>
    {failure && <div ref={failureRef}><RequestFailure key={`${failure.context}:${failure.error.message}`} error={failure.error} context={failure.context}/>{failure.retry && <button disabled={!!busy} onClick={failure.retry}>重新检查连接</button>}</div>}
    {removed && <div className="remote-removed" role="status"><span>已从 Lintel 移除 <strong>{removed}</strong>。系统 SSH 配置与原任务记录均保留。</span><button disabled={!!busy} onClick={() => void run('undo', `重新登记 ${removed}`, async () => { await remoteRequest({ op: 'add_host', alias: removed }); if (alive.current) { setRemoved(null); await refresh(); } })}>撤销移除</button></div>}
    <div className="host-row"><div><strong>本机</strong><small>当前 Mac 的环境与本地记录</small></div><button disabled={!!busy || current === null} onClick={() => onSelect(null)}>{current === null ? '当前工作空间' : '切换到本机'}</button></div>
    {inventory.hosts.map(host => <div className="host-row" key={host.alias}>
      <div><strong>{host.alias}</strong><small>{current === host.alias ? '当前工作空间 · 重连可重新核验' : '已登记的 SSH 引用 · 尚未在此面板核验'}</small></div>
      <div className="host-actions"><button disabled={!!busy} onClick={() => void run(`connect:${host.alias}`, `连接 ${host.alias}`, async () => { await remoteRequest({ op: 'connect', alias: host.alias }); if (alive.current) onSelect(host.alias); })}>{busy === `connect:${host.alias}` ? '正在检查连接…' : current === host.alias ? '重新连接' : '连接并管理'}</button><button className="text-button" aria-label={`从 Lintel 移除 ${host.alias}`} title="仅移除 Lintel 登记；系统 SSH 配置和任务记录保留" disabled={!!busy} onClick={() => void run(`remove:${host.alias}`, `移除 ${host.alias}`, () => remove(host.alias))}>移除</button></div>
    </div>)}
    <form className="remote-add" onSubmit={event => { event.preventDefault(); void run('add', `登记 ${alias.trim()}`, async () => { await remoteRequest({ op: 'add_host', alias: alias.trim() }); if (alive.current) { setAlias(''); await refresh(); } }); }}><label className="field">SSH Host alias<input disabled={!native || !!busy} list="ssh-aliases" value={alias} onChange={event => setAlias(event.target.value)} placeholder="例如：writing-server" spellCheck={false}/><datalist id="ssh-aliases">{aliases.map(value => <option key={value} value={value}/>)}</datalist><small>使用 ~/.ssh/config 已有别名；不在这里收集密码、私钥或 host key。</small></label><button disabled={!native || !!busy || !alias.trim()}>{busy === 'add' ? '正在登记…' : '登记主机'}</button></form><p className="small-print">{coverage || '登记仅保存连接引用。连接会核验已有 host key，未知主机请先用系统 SSH 建立可信记录。'}</p>
    <section className="separated"><div className="browser-panel-heading"><h3>持久提交记录</h3><button disabled={!native || !!busy} onClick={() => void run('refresh', '刷新主机与任务', refresh)}><Icon name="refresh" size={14}/>刷新</button></div>{!inventory.tasks.length ? <p className="small-print">还没有从桌面提交的远程任务。连接中断后，可在这里查询原任务。</p> : inventory.tasks.map(task => <div className="remote-task" key={`${task.alias}-${task.plan_id}`}><div><strong>{task.alias}</strong><code>{task.plan_id}</code>{!inventory.hosts.some(host => host.alias === task.alias) && <small>主机已从列表移除 · 原任务仍可查询</small>}</div><Status value={task.status}/><button disabled={!!busy} onClick={() => void run(`query:${task.plan_id}`, `查询 ${task.alias} 的原任务`, async () => { const result = await remoteRequest<Receipt>({ op: 'reconnect', alias: task.alias, plan_id: task.plan_id }); if (alive.current) { setReceipt(result); await refresh(); } })}>查询原任务</button></div>)}</section>
    {receipt && <section className="remote-receipt"><h3>{receipt.title}</h3><Status value={receipt.status}/><div className="steps">{receipt.steps.map(step => <div className="step" key={step.id}><div><strong>{step.label}</strong><p>{step.message}</p></div><Status value={step.status}/></div>)}</div><Notice>查询只取回原结果。完整恢复与归档入口位于这台主机的“记录与恢复”。主机已移除时，可先重新登记并连接。</Notice></section>}
  </div>;
}
