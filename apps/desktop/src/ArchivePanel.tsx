import { useEffect, useRef, useState } from 'react';
import { request } from './api';
import Clawd from './Clawd';
import { ResourceLink } from './Resources';
import type { ArchiveManifest, Environment, Plan, Receipt } from './types';
import { Icon, Notice, formatBytes, formatDate } from './ui';

export default function ArchivePanel({ send = request, jobs, environments, initialJob, selectedId, onImport }: { send?: typeof request; jobs: Receipt[]; environments: Environment[]; initialJob?: string; selectedId: string; onImport: (environment: Environment, plan: Plan, passphrase: string) => void }) {
  const archives = jobs.filter(job => job.archive_path);
  const [source, setSource] = useState<'job'|'file'>(archives.length ? 'job' : 'file');
  const [jobId, setJobId] = useState(initialJob ?? archives[0]?.id ?? '');
  const [archivePath, setArchivePath] = useState('');
  const [targetId, setTargetId] = useState(selectedId);
  const [passphrase, setPassphrase] = useState('');
  const [manifest, setManifest] = useState<ArchiveManifest | null>(null);
  const [categories, setCategories] = useState(['instructions', 'memory', 'sessions']);
  const [preview, setPreview] = useState<{ path: string; text: string; bytes: number; truncated: boolean } | null>(null);
  const [busy, setBusy] = useState('');
  const [error, setError] = useState('');
  const alive = useRef(true);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  const sourceFields = source === 'job' ? { job_id: jobId } : { archive_path: archivePath.trim() };
  const validSource = source === 'job' ? !!jobId : archivePath.trim().startsWith('/');
  function clear() { setManifest(null); setPreview(null); setPassphrase(''); setError(''); }
  async function run(name: string, action: () => Promise<void>) { setBusy(name); setError(''); try { await action(); } catch (err) { if (alive.current) setError(err instanceof Error ? err.message : String(err)); } finally { if (alive.current) setBusy(''); } }
  return <div className="archive-panel"><div className="module-intro"><Clawd small mood="pack"/><div><h3>值得留下的，都好好收着</h3><p>阅读本工具生成的加密工作包，或选择迁入另一环境。独立文件不需要原机器的任务记录。</p></div></div>
    {error && <div role="alert"><Notice tone="error">{error}</Notice></div>}
    <div className="segmented archive-source" aria-label="归档来源"><button disabled={!!busy || !archives.length} aria-pressed={source === 'job'} onClick={() => { setSource('job'); clear(); }}>当前主机的任务归档</button><button disabled={!!busy} aria-pressed={source === 'file'} onClick={() => { setSource('file'); clear(); }}>独立加密包</button></div>
    {source === 'job' ? <label className="field">工作归档<select disabled={!!busy} value={jobId} onChange={event => { setJobId(event.target.value); clear(); }}>{archives.map(job => <option key={job.id} value={job.id}>{job.title} · {formatDate(job.created_at)}</option>)}</select></label> : <label className="field">加密包完整路径<input aria-label="加密包完整路径" aria-describedby="archive-source-help" disabled={!!busy} value={archivePath} onChange={event => { setArchivePath(event.target.value); clear(); }} placeholder="/path/to/work-package.age" spellCheck={false}/><span id="archive-source-help" className="small-print">文件须已放在当前目标主机。切换 SSH 主机时使用那台机器上的路径；传送密文的步骤见迁移指南。</span></label>}
    {!archives.length && <p className="small-print">当前主机还没有任务归档。可从“工作保全”生成，或直接打开已有的独立加密包。</p>}
    <form className="archive-unlock" onSubmit={event => { event.preventDefault(); void run('unlock', async () => { const result = await send('archive_inspect', { ...sourceFields, archive_passphrase: passphrase }); if (alive.current) { setManifest(result); setPreview(null); } }); }}><label className="field">归档口令<input disabled={!!busy} type="password" value={passphrase} onChange={event => { setPassphrase(event.target.value); setManifest(null); setPreview(null); }} autoComplete="off" placeholder="仅在此窗口临时使用"/></label><button disabled={!!busy || !passphrase || !validSource}>{busy === 'unlock' ? '正在解锁…' : '解锁并查看'}</button></form>
    <p className="small-print">口令不进入草案或本地存储；关闭面板后清除。解锁只阅读资料。</p>
    {manifest && <><div className="archive-count"><span>{manifest.files.length} 个文件</span><span>{formatBytes(manifest.files.reduce((total, file) => total + file.bytes, 0))}</span></div><div className="archive-files">{manifest.files.map(file => <button key={file.path} className={preview?.path === file.path ? 'selected-file' : ''} disabled={!!busy} onClick={() => void run('read', async () => { const result = await send('archive_read', { ...sourceFields, archive_passphrase: passphrase, path: file.path }); if (alive.current) setPreview(result); })}><Icon name="terminal" size={14}/><code>{file.path}</code><span>{formatBytes(file.bytes)}</span></button>)}</div>{preview && <div className="archive-text"><div className="surface-heading"><h4>{preview.path}</h4><span className="small-label">只读文本</span></div><pre tabIndex={0}>{preview.text}</pre>{preview.truncated && <Notice>仅展示前 1 MiB；完整原文件仍在加密归档内。</Notice>}</div>}{manifest.notes && <Notice>{manifest.notes}</Notice>}
      <section className="archive-import"><h3>选择性迁入</h3><p>已存在的同名文件不会被覆盖。记忆与会话进入待用区；后续会话需正常登录和启动。</p>{environments.length ? <><label className="field">迁入目标<select disabled={!!busy} value={targetId} onChange={event => setTargetId(event.target.value)}>{environments.map(environment => <option key={environment.id} value={environment.id}>{environment.name}</option>)}</select></label><fieldset disabled={!!busy} className="compact-options"><legend>迁入类别</legend>{[['instructions','个人指令'],['memory','记忆文本'],['sessions','会话资料']].map(([id,title]) => <label key={id}><input type="checkbox" checked={categories.includes(id)} onChange={event => setCategories(current => event.target.checked ? [...current,id] : current.filter(value => value !== id))}/>{title}</label>)}</fieldset><button className="primary" disabled={!!busy || !targetId || !categories.length} onClick={() => void run('import', async () => { const target = environments.find(environment => environment.id === targetId)!; const plan = await send('plan_import', { environment_id: target.id, ...sourceFields, categories, archive_passphrase: passphrase }); if (alive.current) onImport(target, plan, passphrase); })}>{busy === 'import' ? '正在准备计划…' : '预览迁入计划'}<Icon name="arrow" size={14}/></button></> : <Notice>先关闭面板，在“环境”建立或登记迁入目标。工作包已经可以独立阅读。</Notice>}</section>
    </>}
    <div className="inline-route"><ResourceLink resource="work-guide">工作包、迁入与跨主机步骤</ResourceLink></div>
  </div>;
}
