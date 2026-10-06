import type { Plan } from './types';
import { Icon, formatBytes } from './ui';

const categories: Record<string, string> = { instructions: '个人指令', memory: '记忆文本', sessions: '会话资料' };

export default function PlanReview({ plan }: { plan: Plan }) {
  const target=plan.planned_target;
  const manifest = target ? { files:target.files,package:plan.import_manifest?.package ?? null } : plan.import_manifest;
  return <>
    {plan.work_selection&&<section className="work-selection-review" aria-label="批准中的原件范围"><h4>{plan.work_selection.mode==='paths'?'仅保全这些原件':'按整类保全原件'}</h4><p>{plan.work_selection.mode==='paths'?'这份精确清单已经冻结。未列出的原件不会加入归档；选中内容改变会要求重新预览。':'归档范围由所选类别的完整原件清单冻结；执行时再次核对。'}</p>{plan.work_selection.paths&&<ul>{plan.work_selection.paths.map(path=><li key={path}><code>{path}</code></li>)}</ul>}</section>}
    {target && <section className="frozen-destination"><span className="eyebrow">{target.create?'将准备新的配置环境':'使用已登记配置环境'}</span><h4>批准中的最终配置 root</h4><code>{target.new_root}</code><p>计划阶段未创建新 root。执行时复查目录身份、文件落点与外部占用。</p></section>}
    {manifest && <section className="import-review" aria-label="最终迁入清单">
      <div className="import-review-heading"><div><span className="eyebrow">from archive to this environment</span><h4>这些文件，会放在这里</h4></div><span>{manifest.files.length} 个文件 · {formatBytes(manifest.files.reduce((sum, file) => sum + file.size, 0))}</span></div>
      <p>以下是本次批准的完整落点；旧版计划使用相对于配置 root 的位置。重名文件已经分配新名称，已有内容保留。</p>
      <ol className="import-file-list">{manifest.files.map(file => <li key={file.destination}>
        <div className="import-file-meta"><span>{categories[file.category] ?? file.category}</span><span>{formatBytes(file.size)}</span></div>
        <div className="import-path"><span>包内来源</span><code>{file.source}</code></div>
        <div className="import-path destination"><span>最终位置</span><code>{file.destination}</code></div>
        <p className="file-purpose">{file.category==='instructions' && (target ? target.activation.instructions : file.destination==='CLAUDE.md') ? "用途：个人指令；客户端可能自动加载" : "用途：待用资料；不注册为活跃原生会话"}</p><details><summary>核对文件摘要</summary><code className="full-path">SHA-256 · {file.sha256}</code></details>
      </li>)}</ol>
      {manifest.package && <details className="import-package"><summary>核对这份工作包</summary><dl><div><dt>格式</dt><dd>{manifest.package.format}</dd></div><div><dt>生成器（包内声明）</dt><dd>{manifest.package.generator ?? '未声明'}</dd></div><div><dt>加密包 SHA-256</dt><dd><code>{manifest.package.sha256}</code></dd></div></dl></details>}
    </section>}
    <div className="plan-columns">
      <section><h4>配置字段 <span>{plan.changes.length} 项变更</span></h4>
        {plan.changes.length ? <div className="change-list">{plan.changes.map((change, index) => <div className="change" key={`${change.key}-${index}`}>
          <strong>{change.label}</strong><div className="change-values"><code>{change.before ?? '未设置'}</code><Icon name="arrow" size={13}/><code>{change.after ?? '移除该项'}</code></div>
          <details><summary>查看来源</summary><code className="full-path">{change.key}</code><code className="full-path">{change.path}</code></details>
        </div>)}</div> : <p className="muted">{manifest ? '本次只迁入上方文件，不修改配置字段。' : '本次不修改配置字段。文件或环境操作见下方实际步骤。'}</p>}
      </section>
      <section className="preserve-column"><h4>将保留</h4><ul className="keep-list">{plan.preserves.map((item, index) => <li key={index}>{item}</li>)}</ul></section>
    </div>
  </>;
}
