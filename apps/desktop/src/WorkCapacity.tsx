import {useEffect,useState} from 'react';
import {request} from './api';
import type {Environment,WorkPreflight} from './types';
import {formatBytes,formatDate,Icon,Notice} from './ui';
import RequestFailure,{asError} from './RequestFailure';

export default function WorkCapacity({send=request,environment,categories,selectedPaths=null,selectionReady=true,onResult}:{send?:typeof request;environment:Environment;categories:string[];selectedPaths?:string[]|null;selectionReady?:boolean;onResult:(result:WorkPreflight|null)=>void}) {
  const [result,setResult]=useState<WorkPreflight|null>(null),[error,setError]=useState<Error|null>(null),[busy,setBusy]=useState(false),[refresh,setRefresh]=useState(0);
  const selection=JSON.stringify([categories,selectedPaths]);
  useEffect(()=>{
    let current=true;setResult(null);onResult(null);setError(null);
    if(!categories.length||!selectionReady||selectedPaths?.length===0){setBusy(false);return;}
    setBusy(true);
    void send('work_preflight',{environment_id:environment.id,categories,...(selectedPaths!==null?{selected_paths:selectedPaths}:{})}).then(report=>{
      if(report.environment_id!==environment.id||report.root!==environment.root)throw new Error('容量结果与当前目标不匹配，请重新检查。');
      if(current){setResult(report);onResult(report);}
    }).catch(err=>{if(current)setError(asError(err));}).finally(()=>{if(current)setBusy(false);});
    return()=>{current=false;};
  },[send,environment.id,environment.root,selection,selectionReady,refresh]);
  return <section className="surface capacity-preflight" aria-label="工作容量预检">
    <div className="surface-heading"><h2>工作容量预检</h2><button className="text-button" disabled={busy||!categories.length||!selectionReady||selectedPaths?.length===0} onClick={()=>setRefresh(v=>v+1)}><Icon name="refresh" size={14}/>刷新容量</button></div>
    <div className="padded">
      {busy&&<p role="status">正在检查所选原件的文件元数据…</p>}
      {!categories.length&&<p className="muted">先选择要保留的工作类别。</p>}
      {!!categories.length&&(!selectionReady||selectedPaths?.length===0)&&<p className="muted">请完成文件清单核对，并至少选择一份原件。</p>}
      {error&&<><RequestFailure error={error} context="工作容量预检"/><p className="small-print">未取得容量结论。旧 runner 不支持此入口时可更新 runner；原计划预览仍会执行自己的完整准入检查。</p></>}
      {result&&<>
        <div className="fact-row"><span>所选已扫描内容</span><strong>{result.totals.files} 个文件 / {formatBytes(result.totals.bytes)}</strong></div>
        <p className="small-print">单文件 {formatBytes(result.limits.file_bytes)} · 所选合计 {formatBytes(result.limits.total_bytes)} · {result.limits.files.toLocaleString()} 个文件以内</p>
        <Notice tone={result.eligible?'neutral':'warning'}>{!result.complete?'扫描未完整覆盖；不能据当前统计生成完整保全结论。':result.eligible?'当前元数据在容量范围内；计划预览仍会读取并核对全部选中原件。':'当前选择超过容量或存在阻塞项。可以取消类别，或使用上方的精确选择排除具体原件后重新检查；原件不会被删除或截断。'}</Notice>
        {!!result.blockers.length&&<ul className="capacity-blockers">{result.blockers.map((blocker,i)=><li key={i}>{blocker.path&&<code>{blocker.path}</code>}<span>{blocker.message}</span><small>{blocker.code}</small></li>)}</ul>}
        {result.blockers_truncated&&<p className="small-print">阻塞列表已达显示上限；未列出的对象仍计入阻塞，不代表可通过。</p>}
        <p className="small-print">{formatDate(result.checked_at)} · 只读元数据，不读取正文、凭据或执行指令。</p>
      </>}
    </div>
  </section>;
}
