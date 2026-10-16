import { useState } from 'react';
import { Icon, Notice } from './ui';
import { useI18n } from './i18n-provider';
import { RECIPE_IDS, recipeTargets, readRecipe, saveRecipe, type RecipeId, type RecipeIntent, type PolicyRecipeIntent, type ResidueRecipeIntent, type CleanupRecipeIntent } from './recipeIntent';
import type { PolicyPreset, TrustedDevices } from './types';
import './Recipes.css';

// Quick recipes factor the three ordinary journeys into editable intent. A
// recipe never replays an old approval: applying it seeds the current draft (or
// the residue selection) for this host/environment, and the normal preview step
// generates a fresh plan. Only ordinary configuration is saved — never a secret,
// identity value, exporter header, credential URL or path.
export default function Recipes({ hostAlias, environmentId, disabled, onPolicy, onRebuild,
  onApplyPolicy, onApplyResidue, onApplyCleanup, scope }: {
  hostAlias: string | null; environmentId: string; disabled: boolean;
  onPolicy: () => void; onRebuild: () => void;
  onApplyPolicy: (intent: PolicyRecipeIntent) => void;
  onApplyResidue: (intent: ResidueRecipeIntent) => void;
  onApplyCleanup: (intent: CleanupRecipeIntent) => void;
  scope: 'policy' | 'rebuild';
}) {
  const { t } = useI18n();
  const [open, setOpen] = useState<RecipeId | null>(null);
  const [notice, setNotice] = useState<RecipeId | null>(null);
  function apply(id: RecipeId) {
    const intent = readRecipe(id, hostAlias, environmentId);
    if (intent.kind === 'policy') { onApplyPolicy(intent); onPolicy(); }
    else if (intent.kind === 'residue') { onApplyResidue(intent); onRebuild(); }
    else { onApplyCleanup(intent); onRebuild(); }
  }
  function save(id: RecipeId) {
    // Saving captures the neutral default intent for this host/environment; the
    // editable fields are the recipe's own controls, so no live draft is stored.
    saveRecipe(id, hostAlias, environmentId, readRecipe(id, hostAlias, environmentId));
    setNotice(id);
  }
  const targetLabel = (id: RecipeId) => t(recipeTargets[id] === 'policy' ? 'recipes.targetPolicy' : 'recipes.targetRebuild');
  return <section className="surface recipes" aria-label={t('recipes.aria')}>
    <div className="surface-heading"><div><h2>{t('recipes.title')}</h2><span className="small-label">{t('recipes.subtitle')}</span></div></div>
    <p className="small-print recipes-intro">{t('recipes.intro')}</p>
    <div className="recipe-list">{RECIPE_IDS.map(id => {
      const intent = readRecipe(id, hostAlias, environmentId);
      const matchesScope = recipeTargets[id] === scope;
      return <div className={`recipe-row ${matchesScope ? 'current-scope' : 'other-scope'}`} key={id}>
        <div className="recipe-head">
          <div className="recipe-copy">
            <strong>{t(`recipes.name.${id}` as never)}</strong>
            <p>{t(`recipes.detail.${id}` as never)}</p>
            <div className="recipe-meta">
              <span className={`recipe-chip recipe-target${matchesScope ? ' scope' : ''}`}>{targetLabel(id)}</span>
              {intent.kind === 'policy' && <span className="recipe-chip">{t('recipes.intentPolicy', { count: Object.keys(intent.customSettings ?? {}).filter(key => (intent.customSettings ?? {})[key] !== 'keep').length })}</span>}
              {intent.kind === 'residue' && <span className="recipe-chip">{t('recipes.intentResidue')}</span>}
              {intent.kind === 'cleanup' && <span className="recipe-chip">{t('recipes.intentCleanup', { count: intent.categories.length })}</span>}
            </div>
          </div>
          <div className="recipe-apply-col">
            <button className={`recipe-apply${matchesScope ? ' primary' : ''}`} disabled={disabled} onClick={() => apply(id)}>{t('recipes.apply')}<Icon name="arrow" size={14}/></button>
          </div>
        </div>
        <div className="recipe-actions">
          <button className="text-button recipe-edit-toggle" disabled={disabled} aria-expanded={open === id} onClick={() => setOpen(open === id ? null : id)}><Icon name="chevron" size={12}/>{open === id ? t('recipes.hideEdit') : t('recipes.edit')}</button>
          <button className="text-button recipe-save" disabled={disabled} onClick={() => save(id)}>{t('recipes.saveDefault')}</button>
        </div>
        {open === id && <div className="recipe-edit">
          <p className="small-print">{t('recipes.editNote')}</p>
          {intent.kind === 'policy' ? <PolicyRecipeFields id={id} hostAlias={hostAlias} environmentId={environmentId}/>
            : intent.kind === 'residue' ? <ResidueRecipeFields id={id} hostAlias={hostAlias} environmentId={environmentId}/>
            : <CleanupRecipeFields id={id} hostAlias={hostAlias} environmentId={environmentId}/>}
        </div>}
        {notice === id && <p className="small-print recipe-notice" role="status">{t('recipes.savedDefault')}</p>}
      </div>;
    })}</div>
    <div className="recipes-note"><Notice>{t('recipes.freshPlanNote')}</Notice></div>
  </section>;
}

function PolicyRecipeFields({ id, hostAlias, environmentId }: { id: RecipeId; hostAlias: string | null; environmentId: string }) {
  const { t } = useI18n();
  const [intent, setIntent] = useState<RecipeIntent>(() => readRecipe(id, hostAlias, environmentId));
  if (intent.kind !== 'policy') return null;
  function update(patch: Partial<PolicyRecipeIntent>) {
    const next = { ...intent, ...patch } as PolicyRecipeIntent;
    setIntent(next); saveRecipe(id, hostAlias, environmentId, next);
  }
  return <div className="recipe-fields">
    <label className="field">{t('recipes.fieldPreset')}<select value={intent.preset} onChange={event => update({ preset: event.target.value as PolicyPreset })}><option value="preserve">{t('policy.preserve')}</option><option value="reduce">{t('policy.reduce')}</option><option value="custom">{t('policy.custom')}</option></select></label>
    <label className="checkbox-inline"><input type="checkbox" checked={intent.keepRemoteControl} onChange={event => update({ keepRemoteControl: event.target.checked })}/><span>{t('recipes.fieldKeepRemote')}</span></label>
    <label className="field">{t('recipes.fieldTrusted')}<select value={intent.trustedDevices ?? 'unknown'} onChange={event => update({ trustedDevices: event.target.value as TrustedDevices })}><option value="unknown">{t('app.trustedUnknown')}</option><option value="required">{t('app.trustedRequired')}</option><option value="not_required">{t('app.trustedNotRequired')}</option></select></label>
    {intent.preset === 'custom' && <p className="small-print">{t('recipes.customNote')}</p>}
  </div>;
}

function ResidueRecipeFields({ id, hostAlias, environmentId }: { id: RecipeId; hostAlias: string | null; environmentId: string }) {
  const { t } = useI18n();
  const [intent, setIntent] = useState<RecipeIntent>(() => readRecipe(id, hostAlias, environmentId));
  if (intent.kind !== 'residue') return null;
  function update(patch: Partial<ResidueRecipeIntent>) {
    const next = { ...intent, ...patch } as ResidueRecipeIntent;
    setIntent(next); saveRecipe(id, hostAlias, environmentId, next);
  }
  return <div className="recipe-fields">
    <p className="small-print">{t('recipes.residueNote')}</p>
    <label className="checkbox-inline"><input type="checkbox" checked={intent.reduceContent} onChange={event => update({ reduceContent: event.target.checked })}/><span>{t('recipes.combineReduce')}</span></label>
    <label className="checkbox-inline"><input type="checkbox" checked={intent.resetIdentity} onChange={event => update({ resetIdentity: event.target.checked })}/><span>{t('recipes.combineReset')}</span></label>
  </div>;
}

function CleanupRecipeFields({ id, hostAlias, environmentId }: { id: RecipeId; hostAlias: string | null; environmentId: string }) {
  const { t } = useI18n();
  const [intent, setIntent] = useState<RecipeIntent>(() => readRecipe(id, hostAlias, environmentId));
  if (intent.kind !== 'cleanup') return null;
  function update(patch: Partial<CleanupRecipeIntent>) {
    const next = { ...intent, ...patch } as CleanupRecipeIntent;
    setIntent(next); saveRecipe(id, hostAlias, environmentId, next);
  }
  return <div className="recipe-fields">
    <label className="field">{t('recipes.fieldRecipe')}<select value={intent.recipe} onChange={event => update({ recipe: event.target.value as CleanupRecipeIntent['recipe'] })}><option value="reset_client">{t('cleanup.reset')}</option><option value="retire">{t('cleanup.retire')}</option></select></label>
    <fieldset className="recipe-fields-group"><legend>{t('recipes.fieldCategories')}</legend>{(['instructions', 'memory', 'sessions'] as const).map(category => <label className="checkbox-inline" key={category}><input type="checkbox" checked={intent.categories.includes(category)} onChange={event => update({ categories: event.target.checked ? [...intent.categories, category] : intent.categories.filter(item => item !== category) })}/><span>{t(`category.${category}` as never)}</span></label>)}</fieldset>
    <label className="checkbox-inline"><input type="checkbox" checked={intent.activateInstructions} disabled={!intent.categories.includes('instructions')} onChange={event => update({ activateInstructions: event.target.checked })}/><span>{t('work.activateInstructions')}</span></label>
  </div>;
}
