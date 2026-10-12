import { useState } from 'react';
import { Icon, Notice, formatBytes } from './ui';
import { useI18n } from './i18n-provider';

type ExtensionPlan = {
  schema: number; browser: string; package: string; version: string;
  sha256: string; bytes: number; files: { path: string; sha256: string; bytes: number }[];
  extension_path: string; manifest_path: string; effect: string;
  existing_installation: unknown | null;
  recovery?: { phase: string; action: string; message: string } | null;
  status: 'ready' | 'already-installed' | 'conflict';
  install_action: 'install' | 'upgrade' | 'none' | 'blocked';
};
type Props = {
  browser: string; native: boolean; busy: boolean;
  request: <T>(payload: Record<string, unknown>) => Promise<T>;
  run: (work: () => Promise<void>) => Promise<void>;
};

export default function BrowserExtensionSetup({ browser, native, busy, request, run }: Props) {
  const { t } = useI18n();
  const [plan, setPlan] = useState<ExtensionPlan | null>(null);
  const [prepared, setPrepared] = useState<ExtensionPlan | null>(null);
  const [copied, setCopied] = useState(false);
  const firefox = browser === 'firefox';
  const manager = firefox ? 'about:debugging#/runtime/this-firefox' : browser === 'edge' ? 'edge://extensions' : 'chrome://extensions';
  return <section className="browser-extension-setup" aria-label={t('bext.aria')}>
    <h4 className="browser-setup-step"><span>1</span>{t('bext.step')}</h4>
    <p className="small-print">{t('bext.intro')}</p>
    <button disabled={busy || !native} onClick={() => void run(async () => {
      setPlan(null); setPrepared(null); setCopied(false);
      setPlan(await request<ExtensionPlan>({ op: 'bundled_extension_plan', browser }));
    })}>{t('bext.previewDir')}<Icon name="arrow" size={14}/></button>
    {plan && <section className="installation-plan browser-host-plan" aria-label={t('bext.planAria')}>
      <div className="browser-host-plan-heading"><h4>{t(plan.install_action === 'upgrade' ? 'bext.planUpgrade' : plan.install_action === 'none' ? 'bext.planReady' : 'bext.planNew')}</h4><span>v{plan.version} · {formatBytes(plan.bytes)}</span></div>
      <dl className="browser-host-scope"><div><dt>{t('bext.extensionDir')}</dt><dd><code className="full-path">{plan.extension_path}</code></dd></div><div><dt>{t('bext.fileScope')}</dt><dd>{t('bext.fileCount',{count:plan.files.length,browser:firefox ? 'Firefox' : 'Chrome / Edge'})}</dd></div></dl>
      {plan.recovery && <Notice tone="warning">{t('bext.recoveryPrefix')}{t(plan.install_action === 'blocked' ? 'bext.recoveryBlocked' : 'bext.recoveryContinue')}</Notice>}
      {!plan.recovery && plan.install_action === 'upgrade' && <Notice>{t('bext.upgradeNote')}</Notice>}
      {!plan.recovery && plan.install_action === 'blocked' && <Notice tone="warning">{t('bext.blockedNote')}</Notice>}
      <details className="browser-host-details"><summary>{t('bext.detailsSummary')}</summary><p>{t('bext.extensionSha')} <code className="full-path">{plan.sha256}</code></p><ul className="browser-extension-files">{plan.files.map(file => <li key={file.path}><code>{file.path}</code><span>{formatBytes(file.bytes)}</span></li>)}</ul>{plan.existing_installation !== null && <><h5>{t('bext.currentDirRecord')}</h5><pre>{JSON.stringify(plan.existing_installation, null, 2)}</pre></>}{plan.recovery && <><h5>{t('bext.recoveryScope')}</h5><pre>{JSON.stringify(plan.recovery, null, 2)}</pre></>}</details>
      <button className="primary" disabled={busy || !native || plan.install_action === 'blocked'} onClick={() => void run(async () => {
        try {
          const result = await request<{ status: string; plan: ExtensionPlan }>({ op: 'install_bundled_extension', approved_plan: plan });
          setPrepared(result.plan); setPlan(null); setCopied(false);
        } catch (error) { setPlan(null); setPrepared(null); throw error; }
      })}>{t(plan.recovery ? 'bext.approveContinue' : plan.install_action === 'upgrade' ? 'bext.approveUpgrade' : plan.install_action === 'none' ? 'bext.verifyPrepared' : 'bext.approvePrepare')}</button>
      <p className="small-print">{t('bext.filesOnlyNote')}</p>
    </section>}
    {prepared && <div className="browser-extension-ready" role="status">
      <strong>{t('bext.readyTitle')}</strong><code className="full-path">{prepared.extension_path}</code>
      <div className="button-row"><button disabled={busy || !native} onClick={() => void run(async () => { await request({ op: 'reveal_bundled_extension', browser }); })}><Icon name="arrow" size={14}/>{t('bext.revealFinder')}</button><button disabled={busy} onClick={() => void run(async () => { await navigator.clipboard.writeText(firefox ? prepared.manifest_path : prepared.extension_path); setCopied(true); })}><Icon name="copy" size={14}/>{t(copied ? 'bext.copied' : firefox ? 'bext.copyManifest' : 'bext.copyDir')}</button></div>
      <ol className="browser-load-instructions"><li>{t('bext.loadStep1')} <code>{manager}</code>{t('bext.loadStep1b')}</li><li>{firefox ? <>{t('bext.loadFf1')} <code>manifest.json</code>{t('bext.loadFf2')}</> : <>{t('bext.loadCr1')}</>}</li><li>{firefox ? <>{t('bext.idFf1')} <code>lintel@lintel.local</code> {t('bext.idFf2')}</> : <>{t('bext.idCr')}</>}</li></ol>
    </div>}
    <p className="small-print browser-extension-limit">{t(firefox ? 'bext.limitFirefox' : 'bext.limitChromium')}</p>
  </section>;
}
