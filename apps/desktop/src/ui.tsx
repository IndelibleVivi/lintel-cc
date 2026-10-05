import { createContext, useContext, useEffect, useRef, useId, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import type { Receipt } from './types';

export const WorkspaceActionsContext = createContext<HTMLElement | null>(null);
export function WorkspaceActions({ children }: { children: ReactNode }) {
  const target = useContext(WorkspaceActionsContext);
  const bar = <div className="action-bar">{children}</div>;
  return target ? createPortal(bar, target) : bar;
}

export function hasConfirmedWorkArchive(receipt: Receipt): boolean {
  return !!receipt.archive_path && (!!receipt.archive_digest || receipt.steps.some(step => step.id === 'archive' && step.status === 'completed'));
}

export type IconName = 'sidebar' | 'environments' | 'policy' | 'rebuild' | 'history' | 'settings' | 'help' | 'plus' | 'arrow' | 'check' | 'refresh' | 'copy' | 'close' | 'terminal' | 'chevron' | 'warning' | 'sun' | 'moon' | 'download';
const paths: Record<IconName, ReactNode> = {
  sidebar: <><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16"/></>,
  environments: <><rect x="3" y="4" width="18" height="13" rx="2"/><path d="M8 21h8m-4-4v4"/></>,
  policy: <><path d="M4 6h16M4 12h16M4 18h16"/><circle cx="8" cy="6" r="2"/><circle cx="16" cy="12" r="2"/><circle cx="10" cy="18" r="2"/></>,
  rebuild: <><path d="M5 9a8 8 0 1 1-1 7M5 3v6h6"/><path d="M10 16v-5h5v5"/></>,
  history: <><path d="M3 11a9 9 0 1 1 3 8M3 5v6h6"/><path d="M12 7v5l3 2"/></>,
  settings: <><path d="m10 3-1 3-3 1-2-1-2 4 2 2v3l-1 2 3 3 3-1 3 1 1 2h4l1-3 2-2 2-1-1-4-2-1-1-3-3-2-2 1Z"/><circle cx="12" cy="12" r="3"/></>,
  help: <><circle cx="12" cy="12" r="9"/><path d="M9.5 9a2.5 2.5 0 0 1 5 0c0 2-2.5 2-2.5 4m0 3h.01"/></>,
  plus: <path d="M12 5v14M5 12h14"/>, arrow: <path d="M4 12h16m-6-6 6 6-6 6"/>,
  check: <path d="m5 12 4 4L19 6"/>, refresh: <><path d="M20 7v5h-5M4 17v-5h5"/><path d="M6 6a8 8 0 0 1 14 6M4 12a8 8 0 0 0 14 6"/></>,
  copy: <><rect x="8" y="8" width="12" height="13" rx="2"/><path d="M16 8V3H3v13h5"/></>, close: <path d="m6 6 12 12M6 18 18 6"/>,
  terminal: <><rect x="2" y="4" width="20" height="16" rx="3"/><path d="m6 9 3 3-3 3m6 0h5"/></>, chevron: <path d="m9 5 7 7-7 7"/>,
  warning: <><path d="m12 3 10 18H2Z"/><path d="M12 9v5m0 3h.01"/></>,
  sun: <><circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1 1m12 12 1 1M5 19l1-1M18 6l1-1"/></>,
  moon: <path d="M20 14a8 8 0 0 1-10-10A9 9 0 1 0 20 14Z"/>,
  download: <><path d="M12 3v12m-5-5 5 5 5-5M4 16v5h16v-5"/></>,
};
export function Icon({ name, size = 18 }: { name: IconName; size?: number }) { return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.65" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">{paths[name]}</svg>; }
export function Brand() { return <svg className="brand-mark" viewBox="0 0 32 32" fill="none" aria-hidden="true"><rect className="brand-beam" x="4" y="8.5" width="23" height="4"/><rect className="brand-support" x="6.5" y="12.5" width="4" height="10.5"/><rect className="brand-support" x="20.5" y="12.5" width="3.5" height="10.5"/><rect className="brand-mark-cursor" x="24.75" y="21" width="4" height="2"/></svg>; }
const names: Record<string, string> = { launch_attempt:'打开意图已记录，待核对', resume_attempt:'续聊意图已记录，待核对', launch_intent:'启动意图已记录，待核对', resume_intent:'续聊意图已记录，待核对', resume_prepared:'运行副本已准备，待核对', launch_failed:'启动未完成', uncertain:'结果待核对', verifying: '正在读回', interrupted: '已中断', expired: '预览已过期', cancelled: '已取消', superseded: '已被后续版本替代', requires_explicit_target: '需指定目标服务', linux_runner_only: '仅 Linux 运行器', needs_reconciliation: '待核对', launch_requested: '已请求启动', terminal_required: '需要交互终端', submission_unknown: '提交结果待核对', query_only: '等待查询原任务', response_received: '已收到错误响应', separate_module: '独立模块', not_delivered: '尚未交付', awaiting_browser_restart: '等待浏览器重启', not_completed: '尚未完成', not_present: '未发现', retired: '已退役', reactivated: '已重新启用', completed: '已完成', partially_completed: '部分完成', failed: '未完成', configured: '已配置', discovered: '已发现', registered: '已登记', ready: '可用', available: '可用', limited: '范围受限', unsupported: '尚不支持', unverified: '未验证', pending: '待处理', executing: '执行中', accepted: '已接收', verified: '已读回', unchanged: '无变化', clean: '无变化', drifted: '检测到变化', changed: '有变化', unknown: '未确认', preserved: '已保留', skipped: '已跳过', needs_restart: '等待新启动', launched: '已启动', not_configured: '未配置', owned: 'Lintel 管理', external: '外部环境', user: '用户管理', local: '本机', 'claude-code': 'Claude Code', 'next_launch': '下一次新启动', 'next-launch': '下一次新启动', immediate: '立即', not_enforced: '未强制约束', planned: '待确认', not_run: '未运行' };
export const label = (value: string) => names[value] ?? value;
export function Status({ value }: { value: string }) { const good = ['completed','configured','ready','available','verified','unchanged','clean','preserved'].includes(value); const tone = good ? 'good' : ['failed', 'uncertain', 'launch_failed'].includes(value) ? 'error' : ['needs_reconciliation', 'submission_unknown', 'interrupted', 'partially_completed','launch_intent','resume_intent','resume_prepared','launch_attempt','resume_attempt'].includes(value) ? 'attention' : ''; return <span className={`status ${tone}`}><span className="status-dot"/>{label(value)}</span>; }
export function Notice({ children, tone = 'neutral' }: { children: ReactNode; tone?: 'neutral'|'warning'|'error' }) { return <div className={`notice ${tone}`}><Icon name={tone === 'neutral' ? 'help' : 'warning'} size={17}/><div>{children}</div></div>; }
export function Modal({ title, children, onClose, wide = false }: { title: string; children: ReactNode; onClose: () => void; wide?: boolean }) {
  const ref = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  useEffect(() => { const previous = document.activeElement as HTMLElement | null; ref.current?.showModal(); return () => { ref.current?.close(); if (previous?.isConnected) previous.focus(); else requestAnimationFrame(()=>{if(document.querySelector('dialog[open]'))return;const target=document.querySelector<HTMLElement>('main h1,main h2');if(target){target.tabIndex=-1;target.focus({preventScroll:true})}}); }; }, []);
  useEffect(() => { ref.current?.querySelector('.modal-body')?.scrollTo({ top: 0 }); }, [title]);
  return <dialog ref={ref} className={wide ? 'modal wide' : 'modal'} onCancel={event => { event.preventDefault(); onClose(); }} aria-labelledby={titleId}><div className="modal-heading"><h2 id={titleId}>{title}</h2><button autoFocus className="icon-button" aria-label="关闭面板" onClick={onClose}><Icon name="close"/></button></div>{children}</dialog>;
}
export function formatDate(value: string) { const numeric = Number(value); const date = new Date(Number.isFinite(numeric) ? numeric * (numeric < 1e12 ? 1000 : 1) : value); return Number.isNaN(date.getTime()) ? value : new Intl.DateTimeFormat('zh-CN', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }).format(date); }
export function formatBytes(value: number) { return value < 1024 ? `${value} B` : value < 1024 ** 2 ? `${(value / 1024).toFixed(1)} KB` : `${(value / 1024 ** 2).toFixed(1)} MB`; }
