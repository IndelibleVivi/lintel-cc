import { useState, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { transport } from './api';

const resources = {
  developer: 'https://github.com/IndelibleVivi',
  source: 'https://github.com/IndelibleVivi/lintel-cc',
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
  return <section className="developer-resources separated" aria-label="开发者与项目"><h3>开发者与项目</h3><p>由 Faye 发起和开发。Lintel 是独立工具；源码、使用说明与能力边界都在项目仓库中。</p><div className="resource-links"><ResourceLink resource="developer">Faye · GitHub</ResourceLink><ResourceLink resource="source">Lintel 源码与说明</ResourceLink></div><p className="small-print">Lintel 仓库当前为私有，访问需要仓库权限。</p><h4>第一次用 VPS，也有地方慢慢学</h4><p>Infra Field Guide 从 VPS 101、SSH 登录讲起，再到部署、备份、迁移与排障；按眼前的问题选一章就好。</p><div className="resource-links"><ResourceLink resource="vps-guide">Infra Field Guide 仓库</ResourceLink><ResourceLink resource="vps-basics">VPS 101</ResourceLink><ResourceLink resource="ssh-troubleshooting">连接与排障</ResourceLink></div><p className="small-print">链接在系统浏览器打开，不会附带环境资料或诊断输出。</p></section>;
}
