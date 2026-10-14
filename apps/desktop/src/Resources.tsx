import { useState, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { transport } from './api';
import { useI18n } from './i18n-provider';

import './help.css';
import resourceTable from '../../../contracts/documentation-resources.json';
import { buildIdentity } from './buildIdentity';
import taskCatalog from '../../../contracts/task-catalog.json';
const ownDocsPrefix = `${resourceTable.source}/blob/main/docs/`;
const resources = Object.fromEntries(Object.entries(resourceTable).map(([key,url]) => [key, buildIdentity.docs_revision === 'unknown' ? url : url.replace(ownDocsPrefix, `${resourceTable.source}/blob/${buildIdentity.docs_revision}/docs/`)])) as typeof resourceTable;

export function ResourceLink({ resource, children }: { resource: keyof typeof resources; children: ReactNode }) {
  const { t } = useI18n();
  const [error, setError] = useState(false);
  return <span className="resource-link"><a href={resources[resource]} target="_blank" rel="noreferrer" onClick={event => {
    if (transport !== 'native') return;
    event.preventDefault(); setError(false);
    void invoke('open_resource', { resource }).catch(() => setError(true));
  }}>{children}<span aria-hidden="true"> ↗</span>{resources[resource].startsWith(ownDocsPrefix) ? <small className="resource-version">{t('resources.latestDev')}</small> : resources[resource].includes('/blob/main/docs/') && <small className="resource-version">{t('resources.latestGuide')}</small>}</a>{error && <span className="resource-error" role="alert">{t('resources.openFailed')}<code>{resources[resource]}</code></span>}</span>;
}

export default function DeveloperResources() {
  const { t } = useI18n();
  return <section className="developer-resources separated" aria-label={t('resources.aria')}><h3>{t('resources.title')}</h3><p>{t('resources.intro')}</p><div className="resource-links"><ResourceLink resource="developer">{t('resources.developer')}</ResourceLink><ResourceLink resource="source">{t('resources.source')}</ResourceLink></div><p className="small-print">{t('resources.note')}</p><h4>{t('resources.vpsTitle')}</h4><p>{t('resources.vpsIntro')}</p><div className="resource-links"><ResourceLink resource="vps-guide">{t('resources.vpsGuideRepo')}</ResourceLink><ResourceLink resource="vps-basics">{t('resources.vpsBasics')}</ResourceLink><ResourceLink resource="ssh-troubleshooting">{t('resources.connTroubleshooting')}</ResourceLink></div><p className="small-print">{t('resources.linkNote')}</p></section>;
}

export const productTasks=taskCatalog.tasks;
// Localized task help. Labels/summaries come from the additive `label_en` /
// `summary_en` metadata when present, falling back to the shared Chinese
// catalog wording (never translated on the fly).
export function useTaskHelp() {
  const { t, locale } = useI18n();
  const english = locale === 'en';
  const label = (task: typeof productTasks[number]) => english && 'label_en' in task && task.label_en ? task.label_en as string : task.label;
  const summary = (task: typeof productTasks[number]) => english && 'summary_en' in task && task.summary_en ? task.summary_en as string : task.summary;
  const actionKeys:Record<string,string>={
    reduce_egress:'resources.action.protect', preserve_work:'resources.action.work', repair_cleanup_retire:'resources.action.cleanup',
    browser_profile:'resources.action.browser', ssh_remote:'resources.action.remote', recover_results:'resources.action.recovery',
  };
  return { t, productTasks, label, summary, actionKey: (id: string) => actionKeys[id] ?? id };
}

export function TaskHelp({ onProtect, onWork, onCleanup, onBrowser, onRemote, onRecovery,onChooseTask,showGuides = true }: { onProtect: () => void; onWork: () => void; onCleanup: () => void; onBrowser: () => void; onRemote: () => void; onRecovery: () => void;onChooseTask?:(id:string)=>void;showGuides?:boolean }) {
  const { t, productTasks, label, summary, actionKey } = useTaskHelp();
  const actions:Record<string,()=>void>={
    reduce_egress:onProtect, preserve_work:onWork, repair_cleanup_retire:onCleanup,
    browser_profile:onBrowser, ssh_remote:onRemote, recover_results:onRecovery,
  };
  return <><h3>{t('resources.startFromGoal')}</h3><div className="help-tasks">{productTasks.map(task => <section key={task.id} data-task-id={task.id}><h4>{label(task)}</h4><p>{summary(task)}</p><button className="text-button" onClick={()=>{onChooseTask?.(task.id);actions[task.id]?.();}}>{t(actionKey(task.id))}<span aria-hidden="true"> →</span></button></section>)}</div>{showGuides && <div className="resource-links"><ResourceLink resource="operator-guide">{t('resources.operatorGuide')}</ResourceLink><ResourceLink resource="agent-guide">{t('resources.agentGuide')}</ResourceLink></div>}</>;
}
