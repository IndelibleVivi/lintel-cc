# 认证来源、清理与独立复查

简体中文 · [English](cleanup.en.md) · [操作指南](operator-guide.md#cleanup)

**本指南对应 2026-11-13 开发源码，已发布 App 0.1.1／CLI 0.1.0 尚未包含这轮能力。** 它延续 Lintel 的环境、工作保全、重建与原任务查询。具体主机和配置根始终是操作目标。

## 先理解这份清单

`claude setup-token` 生成的长期 OAuth token 只会打印到终端，命令本身不保存它。之后可能由 shell、Claude settings、IDE、服务或 CI 注入，所以没有一个固定的“setup-token 文件”可全部删除。普通登录的文件与 Keychain 又是不同来源。官方机制与版本依据见[工程证据记录](credential-provenance-evidence.md)。

从侧栏“清理与重建”进入“清理本机登录”，查看认证来源。清单只在有限的已知位置读取必要内容，不运行 shell 初始化、helper、hooks、MCP 或 Claude，不扫描历史记录或整块磁盘，也不展示 token。环境／组件入口指向同一清理流程。

| 看到的事实 | 它能说明什么 | 不能据此认定什么 |
| --- | --- | --- |
| 本地来源已识别 | 精确文件、Keychain 登录字段、JSON 字段或受支持的 shell 赋值存在 | 此来源正在被 Claude 使用，或 token 仍有效 |
| 当前执行环境已观察到 | Lintel App／当前 runner 进程中存在相应变量；空值另外标明 | 变量来自哪一行配置、其值可用于认证，或其他 Terminal／IDE 也相同 |
| 仅发现声明 | 配置中出现了 helper、动态 shell 或认证选择项 | 其脚本已运行、返回了有效 token，或已被该客户端采用 |
| 无法检查／无法归属 | 权限、格式、类型、预算或归属阻止进一步核对 | 不存在凭据，或可以安全删除 |
| 其他上下文未覆盖 | 其他进程、IDE、服务、主机、CI 或管理员系统可能另有输入 | 本机这次检查已经覆盖这些地方 |

存在凭据与正在使用凭据分别表达。多个来源可以同时存在，清单不会凭一个优先级表猜当前登录。需要官方 CLI 的当前报告时，明确选择运行状态检查；它只代表本次执行器和准确配置根。

## 在 App 中完成一次清理

1. 顶栏确认主机、Claude Code 环境与配置目录。自定义 `CLAUDE_CONFIG_DIR` 必须核对完整路径；项目工作目录与配置根分别表达。
2. 在“清理本机登录”选择修复登录、清理并重建或退役。刷新后，已保存登录（文件与可处理的 Keychain 字段）默认建议选中；认证注入按需逐项选择，或明确使用该组的全选／清空，不自动勾选共享 shell。技术依据与准确位置可逐步展开；无法处理项说明原因。shell 和用户配置可能影响同一用户的多个环境，必须看其共享范围。
3. 关闭相关 Terminal、IDE 与自动重启来源。明确绑定的 systemd 服务仍使用原有独立暂停计划。已发现的运行写入者或无法完成安全检查会阻止执行，勾选停止确认不能绕过它们。
4. 预览同一份清理计划。所选来源的准确位置默认展开；预览列出精确变更范围、将保留的工作、未知项，以及重建／退役需要的工作保全。重建和退役继续要求至少一个工作类别及加密口令。
5. 审阅后一次批准。执行前再次核对所选文件／Keychain 项和冻结范围；修改中发现外部编辑会停止后续动作，不覆盖它。认证编辑不保存可恢复 token 的明文或密文备份。
6. 先看本地移除、仍存在与待核对数量，再查逐项来源和步骤。当前执行器状态单列，其他上下文与服务端撤销可展开查看。重新检查只关联原任务读取，不重执行；提交结果不明时保留并查询原 ID。

一次批准只合并这份计划可以安全处理的来源。不能自动处理的对象会留下明确说明和下一步，不会悄悄扩大到其他主机或账户。

显式选择清理后的运行状态检查时，计划也会冻结认证程序的路径、身份或缺失状态。预览后才安装／发现程序会使旧批准返回 `executable_changed`，需要重新预览；原计划已选择该检查时，带原任务 ID 的复查也不会自动改用后来出现的程序。不带原任务 ID 的一次新显式检查可以使用当前已发现的程序。

## 本轮可处理与保留的范围

| 来源 | 开发源码的处理方式 |
| --- | --- |
| 目标 `.credentials.json` 中已识别 `claudeAiOauth` | 精确移除该登录字段；保留其他字段与文件用途。未知／损坏 schema 不按整个文件删除 |
| 目标客户端状态中已识别的 `primaryApiKey` | 默认 root 对应 `~/.claude.json`，自定义 root 对应该 root 的 `.claude.json`；只移除该字段，其他状态与 MCP 配置保留 |
| 目标用户 settings 的已知认证环境变量 | 按所选字段精确移除；保留其他配置。空值、类型异常和实际生效问题分开显示 |
| 用户 settings 中 `apiKeyHelper` | 移除该声明；不运行 helper，不删除脚本、密码库或它的外部凭据 |
| 已知 shell 启动文件中的受支持独立字面量赋值 | 精确移除被冻结的赋值范围，保留文件其他内容；复杂语法／动态注入保留并显示限制 |
| Lintel／runner 已继承的变量 | 只观察，不尝试改写父 shell、其他进程或永久环境；重新启动相关入口后再查 |
| 项目／managed 配置、未知 shell、IDE、服务和外部 profiles | 按当前实现的有限读取说明候选来源与缺口；未获安全编辑支持的行不可选择 |
| macOS 有限服务中的 Keychain 登录 | 精确移除已识别的 `/claudeAiOauth`；保留 `mcpOAuth`、`pluginSecrets` 和未知字段，不删除整个 item。锁定、拒绝访问、格式未知或超限保留明确限制 |
| 会话、指令、记忆、项目、MCP／插件配置和未知凭据字段 | 保留；工作保全／迁入仍使用原有独立内容契约 |

来源清理采用字段级、版本化的有限识别。`claudeAiOauth` 或 `primaryApiKey` 消失并不证明其他类型凭据已清除。混合 `.claude.json` 除所选已支持字段外继续保留，不能为了清登录删除其中 MCP 或其他工作状态。重建会准备新 root 并迁入已批准工作；退役停用 Lintel 中的旧启动入口；两者均不等于卸载客户端或清空原目录。

Shell 支持按整份文件保守判断：已知启动文件中出现多行语法、控制结构、函数、子 shell、heredoc、续行、管道或重定向时，本文件全部只报告声明，不提供编辑。可共存的普通单行 `source`、alias、PATH 声明不会被执行；仅完整、独立的字面量认证赋值可选择。fish 当前只读观察。JSON 单文件最多 1 MiB，shell 文件最多 256 KiB；来源模块合计最多 8 MiB、256 项来源、128 个 shell 提及位置，启动候选扫描另有最多 16 MiB 的独立预算。超限显示覆盖不完整，不按未发现处理。

macOS 使用原生 Security.framework，按服务名和当前用户账号读取，批准绑定原 item 的 persistent reference、账号和整份 payload 的每计划加盐摘要；这些身份仅进私有计划，不进入公开来源或回执。`SecItemUpdate` 只更新 data，不重建 item；凭据只在内存转换，不导出到临时文件或恢复备份。每项最多 1 MiB，计入来源模块 8 MiB 总预算。`limits.keychain_scan_complete` 只表示本次有限服务读取是否完整，`limits.keychain:"selected_services_only"` 不代表整个 Keychain 已扫描。默认根核对 bare 服务及 root hash，自定义根仅核对 hash；命名与版本依据见[证据记录](credential-provenance-evidence.md)。

### 为什么本轮不能勾选自动官方注销

Anthropic 的当前 [Keychain 恢复说明](https://code.claude.com/docs/en/troubleshoot-install) 指出，`/logout` 还会移除保存的 MCP 登录和插件敏感值。CLI `auth logout` 尚无已验证且满足本轮保留范围的适配。因此新自动注销请求返回 `official_logout_scope_unsafe`，不会执行命令；这项决定来自官方文档与范围要求，不是本轮真实客户端实跑结论。

此限制针对官方 logout 的额外影响，不妨碍上述精确 Keychain 登录字段清理。未选服务、未知历史别名、其他账号与共享 profiles 仍保留覆盖缺口；本地结果不表示服务端撤销。

## 用 CLI 查看、选择与批准

以下 ID 是占位；必须使用当前主机返回的真实环境、来源、计划 ID 与 hash。命令正文没有 token。`call` 进入与 App 相同的 operation schema 和 core，stdin 中的 JSON 不是另一套清理实现。

```sh
lintel describe auth_inventory --json
lintel schema plan_cleanup
lintel call auth_inventory <<'JSON'
{"environment_id":"<environment-id>","project_cwd":"/synthetic/project"}
JSON
```

`project_cwd` 可省略；提供时必须是当前目标主机上存在的准确目录。它扩充只读来源上下文，不把项目文件默认为可清理目标。

常用 named CLI 也映射到上述同一操作：

```sh
lintel auth inventory --environment '<environment-id>' --json
lintel auth cleanup plan --environment '<environment-id>' --recipe repair_login \
  --writers-confirmed-stopped --auth-source '<source-id-from-inventory>' --json
lintel auth verify --environment '<environment-id>' --job '<original-job-id>' --json
```

`--auth-source` 可以重复；可选 `--project-cwd` 补充项目只读上下文。清理计划的 `--verify-auth-status` 与独立复查的 `--observe-runtime` 才请求官方运行状态，默认不启用。以下 raw 示例展示相同字段；选择一种方式生成计划即可。

从清单选择可处理行的原始 ID，明确创建计划：

```sh
lintel call plan_cleanup <<'JSON'
{
  "environment_id":"<environment-id>",
  "recipe":"repair_login",
  "writers_confirmed_stopped":true,
  "official_logout":false,
  "auth_source_ids":["<source-id-from-inventory>"],
  "verify_auth_status":false
}
JSON
lintel plan show '<returned-plan-id>'
lintel job submit --plan '<returned-plan-id>' --approval '<returned-plan-hash>'
lintel job show '<returned-plan-id>'
```

`auth_source_ids` 冻结选择；原清单刷新、文件变化或旧计划失效时重新预览。它不是可自由填写的删除路径。`verify_auth_status:true` 明确请求运行官方状态命令；默认静态复查不运行 Claude。远端使用已有有限 SSH request／submit 与原任务查询，来源 ID 不跨主机混用。

独立重新检查时可以关联原任务，默认仅重新读取有限来源：

```sh
lintel call auth_verify <<'JSON'
{"environment_id":"<environment-id>","job_id":"<original-job-id>","observe_runtime":false}
JSON
```

显式选择 `observe_runtime:true` 才调用官方 `auth status`。它可能访问客户端认证设施、执行 helper 或联网；结果不暴露原始 token 或账户资料。验证先运行所选状态检查，再做最终文件读回，以观察 status／helper 可能重新生成的凭据。CLI 版本／输出无法确认准确配置根时报告限制，不把退出码、未知格式或网络失败转成“所有认证已清除”。

## 看懂清理后的结果

| 结果层次 | 如何继续 |
| --- | --- |
| 所选本地来源已处理 | 核对具体来源和步骤；其他行仍可能存在 |
| 来源再次出现、编辑失败或无法读取 | 保留当前文件，查看原任务错误和位置；查明回写者后重新预览，不自动重删 |
| 当前执行器仍带认证变量或仍可认证 | 在实际使用的 Terminal／IDE／服务中停止旧进程、处理其他来源并重新启动；不要把已处理文件再删一遍 |
| 未选／无法检查的 Keychain、profiles 或其他上下文未知 | 在对应入口独立核对，或让管理员处理；本次本地结果不扩张到它们 |
| 服务端撤销未验证 | 按凭据提供方的正常管理流程独立撤销／更新。删除本机声明不使其他设备、CI 或已复制 token 自动失效 |

Shell 文件已改不会改变已经运行的进程；Claude settings 中删除变量也不能保证当前 session 立即忘掉它。重新启动后的环境再次检查，才产生新的观察。

## 旧计划、中断和保全

未带新认证范围绑定的旧、未执行清理计划需要重新预览。旧已接受任务与回执继续按原 ID 查询，不重放。旧 raw caller 仍可省略新增字段，但新计划只默认选择可精确识别的目标本地登录字段；不再把混合凭据文件或 `.claude.json` 整体作为登录清理目标。

提交返回 `submission_uncertain` 时，保留原计划 ID 并查询原任务。安全检查尚未完成时，查询可能暂时返回 `job_not_found`；这表示尚无持久接收证据，不能据此再次提交或换一个 ID 执行。等待后继续查询；若始终无法确认结果，保持未知并保留原 ID，不将缺少回执解释为已完成或从未启动。

重建／退役保持工作加密保全、准确新 root、冲突检测和原任务查询。新认证清理不把旧 token 复制到计划、日志、回执或状态恢复备份；此前已经存在的备份、shell history、同步副本与其他设备不被静默清扫。若原任务中断，先看已完成的工作归档、新 root 与来源步骤，保留原任务证据。

文件精确编辑会将原文件移入同目录的私有暂存，再以不覆盖方式发布已移除目标字段的新文件。原文件是移动而非备份复制；成功读回后删除暂存原件。中断或冲突可能留下含凭据的原件或发布暂存，回执及独立复查的 `retained_sensitive_material` 会列出位置与状态并阻止将本地清理判为完成。暂存目录被改名／替换、身份失联时，`location_unknown` 的路径只是最后已知位置，不能推断材料已经消失；Lintel 不扫描追踪。不要仅因目标字段已不在原路径就忽略这些材料；按原任务核对，Lintel 不自动扫除或重放。

Keychain 更新紧邻前重读原项并比较完整内容，更新后完整读回；它没有对任意外部 writer 的原生 CAS。发现变化或更新不确定时保留原 ID 核对，不自动重试更新。安全编辑仍需停止外部写入者。冻结和复查可拒绝已观察到的编辑，但不能对未协作的任意进程提供跨文件原子事务，也不能证明 SSD、快照或服务端的物理销毁。真实 macOS Keychain 与正常用户登录验收独立于 synthetic 测试，准确状态见 [current-state.md](current-state.md)。
