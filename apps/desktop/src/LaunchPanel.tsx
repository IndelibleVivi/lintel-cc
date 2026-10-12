import { LocalizedError } from './i18n';
import { useEffect, useRef, useState } from 'react';
import { requester } from './api';
import type { Environment, Plan, LaunchRecord, InputReference } from './types';
import RequestFailure, { asError } from './RequestFailure';
import { Icon, Notice, Status } from './ui';
import StartupSources from './StartupSources';
import { useI18n } from './i18n-provider';
import { present } from './i18n-presentation';
export type ResumeSource={job_id?:string;archive_path?:string;archive_passphrase:string;path:string};
export default function LaunchPanel({environment,hostAlias,continuationText,source,reference,proxyUrl,initialMode='interactive'}:{environment:Environment;hostAlias:string|null;continuationText?:string;source?:ResumeSource;reference?:InputReference;proxyUrl?:string;initialMode?:'interactive'|'resume'}) {
  const { t, locale } = useI18n();
  const presentText = (value: string | null | undefined) => present(value, locale);
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
    if(!queryOnly && snapshot.mode==='interactive' && continuationText!==undefined){try{await navigator.clipboard.writeText(snapshot.text)}catch{throw new LocalizedError('launchPanel.copyFailed')}if(alive.current)setCopied(true)}
    if(queryOnly){const record=await send('launch_query',{request_id:snapshot.plan.launch_request?.id ?? snapshot.plan.id});if(alive.current){setResult(record);setUncertain(false)}return;}
    launchAttempted=true;const request={request_id:snapshot.plan.launch_request?.id ?? snapshot.plan.id,approval:snapshot.plan.hash};const r=snapshot.mode==='resume' && source ? await send('resume_request',{...request,archive_passphrase:source.archive_passphrase}) : await send('launch_request',request);if(alive.current){setResult(r);setUncertain(false)}
  }catch(e){if(alive.current){setError(asError(e));if(launchAttempted)setUncertain(true)}}finally{lock.current=false;if(alive.current)setBusy('')}}
  return <div className="launch-workspace"><div className="launch-target"><span>{hostAlias ? t('launchPanel.sshLabel',{alias:hostAlias}) : t('app.localLabel')} · {environment.name}</span><code>{environment.root}</code><p>{t('launchPanel.targetNote')}</p></div>
    {source && <fieldset className="launch-modes" disabled={!!busy || !!result || uncertain}><legend>{t('launchPanel.modesLegend')}</legend><label><input type="radio" checked={mode==='interactive'} onChange={()=>setMode('interactive')}/>{t(continuationText!==undefined?'launchPanel.modeInteractiveContinuation':'launchPanel.modeInteractiveBlank')}</label><label><input type="radio" checked={mode==='resume'} onChange={()=>setMode('resume')}/>{t('launchPanel.modeResume')}</label></fieldset>}
    <label className="field">{t('launchPanel.cwdLabel')}<input aria-label={t('launchPanel.cwdLabel')} value={cwd} onChange={e=>setCwd(e.target.value)} disabled={!!busy || !!result || uncertain} spellCheck={false} placeholder="/path/to/your/project"/><span className="small-print">{t('launchPanel.cwdNote',{host:hostAlias ?? t('app.localLabel')})}</span></label>
    {continuationText!==undefined && mode==='interactive' && <label className="field">{t('launchPanel.finalDraftLabel')}<textarea className="context-editor" aria-label={t('launchPanel.finalDraftLabel')} value={text} onChange={e=>setText(e.target.value)} disabled={!!busy || !!result || uncertain} spellCheck={false}/><span className="small-print">{t('launchPanel.finalDraftNote')}</span></label>}
    {error && <RequestFailure error={error} context={t('launchPanel.context')}/>}
    {hostAlias && mode==='resume' && <Notice tone="warning">{t('launchPanel.resumePassphraseNotice')}</Notice>}
    {!result && !uncertain && <button disabled={!!busy || !cwd.trim().startsWith('/') || (continuationText!==undefined && mode==='interactive' && !text.trim())} onClick={()=>void preview()}>{busy==='preview'?t('launchPanel.checkingTarget'):t('launchPanel.checkTarget')}<Icon name="arrow" size={15}/></button>}
    {plan && <section className="launch-review"><h3>{t(mode==='resume'?'launchPanel.resumePreviewTitle':'launchPanel.previewTitle')}</h3><dl><div><dt>{t('launchPanel.configRoot')}</dt><dd><code>{plan.launch_request?.config_root ?? plan.resume?.config_root ?? environment.root}</code></dd></div><div><dt>{t('launchPanel.projectCwd')}</dt><dd><code>{plan.launch_request?.project_cwd ?? plan.resume?.project_cwd ?? cwd}</code></dd></div><div><dt>{t('launchPanel.client')}</dt><dd><code>{plan.launch_request?.executable ?? plan.resume?.executable ?? environment.executable ?? t('launchPanel.notFound')}</code></dd></div><div><dt>{t('launchPanel.staticVersion')}</dt><dd>{plan.launch_request?.client_version ?? plan.resume?.client_version ?? t('launchPanel.unknownVersion')}</dd></div></dl>{plan.resume && <><Notice tone={plan.resume.supported?'neutral':'warning'}><strong>{t(plan.resume.supported?'launchPanel.resumeSupported':'launchPanel.resumeUnsupported')}</strong><p>{presentText(plan.resume.reason)}</p><p>{t('launchPanel.resumeUnverified')}</p></Notice><div className="fact-row"><span>{t('launchPanel.privateCopy')}</span><code>{plan.resume.private_copy_path}</code></div><details><summary>{t('launchPanel.clientSupport')}</summary><pre>{JSON.stringify(plan.resume.client_support,null,2)}</pre></details></>}
      {plan.resume?.write_scope && <Notice tone="warning"><strong>{t('launchPanel.resumeWriteScope')}</strong><p>{presentText(plan.resume.write_scope.note)}</p><code>{plan.resume.write_scope.config_root}</code><p>{t('launchPanel.resumeWriteProject',{path:plan.resume.write_scope.project_cwd ?? plan.resume.project_cwd ?? ''})}</p></Notice>}
      <StartupSources inspection={startup}/>
      <Notice>{t('launchPanel.terminalNotice')}</Notice>
      {!result && !uncertain && <><label className="review-checkbox"><input type="checkbox" checked={reviewed} disabled={!!busy} onChange={e=>setReviewed(e.target.checked)}/>{t('launchPanel.reviewCheck')}</label><button className="primary" disabled={!!busy || !reviewed || !sourcesChecked || plan.resume?.supported===false} onClick={()=>void open()}>{busy==='launch'?t('launchPanel.processing'):t(mode==='resume'?'launchPanel.approveResume':continuationText!==undefined?'launchPanel.copyAndOpen':'launchPanel.approveAndOpen')}<Icon name="arrow" size={15}/></button></>}
      <p className="launch-original-id">{t('launchPanel.originalRequest')} <code>{plan.launch_request?.id ?? plan.id}</code></p>
    </section>}
    {(copied || result || uncertain) && <section className="launch-result" role="status"><h3>{t('launchPanel.resultTitle')}</h3>{continuationText!==undefined && mode==='interactive' && <p>{t('launchPanel.copyResult',{state:copied?t('launchPanel.copyDone'):t('launchPanel.copyNotDone')})}</p>}<p>{t('launchPanel.launchState')}{result?<><Status value={result.status}/>{result.message ?? result.error?.message ?? t('launchPanel.launchReadBack')}</>:uncertain?t('launchPanel.launchUncertain'):t('launchPanel.launchNotRequested')}</p><p>{t('launchPanel.pasteSend')}</p>{uncertain && <button disabled={!!busy} onClick={()=>void open(true)}>{t('launchPanel.queryOriginal')}</button>}{result?.status==='planned' && result.observed==='plan' && <button disabled={!!busy} onClick={repreview}>{t('launchPanel.recheckTarget')}</button>}</section>}
  </div>;
}
