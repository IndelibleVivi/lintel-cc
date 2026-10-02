import { useCallback, useEffect, useRef, useState, type FormEvent } from 'react';
import { request, RequestError, transport } from './api';
import BrowserPanel from './BrowserPanel';
import NetworkPanel from './NetworkPanel';
import type { Capability, Draft, Drift, Environment, Inspection, Plan, Receipt } from './types';
import { Brand, Icon, Modal, Notice, Status, formatBytes, formatDate, label, type IconName } from './ui';

type Page = 'environments' | 'policy' | 'rebuild' | 'history';
type Tab = 'overview' | 'outbound' | 'state' | 'launch';
type Flow = { environment: Environment; plan?: Plan; receipt?: Receipt; uncertain?: boolean };
const nav: { id: Page; title: string; icon: IconName }[] = [ { id: 'environments', title: '环境', icon: 'environments' }, { id: 'policy', title: '保护方案', icon: 'policy' }, { id: 'rebuild', title: '清理与重建', icon: 'rebuild' }, { id: 'history', title: '记录与恢复', icon: 'history' } ];
const workClasses = [{ id: 'instructions', title: '个人指令', detail: '纯文本指令；不运行其中引用的命令' }, { id: 'memory', title: '记忆文本', detail: '原始文本迁入待用区；不自动接管新环境记忆' }, { id: 'sessions', title: '会话资料', detail: '以原始资料迁入待用区；不宣称可继续旧会话' }];
const defaultDraft: Draft = { preset: 'reduce', keepRemoteControl: false };
function loadDrafts(): Record<string, Draft> { try { return JSON.parse(localStorage.getItem('lintel.drafts') ?? '{}'); } catch { return {}; } }
function errorText(error: unknown) { return error instanceof RequestError ? `${error.message}（${error.code}）` : error instanceof Error ? error.message : '操作未完成，请重新检查本地执行器。'; }

export default function App() {
  const [page, setPage] = useState<Page>('environments');
  const [tab, setTab] = useState<Tab>('overview');
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [showInspector, setShowInspector] = useState(false);
  const [environments, setEnvironments] = useState<Environment[]>([]);
  const [capabilities, setCapabilities] = useState<Capability[]>([]);
  const [selectedId, setSelectedId] = useState(localStorage.getItem('lintel.selected') ?? '');
  const [inspection, setInspection] = useState<Inspection | null>(null);
  const [jobs, setJobs] = useState<Receipt[]>([]);
  const [drafts, setDrafts] = useState<Record<string, Draft>>(loadDrafts);
  const [categories, setCategories] = useState(['instructions', 'memory', 'sessions']);
  const [busy, setBusy] = useState('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [notification, setNotification] = useState('');
  const [flow, setFlow] = useState<Flow | null>(null);
  const [archivePassphrase, setArchivePassphrase] = useState('');
  const [archiveConfirmation, setArchiveConfirmation] = useState('');
  const [dialog, setDialog] = useState<'add' | 'settings' | 'help' | 'support' | null>(null);
  const [addMode, setAddMode] = useState<'register'|'create'>('create');
  const [name, setName] = useState('');
  const [root, setRoot] = useState('');
  const [formError, setFormError] = useState('');
  const [drift, setDrift] = useState<Drift | null>(null);
  const [support, setSupport] = useState('');
  const [theme, setTheme] = useState(localStorage.getItem('lintel.theme') ?? 'system');
  const [lastCheck, setLastCheck] = useState<Date | null>(null);
  const [receiptFilter, setReceiptFilter] = useState<'all'|'target'>('all');
  const executionLock = useRef(false);
  useEffect(() => { setArchivePassphrase(''); setArchiveConfirmation(''); }, [flow?.plan?.id, flow?.receipt?.id]);
  const selected = environments.find(env => env.id === selectedId);
  const draft = drafts[selectedId] ?? defaultDraft;
  const targetJobs = jobs.filter(job => job.environment_id === selectedId);

  useEffect(() => { document.documentElement.dataset.theme = theme; localStorage.setItem('lintel.theme', theme); }, [theme]);
  useEffect(() => { localStorage.setItem('lintel.drafts', JSON.stringify(drafts)); }, [drafts]);
  useEffect(() => { localStorage.setItem('lintel.selected', selectedId); }, [selectedId]);
  useEffect(() => { if (!notification) return; const timeout = setTimeout(() => setNotification(''), 6000); return () => clearTimeout(timeout); }, [notification]);

  const refresh = useCallback(async () => {
    const [inventory, history] = await Promise.all([request('discover', {}), request('jobs', {})]);
    setEnvironments(inventory.environments); setCapabilities(inventory.capabilities); setJobs(history.jobs);
    setSelectedId(current => inventory.environments.some(env => env.id === current) ? current : inventory.environments[0]?.id ?? '');
    setLastCheck(new Date());
  }, []);
  useEffect(() => { refresh().catch(error => setError(errorText(error))).finally(() => setLoading(false)); }, [refresh]);
  useEffect(() => {
    let active = true; setInspection(null); setDrift(null);
    if (selectedId) request('inspect', { environment_id: selectedId }).then(data => { if (active) setInspection(data); }).catch(error => { if (active) setError(errorText(error)); });
    return () => { active = false; };
  }, [selectedId, lastCheck]);

  async function perform(action: string, work: () => Promise<void>) { setBusy(action); setError(''); try { await work(); } catch (error) { setError(errorText(error)); } finally { setBusy(''); } }
  function updateDraft(value: Partial<Draft>) { setDrafts(current => ({ ...current, [selectedId]: { ...(current[selectedId] ?? defaultDraft), ...value } })); }
  function selectEnvironment(id: string) { setSelectedId(id); setError(''); }
  async function copy(value: string) { try { await navigator.clipboard.writeText(value); setNotification('已复制到剪贴板'); } catch { setError('剪贴板不可用。请在展开的完整文本中选择并复制。'); } }
  const openAdd = (mode: 'register'|'create') => { setAddMode(mode); setFormError(''); setName(''); setRoot(''); setDialog('add'); };
  async function addEnvironment(event: FormEvent) {
    event.preventDefault(); if (!name.trim() || (addMode === 'register' && !root.trim())) return;
    setBusy('add'); setFormError('');
    try { const environment = addMode === 'create' ? await request('create_environment', { name: name.trim() }) : await request('register', { name: name.trim(), root: root.trim() }); await refresh(); setSelectedId(environment.id); setPage('environments'); setDialog(null); setNotification(addMode === 'create' ? '新环境已建立。登录与实际启动仍需单独确认。' : '环境已登记'); }
    catch (error) { setFormError(errorText(error)); } finally { setBusy(''); }
  }
  function preview(kind: 'policy'|'rebuild') {
    if (!selected) return; const target = selected;
    void perform('plan', async () => { const plan = kind === 'policy' ? await request('plan_policy', { environment_id: target.id, preset: draft.preset, keep_remote_control: draft.keepRemoteControl }) : await request('plan_reset', { environment_id: target.id, recipe: 'rebuild', categories }); setFlow({ environment: target, plan }); });
  }
  async function executePlan() {
    if (!flow?.plan || executionLock.current || flow.uncertain || flow.receipt) return;
    if (flow.plan.archive_passphrase_required && (archivePassphrase.length < 12 || archivePassphrase !== archiveConfirmation)) return;
    const current = flow; executionLock.current = true; setBusy('execute'); setError('');
    try { const receipt = await request('execute', { plan_id: current.plan!.id, approval: current.plan!.hash, ...(current.plan!.archive_passphrase_required ? { archive_passphrase: archivePassphrase } : {}) }); setFlow({ ...current, receipt }); await refresh(); }
    catch (error) { setFlow({ ...current, uncertain: true }); setError(errorText(error)); }
    finally { executionLock.current = false; setBusy(''); }
  }
  function reconcile() {
    if (!flow?.plan) return;
    void perform('reconcile', async () => { const found = await request('job', { job_id: flow.plan!.id }); setFlow({ ...flow, receipt: found, uncertain: false }); await refresh(); });
  }
  function restore(receipt: Receipt) {
    const target = environments.find(env => env.id === receipt.environment_id);
    if (!target) { setError('原环境不在当前清单中，无法准备恢复。请重新检查环境。'); return; }
    void perform('restore', async () => { const plan = await request('plan_restore', { job_id: receipt.id }); setFlow({ environment: target, plan }); });
  }
  function launch(environment: Environment) { void perform('launch', async () => { const result = await request('launch', { environment_id: environment.id }); setNotification(`${label(result.status)} · ${result.message}`); }); }
  function checkDrift() { if (selected) void perform('drift', async () => { setDrift(await request('drift', { environment_id: selected.id })); }); }
  function supportPreview() { void perform('support', async () => { setSupport(JSON.stringify(await request('export_support', {}), null, 2)); setDialog('support'); }); }
  function Path({ value }: { value: string }) { return <div className="path"><code title={value}>{value}</code><button className="icon-button" aria-label="复制完整路径" onClick={() => void copy(value)}><Icon name="copy" size={14}/></button></div>; }
  function Warnings({ items }: { items: string[] }) { return <>{items.length > 0 && <Notice tone="warning"><ul className="compact-list">{items.map((item, index) => <li key={index}>{item}</li>)}</ul></Notice>}</>; }

  return <div className={`app-shell ${sidebarCollapsed ? 'sidebar-collapsed' : ''}`}>
    <aside className="sidebar">
      <div className="brand"><strong>Lintel</strong><button className="icon-button" aria-label={sidebarCollapsed ? '展开侧栏' : '收起侧栏'} onClick={() => setSidebarCollapsed(value => !value)}><Icon name="sidebar" size={18}/></button></div>
      <button className="new-space" onClick={() => openAdd('create')}><span><Icon name="plus" size={15}/></span><span>新建环境</span></button>
      <nav aria-label="主导航">{nav.map(item => <button key={item.id} className={`nav-item ${page === item.id ? 'active' : ''}`} title={item.title} aria-current={page === item.id ? 'page' : undefined} onClick={() => setPage(item.id)}><Icon name={item.icon}/><span>{item.title}</span></button>)}</nav>
      <div className="sidebar-environments"><div className="nav-caption">你的环境<button className="icon-button" aria-label="添加现有环境" onClick={() => openAdd('register')}><Icon name="plus" size={13}/></button></div>{environments.map(env => <button key={env.id} className={`sidebar-environment ${env.id === selectedId ? 'chosen' : ''}`} onClick={() => { selectEnvironment(env.id); setPage('environments'); }} title={env.name}>{env.name.replace('Claude Code · ', '')}</button>)}</div>
      <div className="sidebar-bottom"><button className="nav-item" onClick={() => setDialog('help')} title="帮助"><Icon name="help"/><span>帮助</span></button><button className="profile-settings" onClick={() => setDialog('settings')} title="设置与模块"><span className="profile-symbol">L</span><span><strong>本地工作空间</strong><small>Lintel · 开发候选</small></span><Icon name="chevron" size={14}/></button></div>
    </aside>
    <div className="workspace">
      <header className={`target-bar ${page === 'environments' ? 'home-target' : ''}`}><div className="target-icon"><Icon name="environments" size={19}/></div><div className="target-context"><span className="eyebrow">当前操作目标</span>{selected ? <div className="target-line"><span>{label(selected.host)}</span><span className="slash">/</span><select aria-label="当前环境" value={selectedId} onChange={event => selectEnvironment(event.target.value)}>{environments.map(env => <option key={env.id} value={env.id}>{env.name}</option>)}</select></div> : <strong>尚未选择环境</strong>}</div><span className="connection"><span className="status-dot"/>{transport === 'native' ? '本地执行器' : transport === 'synthetic' ? '合成测试执行器' : '桌面连接不可用'}</span><button className="icon-button refresh-button" aria-label="重新检查环境" disabled={!!busy || loading} onClick={() => void perform('refresh', refresh)}><Icon name="refresh"/></button></header>
      {transport === 'synthetic' && <div className="fixture-banner"><Icon name="terminal" size={15}/><span>测试空间 · 操作仅作用于合成数据</span></div>}
      <main className={`main-content ${page === 'environments' ? 'home-content' : ''}`} id="main-content">
        {error && !flow && <div role="alert" className="error-wrap"><Notice tone="error">{error}</Notice><button className="text-button" onClick={() => setError('')}>收起</button></div>}
        {loading ? <div className="empty-state"><span className="loading-mark"/><h2>正在检查本地环境</h2><p>只读取已知目录与安装信息，不启动 Claude。</p></div> : transport === 'unavailable' ? <div className="empty-state"><Brand/><h1>从桌面应用开始</h1><p>此浏览器页面没有本机执行权限。<br/>开发预览请显式运行 <code>npm run dev:synthetic</code>。</p><button onClick={() => setDialog('help')}>查看使用说明</button></div> : <>
          {page === 'environments' && <>
            <section className="home-stage"><div className="home-caption">你的设备 · 你的选择</div><h1 className="home-title"><Brand/><span>给 Claude，一个清爽的开始</span></h1>
            {environments.length === 0 ? <div className="empty-state bordered"><h2>从一个环境开始</h2><p>添加现有 Claude Code 配置目录，或建立新的专用环境。</p><div className="button-row"><button onClick={() => openAdd('register')}>添加现有环境</button><button className="primary" onClick={() => openAdd('create')}>建立专用环境<Icon name="arrow" size={16}/></button></div></div> : <>
              {selected && <div className="environment-composer"><div className="composer-body"><select aria-label="选择要保护的环境" value={selectedId} onChange={event => selectEnvironment(event.target.value)}>{environments.map(env => <option key={env.id} value={env.id}>{env.name}</option>)}</select><p>关闭不必要的外发，保留你的工作内容。</p></div><div className="composer-footer"><button className="icon-button composer-plus" aria-label="添加现有环境" onClick={() => openAdd('register')}><Icon name="plus" size={19}/></button><div className="composer-policy"><Icon name="policy" size={16}/><select aria-label="首页保护方案" value={draft.preset} onChange={event => updateDraft({ preset: event.target.value as Draft['preset'] })}><option value="reduce">减少外发</option><option value="preserve">保持功能</option></select></div><button className="primary composer-submit" aria-label="预览变更" title="预览变更" disabled={!!busy} onClick={() => preview('policy')}><Icon name="arrow" size={19}/></button></div></div>}
              <div className="home-shortcuts"><button onClick={() => setPage('policy')}><Icon name="policy" size={16}/>调整方案</button><button onClick={() => { setTab('state'); setShowInspector(true); }}><Icon name="environments" size={16}/>工作内容</button><button onClick={() => setPage('rebuild')}><Icon name="rebuild" size={16}/>重建环境</button><button onClick={() => setPage('history')}><Icon name="history" size={16}/>变更记录</button></div>
              <div className="home-assurance">每次修改先预览。工作内容留在本机。</div>
              {selected && <div className="inspector-toggle"><button className="text-button" aria-expanded={showInspector} onClick={() => setShowInspector(value => !value)}>环境详情<Icon name="chevron" size={13}/></button><span>{lastCheck ? `${lastCheck.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })} 已检查` : '尚未检查'}</span><button className="icon-button" aria-label="重新检查环境" disabled={!!busy} onClick={() => void perform('refresh', refresh)}><Icon name="refresh" size={14}/></button></div>}
              {selected && showInspector && <section className="detail-panel"><div className="tabs" role="tablist" aria-label="环境详情" onKeyDown={event => { const ids: Tab[] = ['overview','outbound','state','launch']; let next = ids.indexOf(tab); if (event.key === 'ArrowRight') next = (next + 1) % ids.length; else if (event.key === 'ArrowLeft') next = (next + ids.length - 1) % ids.length; else if (event.key === 'Home') next = 0; else if (event.key === 'End') next = ids.length - 1; else return; event.preventDefault(); setTab(ids[next]); (event.currentTarget.querySelectorAll('button')[next] as HTMLButtonElement).focus(); }}>{([['overview','概况'],['outbound','外发与权限'],['state','本地状态'],['launch','启动与来源']] as const).map(([id,title]) => <button role="tab" tabIndex={tab === id ? 0 : -1} aria-selected={tab === id} aria-controls="environment-tab-content" key={id} onClick={() => setTab(id)}>{title}</button>)}</div><div className="detail-body" id="environment-tab-content" role="tabpanel">
                {!inspection ? <p className="muted">正在读取此环境…</p> : <>
                  {tab === 'overview' && <><div className="overview-grid"><button className="overview-item" onClick={() => setPage('policy')}><span className="feature-icon blue"><Icon name="policy" size={20}/></span><span><strong>外发控制</strong><small>{inspection.settings.filter(setting => setting.value === '1').length} / {inspection.settings.length} 项已设置关闭</small></span><Icon name="chevron" size={14}/></button><button className="overview-item" onClick={() => setTab('state')}><span className="feature-icon violet"><Icon name="environments" size={20}/></span><span><strong>工作内容</strong><small>{inspection.assets.filter(asset => ['instructions','memory','sessions'].includes(asset.category)).reduce((sum,asset) => sum + asset.count,0)} 项可识别资料</small></span><Icon name="chevron" size={14}/></button><button className="overview-item" onClick={() => setPage('history')}><span className="feature-icon amber"><Icon name="history" size={20}/></span><span><strong>变更记录</strong><small>{targetJobs.length ? `${targetJobs.length} 次任务 · 查看结果` : '还没有执行过任务'}</small></span><Icon name="chevron" size={14}/></button></div><details className="environment-diagnostics"><summary><span><Icon name="help" size={15}/>配置来源与验证范围</span><Icon name="chevron" size={14}/></summary><div className="fact-row"><span>配置目录</span><Path value={selected.root}/></div><div className="fact-row"><span>验证范围</span><span>已读取配置；实际启动与网络约束需分别验证</span></div><Warnings items={inspection.warnings}/></details></>}
                  {tab === 'outbound' && <><SettingsRows settings={inspection.settings}/><NetworkPanel key={selected.id} environmentId={selected.id} executable={!!selected.executable}/><div className="separated"><h3>浏览器权限</h3><p className="muted">在设置与模块中连接具体浏览器 profile。网络配置、连接观察和强约束是独立能力。</p><button className="text-button" onClick={() => setDialog('settings')}>查看已接入模块<Icon name="arrow" size={14}/></button></div></>}
                  {tab === 'state' && <><div className="asset-grid">{inspection.assets.map(asset => <div className="asset-row" key={asset.category}><span>{categoryLabel(asset.category)}</span><strong>{asset.count} 项</strong><span className="muted">{formatBytes(asset.bytes)}</span></div>)}</div><Notice>数量来自可识别文件的元数据，不代表凭据已隔离或旧状态已清理。</Notice><button className="text-button section-action" onClick={() => setPage('rebuild')}>选择保留内容并准备重建<Icon name="arrow" size={15}/></button></>}
                  {tab === 'launch' && <><div className="fact-row"><span>启动程序</span>{selected.executable ? <Path value={selected.executable}/> : <span>尚未定位；不会擅自安装 Claude</span>}</div><div className="fact-row"><span>配置根</span><Path value={selected.root}/></div><div className="fact-row"><span>已有会话</span><span>不会自动退出或重启</span></div><Notice>新启动使用此环境的准确程序与配置根。其他终端、IDE 与服务入口的生效状态需独立确认。</Notice></>}
                </>}
              </div></section>}
              {selected && showInspector && <div className="drift-block"><div><h3>上次之后，有什么变化？</h3><p>检查已管理的设置，由你决定是否接受。</p></div><button disabled={!!busy} onClick={checkDrift}><Icon name="refresh" size={15}/>检查变化</button>{drift && <div className="drift-result"><Status value={drift.status}/>{drift.changes.length > 0 ? <><SettingsRows settings={drift.changes}/><div className="button-row"><button onClick={() => setPage('policy')}>重新应用方案</button><button disabled={!!busy} onClick={() => void perform('accept', async () => { await request('accept_drift', { environment_id: selected.id }); setDrift(await request('drift', { environment_id: selected.id })); setNotification('已将当前值记为此环境的基线'); })}>接受当前值</button></div></> : <span>当前检查范围内没有待处理差异。</span>}</div>}</div>}
            </>}
            </section>
          </>}
          {page === 'policy' && <>
            <div className="page-heading"><div><h1>保护方案</h1><p>选好需要保留的功能，修改先留在当前环境的草案里。</p></div></div>
            {!selected ? <SelectTarget onAdd={() => openAdd('register')}/> : <>
              <fieldset className="preset-grid"><legend className="sr-only">保护方案</legend><label className={`preset ${draft.preset === 'preserve' ? 'chosen' : ''}`}><input type="radio" name="preset" value="preserve" checked={draft.preset === 'preserve'} onChange={() => updateDraft({ preset: 'preserve' })}/><span className="preset-copy"><strong>保持功能</strong><span>关闭错误回报与反馈调查。<br/>保留指标和主动反馈的当前设置。</span></span></label><label className={`preset ${draft.preset === 'reduce' ? 'chosen' : ''}`}><input type="radio" name="preset" value="reduce" checked={draft.preset === 'reduce'} onChange={() => updateDraft({ preset: 'reduce' })}/><span className="preset-copy"><strong>减少外发</strong><span>分别控制产品指标、错误回报、<br/>主动反馈与调查。</span></span></label></fieldset>
              <section className="surface"><div className="surface-heading"><h2>保留你需要的能力</h2><span className="small-label">仅当前环境</span></div><label className="checkbox-row"><span><strong>保留 Remote Control</strong><span>保留所需的指标 / feature-flag 通道；这时不能宣称产品指标已关闭。</span></span><input type="checkbox" checked={draft.keepRemoteControl} onChange={event => updateDraft({ keepRemoteControl: event.target.checked })}/></label><div className="preserve-note"><Icon name="check" size={16}/><span>官方更新、WebFetch 安全预检查、用户 OTel 与通用代理保持原样。</span></div></section>
              <section className="surface"><div className="surface-heading"><h2>当前配置</h2><span className="small-label">读回值 ≠ 实际运行证明</span></div>{inspection ? <SettingsRows settings={inspection.settings}/> : <p className="padded muted">正在读取设置…</p>}</section>
              <Notice>配置将在 core 生成的计划中确定。当前打开的 Claude 会话不会被自动退出，生效时机以每项结果为准。</Notice>
              <div className="action-bar"><div><strong>草案已保留</strong><span>仅作用于 {selected.name}</span></div><button className="primary" disabled={!!busy} onClick={() => preview('policy')}>{busy === 'plan' ? '正在生成计划…' : '预览变更'}<Icon name="arrow" size={16}/></button></div>
            </>}
          </>}
          {page === 'rebuild' && <>
            <div className="page-heading"><div><h1>保住内容，重新开始</h1><p>准备独立的新环境，选择迁入的工作内容。</p></div></div>
            {!selected ? <SelectTarget onAdd={() => openAdd('register')}/> : <>
              <Notice tone="warning"><strong>此版本提供“新环境重建”。</strong>选定工作内容会加密归档。旧目录、登录与浏览器数据不会因此被清除；远端注销、写入者暂停及原地清理尚未接入。不能把新目录建立视为旧状态已清场。</Notice>
              <div className="rebuild-split"><section className="surface"><div className="surface-heading"><h2>将处理</h2><Icon name="rebuild" size={18}/></div><div className="padded"><ol className="process-list"><li><span>01</span><div><strong>加密归档选定内容</strong><p>由 core 识别工作类别，使用你设置的口令加密。</p></div></li><li><span>02</span><div><strong>建立新的配置目录</strong><p>使用独立环境名称与启动入口。</p></div></li><li><span>03</span><div><strong>迁入批准的工作内容</strong><p>指令可直接使用；会话与记忆保留原始资料，不自动启用执行配置。</p></div></li></ol></div></section><section className="surface"><div className="surface-heading"><h2>将保留</h2><Icon name="check" size={18}/></div><div className="padded"><ul className="keep-list"><li>原环境与原始工作内容</li><li>未选择迁入的资料</li><li>其他环境、服务与登录</li><li>通用代理与系统网络设置</li></ul><p className="muted">新环境不证明账户已解除关联；正常登录由官方流程完成。</p></div></section></div>
              <section className="surface"><div className="surface-heading"><h2>迁入哪些工作内容</h2><span className="small-label">可逐项调整</span></div>{workClasses.map(category => <label className="checkbox-row" key={category.id}><span><strong>{category.title}</strong><span>{category.detail}</span></span><input type="checkbox" checked={categories.includes(category.id)} onChange={event => setCategories(current => event.target.checked ? [...current, category.id] : current.filter(item => item !== category.id))}/></label>)}</section>
              <div className="action-bar"><div><strong>{selected.name}</strong><span>先预览范围，再批准建立新环境</span></div><button className="primary" disabled={!!busy} onClick={() => preview('rebuild')}>{busy === 'plan' ? '正在生成计划…' : '准备重建'}<Icon name="arrow" size={16}/></button></div>
            </>}
          </>}
          {page === 'history' && <>
            <div className="page-heading"><div><h1>每一步，都有记录</h1><p>查看真实执行结果；可恢复的配置会先检查后续编辑。</p></div><button disabled={!!busy} onClick={() => void perform('refresh', refresh)}><Icon name="refresh" size={16}/>刷新</button></div>
            <div className="history-toolbar"><div className="segmented"><button aria-pressed={receiptFilter === 'all'} onClick={() => setReceiptFilter('all')}>全部环境</button><button aria-pressed={receiptFilter === 'target'} onClick={() => setReceiptFilter('target')}>当前环境</button></div><span className="muted">关闭窗口不会删除任务记录</span></div>
            {(receiptFilter === 'all' ? jobs : targetJobs).length === 0 ? <div className="empty-state bordered"><Icon name="history" size={32}/><h2>还没有执行记录</h2><p>应用一份方案后，这里会保留每一步的结果与恢复入口。</p><button onClick={() => setPage('policy')}>选择保护方案</button></div> : <div className="job-list">{(receiptFilter === 'all' ? jobs : targetJobs).map(job => { const env = environments.find(item => item.id === job.environment_id); return <div className="job-row" key={job.id}><span className="job-icon"><Icon name="history" size={19}/></span><div className="job-copy"><strong>{job.title}</strong><span>{env?.name ?? job.environment_id} · {formatDate(job.created_at)}</span><code>{job.id}</code></div><Status value={job.status}/><button onClick={() => { if (env) void perform('query', async () => { const receipt = await request('job', { job_id: job.id }); setFlow({ environment: env, receipt }); }); else setError('未找到该记录对应的环境，请重新检查环境清单。'); }}>查看结果</button></div>; })}</div>}
            <div className="support-footer"><span>需要排查问题？先预览脱敏后的支持资料。</span><button className="text-button" disabled={!!busy} onClick={supportPreview}>预览支持资料<Icon name="arrow" size={15}/></button></div>
          </>}
        </>}
      </main>
    </div>
    {notification && <div className="toast" role="status"><Icon name="check" size={17}/>{notification}<button className="icon-button" aria-label="关闭提示" onClick={() => setNotification('')}><Icon name="close" size={14}/></button></div>}
    {dialog === 'add' && <Modal title={addMode === 'create' ? '建立专用环境' : '添加现有环境'} onClose={() => { if (!busy) setDialog(null); }}><form onSubmit={addEnvironment}><div className="modal-body"><div className="segmented add-tabs"><button type="button" aria-pressed={addMode === 'create'} onClick={() => setAddMode('create')}>建立新环境</button><button type="button" aria-pressed={addMode === 'register'} onClick={() => setAddMode('register')}>登记已有目录</button></div><p>{addMode === 'create' ? 'Lintel 会建立新的配置根，不复制现有登录、凭据或执行配置。' : '添加明确的 Claude Code 配置目录。登记不会改变目录中的设置。'}</p><label className="field">环境名称<input required maxLength={100} value={name} onChange={event => setName(event.target.value)} placeholder="例如：写作环境"/></label>{addMode === 'register' && <label className="field">配置目录完整路径<input required value={root} onChange={event => setRoot(event.target.value)} placeholder="/path/to/claude-config" spellCheck={false}/><small>这是配置根，不是项目代码目录。</small></label>}{formError && <div role="alert"><Notice tone="error">{formError}</Notice></div>}</div><div className="modal-footer"><button type="button" disabled={!!busy} onClick={() => setDialog(null)}>取消</button><button className="primary" disabled={!!busy || !name.trim() || (addMode === 'register' && !root.trim())} type="submit">{busy === 'add' ? '正在处理…' : addMode === 'create' ? '建立环境' : '添加环境'}</button></div></form></Modal>}
    {dialog === 'settings' && <Modal title="设置与模块" wide onClose={() => setDialog(null)}><div className="modal-body"><h3>外观</h3><div className="segmented theme-picker">{[['system','跟随系统'],['light','浅色'],['dark','深色']].map(([value,title]) => <button key={value} aria-pressed={theme === value} onClick={() => setTheme(value)}>{title}</button>)}</div><h3 className="separated">当前模块能力</h3><p className="muted">以下状态来自本地执行器。模块实现与桌面接入、安装验证分别记录。</p><div className="capability-list">{capabilities.map(capability => <div className="capability" key={capability.name}><div><strong>{label(capability.name)}</strong><p>{capability.reason}</p></div><Status value={capability.status}/></div>)}</div><BrowserPanel/><Notice>受控通道位于环境的“外发与权限”。SSH 任务尚未接入此桌面版本。后台漂移检查、菜单栏与自动更新也未启用；请使用主界面主动检查。</Notice><div className="fact-row"><span>数据与隐私</span><span>本地处理，无默认分析上传或远程字体。</span></div></div><div className="modal-footer"><button onClick={() => setDialog(null)}>完成</button></div></Modal>}
    {dialog === 'help' && <Modal title="使用 Lintel" wide onClose={() => setDialog(null)}><div className="modal-body help-content"><div className="help-brand"><Brand/><div><h3>清楚知道改了什么</h3><p>Lintel 管理具体的 Claude 使用环境。</p></div></div><ol><li><strong>选择环境。</strong>用环境名称、主机和完整配置路径确认目标。</li><li><strong>编辑草案。</strong>在“保护方案”选择功能取舍，切换目标不会串用草案。</li><li><strong>确认计划。</strong>对照“将修改”与“将保留”，一次授权执行。</li><li><strong>查看结果。</strong>按步骤核对完成、限制和新启动要求；需要时先预览恢复。</li></ol><Notice>新目录不等于旧登录已退出，配置值不等于强网络约束。尚未交付的完整产品能力见“设置与模块”。</Notice><h3>遇到错误</h3><p>计划生成后目标发生变化时，重新生成预览。执行结果不确定时查询原任务，不再次提交清理。支持资料可在“记录与恢复”本地预览，不会自动上传。</p></div><div className="modal-footer"><button onClick={() => setDialog('settings')}>查看模块能力</button><button className="primary" onClick={() => setDialog(null)}>知道了</button></div></Modal>}
    {dialog === 'support' && <Modal title="脱敏支持资料" wide onClose={() => setDialog(null)}><div className="modal-body"><Notice>请先检查内容，再自行分享。不会自动上传；不应包含令牌、Cookie 或会话正文。</Notice><textarea className="support-preview" aria-label="脱敏支持资料内容" value={support} readOnly spellCheck={false}/></div><div className="modal-footer"><button onClick={() => setDialog(null)}>关闭</button><button className="primary" onClick={() => void copy(support)}><Icon name="copy" size={15}/>复制资料</button></div></Modal>}
    {flow && <Modal title={flow.receipt ? '执行结果' : flow.uncertain ? '结果待核对' : '确认这份计划'} wide onClose={() => { if (busy !== 'execute') { setFlow(null); setError(''); } }}><div className="plan-target"><Icon name="terminal" size={21}/><div><strong>{flow.environment.name}</strong><span>{label(flow.environment.host)} · {label(flow.environment.surface)}</span></div><span className="small-label">仅此目标</span></div><div className="modal-body flow-body"><h3 className="flow-title">{flow.receipt?.title ?? flow.plan?.title}</h3>{error && <div role="alert"><Notice tone="error">{error}</Notice></div>}
      {flow.receipt ? <><div className="receipt-summary"><Status value={flow.receipt.status}/><span>{formatDate(flow.receipt.created_at)}</span></div><div className="steps">{flow.receipt.steps.map((step, index) => <div className="step" key={step.id}><span className="step-number">{index + 1}</span><div><strong>{step.label}</strong><p>{step.message}</p></div><Status value={step.status}/></div>)}</div><Warnings items={flow.receipt.warnings}/>{flow.receipt.new_root && <div className="fact-row"><span>新配置目录</span><Path value={flow.receipt.new_root}/></div>}{flow.receipt.archive_path && <div className="fact-row"><span>工作归档</span><Path value={flow.receipt.archive_path}/></div>}<div className="receipt-id"><span>任务 ID</span><code>{flow.receipt.id}</code><button className="icon-button" aria-label="复制任务 ID" onClick={() => void copy(flow.receipt!.id)}><Icon name="copy" size={14}/></button></div>{!flow.receipt.restorable && <p className="muted">此任务没有可用的配置恢复动作。保留项与未完成项以各步骤结果为准。</p>}</> : flow.plan && <><Warnings items={flow.plan.warnings}/><div className="plan-columns"><section><h4>将修改 <span>{flow.plan.changes.length} 项</span></h4>{flow.plan.changes.length ? <div className="change-list">{flow.plan.changes.map((change,index) => <div className="change" key={`${change.key}-${index}`}><strong>{change.label}</strong><div className="change-values"><code>{change.before ?? '未设置'}</code><Icon name="arrow" size={13}/><code>{change.after ?? '移除该项'}</code></div><details><summary>查看来源</summary><code className="full-path">{change.key}</code><Path value={change.path}/></details></div>)}</div> : <p className="muted">没有字段修改。下方步骤仍可能包含新环境建立或内容迁入。</p>}</section><section className="preserve-column"><h4>将保留</h4><ul className="keep-list">{flow.plan.preserves.map((item,index) => <li key={index}>{item}</li>)}</ul></section></div>{flow.plan.archive_passphrase_required && <section className="archive-password"><h4>保护工作归档</h4><p>本次将加密归档 {flow.plan.file_count ?? '所选'} 个文件。设置至少 12 个字符的口令并自行妥善保存；Lintel 不保存口令。之后读取归档需要此口令。</p><div className="password-fields"><label className="field">归档口令<input type="password" autoComplete="new-password" minLength={12} value={archivePassphrase} onChange={event => setArchivePassphrase(event.target.value)}/></label><label className="field">再次输入口令<input type="password" autoComplete="new-password" value={archiveConfirmation} onChange={event => setArchiveConfirmation(event.target.value)}/></label></div>{archiveConfirmation && archiveConfirmation !== archivePassphrase && <p className="password-error">两次输入的口令不一致。</p>}</section>}<section className="plan-steps"><h4>实际步骤</h4>{flow.plan.actions.map(action => <div className="action-description" key={action.id}><span>{action.label}</span><span className="small-label">{action.reversible ? '可恢复' : '无自动恢复'}</span></div>)}</section><details className="plan-identity"><summary>目标与计划标识</summary><Path value={flow.environment.root}/><code className="full-path">{flow.plan.id}</code></details>{flow.uncertain && <Notice tone="warning">没有再次执行。请查询这份计划的原任务；未收到结果不代表操作没有发生。</Notice>}</>}
      </div><div className="modal-footer flow-footer"><span>{flow.environment.name}</span><div className="button-row">{flow.receipt ? <><button onClick={() => { setFlow(null); setPage('history'); }}>查看记录</button>{flow.receipt.restorable && <button disabled={!!busy} onClick={() => restore(flow.receipt!)}>预览恢复</button>}<button className="primary" disabled={!!busy || !(flow.receipt.new_environment_id ? environments.find(env => env.id === flow.receipt!.new_environment_id)?.executable : flow.environment.executable)} onClick={() => { const target = environments.find(env => env.id === flow.receipt?.new_environment_id) ?? flow.environment; launch(target); }}>打开 Claude<Icon name="arrow" size={15}/></button></> : flow.uncertain ? <><button onClick={() => { setFlow(null); setPage('history'); setError(''); }}>返回记录</button><button className="primary" disabled={!!busy} onClick={reconcile}>查询原任务</button></> : <><button disabled={!!busy} onClick={() => { setFlow(null); setError(''); }}>返回调整</button><button className="primary" disabled={!!busy || !flow.plan?.actions.length || (!!flow.plan?.archive_passphrase_required && (archivePassphrase.length < 12 || archivePassphrase !== archiveConfirmation))} onClick={() => void executePlan()}>{busy === 'execute' ? '正在执行，请稍候…' : '批准并执行'}<Icon name="arrow" size={15}/></button></>}</div></div></Modal>}
  </div>;
}

function SettingsRows({ settings }: { settings: Inspection['settings'] }) { return <div className="settings-list">{settings.length === 0 ? <p className="padded muted">此环境没有可展示的已识别设置。</p> : settings.map(setting => <details className="setting-row" key={setting.key}><summary><span><strong>{setting.label}</strong><span className="setting-timing">{label(setting.effect_timing)}</span></span><code>{setting.value ?? '未设置'}</code><Status value={setting.status}/><Icon name="chevron" size={13}/></summary><div className="setting-detail"><code>{setting.key}</code><span>来源：{setting.source}</span><span>记录状态：{label(setting.status)} · 生效时机：{label(setting.effect_timing)}</span></div></details>)}</div>; }
function categoryLabel(value: string) { return ({ instructions: '个人指令', memory: '记忆文本', sessions: '会话资料', settings: '设置文件', projects: '项目资料', credentials: '凭据元数据' } as Record<string,string>)[value] ?? value; }
function SelectTarget({ onAdd }: { onAdd: () => void }) { return <div className="empty-state bordered"><Icon name="environments" size={32}/><h2>先选择一个具体环境</h2><p>每份方案绑定其主机和配置目录。</p><button onClick={onAdd}>添加现有环境</button></div>; }
