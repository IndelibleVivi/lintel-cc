import { LocalizedError } from './i18n';
import { useEffect, useRef, useState } from 'react';
import { request } from './api';
import Clawd from './Clawd';
import WorkCapacity from './WorkCapacity';
import WorkFileSelection from './WorkFileSelection';
import {readTaskDraft,saveTaskDraft} from './taskDrafts';
import { ResourceLink } from './Resources';
import type { Environment, Inspection, Plan, WorkPreflight } from './types';
import { Icon, Notice, formatBytes, WorkspaceActions } from './ui';
import RequestFailure, { asError } from './RequestFailure';
import { useI18n } from './i18n-provider';

const classDefs = [['instructions','category.instructions','work.classInstructions'],['memory','category.memory','work.classMemory'],['sessions','category.sessions','work.classSessions']] as const;

export default function WorkPanel({ send = request, environment, inspection, onPlan, onArchives,hostAlias=null }: {hostAlias?:string|null; send?: typeof request; environment: Environment; inspection: Inspection | null; onPlan: (plan: Plan) => void; onArchives: () => void }) {
  const { t } = useI18n();
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
  // Path metadata is scoped to this host/environment interaction, not persisted
  // into ordinary drafts or reused after leaving the page.
  const [selectedPaths,setSelectedPaths]=useState<string[]|null>(null);
  const [selectionReady,setSelectionReady]=useState(true);
  const alive = useRef(true);const actionLock=useRef(false);
  useEffect(() => { alive.current=true; return () => {alive.current=false;}; }, []);
  useEffect(()=>saveTaskDraft(draftKey,{mode,categories,output,name,activateInstructions}),[draftKey,mode,categories,output,name,activateInstructions]);
  async function preview() {
    if(actionLock.current)return;actionLock.current=true;setBusy(true); setError(null);
    try {
      const fields = { environment_id:environment.id,categories,...(selectedPaths!==null?{selected_paths:selectedPaths}:{}) };
      const plan = mode === 'archive' ? await send('plan_archive',{...fields,...(output.trim()?{output_path:output.trim()}:{})}) : await send('plan_preserve',{...fields,activate:{instructions:activateInstructions && categories.includes('instructions')},...(name.trim()?{name:name.trim()}:{})});
      if(selectedPaths!==null&&(plan.work_selection?.mode!=='paths'||JSON.stringify([...(plan.work_selection.paths??[])].sort())!==JSON.stringify([...selectedPaths].sort())))throw new LocalizedError('workFiles.exactNotConfirmed');
      if (alive.current) onPlan(plan);
    } catch(err) { if(alive.current) setError(asError(err)); }
    finally { actionLock.current=false;if(alive.current) setBusy(false); }
  }
  return <>
    <div className="page-heading"><div><span className="eyebrow">work / keep what matters</span><h1>{t('work.title')}</h1><p>{t('work.subtitle')}</p></div><Clawd small mood="pack"/></div>
    {error && <RequestFailure error={error} context={t('nav.work')}/>}
    <fieldset className="recipe-grid work-modes" disabled={busy}><legend className="sr-only">{t('work.modeLegend')}</legend>{[['archive',t('work.archiveOnly'),t('work.archiveOnlyDetail')],['preserve',t('work.preserveNew'),t('work.preserveNewDetail')]].map(([id,title,detail]) => <label className={`recipe ${mode===id?'chosen':''}`} key={id}><input type="radio" name="work-mode" checked={mode===id} onChange={() => {setMode(id as typeof mode);setError(null);}}/><span><strong>{title}</strong><small>{detail}</small></span></label>)}</fieldset>
    <section className="surface work-selection"><div className="surface-heading"><h2>{t('work.keepWhat')}</h2><span className="small-label">{environment.name}</span></div>{classDefs.map(([id,titleKey,detailKey]) => {const asset=inspection?.assets.find(a=>a.category===id);return <label className="checkbox-row" key={id}><span><strong>{t(titleKey)}{asset&&<small> · {t('work.filesBytes',{count:asset.count,bytes:formatBytes(asset.bytes)})}</small>}</strong><span>{t(detailKey)}</span></span><input type="checkbox" disabled={busy} checked={categories.includes(id)} onChange={event=>setCategories(current=>event.target.checked?[...current,id]:current.filter(c=>c!==id))}/></label>;})}<div className="padded">{mode==='archive'?<label className="field">{t('work.outputLabel')}<input aria-label={t('work.outputLabel')} value={output} disabled={busy} onChange={event=>setOutput(event.target.value)} placeholder={t('work.outputPlaceholder')}/><span className="small-print">{t('work.outputNote')}</span></label>:<label className="field">{t('work.nameLabel')}<input value={name} disabled={busy} onChange={event=>setName(event.target.value)} placeholder={t('work.namePlaceholder')}/></label>}</div>{mode==='preserve' && <label className="checkbox-row"><span><strong>{t('work.activateInstructions')}</strong><span>{t('work.activateDetail')}</span></span><input type="checkbox" disabled={busy || !categories.includes('instructions')} checked={activateInstructions} onChange={event=>setActivateInstructions(event.target.checked)}/></label>}</section>
    <WorkFileSelection fileLimit={capacity?.limits.file_bytes} send={send} environment={environment} categories={categories} paths={selectedPaths} disabled={busy} onChange={setSelectedPaths} onReady={setSelectionReady}/>
    <WorkCapacity send={send} environment={environment} categories={categories} selectedPaths={selectedPaths} selectionReady={selectionReady} onResult={setCapacity}/>
    <Notice>{t('work.notice')}</Notice>
    <WorkspaceActions><button disabled={busy} onClick={onArchives}>{t('work.readPackages')}</button><button className="primary" disabled={busy||!categories.length||!selectionReady||(selectedPaths!==null&&!selectedPaths.length)||capacity?.eligible===false|| (mode==='archive'&&!!output.trim()&&!output.trim().startsWith('/'))} onClick={()=>void preview()}>{busy?t('work.previewing'):t('work.preview')}<Icon name="arrow" size={15}/></button></WorkspaceActions>
    <div className="inline-route"><ResourceLink resource="work-guide">{t('work.guide')}</ResourceLink></div>
  </>;
}
