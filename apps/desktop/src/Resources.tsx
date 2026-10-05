import { useState, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { transport } from './api';

import resourceTable from '../../../contracts/documentation-resources.json';
import { buildIdentity } from './buildIdentity';
import taskCatalog from '../../../contracts/task-catalog.json';
const resources = Object.fromEntries(Object.entries(resourceTable).map(([key,url]) => [key, buildIdentity.docs_revision === 'unknown' ? url : url.replace('/blob/main/docs/', `/blob/${buildIdentity.docs_revision}/docs/`)])) as typeof resourceTable;

export function ResourceLink({ resource, children }: { resource: keyof typeof resources; children: ReactNode }) {
  const [error, setError] = useState('');
  return <span className="resource-link"><a href={resources[resource]} target="_blank" rel="noreferrer" onClick={event => {
    if (transport !== 'native') return;
    event.preventDefault(); setError('');
    void invoke('open_resource', { resource }).catch(() => setError('浏览器未能打开。可复制链接手动访问。'));
  }}>{children}<span aria-hidden="true"> ↗</span>{resources[resource].includes("/blob/main/docs/") && <small className="resource-version">最新开发说明</small>}</a>{error && <span className="resource-error" role="alert">{error}<code>{resources[resource]}</code></span>}</span>;
}

export default function DeveloperResources() {
  return <section className="developer-resources separated" aria-label="开发者与项目"><h3>开发者与项目</h3><p>由 Faye 发起和开发。Lintel 是独立工具；源码、使用说明与能力边界都在项目仓库中。</p><div className="resource-links"><ResourceLink resource="developer">Faye · GitHub</ResourceLink><ResourceLink resource="source">Lintel 源码与说明</ResourceLink></div><p className="small-print">完整产品目标、实际验收证据与当前限制见项目仓库。</p><h4>第一次用 VPS，也有地方慢慢学</h4><p>Infra Field Guide 从 VPS 101、SSH 登录讲起，再到部署、备份、迁移与排障；按眼前的问题选一章就好。</p><div className="resource-links"><ResourceLink resource="vps-guide">Infra Field Guide 仓库</ResourceLink><ResourceLink resource="vps-basics">VPS 101</ResourceLink><ResourceLink resource="ssh-troubleshooting">连接与排障</ResourceLink></div><p className="small-print">链接在系统浏览器打开，不会附带环境资料或诊断输出。</p></section>;
}


export const productTasks=taskCatalog.tasks;
export function TaskHelp({ onProtect, onWork, onCleanup, onBrowser, onRemote, onRecovery,onChooseTask }: { onProtect: () => void; onWork: () => void; onCleanup: () => void; onBrowser: () => void; onRemote: () => void; onRecovery: () => void;onChooseTask?:(id:string)=>void }) {
  const actions:Record<string,{label:string;go:()=>void}>={
    reduce_egress:{label:'选择保护方案',go:onProtect},
    preserve_work:{label:'工作保全',go:onWork},
    repair_cleanup_retire:{label:'清理与重建',go:onCleanup},
    browser_profile:{label:'浏览器工作空间',go:onBrowser},
    ssh_remote:{label:'主机与 SSH 连接',go:onRemote},
    recover_results:{label:'记录与恢复',go:onRecovery},
  };
  return <><h3>从眼前的目标开始</h3><div className="help-tasks">{productTasks.map(task => <section key={task.id} data-task-id={task.id}><h4>{task.label}</h4><p>{task.summary}</p><button className="text-button" onClick={()=>{onChooseTask?.(task.id);actions[task.id].go();}}>{actions[task.id].label}<span aria-hidden="true"> →</span></button></section>)}</div><div className="resource-links"><ResourceLink resource="operator-guide">完整人类操作指南</ResourceLink><ResourceLink resource="agent-guide">Agent CLI 指南</ResourceLink></div></>;
}
