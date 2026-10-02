import { useEffect, useState } from 'react';
import { request, transport } from './api';
import Clawd from './Clawd';
import type { CleanupInspection, Environment, Plan } from './types';
import { Icon, Notice, Status } from './ui';

type Recipe = 'rebuild' | 'repair_login' | 'reset_client' | 'retire';
const recipes: { id: Recipe; title: string; description: string }[] = [
  { id: 'rebuild', title: '新环境重建', description: '保留旧环境，先整理一份新开始。' },
  { id: 'repair_login', title: '修复本地登录', description: '处理已识别的凭据文件，保留工作内容。' },
  { id: 'reset_client', title: '清理并重建', description: '归档工作、处理旧状态，再准备新环境。' },
  { id: 'retire', title: '退役此环境', description: '处理旧状态并停用启动入口，保留原工作目录。' },
];
const classes = [['instructions', '个人指令', '纯文本指令；不运行引用的命令'], ['memory', '记忆文本', '保留原始文本，迁入后置于待用区'], ['sessions', '会话资料', '保留原始资料，不宣称恢复可续聊会话']];
export default function CleanupPanel({ send = request, environment, onPlan, onBrowser }: { send?: typeof request; environment: Environment; onPlan: (plan: Plan) => void; onBrowser: () => void }) {
  const [recipe, setRecipe] = useState<Recipe>('rebuild');
  const [categories, setCategories] = useState(['instructions', 'memory', 'sessions']);
  const [inspection, setInspection] = useState<CleanupInspection | null>(null);
  const [stopped, setStopped] = useState(false);
  const [logout, setLogout] = useState(false);
  const [auth, setAuth] = useState<{ auth_method: string; logged_in: boolean; remote_revocation: string } | null>(null);
  const [busy, setBusy] = useState('');
  const [error, setError] = useState('');
  async function run(name: string, action: () => Promise<void>) { setBusy(name); setError(''); try { await action(); } catch (err) { setError(err instanceof Error ? err.message : String(err)); } finally { setBusy(''); } }
  async function inspect() { setInspection(await send('cleanup_inspect', { environment_id: environment.id })); }
  useEffect(() => { void run('inspect', inspect); }, [environment.id]);
  async function preview() { await run('plan', async () => { const plan = recipe === 'rebuild' ? await send('plan_reset', { environment_id: environment.id, recipe: 'rebuild', categories }) : await send('plan_cleanup', { environment_id: environment.id, recipe, writers_confirmed_stopped: stopped, official_logout: logout, categories }); onPlan(plan); }); }
  return <>
    <div className="page-heading"><div><span className="eyebrow">02 / a fresh start</span><h1>保住内容，重新开始</h1><p>确认处理范围，把值得留下的内容妥善收好。</p></div><Clawd small mood="pack"/></div>
    {error && <div role="alert"><Notice tone="error">{error}</Notice></div>}
    <fieldset className="recipe-grid"><legend className="sr-only">清理配方</legend>{recipes.map(item => <label className={`recipe ${recipe === item.id ? 'chosen' : ''}`} key={item.id}><input type="radio" name="cleanup-recipe" checked={recipe === item.id} onChange={() => { setRecipe(item.id); setError(''); }}/><span><strong>{item.title}</strong><small>{item.description}</small></span></label>)}</fieldset>
    <div className="rebuild-split"><section className="surface"><div className="surface-heading"><h2>将处理</h2><Icon name="rebuild" size={17}/></div><div className="padded"><ol className="process-list">{(recipe === 'rebuild' ? ['加密归档所选工作内容', '建立独立的新配置目录', '迁入所选内容，旧环境保留'] : recipe === 'repair_login' ? ['复查写入者已停止', ...(logout ? ['调用官方注销并读回'] : []), '移除预览列出的本地凭据文件'] : ['先加密归档工作与混合客户端状态', ...(logout ? ['调用官方注销并读回'] : []), '移除预览列出的凭据与客户端状态', recipe === 'reset_client' ? '建立新环境并迁入工作内容' : '退役本环境启动入口']).map((item,index) => <li key={item}><span>{String(index+1).padStart(2,'0')}</span><div><strong>{item}</strong></div></li>)}</ol></div></section><section className="surface"><div className="surface-heading"><h2>将保留</h2><Icon name="check" size={17}/></div><div className="padded"><ul className="keep-list"><li>原始工作内容与项目文件</li><li>settings、hooks、MCP 与插件文件</li><li>其他环境、浏览器与目录外认证</li><li>通用代理与系统网络设置</li></ul><p className="small-print">保留文件不等于在新环境自动启用。浏览器数据需要单独确认。</p></div></section></div>
    {recipe === 'rebuild' ? <Notice>此配方保留旧目录与登录。新环境建立不会证明旧状态已清场，结果按实际范围显示。</Notice> : <>
      <section className="surface"><div className="surface-heading"><h2>已识别的本地范围</h2><button className="text-button" disabled={!!busy} onClick={() => void run('inspect', inspect)}><Icon name="refresh" size={14}/>刷新范围</button></div><div className="padded">{!inspection ? <p className="muted">正在检查文件与写入者…</p> : <><div className="cleanup-files">{inspection.files.filter(file => recipe !== 'repair_login' || file.category === 'credentials').map(file => <div key={file.path}><span>{file.category === 'credentials' ? '本地凭据' : '客户端状态'}</span><code>{file.path}</code><Status value={file.present ? 'discovered' : 'not_present'}/></div>)}</div><p className="small-print">{inspection.coverage}</p>{inspection.shared_profile_present && <Notice tone="warning">发现目录外共享 Anthropic profile。此配方不能核验共享注销范围，官方注销不可用。</Notice>}{inspection.writers.length > 0 && <Notice tone="warning">仍有无法归属到此目录的 Claude 写入进程：{inspection.writers.map(writer => `${writer.name} (${writer.pid})`).join('、')}。请核对后自行关闭；Lintel 不会全局结束进程。</Notice>}</>}</div></section>
      <section className="surface"><div className="surface-heading"><h2>执行前，由你确认</h2><span className="small-label">仅 {environment.name}</span></div><label className="checkbox-row"><span><strong>已关闭目标的终端、IDE 与自动重启来源</strong><span>core 会在预览与执行时再次检查写入进程。</span></span><input type="checkbox" checked={stopped} onChange={event => setStopped(event.target.checked)}/></label><label className="checkbox-row"><span><strong>同时请求官方注销</strong><span>调用本环境的 Claude auth logout，可能联系服务端。服务端 token 撤销仍单独标为未验证。</span></span><input type="checkbox" checked={logout} disabled={transport !== 'native' || !inspection?.official_logout_available || inspection.shared_profile_present} onChange={event => setLogout(event.target.checked)}/></label><div className="padded"><button disabled={!!busy || transport !== 'native' || !inspection?.official_logout_available} onClick={() => void run('auth', async () => setAuth(await send('auth_probe', { environment_id: environment.id })))}>显式检查认证来源</button>{auth && <p className="small-print">{auth.logged_in ? 'CLI 报告已登录' : 'CLI 报告未登录'} · 来源 {auth.auth_method} · 服务端撤销 {auth.remote_revocation}</p>}{transport !== 'native' && <p className="small-print">合成测试不调用真实 Claude 认证命令；只验证临时文件处理。</p>}</div></section>
      {!logout && <Notice tone="warning">本次仅处理预览内的本地文件；未执行官方注销，Keychain 与目录外认证未知，结果会保留“部分完成”。</Notice>}
    </>}
    {recipe !== 'repair_login' && <section className="surface work-selection"><div className="surface-heading"><h2>{recipe === 'retire' ? '归档哪些工作内容' : '迁入哪些工作内容'}</h2><span className="small-label">执行前设置加密口令</span></div>{classes.map(([id,title,detail]) => <label className="checkbox-row" key={id}><span><strong>{title}</strong><span>{detail}</span></span><input type="checkbox" checked={categories.includes(id)} onChange={event => setCategories(current => event.target.checked ? [...current,id] : current.filter(value => value !== id))}/></label>)}</section>}
    <div className="inline-route"><span>还需要处理浏览器登录与站点数据？</span><button className="text-button" onClick={onBrowser}>浏览器工作空间<Icon name="arrow" size={14}/></button></div>
    <div className="action-bar"><div><strong>{environment.name}</strong><span>{recipe === 'rebuild' ? '先预览范围，再批准建立新环境' : '精确文件范围与不可恢复动作会在计划中展开'}</span></div><button className="primary" disabled={!!busy || (recipe !== 'rebuild' && (!stopped || !inspection || inspection.writers.length > 0)) || environment.status === 'retired'} onClick={() => void preview()}>{busy === 'plan' ? '正在生成计划…' : '预览这份计划'}<Icon name="arrow" size={16}/></button></div>
  </>;
}
