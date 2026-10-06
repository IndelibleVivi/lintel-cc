import { useEffect, useRef, useState } from 'react';
import { request } from './api';
import Clawd from './Clawd';
import WorkCapacity from './WorkCapacity';
import {readTaskDraft,saveTaskDraft} from './taskDrafts';
import { ResourceLink } from './Resources';
import type { Environment, Inspection, Plan, WorkPreflight } from './types';
import { Icon, Notice, formatBytes, WorkspaceActions } from './ui';
import RequestFailure, { asError } from './RequestFailure';

const classes = [['instructions','个人指令','保留原始文本，不运行其中命令'],['memory','记忆文本','迁入待用区，保持原始资料'],['sessions','会话资料','保留可阅读资料，不保证可以续聊']];

export default function WorkPanel({ send = request, environment, inspection, onPlan, onArchives,hostAlias=null }: {hostAlias?:string|null; send?: typeof request; environment: Environment; inspection: Inspection | null; onPlan: (plan: Plan) => void; onArchives: () => void }) {
  const draftKey='lintel.work-draft:'+JSON.stringify([hostAlias,environment.id]);
  const initial=readTaskDraft(draftKey,{mode:'archive',categories:['instructions','memory','sessions'],output:'',name:'',activateInstructions:false});
  const [mode,setMode] = useState<'archive'|'preserve'>(initial.mode==='preserve'?'preserve':'archive');
  const [categories,setCategories] = useState(initial.categories);
  const [output,setOutput] = useState(initial.output);
  const [name,setName] = useState(initial.name);
  const [busy,setBusy] = useState(false);
  const [error,setError] = useState<Error|null>(null);
  const [activateInstructions,setActivateInstructions] = useState(initial.activateInstructions);
  const [capacity,setCapacity] = useState<WorkPreflight|null>(null);
  const alive = useRef(true);const actionLock=useRef(false);
  useEffect(() => { alive.current=true; return () => {alive.current=false;}; }, []);
  useEffect(()=>saveTaskDraft(draftKey,{mode,categories,output,name,activateInstructions}),[draftKey,mode,categories,output,name,activateInstructions]);
  async function preview() {
    if(actionLock.current)return;actionLock.current=true;setBusy(true); setError(null);
    try {
      const fields = { environment_id:environment.id,categories };
      const plan = mode === 'archive' ? await send('plan_archive',{...fields,...(output.trim()?{output_path:output.trim()}:{})}) : await send('plan_preserve',{...fields,activate:{instructions:activateInstructions && categories.includes('instructions')},...(name.trim()?{name:name.trim()}:{})});
      if (alive.current) onPlan(plan);
    } catch(err) { if(alive.current) setError(asError(err)); }
    finally { actionLock.current=false;if(alive.current) setBusy(false); }
  }
  return <>
    <div className="page-heading"><div><span className="eyebrow">work / keep what matters</span><h1>正在做的事，好好收着</h1><p>先保全工作，再决定要不要从新环境继续。</p></div><Clawd small mood="pack"/></div>
    {error && <RequestFailure error={error} context="工作保全"/>}
    <fieldset className="recipe-grid work-modes" disabled={busy}><legend className="sr-only">工作保全目标</legend>{[['archive','只生成工作归档','加密收好选定资料；不建立环境、不改登录。'],['preserve','保全并准备新环境','先加密归档，再建立新根、迁入所选资料。']].map(([id,title,detail]) => <label className={`recipe ${mode===id?'chosen':''}`} key={id}><input type="radio" name="work-mode" checked={mode===id} onChange={() => {setMode(id as typeof mode);setError(null);}}/><span><strong>{title}</strong><small>{detail}</small></span></label>)}</fieldset>
    <section className="surface work-selection"><div className="surface-heading"><h2>保留哪些工作内容</h2><span className="small-label">{environment.name}</span></div>{classes.map(([id,title,detail]) => {const asset=inspection?.assets.find(a=>a.category===id);return <label className="checkbox-row" key={id}><span><strong>{title}{asset&&<small> · {asset.count} 个文件 / {formatBytes(asset.bytes)}</small>}</strong><span>{detail}</span></span><input type="checkbox" disabled={busy} checked={categories.includes(id)} onChange={event=>setCategories(current=>event.target.checked?[...current,id]:current.filter(c=>c!==id))}/></label>;})}<div className="padded">{mode==='archive'?<label className="field">另存加密包的完整路径（可选）<input aria-label="另存加密包的完整路径（可选）" value={output} disabled={busy} onChange={event=>setOutput(event.target.value)} placeholder="留空则保存在 Lintel 的工作归档中"/><span className="small-print">路径属于当前目标主机；已有文件不会被覆盖。把加密包带去另一台机器后，可以不依赖原任务记录读取和迁入。</span></label>:<label className="field">新环境名称（可选）<input value={name} disabled={busy} onChange={event=>setName(event.target.value)} placeholder="给这份新开始起个名字"/></label>}</div>{mode==='preserve' && <label className="checkbox-row"><span><strong>启用已审阅的个人指令</strong><span>勾选后 CLAUDE.md 进入客户端可能自动读取的指令位置；否则与记忆、会话一起作为待用资料保留。</span></span><input type="checkbox" disabled={busy || !categories.includes('instructions')} checked={activateInstructions} onChange={event=>setActivateInstructions(event.target.checked)}/></label>}</section>
    <WorkCapacity send={send} environment={environment} categories={categories} onResult={setCapacity}/>
    <Notice>旧配置、旧登录与原始工作内容保留。归档不带入凭据、settings、hooks 或 MCP；新环境仍需选择保护方案并正常登录。</Notice>
    <WorkspaceActions><button disabled={busy} onClick={onArchives}>阅读或迁入已有工作包</button><button className="primary" disabled={busy||!categories.length||capacity?.eligible===false|| (mode==='archive'&&!!output.trim()&&!output.trim().startsWith('/'))} onClick={()=>void preview()}>{busy?'正在核对文件范围…':'预览保全计划'}<Icon name="arrow" size={15}/></button></WorkspaceActions>
    <div className="inline-route"><ResourceLink resource="work-guide">保全、迁移与跨主机使用</ResourceLink></div>
  </>;
}
