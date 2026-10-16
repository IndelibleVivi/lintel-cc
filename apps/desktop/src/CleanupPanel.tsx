import { useEffect, useRef, useState } from 'react';
import { request, transport } from './api';
import Clawd from './Clawd';
import {readTaskDraft,saveTaskDraft} from './taskDrafts';
import { ResourceLink } from './Resources';
import ServiceControl from './ServiceControl';
import type { Api, CleanupInspection, Environment, Plan } from './types';
import { Icon, Notice, Status, WorkspaceActions } from './ui';
import RequestFailure, { asError } from './RequestFailure';
import { useI18n } from './i18n-provider';

type Recipe = 'repair_login' | 'reset_client' | 'retire';
const recipeKeys: Record<Recipe, { title: string; description: string }> = {
  repair_login: { title: 'cleanup.repair', description: 'cleanup.repairDetail' },
  reset_client: { title: 'cleanup.reset', description: 'cleanup.resetDetail' },
  retire: { title: 'cleanup.retire', description: 'cleanup.retireDetail' },
};
const classDefs = [['instructions','category.instructions','cleanup.classInstructions'],['memory','category.memory','cleanup.classMemory'],['sessions','category.sessions','cleanup.classSessions']] as const;
type CleanupSeed = { key: string; intent: { recipe: 'repair_login' | 'reset_client' | 'retire'; categories: string[]; activateInstructions: boolean } };
export default function CleanupPanel({ send = request, environment, refreshKey, onPlan, onBrowser, onWork, hostAlias = null, seed }: {hostAlias?:string|null; send?: typeof request; environment: Environment; refreshKey?: number; onPlan: (plan: Plan) => void; onBrowser: () => void; onWork: () => void; seed?: CleanupSeed | null }) {
  const { t } = useI18n();
  const draftKey='lintel.cleanup-draft:'+JSON.stringify([hostAlias,environment.id]);
  const initial=readTaskDraft(draftKey,{recipe:'repair_login',categories:['instructions','memory','sessions'],activateInstructions:false});
  const [recipe, setRecipe] = useState<Recipe>(['repair_login','reset_client','retire'].includes(initial.recipe)?initial.recipe as Recipe:'repair_login');
  const [categories, setCategories] = useState(initial.categories);
  const [inspection, setInspection] = useState<CleanupInspection | null>(null);
  const [stopped, setStopped] = useState(false);
  const [logout, setLogout] = useState(false);
  const [auth, setAuth] = useState<Api['auth_probe']['response'] | null>(null);
  const [busy, setBusy] = useState('');
  const [error, setError] = useState<Error|null>(null);
  const [activateInstructions,setActivateInstructions]=useState(initial.activateInstructions);
  const alive=useRef(true);const actionLock=useRef(false);
  useEffect(()=>{alive.current=true;return()=>{alive.current=false}},[]);
  useEffect(()=>saveTaskDraft(draftKey,{recipe,categories,activateInstructions}),[draftKey,recipe,categories,activateInstructions]);
  // A recipe seeds the current draft once for this host/environment generation.
  useEffect(()=>{ if(!seed)return; setRecipe(seed.intent.recipe); setCategories(seed.intent.categories); setActivateInstructions(seed.intent.activateInstructions); setError(null); },[seed?.key]);
  async function run(name: string, action: () => Promise<void>) { if(actionLock.current)return;actionLock.current=true;setBusy(name); setError(null); try { await action(); } catch (err) { if(alive.current)setError(asError(err)); } finally { actionLock.current=false;if(alive.current)setBusy(''); } }
  async function inspect() { const found=await send('cleanup_inspect', { environment_id: environment.id });if(alive.current)setInspection(found); }
  useEffect(() => { void run('inspect', inspect); }, [environment.id, refreshKey]);
  async function preview() { await run('plan', async () => { const plan = await send('plan_cleanup', { environment_id: environment.id, recipe, writers_confirmed_stopped: stopped, official_logout: logout, categories, activate:{instructions:activateInstructions && categories.includes("instructions")} }); if(alive.current)onPlan(plan); }); }
  return <>
    <div className="page-heading"><div><span className="eyebrow">02 / a fresh start</span><h1>{t('cleanup.title')}</h1><p>{t('cleanup.subtitle')}</p></div><Clawd small mood="pack"/></div>
    {error && <RequestFailure error={error} context={t('nav.rebuild')}/>}
    <fieldset className="recipe-grid"><legend className="sr-only">{t('cleanup.recipeLegend')}</legend>{(Object.keys(recipeKeys) as Recipe[]).map(id => <label className={`recipe ${recipe === id ? 'chosen' : ''}`} key={id}><input type="radio" name="cleanup-recipe" checked={recipe === id} onChange={() => { setRecipe(id); setError(null); }}/><span><strong>{t(recipeKeys[id].title)}</strong><small>{t(recipeKeys[id].description)}</small></span></label>)}</fieldset>
    <div className="rebuild-split"><section className="surface"><div className="surface-heading"><h2>{t('cleanup.willHandle')}</h2><Icon name="rebuild" size={17}/></div><div className="padded"><ol className="process-list">{(recipe === 'repair_login' ? [t('cleanup.stepRecheckWriters'), ...(logout ? [t('cleanup.stepLogout')] : []), t('cleanup.stepRemoveCreds')] : [t('cleanup.stepArchive'), ...(recipe === 'reset_client' ? [t('cleanup.stepNewEnv')] : []), ...(logout ? [t('cleanup.stepLogout')] : []), t('cleanup.stepRemoveState'), ...(recipe === 'retire' ? [t('cleanup.stepRetire')] : [])]).map((item,index) => <li key={item}><span>{String(index+1).padStart(2,'0')}</span><div><strong>{item}</strong></div></li>)}</ol></div></section><section className="surface"><div className="surface-heading"><h2>{t('cleanup.willKeep')}</h2><Icon name="check" size={17}/></div><div className="padded"><ul className="keep-list"><li>{t('cleanup.keep1')}</li><li>{t('cleanup.keep2')}</li><li>{t('cleanup.keep3')}</li><li>{t('cleanup.keep4')}</li></ul><p className="small-print">{t('cleanup.keepNote')}</p></div></section></div>
    <><div className="inline-route"><span>{t('cleanup.onlyWork')}</span><button className="text-button" onClick={onWork}>{t('cleanup.goWork')}</button></div>
      <ServiceControl send={send} environment={environment} onPlan={onPlan}/>
      <section className="surface"><div className="surface-heading"><h2>{t('cleanup.localScope')}</h2><button className="text-button" disabled={!!busy} onClick={() => void run('inspect', inspect)}><Icon name="refresh" size={14}/>{t('cleanup.refreshScope')}</button></div><div className="padded">{!inspection ? <p className="muted">{t('cleanup.checkingFiles')}</p> : <><div className="cleanup-files">{inspection.files.filter(file => recipe !== 'repair_login' || file.category === 'credentials').map(file => <div key={file.path}><span>{t(file.category === 'credentials' ? 'cleanup.localCredential' : 'cleanup.clientState')}</span><code>{file.path}</code><Status value={file.present ? 'discovered' : 'not_present'}/></div>)}</div><p className="small-print">{inspection.coverage}</p>{inspection.shared_profile_present && <Notice tone="warning">{t('cleanup.sharedProfile')}</Notice>}{inspection.writers.length > 0 && <Notice tone="warning">{t('cleanup.writers',{writers:inspection.writers.map(writer => `${writer.name} (${writer.pid})`).join(t('list.separator'))})}</Notice>}</>}</div></section>
      <section className="surface"><div className="surface-heading"><h2>{t('cleanup.confirmTitle')}</h2><span className="small-label">{t('cleanup.scopeLabel',{name:environment.name})}</span></div><label className="checkbox-row"><span><strong>{t('cleanup.writersStopped')}</strong><span>{t('cleanup.writersStoppedDetail')}</span></span><input type="checkbox" checked={stopped} onChange={event => setStopped(event.target.checked)}/></label><label className="checkbox-row"><span><strong>{t('cleanup.officialLogout')}</strong><span>{t('cleanup.officialLogoutDetail')}</span></span><input type="checkbox" checked={logout} disabled={transport !== 'native' || !inspection?.official_logout_available || inspection.shared_profile_present} onChange={event => setLogout(event.target.checked)}/></label><div className="padded"><button disabled={!!busy || transport !== 'native' || !inspection?.official_logout_available} onClick={() => void run('auth', async () => setAuth(await send('auth_probe', { environment_id: environment.id })))}>{t('cleanup.authProbe')}</button>{auth && <p className="small-print">{t('cleanup.authNote',{status:auth.logged_in ? t('cleanup.authLoggedIn') : t('cleanup.authLoggedOut'),source:auth.auth_method,identity:auth.identity_observed ? t('cleanup.authKnownIdentity') : auth.logged_in ? t('cleanup.authUnknownIdentity') : t('cleanup.authNoLogout'),revocation:auth.remote_revocation})}</p>}{auth?.logged_in && auth.identity_observed === false && <Notice tone="warning">{t('cleanup.authNoIdentity')}</Notice>}{transport !== 'native' && <p className="small-print">{t('cleanup.syntheticAuth')}</p>}</div></section>
      {!logout && <Notice tone="warning">{t('cleanup.localOnly')}</Notice>}
    </>
    {recipe !== 'repair_login' && <section className="surface work-selection"><div className="surface-heading"><h2>{t(recipe === 'retire' ? 'cleanup.archiveWhich' : 'cleanup.importWhich')}</h2><span className="small-label">{t('cleanup.beforeExecPassphrase')}</span></div>{classDefs.map(([id,titleKey,detailKey]) => <label className="checkbox-row" key={id}><span><strong>{t(titleKey)}</strong><span>{t(detailKey)}</span></span><input type="checkbox" checked={categories.includes(id)} onChange={event => setCategories(current => event.target.checked ? [...current,id] : current.filter(value => value !== id))}/></label>)}{recipe==='reset_client' && <label className="checkbox-row"><span><strong>{t('cleanup.activateInstructions')}</strong><span>{t('cleanup.activateDetail')}</span></span><input type="checkbox" disabled={!categories.includes('instructions')} checked={activateInstructions} onChange={event=>setActivateInstructions(event.target.checked)}/></label>}</section>}
    {recipe !== 'repair_login' && categories.length === 0 && <Notice>{t('cleanup.selectOne')}</Notice>}
    <div className="inline-route"><ResourceLink resource="cleanup-guide">{t('cleanup.guide')}</ResourceLink></div><div className="inline-route"><span>{t('cleanup.browserPrompt')}</span><button className="text-button" onClick={onBrowser}>{t('cleanup.browserSpace')}<Icon name="arrow" size={14}/></button></div>
    <WorkspaceActions><div><strong>{environment.name}</strong><span>{t('cleanup.footerScope')}</span></div><button className="primary" disabled={!!busy || (!stopped || !inspection || inspection.writers.length > 0) || (recipe !== 'repair_login' && categories.length === 0) || environment.status === 'retired'} onClick={() => void preview()}>{busy === 'plan' ? t('cleanup.previewing') : t('cleanup.preview')}<Icon name="arrow" size={16}/></button></WorkspaceActions>
  </>;
}
