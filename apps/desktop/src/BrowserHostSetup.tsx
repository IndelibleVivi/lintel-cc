import { useEffect, useRef, useState } from 'react';
import { Icon, Notice, formatBytes } from './ui';
import BrowserExtensionSetup from './BrowserExtensionSetup';
import { useI18n } from './i18n-provider';

type HostPlan = {
  browser: string; extension_id: string; version: string; sha256: string; bytes: number;
  host_path: string; manifest_path: string; manifest: unknown; state_path: string;
  effect: string; existing_manifest: unknown | null;
  status: 'ready' | 'already-registered' | 'conflict';
  install_action: 'install' | 'upgrade' | 'none' | 'blocked';
};
type Props = {
  native: boolean; busy: boolean;
  request: <T>(payload: Record<string, unknown>) => Promise<T>;
  run: (work: () => Promise<void>) => Promise<void>;
  onInstalled: (code: 'installedVerified' | 'installedUpdated' | 'installedNew') => void;
};

export default function BrowserHostSetup({ native, busy, request, run, onInstalled }: Props) {
  const { t } = useI18n();
  const [browser, setBrowser] = useState('chrome');
  const [extensionId, setExtensionId] = useState('');
  const [plan, setPlan] = useState<HostPlan | null>(null);
  const [pendingRecovery, setPendingRecovery] = useState(false);
  const [pendingError, setPendingError] = useState('');
  const queryGeneration = useRef(0);
  useEffect(() => {
    let active = true;
    const generation = ++queryGeneration.current;
    setPendingError(''); setPendingRecovery(false);
    if (native) void request<HostPlan | null>({ op: 'bundled_host_pending', browser }).then(pending => {
      if (active && generation === queryGeneration.current && pending) { setPlan(pending); setExtensionId(pending.extension_id); setPendingRecovery(true); }
    }).catch(error => { if (active && generation === queryGeneration.current) setPendingError(String(error)); });
    return () => { active = false; };
  }, [native, browser]);

  const actionName = t(pendingRecovery ? 'bhost.pendingContinue' : plan?.install_action === 'upgrade' ? 'bhost.approveUpgrade' : plan?.install_action === 'none' ? 'bhost.verifyInstalled' : 'bhost.approveInstall');
  return <div className="browser-host-setup">
    <div className="browser-target-field">
      <label className="field">{t('bhost.targetBrowser')}<select disabled={busy} value={browser} onChange={event => {
        ++queryGeneration.current; setPendingError(''); setPendingRecovery(false); setBrowser(event.target.value); setExtensionId(event.target.value === 'firefox' ? 'lintel@lintel.local' : ''); setPlan(null);
      }}>{[['chrome', 'Chrome'], ['edge', 'Edge'], ['firefox', 'Firefox']].map(([value, title]) => <option key={value} value={value}>{title}</option>)}</select></label>
    </div>
    <BrowserExtensionSetup key={browser} browser={browser} native={native} busy={busy} request={request} run={run}/>
    <h4 className="browser-setup-step"><span>2</span>{t('bhost.installStep')}</h4>
    <label className="field browser-extension-id">{t('bhost.extensionId')}<input disabled={busy} value={extensionId} onChange={event => { ++queryGeneration.current; setExtensionId(event.target.value); setPlan(null); setPendingRecovery(false); }} spellCheck={false} autoComplete="off" placeholder={t('bhost.extensionIdPlaceholder')}/><small>{t(browser === 'firefox' ? 'bhost.firefoxIdNote' : 'bhost.extensionIdNote')}</small></label>
    <p className="small-print">{t('bhost.note')}</p>
    {pendingError && <Notice tone="warning">{t('bhost.pendingError')} {pendingError}</Notice>}
    {pendingRecovery && <Notice>{t('bhost.pendingNotice')}</Notice>}
    <button disabled={busy || !native || !extensionId.trim()} onClick={() => void run(async () => {
      ++queryGeneration.current; setPendingError('');
      const pending = await request<HostPlan | null>({ op: 'bundled_host_pending', browser });
      if (pending) { setExtensionId(pending.extension_id); setPlan(pending); setPendingRecovery(true); }
      else { setPendingRecovery(false); setPlan(null); setPlan(await request<HostPlan>({ op: 'bundled_host_plan', browser, extension_id: extensionId.trim() })); }
    })}>{t('bhost.previewInstall')}<Icon name="arrow" size={14}/></button>
    {plan && <section className="installation-plan browser-host-plan" aria-label={t('bhost.planAria')}>
      <div className="browser-host-plan-heading"><h4>{t(plan.install_action === 'upgrade' ? 'bhost.planUpgrade' : plan.status === 'already-registered' ? 'bhost.planRegistered' : 'bhost.planNew')}</h4><span>v{plan.version} · {formatBytes(plan.bytes)}</span></div>
      <dl className="browser-host-scope">
        <div><dt>{t('bhost.authorizedExtension')}</dt><dd><code>{plan.extension_id}</code><span>{({ chrome: 'Chrome', edge: 'Edge', firefox: 'Firefox' } as Record<string, string>)[plan.browser]}</span></dd></div>
        <div><dt>{t('bhost.hostLocation')}</dt><dd><code className="full-path">{plan.host_path}</code></dd></div>
        <div><dt>{t('bhost.manifestFile')}</dt><dd><code className="full-path">{plan.manifest_path}</code></dd></div>
      </dl>
      {plan.install_action === 'upgrade' && <Notice>{t('bhost.upgradeNote')}</Notice>}
      {plan.install_action === 'blocked' && <Notice tone="warning">{t('bhost.blockedNote')}</Notice>}
      <details className="browser-host-details"><summary>{t('bhost.detailsSummary')}</summary>
        {plan.existing_manifest !== null && <><h5>{t('bhost.currentRegistration')}</h5><pre>{JSON.stringify(plan.existing_manifest, null, 2)}</pre></>}
        <h5>{t('bhost.proposedRegistration')}</h5><pre>{JSON.stringify(plan.manifest, null, 2)}</pre>
        <p>{t('bhost.componentSha')} <code className="full-path">{plan.sha256}</code></p>
        <p>{t('bhost.pairingStateDir')} <code className="full-path">{plan.state_path}</code></p>
      </details>
      <button className="primary" disabled={busy || !native || plan.install_action === 'blocked'} onClick={() => void run(async () => {
        let result: { status: string; plan: HostPlan };
        try { result = await request({ op: 'install_bundled_host', approved_plan: plan }); }
        catch (error) { setPlan(null); throw error; }
        setPlan(null); setPendingRecovery(false);
        onInstalled(result.status === 'already-registered' ? 'installedVerified' : result.status === 'updated' ? 'installedUpdated' : 'installedNew');
      })}>{actionName}</button>
      <p className="small-print">{t('bhost.footer')}</p>
    </section>}
  </div>;
}
