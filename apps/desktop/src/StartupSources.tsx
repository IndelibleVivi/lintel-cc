import type { StartupInspection, StartupSource } from './types';
import { Notice } from './ui';

const scopes:Record<string,string>={user:'用户设置',user_instructions:'用户指令',user_rules:'用户 rules',user_agents:'用户 agents',user_skills:'用户 skills',managed_candidate:'本地 managed 候选',credential_metadata:'凭据文件（仅元数据）',config_global_state_metadata:'配置目录账号／MCP 状态（仅元数据）',shared_global_state_metadata:'共享账号／MCP 状态（仅元数据）',shared_profile_metadata:'共享认证 profile（仅元数据）',project:'项目设置',project_local:'项目本地设置',project_mcp:'项目 MCP',project_instructions:'项目指令',project_local_instructions:'项目本地指令',project_agents_instructions:'项目 AGENTS',project_rules:'项目 rules',ancestor_candidate:'上级目录候选',managed_file:'系统 managed 设置',managed_mcp:'系统 managed MCP',managed_instructions:'系统 managed 指令',managed_drop_in_directory:'Managed drop-ins 目录',managed_drop_in:'Managed drop-in'};
const states:Record<string,string>={observed:'已发现',observed_directory:'已发现目录 · 内容未核对',not_found:'已检查 · 未发现',unsupported_type:'文件类型暂不支持',access_limited:'访问受限'};
function SourceRow({source}:{source:StartupSource}) {
  const declarations=[source.hooks_declared?'hooks':null,source.mcp_declared?'MCP':null,source.api_key_helper_declared?'credential helper':null,source.policy_helper_declared?'policy helper':null,source.plugins_declared?'plugins':null,...(source.auth_selectors_declared ?? [])].filter(Boolean);
  return <li><div><span>{scopes[source.scope] ?? source.scope}</span><small>{states[source.state] ?? '未知'}</small></div><code>{source.path}</code>{declarations.length>0 && <p>有声明：{declarations.join(' · ')}</p>}{source.content_state==='unknown' && <p>内容无法解析，实际参与情况未知。</p>}{source.content_state==='limit' && <p>达到读取上限，正文未核对。</p>}</li>;
}
export default function StartupSources({inspection}:{inspection?:StartupInspection}) {
  if(inspection?.schema!=='lintel.startup/1')return <Notice tone="warning">当前 runner 未提供启动来源核对。请更新 runner 后重新预览；当前预览不能批准启动。</Notice>;
  const visible=inspection.sources.filter(s=>s.state!=='not_found');
  const absent=inspection.sources.filter(s=>s.state==='not_found');
  const variables=inspection.authentication.runner_environment.filter(v=>v.present);
  return <section className="startup-sources" aria-label="启动来源核对"><h3>启动前，再看一眼来源</h3><p>配置 root 已选定，项目与受管配置仍可能参与。这里列出有限候选；实际加载要在目标终端核对。</p>
    {(!inspection.candidate_scan_complete || inspection.content_limited) && <Notice tone="warning">部分候选或正文未完整核对。请查看标注与尚未核验的来源。</Notice>}
    {visible.length>0?<ul className="startup-source-list">{visible.map(s=><SourceRow key={s.path} source={s}/>)}</ul>:<p>有限候选中未发现已有文件；认证与目录外来源仍未核验。</p>}
    {absent.length>0 && <details><summary>已检查但未发现的 {absent.length} 个候选路径</summary><ul className="startup-source-list">{absent.map(s=><SourceRow key={s.path} source={s}/>)}</ul></details>}
    <div className="startup-auth"><strong>认证还需要你在终端核对</strong><p>只查看凭据位置的元数据，没有读取 token、Keychain 或账号状态。</p>{variables.length>0 && <p>当前 runner 有这些环境入口：<code>{variables.map(v=>v.name).join(' · ')}</code>。新 Terminal 的 shell 环境未核验。</p>}</div>
    <details><summary>尚未核验的来源</summary><ul>{inspection.unknown_sources.map(s=><li key={s}>{s}</li>)}</ul></details>
    <ol className="startup-next-steps">{inspection.next_steps.map(s=><li key={s}>{s}</li>)}</ol>
  </section>;
}
