import { useEffect, useRef, useState } from 'react';
import { request } from './api';
import type { LaunchRecord } from './types';
import RequestFailure, { asError } from './RequestFailure';
import { Icon, Notice, Status } from './ui';
import { useI18n } from './i18n-provider';
import { present } from './i18n-presentation';

export default function LaunchRecords({send=request,hostAlias,onHandoff}:{send?:typeof request;hostAlias:string|null;onHandoff:(record:LaunchRecord)=>void}) {
  const { t, formatDateTime, locale } = useI18n();
  const [records,setRecords]=useState<LaunchRecord[]>([]);
  const [query,setQuery]=useState('');const [result,setResult]=useState<LaunchRecord|null>(null);
  const [busy,setBusy]=useState(false);const [error,setError]=useState<Error|null>(null);
  const alive=useRef(true);const lock=useRef(false);
  useEffect(()=>{alive.current=true;setRecords([]);setResult(null);setQuery('');
    void send('launches',{}).then(r=>{if(alive.current)setRecords(r.launches)}).catch(e=>{if(alive.current)setError(asError(e))});
    return()=>{alive.current=false};
  },[send]);
  async function lookup(id:string){if(lock.current)return;lock.current=true;setBusy(true);setError(null);try{const r=await send('launch_query',{request_id:id.trim()});if(alive.current){setResult(r);setQuery(id)}}catch(e){if(alive.current)setError(asError(e))}finally{lock.current=false;if(alive.current)setBusy(false)}}
  return <section className="launch-records surface"><div className="surface-heading"><h2>{t('launchRecords.title')}</h2><span className="small-label">{t('launchRecords.hostCount',{host:hostAlias?`SSH · ${hostAlias}`:t('app.localLabel'),count:records.length})}</span></div>
    <div className="launch-records-body">
    <p>{t('launchRecords.intro')}</p>
    {error && <RequestFailure error={error} context={t('launchRecords.context')}/>}
    <form className="launch-record-query" onSubmit={e=>{e.preventDefault();void lookup(query)}}><label className="field">{t('launchRecords.queryLabel')}<input aria-label={t('launchRecords.queryLabel')} value={query} onChange={e=>setQuery(e.target.value)} spellCheck={false} placeholder={t('launchRecords.queryPlaceholder')}/></label><button disabled={busy || !query.trim()}>{t('launchRecords.query')}<Icon name="refresh" size={14}/></button></form>
    {records.length>0 && <div className="launch-record-list">{records.map(record=><button key={record.request_id} disabled={busy} onClick={()=>void lookup(record.request_id)}><span><strong>{t(record.mode==='resume'?'launchRecords.resume':'launchRecords.newSession')}</strong><code>{record.project_cwd ?? t('launchRecords.projectMissing')}</code><small>{record.recorded_at?formatDateTime(record.recorded_at):t('launchRecords.timeMissing')} · {record.request_id}</small></span><Status value={record.status}/></button>)}</div>}
    {result && <div className="launch-record-detail" role="status"><Status value={result.status}/><dl><div><dt>{t('launchRecords.originalId')}</dt><dd><code>{result.request_id}</code></dd></div><div><dt>{t('launchRecords.configRoot')}</dt><dd><code>{result.root ?? result.config_root ?? t('launchRecords.legacyMissing')}</code></dd></div><div><dt>{t('launchRecords.projectCwd')}</dt><dd><code>{result.project_cwd ?? t('launchRecords.legacyMissing')}</code></dd></div></dl><p>{result.message ? present(result.message, locale) : result.error?.message ?? t('launchRecords.readBack')}</p><Notice>{t('launchRecords.notice')}</Notice><button onClick={()=>onHandoff(result)}>{t('launchRecords.toAgent')}<Icon name="arrow" size={14}/></button></div>}
    </div>
  </section>;
}
