import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { RequestError, transport } from './api';
import Clawd from './Clawd';
import { Icon, Notice, Status } from './ui';
import type { Receipt } from './types';

export async function remoteRequest<T>(payload: Record<string, unknown>): Promise<T> {
  if (transport !== 'native') throw new RequestError('NATIVE_REQUIRED', 'SSH 连接只在桌面应用中可用。合成预览不会连接真实主机。');
  const result = await invoke<{ok:true;data:T}|{ok:false;error:{code:string;message:string}}>('remote_request', {payload});
  if (!result.ok) throw new RequestError(result.error.code, result.error.message);
  return result.data;
}
type Inventory = { hosts: {alias:string}[]; tasks: {alias:string;plan_id:string;lookup_id:string;status:string}[] };
export default function RemotePanel({ current, onSelect }: { current: string | null; onSelect: (alias: string | null) => void }) {
  const [inventory, setInventory] = useState<Inventory>({hosts:[],tasks:[]});
  const [aliases, setAliases] = useState<string[]>([]);
  const [alias, setAlias] = useState('');
  const [coverage, setCoverage] = useState('');
  const [busy, setBusy] = useState('');
  const [error, setError] = useState('');
  const [receipt, setReceipt] = useState<Receipt | null>(null);
  const native = transport === 'native';
  async function run(name: string, action: () => Promise<void>) { setBusy(name); setError(''); try { await action(); } catch (err) { setError(err instanceof Error ? err.message : String(err)); } finally { setBusy(''); } }
  async function refresh() { const hosts = await remoteRequest<Inventory>({op:'hosts'}); setInventory(hosts); const config = await remoteRequest<{aliases:string[];coverage:string}>({op:'aliases'}); setAliases(config.aliases); setCoverage(config.coverage); }
  useEffect(() => { if (native) void run('refresh', refresh); }, []);
  return <div className="remote-panel"><div className="module-intro"><Clawd small mood="work"/><div><h3>远一点，也能看清每一步</h3><p>使用系统 OpenSSH 的现有 Host alias；目标上的 runner 负责计划、执行和记录。</p></div></div>
    {!native && <Notice>合成测试没有真实 SSH 连接。请在桌面应用中选择已配置的 SSH alias。</Notice>}
    {error && <div role="alert"><Notice tone="error">{error}</Notice></div>}
    <div className="host-row"><div><strong>本机</strong><small>当前 Mac 的环境与本地记录</small></div><button disabled={!!busy} onClick={() => onSelect(null)}>{current === null ? '当前工作空间' : '切换到本机'}</button></div>
    {inventory.hosts.map(host => <div className="host-row" key={host.alias}><div><strong>{host.alias}</strong><small>已登记的 SSH 引用 · 连接状态待检查</small></div><button disabled={!!busy} onClick={() => void run('connect', async () => { await remoteRequest({op:'connect',alias:host.alias}); onSelect(host.alias); })}>{current === host.alias ? '重新连接' : '连接并管理'}</button></div>)}
    <form className="remote-add" onSubmit={event => { event.preventDefault(); void run('add', async () => { await remoteRequest({op:'add_host',alias:alias.trim()}); setAlias(''); await refresh(); }); }}><label className="field">SSH Host alias<input disabled={!native} list="ssh-aliases" value={alias} onChange={event => setAlias(event.target.value)} placeholder="例如：writing-server" spellCheck={false}/><datalist id="ssh-aliases">{aliases.map(value => <option key={value} value={value}/>)}</datalist><small>使用 ~/.ssh/config 已有别名；不在这里收集密码、私钥或 host key。</small></label><button disabled={!native || !!busy || !alias.trim()}>登记主机</button></form><p className="small-print">{coverage || '登记仅保存连接引用。连接会核验已有 host key，未知主机请先用系统 SSH 建立可信记录。'}</p>
    <Notice>远端需要已有 Lintel runner。桌面不会自动安装、更新软件或修改 SSH 配置。远端命令和请求范围固定，不接受任意 shell 命令。</Notice>
    <section className="separated"><div className="browser-panel-heading"><h3>持久提交记录</h3><button disabled={!native || !!busy} onClick={() => void run('refresh', refresh)}><Icon name="refresh" size={14}/>刷新</button></div>{!inventory.tasks.length ? <p className="small-print">还没有从桌面提交的远程任务。连接中断后，可在这里查询原任务。</p> : inventory.tasks.map(task => <div className="remote-task" key={`${task.alias}-${task.plan_id}`}><div><strong>{task.alias}</strong><code>{task.plan_id}</code></div><Status value={task.status}/><button disabled={!!busy} onClick={() => void run('query', async () => { setReceipt(await remoteRequest<Receipt>({op:'reconnect',alias:task.alias,plan_id:task.plan_id})); await refresh(); })}>查询原任务</button></div>)}</section>
    {receipt && <section className="remote-receipt"><h3>{receipt.title}</h3><Status value={receipt.status}/><div className="steps">{receipt.steps.map(step => <div className="step" key={step.id}><div><strong>{step.label}</strong><p>{step.message}</p></div><Status value={step.status}/></div>)}</div><Notice>查询只取回原结果。完整恢复与归档入口位于这台主机的“记录与恢复”。</Notice></section>}
  </div>;
}
