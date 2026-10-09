import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Modal, Notice } from './ui';
import { ResourceLink } from './Resources';
import './app-updates.css';

type UpdateStatus = {
  current_version: string; channel: 'preview' | 'stable' | null; configured: boolean;
  phase: string; background_check: boolean; checked_at: number | null; failure: string | null;
  candidate: { id: string; version: string; notes: string | null; url: string; signature_verified: boolean } | null;
  downloaded_bytes: number; total_bytes: number | null;
  last_install: { id: string; from_version: string; to_version: string; status: string } | null;
  installation_unresolved: boolean;
};
type Envelope = { ok: true; data: UpdateStatus } | { ok: false; error: { code: string; message: string } };
async function send(payload: Record<string, unknown>) {
  const result = await invoke<Envelope>('app_update_request', { payload });
  if (!result.ok) throw new Error(`${result.error.message}（${result.error.code}）`);
  return result.data;
}
const phaseText: Record<string, string> = {
  idle: '尚未检查', unconfigured: '此构建尚未配置更新发行', checking: '正在检查', latest: '本次检查没有发现更高版本',
  available: '发现新版本', downloading: '正在下载并核验', verified: '签名与版本核验通过',
  installing: '正在安装', installed: '安装完成，重启后运行新版本', uncertain: '安装结果待核对', error: '本次操作未完成',
};

// Always mounted by App: opt-in checks survive closing Settings. Neither check
// nor download installs anything; installation and restart have native gates.
export default function AppUpdates({ visible, disabled, onClose }: { visible: boolean; disabled: boolean; onClose: () => void }) {
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [pending, setPending] = useState('');
  const [failure, setFailure] = useState('');
  const [approve, setApprove] = useState(false);
  const active = useRef(true);
  const statusSequence = useRef(0);
  const inFlight = useRef(false);
  const nextBackgroundCheck = useRef(0);
  const backgroundConsent = useRef(false);
  const read = useCallback(async () => {
    const sequence = ++statusSequence.current;
    try { const value = await send({ op: 'status' }); if (active.current && sequence === statusSequence.current) { backgroundConsent.current = value.background_check; setStatus(value); } }
    catch (error) { if (active.current && visible) setFailure(error instanceof Error ? error.message : String(error)); }
  }, [visible]);
  useEffect(() => { active.current = true; return () => { active.current = false; }; }, []);
  useEffect(() => { void read(); }, [read]);
  const run = useCallback(async (op: string, fields: Record<string, unknown> = {}) => {
    if (inFlight.current) return;
    inFlight.current = true; ++statusSequence.current; setPending(op); setFailure('');
    if (op === 'preference' && fields.background_check === false) backgroundConsent.current = false;
    try {
      const value = await send({ op, ...fields });
      backgroundConsent.current = value.background_check;
      ++statusSequence.current;
      if (active.current) { setStatus(value); setApprove(false); }
    } catch (error) {
      if (active.current) setFailure(error instanceof Error ? error.message : String(error));
      await read();
    } finally { inFlight.current = false; if (active.current) setPending(''); }
  }, [read]);
  useEffect(() => {
    if (!pending || !['check', 'download', 'install'].includes(pending)) return;
    const timer = setInterval(() => void read(), 1000);
    return () => clearInterval(timer);
  }, [pending, read]);
  useEffect(() => {
    if (!status?.configured || !status.background_check || status.phase === 'installed') return;
    const check = () => {
      if (!backgroundConsent.current || document.visibilityState !== 'visible' || inFlight.current || Date.now() < nextBackgroundCheck.current) return;
      nextBackgroundCheck.current = Date.now() + 60 * 60 * 1000;
      void run('check');
    };
    check();
    const timer = setInterval(check, 60 * 1000);
    window.addEventListener('focus', check); document.addEventListener('visibilitychange', check);
    return () => { clearInterval(timer); window.removeEventListener('focus', check); document.removeEventListener('visibilitychange', check); };
  }, [status?.configured, status?.background_check, status?.phase === 'installed', run]);
  useEffect(() => { setApprove(false); }, [status?.candidate?.id]);
  if (!visible) return null;
  const candidate = status?.candidate;
  const busy = !!pending;
  const progress = status?.total_bytes ? Math.min(100, Math.round(status.downloaded_bytes / status.total_bytes * 100)) : null;
  return <Modal title="Lintel 更新" wide onClose={busy ? () => undefined : onClose}>
    <div className="modal-body app-updates" aria-busy={busy}>
      <div className="update-version"><div><span className="eyebrow">app / version</span><h3>{status?.current_version ?? '正在读取版本'}</h3></div><span className="small-label">{status?.channel === 'stable' ? 'Stable' : status?.channel === 'preview' ? 'Preview' : '未配置发行通道'}</span></div>
      <p role="status">{status ? phaseText[status.phase] ?? status.phase : '正在读取本 App 更新状态…'}</p>
      {failure && <Notice tone="error">{failure}</Notice>}
      {status?.failure && status.failure !== failure && <Notice tone="error">{status.failure}</Notice>}
      {!status?.configured && <Notice>当前源码构建没有正式发行公钥与 feed。未来发行构建会固定这两项；现有未带 updater 的旧 App 需要手动升级一次。</Notice>}
      <div className="button-row"><button disabled={busy} onClick={() => void run('check')}>{pending === 'check' ? '正在检查…' : '检查更新'}</button>{status?.checked_at && <span className="small-print">上次成功检查 {new Date(status.checked_at * 1000).toLocaleString()}</span>}</div>
      <label className="update-preference"><input type="checkbox" checked={status?.background_check ?? false} disabled={!status || busy || !status.configured} onChange={event => void run('preference', { background_check: event.target.checked })}/><span>在 App 打开且可见时后台检查<small>默认关闭；启用后至多每小时检查一次，不自动下载、安装或重启。</small></span></label>
      <p className="small-print">检查访问固定的 Lintel HTTPS feed；下载访问 GitHub Releases。请求会让托管方看到出口 IP，不上传环境、账号、会话或设备标识。它与 Claude 的遥测设置分别管理。</p>
      <ResourceLink resource="app-updates">安装、更新与发行说明</ResourceLink>
      {candidate && <section className="update-candidate" aria-label="更新候选">
        <div className="surface-heading"><h3>Lintel {candidate.version}</h3><span className="small-label">{candidate.signature_verified ? '已核验' : '待下载核验'}</span></div>
        {candidate.notes && <p className="update-notes">{candidate.notes}</p>}
        <p className="small-print">来源 <code>{candidate.url}</code></p>
        {status?.phase === 'downloading' && <><progress aria-label="更新下载进度" max={100} value={progress ?? undefined}/><p className="small-print">已下载 {(status.downloaded_bytes / 1024 / 1024).toFixed(1)} MiB{progress !== null ? ` · ${progress}%` : ''} · 下载结束后仍需签名与版本核验。</p></>}
        {!candidate.signature_verified && <button disabled={busy || status?.installation_unresolved} onClick={() => void run('download', { candidate_id: candidate.id })}>{pending === 'download' ? '正在下载…' : '下载并核验'}</button>}
        {candidate.signature_verified && status?.phase !== 'installed' && <>
          <Notice>安装替换本 App；浏览器扩展、native host 和远端 runner 保持各自的审阅安装流程。native 层会阻止正在执行操作或仍有活跃通道时安装。重启前请保存正在编辑的交接稿。</Notice>
          <label className="update-preference"><input type="checkbox" checked={approve} disabled={busy || disabled || status?.installation_unresolved} onChange={event => setApprove(event.target.checked)}/><span>我已审阅此版本，准备安装</span></label>
          <button className="primary" disabled={!approve || busy || disabled || status?.installation_unresolved} onClick={() => void run('install', { candidate_id: candidate.id })}>{pending === 'install' ? '正在安装…' : '安装这份已核验更新'}</button>
        </>}
      </section>}
      {disabled && <Notice>当前 App 工作尚未结束，请完成后再安装或重启。</Notice>}
      {status?.last_install && <div className="update-record"><strong>原更新记录 · {status.last_install.to_version}</strong><code>{status.last_install.id}</code><span>{status.last_install.status === 'installed' ? '安装步骤已确认；当前进程仍显示启动时版本' : '原安装结果保留，状态：' + status.last_install.status}</span></div>}
      {status?.installation_unresolved && <Notice tone="error">保留原更新记录，避免重放安装。请核对 App 位置与版本；需要时用官方安装包手动修复，然后重新打开。</Notice>}
    </div>
    <div className="modal-footer"><button disabled={busy} onClick={onClose}>完成</button>{status?.phase === 'installed' && status.last_install && <button className="primary" disabled={busy || disabled} onClick={() => void run('restart', { candidate_id: status.last_install!.id })}>保存好工作后重启 Lintel</button>}</div>
  </Modal>;
}
