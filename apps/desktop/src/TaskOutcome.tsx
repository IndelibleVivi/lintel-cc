import type {Receipt} from './types';
import {Status} from './ui';
const outcomes:Record<string,string>={completed:'所选步骤已完成',partial:'部分阶段已完成',blocked:'当前任务受阻',failed:'任务未完成',uncertain:'结果待核对',query_only:'沿原任务查询',in_progress:'任务正在进行'};
const coverage:Record<string,string>={done:'已完成',not_requested:'未请求',not_checked:'未检查',unverified:'未验证'};
export default function TaskOutcome({receipt}:{receipt:Receipt}) {
  const result=receipt.task_result;
  if(!result)return <div className="receipt-summary"><Status value={receipt.status}/><span>旧版回执：以各步骤和覆盖范围为准</span></div>;
  return <section className="task-outcome" aria-label="本次所选任务结果"><h3>{result.primary ?? outcomes[result.outcome] ?? '查看原任务结果'}</h3>
    {!!result.next_actions.length && <div className="outcome-next"><ul>{result.next_actions.map((action,i)=><li key={i}>{action.label}</li>)}</ul></div>}
    {!!result.coverage.length && <details open={result.coverage.some(c=>c.state!=='done')}><summary>检查覆盖与限制</summary><ul>{result.coverage.map((fact,i)=><li key={i}><strong>{coverage[fact.state] ?? fact.state}</strong> · {fact.detail}</li>)}</ul></details>}
    <details><summary>协议回执与原标识</summary><div className="fact-row"><span>legacy status</span><Status value={receipt.status}/></div><code className="full-path">{receipt.id}</code></details>
  </section>;
}
