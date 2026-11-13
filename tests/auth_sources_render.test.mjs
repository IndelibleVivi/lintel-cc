// Browser-independent render evidence only. Interactive and visual acceptance
// remains auth_sources_ui_journey.mjs with a real headless browser.
import assert from 'node:assert/strict';
import {test,after} from 'node:test';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {createServer} from '../apps/desktop/node_modules/vite/dist/node/index.js';
import React from '../apps/desktop/node_modules/react/index.js';
import {renderToStaticMarkup} from '../apps/desktop/node_modules/react-dom/server.node.js';
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const server=await createServer({root:path.join(repo,'apps/desktop'),server:{middlewareMode:true,hmr:false,watch:null},appType:'custom'});
after(()=>server.close());
const {AuthenticationSourceRow,AuthenticationInventoryView,AuthenticationPlanReview,AuthenticationResult}=await server.ssrLoadModule('/src/AuthenticationSources.tsx');
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

test('frozen plan clearly includes shared-file consequences and opt-in runtime execution',()=>{
 for(const locale of ['en','zh-CN']){
  const html=render(AuthenticationPlanReview,{authentication},locale);
  assert.ok(html.includes(catalogs[locale]['cleanup.authSharedImpact']));assert.ok(html.includes(catalogs[locale]['cleanup.authRuntimeIncluded']));
  assert.equal(html.includes(secret),false);assert.equal((html.match(/type="checkbox"/g)||[]).length,0);
  const empty=render(AuthenticationPlanReview,{authentication:{...authentication,selected_sources:[],local_source_count:0,runtime_verification_requested:false}},locale);
  assert.ok(empty.includes(catalogs[locale]['cleanup.authNoSelection']));assert.ok(empty.includes(catalogs[locale]['cleanup.authRuntimeNotIncluded']));
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
  assert.ok(html.includes(catalogs[locale]['cleanup.authOtherUnverified']));assert.ok(html.includes(catalogs[locale]['cleanup.authServerUnverified']));
  assert.ok(html.includes(catalogs[locale]['cleanup.authRetainedTitle']));assert.ok(html.includes(catalogs[locale]['cleanup.authRetainedLocationUnknown']));assert.ok(html.includes('/synthetic/.lintel-held/state'));
  assert.ok(html.includes(catalogs[locale]['cleanup.authFreshDoesNotRewrite']));assert.equal(html.includes(secret),false);
 }
});

test('unknown runtime results and zero-selection verification never render a signed-out claim',()=>{
 for(const locale of ['en','zh-CN']){
  const html=render(AuthenticationResult,{receipt:{...receipt,auth_verification:{...report,local_sources:[],runtime:{state:'unavailable',error_code:'auth_schema_unknown'},retained_sensitive_material:[]}}},locale);
  assert.ok(html.includes(catalogs[locale]['cleanup.authNoSelection']));assert.ok(html.includes(catalogs[locale]['cleanup.authRuntimeUnavailable']));
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
