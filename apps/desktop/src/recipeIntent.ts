import type { CustomSettings, CustomValues, PolicyPreset, TrustedDevices } from './types';
import { saveTaskDraft } from './taskDrafts';

// Three editable quick recipes. Only ordinary, non-sensitive intent is stored
// here: a policy preset with its finite field choices, or a residue/cleanup
// selection. Never a secret, an identity value, an exporter header, a credential
// URL, a path or a frozen plan — every run regenerates a fresh plan for the
// current host and environment and always requires a new approval.
export type RecipeId = 'reduce_content' | 'clear_residue' | 'preserve_and_rebuild';
export const RECIPE_IDS: RecipeId[] = ['reduce_content', 'clear_residue', 'preserve_and_rebuild'];

// The finite, verified content-recording fields "reduce body recording" turns
// off. These are the real OTel content families from the versioned rule catalog,
// not a no-op: metrics stay on, only content recording is disabled. None of them
// carries a credential value.
export const CONTENT_RECORDING_KEYS: string[] = [
  'OTEL_LOG_USER_PROMPTS',
  'OTEL_LOG_ASSISTANT_RESPONSES',
  'OTEL_LOG_TOOL_CONTENT',
  'OTEL_LOG_TOOL_DETAILS',
  'OTEL_LOG_RAW_API_BODIES',
  'OTEL_LOG_MANAGED_SETTINGS',
];
export function contentRecordingOff(): CustomSettings {
  return Object.fromEntries(CONTENT_RECORDING_KEYS.map(key => [key, 'disable'] as const));
}

export type PolicyRecipeIntent = {
  kind: 'policy'; preset: PolicyPreset; keepRemoteControl: boolean; trustedDevices?: TrustedDevices;
  releaseSettings?: string[]; customSettings?: CustomSettings;customValues?:CustomValues;
};
// A residue-clear recipe is a metadata-driven cleanup of the runtime residue the
// ResiduePanel has selected; it is NOT a login repair. The optional combined
// flags seed an optional single-plan client workflow.
export type ResidueRecipeIntent = {
  kind: 'residue'; reduceContent: boolean; resetIdentity: boolean;
};
// A cleanup recipe keeps the existing cleanup recipes only. `repair_login` is a
// distinct journey and is never the target of the "clear residue" recipe.
export type CleanupRecipeIntent = {
  kind: 'cleanup'; recipe: 'reset_client' | 'retire';
  categories: string[]; activateInstructions: boolean;
};
export type RecipeIntent = PolicyRecipeIntent | ResidueRecipeIntent | CleanupRecipeIntent;

const defaults: Record<RecipeId, RecipeIntent> = {
  // Reduce recording disables the real content-recording fields (never empty).
  reduce_content: { kind: 'policy', preset: 'custom', keepRemoteControl: true, customSettings: contentRecordingOff() },
  clear_residue: { kind: 'residue', reduceContent: false, resetIdentity: false },
  preserve_and_rebuild: { kind: 'cleanup', recipe: 'reset_client', categories: ['instructions', 'memory', 'sessions'], activateInstructions: false },
};
export const recipeTargets: Record<RecipeId, 'policy' | 'rebuild'> = {
  reduce_content: 'policy', clear_residue: 'rebuild', preserve_and_rebuild: 'rebuild',
};

const recipeKeys=new Set([...CONTENT_RECORDING_KEYS,'DISABLE_TELEMETRY','DISABLE_ERROR_REPORTING','DISABLE_FEEDBACK_COMMAND','CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY','DO_NOT_TRACK','DISABLE_GROWTHBOOK','CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC','CLAUDE_CODE_ENABLE_TELEMETRY','CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH','OTEL_LOGS_EXPORTER','OTEL_TRACES_EXPORTER','OTEL_METRICS_EXPORTER','OTEL_EXPORTER_OTLP_ENDPOINT','OTEL_EXPORTER_OTLP_HEADERS','OTEL_METRICS_INCLUDE_ACCOUNT_UUID','OTEL_METRICS_INCLUDE_SESSION_ID','OTEL_METRICS_INCLUDE_VERSION','OTEL_METRICS_INCLUDE_ENTRYPOINT','OTEL_METRICS_INCLUDE_REPOSITORY','CLAUDE_CODE_ENABLE_FEEDBACK_SURVEY_FOR_OTEL','CLAUDE_CODE_DISABLE_WEB_FETCH']);

function sanitize(id: RecipeId, value: Record<string, unknown>): RecipeIntent {
  const base = defaults[id];
  if (base.kind === 'policy') {
    const preset: PolicyPreset = ['preserve', 'reduce', 'custom'].includes(String(value.preset)) ? value.preset as PolicyPreset : base.preset;
    const customSettings: CustomSettings = value.customSettings && typeof value.customSettings==='object' ? {} : { ...base.customSettings };
    if (value.customSettings && typeof value.customSettings === 'object') {
      for (const [key, choice] of Object.entries(value.customSettings as Record<string, unknown>)) {
        if (recipeKeys.has(key) && ['keep', 'disable', 'enable', 'remove', 'set'].includes(String(choice)) && (key !== 'OTEL_EXPORTER_OTLP_HEADERS' || ['keep','disable','remove'].includes(String(choice)))) customSettings[key] = choice as CustomSettings[string];
      }
    }
    // Only finite ordinary values are portable defaults. Headers, receiver
    // endpoints, filesystem destinations and arbitrary strings stay in memory.
    const customValues:CustomValues={};
    const rawValues=value.customValues && typeof value.customValues==='object'?value.customValues as Record<string,unknown>:{};
    for(const [key,choice] of Object.entries(customSettings)){
      if(choice!=='set')continue;
      const raw=rawValues[key];
      const ordinary=typeof raw==='string' && raw.length<=32 && (
        (key==='CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH'&&/^\d{1,10}$/.test(raw)) ||
        (['OTEL_LOGS_EXPORTER','OTEL_TRACES_EXPORTER','OTEL_METRICS_EXPORTER'].includes(key)&&raw.split(',').every(token=>['otlp','console','none',...(key==='OTEL_METRICS_EXPORTER'?['prometheus']:[])].includes(token))&&(!raw.includes('none')||raw==='none')) ||
        (['DISABLE_TELEMETRY','DISABLE_ERROR_REPORTING','DISABLE_FEEDBACK_COMMAND','CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY','DO_NOT_TRACK','DISABLE_GROWTHBOOK','CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC'].includes(key)&&['0','1','true','false','yes','no','on','off'].includes(raw.toLowerCase())) ||
        (key==='OTEL_LOG_RAW_API_BODIES'&&['0','1'].includes(raw))
      );
      if(ordinary)customValues[key]=raw;else customSettings[key]='keep';
    }
    const trusted: TrustedDevices | undefined = ['unknown', 'required', 'not_required'].includes(String(value.trustedDevices)) ? value.trustedDevices as TrustedDevices : undefined;
    return { kind: 'policy', preset, keepRemoteControl: value.keepRemoteControl !== false, trustedDevices: trusted, releaseSettings: Array.isArray(value.releaseSettings) ? (value.releaseSettings as unknown[]).filter((k): k is string => typeof k === 'string') : [], customSettings,customValues };
  }
  if (base.kind === 'residue') {
    return { kind: 'residue', reduceContent: value.reduceContent === true, resetIdentity: value.resetIdentity === true };
  }
  const recipe = ['reset_client', 'retire'].includes(String(value.recipe)) ? value.recipe as CleanupRecipeIntent['recipe'] : base.recipe;
  const categories = Array.isArray(value.categories) ? (value.categories as unknown[]).filter((c): c is string => typeof c === 'string' && ['instructions', 'memory', 'sessions'].includes(c)) : base.categories;
  return { kind: 'cleanup', recipe, categories, activateInstructions: value.activateInstructions === true };
}

export function recipeKey(id: RecipeId, hostAlias: string | null, environmentId: string) {
  return `lintel.recipe:${id}:` + JSON.stringify([hostAlias, environmentId]);
}
export function readRecipe(id: RecipeId, hostAlias: string | null, environmentId: string): RecipeIntent {
  try { const raw=localStorage.getItem(recipeKey(id,hostAlias,environmentId)); if(raw && raw.length<=32768){const value:unknown=JSON.parse(raw);if(value && typeof value==='object'&&!Array.isArray(value))return sanitize(id,value as Record<string,unknown>);} } catch { /* A corrupt local default never becomes approval. */ }
  return sanitize(id,{});
}
export function saveRecipe(id: RecipeId, hostAlias: string | null, environmentId: string, intent: RecipeIntent) {
  const stored = sanitize(id, intent as unknown as Record<string, unknown>);
  const key = recipeKey(id, hostAlias, environmentId);
  saveTaskDraft(key, stored as unknown as Record<string, unknown>);
  let saved = false;
  try { saved = localStorage.getItem(key) === JSON.stringify(stored); } catch { /* Keep the live draft usable. */ }
  // Report finite omitted field names, never their private values. The live
  // draft is independent and must not be silently replaced by this projection.
  const omittedKeys = intent.kind === 'policy' && stored.kind === 'policy'
    ? Object.entries(intent.customSettings ?? {}).filter(([key, choice]) => recipeKeys.has(key) && choice !== 'keep' && stored.customSettings?.[key] !== choice).map(([key]) => key)
    : [];
  return { saved, omittedKeys };
}
export function defaultRecipe(id: RecipeId): RecipeIntent { return defaults[id]; }
