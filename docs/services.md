# 精确 systemd 服务暂停与恢复

Lintel 的 Linux runner 可以预览并暂停明确绑定到一个登记配置 root 的单一 systemd service，并在独立批准后恢复原启动状态。GUI 的远程服务入口、CLI JSON 与 TUI 都调用 `crates/core/src/service.rs`。暂停服务不注销账号、不清理凭据，也不代替随后单独预览的清理操作。

## 可支持的目标

请求必须明确提供环境 ID、`manager: "user" | "system"` 与完整 service unit 名称，例如 `example.service`。`user` 只操作当前 UID 的 user manager；`system` 变更要求 runner 当前已以 root 身份执行。Lintel 不执行 sudo，不调整 linger、PAM、login policy，也不停止 manager、全局 supervisor 或 SSH。

支持直接 loaded 的持久 simple、exec 或 notify service，统一 cgroup v2、`KillMode=control-group`、无 cgroup delegation。unit 必须直接且唯一配置 `CLAUDE_CONFIG_DIR=<登记 root>`，不使用 EnvironmentFile、PassEnvironment 或 UnsetEnvironment 提供未知覆盖。active service 的 MainPID 和 cgroup 内每个进程必须属于当前 runner 用户，且 `/proc` 环境证据与该 root 一致。root 也必须属于当前用户。服务配置文件需是 root 或当前用户拥有、非共享可写且无路径 symlink 的普通文件。

alias、glob、裸 service 前缀、未实例化 template、transient/generated/masked unit、自定义 unit 搜索目录、转换中或 failed 状态不受支持。带停止传播、其他 unit 对它的 Requires/BindsTo/PartOf 关系、自定义 stop/success/failure 动作、host action，以及可能启动或停止相邻 unit 的依赖关系会被明确拒绝。标准的 system/user slice 与基础 target 依赖可以保留。带 `Restart=always` 或精确 timer/socket 触发来源的独立 service 可以暂停；这些触发 unit 自身不会被停止。

CLI JSON、计划、回执与清理检查只保留 root 绑定及必要 service 状态，不保存或导出整套进程环境、API key 或 unit 文件原文。计划冻结 unit 文件与 drop-in 的原对象、字节摘要、manager 配置和启动状态，以判断预览后的外部改动。

适配器通过带准确签名的 D-Bus 查询核验 `EnvironmentFiles`、`ExecStop` 与 `ExecStopPost` 数组，只有明确返回空数组才通过；属性缺失、未知格式或非空值仍拒绝。systemd 255 的 `systemctl show --all` 会省略这类空数组的整行，因此缺行不能作为空值证据。`busctl get-property --json=short` 的 `data` 是属性值本身；root 环境、drop-in、condition 与 unit 搜索目录均按这个结构读取。

已查询的 13 个 unit 关系属性具有无序成员语义：`RequiredBy`、`BoundBy`、`ConsistsOf`、`PropagatesStopTo`、`TriggeredBy`、`OnFailure`、`OnSuccess`、`Requires`、`Requisite`、`BindsTo`、`Wants`、`Upholds`、`Conflicts`。适配器在读取时按 `systemctl` 打印格式的 ASCII 空格分隔并排序完整 token，再冻结预览；不解释 token 的引号或转义，不去重。有效 unit ID 不含字面空白，编码后的名称仍作为一个完整 token 保留。manager reload 后只改变这些成员的排列，不会被判作配置编辑；成员增删仍会拒绝。其他属性、源文件清单、DropInPaths 和 Conditions 的顺序保持原样，源文件对象与字节摘要仍精确比较。旧预览若冻结了不同于当前规范的 snapshot，由既有 `stale_service_plan` 检查要求重新预览；不改写旧计划或批准。

## 操作与批准

先登记实际配置 root。以下示例的 UUID 是待替换的已登记环境 ID；所有 unit 名称和状态目录由操作者明确选择。

```json
{"command":"service_inspect","environment_id":"<registered UUID>","manager":"user","unit":"example.service"}
```

`service_inspect` 是只读入口，显示 root 绑定、active/sub state、enablement、Restart、MainPID、cgroup、触发来源，以及原暂停任务是否仍被当前 manager 实际加载。`quiesced: true` 只在精确 blocker 已加载、permit 不存在、service inactive、MainPID 为零且 cgroup 为空时成立。旧 completed receipt 只证明当时的执行结果。

```json
{"command":"plan_service_quiesce","environment_id":"<registered UUID>","manager":"user","unit":"example.service"}
```

预览包含 manager、unit、root、原启动状态、完整 blocker 路径与影响范围。`execute` 必须使用该计划的 `id` 和准确的 `hash`。远程变更继续走 runner 的 durable `submit`，重连后只查询原 job，不能创建新计划来代替结果不确定的旧任务。CLI 的交互 TUI 通过 `s` 进入 inspect、quiesce、resume，并要求输入 `apply` 批准具体计划。

core 先持久保存 job 与 blocker 写入意图，然后在该 unit 的默认持久配置目录创建唯一文件：

- user manager：当前 Engine home 的 `.config/systemd/user/<unit>.d/90-lintel-<plan UUID>.conf`；
- system manager：`/etc/systemd/system/<unit>.d/90-lintel-<plan UUID>.conf`。

blocker 添加一个非 trigger `ConditionPathExists`，指向 state 内该任务唯一且永不创建的 `.service-resume-permit` 路径。permit 不存在时，每种启动来源都跳过目标 service；state 暂时不可用或晚挂载时仍阻止启动。core reload manager，读取结构化 D-Bus `Conditions` 确认精确条件已加载，再只 stop 已批准 unit，并核验 cgroup 空。enablement、原 unit 文件、其他 drop-in 与邻居 service 不被改写。

ConditionPathExists 的路径值按原始绝对路径写入，不加 JSON 或 shell 引号；路径中的 `%` 写为 `%%`，避免 systemd specifier 展开改变目标。state 路径需是无控制字符的 UTF-8 路径，无法表达时在预览阶段明确拒绝。执行还会核对冻结 blocker 与当前生成规则；旧暂停计划若保存了不同语义的文本，会返回 `stale_service_plan`，要求重新预览。文件存在或 daemon-reload 成功都不能替代实际 loaded condition 的精确核验。

暂停是持久的，直到独立批准恢复。它不会自动随着窗口关闭、SSH logout 或客户端重开而恢复；磁盘 blocker 可跨 reboot 保留。user manager 在 logout 后是否继续运行、runner job 是否完成，以及 reboot 后当前 unit 是否仍正确加载，都需要独立真实运行证据，不能从配置或旧 receipt 推断。

## 恢复与冲突

```json
{"command":"plan_service_resume","job_id":"<original quiesce job UUID>"}
```

恢复权威是原 durable plan/job，不是新 registry。预览重新核验 root 对象、unit 源文件、drop-in、精确 blocker、permit 与当前进程状态。相关配置被编辑、替换、重新绑定，或 blocker 被移除/修改/覆盖条件时，返回具体冲突或限制并保留外部编辑。操作者先核对来源，再重新预览；Lintel 不会以恢复为名回写整份 unit 配置。

批准后只移除原任务仍拥有的 blocker。删除使用既有隔离机制绑定冻结的文件对象；删除窗口内出现替换时保留新文件，并在回执中记录恢复目录。core reload 并核验配置。原先 active 的 service 才启动；原先 inactive 的 service 保持 inactive。若 stop 已中断且原先运行的同一绑定服务仍 active，独立恢复只解除 blocker，不再执行 start 或 stop。恢复本身也拥有 durable job，同一个 execute 不重复 start。

任务中断或命令失败会标为 `needs_reconciliation`。重复 execute 只返回原 job；不能自动重发 stop、清理或 start。blocker 尚未加载、写入未完成、state 遗失或当前配置无法解释时，恢复可能无法生成；保留现状与原回执，由操作者核对固定 unit 的配置与 manager，而不是删除 journal/deduplication 记录后重试。

暂停期间，若 reload 后精确 blocker 已加载，但移除自有 hold 后的配置与原预览不同，回执的 `service.configuration_conflict` 保存实际比较的差异键。固定安全属性附 `expected/current` 值；环境赋值、User、condition 参数和源路径只记录差异键，不输出值。源文件按冻结清单中的 index 报告差异键及 digest/device/inode/mode/owner，不输出路径或文件原文。属性长值和超过 32 项的文件差异会明确标记截断。诊断基于读取时已规范化的 unit 关系属性及其他冻结配置；未解释的差异仍在 stop 前拒绝，并保留 blocker 与原 journal，供操作者核对外部编辑。

清理计划仍要求确认非 systemd 写入者已停止，同时扫描可访问 manager 的 loaded 与 installed service 直接 root 绑定，并复查原 hold 的实时证据。发现绑定此 root 却未被批准 hold 的 service，即使填写 `writers_confirmed_stopped: true` 也不会生成清理计划。检测明确属于其他 root 的 Claude 进程时，Linux 清理不把它当作目标写入者。未知 supervisor、wrapper、IDE、手动进程与通过脚本/EnvironmentFile 隐藏的服务绑定仍需独立核对；本适配器没有提供全机写入者识别。

## 独立 acceptance

默认 synthetic 验证与真实 systemd acceptance 是独立入口。

```sh
cargo test -p lintel-core service::tests::
cargo build -p lintel-runner
python3 tests/service_journey.py target/debug/lintel
```

macOS 可验证 schema、批准、journal、冲突与 synthetic 状态转换，不提供 Linux systemd 或 cgroup 运行证据。明确选择的 disposable Linux test system 必须有真实 systemd、cgroup v2、Python 3 和当前 root 权限；脚本只创建唯一 inert fixtures，不使用真实 Claude、账号、个人 home 或网络服务。

```sh
python3 tests/service_systemd_journey.py --runner /absolute/test-runner/lintel --json /var/tmp/service-report.json
```

完整模式核验普通 `/etc/systemd/system` 本地 unit、Restart=always、实际 timer 与 manual activation、精确 root 与 alias 拒绝、邻居持续写入且 InvocationID 不变、durable query/replay、unit 外部编辑冲突、独立恢复与原 inactive 状态。已完成的检查立即保存在报告中。timer/manual 阻止验证通过后，脚本停止唯一自有的 200ms trigger timer，保持其 enablement 与源文件，等待目标 inactive/dead、MainPID 为零且 Job 为空，随后独立核验外部编辑冲突；恢复阶段也在 held manual activation 观察后停止同一自有 timer，核验稳定状态再请求产品 inspect/resume。报告保留 activation、timer phase 和外部编辑请求前后的有限状态及准确错误。脚本不会把 pending/failed 拒绝改判为外部编辑冲突，也不 reset-failed 或重发 Lintel mutation。结束后只清理脚本自有 fixtures。

为真实 reboot 保留 fixtures：

```sh
python3 tests/service_systemd_journey.py --runner /absolute/test-runner/lintel --json /var/tmp/service-prepare.json --phase prepare
# 在明确授权的 disposable Linux test system 上另行进行 reboot。
python3 tests/service_systemd_journey.py --runner /absolute/test-runner/lintel --json /var/tmp/service-recover.json --phase recover --previous-report /var/tmp/service-prepare.json
```

prepare 报告保存 synthetic fixture 标识、home/state/root、unit 名、原 quiesce job、preboot inspect、hold 路径与 boot ID。首次 inspect 前还记录自有 fixture 的 `EnvironmentFiles`、`ExecStop` 与 `ExecStopPost` 有限原始 show/JSON 输出，失败时也写入 `--json` 报告，便于诊断属性读取。execute 未完成时，`failed_service_receipt` 保留原回执及其有限配置差异诊断；报告还附上自有 hold 文本、loaded Conditions/DropInPaths、服务状态与本 unit 最近 30 条有容量上限的 journal 输出，回收后再清理自有 fixtures。prepare 保留的 timer 已停止但仍 enabled，真实 reboot 后要求它重新 active。recover 先查询同一个 job，核验目标跨 reboot 未回写、blocker 仍实际加载且邻居运行，再独立批准恢复。只有 boot ID 实际变化才报告 `persistent_hold_survived_real_reboot`。脚本不自行 reboot、不修改 login policy，也不把服务验收等同于完整 G03 的 SSH/PAM/logout/任务存活验收。

clean `8c9d85c` 的 [CI37177180828](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37177180828) 已在 disposable Ubuntu 24.04.4／systemd 255 root system manager 执行完整模式和 prepare → 真 reboot → recover。暂停期间目标 inactive/dead、MainPID 为零、cgroup 为空；timer/manual 启动不能回写，邻居继续运行且不被重启。外部编辑返回 `service_restore_conflict` 并保留 blocker，独立恢复、原 inactive 状态与重复执行不重启通过；真实变化的 boot ID、重启后 loaded hold 和同一 quiesce job 已核对。user-manager 生命周期、其他 supervisor 与生产 VPS 不由这套结果验收。

机制依据：[systemd unit conditions](https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html)、[systemd service Restart](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html)、[systemctl stop/mask](https://www.freedesktop.org/software/systemd/man/latest/systemctl.html)、[systemd D-Bus Conditions 与依赖属性](https://www.freedesktop.org/software/systemd/man/latest/org.freedesktop.systemd1.html)。空数组与 JSON 读取行为依据官方 v255 的 [systemctl 属性打印实现](https://github.com/systemd/systemd/blob/v255/src/systemctl/systemctl-show.c#L1168-L1309)及 [busctl get-property 实现](https://github.com/systemd/systemd/blob/v255/src/busctl/busctl.c#L1983-L2019)；condition 路径语法依据 [condition parser](https://github.com/systemd/systemd/blob/v255/src/core/load-fragment.c#L2795-L2844)和 [specifier 转义实现](https://github.com/systemd/systemd/blob/v255/src/shared/specifier.c#L49-L87)。关系属性的无序成员语义及分隔格式依据 [dependency getter](https://github.com/systemd/systemd/blob/v255/src/core/dbus-unit.c#L133-L167)、[各属性的 getter 映射](https://github.com/systemd/systemd/blob/v255/src/core/dbus-unit.c#L790-L815)、[字符串数组打印](https://github.com/systemd/systemd/blob/v255/src/shared/bus-print-properties.c#L217-L250)和 [unit ID 字符规则](https://github.com/systemd/systemd/blob/v255/src/basic/unit-name.c#L21-L81)。runtime mask 对高优先级本地 unit 的限制是选用独立持久 drop-in 的原因。
