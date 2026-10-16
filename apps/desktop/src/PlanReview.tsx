import type { Plan } from './types';
import { Icon, formatBytes } from './ui';
import { useI18n } from './i18n-provider';
import { present } from './i18n-presentation';

const categoryKeys: Record<string, string> = { instructions: 'category.instructions', memory: 'category.memory', sessions: 'category.sessions' };

function FrozenTargets({plan}:{plan:Plan}) {
  const {t,locale}=useI18n();
  return <>
    {plan.configuration&&<section className="frozen-destination"><h4>{t('config.file')}</h4><code>{plan.configuration.path}</code><p>{plan.configuration.scope}</p></section>}
    {plan.residue_selection&&<section className="frozen-destination"><h4>{t('residue.selectedCount',{count:plan.residue_selection.files.length})}</h4><p>{t(plan.residue_selection.archive_before_clear?'planReview.encryptedFirst':'planReview.withoutBackup')}</p><ul>{plan.residue_selection.files.map(file=><li key={file.path}><code>{file.path}</code> · {formatBytes(file.bytes)}</li>)}</ul></section>}
    {plan.identity_reset&&<section className="frozen-destination"><h4>{t('identity.title')}</h4><code>{plan.identity_reset.path}</code><p>{t('planReview.identityFields',{keys:plan.identity_reset.remove_keys.join(', ')})}</p><p>{t('planReview.retainedFields',{keys:plan.identity_reset.retained_keys.join(', ')})}</p><p>{t('residue.combineResetIdentityDetail')}</p><ul>{plan.identity_reset.backup_paths.map(path=><li key={path}><code>{path}</code></li>)}</ul></section>}
    {plan.residue_restore&&<section className="frozen-destination"><h4>{t('planReview.restoreOriginal')}</h4><p>{t('residueArchive.restoreNote')}</p><ul>{plan.residue_restore.files.map(file=><li key={file.path}><code>{file.destination}</code> · {formatBytes(file.bytes)}</li>)}</ul></section>}
    {plan.workflow&&<section className="frozen-destination"><h4>{t('residue.combinedPreview')}</h4><ol>{plan.workflow.map(step=><li key={step.id}><strong>{present(step.title,locale)}</strong><FrozenTargets plan={step}/></li>)}</ol></section>}
  </>;
}

export default function PlanReview({ plan }: { plan: Plan }) {
  const { t, locale } = useI18n();
  const p = (value: string | null | undefined) => present(value, locale);
  const target=plan.planned_target;
  const manifest = target ? { files:target.files,package:plan.import_manifest?.package ?? null } : plan.import_manifest;
  return <>
    <FrozenTargets plan={plan}/>
    {plan.work_selection&&<section className="work-selection-review" aria-label={t('planReview.selectionAria')}><h4>{t(plan.work_selection.mode==='paths'?'planReview.exactOnly':'planReview.wholeCategory')}</h4><p>{t(plan.work_selection.mode==='paths'?'planReview.exactNote':'planReview.wholeNote')}</p>{plan.work_selection.paths&&<ul>{plan.work_selection.paths.map(path=><li key={path}><code>{path}</code></li>)}</ul>}</section>}
    {target && <section className="frozen-destination"><span className="eyebrow">{t(target.create?'planReview.newEnv':'planReview.registeredEnv')}</span><h4>{t('planReview.finalRoot')}</h4><code>{target.new_root}</code><p>{t('planReview.rootNote')}</p></section>}
    {manifest && <section className="import-review" aria-label={t('planReview.importAria')}>
      <div className="import-review-heading"><div><span className="eyebrow">from archive to this environment</span><h4>{t('planReview.importTitle')}</h4></div><span>{t('planReview.importCount',{count:manifest.files.length,bytes:formatBytes(manifest.files.reduce((sum, file) => sum + file.size, 0))})}</span></div>
      <p>{t('planReview.importNote')}</p>
      <ol className="import-file-list">{manifest.files.map(file => <li key={file.destination}>
        <div className="import-file-meta"><span>{categoryKeys[file.category] ? t(categoryKeys[file.category]) : file.category}</span><span>{formatBytes(file.size)}</span></div>
        <div className="import-path"><span>{t('planReview.packageSource')}</span><code>{file.source}</code></div>
        <div className="import-path destination"><span>{t('planReview.finalLocation')}</span><code>{file.destination}</code></div>
        <p className="file-purpose">{file.category==='instructions' && (target ? target.activation.instructions : file.destination==='CLAUDE.md') ? t('planReview.purposeInstructions') : t('planReview.purposeReference')}</p><details><summary>{t('planReview.checkDigest')}</summary><code className="full-path">SHA-256 · {file.sha256}</code></details>
      </li>)}</ol>
      {manifest.package && <details className="import-package"><summary>{t('planReview.checkPackage')}</summary><dl><div><dt>{t('planReview.format')}</dt><dd>{manifest.package.format}</dd></div><div><dt>{t('planReview.generator')}</dt><dd>{manifest.package.generator ?? t('planReview.notDeclared')}</dd></div><div><dt>{t('planReview.packageSha')}</dt><dd><code>{manifest.package.sha256}</code></dd></div></dl></details>}
    </section>}
    <div className="plan-columns">
      <section><h4>{t('planReview.configFields')} <span>{t('planReview.changeCount',{count:plan.changes.length})}</span></h4>
        {plan.changes.length ? <div className="change-list">{plan.changes.map((change, index) => <div className="change" key={`${change.key}-${index}`}>
          <strong>{p(change.label)}</strong><div className="change-values"><code>{change.masked && change.before_present!==false ? '••••••' : change.before ?? t('planReview.notSet')}</code><Icon name="arrow" size={13}/><code>{change.masked && (change.after_present===true || change.after!==null) ? '••••••' : change.after ?? t('planReview.removeItem')}</code></div>
          <details><summary>{t('planReview.viewSource')}</summary><code className="full-path">{change.key}</code><code className="full-path">{change.path}</code></details>
        </div>)}</div> : <p className="muted">{t(manifest ? 'planReview.onlyFiles' : 'planReview.noFieldChanges')}</p>}
      </section>
      <section className="preserve-column"><h4>{t('planReview.willKeep')}</h4><ul className="keep-list">{plan.preserves.map((item, index) => <li key={index}>{p(item)}</li>)}</ul></section>
    </div>
  </>;
}
