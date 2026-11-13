# Authentication and credential cleanup: independent release audit

委托日期：2026-11-14；本分项实际检索与 Linux 实验：2026-11-13 UTC。基线为最新 main `73f34ed06b5d630d2a36910baf88ff5713aede66`，修复纳入同一 release-readiness 分支。提交、候选产物与最终 CI 状态由本目录总交付记录关联。本报告不把开发源码、原生 API 验证或惰性客户端实验当作已安装公开产品。

## 结论

认证清理的主体实现有明确的边界和可靠的保全设计，具备进入下一版 Preview 候选的基础。本轮发现的主要问题在**执行上下文与外部客户端的新选择规则衔接**：同一个物理目录并不总是同一个凭据命名空间；Linux 停写检查又误用了路径字符串相等性。两类问题都能影响正常用户：原本已登录的默认环境由 Lintel 启动后显示未登录，或清理时漏掉仍可回写目标凭据的进程。

这些问题已在源码和生产 CLI 路径修复。新的回归包含真实 PTY、独立 CLI 进程和 Linux `/proc` 进程识别，补上了 Rust 单元测试主动绕开的运行路径。Keychain 字段级更新的原生 API 设计没有被重写；保留既有身份绑定、未知字段保全和原任务不重放语义。

本分项建议 **CONDITIONAL GO，作为下一版 Preview 候选**：使用包含本轮修复的固定提交完成 canonical CI 和独立 macOS 临时 Keychain gate。真实 Claude 2.1.325 登录、系统锁定／拒绝提示、默认 Keychain 搜索列表和签名权限体验仍需要按下文程序取得原生证据；它们不能由 Linux 合成运行推导。此结论不替代 App 签名、Gatekeeper、安装包和 updater 的独立发行决定。

## 1. 审查路径与已成立的保护

审查范围包括 `auth_sources.rs` → `plan_auth_cleanup` → Core 原批准／任务接收 → `edit_source` → `auth_verification`，以及 `auth_command`、普通 CLI/TUI、Core Terminal、有限 launch/resume 的真实执行入口。阅读对应单元、故障注入、CLI journey 和临时 Keychain 原生测试，并重新核对官方认证资料与实际分发实现。

以下结论由源码与已有回归共同支持，值得保留：

| 能力 | 实现证据与实际边界 |
| --- | --- |
| 不把存在性当作实际认证来源 | `auth_sources::scan` 分开文件／Keychain 槽位、进程变量、声明、未知和未覆盖；`actual_use` 保持未验证。没有用一个优先级表猜用户当前账户 |
| 精确修改共享容器 | `auth_sources::json_replacement` 只接受已识别 locator；重复 JSON key 由 strict parser 拒绝。`claudeAiOauth`／`primaryApiKey` 之外的 MCP、插件和未知字段保留 |
| 源文件与批准绑定 | `read_stable` 使用有界、非跟随链接的描述符读取并复查；`salted_snapshot` 冻结身份、内容和父目录。外部变化在接收前或编辑窗口被拒绝 |
| 不生成 token 恢复包 | 计划保存每计划加盐摘要、locator 和私有身份，不保存 before/after 内容。文件原件移入私有暂存而不是复制成备份；Keychain 转换只在内存发生 |
| 中断后不伪造清理完成 | `edit_source` 在移动或更新前持久化 intent；残留原件、替代暂存和失联目录保留在 `retained_sensitive_material`。已尝试原 ID 只查询，不能再次删除重新登录的数据 |
| 精确 Keychain 身份 | `keychain/macos.rs::Query::new` 以 service/account 查询，并对冻结更新加入 persistent reference。`SecItemUpdate` 只传 `kSecValueData`，没有删除／重建整个 item |
| 更新前后核对完整容器 | `keychain::replace` 重读原身份和完整字节，更新后按同一身份读回；非目标字段变化会使原批准失效。它没有假装提供外部 writer 的 CAS |
| 状态运行显式且有界 | `auth_observation` 只接受有限方法、退出状态和目标目录；输出不带账户资料或 token。`bounded` 的进程组、超时和输出上限防止 helper 的挂起／管道继承失控 |
| 验证不是历史回执的复述 | 状态命令在最终本地读回之前运行，随后能发现 status/helper 重建的凭据；独立复查不改写历史回执 |

这些优势不是全由 mock 推导。例如字段保全、路径替换、共享文件改变、读回期间重新登录、残留暂存、原任务不重放均有独立输入与故障窗口；具体测试见 `auth_sources_tests.rs`、`auth_cleanup_tests.rs` 和 `keychain/native_tests.rs`。原生 item 行为由专门临时 Keychain gate 验证，日常 `cfg(test)` 不接触用户 Keychain。

## 2. 已确认并修复的问题

### AUTH-01：默认目录被改写成显式选择，实际使用另一凭据命名空间

**影响：高，发行前应修复。** 普通 macOS 用户在未设置 `CLAUDE_CONFIG_DIR` 的 Terminal 中登录；Lintel 自动发现默认 `~/.claude`；随后显式状态或启动代码无条件设置 `CLAUDE_CONFIG_DIR=<同一绝对目录>`。已检查的 Claude 分发按“是否显式选择”决定 bare 或带 hash 的 Keychain 服务，因此物理目录没变，使用的登录却变了。

基线源码位置：`cleanup.rs::auth_command`、`lib.rs::direct_tty_launch` 与 `terminal_launch_args` 的 `.command` 文本，以及 `apps/runner/src/main.rs::launch_command`。仅修 Core 有限启动会遗漏普通 `lintel launch <environment>` 和 TUI。

**修复：** `keychain.rs::execution_context` 建立共享的有限目录描述：默认根无匹配显式选择时保持变量未设置；匹配所选物理根的绝对原始拼写保持不变；自定义根没有匹配选择时使用批准的绝对根。`HOME` 也使用 Engine 的 home。直接 Command 与 macOS Terminal 脚本消费同一描述；Terminal 脚本先移除接收 shell 的两个目录变量再设置批准值。普通 CLI/TUI 通过 `launch_context` 与 `configure_client_authentication` 使用相同规则。

继承的相对目录若从当前入口解析到所选根，Lintel 拒绝执行，不把它改写成新的绝对凭据命名空间；在另一个项目 cwd 复用原相对值也不安全。状态报告的 `configDirectory` 接受**绝对且可解析到同一物理根**的路径，避免将诚实的末尾斜杠或别名报告误判为外部环境；凭据命名空间仍由原始选择拼写决定。

**回归：** `default_root_status_does_not_invent_an_explicit_credential_namespace`；`preserve_default_namespace_and_matching_absolute_spelling`；`tests/auth_sources_journey.py::execution_context_journey` 用独立 CLI 与真实 PTY 验证默认未设置、末尾斜杠、`child/..`、symlink 别名和匹配相对路径拒绝。测试客户端是只记录输入的本地 shell，明确不声称真实 Keychain 登录已成功。

### AUTH-02：新安全存储选择项可以跨根，旧预览也没有绑定选择

**影响：高，对使用独立安全存储选择的用户成立。** 2.1.325 native 分发包含 `CLAUDE_SECURESTORAGE_CONFIG_DIR`，优先于普通配置选择。基线只固定 `CLAUDE_CONFIG_DIR`；即使状态输出的 `configDirectory` 是选中根，凭据仍可能来自另一根。目录选择还可在分开的 CLI 预览／批准进程之间改变，原计划 hash 未检测这种变化。

**修复：** inventory 将该名称作为只读选择项；`execution_context` 仅允许可核对地指向所选根的原始绝对拼写，或默认根上的显式空值。其他根、相对、缺失或无法核对的安全存储路径在执行前返回 `auth_scope_unverified`，没有客户端运行。有限 Keychain 服务清单仅补充解析到所选根的原始 hash，不扫描其他根或账户。

`plan_launch`、`plan_resume` 和请求 `auth status` 的清理计划冻结目录描述。接收前重查，resume 的 decrypt/copy 窗口和最终启动再次检查；缺少新描述的未尝试旧计划返回 `stale_auth_context`。既有 durable intent／原任务查询先于新检查，保留原 ID，不把旧尝试变成另一次启动或清理。

此绑定只涵盖目录选择；它不宣称冻结 token、账户或接收 Terminal 的全部环境。其他认证输入仍通过已有 startup/inventory 的未验证边界说明。

**回归：** `secure_storage_override_cannot_cross_the_selected_root`；`authentication_approval_requires_the_frozen_execution_namespace`；`launch_authentication_directory_choice_is_frozen_before_intent`；`resume_without_frozen_authentication_directory_requires_new_preview`。fresh-process journey 在另一进程改变显式默认选择，通过真实 PTY 执行原有限请求，要求返回 `stale_auth_context` 且没有 launch intent；跨根安全存储错误还要求 status 调用计数不增加。

### AUTH-03：Linux 把仍可写目标的进程当作邻居

**影响：高，降低清理的必要停写保护。** 基线 `known_writers` 读取 `/proc/<pid>/environ` 后，只要单个 `CLAUDE_CONFIG_DIR` 的字节不等于所选绝对根就排除运行者。`/root/path/`、`/root/path/child/..`、symlink 别名、相对路径都能指向同一目录；新的独立安全存储变量还允许配置在邻居根、凭据写到所选根。后续文件／Keychain重查能发现部分冲突，但不能替代停写条件。

**修复：** `cleanup.rs::writer_has_other_root` 仅在普通配置路径是唯一、绝对、可解析且物理目标确实不同，并且显式安全存储路径也满足同样条件时，排除该运行者。未设置独立安全存储时沿用普通配置目标。空值、相对、重复、不可访问或无法核对的绑定保留为写入者。Lintel 不全局杀进程。

**区别真实成功与假成功的实验：** 原字符串比较在 `writer_root_aliases_are_not_dismissed_as_neighbors` 的末尾斜杠例上实际失败；替换为物理目录判断后通过。由于 Rust `cfg(test)` 主动使 `known_writers` 返回空，单元测试本身不能证明生产进程枚举。新增 journey 将系统 `/bin/sleep` 复制为临时可执行文件 `claude`，设置合成目录变量并启动子进程；真实 `ps`／`/proc` 路径必须对同根别名和“config=邻居、secure=所选根”返回 `writers_active`，对两个目标均为已知邻居允许预览。测试结束显式终止并等待这个惰性进程。

### AUTH-04：异常 USER 的版本规则变化可被误报为无 Keychain 登录

**影响：有限，异常启动环境或账号拼写。** 旧分发使用 `USER` 或 OS username；2.1.325 将不匹配 `[A-Za-z0-9._-]+` 的值映射到 `claude-code-user`。基线 Lintel 直接接受任意非空、长度不超过 256 的 UTF-8 `USER`，可能查询错误的 account 后报未发现。

**修复：** `keychain.rs::supported_account` 与 native `account(None)` 对无法与已检查版本安全对应的拼写返回 `keychain_unavailable`。它不猜测新 fallback、不扫其他 account，也不改变已有 frozen identity 的精确 account；普通 ASCII 账户维持原行为。非 UTF-8 名称同样保留未知。

**回归：** `account_selector_drift_is_unknown_instead_of_false_absence` 验证正常名称保持，空白、Unicode、引号、路径分隔、换行和超长值拒绝。该纯函数在 Linux 执行；native 调用联接必须由 macOS CI 编译／运行确认。

## 3. 当前官方机制与实际分发依据

### 官方文档

资料实际检索于 2026-11-13 UTC。以下是主要一手来源，详细来源矩阵仍在 [credential-provenance-evidence.md](../credential-provenance-evidence.md)：

- [Anthropic Authentication](https://code.claude.com/docs/en/authentication)：`setup-token` 产生长期 OAuth token、打印但不持久保存；后续注入位置才是需要识别的来源。认证优先级受 provider、host surface、API key 采用和 profile 模式影响。
- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference)：有限 `auth status` 方法、退出值及 `configDirectory` 版本条件。未知返回保持无法核对。
- [Credential management](https://code.claude.com/docs/en/authentication#credential-management)：macOS Keychain、文件 fallback 与 `CLAUDE_CONFIG_DIR` 选择，不能用文件不存在证明无登录。
- [Troubleshoot installation and login](https://code.claude.com/docs/en/troubleshoot-install)：logout 可能移除 MCP/plugin 的其他认证资料，支持保留现有自动 logout 拒绝策略。
- [Apple SecItemCopyMatching](https://developer.apple.com/documentation/security/secitemcopymatching(_:_:))、[SecItemUpdate](https://developer.apple.com/documentation/security/secitemupdate(_:_:)) 和 [TN3137: On Mac keychains](https://developer.apple.com/documentation/technotes/tn3137-on-mac-keychains)：核对 SecItem API、data-only 更新和 macOS file-based／data-protection Keychain 的不同环境。这里没有声称 API提供跨外部 writer 的比较后交换。

### 静态分发记录

没有执行 vendor 客户端，也没有读取真实凭据。只从官方 NPM registry 下载指定版本 tarball，核对发布元数据 integrity，提取和检查分发字节中的实现；vendor 二进制没有写入 Lintel 仓库。

| 字段 | 证据 |
| --- | --- |
| 当前检查包 | `@anthropic-ai/claude-code-darwin-arm64@2.1.325` |
| 元数据 | `https://registry.npmjs.org/@anthropic-ai/claude-code-darwin-arm64/2.1.325` |
| tarball | `https://registry.npmjs.org/@anthropic-ai/claude-code-darwin-arm64/-/claude-code-darwin-arm64-2.1.325.tgz` |
| tarball SHA256 | `7f498c560491eb4a4f7b60b44e7598ad0226761bf163c3d7b113e59d064668ac` |
| registry SHA512 integrity | `Ds1f1B+qy2BWDI+VnmDHwkNyTbXCwFrBtrinPD95RBmUubFJu9nCzMgn+wrg5VwwF6vtkkzGexogneZIYZKb6A==` |
| binary member | `package/claude`，347,406,976 bytes |
| binary SHA256 | `84590fefe6338be9087f5814b61f9a54c5f21524163a4c127cb5a808e31009f2` |
| 0-based 字节位置 | `H3` 安全存储目录：281,199,606；`N3` 服务名：281,199,777；`xH` account：281,200,072 |
| 历史对照包 | `@anthropic-ai/claude-code@2.1.101`，tarball SHA256 `c165d72c2a54d2f1eb9bbd6e2ab1369d5e14a3c46a7bc3b0791227ecaa0c1459` |

2.1.325 的服务函数仍从 NFC 目录拼写产生 SHA256 的 8 位后缀；默认未设置和显式设置不同。独立安全存储变量以“是否存在”和“是否为空”分别决定命名空间，不能仅判断真值。`claudeAiOauth` 与其他用途字段仍共同位于容器中。以上支持**有限识别和拒绝不明情况**，不形成永久内部 schema 合同。

registry integrity 与同一下载来源的 SHA256说明取回字节对应指定记录；它们不是独立签名来源认证，也不替代正式产品的签名／公证链。

## 4. 验证记录与重跑

以下在 Linux x86_64、合成临时 HOME/state、惰性本地客户端上执行；环境没有个人 Keychain。所有平台回归仍须对最终固定提交运行，不能用中间工作树结果替代 candidate CI。

| 实验 | 结果／能证明的范围 |
| --- | --- |
| 原始 writer 比较的反例 | 末尾斜杠误被排除；旧逻辑实际触发断言失败 |
| 物理 writer 判断单元回归 | 通过；包括不同根、别名、相对／未知／重复绑定以及独立安全存储目标 |
| `cargo test -p lintel-core --locked auth_ -- --nocapture` | 49 passed，0 failed；包括默认状态与旧批准拒绝的新例，复用全部 auth 既有冲突／保全回归 |
| `cargo test -p lintel-core --locked authentication_directory -- --nocapture` | 2 passed，0 failed；有限 launch/resume 在 durable intent／private copy 前拒绝缺失或变化的目录绑定 |
| `cargo test -p lintel-core --locked keychain::execution_context_tests` | 3 passed，0 failed；默认／显式拼写、secure selector、账户规则的纯函数与真实 shell 命令消费 |
| `cargo test -p lintel-core --locked --lib` | 359 passed，0 failed，2 ignored，267.75 s；两个 ignored 为 jobs metadata scale/100k benchmark，macOS native test 在 Linux 不编译，不能把此总数当 native 证明 |
| `cargo build -p lintel-runner --locked` + `python3 tests/auth_sources_journey.py` | PASS 原精确清理 journey；PASS 新生产 execution-context journey，包括真实 PTY 普通启动、跨进程批准变化和 `/proc` 写入者识别 |
| macOS Security.framework | 本工作环境不能执行；复用并要求重新运行独立 `keychain_native_engine_journey`，不得宣称本机取得 native 证据 |

建议直接执行现有入口，不另建平行验收模型：

```sh
cargo test -p lintel-core --locked auth_
cargo test -p lintel-core --locked authentication_directory
cargo test -p lintel-core --locked keychain::execution_context_tests
cargo test -p lintel-core --locked writer_root_aliases
cargo build -p lintel-runner --locked
python3 tests/auth_sources_journey.py target/debug/lintel
```

故障注入测试中的合成 panic 是有意模拟 durable intent 后中断，由测试捕获；不能把日志里出现 panic 一词直接算作 suite 失败。反过来，Rust unit 模式绕过 `known_writers` 是实际覆盖缺口，必须保留新生产进程实验，而不能只看总通过数。

## 5. 剩余原生验收：具体条件与执行方法

1. **候选提交的临时 Keychain gate。** 使用仓库现有 `keychain-native-test`／`keychain_native_engine_journey`，只在 `GITHUB_ACTIONS=true`、`RUNNER_ENVIRONMENT=github-hosted` 的一次性 macOS 账号运行。它创建独立 Keychain，验证默认／自定义 service、原 persistent identity、file+Keychain同次批准、未知/MCP/plugin保留、外部修改拒绝和重放不更新。普通 `cargo test` 的 ignored 标签必须由此 gate 单独兑现，不能算无条件通过。
2. **当前真实分发的 namespace 选择。** 在同类受控 macOS 账号，以固定版本和 hash 的客户端，用合成项目及测试凭据容器分别覆盖默认 unset、显式默认、尾斜杠绝对别名、自定义根和 `CLAUDE_SECURESTORAGE_CONFIG_DIR`。先独立记录 service/account/原 item identity，再用 Lintel 启动／显式状态核对目标选择。任何真实联网登录需另用专用测试账号；不得把个人 Keychain交给自动程序。输出只保留命名空间、布尔结果和摘要，不能输出 token。
3. **锁定／拒绝交互和系统搜索列表。** 对临时 Keychain执行锁定后读、拒绝授权、item被独立替换；检查 UI始终将结果列为 unknown/denied、原任务可找到且不自动重试。现有 happy-path临时 Keychain不能证明正式签名App访问 login Keychain时的系统提示。真实系统提示的验收由 macOS桌面受控会话完成，不能伪造提示选择。
4. **未协作外部 writer。** Keychain没有产品可用的跨进程 payload CAS。既有“停止写入者、紧邻重读、完整读回”是明确运行条件；独立安全存储目录已纳入 Linux排除判断。macOS仍保守保留已知 Claude进程，不为方便清理而扩大排除。用户忽略未知IDE/自动重启来源时，最可能看到部分完成或仍存在，需要按原任务复查；不能保证任意第三方 writer永不在最后两个API调用间写入。

前两项中的自动部分已由现有 gate和新增回归给出可执行入口；实际完成状态必须写回总验收表。无需为了本次发行实现完整账户抽象、扫描所有历史服务名、自动登录或全局清扫。它们既增加范围，也不能修复本次已经明确的质量问题。

## 6. 维护者交接

代码集中在 `auth_sources.rs`、`auth_cleanup.rs`、`cleanup.rs`、`keychain.rs`、native account派生、`launch.rs` 和两个现有启动入口。新增测试直接位于各模块、`auth_cleanup_tests.rs`、`baseline_tests.rs` 与已有 canonical CLI journey。维护的用户／工程文档为 `cleanup.md/.en.md` 和 `credential-provenance-evidence.md/.en.md`。

后续更新外部 Claude版本时，先重新核对有限存储命名、账户选择与状态schema，再增加已确认的分支；未知选择应该保留无法核对，不应通过扫描所有Keychain账户或删除整个容器来获得表面完整率。查原任务、保留非目标资料、把本地结果与实际认证／撤销分开，这些现有产品选择应继续保留。

## 7. 固定提交终审与升级行为

本节重新审阅的最终源提交是 `e573ce228e15ee6cb03c2705baf70a8bab1a8c75`，对应 [draft PR #8](https://github.com/IndelibleVivi/lintel-cc/pull/8)。这是源码终审锚点；PR/CI 后续状态、发布资产和实际安装结果以总交付记录为准。终审复用了前述测试，没有再次运行完整 suite，也没有取得真实账号或 Gatekeeper 验收。

### 准确源码位置

下列行号均指向 `e573ce2`：

| 位置 | 对应保证 |
| --- | --- |
| `crates/core/src/keychain.rs:34`、`:43`、`:92` | 建立和重查有限认证目录描述；匹配相对选择拒绝、默认未设置、绝对原拼写和独立安全存储边界 |
| `crates/core/src/keychain.rs:102`、`:128` | 直接 Command 与 macOS shell 使用相同设置／移除规则，正确引用路径中的引号 |
| `crates/core/src/keychain.rs:155`、`:180` | 有限 service 补充与不兼容 account 选择保守拒绝 |
| `crates/core/src/keychain/macos.rs:197`、`:315`、`:356` | 当前 account 派生；原生 read 和只改 data 的 update；已冻结 identity 不按新 USER 重新猜测 |
| `crates/core/src/cleanup.rs:17`、`:220` | 生产 Linux writer 只有配置与独立安全存储目标均在别处才排除 |
| `crates/core/src/cleanup.rs:127`、`:158`、`:168` | 显式状态调用共享目录选择；报告的绝对路径必须解析回所选根 |
| `crates/core/src/auth_cleanup.rs:131`、`:217` | runtime status 预览才需要目录绑定；执行前核对程序与该绑定 |
| `crates/core/src/auth_cleanup.rs:331`、`:365`、`:381` | 原任务独立复查、runtime 限制单独呈现、最终本地重新读取 |
| `crates/core/src/launch.rs:466`、`:515`、`:565`、`:622` | 启动预览绑定、原记录优先查询、首次接收前与最终入口重查 |
| `crates/core/src/launch.rs:711`、`:731`、`:780`、`:831`、`:882`、`:934`、`:961` | resume 同样绑定，并在解密、复制和启动窗口重复重查 |
| `crates/core/src/lib.rs:111`、`:850`、`:1244`、`:1842` | Core direct TTY、普通启动描述、native Terminal 与公共窄 helper |
| `apps/runner/src/main.rs:119`、`:136` | 普通 CLI 和 TUI 的共享 `launch_command`，没有遗漏的无条件配置目录覆盖 |
| `apps/runner/src/submission.rs:273`、`apps/runner/src/supervisor.rs:259` | 后台 worker 从私有 stdin 取得原环境；systemd manager 的 `env_clear` 不会悄悄丢掉目录选择或把凭据放入 argv／D-Bus 属性 |

### 旧批准与已有工作的兼容矩阵

| 旧对象 | 升级后的行为 | 用户需要做什么 |
| --- | --- | --- |
| 未尝试的旧有限 launch/resume，缺少目录绑定 | `stale_auth_context`，在 durable intent、private resume copy 和客户端启动前拒绝 | 新建预览；不要把旧 hash 当成新批准 |
| 已有 launch/resume 原始记录 | 按原 ID 查询，先于新目录／程序检查；不会再次打开 Terminal 或要求重新执行原会话 | 回到原记录看最后已知结果 |
| 已有精确来源绑定、未请求 runtime status 的旧清理计划 | 继续按原来源／文件／Keychain／停写前提判断；不因缺少一个不会使用的 runtime 描述而整体失效 | 正常审阅；来源变化时才重新预览 |
| 请求 runtime status、但缺少新目录绑定的未尝试清理计划 | `stale_auth_context`，在接收和清理前拒绝 | 重新预览含状态检查的计划 |
| 已接受或完成的清理任务 | 原回执及 ID 保留；通用 execute 的原 hash／计划完整性校验仍存在，随后返回原任务，不重放 | 使用 job 查询；不要把错误 hash 的 execute 当查询 API |
| 原任务曾请求 runtime status，现有目录选择变化或旧绑定缺失 | 独立 `auth_verify` 仍读取本地来源；runtime 单列 `unavailable` / `stale_auth_context`，不修改历史回执 | 可继续核对本地结果；需要当前报告时另发明确的新检查 |
| 原任务没有逐来源认证 schema | 带 job 的新式认证复查返回 `legacy_auth_coverage`；原回执仍能查询 | 不带 job ID 检查当前来源；不据此重放旧清理 |

终审没有发现遗漏的自动客户端执行入口或新增的发行阻断缺陷。目录描述是**执行入口的选择绑定**，不是以 OS 文件描述符固定整个后续客户端世界：macOS `open -a Terminal` 为异步交接，回执准确使用 `launch_requested`。交接之后的路径／可执行文件／别名替换、接收 Terminal 的其他认证变量和外部客户端实际加载结果仍不能被这个回执证明；不要在 Release notes 中将它升级为“已认证”“已启动成功并验证来源”或隔离保证。

认证分项没有新增必须交给 Faye 的签名或账户决定。固定候选的 CI、临时 Keychain 和受控惰性客户端验收可继续由工程自动执行。只有另行选择真实联网登录／撤销验收时，才需要专用测试账户及其授权；不能为了填满证据表使用个人凭据。`stale_auth_context` 的下一步是重新预览，`auth_scope_unverified` 的下一步是核对原启动入口的目录选择，Keychain locked/denied 的下一步是针对相应系统入口复查；这些都不应引导用户重复执行已接受的清理任务。

### 首次当前提交 native CI 的真实结果与 harness 修正

对 `e573ce2` 的 [run 43520398332](https://github.com/IndelibleVivi/lintel-cc/actions/runs/43520398332)，native macOS job `132213004331` 的 canonical evidence 为 **18 passed / 1 failed**，不能以其他绿色 job 代替该结果。`auth-sources-core-test` 的 49 项通过，独立 `keychain-native-test` 也实际通过；失败是新 `journey-auth-sources` 的 PTY 输出捕获，原有精确清理 journey 已先通过。

失败发生在测试将已捕获 stdout 解析为 JSON 时，尚未进入预期 `stale_auth_context` 断言。测试 helper 在读取 master 前先关闭最后一个 slave FD，依赖了 Linux 在 hangup 后仍保留输出的行为。Apple 的 [XNU `ptsclose`](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/tty_dev.c#L325) 调用 `ttyclose`；后者会[清空读写队列](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/tty.c#L376)。因此修正测试为：子进程退出后保留父进程的 slave，以非阻塞方式读尽 master，再在 finally 关闭两端。预期错误码和“无 durable intent”断言保留，并增加缺失输出的明确诊断；没有改动产品实现来迁就测试。

修正后的 Linux 两段 journey 均通过。macOS 对修正提交的再次 canonical 运行仍必须单独确认，首次失败证据不被覆盖。完整 job log、canonical report 摘取、SHA256 与上述 Apple source blob 身份保存在交付 evidence 的 `current-ci/push-43520398332/auth-pty-diagnosis.json` 和同目录 `native-job-132213004331.log`。这次实际执行将临时 Keychain API 证据补齐到了该提交，但没有进行个人账号登录、正式安装或 Gatekeeper 验收。
