// Browser-independent render evidence only. Interactive and visual acceptance
// remains auth_sources_ui_journey.mjs with a real headless browser.
import assert from 'node:assert/strict';
import {test,after} from 'node:test';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import os from 'node:os';
import {createServer} from '../apps/desktop/node_modules/vite/dist/node/index.js';
import React from '../apps/desktop/node_modules/react/index.js';
import {renderToStaticMarkup} from '../apps/desktop/node_modules/react-dom/server.node.js';
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const server=await createServer({root:path.join(repo,'apps/desktop'),configFile:false,cacheDir:path.join(os.tmpdir(),'lintel-auth-sources-vite'),server:{middlewareMode:true,hmr:false,watch:null,ws:false},appType:'custom'});
after(()=>server.close());
const {AuthenticationSourceRow,AuthenticationInventoryView,AuthenticationSelection,AuthenticationPlanReview,AuthenticationResult}=await server.ssrLoadModule('/src/AuthenticationSources.tsx');
const {LanguageContext}=await server.ssrLoadModule('/src/i18n-provider.tsx');
const {translate}=await server.ssrLoadModule('/src/i18n.ts');
const {zhCN}=await server.ssrLoadModule('/src/i18n.catalog.zh-CN.ts');
const {en}=await server.ssrLoadModule('/src/i18n.catalog.en.ts');
const catalogs={'zh-CN':zhCN,en};
function render(Component,props,locale='en'){
 const context={locale,setting:locale,matched:locale,preferred:[locale],setLanguage(){},t:(key,args)=>translate(catalogs,locale,key,args),formatDateTime:value=>String(value),formatTime:value=>String(value),formatNumber:value=>String(value),localeLabel:value=>value};
 return renderToStaticMarkup(React.createElement(LanguageContext.Provider,{value:context},React.createElement(Component,props)));
}
const secret='SYNTHETIC_VALUE_MUST_NOT_RENDER';
const sources=[
 {id:'auth-synthetic-credential',kind:'credential_field',name:'claudeAiOauth',path:'/synthetic/.claude/.credentials.json',locator:'/claudeAiOauth',scope:'selected_root',presence:'present',evidence:'confirmed_present',removable:true,reason:'known_claude_login_field',value:secret},
 {id:'auth-synthetic-shell',kind:'shell_assignment',name:'CLAUDE_CODE_OAUTH_TOKEN',path:'/synthetic/.zshrc',locator:'CLAUDE_CODE_OAUTH_TOKEN',scope:'host_shared',presence:'present',evidence:'declared',removable:true,reason:'literal_assignment',locations:[{line:3}],raw_line:secret},
 {id:'auth-synthetic-process',kind:'process_env',name:'ANTHROPIC_API_KEY',scope:'current_process',presence:'present',evidence:'observed_context',removable:false,reason:'context_only',value:secret},
 {id:'auth-synthetic-dynamic',kind:'shell_assignment',name:'ANTHROPIC_AUTH_TOKEN',path:'/synthetic/.bashrc',locator:'ANTHROPIC_AUTH_TOKEN',scope:'host_shared',presence:'present',evidence:'declared',removable:false,reason:'dynamic_or_complex_shell',command:secret},
 {id:'auth-synthetic-keychain',kind:'keychain',name:'keychain',scope:'external',presence:'unknown',evidence:'unknown',removable:false,reason:'keychain_uninspected'},
 {id:'auth-synthetic-absent',kind:'shell_file',name:'source_file',path:'/synthetic/.profile',scope:'host_shared',presence:'absent',evidence:'not_found',removable:false,reason:'not_found'},
];
const inventory={schema:'lintel.auth-sources/1',sources,context:{execution:'current_lintel_process',inherited_config_dir:'different',secret},limits:{bounded_scan_complete:false,coverage_complete:false}};
const authentication={schema:'lintel.auth-cleanup/1',selected_sources:sources.slice(0,2),local_source_count:2,context:inventory.context,runtime_verification_requested:true,boundaries:[]};
const report={schema:'lintel.auth-verification/1',checked_at:'2026-11-13T00:00:00Z',local_sources:[{...sources[0],state:'removed'},{...sources[1],state:'changed'}],inventory,runtime:{state:'observed',observation:{auth_method:'oauth_token',logged_in:true,email:secret,config_directory:'/synthetic/.claude'}},other_contexts:'unverified',server_revocation:'unverified',retained_sensitive_material:[{path:'/synthetic/.lintel-held/state',state:'retained'},{path:'/synthetic/last-known',state:'location_unknown'}],coverage:'selected_sources_only'};
const receipt={id:'11111111-1111-4111-8111-111111111111',plan_id:'11111111-1111-4111-8111-111111111111',environment_id:'22222222-2222-4222-8222-222222222222',status:'needs_reconciliation',title:'Synthetic cleanup',created_at:report.checked_at,steps:[],warnings:[],restorable:false,auth_verification:report};

test('source inventory renders evidence, exact shared location, unknowns and bounded limits in both languages without values',()=>{
 for(const locale of ['en','zh-CN']){
  const html=render(AuthenticationInventoryView,{inventory,selected:[sources[0].id],onToggle(){}},locale);
  assert.equal(html.includes(secret),false);assert.ok(html.includes('CLAUDE_CODE_OAUTH_TOKEN'));assert.ok(html.includes('/synthetic/.zshrc'));
  assert.equal((html.match(/type="checkbox"/g)||[]).length,2);assert.equal((html.match(/checked=""/g)||[]).length,1);
  assert.ok(html.includes(catalogs[locale]['cleanup.authEvidenceContext']));assert.ok(html.includes(catalogs[locale]['cleanup.authReasonDynamic']));
  assert.ok(html.includes(catalogs[locale]['cleanup.authDifferentRoot']));assert.ok(html.includes(catalogs[locale]['cleanup.authScanIncomplete']));
  assert.ok(html.includes(catalogs[locale]['cleanup.authScopeShared']));
 }
});

test('inventory prioritizes current results, separates read-only context and keeps source details collapsed',()=>{
 for(const locale of ['en','zh-CN']){
  const html=render(AuthenticationInventoryView,{inventory,selected:[sources[0].id],onToggle(){}},locale);
  const saved=html.indexOf(catalogs[locale]['cleanup.authSavedGroup']);
  const dynamic=html.indexOf(catalogs[locale]['cleanup.authInjectionGroup']);
  const auxiliary=html.indexOf(catalogs[locale]['cleanup.authAuxiliarySummary']);
  assert.ok(saved>=0&&dynamic>=0&&auxiliary>=0);
  assert.ok(saved<dynamic&&dynamic<auxiliary,'saved logins, optional injection and one auxiliary disclosure have a stable reading order');
  assert.equal((html.match(/class="authentication-disclosure authentication-auxiliary"/g)||[]).length,1,'only one auxiliary disclosure renders');
  assert.equal([...html.matchAll(/<summary class="authentication-disclosure-summary"/g)].length,[...html.matchAll(/<details /g)].length,'every disclosure uses the explicit summary');
  const attention=render(AuthenticationInventoryView,{inventory:{...inventory,sources:[{...sources[0],removable:false},{...sources[4],presence:'present',evidence:'inaccessible',reason:'keychain_locked'}]}},locale);
  assert.ok(attention.indexOf(catalogs[locale]['cleanup.authAttentionGroup'])>=0);
  assert.ok(html.includes(catalogs[locale]['cleanup.authContextNote']));
  assert.ok(html.includes(catalogs[locale]['cleanup.authSourceDetails']));
  const details=[...html.matchAll(/<details /g)].length,summaries=[...html.matchAll(/<summary class="authentication-disclosure-summary"/g)].length;
  assert.equal(summaries,details,'every disclosure uses the shared explicit summary');
  assert.equal(html.includes(' open=""'),false,'ordinary inventory disclosures start collapsed');
  assert.ok(html.includes(catalogs[locale]['cleanup.authScopeProcess']));
 }
});

test('selection summary is explicit and treats zero cleanable sources as a verification-only state',()=>{
 for(const locale of ['en','zh-CN']){
  const selection=render(AuthenticationSelection,{inventory,selected:[],onToggle(){},onRestoreRecommended(){}},locale);
  assert.ok(selection.includes(translate(catalogs,locale,'cleanup.authSelectionSummary',{selected:0,available:2})));
  assert.ok(selection.includes(catalogs[locale]['cleanup.authVerificationOnlyTitle']));
  assert.ok(selection.includes(catalogs[locale]['cleanup.authChooseSources']));
  assert.equal(selection.includes(catalogs[locale]['cleanup.authRecommendedSelection']),false,'without default_selected metadata the UI does not invent recommendations');
  assert.equal(selection.includes(catalogs[locale]['cleanup.authClearSelection']),false);
  const empty=render(AuthenticationSelection,{inventory:{...inventory,sources:[sources[2],sources[4],sources[5]]},selected:[],onToggle(){},onRestoreRecommended(){}},locale);
  assert.equal(empty.includes(catalogs[locale]['cleanup.authNoCleanableSources']),false,'uncertain scans do not show a second clear-state summary');
  assert.ok(empty.includes(catalogs[locale]['cleanup.authNoCleanableUncertain']),'locked or incomplete scans retain an uncertainty state');
  assert.equal((empty.match(/data-auth-no-cleanable=/g)||[]).length,1,'one concise empty-state explanation');
  assert.equal(empty.includes(catalogs[locale]['cleanup.authSelectGroup']),false);
  assert.equal(empty.includes(catalogs[locale]['cleanup.authRecommendedSelection']),false);
  assert.equal(empty.includes(catalogs[locale]['cleanup.authVerificationOnlyTitle']),false);
 }
});

test('selection offers one verification-only explanation and a useful clear action only for selected sources',()=>{
 for(const locale of ['en','zh-CN']){
  const selected=render(AuthenticationSelection,{inventory,selected:[sources[0].id],onToggle(){},onRestoreRecommended(){}},locale);
  assert.ok(selected.includes(catalogs[locale]['cleanup.authClearSelection']));
  assert.equal(selected.includes(catalogs[locale]['cleanup.authVerificationOnlyTitle']),false);
  const verify=render(AuthenticationSelection,{inventory,selected:[],onToggle(){},onRestoreRecommended(){}},locale);
  assert.equal(verify.split(catalogs[locale]['cleanup.authVerificationOnlyTitle']).length-1,1);
  assert.equal(verify.includes(catalogs[locale]['cleanup.authClearSelection']),false);
  assert.equal(selected.includes(secret),false);
 }
});

test('all details use one native disclosure summary with explicit state, icons and inset focus styling',async()=>{
 for(const locale of ['en','zh-CN']){
  const html=render(AuthenticationInventoryView,{inventory,selected:[sources[0].id],onToggle(){}},locale);
  assert.ok(html.includes('authentication-disclosure-summary'));
  assert.ok(html.includes(catalogs[locale]['cleanup.authDisclosureExpand']));
  assert.ok(html.includes(catalogs[locale]['cleanup.authDisclosureCollapse']));
  assert.ok(html.includes('authentication-disclosure-open'));
  assert.ok(html.includes('authentication-disclosure-closed'));
  assert.equal([...html.matchAll(/<summary class="authentication-disclosure-summary"/g)].length,[...html.matchAll(/<details /g)].length);
 }
 const css=await import('node:fs/promises').then(fs=>fs.readFile(path.join(repo,'apps/desktop/src/authentication.css'),'utf8'));
 assert.ok(css.includes('.authentication-disclosure-summary:focus-visible{outline:none;box-shadow:inset 0 -2px var(--focus);background:var(--accent-bg)}'));
 assert.ok(css.includes('[open]>.authentication-disclosure-summary>.ui-icon{transform:rotate(-90deg)}'));
 assert.equal(css.includes('outline-offset:2px'),false);
});

test('auxiliary inventory renders context, absent locations and external diagnostics through one disclosure',()=>{
 const auxiliary=React.createElement('div',{className:'authentication-surface-auxiliary'},React.createElement('input',{'aria-label':'Synthetic project cwd'}));
 for(const locale of ['en','zh-CN']){
  const html=render(AuthenticationInventoryView,{inventory,selected:[],onToggle(){},auxiliary},locale);
  assert.equal([...html.matchAll(/class="authentication-disclosure authentication-auxiliary"/g)].length,1);
  assert.equal([...html.matchAll(/class="authentication-disclosure authentication-context"/g)].length,1);
  assert.equal([...html.matchAll(/class="authentication-disclosure authentication-absent"/g)].length,1);
  assert.ok(html.includes('Synthetic project cwd'));
  assert.ok(html.indexOf('Synthetic project cwd')<html.indexOf(catalogs[locale]['cleanup.authContextNote']),'project input precedes lengthy read-only evidence');
  assert.equal(html.indexOf(catalogs[locale]['cleanup.authAuxiliaryNote'])<html.indexOf(catalogs[locale]['cleanup.authContextNote']),true);
  assert.equal(html.includes('authentication-source-details" open='),false);
 }
});

test('frozen plan clearly includes shared-file consequences and opt-in runtime execution',()=>{
 for(const locale of ['en','zh-CN']){
  const html=render(AuthenticationPlanReview,{authentication},locale);
  assert.ok(html.includes(catalogs[locale]['cleanup.authSharedImpact']));assert.ok(html.includes(catalogs[locale]['cleanup.authRuntimeIncluded']));
  assert.equal(html.includes(secret),false);assert.equal((html.match(/type="checkbox"/g)||[]).length,0);
  const empty=render(AuthenticationPlanReview,{authentication:{...authentication,selected_sources:[],local_source_count:0,runtime_verification_requested:false}},locale);
  assert.ok(empty.includes(catalogs[locale]['cleanup.authNoSelection']));assert.ok(empty.includes(catalogs[locale]['cleanup.authRuntimeNotIncluded']));
  const verification=render(AuthenticationPlanReview,{authentication:{...authentication,selected_sources:[]},verificationOnly:true},locale);
  assert.ok(verification.includes(catalogs[locale]['cleanup.authVerificationPlanNote']));
  assert.equal(empty.includes(catalogs[locale]['cleanup.authVerificationPlanNote']),false,'an empty authentication subset alone does not make rebuilding or retirement read-only');
 }
});

test('empty, nonempty and non-string source shapes remain distinct without implying validity or exposing values',()=>{
 const shapes=[['empty','cleanup.authValueEmpty'],['nonempty','cleanup.authValueNonempty'],['non_string','cleanup.authValueNonString']];
 for(const locale of ['en','zh-CN'])for(const kind of ['process_env','settings_env']){
  for(const [value_state,key] of shapes){
   const html=render(AuthenticationSourceRow,{source:{...sources[2],kind,value_state}},locale);
   assert.ok(html.includes(catalogs[locale][key]));assert.ok(html.includes(catalogs[locale]['cleanup.authValueShapeOnly']));
   assert.ok(html.includes(catalogs[locale]['cleanup.authEvidenceContext']),'shape does not replace the evidence level');
   assert.equal(html.includes(secret),false);
   for(const [other,otherKey] of shapes)if(other!==value_state)assert.equal(html.includes(catalogs[locale][otherKey]),false);
  }
  for(const value_state of ['not_set',undefined,secret]){
   const html=render(AuthenticationSourceRow,{source:{...sources[2],kind,value_state}},locale);
   assert.equal(html.includes('data-value-state='),false);assert.equal(html.includes(secret),false);
  }
 }
});

test('result keeps removed sources, still-active auth, unknown scopes and retained originals distinct',()=>{
 for(const locale of ['en','zh-CN']){
  const html=render(AuthenticationResult,{receipt},locale);
  assert.ok(html.includes(catalogs[locale]['cleanup.authRemoved']));assert.ok(html.includes(catalogs[locale]['cleanup.authChanged']));
  assert.ok(html.includes(translate(catalogs,locale,'cleanup.authRuntimeStillLoggedIn',{method:catalogs[locale]['cleanup.authMethodOauth']})));
  assert.ok(html.includes(catalogs[locale]['cleanup.authRuntimeScope']));
  for(const instruction of locale==='en'?['The runner may still inherit old values.','After the current task finishes','This does not change the parent shell or already running processes.']:['执行器自身也可能继承旧值','当前任务结束后重新启动','不会自动改变父 Shell 或已有进程。'])assert.ok(html.includes(instruction));
  assert.ok(html.includes(catalogs[locale]['cleanup.authOtherUnverified']));assert.ok(html.includes(catalogs[locale]['cleanup.authServerUnverified']));
  assert.ok(html.includes(catalogs[locale]['cleanup.authRetainedTitle']));assert.ok(html.includes(catalogs[locale]['cleanup.authRetainedLocationUnknown']));assert.ok(html.includes('/synthetic/.lintel-held/state'));
  assert.ok(html.includes(catalogs[locale]['cleanup.authFreshDoesNotRewrite']));assert.equal(html.includes(secret),false);
 }
});

test('unknown runtime results and zero-selection verification never render a signed-out claim',()=>{
 for(const locale of ['en','zh-CN']){
  const html=render(AuthenticationResult,{receipt:{...receipt,auth_verification:{...report,local_sources:[],runtime:{state:'unavailable',error_code:'auth_schema_unknown'},retained_sensitive_material:[]}}},locale);
  assert.ok(html.includes(catalogs[locale]['cleanup.authVerificationOnlyResult']));assert.ok(html.includes(catalogs[locale]['cleanup.authRuntimeUnavailable']));
  assert.equal(html.includes('No login reported'),false);assert.equal(html.includes('当前未报告登录'),false);
 }
});

test('an unavailable post-cleanup inventory stays unknown instead of crashing or claiming absence',()=>{
 for(const locale of ['en','zh-CN']){
  const unavailable={schema:'lintel.auth-sources/1',sources:[],error_code:'root_unreadable',coverage:'unknown'};
  const html=render(AuthenticationResult,{receipt:{...receipt,auth_verification:{...report,inventory:unavailable}}},locale);
  assert.ok(html.includes(catalogs[locale]['cleanup.authInventoryUnavailable']));
  assert.ok(html.includes('root_unreadable'));assert.ok(html.includes(catalogs[locale]['cleanup.authOtherUnverified']));
  assert.equal(html.includes(catalogs[locale]['cleanup.authNoneObserved']),false);
 }
});


test('exact Keychain login is selectable with its service while secret and container stay independent',()=>{
 for(const locale of ['en','zh-CN']){
  const login={id:'auth-keychain-login',kind:'keychain_field',name:'claudeAiOauth',locator:'/claudeAiOauth',service:'Claude Code-credentials-synthetic',scope:'selected_root',presence:'present',evidence:'confirmed_present',removable:true,default_selected:true,value:secret};
  const html=render(AuthenticationInventoryView,{inventory:{...inventory,sources:[login,sources[4]]},selected:[login.id],onToggle(){}},locale);
  assert.ok(html.includes(catalogs[locale]['cleanup.authSavedClaude']));
  assert.ok(html.includes(login.service));assert.ok(html.includes('/claudeAiOauth'));
  assert.equal((html.match(/type="checkbox"/g)||[]).length,1);
  assert.equal((html.match(/checked=""/g)||[]).length,1);
  assert.equal(html.includes(secret),false);
  assert.ok(html.includes(catalogs[locale]['cleanup.authContextCount'].replace('{count}','1')));
  const plan=render(AuthenticationPlanReview,{authentication:{...authentication,selected_sources:[login]}},locale);
  assert.ok(plan.includes(login.service));assert.equal(plan.includes(secret),false);
  assert.ok(plan.includes('authentication-source-details" open=""'),'frozen locations start expanded at approval');
  const result=render(AuthenticationResult,{receipt:{...receipt,auth_verification:{...report,local_sources:[{...login,state:'removed'}]}}},locale);
  assert.ok(result.includes(login.service));
 }
});

test('Keychain failures retain an actionable unknown state rather than an absent or signed-out claim',()=>{
 const reasons={keychain_locked:'authKeychainLocked',keychain_access_denied:'authKeychainDenied',keychain_unavailable:'authKeychainUnavailable',keychain_unsupported:'authKeychainUnsupported',keychain_synthetic_unavailable:'authKeychainSynthetic',keychain_invalid_format:'authKeychainFormat',keychain_too_large:'authKeychainLarge'};
 for(const locale of ['en','zh-CN'])for(const [reason,key] of Object.entries(reasons)){
  const source={id:'auth-keychain-unavailable',kind:'keychain',name:'claudeAiOauth',locator:'/claudeAiOauth',service:'Claude Code-credentials-synthetic',scope:'selected_root',presence:'unknown',evidence:'inaccessible',removable:false,reason};
  const html=render(AuthenticationInventoryView,{inventory:{...inventory,sources:[source]},selected:[],onToggle(){}},locale);
  assert.ok(html.includes(catalogs[locale]['cleanup.'+key]));
  assert.ok(html.includes('authentication-keychain-state'));
  assert.equal((html.match(/type="checkbox"/g)||[]).length,0);
  assert.equal(html.includes(catalogs[locale]['cleanup.authNoneObserved']),false);
  assert.equal(html.includes('authentication-absent'),false);
 }
});
