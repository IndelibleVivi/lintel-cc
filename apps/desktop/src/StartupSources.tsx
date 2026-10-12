import type { StartupInspection, StartupSource } from './types';
import { Notice } from './ui';
import { useI18n } from './i18n-provider';
import { present } from './i18n-presentation';

const scopeKeys:Record<string,string>={user:'startup.scope.user',user_instructions:'startup.scope.userInstructions',user_rules:'startup.scope.userRules',user_agents:'startup.scope.userAgents',user_skills:'startup.scope.userSkills',managed_candidate:'startup.scope.managedCandidate',credential_metadata:'startup.scope.credentialMetadata',config_global_state_metadata:'startup.scope.configGlobalState',shared_global_state_metadata:'startup.scope.sharedGlobalState',shared_profile_metadata:'startup.scope.sharedProfile',project:'startup.scope.project',project_local:'startup.scope.projectLocal',project_mcp:'startup.scope.projectMcp',project_instructions:'startup.scope.projectInstructions',project_local_instructions:'startup.scope.projectLocalInstructions',project_agents_instructions:'startup.scope.projectAgents',project_rules:'startup.scope.projectRules',ancestor_candidate:'startup.scope.ancestorCandidate',managed_file:'startup.scope.managedFile',managed_mcp:'startup.scope.managedMcp',managed_instructions:'startup.scope.managedInstructions',managed_drop_in_directory:'startup.scope.managedDropInDir',managed_drop_in:'startup.scope.managedDropIn'};
const stateKeys:Record<string,string>={observed:'startup.state.observed',observed_directory:'startup.state.observedDirectory',not_found:'startup.state.notFound',unsupported_type:'startup.state.unsupportedType',access_limited:'startup.state.accessLimited'};
const declarationKeys:Record<string,string>={hooks:'startup.decl.hooks',MCP:'startup.decl.mcp','credential helper':'startup.decl.credentialHelper','policy helper':'startup.decl.policyHelper',plugins:'startup.decl.plugins'};

function SourceRow({source}:{source:StartupSource}) {
  const { t, locale } = useI18n();
  const presentText = (value: string | null | undefined) => present(value, locale);
  const declarations=[source.hooks_declared?'hooks':null,source.mcp_declared?'MCP':null,source.api_key_helper_declared?'credential helper':null,source.policy_helper_declared?'policy helper':null,source.plugins_declared?'plugins':null,...(source.auth_selectors_declared ?? [])].filter(Boolean) as string[];
  return <li><div><span>{scopeKeys[source.scope] ? t(scopeKeys[source.scope]) : source.scope}</span><small>{stateKeys[source.state] ? t(stateKeys[source.state]) : t('app.unknown')}</small></div><code>{source.path}</code>{declarations.length>0 && <p>{t('startup.hasDeclaration',{items:declarations.map(d=>declarationKeys[d]?t(declarationKeys[d]):d).join(' · ')})}</p>}{source.content_state==='unknown' && <p>{t('startup.contentUnknown')}</p>}{source.content_state==='limit' && <p>{t('startup.contentLimit')}</p>}</li>;
}
export default function StartupSources({inspection}:{inspection?:StartupInspection}) {
  const { t, locale } = useI18n();
  const presentText = (value: string | null | undefined) => present(value, locale);
  if(inspection?.schema!=='lintel.startup/1')return <Notice tone="warning">{t('startup.noRunner')}</Notice>;
  const visible=inspection.sources.filter(s=>s.state!=='not_found');
  const absent=inspection.sources.filter(s=>s.state==='not_found');
  const variables=inspection.authentication.runner_environment.filter(v=>v.present);
  return <section className="startup-sources" aria-label={t('startup.aria')}><h3>{t('startup.title')}</h3><p>{t('startup.intro')}</p>
    {(!inspection.candidate_scan_complete || inspection.content_limited) && <Notice tone="warning">{t('startup.partial')}</Notice>}
    {visible.length>0?<ul className="startup-source-list">{visible.map(s=><SourceRow key={s.path} source={s}/>)}</ul>:<p>{t('startup.noneFound')}</p>}
    {absent.length>0 && <details><summary>{t('startup.absentSummary',{count:absent.length})}</summary><ul className="startup-source-list">{absent.map(s=><SourceRow key={s.path} source={s}/>)}</ul></details>}
    <div className="startup-auth"><strong>{t('startup.authTitle')}</strong><p>{t('startup.authBody')}</p>{variables.length>0 && <p>{t('startup.authEnv',{names:variables.map(v=>v.name).join(' · ')})}</p>}</div>
    <details><summary>{t('startup.unknownSources')}</summary><ul>{inspection.unknown_sources.map(s=><li key={s}>{presentText(s)}</li>)}</ul></details>
    <ol className="startup-next-steps">{inspection.next_steps.map(s=><li key={s}>{presentText(s)}</li>)}</ol>
  </section>;
}
