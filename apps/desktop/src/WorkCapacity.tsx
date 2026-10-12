import { LocalizedError } from './i18n';
import {useEffect,useState} from 'react';
import {request} from './api';
import type {Environment,WorkPreflight} from './types';
import {formatBytes,Icon,Notice} from './ui';
import RequestFailure,{asError} from './RequestFailure';
import {useI18n} from './i18n-provider';

export default function WorkCapacity({send=request,environment,categories,selectedPaths=null,selectionReady=true,onResult}:{send?:typeof request;environment:Environment;categories:string[];selectedPaths?:string[]|null;selectionReady?:boolean;onResult:(result:WorkPreflight|null)=>void}) {
  const { t, formatDateTime, formatNumber } = useI18n();
  const [result,setResult]=useState<WorkPreflight|null>(null),[error,setError]=useState<Error|null>(null),[busy,setBusy]=useState(false),[refresh,setRefresh]=useState(0);
  const selection=JSON.stringify([categories,selectedPaths]);
  useEffect(()=>{
    let current=true;setResult(null);onResult(null);setError(null);
    if(!categories.length||!selectionReady||selectedPaths?.length===0){setBusy(false);return;}
    setBusy(true);
    void send('work_preflight',{environment_id:environment.id,categories,...(selectedPaths!==null?{selected_paths:selectedPaths}:{})}).then(report=>{
      if(report.environment_id!==environment.id||report.root!==environment.root)throw new LocalizedError('capacity.targetMismatch');
      if(current){setResult(report);onResult(report);}
    }).catch(err=>{if(current)setError(asError(err));}).finally(()=>{if(current)setBusy(false);});
    return()=>{current=false;};
  },[send,environment.id,environment.root,selection,selectionReady,refresh]);
  return <section className="surface capacity-preflight" aria-label={t('capacity.aria')}>
    <div className="surface-heading"><h2>{t('capacity.title')}</h2><button className="text-button" disabled={busy||!categories.length||!selectionReady||selectedPaths?.length===0} onClick={()=>setRefresh(v=>v+1)}><Icon name="refresh" size={14}/>{t('capacity.refresh')}</button></div>
    <div className="padded">
      {busy&&<p role="status">{t('capacity.checking')}</p>}
      {!categories.length&&<p className="muted">{t('capacity.chooseCategory')}</p>}
      {!!categories.length&&(!selectionReady||selectedPaths?.length===0)&&<p className="muted">{t('capacity.needSelection')}</p>}
      {error&&<><RequestFailure error={error} context={t('capacity.context')}/><p className="small-print">{t('capacity.noResult')}</p></>}
      {result&&<>
        <div className="fact-row"><span>{t('capacity.scanned')}</span><strong>{t('capacity.filesBytes',{files:result.totals.files,bytes:formatBytes(result.totals.bytes)})}</strong></div>
        <p className="small-print">{t('capacity.limits',{file:formatBytes(result.limits.file_bytes),total:formatBytes(result.limits.total_bytes),files:formatNumber(result.limits.files)})}</p>
        <Notice tone={result.eligible?'neutral':'warning'}>{!result.complete?t('capacity.incomplete'):result.eligible?t('capacity.eligible'):t('capacity.exceeds')}</Notice>
        {!!result.blockers.length&&<ul className="capacity-blockers">{result.blockers.map((blocker,i)=><li key={i}>{blocker.path&&<code>{blocker.path}</code>}<span>{blocker.message}</span><small>{blocker.code}</small></li>)}</ul>}
        {result.blockers_truncated&&<p className="small-print">{t('capacity.truncated')}</p>}
        <p className="small-print">{t('capacity.footer',{time:formatDateTime(result.checked_at)})}</p>
      </>}
    </div>
  </section>;
}
