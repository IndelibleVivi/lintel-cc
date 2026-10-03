import { useState } from 'react';
import { Icon, Notice, formatBytes } from './ui';

type ExtensionPlan = {
  schema: number; browser: string; package: string; version: string;
  sha256: string; bytes: number; files: { path: string; sha256: string; bytes: number }[];
  extension_path: string; manifest_path: string; effect: string;
  existing_installation: unknown | null;
  recovery?: { phase: string; action: string; message: string } | null;
  status: 'ready' | 'already-installed' | 'conflict';
  install_action: 'install' | 'upgrade' | 'none' | 'blocked';
};
type Props = {
  browser: string; native: boolean; busy: boolean;
  request: <T>(payload: Record<string, unknown>) => Promise<T>;
  run: (work: () => Promise<void>) => Promise<void>;
};

export default function BrowserExtensionSetup({ browser, native, busy, request, run }: Props) {
  const [plan, setPlan] = useState<ExtensionPlan | null>(null);
  const [prepared, setPrepared] = useState<ExtensionPlan | null>(null);
  const [copied, setCopied] = useState(false);
  const firefox = browser === 'firefox';
  const manager = firefox ? 'about:debugging#/runtime/this-firefox' : browser === 'edge' ? 'edge://extensions' : 'chrome://extensions';
  return <section className="browser-extension-setup" aria-label="准备伴随扩展">
    <h4 className="browser-setup-step"><span>1</span>准备并加载扩展</h4>
    <p className="small-print">App 自带伴随扩展，无需下载源码或运行构建命令。先预览目录，再批准把文件放到当前用户的 Lintel 数据区。</p>
    <button disabled={busy || !native} onClick={() => void run(async () => {
      setPlan(null); setPrepared(null); setCopied(false);
      setPlan(await request<ExtensionPlan>({ op: 'bundled_extension_plan', browser }));
    })}>预览扩展目录<Icon name="arrow" size={14}/></button>
    {plan && <section className="installation-plan browser-host-plan" aria-label="扩展目录预览">
      <div className="browser-host-plan-heading"><h4>{plan.install_action === 'upgrade' ? '核对后更新扩展文件' : plan.install_action === 'none' ? '目录已经准备好' : '批准后文件放在这里'}</h4><span>v{plan.version} · {formatBytes(plan.bytes)}</span></div>
      <dl className="browser-host-scope"><div><dt>扩展目录</dt><dd><code className="full-path">{plan.extension_path}</code></dd></div><div><dt>文件范围</dt><dd>{plan.files.length} 个文件 · {firefox ? 'Firefox' : 'Chrome / Edge'}</dd></div></dl>
      {plan.recovery && <Notice tone="warning">上次扩展目录准备中断。{plan.install_action === 'blocked' ? 'App 资源或目录与上次批准不一致，当前无法继续。请先核对原版本资源与目录变化；现有文件会保留。' : '请重新核对这份预览，批准后继续准备。路径保持不变；完成后再在浏览器加载或重新加载。'}</Notice>}
      {!plan.recovery && plan.install_action === 'upgrade' && <Notice>将更新这份由 Lintel 准备的目录，路径保持不变。随后请在目标浏览器的扩展管理页重新加载 Lintel。</Notice>}
      {!plan.recovery && plan.install_action === 'blocked' && <Notice tone="warning">这个位置有不属于本安装器的文件，或已有扩展被改动。无法覆盖；现有内容会保留。</Notice>}
      <details className="browser-host-details"><summary>查看文件清单与版本摘要</summary><p>整份扩展 SHA-256 <code className="full-path">{plan.sha256}</code></p><ul className="browser-extension-files">{plan.files.map(file => <li key={file.path}><code>{file.path}</code><span>{formatBytes(file.bytes)}</span></li>)}</ul>{plan.existing_installation !== null && <><h5>当前目录记录</h5><pre>{JSON.stringify(plan.existing_installation, null, 2)}</pre></>}{plan.recovery && <><h5>上次中断与恢复范围</h5><pre>{JSON.stringify(plan.recovery, null, 2)}</pre></>}</details>
      <button className="primary" disabled={busy || !native || plan.install_action === 'blocked'} onClick={() => void run(async () => {
        try {
          const result = await request<{ status: string; plan: ExtensionPlan }>({ op: 'install_bundled_extension', approved_plan: plan });
          setPrepared(result.plan); setPlan(null); setCopied(false);
        } catch (error) { setPlan(null); setPrepared(null); throw error; }
      })}>{plan.recovery ? '批准继续准备扩展' : plan.install_action === 'upgrade' ? '批准更新扩展文件' : plan.install_action === 'none' ? '核对已准备目录' : '批准准备扩展目录'}</button>
      <p className="small-print">只准备文件。浏览器的加载、权限与 profile 配对仍需你分别确认。</p>
    </section>}
    {prepared && <div className="browser-extension-ready" role="status">
      <strong>扩展目录已准备好</strong><code className="full-path">{prepared.extension_path}</code>
      <div className="button-row"><button disabled={busy || !native} onClick={() => void run(async () => { await request({ op: 'reveal_bundled_extension', browser }); })}><Icon name="arrow" size={14}/>在 Finder 中打开</button><button disabled={busy} onClick={() => void run(async () => { await navigator.clipboard.writeText(firefox ? prepared.manifest_path : prepared.extension_path); setCopied(true); })}><Icon name="copy" size={14}/>{copied ? '已复制' : firefox ? '复制 manifest 路径' : '复制目录路径'}</button></div>
      <ol className="browser-load-instructions"><li>在目标 profile 的地址栏打开 <code>{manager}</code>。</li><li>{firefox ? <>选择「Load Temporary Add-on」，选取目录里的 <code>manifest.json</code>。</> : <>启用「开发者模式」，点击「加载已解压的扩展 / Load unpacked」，选择上面的扩展目录。更新后点击 Lintel 的重新加载按钮。</>}</li><li>{firefox ? <>下面使用固定 ID <code>lintel@lintel.local</code> 安装本地连接。</> : <>找到 Lintel，将管理页显示的准确扩展 ID 填到下一步。</>}</li></ol>
    </div>}
    <p className="small-print browser-extension-limit">{firefox ? 'Firefox 当前是临时开发加载，退出浏览器后会移除；长期安装的签名分发尚未完成。' : '当前为开发候选，需在目标浏览器启用开发者模式；商店发布尚未完成。Chrome / Edge 使用同一份扩展目录，每个 profile 仍需单独加载与配对。'}</p>
  </section>;
}
