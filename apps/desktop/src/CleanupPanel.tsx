import { useEffect, useRef, useState } from 'react';
import { request } from './api';
import Clawd from './Clawd';
import { readTaskDraft, saveTaskDraft } from './taskDrafts';
import { ResourceLink } from './Resources';
import ServiceControl from './ServiceControl';
import type { Api, AuthenticationInventory, CleanupInspection, Environment, Plan } from './types';
import { Icon, Notice, Status, WorkspaceActions } from './ui';
import RequestFailure, { asError } from './RequestFailure';
import { useI18n } from './i18n-provider';
import { AuthenticationAuxiliary, AuthenticationDisclosureSummary, AuthenticationSelection, authenticationMethodLabel } from './AuthenticationSources';
import { LocalizedError } from './i18n';

type Recipe = 'repair_login' | 'reset_client' | 'retire';
const recipeKeys: Record<Recipe, { title: string; description: string }> = {
  repair_login: { title: 'cleanup.repair', description: 'cleanup.repairDetail' },
  reset_client: { title: 'cleanup.reset', description: 'cleanup.resetDetail' },
  retire: { title: 'cleanup.retire', description: 'cleanup.retireDetail' },
};
const classDefs = [['instructions','category.instructions','cleanup.classInstructions'],['memory','category.memory','cleanup.classMemory'],['sessions','category.sessions','cleanup.classSessions']] as const;
type CleanupSeed = { key: string; intent: { recipe: Recipe; categories: string[]; activateInstructions: boolean } };
export default function CleanupPanel({ send = request, environment, refreshKey, onPlan, onBrowser, onWork, hostAlias = null, seed }: {hostAlias?:string|null; send?: typeof request; environment: Environment; refreshKey?: number; onPlan: (plan: Plan) => void; onBrowser: () => void; onWork: () => void; seed?: CleanupSeed | null }) {
  const { t } = useI18n();
  const draftKey='lintel.cleanup-draft:'+JSON.stringify([hostAlias,environment.id]);
  const initial=readTaskDraft(draftKey,{recipe:'repair_login',categories:['instructions','memory','sessions'],activateInstructions:false});
  const [recipe, setRecipe] = useState<Recipe>(['repair_login','reset_client','retire'].includes(initial.recipe)?initial.recipe as Recipe:'repair_login');
  const [categories, setCategories] = useState(initial.categories);
  const [inspection, setInspection] = useState<CleanupInspection | null>(null);
  const [inventory,setInventory]=useState<AuthenticationInventory|null>(null);
  const [selectedSources,setSelectedSources]=useState<string[]>([]);
  const [project,setProject]=useState(''),[inspectedProject,setInspectedProject]=useState('');
  const [stopped, setStopped] = useState(false);
  const [verifyRuntime,setVerifyRuntime]=useState(false);
  const [auth, setAuth] = useState<Api['auth_probe']['response'] | null>(null);
  const [busy, setBusy] = useState('');
  const [error, setError] = useState<Error|null>(null);
  const [activateInstructions,setActivateInstructions]=useState(initial.activateInstructions);
  const generation=useRef(0),actionLock=useRef(false),inventorySeen=useRef(false);
  // Source IDs, observations and confirmation never enter the saved intent.
  useEffect(()=>saveTaskDraft(draftKey,{recipe,categories,activateInstructions}),[draftKey,recipe,categories,activateInstructions]);
  useEffect(()=>{if(!seed)return;setRecipe(seed.intent.recipe);setCategories(seed.intent.categories);setActivateInstructions(seed.intent.activateInstructions);setError(null);},[seed?.key]);
  async function run<T>(name:string,action:()=>Promise<T>,accept:(value:T)=>void) {
    if(actionLock.current)return;actionLock.current=true;const token=++generation.current;setBusy(name);setError(null);
    try{const result=await action();if(generation.current===token)accept(result);}
    catch(err){if(generation.current===token)setError(asError(err));}
    finally{if(generation.current===token){actionLock.current=false;setBusy('');}}
  }
  function inspect(cwd:string) {
    setInventory(null);setInspection(null);setAuth(null);
    return run('inspect',async()=>{
      // A metadata or writer-check failure must not hide the bounded source
      // inventory. Preview remains blocked until both observations succeed.
      const [files,sources]=await Promise.allSettled([
        send('cleanup_inspect',{environment_id:environment.id}),
        send('auth_inventory',{environment_id:environment.id,...(cwd?{project_cwd:cwd}:{})}),
      ]);
      return {files,sources};
    },({files,sources})=>{
      if(files.status==='fulfilled')setInspection(files.value);else setError(asError(files.reason));
      if(sources.status==='rejected'){setError(asError(sources.reason));return;}
      const observed=sources.value;
      if((observed.environment_id&&observed.environment_id!==environment.id)||(observed.context?.selected_config_directory&&observed.context.selected_config_directory!==environment.root))throw new LocalizedError('component.targetMismatch');
      setInventory(observed);setInspectedProject(cwd);
      const wasSeen=inventorySeen.current;
      setSelectedSources(current=>wasSeen?current.filter(id=>observed.sources.some(source=>source.id===id&&source.removable)):observed.sources.filter(source=>source.removable&&source.default_selected===true&&source.scope==='selected_root').map(source=>source.id));
      inventorySeen.current=true;
    });
  }
  useEffect(()=>{void inspect('');return()=>{generation.current++;actionLock.current=false;};},[environment.id,environment.root,hostAlias,refreshKey]);
  async function preview() {
    await run('plan',()=>send('plan_cleanup',{
      environment_id:environment.id,recipe,writers_confirmed_stopped:stopped,official_logout:false,categories,
      auth_source_ids:selectedSources,verify_auth_status:verifyRuntime,...(inspectedProject?{project_cwd:inspectedProject}:{}),
      activate:{instructions:activateInstructions&&categories.includes('instructions')},
    }),onPlan);
  }
  const sharedSelected=inventory?.sources.some(source=>source.scope==='host_shared'&&selectedSources.includes(source.id));
  const projectChanged=project.trim()!==inspectedProject;
  const probeAvailable=inspection?.auth_probe_available??!!environment.executable;
  const verificationOnly=recipe==='repair_login'&&!selectedSources.length;
  const writersActive=!!inspection?.writers.length;
  const footerRequirement=busy?t('cleanup.authGateBusy')
    :environment.status==='retired'?t('cleanup.authGateRetired')
    :!inventory?t('cleanup.authGateSources')
    :!inspection?t('cleanup.authGateFiles')
    :writersActive?t('cleanup.authGateWriters')
    :!stopped?t('cleanup.authGateWritersStopped')
    :projectChanged?t('cleanup.authGateProject')
    :recipe!=='repair_login'&&categories.length===0?t('cleanup.authGateCategories')
    :undefined;
  const previewDisabled=!!busy||!stopped||!inspection||!inventory||projectChanged||!!inspection?.writers.length||(recipe!=='repair_login'&&categories.length===0)||environment.status==='retired';
  const steps=recipe==='repair_login'?[t('cleanup.stepRecheckWriters'),t(verificationOnly?'cleanup.authStepVerifyOnly':'cleanup.authStepRemove'),t('cleanup.authStepVerify')]:[t('cleanup.authStepArchive'),...(recipe==='reset_client'?[t('cleanup.stepNewEnv')]:[]),t('cleanup.authStepRemove'),...(recipe==='retire'?[t('cleanup.stepRetire')]:[]),t('cleanup.authStepVerify')];
  const auxiliary=<>
    <details className="authentication-disclosure authentication-project" open={!inventory&&!!error}><AuthenticationDisclosureSummary>{t('cleanup.authProjectLabel')}</AuthenticationDisclosureSummary><label className="field">{t('cleanup.authProjectLabel')}<input value={project} onChange={event=>setProject(event.target.value)} disabled={!!busy} placeholder={t('component.projectPlaceholder')}/><span className="small-print">{t('cleanup.authProjectNote')}</span></label></details>
    {inspection&&<details className="authentication-disclosure authentication-legacy-metadata"><AuthenticationDisclosureSummary>{t('cleanup.authLegacyMetadata')}</AuthenticationDisclosureSummary><div className="cleanup-files">{inspection.files.map(file=><div className="cleanup-file" key={file.path}><details className="authentication-disclosure cleanup-file-path"><AuthenticationDisclosureSummary><span>{t(file.category==='credentials'?'cleanup.localCredential':'cleanup.clientState')}</span><code>{file.path.split('/').pop()||file.path}</code></AuthenticationDisclosureSummary><code className="cleanup-file-full-path">{file.path}</code></details><Status value={file.present?'discovered':'not_present'}/></div>)}</div></details>}
  </>;
  return <>
    <div className="page-heading"><div><span className="eyebrow">02 / a fresh start</span><h1>{t('cleanup.title')}</h1><p>{t('cleanup.subtitle')}</p></div><Clawd small mood="pack"/></div>
    {error&&<RequestFailure error={error} context={t('nav.rebuild')}/>}
    <fieldset className="recipe-grid authentication-recipes"><legend className="sr-only">{t('cleanup.recipeLegend')}</legend>{(Object.keys(recipeKeys) as Recipe[]).map(id=><label className={`recipe ${recipe===id?'chosen':''}`} key={id}><input type="radio" name="cleanup-recipe" checked={recipe===id} onChange={()=>{setRecipe(id);setError(null);}}/><span><strong>{t(recipeKeys[id].title)}</strong><small>{t(recipeKeys[id].description)}</small></span></label>)}</fieldset>
    <section className={`surface cleanup-scope ${recipe==='repair_login'?'primary':''}`} aria-label={t('cleanup.authInventoryTitle')}>
      <div className="authentication-section-heading"><div><span className="eyebrow">{t('cleanup.authInventoryEyebrow')}</span><h2>{t('cleanup.authInventoryTitle')}</h2><p>{t('cleanup.authInventoryIntro')}</p></div><button className="text-button" disabled={!!busy} onClick={()=>void inspect(project.trim())}><Icon name="refresh" size={14}/>{t(busy==='inspect'?'cleanup.authInventoryScanning':'cleanup.authInventoryAction')}</button></div>
      <p className="authentication-scope-note"><Icon name="history" size={14}/><span>{t('cleanup.authInventoryScope')}</span></p>
      <div className="padded">
        {!inventory?<><p role="status" className="muted">{t(busy==='inspect'?'cleanup.authScanning':'cleanup.authScanNeeded')}</p><AuthenticationAuxiliary open={!!error}>{auxiliary}</AuthenticationAuxiliary></>:<>
          <AuthenticationSelection inventory={inventory} selected={selectedSources} disabled={!!busy} auxiliary={auxiliary} onRestoreRecommended={()=>setSelectedSources(inventory.sources.filter(source=>source.removable&&source.default_selected===true&&source.scope==='selected_root').map(source=>source.id))} onToggle={(id,checked)=>setSelectedSources(current=>checked?[...current.filter(value=>value!==id),id]:current.filter(value=>value!==id))}/>
          {sharedSelected&&<Notice tone="warning">{t('cleanup.authSharedImpact')}</Notice>}
          {projectChanged&&<Notice>{t('cleanup.authRefreshProject')}</Notice>}
        </>}
        {writersActive&&<Notice tone="warning">{t('cleanup.writers',{writers:inspection.writers.map(writer=>`${writer.name} (${writer.pid})`).join(t('list.separator'))})}</Notice>}
      </div>
    </section>
    <details className="authentication-disclosure authentication-process"><AuthenticationDisclosureSummary>{t('cleanup.authProcessDetails')}</AuthenticationDisclosureSummary><div className="rebuild-split"><section className="surface"><div className="surface-heading"><h2>{t('cleanup.willHandle')}</h2><Icon name="rebuild" size={17}/></div><div className="padded"><ol className="process-list">{steps.map((item,index)=><li key={item}><span>{String(index+1).padStart(2,'0')}</span><div><strong>{item}</strong></div></li>)}</ol></div></section><section className="surface"><div className="surface-heading"><h2>{t('cleanup.willKeep')}</h2><Icon name="check" size={17}/></div><div className="padded"><ul className="keep-list"><li>{t('cleanup.keep1')}</li><li>{t('cleanup.authKeepConfig')}</li><li>{t('cleanup.authKeepSecrets')}</li><li>{t('cleanup.keep4')}</li></ul><p className="small-print">{t('cleanup.keepNote')}</p></div></section></div>
    <div className="inline-route"><span>{t('cleanup.onlyWork')}</span><button className="text-button" onClick={onWork}>{t('cleanup.goWork')}</button></div>
    </details>
    <ServiceControl send={send} environment={environment} onPlan={onPlan}/>
    <section className="surface" aria-label={t('cleanup.confirmTitle')}><div className="surface-heading"><h2>{t('cleanup.confirmTitle')}</h2><span className="small-label">{environment.name}</span></div>
      <label className="checkbox-row"><span><strong>{t('cleanup.writersStopped')}</strong><span>{t('cleanup.writersStoppedDetail')}</span></span><input type="checkbox" checked={stopped} onChange={event=>setStopped(event.target.checked)}/></label>
      <label className="checkbox-row"><span><strong>{t('cleanup.authVerifyRuntime')}</strong><span>{t('cleanup.authRuntimeOptInDetail')}</span></span><input type="checkbox" checked={verifyRuntime} disabled={!probeAvailable} onChange={event=>setVerifyRuntime(event.target.checked)}/></label>
      <details className="padded authentication-disclosure authentication-runtime-details"><AuthenticationDisclosureSummary>{t('cleanup.authRuntimeDetails')}</AuthenticationDisclosureSummary><button disabled={!!busy||!probeAvailable} onClick={()=>void run('auth',()=>send('auth_probe',{environment_id:environment.id}),setAuth)}>{t('cleanup.authProbe')}</button><p className="small-print authentication-context-note">{t('cleanup.authRuntimeScope')}</p>
        {auth&&<Notice tone={auth.logged_in?'warning':'neutral'}><strong>{t(auth.logged_in?'cleanup.authRuntimeStillLoggedIn':'cleanup.authRuntimeLoggedOut',{method:authenticationMethodLabel(auth.auth_method,t)})}</strong><p>{t('cleanup.authProbeNote')}</p></Notice>}
        <p className="small-print">{t('cleanup.authLogoutCollateral')}</p>
      </details>
    </section>
    <p className="small-print">{t('cleanup.authPersistentBoundary')}</p>
    {recipe!=='repair_login'&&<section className="surface work-selection"><div className="surface-heading"><h2>{t(recipe==='retire'?'cleanup.archiveWhich':'cleanup.importWhich')}</h2><span className="small-label">{t('cleanup.beforeExecPassphrase')}</span></div>{classDefs.map(([id,titleKey,detailKey])=><label className="checkbox-row" key={id}><span><strong>{t(titleKey)}</strong><span>{t(detailKey)}</span></span><input type="checkbox" checked={categories.includes(id)} onChange={event=>setCategories(current=>event.target.checked?[...current,id]:current.filter(value=>value!==id))}/></label>)}{recipe==='reset_client'&&<label className="checkbox-row"><span><strong>{t('cleanup.activateInstructions')}</strong><span>{t('cleanup.activateDetail')}</span></span><input type="checkbox" disabled={!categories.includes('instructions')} checked={activateInstructions} onChange={event=>setActivateInstructions(event.target.checked)}/></label>}</section>}
    {recipe!=='repair_login'&&categories.length===0&&<Notice>{t('cleanup.selectOne')}</Notice>}
    <div className="inline-route"><ResourceLink resource="cleanup-guide">{t('cleanup.guide')}</ResourceLink></div><div className="inline-route"><span>{t('cleanup.browserPrompt')}</span><button className="text-button" onClick={onBrowser}>{t('cleanup.browserSpace')}<Icon name="arrow" size={14}/></button></div>
    <WorkspaceActions><div className="authentication-footer-scope"><strong>{environment.name}</strong><span>{footerRequirement??t(verificationOnly?'cleanup.authVerificationFooter':'cleanup.footerScope')}</span></div><button className="primary" disabled={previewDisabled} onClick={()=>void preview()}>{busy==='plan'?t('cleanup.previewing'):t(verificationOnly?'cleanup.authPreviewVerification':'cleanup.preview')}<Icon name="arrow" size={16}/></button></WorkspaceActions>
  </>;
}
