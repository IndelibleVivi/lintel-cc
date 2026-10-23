import {useEffect,useRef,useState} from 'react';
import type {request} from './api';
import {RequestError} from './api';
import type {Api,Environment,JobSummary,JobsPage,JobDetailPage,Plan,Receipt} from './types';
import {present} from './i18n-presentation';
import {useI18n} from './i18n-provider';
import {Notice,Status} from './ui';
import RequestFailure,{asError} from './RequestFailure';

type Section=NonNullable<Api['job_detail_page']['request']['section']>;
const sectionLabels:Record<Section,string>={header: 'history.boundedSectionheader', receipt: 'history.boundedRawPage', steps: 'history.boundedSectionsteps', warnings: 'history.boundedSectionwarnings', coverage: 'history.boundedSectioncoverage', task_result: 'history.boundedSectiontask_result', result_coverage: 'history.boundedSectionresult_coverage', next_actions: 'history.boundedSectionnext_actions', next_steps: 'history.boundedSectionnext_steps', error: 'history.boundedSectionerror'};
export default function HistoryPanel({send,environments,selectedId,refreshKey,filter,onFilterChange:setFilter,onPlan,onArchive,onReceipt}:{send:typeof request;hostAlias:string|null;environments:Environment[];selectedId:string;refreshKey?:number;filter:'all'|'attention'|'target';onFilterChange:(value:'all'|'attention'|'target')=>void;onPlan:(environment:Environment|null,plan:Plan)=>void;onArchive:(id:string)=>void;onReceipt:(receipt:Receipt)=>void}) {
 const {t,locale,formatDateTime}=useI18n();
 const [rows,setRows]=useState<JobSummary[]>([]),[page,setPage]=useState<JobsPage|null>(null),[errors,setErrors]=useState<JobsPage['record_errors']>([]);
 const [query,setQuery]=useState('');
 const [pending,setPending]=useState(false),[error,setError]=useState<Error|null>(null);
 const [id,setId]=useState(''),[detail,setDetail]=useState<JobDetailPage|null>(null),[section,setSection]=useState<Section>('header'),[detailError,setDetailError]=useState<Error|null>(null),[detailPending,setDetailPending]=useState(false);
 const generation=useRef(0),detailGeneration=useRef(0);const alive=useRef(true);
 useEffect(()=>{alive.current=true;return()=>{alive.current=false;generation.current++;detailGeneration.current++}},[]);
 function failure(e:unknown){return e instanceof RequestError && ['unsupported_command','unknown_command','invalid_command'].includes(e.code)?new RequestError('bounded_history_unsupported',t('history.boundedUnsupported')):asError(e)}
 async function load(more=false){const token=++generation.current;setPending(true);setError(null);try{
  const result=await send('jobs_page',{query,filter:filter==='attention'?'attention':'all',...(filter==='target'&&selectedId?{environment_id:selectedId}:{}),...(more&&page?.next_offset!=null?{offset:page.next_offset,expected_digest:page.digest}:{})});
  if(!alive.current||token!==generation.current)return;
  setRows(previous=>more?[...previous,...result.jobs]:result.jobs);setErrors(previous=>more?[...previous,...result.record_errors]:result.record_errors);setPage(result);
 }catch(e){if(alive.current&&token===generation.current)setError(failure(e))}finally{if(alive.current&&token===generation.current)setPending(false)}}
 useEffect(()=>{setRows([]);setPage(null);setErrors([]);void load();return()=>{generation.current++}},[send,filter,selectedId,query,refreshKey]);
 async function read(jobId:string,part:Section,more=false){const token=++detailGeneration.current;setDetailPending(true);setDetailError(null);try{
  const result=await send('job_detail_page',{job_id:jobId,section:part,...(detail?.job_id===jobId?{expected_digest:detail.digest}:{}),...(more&&detail?.next_offset!=null?{offset:detail.next_offset,expected_digest:detail.digest}:{})});
  if(alive.current&&token===detailGeneration.current){setDetail(result);setId(jobId);setSection(part)}
  if(part==='header'){
    try{const complete=await send('job_detail_page',{job_id:jobId,section:'receipt',expected_digest:result.digest});if(alive.current&&token===detailGeneration.current&&complete.items.length===1)onReceipt(complete.items[0] as Receipt)}catch(e){if(!(e instanceof RequestError&&e.code==='detail_item_too_large'))throw e;}
  }
 }catch(e){if(alive.current&&token===detailGeneration.current)setDetailError(failure(e))}finally{if(alive.current&&token===detailGeneration.current)setDetailPending(false)}}
 const header=detail?.section==='header'?detail.items[0] as {environment_id?:string;restorable?:boolean;service_restorable?:boolean;network_restorable?:boolean;has_work_archive?:boolean}:undefined;
 async function restore(kind:'plan_restore'|'plan_service_resume'|'plan_network_restore'){setDetailPending(true);setDetailError(null);const token=detailGeneration.current;try{const plan=await send(kind,{job_id:detail!.job_id});if(alive.current&&token===detailGeneration.current)onPlan(environments.find(e=>e.id===header?.environment_id)??null,plan)}catch(e){if(alive.current)setDetailError(asError(e))}finally{if(alive.current)setDetailPending(false)}}
 return <section><p>{t('history.boundedOrder')}</p><div className="button-row"><button disabled={pending} onClick={()=>void load()}>{t('history.boundedRefresh')}</button><select aria-label={t('app.filterAll')} value={filter} onChange={e=>setFilter(e.target.value as typeof filter)}><option value="all">{t('app.filterAll')}</option><option value="attention">{t('app.filterAttention')}</option><option value="target" disabled={!selectedId}>{t('app.filterTarget')}</option></select></div>
 {pending&&<p role="status">{t('history.boundedLoading')}</p>}{error&&<RequestFailure error={error}/>}
 {page&&<p>{t(page.scan_complete?'history.boundedEnd':'history.boundedPartial')}</p>}
 {errors.length>0&&<Notice tone="warning"><p>{t('history.incompleteRecords')}</p>{errors.map((e,i)=><p key={i}><code>{e.job_id??t('history.invalidRecordId')}</code> · {e.code}</p>)}</Notice>}
 <label className="history-search">{t('app.searchRecordsSr')}<input type="search" maxLength={256} value={query} onChange={e=>setQuery(e.target.value)} placeholder={t('app.searchPlaceholder')}/></label><div className="job-list">{rows.map(row=><div className="job-row" key={row.id}><div className="job-copy"><strong>{present(row.title,locale)}{row.title_truncated?'…':''}</strong><span>{formatDateTime(new Date(Number(row.updated_at_ns)/1e6).toISOString())}</span><code>{row.id}</code></div><Status value={row.status}/><button disabled={detailPending} onClick={()=>{setDetail(null);void read(row.id,'header')}}>{t('app.viewResult')}</button></div>)}</div>
 {page?.next_offset!=null&&<button disabled={pending} onClick={()=>void load(true)}>{t('history.boundedMore')}</button>}
 <details className="query-task"><summary>{t('app.querySummary')}</summary><form onSubmit={e=>{e.preventDefault();setDetail(null);void read(id.trim(),'header')}}><label className="field">{t('app.originalTaskId')}<input value={id} onChange={e=>setId(e.target.value)} spellCheck={false}/></label><button disabled={!id.trim()||detailPending}>{t('app.queryOriginalTask')}</button></form><p className="small-print">{t('app.queryNote')}</p></details>
 {detailPending&&<p role="status">{t('history.boundedLoading')}</p>}{detailError&&<RequestFailure error={detailError}/>}
 {detail&&<section><h2>{t('app.viewResult')} · <code>{detail.job_id}</code></h2><p>{t('history.boundedDetail')}</p><div className="button-row">{(['header','steps','warnings','coverage','task_result','result_coverage','next_actions','next_steps','error'] as Section[]).map(part=><button aria-pressed={section===part} disabled={detailPending} key={part} onClick={()=>void read(detail.job_id,part)}>{t(sectionLabels[part])}</button>)}</div><div className="receipt-summary">{detail.items.map((item,index)=>{
 const value=item&&typeof item==='object'&&!Array.isArray(item)?item as Record<string,unknown>:null;
 const text=(key:string)=>typeof value?.[key]==='string'?present(value[key] as string,locale):'';
 return <div key={index}>{typeof item==='string'?<p>{present(item,locale)}</p>:value?<>
 {section==='header'?<><h3>{text('title')}</h3><Status value={text('status')}/><p><code>{text('id')}</code></p><p>{text('primary')}</p></>:<><strong>{text('label')||text('code')||text('scope')||text('key')||text('id')}</strong>{text('status')&&<Status value={text('status')}/>}<p>{text('message')||text('detail')||text('primary')||text('value')}</p></>}
 </>:null}</div>
 })}</div><details><summary>{t('history.boundedRawPage')}</summary><pre className="code-block">{JSON.stringify(detail.items,null,2)}</pre></details>{detail.next_offset!=null&&<button disabled={detailPending} onClick={()=>void read(detail.job_id,section,true)}>{t('history.boundedMore')}</button>}
 {header&&<div className="button-row">{header.has_work_archive&&<button onClick={()=>onArchive(detail.job_id)}>{t('app.viewWorkArchive')}</button>}{header.restorable&&<button disabled={detailPending} onClick={()=>void restore('plan_restore')}>{t('app.previewRestore')}</button>}{header.service_restorable&&<button disabled={detailPending} onClick={()=>void restore('plan_service_resume')}>{t('app.previewRestoreService')}</button>}{header.network_restorable&&<button disabled={detailPending} onClick={()=>void restore('plan_network_restore')}>{t('app.previewRestore')}</button>}</div>}</section>}
 </section>
}
