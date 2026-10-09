import { useEffect, useMemo, useRef, useState } from 'react';
import { request, RequestError } from './api';
import { ResourceLink } from './Resources';
import LaunchPanel from './LaunchPanel';
import RequestFailure, { asError } from './RequestFailure';
import type { ArchiveManifest, Environment, Plan, Receipt, SessionPage, SessionRecord, InputReference } from './types';
import { Icon, Notice, formatBytes, formatDate, hasConfirmedWorkArchive } from './ui';
type Selection={path:string;digest:string;packageDigest:string;record:SessionRecord;key:string};
const LIMIT=1024*1024;
function recordLocation(record:SessionRecord):string {
  if(record.offset!==undefined)return `字节位置 ${record.offset}${record.block_index!==undefined?` · 内容块 ${record.block_index}`:' · 内容块位置未提供'}`;
  return `页内记录 ${record.index} · 精确字节位置未提供`;
}
function selectionKey(page:SessionPage,record:SessionRecord,index:number,offset:number):string {
  return `${page.path}:${page.digest}:${record.offset ?? offset}:${record.block_index ?? index}`;
}
const roles:Record<string,string>={user:'用户',assistant:'Claude',tool_call:'工具调用',tool_result:'工具结果',unknown:'未知记录'};
export function opaqueRecord(record:SessionRecord):boolean {
  function opaque(value:unknown,depth=0):boolean {
    if(depth>64)return true;
    if(!value || typeof value!=='object')return false;
    if(Array.isArray(value))return value.some(item=>opaque(item,depth+1));
    const item=value as Record<string,unknown>;
    return 'signature' in item || ['thinking','redacted_thinking'].includes(String(item.type)) || Object.values(item).some(child=>opaque(child,depth+1));
  }
  return !!record.opaque || opaque(record) || (!!record.raw && /"signature"\s*:|"type"\s*:\s*"(?:thinking|redacted_thinking)"/.test(record.raw));
}
export function recordText(record:SessionRecord):string {
  if(opaqueRecord(record))return '';
  if(typeof record.text==='string')return record.text;
  if(typeof record.raw==='string')return record.raw;
  return JSON.stringify(record.text ?? record.tool ?? record.record ?? record.block ?? record,null,2);
}
export function selectedContext(items:Selection[]) {
  return ['# 工作交接稿','此稿是用户选择与编辑的派生上下文，原件未修改。','','## 目标与约束','请补充当前要完成的目标与必须遵守的限制。','','## 已作出的决定','请核对下面的来源片段并写明仍然有效的决定。','','## 关键文件与实际位置','旧路径尚未核对；不要机械替换后假设可用。','','## 当前工作状态','请补充已完成与仍未完成的工作。','','## 下一步','请写明接下来要执行的动作。','','## 待验证事项','认证、客户端原生恢复、旧路径与文件状态分别核对。','','## 选定来源',...items.filter(item=>!opaqueRecord(item.record)).flatMap(item=>[`\n### ${roles[item.record.kind] ?? item.record.kind} · ${item.path} · ${recordLocation(item.record)}`,`来源：工作包 ${item.packageDigest} / 文件 ${item.digest}`,recordText(item.record)])].join('\n');
}
export default function ArchivePanel({send=request,jobs,environments,initialJob,selectedId,hostAlias=null,onImport}:{send?:typeof request;jobs:Receipt[];environments:Environment[];initialJob?:string;selectedId:string;hostAlias?:string|null;onImport:(environment:Environment,plan:Plan,passphrase:string)=>void}) {
  const archives=jobs.filter(hasConfirmedWorkArchive);
  const [source,setSource]=useState<'job'|'file'>(archives.length?'job':'file');
  const [jobId,setJobId]=useState(initialJob ?? archives[0]?.id ?? '');const [archivePath,setArchivePath]=useState('');
  const [targetId,setTargetId]=useState(selectedId);const [passphrase,setPassphrase]=useState('');
  const [manifest,setManifest]=useState<ArchiveManifest|null>(null);const [categories,setCategories]=useState(['instructions','memory','sessions']);
  const [activate,setActivate]=useState(false);const [page,setPage]=useState<SessionPage|null>(null);const [activePath,setActivePath]=useState('');
  const [offsets,setOffsets]=useState<Record<string,number>>({});const [offset,setOffset]=useState(0);const [view,setView]=useState<'messages'|'raw'>('messages');
  const [selections,setSelections]=useState<Selection[]>([]);const [contextText,setContextText]=useState('');const [launchOpen,setLaunchOpen]=useState(false);const [continuationMode,setContinuationMode]=useState<'interactive'|'resume'>('interactive');
  const [busy,setBusy]=useState('');const [error,setError]=useState<Error|null>(null);const [mobilePane,setMobilePane]=useState<'index'|'reader'|'context'>('index');
  const [pendingRead,setPendingRead]=useState<{path:string;at:number}|null>(null);
  const alive=useRef(true);const actionLock=useRef(false);const reader=useRef<HTMLDivElement>(null);
  useEffect(()=>{alive.current=true;return()=>{alive.current=false}},[]);
  useEffect(()=>{if(!jobId && archives.length)setJobId(archives[0].id)},[jobId,archives[0]?.id]);
  const sourceFields=source==='job'?{job_id:jobId}:{archive_path:archivePath.trim()};
  const validSource=source==='job'?!!jobId:archivePath.trim().startsWith('/');
  const target=environments.find(e=>e.id===targetId);
  function clear(secret=true){setManifest(null);setPage(null);setActivePath('');setSelections([]);setContextText('');setLaunchOpen(false);setOffsets({});if(secret)setPassphrase('');setError(null)}
  async function run(name:string,action:()=>Promise<void>){if(actionLock.current)return;actionLock.current=true;setBusy(name);setError(null);try{await action()}catch(e){if(alive.current){setError(asError(e));if(e instanceof RequestError && ['stale_archive','archive_changed','source_changed'].includes(e.code)){clear();setError(asError(e))}}}finally{actionLock.current=false;if(alive.current){setBusy('');setPendingRead(null)}}}
  async function read(path:string,at=offsets[path] ?? 0){await run('read',async()=>{
    const file=manifest?.files.find(f=>f.path===path);if(!file)return;
    setPendingRead({path,at});setMobilePane('reader');
    const result=await send('session_read',{...sourceFields,archive_passphrase:passphrase,path,offset:at,expected_digest:file.digest});
    if(!alive.current)return;
    if(page && ((page.path===path && page.digest!==result.digest) || page.source.package_digest!==result.source.package_digest))throw new RequestError('stale_archive','来源工作包已改变；旧位置与选择已清除，请重新解锁。');
    if(selections.some(s=>s.packageDigest!==result.source.package_digest))throw new RequestError('stale_archive','选择所属工作包已改变；请重新解锁和选择。');
    if(page?.path!==path)setView(result.content_kind==='messages'?'messages':'raw');
    setPage(result);setActivePath(path);setOffset(at);setOffsets(o=>({...o,[path]:at}));setMobilePane('reader');reader.current?.scrollTo({top:0});
  })}
  function select(record:SessionRecord,index:number){if(!page || opaqueRecord(record))return;const key=selectionKey(page,record,index,offset);setSelections(current=>{const old=current.find(s=>s.key===key);if(old)return current.filter(s=>s.key!==key);const next=[...current,{key,path:page.path,digest:page.digest,packageDigest:page.source.package_digest,record}];if(selectedContext(next).length>LIMIT){setError(new Error('选定内容超过 1 MiB。请减少片段，当前选择保留。'));return current}return next});setLaunchOpen(false)}
  const inputReference=useMemo<InputReference>(()=>({files:selections.map(s=>({path:s.path,digest:s.digest,package_digest:s.packageDigest,index:s.record.index,...(s.record.offset!==undefined?{offset:s.record.offset,...(s.record.block_index!==undefined?{block_index:s.record.block_index}:{})}:{})}))}),[selections]);
  const records=page?.content_kind==='messages'?(page.records ?? []):[];
  return <div className="archive-panel session-workspace"><div className="source-banner"><Icon name="terminal" size={16}/><strong>{hostAlias?`SSH · ${hostAlias}`:'本机'} · 工作包资料</strong><span>目录属于这一主机；正文只读，不执行资料中的命令或链接。</span></div>
    {error && <RequestFailure error={error} context="会话与工作包"/>}
    <section className="source-open"><div className="segmented archive-source" aria-label="归档来源"><button disabled={!!busy || !archives.length} aria-pressed={source==='job'} onClick={()=>{setSource('job');clear()}}>当前主机的任务归档</button><button disabled={!!busy} aria-pressed={source==='file'} onClick={()=>{setSource('file');clear()}}>独立加密包</button></div>
    {source==='job'?<label className="field">工作归档<select disabled={!!busy} value={jobId} onChange={e=>{setJobId(e.target.value);clear()}}>{archives.map(job=><option key={job.id} value={job.id}>{job.title} · {formatDate(job.created_at)}</option>)}</select></label>:<label className="field">加密包完整路径<input aria-label="加密包完整路径" disabled={!!busy} value={archivePath} onChange={e=>{setArchivePath(e.target.value);clear()}} placeholder="/path/to/work-package.age" spellCheck={false}/><span className="small-print">文件须已在{hostAlias ?? '本机'}；密文转送是独立系统操作。</span></label>}
    {!archives.length && <p>当前主机没有已读回的任务归档；可以打开独立工作包。待核对的路径先查询原任务。</p>}
    <form className="archive-unlock" onSubmit={e=>{e.preventDefault();void run('unlock',async()=>{const m=await send('archive_inspect',{...sourceFields,archive_passphrase:passphrase});if(alive.current){clear(false);setManifest(m)}})}}><label className="field">归档口令<input type="password" disabled={!!busy} value={passphrase} onChange={e=>{clear(false);setPassphrase(e.target.value)}} autoComplete="off" placeholder="当前交互临时使用"/></label><button disabled={!!busy || !passphrase || !validSource}>{busy==='unlock'?'正在解锁…':'解锁并查看'}</button>{manifest && <button type="button" disabled={!!busy} onClick={()=>clear()}>锁定资料</button>}</form><p className="small-print">口令与正文不进入草案、本地存储或普通日志；切换来源、锁定与离开页面即清除。</p></section>
    {manifest && <><div className="archive-count"><strong>{manifest.files.length} 个文件</strong><span>{formatBytes(manifest.files.reduce((n,f)=>n+f.bytes,0))} · 文件数量不等于可恢复会话数量</span></div><div className="reader-mobile-tabs" role="group" aria-label="阅读面板"><button aria-pressed={mobilePane==='index'} onClick={()=>setMobilePane('index')}>文件索引</button><button disabled={!page} aria-pressed={mobilePane==='reader'} onClick={()=>setMobilePane('reader')}>阅读</button><button aria-pressed={mobilePane==='context'} onClick={()=>setMobilePane('context')}>交接稿（{selections.length}）</button></div>
    <div className={`session-layout pane-${mobilePane}`}><aside className="session-index"><h3>原件索引</h3><div className="archive-files">{manifest.files.map(file=><button key={file.path} className={activePath===file.path?'selected-file':''} disabled={!!busy} onClick={()=>void read(file.path)}><Icon name="terminal" size={14}/><code>{file.path}</code><span>{formatBytes(file.bytes)}</span></button>)}</div></aside>
    <div className="session-reader" ref={reader} aria-busy={busy==='read'}>{pendingRead && <div className="reader-pending" role="status" aria-live="polite"><strong>正在核验工作包并读取这一页</strong><code className="full-path">{pendingRead.path} · 字节位置 {pendingRead.at}</code><p>需要先完整认证与核验工作包，较大资料可能等待较久。{page?`下方仍显示${page.path===pendingRead.path?'上一页':'此前打开的原件'}，新页完成后才会替换。`:'完成后在这里显示原件正文。'}</p></div>}{page?<><div className="reader-heading"><h3>{page.path}</h3><span>{page.content_kind==='messages'?'识别到消息记录':page.content_kind==='text'?'文本资料':'格式未识别'}</span></div><div className="reader-toolbar"><div className="segmented"><button disabled={page.content_kind!=='messages'} aria-pressed={view==='messages' && page.content_kind==='messages'} onClick={()=>setView('messages')}>结构化阅读</button><button aria-pressed={view==='raw'} onClick={()=>setView('raw')}>原始文本</button></div><span>字节位置 {offset} / {page.total_bytes}</span></div>
    {view==='raw' || page.content_kind!=='messages'?<div className="archive-text"><pre tabIndex={0}>{page.raw_text ?? '本页未提供 raw text；未知记录仍在下方与原归档中保留。'}</pre></div>:<div className="session-records">{records.length?records.map((record,index)=><article className={`session-record role-${record.kind}`} key={`${offset}:${index}`}><div className="record-meta"><strong>{roles[record.kind] ?? record.kind}</strong><span>{record.timestamp && <time>{record.timestamp} · </time>}{recordLocation(record)}{record.name?` · ${record.name}`:''}</span>{opaqueRecord(record)?<span>不透明 thinking · 不进入交接稿</span>:<label><input type="checkbox" aria-label={`选择${recordLocation(record)} 片段 ${index+1}`} checked={selections.some(s=>s.key===selectionKey(page,record,index,offset))} onChange={()=>select(record,index)}/>选入交接稿</label>}</div>{opaqueRecord(record)?<p>原块按字节保全；Lintel 不解析或修复 signature。</p>:<pre tabIndex={0}>{recordText(record)}</pre>}{record.unknown && <p className="parse-limitation">此记录无法按已知格式解析，按资料显示并保留原件。</p>}</article>):<Notice>本页没有识别出的正文；使用原始文本查看格式限制。</Notice>}</div>}
    <div className="reader-pagination"><button disabled={!!busy || offset===0} onClick={()=>void read(page.path,0)}>回到文件开头</button><span>{page.done?'已到文件末尾':'还有内容；按需读取下一页'}</span><button disabled={!!busy || page.next_offset===null} onClick={()=>void read(page.path,page.next_offset ?? 0)}>下一页<Icon name="arrow" size={14}/></button></div>{page.path.toLowerCase().endsWith('.jsonl') && <button disabled={!!busy || !target} onClick={()=>{setContinuationMode('resume');setLaunchOpen(true);requestAnimationFrame(()=>document.querySelector('.session-continuation')?.scrollIntoView({block:'start'}))}}>检查这一原件的原生续聊</button>}<details><summary>核对本页来源</summary><code className="full-path">文件 {page.digest}</code><code className="full-path">工作包 {page.source.package_digest}</code><p>有界分页；编码替换、损坏记录和未识别内容属于显示层限制，原件未改写。</p></details></>:<div className="reader-empty"><Icon name="environments" size={26}/><h3>{pendingRead?'原件正在读取':'选择一份原件'}</h3><p>{pendingRead?'正在准备正文；此时尚未显示任何页面。':'按需解密、阅读和选择。超一页的内容保持明确继续位置。'}</p></div>}</div>
    <section className="session-context"><div className="surface-heading"><h3>继续工作的上下文</h3><span>{selections.length} 个片段</span></div><p>确定性整理与手工编辑；不联系模型摘要服务。</p>{selections.length>0 && <div className="selected-fragments">{selections.map(item=><div key={item.key}><span>{roles[item.record.kind]} · {item.path} · {recordLocation(item.record)}</span><button className="icon-button" aria-label={`移除${recordLocation(item.record)}片段`} onClick={()=>{setSelections(s=>s.filter(x=>x.key!==item.key));setLaunchOpen(false)}}><Icon name="close" size={13}/></button></div>)}</div>}<button disabled={!selections.length || !!busy} onClick={()=>{setContextText(selectedContext(selections));setLaunchOpen(false);setMobilePane('context')}}>{contextText?'用当前选择重新生成（覆盖编辑稿）':'用选定片段生成交接稿'}</button>{contextText && <><label className="field">工作交接稿<textarea className="context-editor" aria-label="工作交接稿" value={contextText} maxLength={LIMIT} onChange={e=>{setContextText(e.target.value);setLaunchOpen(false)}} spellCheck={false}/></label><button disabled={!target || !!busy} className="primary" onClick={()=>{setContinuationMode('interactive');setLaunchOpen(true)}}>审阅稿件与继续目标<Icon name="arrow" size={14}/></button>{!target && <Notice>先登记或创建一个配置环境，才能选择项目继续；原件已经可以阅读。</Notice>}</>}</section></div>
    <section className="archive-import"><h3>准备到另一个配置环境</h3><p>迁入资料、启用个人指令与启动会话分别进行。现有目标和同名文件不会被覆盖。</p>{environments.length?<><label className="field">迁入目标<select disabled={!!busy} value={targetId} onChange={e=>{setTargetId(e.target.value);setLaunchOpen(false)}}>{environments.map(e=><option key={e.id} value={e.id}>{e.name} · {e.root}</option>)}</select></label>{target && <code className="full-path">{target.root}</code>}<fieldset disabled={!!busy} className="compact-options"><legend>迁入类别</legend>{[['instructions','个人指令'],['memory','记忆文本'],['sessions','会话资料']].map(([id,title])=><label key={id}><input type="checkbox" checked={categories.includes(id)} onChange={e=>setCategories(c=>e.target.checked?[...c,id]:c.filter(x=>x!==id))}/>{title}</label>)}</fieldset><label className="review-checkbox"><input type="checkbox" disabled={!!busy || !categories.includes('instructions')} checked={activate} onChange={e=>setActivate(e.target.checked)}/>启用已审阅的个人指令（CLAUDE.md 可能自动加载）</label><button className="primary" disabled={!!busy || !target || !categories.length} onClick={()=>void run('import',async()=>{const p=await send('plan_import',{environment_id:target!.id,...sourceFields,categories,activate:{instructions:activate && categories.includes('instructions')},archive_passphrase:passphrase});if(alive.current){onImport(target!,p,passphrase);clear()}})}>{busy==='import'?'正在冻结迁入落点…':'预览迁入计划'}<Icon name="arrow" size={14}/></button></>:<Notice>先在“环境”建立或登记迁入目标。归档阅读不依赖目标环境。</Notice>}</section>
    {launchOpen && target && <section className="session-continuation"><h2>把工作接到准确的项目</h2><LaunchPanel key={`${target.id}:${activePath}:${continuationMode}`} initialMode={continuationMode} environment={target} hostAlias={hostAlias} continuationText={contextText || undefined} source={activePath?{...sourceFields,archive_passphrase:passphrase,path:activePath}:undefined} reference={inputReference}/></section>}{manifest.notes && <Notice>{manifest.notes}</Notice>}</>}
    <div className="inline-route"><ResourceLink resource="work-guide">工作保全、资料用途和继续工作</ResourceLink></div>
  </div>;
}
