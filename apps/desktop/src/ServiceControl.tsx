import { useState, type FormEvent } from 'react';
import { request } from './api';
import RequestFailure, { asError } from './RequestFailure';
import type { Environment, Plan, ServiceInspection, ServiceManager, ServicePlan } from './types';
import { Icon, Notice } from './ui';

const stateName = (value: string) => ({ active: '运行中', inactive: '已停止', failed: '启动失败', activating: '正在启动', deactivating: '正在停止' }[value] ?? value);

export function ServiceEvidence({ service }: { service: ServicePlan }) {
  return <section className="service-evidence"><h4>{service.after.hold ? '暂停这一个服务' : '恢复这一个服务'}</h4><div className="fact-row"><span>目标 unit</span><code>{service.unit}</code></div><div className="fact-row"><span>管理器</span><span>{service.manager === 'user' ? '当前用户 systemd' : '系统 systemd'}</span></div><div className="fact-row"><span>服务状态</span><span>{stateName(service.before.active_state)} → {stateName(service.after.active_state)}</span></div><div className="fact-row"><span>原重启策略</span><code>{service.before.restart}</code></div><p>{service.after.hold ? '添加属于本任务的持久暂停项，再停止目标服务并读回。它会阻止目标重新启动；其他服务不在操作范围。' : '核对原 unit 与暂停项，再移除本任务的暂停项。原先运行的服务会重新启动；原先停止的服务保持停止。'}</p><details><summary>查看绑定与暂停来源</summary><code className="full-path">{service.root}</code><code className="full-path">{service.hold.path}</code>{service.original_job && <code className="full-path">原任务 · {service.original_job}</code>}</details></section>;
}

export default function ServiceControl({ send = request, environment, onPlan }: { send?: typeof request; environment: Environment; onPlan: (plan: Plan) => void }) {
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
  return <details className="surface service-control"><summary><span><strong>目标后台服务</strong><small>Linux · 先核对，再暂停；恢复也需确认</small></span><Icon name="chevron" size={16}/></summary><div className="padded"><p>如果这个环境由 systemd 服务运行，先检查准确的 unit。Lintel 只接受明确绑定此配置目录的服务，暂停时保留原配置和启动状态。</p><form onSubmit={inspect} className="service-form"><label className="field">管理器<select value={manager} disabled={!!busy} onChange={event => { setManager(event.target.value as ServiceManager); setInspection(null); setError(null); }}><option value="user">当前用户</option><option value="system">系统服务（需 root）</option></select></label><label className="field">服务 unit<input value={unit} disabled={!!busy} onChange={event => { setUnit(event.target.value); setInspection(null); setError(null); }} placeholder="例如 claude-work.service" spellCheck={false} required/><small>完整 .service 名称；不接受通配符或命令。</small></label><button disabled={!!busy || !unit.trim()} type="submit">{busy === 'inspect' ? '正在核对…' : '检查服务'}</button></form>{error && <div role="alert"><RequestFailure error={error}/></div>}{inspection && <div className="service-result"><div className="surface-heading"><h3>{inspection.unit}</h3><span className="small-label">{inspection.quiesced ? '已暂停并阻止启动' : stateName(inspection.active_state)}</span></div><dl className="service-facts"><div><dt>配置绑定</dt><dd><code>{inspection.root}</code></dd></div><div><dt>自动重启</dt><dd><code>{inspection.restart}</code></dd></div><div><dt>主进程</dt><dd>{inspection.main_pid || '无'}</dd></div><div><dt>触发来源</dt><dd>{inspection.triggered_by.length ? inspection.triggered_by.join('、') : '未发现 socket / timer'}</dd></div></dl>{inspection.limitations.length > 0 && <Notice>{inspection.limitations.join('；')}</Notice>}<div className="service-actions"><span>{inspection.quiesced ? '按原任务恢复；外部编辑有冲突时会保留。' : '生成计划不会停止服务。下一步核对范围并批准。'}</span><button disabled={!!busy || !inspection.bound} onClick={preview}>{busy === 'plan' ? '正在准备…' : inspection.quiesced ? '预览恢复服务' : '预览暂停服务'}<Icon name="arrow" size={14}/></button></div></div>}<p className="small-print">暂停后台服务后，仍需关闭此环境的交互终端与 IDE。其他 supervisor、容器和 macOS 服务不在此适配器范围。</p></div></details>;
}
