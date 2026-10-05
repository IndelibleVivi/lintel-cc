import { useState, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { transport } from './api';

const resources = {
  developer: 'https://github.com/IndelibleVivi',
  source: 'https://github.com/IndelibleVivi/lintel-cc',
  'operator-guide': 'https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/operator-guide.md',
  'policy-guide': 'https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/operator-guide.md#protect',
  'work-guide': 'https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/operator-guide.md#work',
  'cleanup-guide': 'https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/operator-guide.md#cleanup',
  'recovery-guide': 'https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/operator-guide.md#recovery',
  'agent-guide': 'https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/agents.md',
  'remote-setup': 'https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/remote.md',
  'browser-setup': 'https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/browser.md',
  'vps-guide': 'https://github.com/IndelibleVivi/infra-field-guide',
  'vps-basics': 'https://github.com/IndelibleVivi/infra-field-guide/blob/main/docs/01-vps-basics.md',
  'ssh-troubleshooting': 'https://github.com/IndelibleVivi/infra-field-guide/blob/main/docs/08-troubleshooting.md',
} as const;

export function ResourceLink({ resource, children }: { resource: keyof typeof resources; children: ReactNode }) {
  const [error, setError] = useState('');
  return <span className="resource-link"><a href={resources[resource]} target="_blank" rel="noreferrer" onClick={event => {
    if (transport !== 'native') return;
    event.preventDefault(); setError('');
    void invoke('open_resource', { resource }).catch(() => setError('浏览器未能打开。可复制链接手动访问。'));
  }}>{children}<span aria-hidden="true"> ↗</span></a>{error && <span className="resource-error" role="alert">{error}<code>{resources[resource]}</code></span>}</span>;
}

export default function DeveloperResources() {
  return <section className="developer-resources separated" aria-label="开发者与项目"><h3>开发者与项目</h3><p>由 Faye 发起和开发。Lintel 是独立工具；源码、使用说明与能力边界都在项目仓库中。</p><div className="resource-links"><ResourceLink resource="developer">Faye · GitHub</ResourceLink><ResourceLink resource="source">Lintel 源码与说明</ResourceLink></div><p className="small-print">完整产品目标、实际验收证据与当前限制见项目仓库。</p><h4>第一次用 VPS，也有地方慢慢学</h4><p>Infra Field Guide 从 VPS 101、SSH 登录讲起，再到部署、备份、迁移与排障；按眼前的问题选一章就好。</p><div className="resource-links"><ResourceLink resource="vps-guide">Infra Field Guide 仓库</ResourceLink><ResourceLink resource="vps-basics">VPS 101</ResourceLink><ResourceLink resource="ssh-troubleshooting">连接与排障</ResourceLink></div><p className="small-print">链接在系统浏览器打开，不会附带环境资料或诊断输出。</p></section>;
}


export function TaskHelp({ onProtect, onWork, onCleanup, onBrowser, onRemote, onRecovery }: { onProtect: () => void; onWork: () => void; onCleanup: () => void; onBrowser: () => void; onRemote: () => void; onRecovery: () => void }) {
  const tasks = [
    { title: '减少外发，保留需要的功能', text: '选择目标环境，预览每项设置与版本条件，再批准。已有 Claude 会话不会自动重启。', action: '选择保护方案', go: onProtect },
    { title: '收好工作，或换一份新开始', text: '只归档，或归档后准备新环境。旧登录保留；新环境的保护方案与登录分别进行。', action: '工作保全', go: onWork },
    { title: '修复登录、重置状态、退役环境', text: '先停止写入者，再核对精确文件与官方注销。新环境准备好后才处理旧状态。', action: '清理与重建', go: onCleanup },
    { title: '处理具体浏览器 profile', text: '连接并配对 profile，在扩展确认动作。清理须真正重启浏览器后再确认收尾。', action: '浏览器工作空间', go: onBrowser },
    { title: '在 SSH 主机上做同样的事', text: '选择已有 SSH alias，检查 runner 与目标。中断时只查询原任务，保留可恢复记录。', action: '主机与 SSH 连接', go: onRemote },
    { title: '找回结果或恢复配置', text: '用原任务 ID 查看已完成步骤与错误。恢复配置和恢复服务分别预览批准。', action: '记录与恢复', go: onRecovery },
  ];
  return <><h3>从眼前的目标开始</h3><div className="help-tasks">{tasks.map(task => <section key={task.title}><h4>{task.title}</h4><p>{task.text}</p><button className="text-button" onClick={task.go}>{task.action}<span aria-hidden="true"> →</span></button></section>)}</div><div className="resource-links"><ResourceLink resource="operator-guide">完整人类操作指南</ResourceLink><ResourceLink resource="agent-guide">Agent CLI 指南</ResourceLink></div></>;
}
