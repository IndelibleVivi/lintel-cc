import { useEffect, useMemo, useRef, useState } from 'react';
import { request, requester } from './api';
import type { AuthenticationInventory, AuthenticationPlan, AuthenticationSource, AuthenticationVerification, Receipt } from './types';
import { Icon, Notice } from './ui';
import RequestFailure, { asError } from './RequestFailure';
import { useI18n } from './i18n-provider';
import { LocalizedError } from './i18n';
import './authentication.css';

const kindKeys:Record<string,string>={
  credential_file:'cleanup.authKindFile',credential_field:'cleanup.authKindCredential',client_state_field:'cleanup.authKindSavedKey',
  settings_env:'cleanup.authKindSettings',api_key_helper:'cleanup.authKindHelper',shell_assignment:'cleanup.authKindShell',
  shell_file:'cleanup.authKindShellFile',settings_source:'cleanup.authKindSettingsSource',process_env:'cleanup.authKindProcess',
  profile:'cleanup.authKindProfile',keychain_field:'cleanup.authKindKeychainLogin',keychain:'cleanup.authKindKeychain',external_context:'cleanup.authKindExternal',
};
const evidenceKeys:Record<string,string>={confirmed_present:'cleanup.authEvidencePresent',observed_context:'cleanup.authEvidenceContext',declared:'cleanup.authEvidenceDeclared',inaccessible:'cleanup.authEvidenceInaccessible',unknown:'cleanup.authEvidenceUnknown',not_found:'cleanup.authEvidenceNotFound'};
const valueStateKeys:Record<string,string>={empty:'cleanup.authValueEmpty',nonempty:'cleanup.authValueNonempty',non_string:'cleanup.authValueNonString'};
const scopeKeys:Record<string,string>={selected_root:'cleanup.authScopeRoot',host_shared:'cleanup.authScopeShared',current_process:'cleanup.authScopeProcess',project:'cleanup.authScopeProject',managed:'cleanup.authScopeManaged',external:'cleanup.authScopeExternal'};
const keychainReasonKeys:Record<string,string>={keychain_locked:'cleanup.authKeychainLocked',keychain_access_denied:'cleanup.authKeychainDenied',keychain_unavailable:'cleanup.authKeychainUnavailable',keychain_unsupported:'cleanup.authKeychainUnsupported',keychain_synthetic_unavailable:'cleanup.authKeychainSynthetic',keychain_no_login:'cleanup.authKeychainNoLogin',keychain_invalid_format:'cleanup.authKeychainFormat',keychain_too_large:'cleanup.authKeychainLarge'};
const reasonKeys:Record<string,string>={
  ...keychainReasonKeys,
  literal_assignment:'cleanup.authReasonLiteral',settings_declaration:'cleanup.authReasonSettings',helper_not_executed:'cleanup.authReasonHelper',
  known_claude_login_field:'cleanup.authReasonKnownField',metadata_only:'cleanup.authReasonMetadata',dynamic_or_complex_shell:'cleanup.authReasonDynamic',
  context_only:'cleanup.authReasonContext',provider_or_profile_selector:'cleanup.authReasonSelector',unreadable:'cleanup.authReasonUnreadable',
  file_too_large:'cleanup.authReasonLarge',unrecognized_format:'cleanup.authReasonFormat',not_found:'cleanup.authReasonAbsent',
  keychain_uninspected:'cleanup.authReasonKeychain',external_contexts:'cleanup.authReasonExternal',project_token_ignored:'cleanup.authReasonProjectToken',
  profile_shared:'cleanup.authReasonProfile',non_string_value:'cleanup.authReasonNonString',no_auth_declaration:'cleanup.authReasonNoDeclaration',
  invalid_profile_pointer:'cleanup.authReasonProfilePointer',source_limit:'cleanup.authReasonLimit',
};
const verificationKeys:Record<string,string>={removed:'cleanup.authRemoved',still_present:'cleanup.authStillPresent',changed:'cleanup.authChanged',unknown:'cleanup.authEvidenceUnknown',not_selected:'cleanup.authNotSelected'};
const retainedKeys:Record<string,string>={retained:'cleanup.authRetainedFile',retained_container:'cleanup.authRetainedDirectory',location_unknown:'cleanup.authRetainedLocationUnknown',unknown:'cleanup.authEvidenceUnknown'};
const methodKeys:Record<string,string>={none:'cleanup.authMethodNone','claude.ai':'cleanup.authMethodLogin',oauth_token:'cleanup.authMethodOauth',api_key:'cleanup.authMethodKey',api_key_helper:'cleanup.authMethodHelper',third_party:'cleanup.authMethodProvider'};
export function authenticationMethodLabel(method:string,t:(key:string)=>string):string {return methodKeys[method]?t(methodKeys[method]):method;}

/** Display only the core's finite public projection. No field values or helper text. */
export function AuthenticationSourceRow({source,selected,onToggle,disabled=false,expanded=false}:{source:AuthenticationSource;selected?:boolean;onToggle?:(id:string,selected:boolean)=>void;disabled?:boolean;expanded?:boolean}) {
  const {t}=useI18n();
  const title=['credential_field','keychain_field'].includes(source.kind)&&source.name==='claudeAiOauth'?t('cleanup.authSavedClaude'):source.name || source.locator || (kindKeys[source.kind]?t(kindKeys[source.kind]):source.kind);
  const label=source.removable?t('cleanup.authSelectSource',{name:title}):title;
  const valueStateKey=source.value_state&&Object.hasOwn(valueStateKeys,source.value_state)?valueStateKeys[source.value_state]:undefined;
  return <div className={`authentication-source ${selected?'selected':''}`} data-auth-source={source.id} data-auth-kind={source.kind} data-auth-reason={source.reason||undefined}>
    <div className="authentication-source-heading">
      <div><span className="small-label">{kindKeys[source.kind]?t(kindKeys[source.kind]):source.kind}</span><strong><code>{title}</code></strong></div>
      {onToggle&&source.removable&&<input type="checkbox" aria-label={label} checked={selected===true} disabled={disabled} onChange={event=>onToggle(source.id,event.target.checked)}/>}
    </div>
    {source.reason&&keychainReasonKeys[source.reason]&&<p className="small-print authentication-keychain-state">{t(keychainReasonKeys[source.reason])}</p>}
    <details className="authentication-source-details" open={expanded}><summary>{t('cleanup.authSourceDetails')}</summary>
    <div className="authentication-source-facts"><span className="authentication-evidence" data-evidence={source.evidence}>{evidenceKeys[source.evidence]?t(evidenceKeys[source.evidence]):source.evidence}</span><span>{scopeKeys[source.scope]?t(scopeKeys[source.scope]):source.scope}</span>{source.removable&&<span>{t('cleanup.authPreciselyRemovable')}</span>}</div>
    {valueStateKey&&<p className="small-print"><span className="authentication-evidence" data-value-state={source.value_state}>{t(valueStateKey)}</span> · {t('cleanup.authValueShapeOnly')}</p>}
    {source.service&&<code className="authentication-source-path">{source.service}</code>}
    {source.path&&<code className="authentication-source-path">{source.path}</code>}
    {source.locator&&source.locator!==source.name&&<code className="authentication-source-locator">{source.locator}</code>}
    {!!source.locations?.length&&<span className="small-print">{t('cleanup.authAtLines',{lines:source.locations.map(location=>location.line).join(', ')})}</span>}
    {source.reason&&!keychainReasonKeys[source.reason]&&<p className="small-print">{reasonKeys[source.reason]?t(reasonKeys[source.reason]):source.reason}</p>}
    {source.name==='CLAUDE_CODE_OAUTH_TOKEN'&&<p className="small-print">{t('cleanup.authSetupTokenNote')}</p>}
    </details>
  </div>;
}

export function AuthenticationInventoryView({inventory,selected,onToggle,disabled=false,compact=false}:{inventory:AuthenticationInventory;selected?:string[];onToggle?:(id:string,selected:boolean)=>void;disabled?:boolean;compact?:boolean}) {
  const {t}=useI18n();
  const visible=inventory.sources.filter(source=>source.presence!=='absent');
  const saved=visible.filter(source=>['credential_field','credential_file','client_state_field','keychain_field'].includes(source.kind)||(source.kind==='keychain'&&['keychain_locked','keychain_access_denied','keychain_unavailable','keychain_invalid_format','keychain_too_large'].includes(source.reason||'')));
  const injections=visible.filter(source=>!saved.includes(source)&&source.removable);
  const context=visible.filter(source=>!saved.includes(source)&&!source.removable);
  const absent=inventory.sources.filter(source=>source.presence==='absent');
  const row=(source:AuthenticationSource)=><AuthenticationSourceRow key={source.id} source={source} selected={selected?.includes(source.id)} onToggle={onToggle} disabled={disabled}/>;
  const unavailable=!!inventory.error_code||!inventory.context;
  const incomplete=!Array.isArray(inventory.limits)&&inventory.limits?.bounded_scan_complete===false;
  const inherited=inventory.context?.inherited_config_dir;
  return <div className={`authentication-inventory ${compact?'compact':''}`}>
    <p className="small-print">{t('cleanup.authInventoryScope')}</p>
    {unavailable&&<Notice tone="warning">{t('cleanup.authInventoryUnavailable')}{inventory.error_code&&<> · <code>{inventory.error_code}</code></>}</Notice>}
    {inherited==='different'&&<Notice tone="warning">{t('cleanup.authDifferentRoot')}</Notice>}
    {incomplete&&<Notice tone="warning">{t('cleanup.authScanIncomplete')}</Notice>}
    {!!saved.length&&<section className="authentication-group"><h3>{t('cleanup.authSavedGroup')}</h3><p className="small-print">{t('cleanup.authSavedGroupNote')}</p><div className="authentication-source-list">{saved.map(row)}</div></section>}
    {!!injections.length&&<section className="authentication-group"><div className="authentication-group-heading"><h3>{t('cleanup.authInjectionGroup')}</h3>{onToggle&&<div className="button-row"><button className="text-button" disabled={disabled} onClick={()=>injections.forEach(source=>onToggle(source.id,true))}>{t('cleanup.authSelectGroup')}</button><button className="text-button" disabled={disabled||!injections.some(source=>selected?.includes(source.id))} onClick={()=>injections.forEach(source=>onToggle(source.id,false))}>{t('cleanup.authClearGroup')}</button></div>}</div><p className="small-print">{t('cleanup.authInjectionGroupNote')}</p><div className="authentication-source-list">{injections.map(row)}</div></section>}
    {!!context.length&&<details className="authentication-context"><summary>{t('cleanup.authContextCount',{count:context.length})}</summary><div className="authentication-source-list">{context.map(row)}</div></details>}
    {!visible.length&&!unavailable&&<p>{t('cleanup.authNoneObserved')}</p>}
    {!!absent.length&&<details className="authentication-absent"><summary>{t('cleanup.authAbsentCount',{count:absent.length})}</summary><div className="authentication-source-list">{absent.map(row)}</div></details>}
  </div>;
}

export function AuthenticationPlanReview({authentication}:{authentication:AuthenticationPlan}) {
  const {t}=useI18n();
  return <section className="authentication-plan frozen-destination" aria-label={t('cleanup.authPlanTitle')}>
    <h4>{t('cleanup.authPlanTitle')}</h4><p>{t('cleanup.authSelectedCount',{count:authentication.selected_sources.length})}</p>
    <div className="authentication-source-list">{authentication.selected_sources.map(source=><AuthenticationSourceRow key={source.id} source={source} expanded/>)}</div>
    {!authentication.selected_sources.length&&<p>{t('cleanup.authNoSelection')}</p>}
    {authentication.selected_sources.some(source=>source.scope==='host_shared')&&<Notice tone="warning">{t('cleanup.authSharedImpact')}</Notice>}
    <p>{t(authentication.runtime_verification_requested?'cleanup.authRuntimeIncluded':'cleanup.authRuntimeNotIncluded')}</p>
    <p className="small-print">{t('cleanup.authNoSecretBackup')}</p><p className="small-print">{t('cleanup.authPersistentBoundary')}</p>
  </section>;
}

export function AuthenticationResult({receipt,hostAlias=null}:{receipt:Receipt;hostAlias?:string|null}) {
  const {t,formatDateTime}=useI18n();
  // This sender belongs to the original task's host, independent of the current
  // target selector. A late response cannot cross a task/host generation.
  const send=useMemo(()=>requester(hostAlias),[hostAlias]);
  const [fresh,setFresh]=useState<AuthenticationVerification|null>(null);
  const [busy,setBusy]=useState(false),[error,setError]=useState<Error|null>(null);
  const generation=useRef(0),lock=useRef(false);
  useEffect(()=>{generation.current++;setFresh(null);setError(null);setBusy(false);lock.current=false;return()=>{generation.current++;lock.current=false;};},[hostAlias,receipt.id]);
  const report=fresh??receipt.auth_verification;
  if(!report)return null;
  async function recheck(observeRuntime:boolean) {
    if(lock.current||!receipt.environment_id)return;
    lock.current=true;const token=++generation.current;setBusy(true);setError(null);
    try {
      const next=await send('auth_verify',{environment_id:receipt.environment_id,job_id:receipt.id,observe_runtime:observeRuntime});
      if((next.environment_id&&next.environment_id!==receipt.environment_id)||(next.original_job_id&&next.original_job_id!==receipt.id))throw new LocalizedError('component.targetMismatch');
      if(generation.current===token)setFresh(next);
    } catch(err){if(generation.current===token)setError(asError(err));}
    finally{if(generation.current===token){lock.current=false;setBusy(false);}}
  }
  const pending=['accepted','executing','verifying','pending'].includes(receipt.status);
  const observation=report.runtime.observation;
  const removed=report.local_sources.filter(source=>source.state==='removed').length;
  const remaining=report.local_sources.filter(source=>source.state==='still_present').length;
  const unresolved=report.local_sources.filter(source=>!['removed','still_present','not_selected'].includes(source.state)).length;
  return <section className="authentication-result" aria-label={t('cleanup.authResultTitle')}>
    <div className="surface-heading"><h4>{t('cleanup.authResultTitle')}</h4><span className="small-label">{t(fresh?'cleanup.authFreshCheck':'cleanup.authRecordedCheck')}</span></div>
    <p className="small-print">{formatDateTime(report.checked_at)} · {t('cleanup.authFreshDoesNotRewrite')}</p>
    {!!report.local_sources.length&&<div className="authentication-outcome" role="status"><strong>{t('cleanup.authRemovedSummary',{count:removed})}</strong>{remaining>0&&<span className="authentication-attention">{t('cleanup.authRemainingSummary',{count:remaining})}</span>}{unresolved>0&&<span className="authentication-attention">{t('cleanup.authPendingSummary',{count:unresolved})}</span>}</div>}
    {error&&<RequestFailure error={error} context={t('cleanup.authResultTitle')}/>}
    <div className="button-row authentication-recheck"><button disabled={busy||pending||!receipt.environment_id} onClick={()=>void recheck(false)}><Icon name="refresh" size={14}/>{t('cleanup.authRecheck')}</button><button disabled={busy||pending||!receipt.environment_id} onClick={()=>void recheck(true)}>{t('cleanup.authRecheckRuntime')}</button>{fresh&&<button className="text-button" disabled={busy} onClick={()=>{setFresh(null);setError(null);}}>{t('cleanup.authShowOriginal')}</button>}</div>
    <p className="small-print">{t('cleanup.authRuntimeOptInDetail')}</p>
    <div className="authentication-coverage">
      <section><details className="authentication-local-results"><summary>{t('cleanup.authLocalCoverage')}</summary><p>{t('cleanup.authLocalCoverageNote')}</p>
        {report.local_sources.length?<ul className="authentication-verified-sources">{report.local_sources.map(source=><li key={source.id}><span className="authentication-evidence" data-evidence={source.state}>{verificationKeys[source.state]?t(verificationKeys[source.state]):source.state}</span><div><code>{source.locator||source.kind}</code>{source.service&&<code className="authentication-source-path">{source.service}</code>}{source.path&&<code className="authentication-source-path">{source.path}</code>}</div></li>)}</ul>:<p>{t('cleanup.authNoSelection')}</p>}
      </details></section>
      <section><h4>{t('cleanup.authRuntimeCoverage')}</h4>
        {report.runtime.state==='observed'&&observation?<><p className={observation.logged_in?'authentication-attention':''}>{t(observation.logged_in?'cleanup.authRuntimeStillLoggedIn':'cleanup.authRuntimeLoggedOut',{method:authenticationMethodLabel(observation.auth_method,t)})}</p><details className="authentication-runtime-scope"><summary>{t('cleanup.authRuntimeDetails')}</summary><p className="small-print">{t('cleanup.authRuntimeScope')}</p>{observation.config_directory&&<code className="authentication-source-path">{observation.config_directory}</code>}</details></>:<p>{t(report.runtime.state==='not_requested'?'cleanup.authRuntimeNotChecked':'cleanup.authRuntimeUnavailable')}{report.runtime.error_code&&<> · <code>{report.runtime.error_code}</code></>}</p>}
      </section>
      <details className="authentication-secondary-coverage"><summary>{t('cleanup.authFurtherCoverage')}</summary><div className="authentication-coverage">
      <section><h4>{t('cleanup.authOtherCoverage')}</h4><p>{t('cleanup.authOtherUnverified')}</p></section>
      <section><h4>{t('cleanup.authServerCoverage')}</h4><p>{t('cleanup.authServerUnverified')}</p></section>
      </div></details>
    </div>
    {!!report.retained_sensitive_material.length&&<Notice tone="warning"><strong>{t('cleanup.authRetainedTitle')}</strong><p>{t('cleanup.authRetainedNote')}</p><ul>{report.retained_sensitive_material.map(item=><li key={item.path}><span>{retainedKeys[item.state]?t(retainedKeys[item.state]):item.state}</span><code className="authentication-source-path">{item.path}</code></li>)}</ul></Notice>}
    {report.inventory&&<details className="authentication-after-inventory"><summary>{t('cleanup.authRemainingSources')}</summary><AuthenticationInventoryView inventory={report.inventory} compact/></details>}
  </section>;
}

/** Read-only source inspection inside the existing Environment / Components tab. */
export function ComponentAuthentication({send=request,environmentId,project}:{send?:typeof request;environmentId:string;project:string}) {
  const {t}=useI18n();
  const [inventory,setInventory]=useState<AuthenticationInventory|null>(null),[busy,setBusy]=useState(false),[error,setError]=useState<Error|null>(null);
  const generation=useRef(0),lock=useRef(false);
  useEffect(()=>()=>{generation.current++;lock.current=false;},[]);
  async function inspect() {
    if(lock.current)return;lock.current=true;const token=++generation.current;setBusy(true);setError(null);
    try{const result=await send('auth_inventory',{environment_id:environmentId,...(project?{project_cwd:project}:{})});if(result.environment_id&&result.environment_id!==environmentId)throw new LocalizedError('component.targetMismatch');if(generation.current===token)setInventory(result);}
    catch(err){if(generation.current===token)setError(asError(err));}
    finally{if(generation.current===token){lock.current=false;setBusy(false);}}
  }
  return <div className="authentication-component"><p className="small-print">{t('cleanup.authComponentNote')}</p><button disabled={busy} onClick={()=>void inspect()}>{t(busy?'cleanup.authScanning':'cleanup.authInspect')}</button>{error&&<RequestFailure error={error} context={t('cleanup.authInventoryTitle')}/>} {inventory&&<AuthenticationInventoryView inventory={inventory} compact/>}</div>;
}
