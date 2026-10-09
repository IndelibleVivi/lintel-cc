import { useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { request, RequestError, transport, type Envelope } from './api';
import { ResourceLink } from './Resources';
import RequestFailure, { asError } from './RequestFailure';
import type { Environment, ExecutionContext, Plan, Receipt, LaunchRecord } from './types';
import { Icon, Notice } from './ui';
import Clawd from './Clawd';

export type HandoffContext = { task:string;hostAlias:string|null;environment?:Environment;plan?:Plan;receipt?:Receipt;launch?:LaunchRecord;uncertain?:boolean;observedAt:string };
type CliCheck = {executable:string;version:{version:string;protocol:number;platform:string;architecture:string};context:ExecutionContext;candidate:{status:string;identity?:string;source_revision?:string};checked_at:number};
export const shellQuote=(value:string)=>`'${value.replaceAll("'", "'\\''")}'`;
// Deliberate metadata projection: never serialize a plan/receipt/state wholesale.
export function operationHandoff(context:HandoffContext, cli:CliCheck, state:ExecutionContext, binding?:string|null) {
  const launch=context.launch?.request_id;
  const job=context.receipt?.id ?? (context.uncertain ? context.plan?.id : undefined);
  const prefix=`env HOME=${shellQuote(state.user.home)} LINTEL_STATE_DIR=${shellQuote(state.state.path)} ${shellQuote(cli.executable)}`;
  const remoteInspect=context.environment ? JSON.stringify({command:'inspect',environment_id:context.environment.id}) : '';
  const commands=launch ? [context.hostAlias ? prefix+' remote launch query '+shellQuote(context.hostAlias)+' '+shellQuote(launch) : prefix+' launch query '+shellQuote(launch)] : job ? [context.hostAlias ? `${prefix} remote job ${shellQuote(context.hostAlias)} ${shellQuote(job)}` : `${prefix} job show ${shellQuote(job)}`] : context.plan ? [context.hostAlias ? `printf '%s\\n' ${shellQuote(JSON.stringify({command:'plan_show',plan_id:context.plan.id}))} | ${prefix} remote request ${shellQuote(context.hostAlias)}` : `${prefix} plan show ${shellQuote(context.plan.id)}`] : context.environment ? [context.hostAlias ? `printf '%s\\n' ${shellQuote(remoteInspect)} | ${prefix} remote request ${shellQuote(context.hostAlias)}` : `${prefix} env inspect ${shellQuote(context.environment.id)}`] : [`${prefix} context --json`,`${prefix} tasks --json`];
  return JSON.stringify({schema:'lintel.agent-handoff/1',task:{id:context.task},target:{host:context.hostAlias ? {kind:'ssh',alias:context.hostAlias,runner_digest:binding ?? null,binding_resolution:launch?'original_launch_record':job?'original_task_record':'current_alias',binding_observed:binding!==undefined,runner_binding_kind:binding===undefined?'unknown':binding===null?'path_compatibility':'pinned_digest'} : {kind:'local'},environment:context.environment ? {id:context.environment.id,name:context.environment.name,config_root:context.environment.root,executable:context.environment.executable} : null},executor:{path:cli.executable,...cli.version,candidate:cli.candidate},state_context:{home:state.user.home,uid:state.user.uid,euid:state.user.euid,source:state.state.source,path:state.state.path},plan:context.plan ? {id:context.plan.id,hash:context.plan.hash,status:context.plan.status} : null,job:context.receipt ? {id:context.receipt.id,plan_id:context.receipt.plan_id,status:context.receipt.status} : job ? {id:job,status:'uncertain'} : null,launch_request:context.launch ? {request_id:context.launch.request_id,status:context.launch.status,mode:context.launch.mode,config_root:context.launch.root ?? context.launch.config_root,project_cwd:context.launch.project_cwd} : null,observed_at:context.observedAt,next_action:{mode:launch || job ? 'query_original_only' : 'inspect_before_action',commands,notes:context.hostAlias && !job && !launch ? ['Use a reviewed finite JSON request for the selected remote environment; no mutation is authorized by this handoff.'] : []},authorization:{copy_grants_new_authority:false,approval:context.launch || context.receipt || context.uncertain ? 'Original task only; never regenerate or resubmit a mutation.' : 'Inspect and review first. Execution requires exact independent approval.',secrets:'Use JSON stdin only; no secrets in argv, logs or this packet.'}},null,2);
}
export default function AgentPanel({context}:{context:HandoffContext}) {
  const [executable,setExecutable]=useState(localStorage.getItem('lintel.cli-path') ?? '');
  const [check,setCheck]=useState<CliCheck|null>(null);
  const [appContext,setAppContext]=useState<ExecutionContext|null>(null);
  const [stateChoice,setStateChoice]=useState<'app'|'cli'>('app');
  const [binding,setBinding]=useState<string|null>();
  const [busy,setBusy]=useState(false);const [error,setError]=useState<Error|null>(null);
  const [reviewed,setReviewed]=useState(false);const [copyState,setCopyState]=useState('');
  const alive=useRef(true);const inspecting=useRef(false);
  useEffect(()=>{alive.current=true;return()=>{alive.current=false}},[]);
  useEffect(()=>{setReviewed(false);setCopyState('');setBinding(undefined)},[context]);
  async function inspect() {
    if(inspecting.current)return;inspecting.current=true;setBusy(true);setError(null);setReviewed(false);setCheck(null);setCopyState('');
    const path=executable.trim();
    try {
      const r=await invoke<Envelope<CliCheck>>('inspect_cli',{executable:path});
      if(!r.ok)throw new RequestError(r.error.code,r.error.message,r.error.diagnostic);
      const app=await request('context',{});
      if(!alive.current)return;setCheck(r.data);setAppContext(app);localStorage.setItem('lintel.cli-path',path);
      if(context.hostAlias && (context.receipt || context.uncertain)) {
        const inv=await invoke<Envelope<{tasks:{alias:string;plan_id:string;runner_digest?:string}[]}>>('remote_request',{payload:{op:'hosts'}});
        if(inv.ok && alive.current)setBinding(inv.data.tasks.find(t=>t.alias===context.hostAlias && t.plan_id===(context.receipt?.plan_id ?? context.plan?.id))?.runner_digest);
      }
    }catch(e){if(alive.current)setError(asError(e))}finally{inspecting.current=false;if(alive.current)setBusy(false)}
  }
  const state=stateChoice==='app' ? appContext : check?.context;
  const text=useMemo(()=>check && state ? operationHandoff(context,check,state,context.launch?.runner_digest !== undefined ? context.launch.runner_digest : binding) : '',[context,check,state,binding]);
  useEffect(()=>{setReviewed(false);setCopyState('')},[text]);
  const mismatch=check && appContext && (check.context.state.path!==appContext.state.path || check.context.user.home!==appContext.user.home || check.context.user.euid!==appContext.user.euid);
  const pinnedState=!!(context.environment || context.plan || context.receipt || context.launch || context.hostAlias);
  const wrongUser=!!(check && state && (check.context.user.uid!==state.user.uid || check.context.user.euid!==state.user.euid));
  const wrongState=!!(mismatch && pinnedState && stateChoice==='cli');
  useEffect(()=>{if(mismatch && pinnedState)setStateChoice('app')},[mismatch,pinnedState]);
  async function copy(){if(!reviewed || !text || wrongUser || wrongState)return;const snapshot=text;try{await navigator.clipboard.writeText(snapshot);if(alive.current)setCopyState('已复制这一版操作交接包。执行仍需对应批准；未提交任何任务。')}catch{if(alive.current)setCopyState('复制失败；文本保留，可手动选择复制。')}}
  return <div className="agent-workspace"><div className="page-heading"><div><span className="eyebrow">terminal / a precise handoff</span><h1>终端与 Agent</h1><p>选定执行器，核对同一份 state，再把眼前的任务准确交接。</p></div><Clawd small mood="work"/></div>
    <div className="agent-target"><span>{context.hostAlias ? `SSH · ${context.hostAlias}` : '本机'}</span><strong>{context.plan?.network || context.receipt?.network_change ? '宿主共享网络' : context.environment?.name ?? '尚未选择环境'}</strong>{context.environment && <code>{context.environment.root}</code>}{context.launch && <p>原启动请求 <code>{context.launch.request_id}</code> · {context.launch.status}</p>}{context.receipt && <p>原任务 <code>{context.receipt.id}</code> · {context.receipt.status}</p>}</div>
    <section className="agent-step"><span className="step-number">01</span><div><h2>选定一份 CLI</h2><p>已有 CLI：输入准确路径。拿到候选包：先按说明解包并核验。源码构建：使用实际生成的 executable。</p><form onSubmit={e=>{e.preventDefault();void inspect()}} className="cli-select"><label className="field">Lintel CLI 完整路径<input value={executable} disabled={busy} onChange={e=>{setExecutable(e.target.value);setCheck(null);setReviewed(false);setCopyState('')}} placeholder="/path/to/lintel-cli/bin/lintel" spellCheck={false}/></label><button disabled={busy || !executable.trim() || transport==='unavailable'}>{busy?'正在静态核对…':'核对 CLI 与上下文'}</button></form><ResourceLink resource="agent-guide">取得、安装和使用 CLI</ResourceLink></div></section>
    {error && <RequestFailure error={error} context="CLI 核对"/>}
    {check && appContext && <><section className="agent-step"><span className="step-number">02</span><div><h2>核对实际执行上下文</h2><div className="context-facts"><div><span>执行器</span><code>{check.executable}</code></div><div><span>接口身份</span><span>{check.version.version} · protocol {check.version.protocol} · {check.version.platform}/{check.version.architecture}</span></div><div><span>候选</span><span>{check.candidate.identity ?? '独立构建／候选身份未知'} · {check.candidate.status==='bytes_match'?'二进制字节与 sidecar 一致，来源未经签名认证':'已观察到静态接口可用'}</span></div><div><span>App state</span><code>{appContext.state.path}</code></div><div><span>CLI state</span><code>{check.context.state.path}</code></div><div><span>执行用户</span><span>uid {check.context.user.uid} / euid {check.context.user.euid}</span></div></div>{mismatch && <Notice tone="warning">App 与 CLI 的 HOME、state 或执行用户不同。明确选择要交接的上下文；Lintel 不复制 state 或修改 shell。用户身份不同须由正确用户启动 CLI。</Notice>}{mismatch && pinnedState && <Notice>当前环境、计划或原任务属于 App state。交接必须使用这份 state，避免丢失原 ID 与 runner 绑定；不会迁移或重新提交任务。</Notice>}{wrongUser && <Notice tone="warning">选定 CLI 的执行用户与任务用户不同。请由原用户打开 App／CLI 后重新核对；设置 HOME 不能切换用户。</Notice>}<fieldset className="context-choice"><legend>交接使用哪份 state</legend><label><input type="radio" checked={stateChoice==='app'} onChange={()=>setStateChoice('app')}/>App 当前 state</label><label><input type="radio" disabled={!!(mismatch && pinnedState)} checked={stateChoice==='cli'} onChange={()=>setStateChoice('cli')}/>CLI 观察到的 state</label></fieldset></div></section>
    <section className="agent-step"><span className="step-number">03</span><div><h2>审阅操作交接包</h2><p>这里包含目标与操作元数据；工作正文通过“会话与资料”单独选择。已接收或不确定的任务只查询原 ID。</p><textarea className="handoff-packet" aria-label="Agent 操作交接包" value={text} readOnly spellCheck={false}/><label className="review-checkbox"><input type="checkbox" checked={reviewed} onChange={e=>setReviewed(e.target.checked)}/>已核对这一版目标、CLI、state 和原任务</label><button className="primary" disabled={!reviewed || !text || wrongUser || wrongState} onClick={()=>void copy()}><Icon name="copy" size={15}/>复制交接包</button>{copyState && <p className="persistent-feedback" role="status">{copyState}</p>}</div></section></>}
    {!check && <Notice>静态核对不会初始化 state、执行 discover 或启动 Claude。没有 CLI 时先按说明取得当前平台产物；本轮没有后台下载或 PATH 修改。</Notice>}
  </div>;
}
