# Credential provenance：认证机制核对与实现边界

2026-11-13 · 工程证据记录 · [English](credential-provenance-evidence.en.md)。本文记录官方机制、Lintel 的设计判断及仍缺少的实机证据；用户操作见 [清理指南](cleanup.md)。新增能力属于开发源码，未包含在已发布 App 0.1.1 或 CLI 0.1.0 中。

## 1. 本轮怎样取得证据

核对 Anthropic 官方当前文档，并阅读 `crates/core/src/cleanup.rs`、`auth_sources.rs`、`auth_cleanup.rs`、`keychain.rs`／`keychain/macos.rs`、`components.rs`、`startup.rs`、App 清理入口与已有操作契约。文档的抓取日期为 2026-11-13；下列版本号是相应文档明确列出的门槛，不代表本轮运行过这些版本。这次机制静态核对不执行真实 Claude、登录、注销、Keychain 操作或模型请求。常规开发测试使用 synthetic roots 与 inert adapters；后来独立进行的临时 Keychain 平台 API 验证另见[当前状态](current-state.md)，不代表个人账号验收。

当前官方文档可更新，不能当成所有历史客户端的执行规范。源码中的有限识别规则、当前执行器的只读观察、显式 CLI 状态结果、真实客户端实效分别保留证据等级。

## 2. 官方来源与决定性事实

| 来源 | 本轮采用的事实 | 对 Lintel 的影响 |
| --- | --- | --- |
| [Authentication](https://code.claude.com/docs/en/authentication#generate-a-long-lived-token) | `setup-token` 生成一年期 OAuth token，打印后由使用者放进环境；命令本身不保存 token | 不建立虚构的 setup-token 文件定位规则。检查 token 的注入位置 |
| [Authentication：credential management](https://code.claude.com/docs/en/authentication#credential-management) | macOS 使用 Keychain；拒绝写入时，登录可落入文件 fallback。`CLAUDE_CONFIG_DIR` 同时影响 fallback 和 Keychain entry 的选择 | 默认根与自定义根分别定位；文件不存在不能排除 Keychain；不猜 Keychain 名称 |
| [CLI reference](https://code.claude.com/docs/en/cli-reference) | `auth status` 返回 JSON，退出码 0／1 分别表示已登录／未登录；`configDirectory` 需要 v2.1.268+。文档列出 `none`、`claude.ai`、`oauth_token`、`api_key`、`api_key_helper`、`third_party` | 有限解析并验证 root；不把未知 schema 当作退出成功。只公开有限状态，丢弃身份字段和原始输出 |
| [Environment variables](https://code.claude.com/docs/en/env-vars) | `CLAUDE_CODE_OAUTH_TOKEN` 优先于 Keychain；refresh token／scopes 可用于自动登录。settings 删除变量不会移除已经运行的会话环境；路径值不会展开 `~`／`$HOME` | 清除声明后仍需重新启动相关上下文；不在当前进程中执行 shell rc 来“证明”生效 |
| [Settings reference：env／apiKeyHelper](https://code.claude.com/docs/en/settings-reference) | `env` 通常覆写 shell，Desktop／宿主启动环境有例外；项目／local settings 忽略 OAuth 注入变量。`apiKeyHelper` 是实际 shell 命令，可取得动态凭据 | 声明存在与实际使用分开；只读 inventory 不运行 helper，不输出其正文 |
| [Settings files and precedence](https://code.claude.com/docs/en/settings) | user、project、local、managed 与一次性 `--settings` 属于不同输入；`/status` 的 settings 来源列表不能逐字段归因 | 有界扫描只能列出候选来源。命令行、MDM、服务器托管策略与未选项目仍可能改变实际认证 |
| [VS Code](https://code.claude.com/docs/en/vs-code) | 扩展包含自己的 CLI，`claudeCode.environmentVariables` 独立注入；GUI 启动可能不继承 shell。扩展的 `CLAUDE_CONFIG_DIR` 必须为绝对路径 | Lintel GUI、Terminal、IDE、SSH runner 的环境不等价；不能把一个执行器的结果提升为整机结论 |
| [WIF reference：profiles](https://platform.claude.com/docs/en/manage-claude/wif-reference#profile-configuration-file) | 目录由 `ANTHROPIC_CONFIG_DIR` 或 macOS/Linux 默认 `~/.config/anthropic` 决定。`configs/<profile>.json` 是配置；`credentials/<profile>.json` 含缓存 token。名称来自 `ANTHROPIC_PROFILE`、`active_config` 或 `default` | Profiles 是跨 Claude Code／SDK／`ant` 的共享来源；元数据观察不授予删除权，也不读取 token 文件 |
| [Troubleshoot installation and login](https://code.claude.com/docs/en/troubleshoot-install) | Keychain 恢复说明明确：`/logout` 还移除保存的 MCP server 登录和 plugin sensitive values | 当前没有有依据的“仅删除 Claude 登录并保留这些凭据”的官方 logout 参数。Lintel 本轮拒绝自动执行这一扩大范围的动作 |

本轮另静态核对官方 NPM 分发的 [`@anthropic-ai/claude-code` 2.1.101](https://registry.npmjs.org/@anthropic-ai/claude-code/2.1.101) 中的 `cli.js`，未执行该客户端：生产服务名为 `Claude Code-credentials`；设置 `CLAUDE_CONFIG_DIR` 时追加 `-` 与 `SHA256(NFC(configdir))` 的前 8 位小写十六进制；account 使用 `USER`，缺省使用 OS username。该存储容器中的 Claude 登录位于 `claudeAiOauth`，与 MCP／插件字段共存。Lintel 据此限定服务和字段，不复制 vendor 实现代码。默认根同时检查 bare 名与 root hash，自定义根仅检查 hash；当前继承的原始配置路径 canonicalize 后等于目标根时，也检查原始路径的 NFC hash。未知历史别名和不同账号上下文保留未覆盖，不扫描整个 Keychain。2.1.323 仅查询过 registry 元数据，未执行或审阅其 native binary，不能宣称该版本的存储机制已验证。

## 3. 优先级能说明什么

官方 [Authentication precedence](https://code.claude.com/docs/en/authentication#authentication-precedence) 的常见 CLI 顺序为：已选云 provider、`ANTHROPIC_AUTH_TOKEN`、已采用的 `ANTHROPIC_API_KEY`、`apiKeyHelper`、`CLAUDE_CODE_OAUTH_TOKEN`、profile／federation、普通登录。交互式 API key 需要采用确认，非交互方式不同；gateway 登录另有优先规则。profile 的 auth mode、是否明确命名、当前登录与宿主 surface 也会改变选择。

因此 inventory 不实施一个宣称“实际使用来源”的简化排序器。它可说明“此变量存在于 Lintel 当前执行环境，值为空／非空”“此文件声明了一个 helper”“此凭据容器存在”。`presence` 只表达存在性，`value_state` 另外区分空值、非空、未设置或非字符串；两者都不等于有效认证。只有显式运行官方状态命令，才产生该执行器／root／时点的 CLI 报告；即使报告来源为 `oauth_token`，也不证明来自某一 shell 文件，更不证明 token 是由 `setup-token` 生成。任意 OAuth access token 都可能通过同名变量注入。

`apiKeyHelper` 的脚本正文、环境变量值、账户身份与 token 不进入 inventory、预览、记录或日志。扫描未知/损坏内容时保留未知状态，不展示一段“便于排错”的原文。来源 ID 用于同一主机和环境下的选择，不是可共享的凭据指纹。

## 4. 两项产品决定

### 精确清理已识别来源

新的来源选择沿用 `plan_cleanup` 与原批准、执行、回执路径。支持来源必须有明确位置、有限语法、当前文件身份与内容绑定；共享文件只改被冻结的精确范围。Shell 命令替换、条件、函数、动态 source、无法归属的 Keychain 和共享 profile 保留为不可自动处理项，不执行内容来补齐证据。

凭据文件的私有完整 schema 没有由上述官方文档稳定定义。本轮识别 `.credentials.json` 的 `claudeAiOauth` 槽位和客户端状态中的字符串 `primaryApiKey`，保留其余字段；这是一项有限的实现支持，不是全部版本、API key 或 MCP 存储的完备模型。未知格式应报限制。默认 root 的客户端状态位于 `~/.claude.json`，自定义 root 位于该 root 的 `.claude.json`。删除整个文件会扩大到未知数据，因此不能用于实现新的“清理已识别认证来源”。

认证清理不产生 token 的恢复备份。文件编辑在有界内存计算；持久绑定只保存私有带盐摘要、定位信息与身份元数据。事务会将原文件移动到源目录旁的私有暂存，再以不覆盖方式发布替代文件；替代文件仍可能含必须保留的非目标凭据。成功后读回并移除暂存原件；中断或冲突残留通过 `retained_sensitive_material` 报告，不能声称清理完成。暂存目录改名／替换导致身份失联时，`location_unknown` 只保留最后已知位置，不扫描追踪，也不将旧路径不存在视为原件已毁。计划或普通回执不得保存 before/after 原文；非目标秘密不进入日志或额外恢复包。中断后先核对原任务；不得靠重放清理恢复确定性。

Keychain 的生产实现使用 Security.framework：按有限服务与当前账号读取，把 persistent reference、账号和每计划加盐的完整 payload 摘要保存在私有绑定。它在内存中只移除 `/claudeAiOauth`，保留 `mcpOAuth`、`pluginSecrets` 与未知字段；`SecItemUpdate` 仅更新 data，保持原 item 身份，不删整个 item，也不导出 secret 临时文件或备份。每项 1 MiB 且计入来源模块 8 MiB 总预算；`limits.keychain_scan_complete` 与文件扫描分别表达，`limits.keychain:"selected_services_only"` 不代表全部 Keychain 已检查。停止已知写入者、紧邻更新前比较和完整读回不能提供真正的外部 writer CAS 或跨来源原子性；变化或不确定时保留原任务 ID 核对，不自动重试更新。

### 拒绝扩大范围的自动官方注销

本轮用户明确要求保留 MCP 及其他工作资料。官方文档记载的 `/logout` 额外影响与该约束冲突；CLI `auth logout` 尚无已验证且符合该保留范围的适配，本轮也没有对真实入口作实机验证。新请求若要求自动 logout，返回 `official_logout_scope_unsafe`；缺少新安全绑定或要求 logout 的未执行旧计划返回 `stale_auth_plan`，要求重新预览。已接受原任务继续可查询。不能将旧可选注销能力表述为本轮已经验证的安全路径，也不能以一个笼统勾选覆盖副作用。

官方 logout 的限制不等于 Keychain 不能精确清理。已识别字段使用上述计划路径；未选服务、其他账号、锁定或格式未知项分别保留未覆盖／待核对状态。用户自行选择官方认证管理时仍需理解 MCP／插件重新授权影响；Lintel 不自动发起，也不把本地移除解释为服务端撤销。

## 5. 验证需要保留的四层结论

1. **本地选中来源**：冻结的赋值或 JSON 字段是否已不在原位置；外部新写入、无法读取、未完成动作分别呈现。
2. **当前执行器**：Lintel 此进程／runner 继承的变量是否仍在；可选官方状态报告是否仍可认证。无法改变调用方 shell 或已经运行的其他进程。
3. **其他上下文**：Terminal、IDE、自定义 shell 入口、服务、其他主机、CI 与管理员注入独立核对。本次未见不等于不存在。
4. **服务端撤销**：没有独立服务端证据时保持未验证。本地文件或声明消失不能证明 token 已失效。

“所选本地步骤完成”与“其他来源仍存在”可以同时成立。独立 `auth_verify` 必须重新读取来源，不能复制清理回执作为验证；显式 `observe_runtime` 才调用官方命令。所选状态命令先执行，随后再做最终本地读回，以观察 status／helper 可能重新生成的凭据；命令不是纯静态观察。验证错误和未覆盖来源均不能降格成绿色成功。

## 6. 版本与后续证据

2.1.101 静态分发证据支持本次有限服务命名与字段识别，不证明用户已安装的版本、实际登录或所有历史 schema。2.1.323 只有 registry 元数据，不能替代 native binary 审阅。尚无满足保留范围的官方 logout 参数。macOS 原生 API 验收与真实客户端验收独立：前者由独立 `keychain-native-test` 在一次性 GitHub-hosted macOS 账号的临时 Keychain 中验证，不能默认在个人机器执行；后者仍需默认／自定义 root、Keychain 可用／锁定、文件 fallback 并存、不同启动环境及 MCP/plugin 保留的独立证据。`cfg(test)`、临时 HOME 与 `LINTEL_TEST_HOME` 的常规测试只用临时 home 中 `.lintel-test-keychain.json` 注入 store，不接触个人 Keychain。当前 native 验收状态以 [current-state.md](current-state.md) 为准；源码实现、synthetic 通过与真实用户登录是不同事实。

本记录不签发 App、CLI、网站或真实用户环境的验收。准确工程验证、提交与发行状态由 [current-state.md](current-state.md) 维护。
