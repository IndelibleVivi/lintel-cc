import {useEffect,useRef,useState} from 'react';
import {request} from './api';
import type {ComponentInspection,Environment} from './types';
import {formatDate,Icon,Notice} from './ui';
import RequestFailure,{asError} from './RequestFailure';

const states:Record<string,string>={observed:'已观察',not_found:'已检查且未发现',unknown:'未知',unsupported:'暂不支持',separate_module:'独立模块',access_limited:'访问受限',observed_directory:'发现目录',unsupported_type:'文件类型不支持'};
const entries:Record<string,string>={policy:'核对保护方案',cleanup:'检查认证与清理范围',browser:'本机浏览器工作空间',service:'精确服务检查／暂停',launch:'核对启动目标'};
export default function ComponentPanel({send=request,environment,onEntry,onJob}:{send?:typeof request;environment:Environment;onEntry:(entry:string)=>void;onJob:(id:string)=>void}) {
  const [report,setReport]=useState<ComponentInspection|null>(null),[project,setProject]=useState(''),[busy,setBusy]=useState(false),[error,setError]=useState<Error|null>(null);
  const generation=useRef(0),lock=useRef(false);
  async function refresh(cwd:string) {
    if(lock.current)return;lock.current=true;const token=++generation.current;
    setBusy(true);setError(null);setReport(null);
    try {
      const result=await send('inspect_components',{environment_id:environment.id,...(cwd.trim()?{project_cwd:cwd.trim()}:{})});
      if(result.environment_id!==environment.id||result.root!==environment.root)throw new Error('组件结果与当前目标不匹配，请重新检查。');
      if(generation.current===token)setReport(result);
    } catch(err){if(generation.current===token)setError(asError(err));}
    finally{if(generation.current===token){lock.current=false;setBusy(false);}}
  }
  useEffect(()=>{void refresh('');return()=>{generation.current++;lock.current=false;};},[]);
  return <section className="component-inspection" aria-label="关联组件与处理范围">
    <div className="surface-heading"><div><h2>关联组件与处理范围</h2><p className="small-print">当前观察与原任务记录分别列出，操作仍需自己的预览。</p></div><button disabled={busy} onClick={()=>void refresh(project)}><Icon name="refresh" size={14}/>刷新组件</button></div>
    <label className="field">用于检查来源的项目目录（可选）<input value={project} onChange={e=>setProject(e.target.value)} disabled={busy} placeholder="当前目标主机上的绝对路径"/><span className="small-print">只检查所选目录的有限配置来源；填写后点“刷新组件”，不会创建目录或启动 Claude。</span></label>
    {error&&<RequestFailure error={error} context="关联组件检查"/>}
    {busy&&<p role="status" className="muted">正在读取有限来源与原任务…</p>}
    {report&&<>
      <p className="small-print">{formatDate(report.checked_at)} · 配置根 <code>{report.root}</code>{report.project_cwd&&<> · 项目 <code>{report.project_cwd}</code></>}</p>
      <div className="component-list">{report.items.map(item=><section className="component-row" key={item.id} data-component={item.id}>
        <div className="component-heading"><h3>{item.title}</h3><span className="component-state">{states[item.state]??item.state}</span></div>
        <p>{item.detail}</p><p className="small-print">范围：{item.scope} · 来源：{item.source}</p>
        {item.id==='cli'&&<div className="fact-row"><span>当前选择</span><code>{item.facts.selected_executable??'未定位'}</code></div>}
        {item.facts.sources&&<details><summary>查看有限配置来源</summary><ul className="component-sources">{item.facts.sources.map((source)=><li key={source.scope+source.path}><strong>{source.scope}</strong> · {states[source.state]??source.state}<code>{source.path}</code>{source.content_state==='unknown'&&<span>内容无法解析，声明情况未知</span>}{source.hooks_declared&&<span>声明了 hooks，未执行</span>}{source.mcp_declared&&<span>声明了 MCP，未启动</span>}</li>)}</ul></details>}
        {item.id==='authentication'&&<p className="small-print">本地凭据文件：{states[item.facts.credential_file?.state??"unknown"]??'未知'} · 共享 profile：{states[item.facts.shared_profile?.state??"unknown"]??'未知'}</p>}
        {item.facts.services?.map((service)=><div className="component-service" key={service.manager+service.unit}><code>{service.manager} / {service.unit}</code><p>{service.state==='observed'&&service.current?`当前 ${service.current.active_state} · ${service.current.quiesced?'原暂停条件仍有效':'暂停条件未确认'}`:'当前无法核验'} · 原记录 {service.recorded_status}</p><button onClick={()=>onJob(service.original_job_id)}>核对原服务任务</button></div>)}
        {item.facts.services_truncated&&<p className="small-print">服务列表已达上限；其他原记录中的 unit 尚未刷新。</p>}
        {item.next_action!=='none'&&entries[item.next_action]&&<button className="text-button" onClick={()=>onEntry(item.next_action)}>{entries[item.next_action]}<Icon name="arrow" size={14}/></button>}
      </section>)}</div>
      <details className="component-records"><summary>原任务的处理记录 · {report.records.length} 份</summary>
        {report.records.map(record=><div key={record.id}><strong>{record.title}</strong><p className="small-print">记录状态 {record.status} · {formatDate(record.created_at)}</p>{record.coverage?.map((fact,i)=><p className="small-print" key={i}>{fact.scope} · {fact.state} · {fact.detail}</p>)}<button onClick={()=>onJob(record.id)}>核对原任务 {record.id}</button></div>)}
        {!report.records.length&&<p>当前已读取范围中没有此环境的任务记录。</p>}
        {(!report.records_complete||report.records_truncated)&&<Notice tone="warning">原记录只覆盖已读取部分；这里的空项不能证明没有其他记录。</Notice>}
      </details>
      <p className="small-print">{report.note}</p>
    </>}
  </section>;
}
