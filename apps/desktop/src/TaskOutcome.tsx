import type {Receipt} from './types';
import {Status} from './ui';
import {useI18n} from './i18n-provider';
import {present} from './i18n-presentation';

const outcomeKeys:Record<string,string>={completed:'outcome.completed',partial:'outcome.partial',blocked:'outcome.blocked',failed:'outcome.failed',uncertain:'outcome.uncertain',query_only:'outcome.queryOnly',in_progress:'outcome.inProgress'};
const coverageKeys:Record<string,string>={done:'coverage.done',not_requested:'coverage.notRequested',not_checked:'coverage.notChecked',unverified:'coverage.unverified'};
export default function TaskOutcome({receipt}:{receipt:Receipt}) {
  const {t,locale}=useI18n();
  const p=(value:string|null|undefined)=>present(value,locale);
  const result=receipt.task_result;
  if(!result)return <div className="receipt-summary"><Status value={receipt.status}/><span>{t('outcome.legacy')}</span></div>;
  return <section className="task-outcome" aria-label={t('outcome.aria')}><h3>{result.primary ? p(result.primary) : outcomeKeys[result.outcome] ? t(outcomeKeys[result.outcome]) : t('outcome.viewOriginal')}</h3>
    {!!result.next_actions.length && <div className="outcome-next"><ul>{result.next_actions.map((action,i)=><li key={i}>{p(action.label)}</li>)}</ul></div>}
    {!!result.coverage.length && <details open={result.coverage.some(c=>c.state!=='done')}><summary>{t('outcome.coverage')}</summary><ul>{result.coverage.map((fact,i)=><li key={i}><strong>{coverageKeys[fact.state] ? t(coverageKeys[fact.state]) : fact.state}</strong> · {p(fact.detail)}</li>)}</ul></details>}
    <details><summary>{t('outcome.protocol')}</summary><div className="fact-row"><span>legacy status</span><Status value={receipt.status}/></div><code className="full-path">{receipt.id}</code></details>
  </section>;
}
