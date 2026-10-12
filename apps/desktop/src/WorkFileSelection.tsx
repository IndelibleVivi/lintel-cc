import { LocalizedError } from './i18n';
import {useEffect,useMemo,useRef,useState} from 'react';
import {request} from './api';
import type {Environment,WorkFile} from './types';
import {formatBytes,Icon,Notice} from './ui';
import RequestFailure,{asError} from './RequestFailure';
import {useI18n} from './i18n-provider';

const categoriesAll=['instructions','memory','sessions'];
const pageSize=50;
type Group={key:string;label:string;files:WorkFile[]};

// These are path groups, never inferred project cwd or parsed conversation titles.
type TFn=(k:string,a?:Record<string,string|number>)=>string;
function project(file:WorkFile,t:TFn):[string,string] {
  const parts=file.path.split('/');
  const start=parts[0]==='projects'?0:parts[0]==='lintel-imports'&&parts[1]==='projects'?1:-1;
  if(start>=0&&parts.length>start+2){const key=parts.slice(0,start+2).join('/');return [key,`${start===1?t('workFiles.referenceProject'):t('workFiles.project')} ${parts[start+1]}`];}
  if(start>=0)return [parts.slice(0,start+1).join('/'),start===1?t('workFiles.referenceProjectsDirect'):t('workFiles.projectsDirect')];
  return ['instructions',t('category.instructions')];
}
function family(file:WorkFile,t:TFn):[string,string] {
  if(file.category!=='sessions')return [file.category,file.category==='memory'?t('workFiles.memoryFile'):t('workFiles.instructionFile')];
  const [prefix]=project(file,t),tail=file.path.slice(prefix.length+1).split('/');
  if(tail.length===1)return [file.path.replace(/\.jsonl$/,''),t('workFiles.session',{name:tail[0]})];
  // The standard <session>/subagents/*.jsonl layout shares its parent session's
  // group. Other layouts remain exact independent path groups.
  if(tail[1]==='subagents')return [`${prefix}/${tail[0]}`,t('workFiles.sessionFile',{name:`${tail[0]}.jsonl`})];
  return [file.path,t('workFiles.session',{name:tail.join('/')})];
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
  const { t } = useI18n();
  if(total<=pageSize)return null;
  return <div className="work-list-pages"><button disabled={!page} onClick={()=>onPage(page-1)}>{t('workFiles.prev')}</button><span>{page+1} / {Math.ceil(total/pageSize)}</span><button disabled={(page+1)*pageSize>=total} onClick={()=>onPage(page+1)}>{t('workFiles.next')}</button></div>;
}
function FileFamily({group,selected,disabled,onToggle,fileLimit}:{fileLimit?:number;group:Group;selected:Set<string>;disabled:boolean;onToggle:(files:WorkFile[],checked:boolean)=>void}) {
  const { t } = useI18n();
  const [page,setPage]=useState(0),count=group.files.filter(file=>selected.has(file.path)).length;
  const currentPage=Math.min(page,Math.max(0,Math.ceil(group.files.length/pageSize)-1));
  return <details className="work-file-family" open={group.files.length<=3}>
    <summary><span>{group.label}</span><small>{t('workFiles.selectedOf',{count,total:group.files.length})}</small></summary>
    {group.files.length>1&&<label className="work-group-choice"><SelectionCheck label={t('workFiles.selectGroup',{label:group.label})} files={group.files} selected={selected} disabled={disabled} onToggle={onToggle}/><span>{t('workFiles.selectGroupNote')}</span></label>}
    <ul className="work-file-list">{group.files.slice(currentPage*pageSize,(currentPage+1)*pageSize).map(file=><li key={file.path}><label><input type="checkbox" aria-label={t('workFiles.selectFile',{path:file.path})} checked={selected.has(file.path)} disabled={disabled} onChange={e=>onToggle([file],e.target.checked)}/><span><code>{file.path}</code><small>{formatBytes(file.bytes)}{fileLimit!==undefined&&file.bytes>fileLimit&&<em>{t('workFiles.oversize',{limit:formatBytes(fileLimit)})}</em>}</small></span></label></li>)}</ul>
    <PageControls page={currentPage} total={group.files.length} onPage={setPage}/>
  </details>;
}
function ProjectGroup({group,selected,disabled,onToggle,fileLimit}:{fileLimit?:number;group:Group;selected:Set<string>;disabled:boolean;onToggle:(files:WorkFile[],checked:boolean)=>void}) {
  const { t } = useI18n();
  const [page,setPage]=useState(0),families=useMemo(()=>groups(group.files,f=>family(f,t)),[group.files,t]);
  const count=group.files.filter(file=>selected.has(file.path)).length,currentPage=Math.min(page,Math.max(0,Math.ceil(families.length/pageSize)-1));
  return <details className="work-project-group" open>
    <summary><span>{group.label}</span><small>{t('workFiles.selectedOfBytes',{count,total:group.files.length,bytes:formatBytes(group.files.reduce((sum,file)=>sum+file.bytes,0))})}</small></summary>
    <label className="work-group-choice"><SelectionCheck label={t('workFiles.selectGroup',{label:group.label})} files={group.files} selected={selected} disabled={disabled} onToggle={onToggle}/><span>{t('workFiles.selectGroupAll')}</span></label>
    {families.slice(currentPage*pageSize,(currentPage+1)*pageSize).map(group=><FileFamily fileLimit={fileLimit} key={group.key} group={group} selected={selected} disabled={disabled} onToggle={onToggle}/>)}
    <PageControls page={currentPage} total={families.length} onPage={setPage}/>
  </details>;
}

export default function WorkFileSelection({send=request,environment,categories,paths,disabled,onChange,onReady,fileLimit}:{fileLimit?:number;send?:typeof request;environment:Environment;categories:string[];paths:string[]|null;disabled:boolean;onChange:(paths:string[]|null)=>void;onReady:(ready:boolean)=>void}) {
  const { t } = useI18n();
  const [files,setFiles]=useState<WorkFile[]|null>(null),[error,setError]=useState<Error|null>(null),[busy,setBusy]=useState(false),[refresh,setRefresh]=useState(0),[query,setQuery]=useState(''),[page,setPage]=useState(0);
  const [incomplete,setIncomplete]=useState<{reason:string;unobserved:string}|null>(null);
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
        if(report.environment_id!==environment.id||report.root!==environment.root)throw new LocalizedError('workFiles.listMismatch');
        if(digest&&report.digest!==digest)throw new LocalizedError('workFiles.digestChanged');
        if(total!==undefined&&report.total_files!==total)throw new LocalizedError('workFiles.totalChanged');
        total=report.total_files;digest=report.digest;
        for(const file of report.files){if(seen.has(file.path))throw new LocalizedError('workFiles.duplicatePath');seen.add(file.path);all.push(file);}
        if(all.length>50000)throw new LocalizedError('workFiles.budget');
        if(!report.complete){setIncomplete({reason:report.reason??'unknown',unobserved:report.unobserved?` · ${report.unobserved}`:''});break;}
        if(report.next_offset===null){if(all.length!==total)throw new LocalizedError('workFiles.missingPage');break;}
        if(report.next_offset!==all.length||report.next_offset<=offset)throw new LocalizedError('workFiles.noProgress');
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
  const projectGroups=groups(shown,f=>project(f,t)),currentPage=Math.min(page,Math.max(0,Math.ceil(projectGroups.length/pageSize)-1));
  function toggle(entries:WorkFile[],checked:boolean){const next=new Set(pathsRef.current??[]);for(const file of entries)checked?next.add(file.path):next.delete(file.path);onChange([...next].sort());}
  return <section className="surface work-file-selection" aria-label={t('workFiles.aria')}>
    <div className="surface-heading"><h2>{t('workFiles.title')}</h2>{pathMode&&<button className="text-button" disabled={busy||disabled} onClick={()=>setRefresh(value=>value+1)}><Icon name="refresh" size={14}/>{t('workFiles.refresh')}</button>}</div>
    <div className="padded">
      {!pathMode?<><p>{t('workFiles.categoryNote')}</p><button disabled={disabled||!categories.length} onClick={()=>{initialize.current=true;onChange([]);}}>{t('workFiles.chooseByPath')}</button></>:<>
        <div className="work-selection-tools"><strong>{t('workFiles.selectedCount',{count:paths?.length??0})}</strong><button disabled={disabled||busy} onClick={()=>onChange(null)}>{t('workFiles.backToCategory')}</button></div>
        <p className="small-print">{t('workFiles.groupingNote')}</p>
        {busy&&<p role="status">{t('workFiles.paging')}</p>}
        {error&&<><RequestFailure error={error} context={t('workFiles.context')}/><p className="small-print">{t('workFiles.unavailable')}</p></>}
        {incomplete&&<Notice tone="warning">{t('workFiles.incomplete',incomplete)}</Notice>}
        {!!missing.length&&<Notice tone="warning">{t('workFiles.missing')}{missing.map(path=><code className="missing-work-path" key={path}>{path}</code>)}<button disabled={disabled||busy} onClick={()=>onChange((paths??[]).filter(path=>known.has(path)))}>{t('workFiles.removeMissing')}</button></Notice>}
        {files&&!error&&<>
          <div className="work-selection-tools"><button disabled={disabled||busy||!!incomplete} onClick={()=>toggle(visible,true)}>{t('workFiles.selectAllCategory')}</button><button disabled={disabled||busy} onClick={()=>onChange([])}>{t('workFiles.clear')}</button><button disabled={disabled||busy||fileLimit===undefined||!visible.some(file=>selected.has(file.path)&&file.bytes>fileLimit)} onClick={()=>toggle(visible.filter(file=>fileLimit!==undefined&&file.bytes>fileLimit),false)}>{t('workFiles.excludeOversize')}</button></div>
          <label className="field">{t('workFiles.filter')}<input type="search" value={query} onChange={event=>{setQuery(event.target.value);setPage(0);}} placeholder={t('workFiles.filterPlaceholder')}/></label>
          {query&&<p className="small-print">{t('workFiles.filterNote')}</p>}
          {!shown.length&&<p className="muted">{t(visible.length?'workFiles.noMatch':'workFiles.none')}</p>}
          {projectGroups.slice(currentPage*pageSize,(currentPage+1)*pageSize).map(group=><ProjectGroup fileLimit={fileLimit} key={group.key+query} group={group} selected={selected} disabled={disabled||busy||!!incomplete} onToggle={toggle}/>)}
          <PageControls page={currentPage} total={projectGroups.length} onPage={setPage}/>
        </>}
      </>}
    </div>
  </section>;
}
