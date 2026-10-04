import type { Receipt } from './types';
import { Notice } from './ui';

export default function ExecutionEvidence({ execution }: { execution: Receipt['execution'] }) {
  if (!execution) return null;
  const limited = execution.mode === 'setsid';
  return <div className="separated execution-evidence">
    <Notice tone={limited ? 'warning' : undefined}>
      <strong>{limited ? '断线后的继续执行未获保证' : '任务已交给主机后台管理'}</strong>
      <p>{execution.continuation}</p>
      {execution.limitation && <p>{execution.limitation}</p>}
      <p>保留原任务 ID；连接中断后只查询这份任务。恢复需要另行预览与批准。</p>
    </Notice>
    {execution.unit && <details><summary>查看任务托管详情</summary><div className="fact-row"><span>管理范围</span><span>{execution.manager === 'system' ? '系统管理器' : '当前用户管理器'}</span></div><div className="fact-row"><span>任务 unit</span><code>{execution.unit}</code></div></details>}
  </div>;
}
