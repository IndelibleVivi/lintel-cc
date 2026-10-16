import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFile} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
const draftSource=await readFile(new URL('../apps/desktop/src/taskDrafts.ts',import.meta.url),'utf8');
const drafts='data:text/javascript;base64,'+Buffer.from(stripTypeScriptTypes(draftSource)).toString('base64');
const source=(await readFile(new URL('../apps/desktop/src/recipeIntent.ts',import.meta.url),'utf8')).replace("'./taskDrafts'",JSON.stringify(drafts));
const recipe=await import('data:text/javascript;base64,'+Buffer.from(stripTypeScriptTypes(source)).toString('base64'));
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
