import { useEffect, useRef, useState } from 'react';
import { requester } from './api';
import type { Environment, Plan, LaunchRecord } from './types';
import RequestFailure, { asError } from './RequestFailure';
import { Icon, Notice, Status } from './ui';
import StartupSources from './StartupSources';
export type ResumeSource={job_id?:string;archive_path?:string;archive_passphrase:string;path:string};
export default function LaunchPanel({environment,hostAlias,continuationText,source,reference,proxyUrl,initialMode='interactive'}:{environment:Environment;hostAlias:string|null;continuationText?:string;source?:ResumeSource;reference?:unknown;proxyUrl?:string;initialMode?:'interactive'|'resume'}) {
  const key=`lintel.project-cwd:${hostAlias ?? 'local'}:${environment.id}`;
  const [cwd,setCwd]=useState(localStorage.getItem(key) ?? '');
  const [mode,setMode]=useState<'interactive'|'resume'>(initialMode);
  const [text,setText]=useState(continuationText ?? '');
  const [plan,setPlan]=useState<Plan|null>(null);const [reviewed,setReviewed]=useState(false);
  const [busy,setBusy]=useState('');const [error,setError]=useState<Error|null>(null);
  const [copied,setCopied]=useState(false);const [result,setResult]=useState<LaunchRecord|null>(null);
  const [uncertain,setUncertain]=useState(false);const lock=useRef(false);const alive=useRef(true);
  useEffect(()=>{alive.current=true;return()=>{alive.current=false}},[]);
  useEffect(()=>{setPlan(null);setReviewed(false);setResult(null);setCopied(false)},[cwd,mode,text,environment.id,hostAlias,reference]);
  const send=requester(hostAlias);
  const startup=plan?.launch_request?.startup ?? plan?.resume?.startup;
  const sourcesChecked=startup?.schema==='lintel.startup/1';
  function repreview(){if(lock.current || result?.status!=='planned' || result.observed!=='plan')return;setPlan(null);setReviewed(false);setResult(null);setCopied(false);setUncertain(false);void preview()}
  async function preview(){if(lock.current)return;lock.current=true;setBusy('preview');setError(null);try{const p=mode==='resume' && source ? await send('plan_resume',{...source,environment_id:environment.id,project_cwd:cwd.trim()}) : await send('plan_launch',{environment_id:environment.id,project_cwd:cwd.trim(),mode:'interactive',...(reference?{input_reference:reference}:{}),...(proxyUrl?{proxy_url:proxyUrl}:{})});if(alive.current){setPlan(p);setReviewed(false);localStorage.setItem(key,cwd.trim())}}catch(e){if(alive.current)setError(asError(e))}finally{lock.current=false;if(alive.current)setBusy('')}}
  async function open(queryOnly=false){if(!plan || lock.current || (!queryOnly && (!reviewed || !sourcesChecked)))return;lock.current=true;setBusy('launch');setError(null);const snapshot={text,plan,mode};let launchAttempted=false;try{
    if(!queryOnly && snapshot.mode==='interactive' && continuationText!==undefined){try{await navigator.clipboard.writeText(snapshot.text)}catch{throw new Error('复制失败；当前文本保留。组合动作已停止，没有请求打开会话。')}if(alive.current)setCopied(true)}
    if(queryOnly){const record=await send('launch_query',{request_id:snapshot.plan.launch_request?.id ?? snapshot.plan.id});if(alive.current){setResult(record);setUncertain(false)}return;}
    launchAttempted=true;const request={request_id:snapshot.plan.launch_request?.id ?? snapshot.plan.id,approval:snapshot.plan.hash};const r=snapshot.mode==='resume' && source ? await send('resume_request',{...request,archive_passphrase:source.archive_passphrase}) : await send('launch_request',request);if(alive.current){setResult(r);setUncertain(false)}
  }catch(e){if(alive.current){setError(asError(e));if(launchAttempted)setUncertain(true)}}finally{lock.current=false;if(alive.current)setBusy('')}}
  return <div className="launch-workspace"><div className="launch-target"><span>{hostAlias ? `SSH · ${hostAlias}` : '本机'} · {environment.name}</span><code>{environment.root}</code><p>配置目录决定这次的设置与状态；项目目录决定在哪里工作。</p></div>
    {source && <fieldset className="launch-modes" disabled={!!busy || !!result || uncertain}><legend>怎样继续这份工作</legend><label><input type="radio" checked={mode==='interactive'} onChange={()=>setMode('interactive')}/>{continuationText!==undefined?'用审阅过的新上下文':'打开新的空白会话'}</label><label><input type="radio" checked={mode==='resume'} onChange={()=>setMode('resume')}/>Claude 原生续聊（独立检查）</label></fieldset>}
    <label className="field">项目工作目录<input aria-label="项目工作目录" value={cwd} onChange={e=>setCwd(e.target.value)} disabled={!!busy || !!result || uncertain} spellCheck={false} placeholder="/path/to/your/project"/><span className="small-print">在{hostAlias ?? '本机'}核对存在与目录身份；不会创建项目或调整权限。</span></label>
    {continuationText!==undefined && mode==='interactive' && <label className="field">最终工作交接稿<textarea className="context-editor" aria-label="最终工作交接稿" value={text} onChange={e=>setText(e.target.value)} disabled={!!busy || !!result || uncertain} spellCheck={false}/><span className="small-print">工作正文只在当前交互与系统剪贴板中处理；不进入普通任务记录。</span></label>}
    {error && <RequestFailure error={error} context="会话启动"/>}
    {hostAlias && mode==='resume' && <Notice tone="warning">App 中的归档口令只用于这次阅读与预览。批准后，请在新开的 SSH Terminal 中再次无回显输入口令；口令不会写入启动参数或原任务记录。</Notice>}
    {!result && !uncertain && <button disabled={!!busy || !cwd.trim().startsWith('/') || (continuationText!==undefined && mode==='interactive' && !text.trim())} onClick={()=>void preview()}>{busy==='preview'?'正在核对目标…':'核对启动目标'}<Icon name="arrow" size={15}/></button>}
    {plan && <section className="launch-review"><h3>{mode==='resume'?'原生续聊预览':'这次将使用'}</h3><dl><div><dt>配置 root</dt><dd><code>{plan.launch_request?.config_root ?? plan.resume?.config_root ?? environment.root}</code></dd></div><div><dt>项目 cwd</dt><dd><code>{plan.launch_request?.project_cwd ?? plan.resume?.project_cwd ?? cwd}</code></dd></div><div><dt>客户端</dt><dd><code>{plan.launch_request?.executable ?? plan.resume?.executable ?? environment.executable ?? '未发现'}</code></dd></div><div><dt>静态版本</dt><dd>{plan.launch_request?.client_version ?? plan.resume?.client_version ?? '未知'}</dd></div></dl>{plan.resume && <><Notice tone={plan.resume.supported?'neutral':'warning'}><strong>{plan.resume.supported?'有限 transcript 入口可用':'此组合暂不支持原生续聊'}</strong><p>{plan.resume.reason}</p><p>认证、原会话是否运行及实际恢复：未验证。客户端可能恢复权限/模型状态并重读当前设置、hooks 与 MCP。</p></Notice><div className="fact-row"><span>私有运行副本</span><code>{plan.resume.private_copy_path}</code></div><details><summary>客户端支持证据</summary><pre>{JSON.stringify(plan.resume.client_support,null,2)}</pre></details></>}
      {plan.resume?.write_scope && <Notice tone="warning"><strong>续聊的写入范围</strong><p>{plan.resume.write_scope.note}</p><code>{plan.resume.write_scope.config_root}</code><p>项目目录：<code>{plan.resume.write_scope.project_cwd ?? plan.resume.project_cwd}</code>。客户端也可能读取原项目或 subagent 路径；私有 transcript 副本不能作为隔离保证。</p></Notice>}
      <StartupSources inspection={startup}/>
      <Notice>普通启动使用真实 Terminal／PTY，不自动发送 prompt。复制、请求打开与用户粘贴发送分别完成；Lintel 没有观察模型接收。</Notice>
      {!result && !uncertain && <><label className="review-checkbox"><input type="checkbox" checked={reviewed} disabled={!!busy} onChange={e=>setReviewed(e.target.checked)}/>已审阅当前稿件、主机、root、项目、配置来源与启动模式</label><button className="primary" disabled={!!busy || !reviewed || !sourcesChecked || plan.resume?.supported===false} onClick={()=>void open()}>{busy==='launch'?'正在处理这一版…':mode==='resume'?'批准并请求原生续聊':continuationText!==undefined?'复制上下文并打开新会话':'批准并打开新会话'}<Icon name="arrow" size={15}/></button></>}
      <p className="launch-original-id">原启动请求 <code>{plan.launch_request?.id ?? plan.id}</code></p>
    </section>}
    {(copied || result || uncertain) && <section className="launch-result" role="status"><h3>这次动作的结果</h3>{continuationText!==undefined && mode==='interactive' && <p>复制：{copied?'已复制这份审阅稿':'未完成'}</p>}<p>启动：{result?<><Status value={result.status}/>{result.message ?? result.error?.message ?? '已读回原启动记录；客户端运行与模型接收仍需在终端确认。'}</>:uncertain?'结果待核对；保留原启动请求，不另发新请求':'尚未请求'}</p><p>粘贴与发送：待你在终端完成。模型接收：未观察。</p>{uncertain && <button disabled={!!busy} onClick={()=>void open(true)}>核对原启动请求</button>}{result?.status==='planned' && result.observed==='plan' && <button disabled={!!busy} onClick={repreview}>重新核对启动目标</button>}</section>}
  </div>;
}
