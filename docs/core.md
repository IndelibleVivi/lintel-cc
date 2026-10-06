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

配置写入在原子 rename 前，将已同步 staged 文件的对象身份、字节摘要、权限与冻结 plan hash 持久写入原 job 的 `settings_write`。若进程在发布后、回执更新前退出，`job`／`plan_restore` 在取得 operation lock 后核对当前文件与原始／待发布快照，并保存 `settings_recovery:{state,reason,message}`。只有匹配原 staged 对象的 `written` 才开放 `restorable`；原对象仍在是 `not_written`，其他对象／字节或计划、root 身份变化是 `ownership_unproven`。两者均保留现状，不自动重写、删除或恢复。中断查询仍为 `needs_reconciliation`，不是把原任务追认为完整成功。外部原子 rewrite 即便生成相同字段，也不算已证明本任务发布；旧任务没有此意图时不补造写入证据。

设置恢复只处理 Lintel 修改过的字段：当前值必须仍等于原任务写入值，否则拒绝。无关的后续编辑保留。当前文件写入路径提供快照复查与原子替换，但**不与不合作的外部编辑器构成原子 compare-and-swap**；最后复查与 rename 之间仍有竞争窗口。有限 [systemd 服务暂停](services.md) 可核对特定绑定 unit 与启动阻止项；它不覆盖全部 Claude、IDE、交互终端或其他 supervisor，因此不能宣称完整并发清场保障。

## 加密归档与新环境

工作类别为 `instructions`、`memory`、`sessions`。执行归档类计划需额外 `archive_passphrase`（至少 12 个字符），只能在此次请求内传递；不进入计划、journal 或支持资料。

`plan_archive` 只加密归档选中的工作内容，`outcome` 为 `archive_only`：不新建环境、不改 settings、不碰凭据，原文件保持不变。可选绝对 `output_path` 会在预览时冻结路径与父目录的 device/inode，接受执行和实际发布前都核对同一目录对象。目录替换返回 `stale_plan`；缺少身份的旧显式输出计划须重新预览，已完成原任务继续查询、不重跑。该路径必须不存在，core 不覆盖已有文件，也不替调用者创建父目录；未给出时归档留在私有 state，receipt 里的 `archive_path` 指回它。`plan_preserve` 在此之上建立新根并迁入所选内容，`outcome` 为 `preserve`：receipt 以 `outcome:"preserved"` 完成，`coverage` 声明 `old_login`/`old_root` 保留、`service_binding:"unchanged"`，`next_steps` 列出在新环境采用保护方案、正常登录与运行验证，并按用途说明待用资料与显式启用的个人指令，原 root 的 service 绑定不改。旧环境的保留步骤以 `preserved`（保留）呈现，不是未完成项。`plan_show` 读回已冻结计划供 CLI 展示，返回与预览相同的字段并剥离私有原始快照、根身份与 `extra`；迁入计划另投影公开清单与批准中的完整目标：新增 `planned_target` 包含准确新 root／ID、文件完整落点、用途与启用选择，执行使用这份冻结分配而不重新生成第二 root；旧 `import_manifest` 的相对路径保持兼容。见下文；口令从不写入计划，因此不会泄出。

`plan_reset` 仍只接受 `recipe: "rebuild"`，语义不变：仅归档后建立新根并迁入，旧 root/登录保留，receipt 维持 `partially_completed`（旧登录/客户端清理步骤未完成）；历史 receipt 不复绿。

当前可识别：根 `CLAUDE.md`、`projects/**/memory/*.md`、`projects/**/*.jsonl`，以及此前迁入的 `lintel-imports/CLAUDE.md`、直接待用区中的 `lintel-N-CLAUDE.md` 和 `lintel-imports/projects/**`（保留原类别，不会包裹成 `lintel-imports/lintel-imports`）。枚举预算为单文件 8 MiB、总量 32 MiB、10,000 个文件、50,000 个 entries、30 秒；超限拒绝计划，不将截断扫描称为完整扫描。路径和文件身份在执行时重新核验；加密归档写入前检查冻结目标父目录所在存储的可用空间，显式导出不以 Lintel state 所在盘代替目标盘。

工作包是标准 age 口令加密的 JSON（`schema: "lintel.work/1"`，携带 `generator`、`created_at` 与逐文件 `path`/`category`/`digest`），生成后重新读取并核对归档字节。新包记录 `generator:"Lintel"`；inspect 从包内读取此自声明字段，旧包或无可识别字段的兼容包返回 `generator:null`，外部包不被直接归因为 Lintel，元数据也不是来源认证。package 自包含，因此可显式带去没有原 job/state 的另一个 Lintel 安装：`archive_inspect`/`archive_read`/`plan_import` 只接受 `job_id` 或绝对 `archive_path` 之一（两者同给或缺省都拒绝）。job 来源会复核任务记录的 `archive_path` 和已记录的 `archive_digest`，原路径被另一个有效包替换也返回 `stale_archive`；旧回执缺少 digest 时保留兼容读取。外部 `archive_path` 来源独立按包内容判断，不归属某个原 job。读完限额、错误口令、损坏包、绝对/父目录跳转/重复路径、未支持类别与文件摘要校验与自身包一致，安全保护不因来源不同而弱化。`plan_import` 冻结来源与 encrypted digest，执行时重读来源，`stale_archive` 会拒绝；同名冲突、未选类别与原文件均受保护，`create_new` 防止覆盖。公开 `import_manifest` 来自冻结清单：`package:{format,generator,sha256}` 是包内格式／自声明生成器与准确加密包摘要；`files:[{source,destination,category,size,sha256}]` 是来源相对路径、相对目标 root 的最终位置、类别、字节数与文件摘要。预览、`plan_show` 和 GUI 使用相同投影，包含 `lintel-N-` 重名分配，不包含正文、口令或其余内部 `extra`。状态备份 `lintel.state/1` 与工作包分开，不支持自动导入混合状态。

工作包与状态备份仅在归档完成后才创建新根。App 默认将指令、会话和记忆准备到 `lintel-imports`；只有明确选择个人指令 activation 才将根 `CLAUDE.md` 放入新 root 的活跃指令位置，旧 raw caller 保留历史默认。不恢复 settings、hooks、MCP、插件或凭据。再次归档或重建时，此前迁入 `lintel-imports` 的参考指令、记忆与会话按保留的类别重新计入 manifest，直接待用区中已有 `lintel-N-CLAUDE.md` 也会保全；参考不因重新归档自动激活。活跃 `projects` 与已有待用区映射到同名文件或文件／父目录冲突时，共同迁入路径分配先保留整批原名称与父目录，再为冲突的文件项加 `lintel-N-` 文件名前缀；两份内容都保留，后缀和类别不变，不覆盖、不重复嵌套。独立包迁入使用同一分配，预览冻结实际目标，执行复核后才写入；目标已有文件仍拒绝。未选定 `output_path` 的独立归档留在 Lintel 私有 state，旧包继续受支持。

## 有限关联组件与容量预检

`inspect_components` 由 `components.rs` 投影 `lintel.components/1`：准确 environment/root、可选 target-host project cwd、观察时间、有限来源／范围／下一入口、原任务记录。CLI 只读取现有静态安装元数据；配置来源只返回路径、存在情况及 hooks/MCP 声明布尔值，不返回命令、参数、env 任意字段或正文。只检查已选择 cwd 的有限文件及 core 已认识的 managed 文件位置；父级、其他组织下发、实际加载与 OS 强约束仍未覆盖。认证只检查已知文件／共享 profile 的元数据，不读取 credentials，也不执行 auth probe。

原记录扫描最多读取 10,000 个目录项，不调用 `job` 的 reconciliation；仅投影同一 environment、文件名与合法原 ID 一致的记录，最多 50 份，读取不完整与展示截断分开。最多刷新 8 个原记录所指的确切 systemd unit，当前 probe 失败标未知并保留原 ID。原 task_result.coverage 是历史证据。Browser、IDE／Desktop 与未支持 supervisor 保持各自范围，不以 root 已处理推断都已处理。

`work_preflight` 与工作统计共用 metadata scanner，选择类别、单文件／合计／文件数、目录预算与 symlink 规则沿用实际归档范围；阻塞对象最多列 200 项，截断不改变拒绝结论；扫描预算保持 50,000 目录项／30 秒，首个未覆盖来源保留相对路径。它没有摘要、冻结计划或副作用批准；内容准入仍由 manifest 和执行复查拥有。既有包格式与 8 MiB／32 MiB／10,000 文件 limits 未改变。

## 有限清理与认证

`cleanup_inspect` 只读取精确状态文件元数据和进程名称；`auth_probe` 是独立显式动作，调用已登记程序的 `auth status`。必须返回可核对的 `configDirectory` 与认证类别，否则拒绝推断；只新增 `identity_observed` 布尔值，不返回邮箱或原始认证输出。真实 Claude / Keychain 尚未验收，当前回归测试用合成 fake CLI。

官方注销的已登录 claude.ai 预览必须观察到非空 `email`；缺失时返回 `auth_identity_unverified`，独立 auth probe 与仅本地清理仍可用。私有 `extra.auth_binding` 保存 `lintel.auth-status/1`、本计划随机 nonce 和完整解析 JSON／登录结果的摘要，不保存原始主体或未知字段。执行前及保全后的 `check_cleanup` 使用相同 nonce 核对；观察字段变化返回 `stale_auth`。凭据文件身份／内容变化或 fallback 新出现仍由原文件快照返回 `stale_plan`。完整状态中的其他字段变化也需要新预览。public_plan 删除整个 extra，回执和 Agent 元数据不携带此绑定。未尝试的旧官方注销计划缺少绑定须新预览；既有接受任务优先查询原 ID。CLI 报告不是 Keychain 凭据世代或来源认证，真实认证、同账号 Keychain-only token 更新和外部并发边界仍未验收。

`plan_cleanup` 接受 `repair_login`、`reset_client`、`retire`，要求 `writers_confirmed_stopped: true`，可选 `official_logout`。默认根的混合状态是 home 下 `.claude.json`；专用根使用其 `.claude.json`。修复登录只处理 `.credentials.json`；其余两类还处理预览中的混合状态文件。工作、settings、hooks、MCP 与插件文件不会被通配删除。

reset_client 的冻结 `actions` 与 receipt 有序 `steps` 采用同一 canonical 顺序：复查写入者 → 状态备份 + 工作加密归档 → 新根创建与迁入 → 再次复查写入者与冻结范围 → 官方注销（如选）→ 精确移除旧文件。任何破坏性动作都不早于保全；reset_client 重读刚生成的工作包时核对本任务的 archive_digest，包被替换返回 stale_archive，在新根创建／迁入／注销／删除前停止。官方注销失败时保留已生成的归档、新根与 create/migrate 完成步骤，后续旧文件移除与 retire 不执行，同 ID 查询原任务不重跑。retire 停用启动入口；`reactivate_environment` 只恢复登记状态，不恢复凭据。

执行被持久接受后失败时，receipt 额外写入结构化 `error: {code,message,phase,recovery}`：`phase` 记录失败发生的真实 receipt 状态（如 `verifying`），`recovery` 指向以同一 ID 查询原任务，机器不必猜中文 warnings；已完成的步骤与产物（归档、新根）保留，不可逆步骤不重放。

已识别 Claude 进程仍运行时拒绝；不会全局杀进程，也不能识别所有 wrapper。Linux 的明确绑定 systemd unit 需通过独立暂停计划，清理前再读回 owned hold、inactive、MainPID 与空 cgroup；手工 `stopped` 声明不能代替该证据。非 systemd 与其他写入者仍须单独处理，详见 [服务指南](services.md)。官方注销仅接受可验证的本地登录来源；存在共享 Anthropic profile 或相应环境变量时，先于较慢的 service／进程检查拒绝。该范围在预览、执行检查及实际注销前复查。实际认证探测前还核对计划冻结的 executable、当前 inventory 与实时发现，重新 discover 不能让旧批准改指另一个程序。不选官方注销时同样在归档／迁入后、删除前复查写入者。仅调用官方 `auth logout`，不猜测 Keychain service 名；服务端 token 撤销始终另列 `unverified`。

本地删除先将对象原子移入同目录的私有隔离目录，再核对被冻结的对象，避免按原路径误删随后替换的新文件。冲突时不覆盖新文件；无法回到原位置的对象保留在回执所列隔离目录，必须核对。它不等于对持有开放文件描述符的外部写入者建立 OS 级 CAS。官方命令导致额外状态变化或状态重新出现时，会停止后续动作。未选择官方注销的回执为 `partially_completed`；精确文件已处理不代表 Keychain、Desktop、IDE、浏览器或目录外认证已清空。

TUI 通过 `lintel tui` 提供上述配方、认证检查、归档阅读/迁入与重新启用。口令只从关闭 echo 的交互终端读取，自动化应使用 JSON stdin；不会写入 plan、journal 或支持资料。

## 启动与支持资料

`crates/core/src/session.rs` 拥有有界 `session_read`：按工作包 source＋文件摘要读取一页，返回字节继续位置、结构化记录及 raw text。未知／损坏记录不从原件删除；thinking／signature 是不透明块。正文与口令不进普通 journal。旧 `archive_read` 保留其既有兼容行为。

`crates/core/src/launch.rs` 拥有 `plan_launch`／`launch_request`、独立 `plan_resume`／`resume_request` 以及只读 `launch_query`／`launches`。配置 root 决定状态，项目 cwd 决定工作目录。预览冻结 target／程序／静态版本；首次执行复查，Terminal／PTY 前持久 intent。重复与中断按原 ID 核对，不能重放启动。resume 准备私有字节副本，不把 archive 原件交给客户端写；有限支持政策、真实认证／实际恢复限制与写入范围见原计划和 [操作指南](operator-guide.md)。

启动目录身份使用 device／inode／owner 加创建时间，避免 Linux 删除重建时立即复用 inode 漏过复查；普通项目内容变化不冻结。创建时间不可用时明确拒绝新预览；旧未启动计划缺少该证据须重新预览，原记录优先查询语义不变。这是写前身份检查，不是对外部 writer 的 OS 级 CAS。

macOS 显式启动动作生成私有 `.command` 并请求 Terminal 打开准确配置根，使用 `CLAUDE_CONFIG_DIR`；有 loopback 通道时，只给新启动传入大小写 HTTP(S) proxy 变量。不会关闭已有会话或消除 `NO_PROXY` 的分流。`launch_requested` 仅表示启动请求已送达，实际 Claude 使用与网络效果未验证。

独立终端用 `lintel launch <environment-id>`，由 CLI exec 目标程序，stdin/stdout 都必须是 TTY，且只接受一个环境 ID；不在隐藏管道内启动交互 agent，不接受 prompt 参数。TUI 的 `o 打开 Claude` 使用同一启动路径，退出 Claude 后返回菜单。

`export_support` 使用白名单，只含平台、版本、计数与能力信息；不包含目录、环境名、配置正文、令牌、会话或目的地主机。它不自动上传。

验证：`cargo test -p lintel-core`、`python3 tests/cli_journey.py`、`python3 tests/policy_journey.py`、`python3 tests/work_preservation_journey.py` 与 `python3 tests/portable_work_journey.py` 均只修改新建的合成目录。core 回归覆盖错误输入、陈旧计划与旧规则、链接拒绝、字段恢复冲突、重复执行、中断记录、并发查询、加密迁移，以及本次新增的 archive-only 不新建环境、output_path 冻结/拒绝覆盖、跨独立 home/state 的包读取与迁入、stale/错误口令/损坏包拒绝、preserve 完成而旧 reset 仍 partial、canonical reset 顺序、logout 假失败保留产物与结构化 error 跨 Engine 持久、`plan_show` 不泄露秘密；CLI journey 跨实际进程验证配置往返，policy journey 进一步验证静态版本身份、Remote Control 条件、七项 inspect、自定义选择/空改动/回执冻结/字段恢复和外部编辑拒绝，portable work journey 跨独立进程验证包携带、只读清单、显式路径迁入与 accepted 后失败码持久。版本探测使用 inert executable，不运行 Claude。


独立加密包和迁入的新文件使用同目录临时文件写入/同步后，以系统原子不覆盖 rename 发布（不会替换已出现的文件）；临时名在同一操作中消失，不产生发布后的双链接清理窗口，再读回核验。import 在预览和正文写入前核对整批实际目标的祖先：path guard 拒绝非目录／链接，批准 root 内的现有目录必须属于当前执行器 UID，实际权限须允许访问和在最近现有父目录写入。已知错误分别为 `path_unreadable`、`wrong_owner`、`migration_destination_unwritable`，在整批正文写入前拒绝；Lintel 不更改已有权限。共同迁入路径只将新建父目录设为 `0700`，已有目标根和父目录权限保留，文件为 `0600`。macOS 使用 renameatx_np(RENAME_EXCL)，Linux 使用 renameat2(RENAME_NOREPLACE)；目标文件系统或内核不支持时返回 atomic_publication_unsupported，保留原数据与原任务，不退回 hard link 或覆盖式写入。settings 的按字段恢复继续使用既有冻结快照与写前复查；这不把外部编辑器纳入原子 CAS。

接受后 `error` 还包含可用的 `step_id` 和 `uncertain_side_effects`：执行中步骤、写后验证的未知效果须从原回执/产物核对；新环境创建在 mkdir 前分配并持久保存准确 new_root/new_environment_id 与执行中步骤；登记失败或中断后原任务仍能定位目录，意图不证明登记成功。整批迁入在开始写入前先持久记录执行中步骤，迁入失败时保留新 root 与已发布文件，并标明仍在执行的 create/migrate 步骤和可能的部分写入；已完成状态备份与工作归档分开保留，不被后续 archive step 覆盖。

归档发布前先持久记录执行中的 archive 步骤、`archive_path` 与 `archive_intent_digest`，后者绑定待发布的准确密文；写后读回成功才记录完成和 `archive_digest`。中断回执中的路径不表示包已完成：查询原任务核对产物，包存在时可用原 job 解锁，尚不存在时返回具体读错误。job 读取先采用完成 digest，没有完成 digest 时核对 intent digest；旧回执两者都没有才沿用兼容行为。

清理产生的混合客户端状态备份同样在发布前保存 `state_archive_path` 和执行中步骤，写入并读回成功才标记完成。路径仅表示原任务的准确目标；中断时先查询原任务并核对产物，不自动重发、覆盖或迁入状态备份。

批准后的迁入先在实际目标文件系统的私有临时目录里，以空文件验证整批路径。preserve/reset 使用新根所在的 environments 目录，import 使用已批准的目标 root；这一步不写归档正文，正常返回清理自身创建的空文件和目录，不递归删除额外内容。大小写折叠或 Unicode 规范化等仍使路径等价时，返回 migration_path_conflict，在新环境创建或任何正文迁入前整批拒绝。包仍可检查与阅读；整理源内容重新归档，或选择可区分这些路径的文件系统。此检查只在已批准执行中运行，预览保持无目标写入。

路径 preflight 在创建临时目录前持久记录 `migration_probe:{path,status}` 和执行中的 `migration_preflight` 步骤。正常清理确认后 status 为 `removed`；无法确认清理则为 `retained` 并停止正文迁入。worker 中断会保留 `executing` 与原路径；按原 job 查询核对目录，查询不删除或重跑它。App 回执显示尚需核对的检查目录。只核对原任务创建的空文件范围，额外内容保留，不把临时目录当成新配置环境。

原子不覆盖 rename 的平台合同见 [Linux rename manual](https://www.man7.org/linux/man-pages/man2/rename.2.html) 与 [Apple rename manual source](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/man/man2/rename.2)。

新增的创建新 root 计划在批准前冻结完整落点。尚未接受且缺少该字段的旧创建计划返回 `stale_plan`，需重新预览；已经接受的历史任务仍先读原 ID 的回执，不重新创建目录。旧 portable import 计划省略 activation 时保留根 CLAUDE.md 的原目的地，旧协议与包不改写。
