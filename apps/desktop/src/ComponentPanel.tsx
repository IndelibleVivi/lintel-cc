import { LocalizedError } from './i18n';
import {useEffect,useRef,useState} from 'react';
import {request} from './api';
import type {ComponentInspection,Environment} from './types';
import {Icon,Notice} from './ui';
import RequestFailure,{asError} from './RequestFailure';
import {useI18n} from './i18n-provider';
import {present} from './i18n-presentation';

const stateKeys:Record<string,string>={observed:'component.state.observed',not_found:'component.state.notFound',unknown:'component.state.unknown',unsupported:'component.state.unsupported',separate_module:'component.state.separateModule',access_limited:'component.state.accessLimited',observed_directory:'component.state.observedDirectory',unsupported_type:'component.state.unsupportedType'};
const entryKeys:Record<string,string>={policy:'component.entry.policy',cleanup:'component.entry.cleanup',browser:'component.entry.browser',service:'component.entry.service',launch:'component.entry.launch'};
export default function ComponentPanel({send=request,environment,onEntry,onJob}:{send?:typeof request;environment:Environment;onEntry:(entry:string)=>void;onJob:(id:string)=>void}) {
  const { t, locale, formatDateTime } = useI18n();
  const presentText = (value: string | null | undefined) => present(value, locale);
  const [report,setReport]=useState<ComponentInspection|null>(null),[project,setProject]=useState(''),[busy,setBusy]=useState(false),[error,setError]=useState<Error|null>(null);
  const generation=useRef(0),lock=useRef(false);
  const stateName = (value: string) => stateKeys[value] ? t(stateKeys[value]) : value;
  async function refresh(cwd:string) {
    if(lock.current)return;lock.current=true;const token=++generation.current;
    setBusy(true);setError(null);setReport(null);
    try {
      const result=await send('inspect_components',{environment_id:environment.id,...(cwd.trim()?{project_cwd:cwd.trim()}:{})});
      if(result.environment_id!==environment.id||result.root!==environment.root)throw new LocalizedError('component.targetMismatch');
      if(generation.current===token)setReport(result);
    } catch(err){if(generation.current===token)setError(asError(err));}
    finally{if(generation.current===token){lock.current=false;setBusy(false);}}
  }
  useEffect(()=>{void refresh('');return()=>{generation.current++;lock.current=false;};},[]);
  return <section className="component-inspection" aria-label={t('component.aria')}>
    <div className="surface-heading"><div><h2>{t('component.aria')}</h2><p className="small-print">{t('component.intro')}</p></div><button disabled={busy} onClick={()=>void refresh(project)}><Icon name="refresh" size={14}/>{t('component.refresh')}</button></div>
    <label className="field">{t('component.projectLabel')}<input value={project} onChange={e=>setProject(e.target.value)} disabled={busy} placeholder={t('component.projectPlaceholder')}/><span className="small-print">{t('component.projectNote')}</span></label>
    {error&&<RequestFailure error={error} context={t('component.context')}/>}
    {busy&&<p role="status" className="muted">{t('component.reading')}</p>}
    {report&&<>
      <p className="small-print">{formatDateTime(report.checked_at)} · {t('component.configRoot')} <code>{report.root}</code>{report.project_cwd&&<> · {t('component.project')} <code>{report.project_cwd}</code></>}</p>
      <div className="component-list">{report.items.map(item=><section className="component-row" key={item.id} data-component={item.id}>
        <div className="component-heading"><h3>{item.title}</h3><span className="component-state">{stateName(item.state)}</span></div>
        <p>{item.detail}</p><p className="small-print">{t('component.scopeLine',{scope:item.scope,source:item.source})}</p>
        {item.id==='cli'&&<div className="fact-row"><span>{t('component.currentChoice')}</span><code>{item.facts.selected_executable??t('component.notLocated')}</code></div>}
        {item.facts.sources&&<details><summary>{t('component.viewSources')}</summary><ul className="component-sources">{item.facts.sources.map((source)=><li key={source.scope+source.path}><strong>{source.scope}</strong> · {stateName(source.state)}<code>{source.path}</code>{source.content_state==='unknown'&&<span>{t('component.contentUnknown')}</span>}{source.hooks_declared&&<span>{t('component.hooksDeclared')}</span>}{source.mcp_declared&&<span>{t('component.mcpDeclared')}</span>}</li>)}</ul></details>}
        {item.id==='authentication'&&<p className="small-print">{t('component.authFiles',{credential:stateName(item.facts.credential_file?.state??'unknown'),shared:stateName(item.facts.shared_profile?.state??'unknown')})}</p>}
        {item.facts.services?.map((service)=><div className="component-service" key={service.manager+service.unit}><code>{service.manager} / {service.unit}</code><p>{service.state==='observed'&&service.current?t('component.serviceObserved',{state:service.current.active_state,quiesced:service.current.quiesced?t('component.quiesceValid'):t('component.quiesceUnconfirmed')}):t('component.serviceUnverifiable')} · {t('component.originalRecord',{status:service.recorded_status})}</p><button onClick={()=>onJob(service.original_job_id)}>{t('component.checkOriginalService')}</button></div>)}
        {item.facts.services_truncated&&<p className="small-print">{t('component.servicesTruncated')}</p>}
        {item.next_action!=='none'&&entryKeys[item.next_action]&&<button className="text-button" onClick={()=>onEntry(item.next_action)}>{t(entryKeys[item.next_action])}<Icon name="arrow" size={14}/></button>}
      </section>)}</div>
      <details className="component-records"><summary>{t('component.recordsSummary',{count:report.records.length})}</summary>
        {report.records.map(record=><div key={record.id}><strong>{presentText(record.title)}</strong><p className="small-print">{t('component.recordLine',{status:record.status,time:formatDateTime(record.created_at)})}</p>{record.coverage?.map((fact,i)=><p className="small-print" key={i}>{fact.scope} · {fact.state} · {presentText(fact.detail)}</p>)}<button onClick={()=>onJob(record.id)}>{t('component.checkOriginalTask',{id:record.id})}</button></div>)}
        {!report.records.length&&<p>{t('component.noRecords')}</p>}
        {(!report.records_complete||report.records_truncated)&&<Notice tone="warning">{t('component.recordsPartial')}</Notice>}
      </details>
      <p className="small-print">{presentText(report.note)}</p>
    </>}
  </section>;
}
