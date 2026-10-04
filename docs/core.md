# Core 与 CLI

`crates/core` 是本地环境清单、计划、文件操作、任务 journal 与字段恢复的唯一实现。GUI 和 `apps/runner` 通过同一个 `handle_request` 调用它。完整目标仍由 [SPEC](SPEC.md) 定义，本页只描述当前实现。

## 读取、计划与批准

`lintel request` 从 stdin 读取一个不超过 1 MiB 的 JSON object，返回一个 JSON envelope；错误时 CLI 以非零状态退出。其余命令见 `lintel --help`。先取得环境 ID，再生成计划，最后提交该计划的准确 hash：

```json
{"command":"plan_policy","environment_id":"00000000-0000-4000-8000-000000000001","preset":"reduce","keep_remote_control":false}
```

```json
{"command":"execute","plan_id":"00000000-0000-4000-8000-000000000002","approval":"exact-hash-returned-by-plan"}
```

示例 ID/hash 必须换成执行器返回值；不要从示例执行真实 mutation。`plan_policy` 冻结规则版本、配置文件快照、根目录身份与具体字段。预览后文件改变、目录替换、错误 approval 会拒绝执行。core 使用严格 JSON 解析，重复键和损坏 JSON 不会被静默标准化后覆盖。

计划冻结的是 settings.json 的**原始字节快照**与根身份，前置条件随动作实际读写范围生成：不修改 settings 的计划（重建、清理、迁入）在 settings 损坏或含重复键时仍可预览与执行，原字节保持不变；策略修改与字段恢复仍要求 settings 可完整解析。策略规则为 `claude-privacy-v3-2026-10-03`：`DISABLE_TELEMETRY`、`DISABLE_ERROR_REPORTING`、`CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` 按非空字符串生效，`"0"`、`"false"` 也会关闭；反馈、调查、`DO_NOT_TRACK`、`DISABLE_GROWTHBOOK` 按标准 boolean 字符串解析，`0/false/no/off` 不生效。已识别 env 键只接受字符串值；未核验的 boolean 写法标为不确定。规则、作用范围、产品证据和功能影响由同一个 core 模型返回，GUI 不自行推断。

程序先按 PATH 定位；非交互 PATH 未包含用户原生安装时，静态检查当前用户的 `~/.local/bin/claude`。不扫描其他用户或执行 shell 初始化。版本只从定位到的原生 `claude/versions/<version>` 路径或 Claude Code npm `package.json` 静态识别；不会运行 Claude 或查询账号。不认识的安装方式、版本家族或 prerelease 保留未知。`discover`、`inspect` 和计划都会重新识别；执行前程序路径/版本证据变化会要求重新预览，不把最新官方文档当作本机版本。

环境概览（`inspect`）返回下述七个已识别 env 键的当前字符串值、settings 路径、状态和 `effect_timing: next_launch`；每项 `runtime_verified: false`，配置状态不构成运行时效果证明。工作内容统计只读文件元数据，不读取内容、不计算 digest、不受归档准入上限影响；扫描超出预算或部分条目不可读时按类别标记 `complete: false`，而不是让无关的设置检查整体失败。

| 方案 | 当前精确修改 |
| --- | --- |
| `preserve` | 设置 `DISABLE_ERROR_REPORTING=1`、`CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY=1`；保留指标与主动反馈的当前值 |
| `reduce` | 上述两项加 `DISABLE_TELEMETRY=1`、`DISABLE_FEEDBACK_COMMAND=1` |
| `custom` | `custom_settings` 按字段选择 `keep` / `disable` / `remove`；缺少字段视为 `keep` |
| `keep_remote_control=true` | `preserve` / `reduce` 根据版本与 Trusted Devices 条件判断是否可同时关闭 telemetry；`custom` 只记录意图并评估准确 diff，不覆盖明确选择 |
| `release_settings` | `preserve` / `reduce` 用户选中的冲突字段进入明确的 `after:null` 删除差异；批准准确计划后才移除，可按字段恢复 |

自定义方案的请求示例：

```json
{"command":"plan_policy","environment_id":"00000000-0000-4000-8000-000000000001","preset":"custom","custom_settings":{"DISABLE_ERROR_REPORTING":"disable","DO_NOT_TRACK":"keep","DISABLE_GROWTHBOOK":"remove"},"keep_remote_control":true,"trusted_devices":"not_required"}
```

`custom_settings` 只接受 `DISABLE_TELEMETRY`、`DISABLE_ERROR_REPORTING`、`DISABLE_FEEDBACK_COMMAND`、`CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY`、`DO_NOT_TRACK`、`DISABLE_GROWTHBOOK`、`CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` 七个键。`keep` 保留当前字符串原值；`disable` 在现值按该键语义已关闭时保留原值，否则写字符串 `"1"`；`remove` 明确删除该 user settings 的键，不保证功能开启，缺失键不产生改动。缺省 `custom_settings` 等于空对象。空 diff 执行保留文件原始字节、权限与文件是否存在，仍更新 drift baseline，receipt 记录完成且 `restorable: false`。`custom_settings` 不可用于其他 preset，`null`、数组、未知键、嵌套对象和未知 action 均拒绝；`custom` 不接受非空 `release_settings`，请用 `remove` 表达删除。

修改位置是已登记 root 的 `settings.json` 中的 `env`。`preserve` / `reduce` 默认不新增总禁用开关；`custom` 可明确选择七个字段中的任意一项。所有改动均进入同一字段 diff、准确批准、读回与恢复路径。不通配清除 OTel，不修改代理、AWS/Google 设置、JSON permissions、hooks 或 MCP；写入沿用私有文件模式，保留已有读权限并移除 group/other 写权限。配置读回不代表既有进程生效；界面标记下一次新启动。项目、组织服务端策略、IDE、服务入口与实际认证来源尚未完整探测。检测到已知本地 managed-settings 文件时，拒绝用低层设置覆盖。

## Remote Control 版本与既有配置

| 版本 / 条件 | telemetry / DO_NOT_TRACK 已生效时 |
| --- | --- |
| Claude Code 2.1.283 之前（含 2.1.154–2.1.282） | 配置与 Remote Control 冲突；2.1.154 之前错误提示不同 |
| 2.1.283+，声明组织不要求 Trusted Devices | 可保留 Remote Control 的配置条件；无需重开 telemetry |
| 2.1.283+，声明组织要求 Trusted Devices | 需要显式解除相关开关 |
| 版本家族或组织条件未知 | `conditional`，不能承诺功能可用 |

`DISABLE_GROWTHBOOK` 与非必要流量总开关有各自取值语义，生效时独立阻断 Remote Control。`trusted_devices` 只接受 `unknown`（默认）、`required`、`not_required`，属于用户声明而非组织探测。规则依据：[官方环境变量](https://code.claude.com/docs/en/env-vars)、[Remote Control 条件](https://code.claude.com/docs/en/remote-control)，核验日期 2026-10-03。

先减少外发再选择保留功能时，core 会计算当前值并列出冲突；在 `preserve` / `reduce` 中，用户通过 `release_settings` 明确选择解除 `DISABLE_TELEMETRY`、`DO_NOT_TRACK`、`DISABLE_GROWTHBOOK` 或 `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC`。在 `custom` 中用字段 `remove` 明确删除。已有值不按写入者擅自删除，外部后续编辑仍使旧计划拒绝执行。删除只作用于该 user settings；shell/项目/组织来源由其设置者掌握。删除不会 unset 既有进程，需重新启动。

`inspect.policy`、`Plan.policy`、`Receipt.policy` 保留同一版本证据和规则；`supported_presets` 明确列出可用方案。计划和回执保留 `preset`、custom 的准确 `custom_settings` 选择以及该准确 diff 的配置推演；规则版本改变会使未执行的旧策略计划要求重新预览。`blocked`、`conditional`、`configuration_compatible` 均不是实际运行证明；账号、订阅、组织启用、设备 enrollment、API endpoint 和其他配置来源未验收。旧 runner 未返回版本化结果或未声明支持 custom 时，桌面明确提示更新执行器。

## 环境与存储

默认 state：macOS `~/Library/Application Support/Lintel`，Linux `~/.local/share/lintel`。开发测试可显式设置 `LINTEL_TEST_HOME` 与 `LINTEL_STATE_DIR`；测试 home 必须是真实存在的独立目录。`discover` 可在清单中登记已存在的默认 `.claude` 根，`register` 接受明确的绝对配置根，`create_environment` 在 Lintel 自有区域生成新根。

目录权限为 0700，记录为 0600。状态采用临时文件、fsync、rename 与目录 fsync。目标检查拒绝符号链接、根/home 目录、异常硬链接与不属于当前用户的目标。发现不会运行 Claude、auth helpers、shell rc、hooks 或 MCP。

## 并发与恢复边界

一个本地 core operation lock 串行化 Lintel 的 mutation。锁文件位于 state 目录下，只互斥**同一 state 目录**的 core 进程；两个指向同一配置根、却使用不同 state 目录的实例不会被这把锁串行化，此时仍靠计划快照与执行前复查兜底，不要把多 state 部署当作已互斥。执行期间 `jobs` / `job` 可以读取已原子落盘的 journal；锁被实际 writer 持有时，不将运行中任务标成中断。没有 writer 时查询非终态任务会转为 `needs_reconciliation`。receipt ID 等于 plan ID，重复 execute 返回原记录，不重做动作。

任务先持久接收，再记录执行、验证、结果。`lintel request` 在前台执行；`lintel submit` 仅接受 execute，通过 stdin 交给 worker，在 accepted journal 已落盘后返回 ACK。Linux PID 1 systemd／当前 euid 0 使用 system transient service；非 root 必须已有 `Linger=yes` 和可用的当前 UID user bus 才使用 user transient service。原调用环境经同机私有 stdin pipe 继承，工作目录保持一致，认证来源／proxy 复查不能因 manager 切换丢失；环境值不写入 unit 配置、命令参数、回执或磁盘。worker 在接受前核验实际 cgroup 属于所选 unit，`Restart=no`，没有自动重发或 manager 启动失败后的第二次 fallback。其他主机保留 `setsid` 并明确记录续跑限制；这是 macOS、无 systemd 和未满足 user manager 条件的现行兼容路径。`execution` 的 mode、manager、unit、continuation、limitation 与 `reboot_survival: false` 来自 runner 内部，在 ACK 前持久写入，调用者请求里的同名字段不能伪造。旧回执没有该字段时保持原事实，不补造托管结论。断线后查询原 ID；恢复另行预览和批准。真实 PAM logout／重启证据与生产 VPS 限制见 [当前状态](current-state.md)。

设置恢复只处理 Lintel 修改过的字段：当前值必须仍等于原任务写入值，否则拒绝。无关的后续编辑保留。当前文件写入路径提供快照复查与原子替换，但**不与不合作的外部编辑器构成原子 compare-and-swap**；最后复查与 rename 之间仍有竞争窗口。有限 [systemd 服务暂停](services.md) 可核对特定绑定 unit 与启动阻止项；它不覆盖全部 Claude、IDE、交互终端或其他 supervisor，因此不能宣称完整并发清场保障。

## 加密归档与新环境

`plan_reset` 当前只接受 `recipe: "rebuild"` 与工作类别 `instructions`、`memory`、`sessions`。执行需额外 `archive_passphrase`（至少 12 个字符），只能在此次请求内传递；不进入计划、journal 或支持资料。

当前可识别：根 `CLAUDE.md`、`projects/**/memory/*.md`、`projects/**/*.jsonl`，以及此前迁入的 `lintel-imports/projects/**`（保留原类别，不会包裹成 `lintel-imports/lintel-imports`）。枚举预算为单文件 8 MiB、总量 32 MiB、10,000 个文件、50,000 个 entries、30 秒；超限拒绝计划，不将截断扫描称为完整扫描。路径和文件身份在执行时重新核验，并检查空间。

工作包是标准 age 口令加密的 JSON，生成后重新读取并核对归档字节。仅在归档完成后创建新环境。`CLAUDE.md` 迁入新根，会话/记忆资料进入 `lintel-imports`；不恢复 settings、hooks、MCP、插件或凭据。再次归档或重建时，此前迁入 `lintel-imports` 的记忆与会话按原逻辑路径重新计入 manifest，不会遗漏。旧根不动，因此 receipt 明确是 `partially_completed`，旧登录/客户端清理步骤未完成。桌面“工作归档”和 TUI 可通过 `archive_inspect` 解锁文件清单、`archive_read` 阅读最多 1 MiB 文本，再用 `plan_import` 选择类别与目标，单独批准迁入。core 验证格式、路径、重复路径、类别、容量、文件摘要，并以 `create_new` 防止覆盖同名文件。状态备份 `lintel.state/1` 与工作包分开，不支持自动导入混合状态。

## 有限清理与认证

`cleanup_inspect` 只读取精确状态文件元数据和进程名称；`auth_probe` 是独立显式动作，调用已登记程序的 `auth status`。必须返回可核对的 `configDirectory` 与认证类别，否则拒绝推断；不返回邮箱或原始认证输出。真实 Claude / Keychain 尚未验收，当前回归测试用合成 fake CLI。

`plan_cleanup` 接受 `repair_login`、`reset_client`、`retire`，要求 `writers_confirmed_stopped: true`，可选 `official_logout`。默认根的混合状态是 home 下 `.claude.json`；专用根使用其 `.claude.json`。修复登录只处理 `.credentials.json`；其余两类还处理预览中的混合状态文件。工作、settings、hooks、MCP 与插件文件不会被通配删除。reset_client 先加密归档并建立新环境，retire 停用启动入口；`reactivate_environment` 只恢复登记状态，不恢复凭据。

已识别 Claude 进程仍运行时拒绝；不会全局杀进程，也不能识别所有 wrapper。Linux 的明确绑定 systemd unit 需通过独立暂停计划，清理前再读回 owned hold、inactive、MainPID 与空 cgroup；手工 `stopped` 声明不能代替该证据。非 systemd 与其他写入者仍须单独处理，详见 [服务指南](services.md)。官方注销仅接受可验证的本地登录来源；存在共享 Anthropic profile 或相应环境变量时，先于较慢的 service／进程检查拒绝。该范围在预览、执行检查及实际注销前复查。仅调用官方 `auth logout`，不猜测 Keychain service 名；服务端 token 撤销始终另列 `unverified`。

本地删除先将对象原子移入同目录的私有隔离目录，再核对被冻结的对象，避免按原路径误删随后替换的新文件。冲突时不覆盖新文件；无法回到原位置的对象保留在回执所列隔离目录，必须核对。它不等于对持有开放文件描述符的外部写入者建立 OS 级 CAS。官方命令导致额外状态变化或状态重新出现时，会停止后续动作。未选择官方注销的回执为 `partially_completed`；精确文件已处理不代表 Keychain、Desktop、IDE、浏览器或目录外认证已清空。

TUI 通过 `lintel tui` 提供上述配方、认证检查、归档阅读/迁入与重新启用。口令只从关闭 echo 的交互终端读取，自动化应使用 JSON stdin；不会写入 plan、journal 或支持资料。

## 启动与支持资料

macOS 显式启动动作生成私有 `.command` 并请求 Terminal 打开准确配置根，使用 `CLAUDE_CONFIG_DIR`；有 loopback 通道时，只给新启动传入大小写 HTTP(S) proxy 变量。不会关闭已有会话或消除 `NO_PROXY` 的分流。`launch_requested` 仅表示启动请求已送达，实际 Claude 使用与网络效果未验证。

独立终端用 `lintel launch <environment-id>`，由 CLI exec 目标程序，stdin/stdout 都必须是 TTY，且只接受一个环境 ID；不在隐藏管道内启动交互 agent，不接受 prompt 参数。TUI 的 `o 打开 Claude` 使用同一启动路径，退出 Claude 后返回菜单。

`export_support` 使用白名单，只含平台、版本、计数与能力信息；不包含目录、环境名、配置正文、令牌、会话或目的地主机。它不自动上传。

验证：`cargo test -p lintel-core`、`python3 tests/cli_journey.py` 与 `python3 tests/policy_journey.py` 均只修改新建的合成目录。core 回归覆盖错误输入、陈旧计划与旧规则、链接拒绝、字段恢复冲突、重复执行、中断记录、并发查询与加密迁移；CLI journey 跨实际进程验证配置往返，policy journey 进一步验证静态版本身份、Remote Control 条件、七项 inspect、自定义选择/空改动/回执冻结/字段恢复和外部编辑拒绝。版本探测使用 inert executable，不运行 Claude。
