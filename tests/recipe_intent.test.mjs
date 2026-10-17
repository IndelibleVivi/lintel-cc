import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFile} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
const draftSource=await readFile(new URL('../apps/desktop/src/taskDrafts.ts',import.meta.url),'utf8');
const drafts='data:text/javascript;base64,'+Buffer.from(stripTypeScriptTypes(draftSource)).toString('base64');
const source=(await readFile(new URL('../apps/desktop/src/recipeIntent.ts',import.meta.url),'utf8')).replace("'./taskDrafts'",JSON.stringify(drafts));
const recipe=await import('data:text/javascript;base64,'+Buffer.from(stripTypeScriptTypes(source)).toString('base64'));
const {translate}=await import('../apps/desktop/src/i18n.ts');
const {zhCN}=await import('../apps/desktop/src/i18n.catalog.zh-CN.ts');
const {en}=await import('../apps/desktop/src/i18n.catalog.en.ts');
const catalogs={'zh-CN':zhCN,en};
const saved=new Map();
globalThis.localStorage={getItem:key=>saved.get(key)??null,setItem:(key,value)=>saved.set(key,value)};
test('the default actually disables content and is separate from login repair',()=>{
  const reduce=recipe.readRecipe('reduce_content',null,'synthetic');
  assert.deepEqual(Object.keys(reduce.customSettings),recipe.CONTENT_RECORDING_KEYS);
  assert.ok(Object.values(reduce.customSettings).every(value=>value==='disable'));
  assert.equal(recipe.readRecipe('clear_residue',null,'synthetic').kind,'residue');
});
test('ordinary choices survive reloading and host scope; private values do not',()=>{
  const intent={kind:'policy',preset:'custom',keepRemoteControl:false,customSettings:{
    OTEL_LOGS_EXPORTER:'set',CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH:'set',OTEL_EXPORTER_OTLP_ENDPOINT:'set',OTEL_EXPORTER_OTLP_HEADERS:'set',OTEL_LOG_RAW_API_BODIES:'set',UNRECOGNIZED:'disable'},customValues:{
    OTEL_LOGS_EXPORTER:'console',CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH:'8192',OTEL_EXPORTER_OTLP_ENDPOINT:'https://synthetic:secret@collector.invalid',OTEL_EXPORTER_OTLP_HEADERS:'synthetic-private-header',OTEL_LOG_RAW_API_BODIES:'file:/synthetic/private'}};
  recipe.saveRecipe('reduce_content',null,'synthetic',intent);
  const loaded=recipe.readRecipe('reduce_content',null,'synthetic');
  assert.equal(loaded.keepRemoteControl,false);assert.equal(loaded.customSettings.OTEL_LOGS_EXPORTER,'set');assert.equal(loaded.customValues.OTEL_LOGS_EXPORTER,'console');assert.equal(loaded.customValues.CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH,'8192');
  assert.equal(loaded.customSettings.OTEL_EXPORTER_OTLP_ENDPOINT,'keep');assert.equal(loaded.customSettings.OTEL_LOG_RAW_API_BODIES,'keep');assert.equal(loaded.customSettings.UNRECOGNIZED,undefined);
  assert.ok(!JSON.stringify([...saved.values()]).match(/secret@|private-header|file:\/synthetic/));
  assert.equal(recipe.readRecipe('reduce_content','another-synthetic-host','synthetic').keepRemoteControl,true);
});

test('saving reports omitted private values but retains safe removal intent',()=>{
  const intent={kind:'policy',preset:'custom',keepRemoteControl:false,customSettings:{OTEL_EXPORTER_OTLP_HEADERS:'remove',OTEL_EXPORTER_OTLP_ENDPOINT:'set',OTEL_LOG_RAW_API_BODIES:'set'},customValues:{OTEL_EXPORTER_OTLP_ENDPOINT:'https://collector.invalid/v1',OTEL_LOG_RAW_API_BODIES:'file:/synthetic/not-a-default'}};
  const result=recipe.saveRecipe('reduce_content',null,'omitted-fields',intent);
  assert.equal(result.saved,true);
  assert.deepEqual(result.omittedKeys,['OTEL_EXPORTER_OTLP_ENDPOINT','OTEL_LOG_RAW_API_BODIES']);
  const loaded=recipe.readRecipe('reduce_content',null,'omitted-fields');
  assert.equal(loaded.customSettings.OTEL_EXPORTER_OTLP_HEADERS,'remove');
  assert.equal(loaded.customSettings.OTEL_EXPORTER_OTLP_ENDPOINT,'keep');
  assert.equal(loaded.customSettings.OTEL_LOG_RAW_API_BODIES,'keep');
  assert.equal(intent.customSettings.OTEL_LOG_RAW_API_BODIES,'set','the in-memory draft remains independent');
  assert.ok(!JSON.stringify([...saved.values()]).includes('not-a-default'));
});
test('a storage failure is reported without altering the live intent',()=>{
  const write=globalThis.localStorage.setItem;
  globalThis.localStorage.setItem=()=>{throw new Error('synthetic storage unavailable')};
  try{assert.equal(recipe.saveRecipe('reduce_content',null,'unavailable',{kind:'policy',preset:'custom',keepRemoteControl:false,customSettings:{DISABLE_ERROR_REPORTING:'disable'}}).saved,false)}
  finally{globalThis.localStorage.setItem=write}
});

// --- Personal-default labeling: a saved intent different from the built-in
// default must not wear the built-in name/promise ---

test('semantic intent equality, not serialized key order, decides default vs customized',()=>{
  // Same effective intent with reordered keys still counts as the default.
  const shuffled={kind:'policy',preset:'custom',keepRemoteControl:true,customSettings:Object.fromEntries([...recipe.CONTENT_RECORDING_KEYS].reverse().map(key=>[key,'disable'])),releaseSettings:[],customValues:{}};
  assert.equal(recipe.isDefaultRecipeIntent('reduce_content',recipe.readRecipe('reduce_content',null,'untouched')),true);
  assert.equal(recipe.isDefaultRecipeIntent('reduce_content',shuffled),true,'key order must not fabricate a difference');
  // 'keep' entries carry no intent and stay equivalent to absent entries.
  const withKeep={kind:'policy',preset:'custom',keepRemoteControl:true,customSettings:{OTEL_LOG_USER_PROMPTS:'disable',OTEL_LOG_ASSISTANT_RESPONSES:'disable',OTEL_LOG_TOOL_CONTENT:'disable',OTEL_LOG_TOOL_DETAILS:'disable',OTEL_LOG_RAW_API_BODIES:'disable',OTEL_LOG_MANAGED_SETTINGS:'disable',DISABLE_TELEMETRY:'keep'}};
  assert.equal(recipe.isDefaultRecipeIntent('reduce_content',withKeep),true);
  // Relevant fields count even when they are not content-recording keys.
  assert.equal(recipe.isDefaultRecipeIntent('reduce_content',{...shuffled,keepRemoteControl:false}),false);
  assert.equal(recipe.isDefaultRecipeIntent('reduce_content',{...shuffled,preset:'reduce'}),false);
  assert.equal(recipe.isDefaultRecipeIntent('reduce_content',{...shuffled,trustedDevices:'required'}),false);
  assert.equal(recipe.isDefaultRecipeIntent('reduce_content',{...shuffled,releaseSettings:['SYNTHETIC_KEY']}),false);
  assert.equal(recipe.isDefaultRecipeIntent('reduce_content',{...shuffled,customSettings:{}}),false,'no fields at all is a different intent');
});

test('saving a recording-ON intent marks the slot customized per host/environment',()=>{
  const enable={kind:'policy',preset:'custom',keepRemoteControl:true,customSettings:{OTEL_LOG_USER_PROMPTS:'enable',OTEL_LOG_ASSISTANT_RESPONSES:'disable',OTEL_LOG_TOOL_CONTENT:'disable',OTEL_LOG_TOOL_DETAILS:'disable',OTEL_LOG_RAW_API_BODIES:'disable',OTEL_LOG_MANAGED_SETTINGS:'disable'}};
  recipe.saveRecipe('reduce_content',null,'records-on',enable);
  const loaded=recipe.readRecipe('reduce_content',null,'records-on');
  assert.equal(loaded.customSettings.OTEL_LOG_USER_PROMPTS,'enable');
  assert.equal(recipe.isDefaultRecipeIntent('reduce_content',loaded),false,'a content-ON default is not the built-in default');
  // Scope: the same recipe in another environment still equals its default.
  assert.equal(recipe.isDefaultRecipeIntent('reduce_content',recipe.readRecipe('reduce_content',null,'other-env')),true);
  const summary=recipe.customizedPolicySummary(loaded);
  assert.deepEqual(summary,{enabled:1,disabled:5,other:0});
});

test('the customized label resolves to accurate personal-default copy in both languages',()=>{
  // The misleading baseline was the slot name/detail claiming recording is off.
  // Guard the copy contract, not a literal sentence: neither language may reuse
  // the built-in promise for a customized slot.
  const nameZh=translate(catalogs,'zh-CN','recipes.name.personalPolicy',{preset:'自定义'});
  const detailZh=translate(catalogs,'zh-CN','recipes.detail.personalPolicy',{preset:'自定义',remote:translate(catalogs,'zh-CN','recipes.remoteKept'),enabled:1,disabled:5,other:0});
  const nameEn=translate(catalogs,'en','recipes.name.personalPolicy',{preset:'Custom'});
  const detailEn=translate(catalogs,'en','recipes.detail.personalPolicy',{preset:'Custom',remote:translate(catalogs,'en','recipes.remoteDropped'),enabled:1,disabled:5,other:0});
  for(const text of [nameZh,detailZh])assert.ok(!text.includes('减少正文记录'),'zh-CN customized label must not claim reduced recording: '+text);
  for(const text of [nameEn,detailEn])assert.ok(!text.toLowerCase().includes('reduce body recording'),'en customized label must not claim reduced recording: '+text);
  assert.ok(detailZh.includes('自定义')&&detailZh.includes('1'),'zh-CN detail names the preset and counts');
  assert.ok(detailEn.includes('Custom')&&detailEn.includes('without Remote Control'),'en detail names the preset and remote state');
  assert.ok(!detailEn.includes('{'),'all placeholders interpolated');
});
