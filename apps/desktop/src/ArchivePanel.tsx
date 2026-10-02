import { useState } from 'react';
import { request } from './api';
import Clawd from './Clawd';
import type { ArchiveManifest, Environment, Plan, Receipt } from './types';
import { Icon, Notice, formatBytes, formatDate } from './ui';

export default function ArchivePanel({ send = request, jobs, environments, initialJob, selectedId, onImport }: { send?: typeof request; jobs: Receipt[]; environments: Environment[]; initialJob?: string; selectedId: string; onImport: (environment: Environment, plan: Plan, passphrase: string) => void }) {
  const archives = jobs.filter(job => job.archive_path);
  const [jobId, setJobId] = useState(initialJob ?? archives[0]?.id ?? '');
  const [targetId, setTargetId] = useState(selectedId);
  const [passphrase, setPassphrase] = useState('');
  const [manifest, setManifest] = useState<ArchiveManifest | null>(null);
  const [categories, setCategories] = useState(['instructions', 'memory', 'sessions']);
  const [preview, setPreview] = useState<{ path: string; text: string; bytes: number; truncated: boolean } | null>(null);
  const [busy, setBusy] = useState('');
  const [error, setError] = useState('');
  async function run(name: string, action: () => Promise<void>) { setBusy(name); setError(''); try { await action(); } catch (err) { setError(err instanceof Error ? err.message : String(err)); } finally { setBusy(''); } }
  return <div className="archive-panel"><div className="module-intro"><Clawd small mood="pack"/><div><h3>值得留下的，都好好收着</h3><p>解锁本工具生成的工作归档，阅读原始文本，或选择迁入另一环境。</p></div></div>
    {error && <div role="alert"><Notice tone="error">{error}</Notice></div>}
    {!archives.length ? <div className="empty-inline">还没有工作归档。完成一次“新环境重建”后，归档会出现在这里。</div> : <>
      <label className="field">工作归档<select disabled={!!busy} value={jobId} onChange={event => { setJobId(event.target.value); setManifest(null); setPreview(null); setPassphrase(''); }}>{archives.map(job => <option key={job.id} value={job.id}>{job.title} · {formatDate(job.created_at)}</option>)}</select></label>
      <form className="archive-unlock" onSubmit={event => { event.preventDefault(); void run('unlock', async () => { setManifest(await send('archive_inspect', { job_id: jobId, archive_passphrase: passphrase })); setPreview(null); }); }}><label className="field">归档口令<input disabled={!!busy} type="password" value={passphrase} onChange={event => { setPassphrase(event.target.value); setManifest(null); setPreview(null); }} autoComplete="off" placeholder="仅在此窗口临时使用"/></label><button disabled={!!busy || !passphrase || !jobId}>{busy === 'unlock' ? '正在解锁…' : '解锁并查看'}</button></form>
      <p className="small-print">口令不进入草案或本地存储；关闭面板后清除。解锁不会迁入或执行归档里的内容。</p>
      {manifest && <><div className="archive-count"><span>{manifest.files.length} 个文件</span><span>{formatBytes(manifest.files.reduce((total, file) => total + file.bytes, 0))}</span></div><div className="archive-files">{manifest.files.map(file => <button key={file.path} className={preview?.path === file.path ? 'selected-file' : ''} disabled={!!busy} onClick={() => void run('read', async () => setPreview(await send('archive_read', { job_id: jobId, archive_passphrase: passphrase, path: file.path })))}><Icon name="terminal" size={14}/><code>{file.path}</code><span>{formatBytes(file.bytes)}</span></button>)}</div>{preview && <div className="archive-text"><div className="surface-heading"><h4>{preview.path}</h4><span className="small-label">只读文本</span></div><pre tabIndex={0}>{preview.text}</pre>{preview.truncated && <Notice>仅展示前 1 MiB；完整原文件仍在加密归档内。</Notice>}</div>}{manifest.notes && <Notice>{manifest.notes}</Notice>}
        <section className="archive-import"><h3>选择性迁入</h3><p>已存在的同名文件不会被覆盖。记忆与会话进入待用区，不宣称恢复旧会话。</p><label className="field">迁入目标<select value={targetId} onChange={event => setTargetId(event.target.value)}>{environments.map(environment => <option key={environment.id} value={environment.id}>{environment.name}</option>)}</select></label><fieldset className="compact-options"><legend>迁入类别</legend>{[['instructions','个人指令'],['memory','记忆文本'],['sessions','会话资料']].map(([id,title]) => <label key={id}><input type="checkbox" checked={categories.includes(id)} onChange={event => setCategories(current => event.target.checked ? [...current,id] : current.filter(value => value !== id))}/>{title}</label>)}</fieldset><button className="primary" disabled={!!busy || !targetId || !categories.length} onClick={() => void run('import', async () => { const target = environments.find(environment => environment.id === targetId)!; const plan = await send('plan_import', { environment_id: target.id, job_id: jobId, categories, archive_passphrase: passphrase }); onImport(target, plan, passphrase); })}>{busy === 'import' ? '正在准备计划…' : '预览迁入计划'}<Icon name="arrow" size={14}/></button></section>
      </>}
    </>}
  </div>;
}
