import type { Receipt } from './types';
import { Notice } from './ui';
import { useI18n } from './i18n-provider';

export default function ExecutionEvidence({ execution }: { execution: Receipt['execution'] }) {
  const { t } = useI18n();
  if (!execution) return null;
  const limited = execution.mode === 'setsid';
  return <div className="separated execution-evidence">
    <Notice tone={limited ? 'warning' : undefined}>
      <strong>{t(limited ? 'execution.notGuaranteed' : 'execution.handedToHost')}</strong>
      <p>{execution.continuation}</p>
      {execution.limitation && <p>{execution.limitation}</p>}
      <p>{t('execution.retainId')}</p>
    </Notice>
    {execution.unit && <details><summary>{t('execution.hostingDetails')}</summary><div className="fact-row"><span>{t('execution.manager')}</span><span>{t(execution.manager === 'system' ? 'execution.systemManager' : 'execution.userManager')}</span></div><div className="fact-row"><span>{t('execution.unit')}</span><code>{execution.unit}</code></div></details>}
  </div>;
}
