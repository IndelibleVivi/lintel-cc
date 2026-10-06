import {useEffect,useMemo,useRef,useState} from 'react';
import {request} from './api';
import type {Environment,WorkFile} from './types';
import {formatBytes,Icon,Notice} from './ui';
import RequestFailure,{asError} from './RequestFailure';

const categoriesAll=['instructions','memory','sessions'];
const pageSize=50;
const fileLimit=8*1024*1024;
type Group={key:string;label:string;files:WorkFile[]};

// These are path groups, never inferred project cwd or parsed conversation titles.
function project(file:WorkFile):[string,string] {
  const parts=file.path.split('/');
  const start=parts[0]==='projects'?0:parts[0]==='lintel-imports'&&parts[1]==='projects'?1:-1;
  if(start>=0&&parts.length>start+2){const key=parts.slice(0,start+2).join('/');return [key,`${start===1?'待用区项目':'项目'} ${parts[start+1]}`];}
  if(start>=0)return [parts.slice(0,start+1).join('/'),start===1?'待用区 projects 直属资料':'projects 直属资料'];
  return ['instructions','个人指令'];
}
function family(file:WorkFile):[string,string] {
  if(file.category!=='sessions')return [file.category,file.category==='memory'?'记忆文件':'指令文件'];
  const [prefix]=project(file),tail=file.path.slice(prefix.length+1).split('/');
  if(tail.length===1)return [file.path.replace(/\.jsonl$/,''),`会话 ${tail[0]}`];
  // The standard <session>/subagents/*.jsonl layout shares its parent session's
  // group. Other layouts remain exact independent path groups.
  if(tail[1]==='subagents')return [`${prefix}/${tail[0]}`,`会话 ${tail[0]}.jsonl`];
  return [file.path,`会话 ${tail.join('/')}`];
}
function groups(files:WorkFile[],keyFor:(file:WorkFile)=>[string,string]):Group[] {
  const result=new Map<string,Group>();
  for(const file of files){const [key,label]=keyFor(file);let group=result.get(key);if(!group){group={key,label,files:[]};result.set(key,group);}group.files.push(file);}
  return [...result.values()];
}
function SelectionCheck({label,files,selected,disabled,onToggle}:{label:string;files:WorkFile[];selected:Set<string>;disabled:boolean;onToggle:(files:WorkFile[],checked:boolean)=>void}) {
  const ref=useRef<HTMLInputElement>(null),count=files.filter(file=>selected.has(file.path)).length;
  useEffect(()=>{if(ref.current)ref.current.indeterminate=count>0&&count<files.length;},[count,files.length]);
  return <input ref={ref} type="checkbox" aria-label={label} checked={count===files.length&&files.length>0} disabled={disabled} onChange={e=>onToggle(files,e.target.checked)}/>;
}
function PageControls({page,total,onPage}:{page:number;total:number;onPage:(page:number)=>void}) {
  if(total<=pageSize)return null;
  return <div className="work-list-pages"><button disabled={!page} onClick={()=>onPage(page-1)}>上一页</button><span>{page+1} / {Math.ceil(total/pageSize)}</span><button disabled={(page+1)*pageSize>=total} onClick={()=>onPage(page+1)}>下一页</button></div>;
}
function FileFamily({group,selected,disabled,onToggle}:{group:Group;selected:Set<string>;disabled:boolean;onToggle:(files:WorkFile[],checked:boolean)=>void}) {
  const [page,setPage]=useState(0),count=group.files.filter(file=>selected.has(file.path)).length;
  const currentPage=Math.min(page,Math.max(0,Math.ceil(group.files.length/pageSize)-1));
  return <details className="work-file-family" open={group.files.length<=3}>
    <summary><span>{group.label}</span><small>{count} / {group.files.length} 已选</small></summary>
    {group.files.length>1&&<label className="work-group-choice"><SelectionCheck label={`选择${group.label}全部文件`} files={group.files} selected={selected} disabled={disabled} onToggle={onToggle}/><span>选择这一组原件（包括本组子会话文件）</span></label>}
    <ul className="work-file-list">{group.files.slice(currentPage*pageSize,(currentPage+1)*pageSize).map(file=><li key={file.path}><label><input type="checkbox" aria-label={`保留 ${file.path}`} checked={selected.has(file.path)} disabled={disabled} onChange={e=>onToggle([file],e.target.checked)}/><span><code>{file.path}</code><small>{formatBytes(file.bytes)}{file.bytes>fileLimit&&<em> · 超过单文件 8 MiB 上限</em>}</small></span></label></li>)}</ul>
    <PageControls page={currentPage} total={group.files.length} onPage={setPage}/>
  </details>;
}
function ProjectGroup({group,selected,disabled,onToggle}:{group:Group;selected:Set<string>;disabled:boolean;onToggle:(files:WorkFile[],checked:boolean)=>void}) {
  const [page,setPage]=useState(0),families=useMemo(()=>groups(group.files,family),[group.files]);
  const count=group.files.filter(file=>selected.has(file.path)).length,currentPage=Math.min(page,Math.max(0,Math.ceil(families.length/pageSize)-1));
  return <details className="work-project-group" open>
    <summary><span>{group.label}</span><small>{count} / {group.files.length} 已选 · {formatBytes(group.files.reduce((sum,file)=>sum+file.bytes,0))}</small></summary>
    <label className="work-group-choice"><SelectionCheck label={`选择${group.label}全部文件`} files={group.files} selected={selected} disabled={disabled} onToggle={onToggle}/><span>选择本组全部原件</span></label>
    {families.slice(currentPage*pageSize,(currentPage+1)*pageSize).map(group=><FileFamily key={group.key} group={group} selected={selected} disabled={disabled} onToggle={onToggle}/>)}
    <PageControls page={currentPage} total={families.length} onPage={setPage}/>
  </details>;
}

export default function WorkFileSelection({send=request,environment,categories,paths,disabled,onChange,onReady}:{send?:typeof request;environment:Environment;categories:string[];paths:string[]|null;disabled:boolean;onChange:(paths:string[]|null)=>void;onReady:(ready:boolean)=>void}) {
  const [files,setFiles]=useState<WorkFile[]|null>(null),[error,setError]=useState<Error|null>(null),[busy,setBusy]=useState(false),[refresh,setRefresh]=useState(0),[query,setQuery]=useState(''),[page,setPage]=useState(0);
  const [incomplete,setIncomplete]=useState<string|null>(null);
  const initialize=useRef(false),pathMode=paths!==null,pathsRef=useRef(paths),categoriesRef=useRef(categories);
  pathsRef.current=paths;categoriesRef.current=categories;
  useEffect(()=>{
    let current=true;setFiles(null);setError(null);setIncomplete(null);onReady(!pathMode);
    if(!pathMode){setBusy(false);return;}
    setBusy(true);
    void (async()=>{
      let offset=0,digest:string|undefined,total:number|undefined;
      const all:WorkFile[]=[],seen=new Set<string>();
      for(;;){
        const report=await send('work_inventory',{environment_id:environment.id,categories:categoriesAll,offset,...(digest?{expected_digest:digest}:{})});
        if(!current)return;
        if(report.environment_id!==environment.id||report.root!==environment.root)throw new Error('文件清单与当前目标不匹配，请刷新。');
        if(digest&&report.digest!==digest)throw new Error('分页期间文件清单改变，请刷新后重新核对。');
        if(total!==undefined&&report.total_files!==total)throw new Error('分页文件总数改变，请刷新。');
        total=report.total_files;digest=report.digest;
        for(const file of report.files){if(seen.has(file.path))throw new Error('分页重复了原件路径，请刷新。');seen.add(file.path);all.push(file);}
        if(all.length>50000)throw new Error('文件清单超过扫描预算；没有使用部分清单。');
        if(!report.complete){setIncomplete(`清单未完整覆盖（${report.reason??'unknown'}${report.unobserved?` · ${report.unobserved}`:''}）。请先核对该入口，再刷新；当前清单不能用于精确保全。`);break;}
        if(report.next_offset===null){if(all.length!==total)throw new Error('文件清单缺页；没有使用部分清单。');break;}
        if(report.next_offset!==all.length||report.next_offset<=offset)throw new Error('文件清单分页无进展，请刷新。');
        offset=report.next_offset;
      }
      if(!current)return;
      setFiles(all);setPage(0);
      if(initialize.current){initialize.current=false;onChange(all.filter(file=>categoriesRef.current.includes(file.category)).map(file=>file.path));}
    })().catch(err=>{if(current)setError(asError(err));}).finally(()=>{if(current)setBusy(false);});
    return()=>{current=false;};
  },[send,environment.id,environment.root,pathMode,refresh]);
  const visible=useMemo(()=>files?.filter(file=>categories.includes(file.category))??[],[files,categories]);
  const selected=useMemo(()=>new Set(paths??[]),[paths]),known=useMemo(()=>new Set(files?.map(file=>file.path)),[files]);
  const missing=files&&!busy?(paths?.filter(path=>!known.has(path))??[]):[];
  // Turning off a category explicitly removes its detailed choices as well.
  useEffect(()=>{if(files&&paths){const allowed=new Set(visible.map(file=>file.path));const next=paths.filter(path=>!known.has(path)||allowed.has(path));if(next.length!==paths.length)onChange(next);}},[files,visible,paths,known,onChange]);
  const ready=!pathMode||!!files&&!busy&&!error&&!incomplete&&!missing.length;
  useEffect(()=>onReady(ready),[ready,onReady]);
  const shown=visible.filter(file=>!query||file.path.toLocaleLowerCase().includes(query.toLocaleLowerCase()));
  const projectGroups=groups(shown,project),currentPage=Math.min(page,Math.max(0,Math.ceil(projectGroups.length/pageSize)-1));
  function toggle(entries:WorkFile[],checked:boolean){const next=new Set(pathsRef.current??[]);for(const file of entries)checked?next.add(file.path):next.delete(file.path);onChange([...next].sort());}
  return <section className="surface work-file-selection" aria-label="精确原件选择">
    <div className="surface-heading"><h2>精确到原件</h2>{pathMode&&<button className="text-button" disabled={busy||disabled} onClick={()=>setRefresh(value=>value+1)}><Icon name="refresh" size={14}/>刷新文件清单</button>}</div>
    <div className="padded">
      {!pathMode?<><p>整类保留会包含该类别的全部原件。遇到超限会话，也可以逐个项目、会话或文件决定。</p><button disabled={disabled||!categories.length} onClick={()=>{initialize.current=true;onChange([]);}}>按项目／会话／文件选择</button></>:<>
        <div className="work-selection-tools"><strong>{paths?.length??0} 个原件已选</strong><button disabled={disabled||busy} onClick={()=>onChange(null)}>改回整类保留</button></div>
        <p className="small-print">分组来自配置 root 内的原始路径；没有读取会话正文或推断项目工作目录。新出现的文件不会自动加入。文件勾选只保留在本次页面中，批准范围由计划冻结。</p>
        {busy&&<p role="status">正在分页核对文件元数据…</p>}
        {error&&<><RequestFailure error={error} context="工作文件清单"/><p className="small-print">精确选择尚不可用；旧 runner 需要更新后再选择文件。可以显式改回整类保留，仍需重新预览。</p></>}
        {incomplete&&<Notice tone="warning">{incomplete}</Notice>}
        {!!missing.length&&<Notice tone="warning">已选原件从清单中消失；请核对后移除失效选择再预览。{missing.map(path=><code className="missing-work-path" key={path}>{path}</code>)}<button disabled={disabled||busy} onClick={()=>onChange((paths??[]).filter(path=>known.has(path)))}>移除失效选择</button></Notice>}
        {files&&!error&&<>
          <div className="work-selection-tools"><button disabled={disabled||busy||!!incomplete} onClick={()=>toggle(visible,true)}>选择当前类别全部原件</button><button disabled={disabled||busy} onClick={()=>onChange([])}>清空文件选择</button><button disabled={disabled||busy||!visible.some(file=>selected.has(file.path)&&file.bytes>fileLimit)} onClick={()=>toggle(visible.filter(file=>file.bytes>fileLimit),false)}>排除超限文件</button></div>
          <label className="field">筛选原件路径<input type="search" value={query} onChange={event=>{setQuery(event.target.value);setPage(0);}} placeholder="项目、会话或文件路径"/></label>
          {query&&<p className="small-print">筛选只改变显示；下面的分组选框只作用于匹配的原件。</p>}
          {!shown.length&&<p className="muted">{visible.length?'没有匹配路径。':'当前类别没有可列出的原件。'}</p>}
          {projectGroups.slice(currentPage*pageSize,(currentPage+1)*pageSize).map(group=><ProjectGroup key={group.key+query} group={group} selected={selected} disabled={disabled||busy||!!incomplete} onToggle={toggle}/>)}
          <PageControls page={currentPage} total={projectGroups.length} onPage={setPage}/>
        </>}
      </>}
    </div>
  </section>;
}
