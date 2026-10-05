import { useState } from 'react';
import { RequestError } from './api';
import { Icon, Notice } from './ui';

const stages = { local: '本机准备', ssh: 'SSH 连接', runner: '远端 Lintel runner', response: '读取远端响应' };
const localGuidance:Record<string,string>={
  stale_plan:'目标或文件在预览后发生了变化。返回编辑页重新核对并预览；已有原任务只查询，不重发。',
  plan_changed:'审批内容已改变。保留已有任务 ID，重新审阅尚未执行的计划。',
  writers_running:'关闭列出的目标写入者与自动重启来源，再刷新检查。受管 service 使用独立暂停预览。',
  executable_missing:'没有发现客户端。到环境“启动与来源”检查实际安装路径；版本识别读取静态元数据。',
  environment_retired:'此环境已退役。到环境页核对后恢复登记；这不会恢复已删除的凭据。',
  project_missing:'选择目标主机上已存在的项目目录；配置 root 与项目 cwd 分别核对。',
  unsupported_resume:'此客户端或会话组合尚未获得有限支持证据。原件可继续阅读，或用审阅稿打开新会话。',
  terminal_required:'交互启动需要真实 Terminal／PTY。这里保留原启动请求，不能通过重发解决。',
  stale_archive:'工作包来源已改变。旧阅读位置和选择已清除，重新解锁后核对原件。',
  browser_component_unavailable:'这台执行主机没有浏览器原生组件。到实际 profile 所在的 Mac 与扩展完成配对。',
};
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
    {!diagnostic && code && localGuidance[code] && <div className="failure-guidance"><h4>下一步</h4><p>{localGuidance[code]}</p></div>}
    {diagnostic && <div className="failure-guidance"><h4>接下来可以这样查</h4><ol>{diagnostic.next_steps.map((step, i) => <li key={i}>{step}</li>)}</ol>
      {diagnostic.command && <div className="diagnostic-command"><span>在你自己的终端核对；Lintel 不会自动执行</span><code>{diagnostic.command}</code></div>}
      {diagnostic.submission_uncertain && <p className="failure-uncertain">远端可能已收到本次操作，请先核对结果。已提交的任务请保留原 ID 并查询，不要再次提交。</p>}
      {diagnostic.stderr_excerpt && <details><summary>查看本次错误输出{diagnostic.stderr_truncated ? '（已截断）' : ''}</summary><p className="small-print">仅在本次窗口内显示，可能含主机名或本机路径。分享前请自行核对。</p><pre>{diagnostic.stderr_excerpt}</pre></details>}
      <div className="button-row"><button className="text-button" onClick={() => void copy()}><Icon name="copy" size={14}/>复制排查摘要</button><span role="status">{copyState}</span></div>
      {copyState.startsWith('剪贴板') && <pre>{report}</pre>}
    </div>}
  </section>;
}
