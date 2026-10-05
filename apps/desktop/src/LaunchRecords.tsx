import { useEffect, useRef, useState } from 'react';
import { request } from './api';
import type { LaunchRecord } from './types';
import RequestFailure, { asError } from './RequestFailure';
import { Icon, Notice, Status, formatDate } from './ui';

export default function LaunchRecords({send=request,hostAlias,onHandoff}:{send?:typeof request;hostAlias:string|null;onHandoff:(record:LaunchRecord)=>void}) {
  const [records,setRecords]=useState<LaunchRecord[]>([]);
  const [query,setQuery]=useState('');const [result,setResult]=useState<LaunchRecord|null>(null);
  const [busy,setBusy]=useState(false);const [error,setError]=useState<Error|null>(null);
  const alive=useRef(true);const lock=useRef(false);
  useEffect(()=>{alive.current=true;setRecords([]);setResult(null);setQuery('');
    void send('launches',{}).then(r=>{if(alive.current)setRecords(r.launches)}).catch(e=>{if(alive.current)setError(asError(e))});
    return()=>{alive.current=false};
  },[send]);
  async function lookup(id:string){if(lock.current)return;lock.current=true;setBusy(true);setError(null);try{const r=await send('launch_query',{request_id:id.trim()});if(alive.current){setResult(r);setQuery(id)}}catch(e){if(alive.current)setError(asError(e))}finally{lock.current=false;if(alive.current)setBusy(false)}}
  return <section className="launch-records surface"><div className="surface-heading"><div><span className="eyebrow">terminal / original request</span><h2>会话启动记录</h2></div><span>{hostAlias?`SSH · ${hostAlias}`:'本机'} · {records.length} 条</span></div>
    <p>关闭窗口、重启 App 或回复丢失后，继续核对原启动请求。这里不再次启动，也不复制工作正文。</p>
    {error && <RequestFailure error={error} context="原启动记录"/>}
    <form className="launch-record-query" onSubmit={e=>{e.preventDefault();void lookup(query)}}><label className="field">原启动请求 ID<input aria-label="原启动请求 ID" value={query} onChange={e=>setQuery(e.target.value)} spellCheck={false} placeholder="原 launch / resume 请求 ID"/></label><button disabled={busy || !query.trim()}>只读核对<Icon name="refresh" size={14}/></button></form>
    {records.length>0 && <div className="launch-record-list">{records.map(record=><button key={record.request_id} disabled={busy} onClick={()=>void lookup(record.request_id)}><span><strong>{record.mode==='resume'?'原生续聊':'新会话'}</strong><code>{record.project_cwd ?? '项目位置未记录'}</code><small>{record.recorded_at?formatDate(record.recorded_at):'时间未记录'} · {record.request_id}</small></span><Status value={record.status}/></button>)}</div>}
    {result && <div className="launch-record-detail" role="status"><Status value={result.status}/><dl><div><dt>原 ID</dt><dd><code>{result.request_id}</code></dd></div><div><dt>配置 root</dt><dd><code>{result.root ?? result.config_root ?? '旧记录未提供'}</code></dd></div><div><dt>项目 cwd</dt><dd><code>{result.project_cwd ?? '旧记录未提供'}</code></dd></div></dl><p>{result.message ?? result.error?.message ?? '已读回记录；不能从 intent 或 Terminal 接收推断客户端已运行。'}</p><Notice>认证、粘贴发送、模型接收与实际恢复需要分别确认。原件与原 ID 保留；核对不会重试副作用。</Notice><button onClick={()=>onHandoff(result)}>把原启动请求交给 Agent<Icon name="arrow" size={14}/></button></div>}
  </section>;
}
