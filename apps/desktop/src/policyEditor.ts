import type { CustomSettings, CustomValues, Draft, PolicyChoice, PolicyRule } from './types';

export const policyLabelKeys: Record<string, string> = {
  CLAUDE_CODE_DISABLE_WEB_FETCH: 'customPolicy.label.webfetch',
  DO_NOT_TRACK: 'customPolicy.label.crossTool',
  CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: 'customPolicy.label.nonessential',
};

export type PolicyGroup = 'privacy' | 'content' | 'telemetry' | 'identity' | 'other';

export const policyChoiceKeys: Record<PolicyChoice, string> = {
  keep: 'customPolicy.choice.keep', disable: 'customPolicy.choice.disable', enable: 'customPolicy.choice.enable',
  remove: 'customPolicy.choice.remove', set: 'customPolicy.choice.set',
};
export const policySemanticsKeys: Record<PolicyRule['semantics'], string> = {
  nonempty: 'customPolicy.semanticsNonempty', boolean: 'customPolicy.semanticsBoolean', default_on: 'customPolicy.semanticsDefaultOn',
  value: 'customPolicy.semanticsValue', raw_api_bodies: 'customPolicy.semanticsRawApiBodies',
};

export function policyStateKey(rule: PolicyRule): string {
  if (rule.masked || rule.value_masked) return 'customPolicy.maskedValue';
  if (rule.semantics === 'raw_api_bodies' && rule.signal_enabled === true && rule.value?.startsWith('file:')) return 'customPolicy.stateFile';
  if (rule.semantics === 'raw_api_bodies' && rule.signal_enabled === true && rule.value === '1') return 'customPolicy.stateInline';
  const signal = rule.signal_enabled ?? (rule.category === 'privacy_switch' && rule.disabled !== null ? !rule.disabled : null);
  if (signal !== null) return signal ? 'customPolicy.stateOn' : 'customPolicy.stateOff';
  return rule.value === null ? 'app.notSet' : 'customPolicy.stateSet';
}

export function policyGroup(rule: PolicyRule): PolicyGroup {
  if (rule.category === 'privacy_switch') return 'privacy';
  if (rule.category?.startsWith('content_')) return 'content';
  if (['telemetry_core', 'exporter', 'exporter_destination'].includes(rule.category ?? '')) return 'telemetry';
  if (['identity_attribute', 'feedback', 'webfetch'].includes(rule.category ?? '')) return 'identity';
  return 'other';
}

export function policyOptions(rule: PolicyRule): PolicyChoice[] {
  if (rule.key === 'OTEL_EXPORTER_OTLP_HEADERS') return ['keep', 'disable', 'remove'];
  if (rule.category === 'privacy_switch') return ['keep', 'disable', 'enable', 'remove', 'set'];
  if (rule.semantics === 'value') return ['keep', 'disable', 'set', 'remove'];
  if (rule.semantics === 'raw_api_bodies') return ['keep', 'disable', 'enable', 'set', 'remove'];
  return ['keep', 'disable', 'enable', 'remove'];
}

export function commonPolicyOptions(rules: PolicyRule[]): PolicyChoice[] {
  return (['disable', 'enable', 'remove', 'keep'] as const).filter(choice => rules.length > 0 && rules.every(rule => visiblePolicyOptions(rule).includes(choice)));
}

// One edit patches the latest target draft once. Calling a captured per-row
// callback in a loop used to overwrite earlier items in the same React batch.
export function editPolicyDraft(draft: Draft, choices: CustomSettings, values: CustomValues = {}): Draft {
  const customValues = { ...draft.customValues, ...values };
  for (const [key, choice] of Object.entries(choices)) if (choice !== 'set') delete customValues[key];
  return { ...draft, customSettings: { ...draft.customSettings, ...choices }, customValues };
}

export function pendingPolicyEdits(rules: PolicyRule[], choices: CustomSettings, values: CustomValues) {
  const pending = rules.filter(rule => (choices[rule.key] ?? 'keep') !== 'keep');
  const invalid = pending.filter(rule => !policyOptions(rule).includes(choices[rule.key]) || (choices[rule.key] === 'set' && !!policyValueError(rule, values[rule.key] ?? '')));
  return { pending, invalid };
}

// A free-value field has no off spelling: core's disable and remove both
// delete that field. Keep old draft intents valid, but show one removal choice.
export function removesValue(rule: PolicyRule) {
  return rule.semantics === 'value' && !rule.key.endsWith('_EXPORTER');
}
export function visiblePolicyOptions(rule: PolicyRule): PolicyChoice[] {
  return policyOptions(rule).filter(choice => !(choice === 'disable' && removesValue(rule)));
}
export function policyChoiceKey(rule: PolicyRule, choice: PolicyChoice, value?: string | null): string {
  if (rule.semantics === 'raw_api_bodies') {
    if (choice === 'enable' || (choice === 'set' && value === '1')) return 'customPolicy.rawInline';
    if (choice === 'set' && value?.startsWith('file:')) return 'customPolicy.rawFile';
    if (choice === 'set' && value === '0') return 'customPolicy.choice.disable';
  }
  if (choice === 'disable' && removesValue(rule)) return 'customPolicy.choice.remove';
  if (choice === 'set' && rule.category === 'privacy_switch') return 'customPolicy.rawVariable';
  if (choice === 'set' && rule.key === 'OTEL_EXPORTER_OTLP_ENDPOINT') return 'customPolicy.setEndpoint';
  if (choice === 'set' && rule.key === 'CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH') return 'customPolicy.setLimit';
  return policyChoiceKeys[choice];
}
export function policyValueError(rule: PolicyRule, value: string): string | null {
  if (!value || (rule.semantics === 'raw_api_bodies' && value === 'file:')) return 'customPolicy.valueRequired';
  if (/[\u0000-\u001f\u007f]/.test(value)) return 'customPolicy.controlCharacters';
  if (rule.key === 'CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH' && !/^\d+$/.test(value)) return 'customPolicy.limitRequired';
  if (rule.key === 'OTEL_EXPORTER_OTLP_ENDPOINT') {
    if (/[@?#]/.test(value)) return 'customPolicy.privateEndpoint';
    if (/\s/.test(value)) return 'customPolicy.endpointRequired';
  }
  if (rule.semantics === 'raw_api_bodies' && !['0', '1'].includes(value)) {
    if (!value.startsWith('file:')) return 'customPolicy.chooseRawMode';
    const directory = value.slice(5);
    if (!directory || directory.trim() !== directory) return 'customPolicy.directoryRequired';
  }
  if (rule.key.endsWith('_EXPORTER')) {
    const allowed = ['otlp', 'console', 'none', ...(rule.key === 'OTEL_METRICS_EXPORTER' ? ['prometheus'] : [])];
    const tokens = value.split(',');
    if (tokens.some(token => !allowed.includes(token)) || new Set(tokens).size !== tokens.length || (tokens.includes('none') && tokens.length > 1)) return rule.key === 'OTEL_METRICS_EXPORTER' ? 'customPolicy.metricsExporterRequired' : 'customPolicy.exporterRequired';
  }
  if (rule.category === 'privacy_switch' && rule.semantics === 'boolean' && !['0','1','true','false','yes','no','on','off'].includes(value.toLowerCase())) return 'customPolicy.booleanRequired';
  return null;
}
