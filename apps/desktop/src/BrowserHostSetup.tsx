import { useState } from 'react';
import { Icon, Notice, formatBytes } from './ui';
import BrowserExtensionSetup from './BrowserExtensionSetup';

type HostPlan = {
  browser: string; extension_id: string; version: string; sha256: string; bytes: number;
  host_path: string; manifest_path: string; manifest: unknown; state_path: string;
  effect: string; existing_manifest: unknown | null;
  status: 'ready' | 'already-registered' | 'conflict';
  install_action: 'install' | 'upgrade' | 'none' | 'blocked';
};
type Props = {
  native: boolean; busy: boolean;
  request: <T>(payload: Record<string, unknown>) => Promise<T>;
  run: (work: () => Promise<void>) => Promise<void>;
  onInstalled: (message: string) => void;
};

export default function BrowserHostSetup({ native, busy, request, run, onInstalled }: Props) {
  const [browser, setBrowser] = useState('chrome');
  const [extensionId, setExtensionId] = useState('');
  const [plan, setPlan] = useState<HostPlan | null>(null);
  const actionName = plan?.install_action === 'upgrade' ? '批准更新本地连接' : plan?.install_action === 'none' ? '核对已安装组件' : '批准安装本地连接';
  return <div className="browser-host-setup">
    <div className="browser-target-field">
      <label className="field">目标浏览器<select disabled={busy} value={browser} onChange={event => {
        setBrowser(event.target.value); setExtensionId(event.target.value === 'firefox' ? 'lintel@lintel.local' : ''); setPlan(null);
      }}>{[['chrome', 'Chrome'], ['edge', 'Edge'], ['firefox', 'Firefox']].map(([value, title]) => <option key={value} value={value}>{title}</option>)}</select></label>
    </div>
    <BrowserExtensionSetup key={browser} browser={browser} native={native} busy={busy} request={request} run={run}/>
    <h4 className="browser-setup-step"><span>2</span>安装本地连接</h4>
    <label className="field browser-extension-id">已安装扩展的 ID<input disabled={busy} value={extensionId} onChange={event => { setExtensionId(event.target.value); setPlan(null); }} spellCheck={false} autoComplete="off" placeholder="扩展管理页中的 ID"/><small>{browser === 'firefox' ? 'Lintel 的 Firefox 扩展 ID 固定；临时扩展会在退出浏览器后移除。' : '从目标 profile 的扩展管理页复制。每份 profile 仍需单独配对。'}</small></label>
    <p className="small-print">App 自带本地连接组件，无需编译或填写程序路径。预览只检查；批准后安装到当前用户的 Lintel 目录，并注册给这个扩展 ID。</p>
    <button disabled={busy || !native || !extensionId.trim()} onClick={() => void run(async () => {
      setPlan(null); setPlan(await request<HostPlan>({ op: 'bundled_host_plan', browser, extension_id: extensionId.trim() }));
    })}>预览本地连接安装<Icon name="arrow" size={14}/></button>
    {plan && <section className="installation-plan browser-host-plan" aria-label="本地连接安装预览">
      <div className="browser-host-plan-heading"><h4>{plan.install_action === 'upgrade' ? '更新前先核对' : plan.status === 'already-registered' ? '已注册，继续核对' : '批准后连接到这里'}</h4><span>v{plan.version} · {formatBytes(plan.bytes)}</span></div>
      <dl className="browser-host-scope">
        <div><dt>授权扩展</dt><dd><code>{plan.extension_id}</code><span>{({ chrome: 'Chrome', edge: 'Edge', firefox: 'Firefox' } as Record<string, string>)[plan.browser]}</span></dd></div>
        <div><dt>组件位置</dt><dd><code className="full-path">{plan.host_path}</code></dd></div>
        <div><dt>注册文件</dt><dd><code className="full-path">{plan.manifest_path}</code></dd></div>
      </dl>
      {plan.install_action === 'upgrade' && <Notice>已有 Lintel 注册将更新到这份组件。下面可对照当前与拟写入内容；旧组件保留，profile 配对仍由原记录核对。</Notice>}
      {plan.install_action === 'blocked' && <Notice tone="warning">已有注册或组件与这份安装不一致，无法在这里覆盖。请先核对下面的当前注册；它会保留原样。</Notice>}
      <details className="browser-host-details"><summary>查看注册内容与组件摘要</summary>
        {plan.existing_manifest !== null && <><h5>当前注册</h5><pre>{JSON.stringify(plan.existing_manifest, null, 2)}</pre></>}
        <h5>拟写入的注册</h5><pre>{JSON.stringify(plan.manifest, null, 2)}</pre>
        <p>组件 SHA-256 <code className="full-path">{plan.sha256}</code></p>
        <p>配对记录目录 <code className="full-path">{plan.state_path}</code></p>
      </details>
      <button className="primary" disabled={busy || !native || plan.install_action === 'blocked'} onClick={() => void run(async () => {
        let result: { status: string; plan: HostPlan };
        try { result = await request({ op: 'install_bundled_host', approved_plan: plan }); }
        catch (error) { setPlan(null); throw error; }
        setPlan(null);
        onInstalled(result.status === 'already-registered' ? '本地连接组件已核对。接下来生成配对请求，核对目标 profile 的短码。' : result.status === 'updated' ? '本地连接已更新。接下来检查配对；安装完成不表示浏览器在线。' : '本地连接已安装并注册。接下来生成配对请求，核对目标 profile 的短码。');
      })}>{actionName}</button>
      <p className="small-print">批准仅注册当前用户的本地连接。profile 短码还需另行批准，安装完成不表示已配对或在线。</p>
    </section>}
  </div>;
}
