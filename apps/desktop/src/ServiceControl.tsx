import { useState, type FormEvent } from 'react';
import { request } from './api';
import RequestFailure, { asError } from './RequestFailure';
import type { Environment, Plan, ServiceInspection, ServiceManager, ServicePlan } from './types';
import { Icon, Notice } from './ui';
import { useI18n } from './i18n-provider';

const stateKeys: Record<string, string> = { active: 'service.stateActive', inactive: 'service.stateInactive', failed: 'service.stateFailed', activating: 'service.stateActivating', deactivating: 'service.stateDeactivating' };

export function ServiceEvidence({ service }: { service: ServicePlan }) {
  const { t } = useI18n();
  const stateName = (value: string) => stateKeys[value] ? t(stateKeys[value]) : value;
  return <section className="service-evidence"><h4>{t(service.after.hold ? 'service.pauseOne' : 'service.resumeOne')}</h4><div className="fact-row"><span>{t('service.targetUnit')}</span><code>{service.unit}</code></div><div className="fact-row"><span>{t('service.manager')}</span><span>{t(service.manager === 'user' ? 'service.userSystemd' : 'service.systemSystemd')}</span></div><div className="fact-row"><span>{t('service.status')}</span><span>{stateName(service.before.active_state)} → {stateName(service.after.active_state)}</span></div><div className="fact-row"><span>{t('service.restartPolicy')}</span><code>{service.before.restart}</code></div><p>{t(service.after.hold ? 'service.pauseNote' : 'service.resumeNote')}</p><details><summary>{t('service.bindingSource')}</summary><code className="full-path">{service.root}</code><code className="full-path">{service.hold.path}</code>{service.original_job && <code className="full-path">{t('service.originalJob',{id:service.original_job})}</code>}</details></section>;
}

export default function ServiceControl({ send = request, environment, onPlan }: { send?: typeof request; environment: Environment; onPlan: (plan: Plan) => void }) {
  const { t } = useI18n();
  const stateName = (value: string) => stateKeys[value] ? t(stateKeys[value]) : value;
  const [manager, setManager] = useState<ServiceManager>('user');
  const [unit, setUnit] = useState('');
  const [inspection, setInspection] = useState<ServiceInspection | null>(null);
  const [busy, setBusy] = useState('');
  const [error, setError] = useState<Error | null>(null);
  async function run(name: string, action: () => Promise<void>) {
    setBusy(name); setError(null);
    try { await action(); } catch (error) { setError(asError(error)); } finally { setBusy(''); }
  }
  function inspect(event: FormEvent) {
    event.preventDefault();
    void run('inspect', async () => setInspection(await send('service_inspect', { environment_id: environment.id, manager, unit: unit.trim() })));
  }
  function preview() {
    if (!inspection) return;
    void run('plan', async () => {
      const plan = inspection.quiesced && inspection.quiesce_job_id
        ? await send('plan_service_resume', { job_id: inspection.quiesce_job_id })
        : await send('plan_service_quiesce', { environment_id: environment.id, manager: inspection.manager, unit: inspection.unit });
      setInspection(null); onPlan(plan);
    });
  }
  return <details className="surface service-control"><summary><span><strong>{t('service.title')}</strong><small>{t('service.subtitle')}</small></span><Icon name="chevron" size={16}/></summary><div className="padded"><p>{t('service.intro')}</p><form onSubmit={inspect} className="service-form"><label className="field">{t('service.managerLabel')}<select value={manager} disabled={!!busy} onChange={event => { setManager(event.target.value as ServiceManager); setInspection(null); setError(null); }}><option value="user">{t('service.currentUser')}</option><option value="system">{t('service.systemService')}</option></select></label><label className="field">{t('service.unitLabel')}<input value={unit} disabled={!!busy} onChange={event => { setUnit(event.target.value); setInspection(null); setError(null); }} placeholder={t('service.unitPlaceholder')} spellCheck={false} required/><small>{t('service.unitNote')}</small></label><button disabled={!!busy || !unit.trim()} type="submit">{busy === 'inspect' ? t('service.checking') : t('service.check')}</button></form>{error && <div role="alert"><RequestFailure error={error}/></div>}{inspection && <div className="service-result"><div className="surface-heading"><h3>{inspection.unit}</h3><span className="small-label">{inspection.quiesced ? t('service.quiesced') : stateName(inspection.active_state)}</span></div><dl className="service-facts"><div><dt>{t('service.configBinding')}</dt><dd><code>{inspection.root}</code></dd></div><div><dt>{t('service.autoRestart')}</dt><dd><code>{inspection.restart}</code></dd></div><div><dt>{t('service.mainPid')}</dt><dd>{inspection.main_pid || t('service.none')}</dd></div><div><dt>{t('service.triggeredBy')}</dt><dd>{inspection.triggered_by.length ? inspection.triggered_by.join(t('list.separator')) : t('service.noTrigger')}</dd></div></dl>{inspection.limitations.length > 0 && <Notice>{inspection.limitations.join(t('list.semicolon'))}</Notice>}<div className="service-actions"><span>{t(inspection.quiesced ? 'service.resumeHint' : 'service.planHint')}</span><button disabled={!!busy || !inspection.bound} onClick={preview}>{busy === 'plan' ? t('service.preparing') : inspection.quiesced ? t('service.previewResume') : t('service.previewPause')}<Icon name="arrow" size={14}/></button></div></div>}<p className="small-print">{t('service.footer')}</p></div></details>;
}
