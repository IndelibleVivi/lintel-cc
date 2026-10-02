import { useState } from 'react';
import { RequestError } from './api';
import { Icon, Notice } from './ui';

const stages = { local: '本机准备', ssh: 'SSH 连接', runner: '远端 Lintel runner', response: '读取远端响应' };
export function asError(error: unknown): Error { return error instanceof Error ? error : new Error(String(error)); }

export default function RequestFailure({ error, context }: { error: Error | string; context?: string }) {
  const [copyState, setCopyState] = useState('');
  const diagnostic = error instanceof RequestError ? error.diagnostic : undefined;
  const code = error instanceof RequestError ? error.code : undefined;
  const message = typeof error === 'string' ? error : error.message;
  const summary = diagnostic?.summary ?? message;
  const report = [context, summary, code && `错误代码：${code}`, diagnostic && `阶段：${stages[diagnostic.stage]} / ${diagnostic.reason}`, diagnostic?.exit_code !== undefined && `退出码：${diagnostic.exit_code}`, diagnostic?.next_steps.join('\n'), diagnostic?.command, diagnostic?.submission_uncertain && '远端结果尚不确定；请先核对状态，已提交的任务只查询原 ID，不重复提交。'].filter(Boolean).join('\n');
  async function copy() {
    try { await navigator.clipboard.writeText(report); setCopyState('已复制排查摘要（不含原始错误输出）'); }
    catch { setCopyState('剪贴板不可用，可在下方选中摘要复制。'); }
  }
  return <section className="request-failure" role="alert" aria-label={context ? `${context}：操作未完成` : '操作未完成'}>
    <Notice tone="error"><strong>{context && `${context} · `}{summary}</strong>{diagnostic && message !== summary && <p>{message}</p>}{code && <small>错误代码：{code}{diagnostic && ` · ${stages[diagnostic.stage]}`}{diagnostic?.exit_code !== undefined && ` · 退出码 ${diagnostic.exit_code}`}</small>}</Notice>
    {diagnostic && <div className="failure-guidance"><h4>接下来可以这样查</h4><ol>{diagnostic.next_steps.map((step, i) => <li key={i}>{step}</li>)}</ol>
      {diagnostic.command && <div className="diagnostic-command"><span>在你自己的终端核对；Lintel 不会自动执行</span><code>{diagnostic.command}</code></div>}
      {diagnostic.submission_uncertain && <p className="failure-uncertain">远端可能已收到本次操作，请先核对结果。已提交的任务请保留原 ID 并查询，不要再次提交。</p>}
      {diagnostic.stderr_excerpt && <details><summary>查看本次错误输出{diagnostic.stderr_truncated ? '（已截断）' : ''}</summary><p className="small-print">仅在本次窗口内显示，可能含主机名或本机路径。分享前请自行核对。</p><pre>{diagnostic.stderr_excerpt}</pre></details>}
      <div className="button-row"><button className="text-button" onClick={() => void copy()}><Icon name="copy" size={14}/>复制排查摘要</button><span role="status">{copyState}</span></div>
      {copyState.startsWith('剪贴板') && <pre>{report}</pre>}
    </div>}
  </section>;
}
