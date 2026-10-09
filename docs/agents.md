# Lintel Agent CLI

此指南面向已有操作授权的 coding agent 和脚本作者。当前为 Lintel 0.1.0 Preview / 源码开发预览，协议 1 / catalog 1；没有正式签名发行或公开 Release 下载。CLI 独立于 GUI，所有配置、计划、执行、归档和恢复调用同一个 core。命令自描述是当前二进制的合同；目标适用性要继续检查，不从“实现存在”推断“这个目标可以执行”。

首次试用可先走 [中文 quickstart](quickstart.md) / [English quickstart](quickstart.en.md)，用一次性合成目录完成保全、原任务查询与独立读包，再按本指南准备实际操作。

## 从 App 接到终端或 Agent

App 的“终端与 Agent”接受一个明确选择的 CLI 完整路径，核对原生平台／架构、protocol、有限静态接口和 HOME／UID／state。没有 CLI 时先使用下面的源码安装，或取得经过核对的当前平台候选包。没有正式下载链接；官网与 App 的 main 文档入口都标为“最新开发说明”。

App 不后台下载、修改 PATH 或复制 journal。缺少 candidate sidecar 表示身份未知；字节一致只说明文件与 sidecar 匹配，不证明签名或源码可信。错误架构、协议不合、摘要不符和无法完成有限静态探测会给出具体错误。

交接包须明确使用原任务所在的 state。对于已接受或结果不确定的 job，执行它给出的原 ID 查询命令；对于会话启动，使用原 launch request 的只读查询。换 CLI 不等于换 state，也不改绑原远端 runner；不从交接文字获得新的执行授权。

## 用户安装候选包

从可信的候选提供方取得当前平台 archive 与索引，核对 candidate 身份和平台；然后使用系统工具解到新的版本目录。下列 identity 是合成示例，须替换为实际索引中的值。

安装与升级只用最简单可检视的机制：解到明确选择的**新身份目录**。先只创建父目录，再用普通 `mkdir "$dest"`（目录已存在会失败）作为是否解包的条件，因此绝不会覆盖已有版本：

```sh
parent="${LINTEL_CLI_ROOT:-$HOME/.local/share/lintel-cli}"
mkdir -p "$parent"
# 用实际索引名替换：<version>-candidate-<revision 前 12 位>
identity="0.1.0-candidate-a239123903b6"
dest="$parent/$identity"
mkdir "$dest" && tar -xzf "candidate-packages/lintel-cli-$identity-macos-arm64.tar.gz" -C "$dest"
```

安装后从选定目录核对实际字节；**目标主机不需要 Node**，用系统 `shasum`／`sha256sum` 校验随包生成的 `SHA256SUMS`。清单覆盖全部 payload、`candidate.json` 与 `README.txt`，只排除清单自身；它证明这些文件的字节一致，不提供签名或可信源码 provenance。再运行绝对路径：

```sh
cd "$dest"
shasum -a 256 -c SHA256SUMS      # macOS
# Linux 上改用：sha256sum -c SHA256SUMS
"$dest/bin/lintel" version --json
"$dest/bin/lintel" capabilities --json
```

升级时把**另一个**身份归档解到另一个目录，重跑上面的校验与 `version`/`capabilities`，再由调用方明确切换到新的绝对路径。安装不替换任何已有版本，不改 PATH、shell rc、App、用户设置、service、Claude、credentials 或浏览器注册；原有 Lintel state 与 job ID 继续可用，只是调用方换了 executable。candidate 不是签名发行：Linux 归档在 macOS 上只做格式与字节检查，没有 Linux 运行时证据；只有本机架构匹配时才在本机执行所选 executable。

卸载某个 CLI 版本时，先停止使用该路径，确认没有该版本正在执行的任务，再删除自己解包的那个候选目录。保留 Lintel state、工作包与原 job ID；移除 executable 不会恢复已经批准的配置，也不会卸载独立安装的浏览器 host 或远端 runner。配置恢复与这些组件各自的管理入口仍是独立操作，不要通过删除 state 代替恢复。


## 从源码独立安装

从已核对的源码 checkout 安装到你明确选择的用户目录（需要 Rust stable 与本地平台 toolchain；CLI 不需要 Node、Tauri、GUI 或已安装 App）：

```sh
cargo install --path apps/runner --locked --root "$HOME/.local/share/lintel-cli/chosen-candidate"
"$HOME/.local/share/lintel-cli/chosen-candidate/bin/lintel" version --json
"$HOME/.local/share/lintel-cli/chosen-candidate/bin/lintel" capabilities --json
```

脚本优先使用这个绝对 executable 路径；要使用 `lintel` 简写，把该目录的 `bin` 明确加入自己的 PATH。安装不自动修改 shell rc、系统 service、Claude、浏览器 profile 或既有 App。Linux runner 不编译 Tauri / browser；macOS CLI 包含有限 browser-host control 的库适配。

升级时在核对新 checkout 后选择**另一个版本目录**重新运行安装，检查 `version`、`capabilities` 和所需 schema，再让调用方选择新的 executable。不要仅凭相同的 `0.1.0` 推断两个候选构建相同；当前候选的已支持操作以 catalog 为准。旧任务仍使用原 Lintel state 与原 ID 查询；SSH controller 给新绑定任务冻结 runner digest，查询旧任务使用其记录的 runner，不因升级改绑。

## 先发现接口，再发现目标

```sh
lintel --help
lintel version --json
lintel capabilities --json
lintel context --json
lintel tasks --json
lintel work session read --help
lintel launch query --help
lintel describe plan_policy --json
lintel schema plan_policy
lintel env list --json
lintel env inspect <environment-id> --json
lintel env components <environment-id> --project-cwd /synthetic/project --json
lintel work preflight --environment <environment-id> --categories instructions,memory,sessions --json
lintel work inventory --environment <environment-id> --categories memory,sessions --json
lintel work archive plan --environment <environment-id> --categories memory,sessions --path projects/synthetic/memory/MEMORY.md --path projects/synthetic/session.jsonl --json
lintel capabilities --environment <environment-id>
```

静态 help/version/capabilities/describe/schema/tasks/context 不运行 Claude、不发现个人目录、不初始化 state。`describe` 返回稳定 operation ID、有限 transports、JSON Schema、平台/目标条件、目标与 Lintel state 副作用、批准方式、秘密字段、结果和恢复语义。显式目标 capabilities 附带该环境的真实 inspection；service 的准确 unit/manager 继续用 `service_inspect`，browser 继续用 profile inspection。

Service unit 只接受准确 `.service` 名称，例如 `claude.service` 或 `claude@synthetic.service`；模板 `claude@.service`、路径、glob 与命令不受支持。schema 与 named CLI/core 共用的名称规则保持一致；named 输入在初始化目标 state 前检查，实际 unit 身份、绑定与 manager 状态仍由 core 核对。

`inspect_components` 对准确 environment 返回有限静态来源与原任务投影，`project_cwd` 可选且属于目标主机。它不调用 Claude、不读取凭据正文、不查询并改写 job；服务仅核对最多 8 个已有记录中的明确 unit，不能代替全局 supervisor 发现。`records` 最多 50 份，扫描／展示缺口分别标记；原任务的历史 coverage 不等于当前运行状态。浏览器 profile 属于独立本机模块，不由 root 推断归属。

`work_preflight` 只读取元数据，返回所选原件数量／字节、限额、阻塞相对路径及扫描完整性；`eligible` 只说明当前元数据准入，不冻结文件、不授予执行。符号链接、不可访问项或预算耗尽必须报告未知／不完整，阻塞列表截断不变成通过。实际 `plan_archive`／`plan_preserve` 继续完整读取原件并冻结摘要；预检后文件改变也必须通过这些原检查。当前流式路线可完整归档该 runner 限额内的大 session；超过其单文件／合计限额仍拒绝。原生 resume 的格式核对另限 ≤8 MiB，不限制独立归档与分页阅读。

`work_inventory` 是有界 metadata-only 分页清单，返回准确 environment/root、`files` 的原相对路径／类别／字节数、`total_files`、`complete`、`digest` 和 `next_offset`。继续页使用返回的 `--offset` 与同一个 `--expected-digest`；元数据清单改变拒绝 `stale_inventory`，不能拼接两份快照。这个摘要只绑定分页元数据，不是内容验证或执行批准；正文摘要由正式计划拥有。

`work preflight`、`work archive plan`、`work preserve plan` 的 repeatable `--path` 对应 additive `selected_paths`。省略则保留整类语义；提供时必须非空、唯一、canonical 相对路径，属于已选类别，不能有 `. / ..`、绝对路径、控制字符或模糊匹配。每个路径最多 4096 UTF-8 字节，最多 10,000 项，序列化数组最多 512 KiB。core 再核对每个原件、祖先和读取边界；只读取选中原件，缺失或不支持对象拒绝，不回退到整类。public plan 的 `work_selection` 显示冻结选择，新环境的 `planned_target` 显示完整最终落点。执行再次核对同一选中 manifest，未选文件增改不扩大范围。cleanup/reset/import 不接受这一字段；既有 archive/session read 的 `--path` 仍表示阅读包内文件。

Named CLI 与有限 SSH 的 `environment_id`、`plan_id`、`job_id` 按 schema 的 UUID 格式检查，接受大小写十六进制的 `8-4-4-4-12` 连字符形式。错误格式返回 `invalid_request`，在本机 state 初始化或 SSH 调用前拒绝；始终使用执行器返回的原 ID。旧 raw protocol-1 仍由 core 按既有 ID 规则处理。

外层 `remote.execute` / `remote.reconnect` 的 `plan_id` 与 `remote.launch` 的 `environment_id` 也共用这条 UUID 规则，在本地记录或 SSH 前检查。`install_id` 属于安装 controller 的独立受限标识规则，使用安装预览返回值。

Named execute 与有限远端新提交的 `approval` 必须是原计划返回的 64 位 lowercase hex hash。格式错误在初始化 named core 或创建远端任务 intent／调用 SSH 前拒绝，格式合法仍须由 core 核对准确 plan.hash 与当前目标。已有远端任务记录的重复 execute 保持 query-only，不因参数错误删除记录或重发；原 ID 的 reconnect 是明确恢复入口。

专用真实 TTY `lintel launch ID` 与 `capabilities --environment ID` 的目标 ID 同样在 core 调用前校验；错误 UUID 返回非零 `invalid_request`，不初始化 state。launch 仍先要求真实 stdin/stdout TTY，不接受 prompt。

`env list` 和 `discover` 可能登记已发现的默认根并保存 inventory；`job` 查询可能持久标记中断。`auth_probe` 会显式运行官方认证状态命令，并以 `identity_observed` 布尔值说明主体是否可核对；官方注销的主体缺失会拒绝预览，认证状态变化会使旧批准失效。未执行旧注销计划缺少绑定时需要新预览，已接受任务仍只查询原 ID。`archive_read` 返回工作正文。不要把它们都当成无副作用元数据操作。`discover` 快捷入口与 `request` JSON discover 返回同一 runner capability 集合。

普通命令 stdout 只有一个 JSON envelope，`ok:false` 退出非零，诊断走 stderr。专用 `launch` 是交互进程接管入口，启动失败诊断写 stderr。`network serve` 是唯一这里明确使用 NDJSON stream 的长期入口。`ok:true` 说明请求处理成功；任务是否完成由 `data.status`、steps、coverage 和 error 判断。

## 完整的计划、批准、查询与恢复

以下 ID/hash 是占位，必须使用**当前主机、当前目标**返回值。先登记准确配置根或从 inventory 选择目标；不得用项目代码目录替代 Claude config root。

```sh
lintel env register --name 'Example workspace' --root /absolute/claude-config
lintel policy plan --environment <environment-id> --preset reduce --no-keep-remote-control
lintel plan show <returned-plan-id>
lintel job submit --plan <returned-plan-id> --approval <returned-plan-hash>
lintel job show <returned-plan-id>
lintel job wait <returned-plan-id> --timeout 30s
lintel restore plan --job <original-job-id>
lintel plan show <restore-plan-id>
lintel job submit --plan <restore-plan-id> --approval <restore-plan-hash>
lintel job wait <restore-plan-id> --timeout 30s
```

`policy plan` 必须显式选择 `--keep-remote-control` 或 `--no-keep-remote-control`。需要 Trusted Devices 条件时传 `--trusted-devices required|not_required|unknown`。custom 使用 `--preset custom --custom-settings '{"DISABLE_TELEMETRY":"disable"}'`；只能对当前 schema 中的七项选择 keep/disable/remove，非 custom 不接受 custom_settings，custom 不接受非空 release_settings。版本/策略/目标变更使批准失效，重新核对并预览。

计划生成会保存冻结计划；不会修改目标 settings。`plan show` 不刷新旧计划，而是读回其公共范围。批准必须匹配该计划的 hash，并来自当前任务对这些动作的既有用户授权；没有通用 `--yes`。后续外部编辑会拒绝恢复；不要把冲突当作强制覆盖理由。

通用 `execute`／`job submit` 只接受其支持的配置、保全、归档、清理、迁入、恢复与服务 mutation 计划；launch、resume 或未知 kind 返回 `invalid_plan_kind`，在新 job／durable ACK 前拒绝。已有原 job ID 仍只查询，不因类型被拒绝而删除原记录或重试提交；启动／续聊改用各自的准确批准入口。

`job submit` 返回 durable ACK，通常 status 为 accepted。ACK 后任务仍可能失败；另一独立进程使用同一个 HOME/state 和原 plan ID 可以查询。`job wait` 在轮询前按共同 schema 校验原 plan ID；无效 UUID 返回非零、`invalid_request`，不会初始化 state。默认等待 30s，支持 0–3600s 或毫秒值；超时返回非零、`error.code=wait_timeout`、原 `plan_id` 和最新 `data`，**不会重新提交**。needs_reconciliation/interrupted 是需要核对的终态，wait 不替你执行恢复。

接受后的失败在持久回执 `error` 中记录 code/message/phase/recovery；已完成步骤和 archive/new_root 等产物保留。查询原记录，不从 warnings 中文文本猜 code、不自动再 execute。还要检查实际托管 `execution`：SSH 断线、父进程退出、logout、reboot 是不同事件，ACK 不保证 reboot survival。当前 runner 不 sudo、不启用 linger、不改变登录策略。

配置发布中断后，先 `job show` 原 ID（或 `call job`）取得 `settings_recovery`。`written` 是核对 staged 文件对象与冻结计划后得到的证据，才允许新建 `plan_restore`；`not_written`／`ownership_unproven` 保留当前文件，不自动恢复、不重发。正常已完成任务的字段恢复仍检查后续编辑，其他字段保留。

## 所有 core 操作与旧调用者

命名入口涵盖常见任务；其余 core 请求使用有限 `call OPERATION`，参数对象从 stdin 进入，同一 schema 校验。交互启动使用真实 TTY 专用入口；`call launch`、`call launch_request`、`call resume_request` 与 JSON request 不得绕过这个边界。App 自己使用有限原生 adapter。例如：

```sh
lintel call cleanup_inspect <<'JSON'
{"environment_id":"<environment-id>"}
JSON
lintel call service_inspect <<'JSON'
{"environment_id":"<environment-id>","manager":"user","unit":"example.service"}
JSON
```

先 `describe` / `schema`，再从真实 inspection 准备准确计划。清理顺序为状态备份/工作归档 → 新根迁入（reset_client）→ 官方注销（若批准）→ 精确文件处理 → 退役（若选）。没有“全环境 export”“全局杀 Claude”“恢复全部”。service pause/resume 是独立批准任务，不自动改绑新根。

旧 `request`、`submit`、`discover`、`inspect ID`、`jobs`、`job ID`、`launch ID`、`tui` 保留。协议 1 raw core request 保留历史 defaults；named CLI 和有限 SSH 使用明确严格字段，不会静默收紧已有 raw caller。`request` 的 execute 为前台处理；后台原 ID 回收使用 submit。`launch` 需要真实 TTY、不接受 prompt，启动的是准确程序和配置根，不能用于后台 mutation。旧 plan_reset/rebuild 及历史“部分完成”回执不改写为新 preservation outcome。

<a id="portable-work"></a>

## 秘密 stdin 与 portable work

秘密只经一次性 JSON stdin，不进入 argv、shell history、永久 request file、plan 或 receipt。手工使用下面的 Python `getpass`（从控制终端无回显读取），也可使用已有获准的秘密输入管道；不要启用会打印输入的 shell tracing。

```sh
lintel work archive plan --environment <source-id> --categories instructions,memory,sessions --output-path /absolute/new-work-package.age
lintel plan show <archive-plan-id>
python3 -c 'import getpass,json; print(json.dumps({"archive_passphrase":getpass.getpass("Archive passphrase: ")}))' |
  lintel job submit --plan <archive-plan-id> --approval <archive-plan-hash>
lintel job wait <archive-plan-id> --timeout 30s
lintel work archive list
```

output_path 可省略以保存在 Lintel state；填写路径时父目录须存在且目的地尚未使用。计划冻结父目录的 device/inode，在接受和发布前复查；目录被替换返回 `stale_plan`，没有这份身份的旧显式输出计划也须重新预览。原任务回执继续只查询、不重跑。archive-only 不建立环境、不处理登录、不改变源文件。`work preserve plan --environment ID --categories ... [--name NAME]` 另行表示归档并准备新环境，成功为完成，outcome=preserved；旧登录保留，接着选新环境保护方案、正常登录与启动。归档仍为 `lintel.work/1`，包含类别、相对路径、原字节和文件 digest，不复制 credentials/settings/hooks/MCP。

当前源码准入为单文件 256 MiB、所选合计 1 GiB、10,000 文件；先用 `work preflight` 读取实际 runner 的 limits，旧候选可能更低。持久 JSON 记录读写共用 16 MiB 上限；超大冻结计划在保存前返回 `state_limit`，正文容量预检通过不代替计划成立，可缩小精确选择后重新预览。work codec 流式编码、完整解密并核验所有原件，再按冻结 mapping 迁入；每页 reader 至多取 256 KiB，正文不常驻整个包。每次读取仍核验全包，state 磁盘须容纳展开原件，目的地另需密文／逐文件临时空间。正常返回清除私有暂存；强制终止可能留下 `.lintel-work-stage-*`，不会自动扫除。reader 限制 scrypt log N ≤ 20；高成本包明确返回 `archive_limit`，不会降级解密。旧 reader 仍有自己的容量限制，不能据包格式相同推断大包可在旧版本读入。 有限 SSH 的 60 秒／2 MiB 响应边界不因 core 容量增加而改变，慢包预览或阅读可能 `transport_unknown`；在目标机独立运行 CLI 核对，已接受 job 保留原 ID 查询。

把**密文**复制到新机器/独立安装后，不需要源 inventory 或源 job：

```sh
python3 -c 'import getpass,json; print(json.dumps({"archive_passphrase":getpass.getpass("Archive passphrase: ")}))' |
  lintel work archive inspect --archive-path /absolute/carried-work-package.age
python3 -c 'import getpass,json; print(json.dumps({"archive_passphrase":getpass.getpass("Archive passphrase: ")}))' |
  lintel work archive read --archive-path /absolute/carried-work-package.age --path CLAUDE.md
python3 -c 'import getpass,json; print(json.dumps({"archive_passphrase":getpass.getpass("Archive passphrase: ")}))' |
  lintel work import plan --environment <destination-id> --archive-path /absolute/carried-work-package.age --categories instructions,memory
```

inspect/read/import 必须恰好一个 source：`--job ID`（本安装记录）或 `--archive-path PATH`（目标主机已有文件）。import 预览和执行先核对整批实际父目录的当前 UID 归属与有效访问／写入权限，已知障碍在任何正文写入前拒绝，不自动 chmod/chown。计划冻结密文字节 digest 与目标文件状态；approved execute 再通过秘密 stdin 给同一口令，包改变、损坏、错误口令、超限或同名目标冲突返回具体错误，不覆盖原文件。跨主机传输由显式系统 SSH/SFTP/scp 或可移动存储完成，见 [人类迁移步骤](operator-guide.md#work)；Lintel 不自动跨主机 transfer，也不上传整个 state。

保全／重建回执的 `new_root` 与 `new_environment_id` 在创建目录前持久保存；它们是 intent，只有 create 步骤完成才证明登记成功。创建或登记失败时只查询原 job，保留准确路径／ID，不因意图存在而重建或自动清理。

新归档／迁入文件使用 macOS/Linux 原子不覆盖 rename；不支持的文件系统／内核返回 `atomic_publication_unsupported`，没有 hard-link 或覆盖 fallback。回执中的 archive_path 是 intent；archive_digest 或完成的 archive 步骤才证明读回。App 仅让已读回的任务归档进入查看／选择入口，独立路径仍由 core 检查原文件。

<a id="browser-adapter"></a>

工作保全、归档、迁入、旧重建及 cleanup 的 reset_client/retire 在 named/finite 请求中必须至少选择一种类别；空 JSON 数组和 `--categories ''` 拒绝。repair_login 不处理工作内容，categories 可省略或为空，schema 按 recipe 条件表达同一规则。protocol-1 raw request 保留历史兼容默认。

`plan_import` 与 `plan show` 的 `import_manifest` 是批准时应核对的最终清单：`package:{format,generator,sha256}`、`files:[{source,destination,category,size,sha256}]`。旧 `import_manifest` 的 source／destination 保持相对路径兼容；新增 `planned_target` 展示批准中的完整配置 root、准确文件落点、用途与启用选择。App 和 CLI 读回相同计划，含重名分配；摘要对应实际冻结字节。它不含文件正文或归档口令。独立 import 的 `task_result.coverage` 中，`planned_target: done` 需要冻结 manifest 的每个文件都有发布、目的地父目录同步及摘要读回之后的完成记录；冻结清单、现有可读字节或总 status 本身都不是这个完成证据。同步结果不确定时保留原 ID／落点查询，不标 done、不重发迁入。

## 分开配置目录、项目目录与会话输入

`CLAUDE_CONFIG_DIR` 指向已登记配置 root；`project_cwd` 是目标主机上已存在、当前用户可用的项目目录。它们可以不同，Lintel 不代为创建或改变项目权限。新启动先预览实际目录、程序和静态版本，再批准这一份不可变请求。`plan_launch`／`plan_resume` 只读取目标与资料并保存冻结计划，不启动外部程序、不需要 TTY，可通过 runner JSON、named CLI 或有限 SSH request 预览。`launch_request`／`resume_request` 则需要准确的原计划与 hash，使用真实 TTY 专用入口，或由 macOS core 原生 adapter 请求 Terminal；runner JSON 和有限 SSH request 不执行它们。远端交互使用独立 remote launch control，绑定原请求和 runner。resume 的批准动作另会发布私有运行副本后启动，不能按只读预览处理：

新 launch／resume 计划的 `startup` 投影列出有限配置候选来源、声明和只查看元数据的认证位置；不返回命令、token 或指令正文，actual_loaded 与认证仍是未核验。候选变化需要重新预览；未尝试的旧计划缺少 private startup binding 也需要新预览，已记录原 ID 仍优先查询。新 Terminal 的 shell 环境、Keychain、组织策略和工作目录外指令 imports 另行核对；`/status` 与 `/mcp` 是用户在目标终端确认的步骤。

新 launch／resume 计划同时冻结目录的 device、inode、owner 与创建时间，以识别 Linux 立即复用 inode 的目录替换；普通新增／删除项目文件不改变目录创建时间。文件系统不能提供创建时间时返回 `directory_identity_unsupported`，不打开客户端。缺少这份身份的旧未启动计划须重新预览；已有原启动记录仍先只读查询，不重放。

```sh
lintel call plan_launch <<'JSON'
{"environment_id":"<environment-id>","project_cwd":"/absolute/project","mode":"interactive"}
JSON
lintel plan show <launch-request-id>
# 在真正的终端运行；不接受 prompt：
lintel launch_request <launch-request-id> <launch-plan-hash>
lintel launch query <launch-request-id>
lintel launch list
```

原 `lintel launch ID` 保留旧默认 cwd=config root；新请求使用明确的项目 cwd。App 的“复制上下文并打开新会话”先复制已审阅的这一稿，再请求 Terminal／PTY；复制失败不会启动。粘贴、发送与模型接收另行确认，正文不写入普通任务记录。

工作包中的会话按需只读、有界分页；请求 `offset` 使用上一响应的 `next_offset`，来源变化时重新解锁，不把旧片段选择映射到新内容。`session_read` 的 `content_kind: text` 提供 `raw_text`，不含 `records`；messages 页提供结构化数组，旧 runner 缺少数组时 App 显示阅读限制，原始文本仍可查看：

```sh
python3 -c 'import getpass,json; print(json.dumps({"archive_passphrase":getpass.getpass("Archive passphrase: ")}))' |
  lintel work session read --archive-path /absolute/work.age --path projects/example/session.jsonl
```

需要继续时，从上一页的 `next_offset` 传入 `--offset`，可用 `--expected-digest` 核对原文件。未知记录、非 ASCII 与特殊换行保留原字节；原始文本显示与结构化解析都是阅读视图，不改写 transcript。thinking／signature 按不透明块处理，不进入自动生成的交接稿。

结构化记录的 `index` 是页内序号；`offset` 是返回记录／片段在原文件中的起始字节，`block_index` 是已解析原行 content 数组的零基块序号，opaque／unknown 块也占据原位置。单内容或 unknown fragment 的块序号为 0；超长行的有界片段不证明完整原行已解析。新交接稿与 `plan_launch.input_reference.files` 保留 path、file／package digest、offset 与 block；元数据不含正文。引用的 `offset` 可选、限整数 0–268435456，`block_index` 可选、限整数 0–1000000 且须同时有 offset；旧缺位置引用继续接受，不补造精度，App 明示缺少字节或块位置。

原生续聊是独立、有限任务，目前只完整评估不超过 8 MiB 的 transcript；大文件仍可完整保全和分页阅读，超过这个 resume 范围明确标为不支持。通过 `call plan_resume` 的一次性 JSON stdin 提交准确的 archive source、path、环境与项目，口令用 `getpass` 获取。先 `describe plan_resume`／`schema plan_resume` 核对当前版本合同；批准后使用真正终端的 `resume_request REQUEST_ID HASH`。支持范围、私有运行副本、认证／原会话未核对状态都在预览中显示。入口使用绝对 transcript 副本与固定 `--resume`／`--fork-session`；不改写 session ID、signature 或索引。读回运行副本与 inert 参数验证不能证明真实 Claude 已恢复成功。

原启动查询不需要 approval 或口令，不执行 Terminal，也不重新准备运行副本。对于 SSH 使用 `lintel remote launch query ALIAS REQUEST_ID`；原启动 runner binding 保留，普通重连和 alias 升级不能重发或换绑它。

## Browser：有限适配与人类等待

macOS CLI：

```sh
lintel browser operations --json
lintel describe browser.submit
lintel schema browser.submit
lintel browser instances
lintel browser operations --instance <paired-instance-id>
lintel browser pair create
lintel browser pair pending
lintel browser pair approve --challenge <returned-challenge>
```

静态 operations 不打开 journal；指定 instance 后返回真实配对/conflict/online 事实，权限仍须扩展确认。只在 profile 所在主机使用，Linux runner 明确返回 browser_component_unavailable。完整安装/加载与 profile 配对是人工步骤，不能用 CLI 成功响应假称已加载扩展。

`browser submit` 与 `browser query` 从 stdin 收有限 Native Messaging control 对象。submit 必須由你生成并保留原 `operation_id` 与准确 `instance_id`，action 必须满足 schema；确认和 permission 由扩展拥有。`browser control` 保留手工 host_path 安装等既有有限操作，不复制 App resource locator 或写浏览器 preferences。

```sh
lintel browser query <<'JSON'
{"instance_id":"<paired-instance-id>","operation_id":"<original-operation-id>"}
JSON
```

profile 未配对、离线、等待浏览器批准、awaiting-browser-restart、uncertain/rejected/completed 分别处理。clear 先隔离准备；真正关闭整个浏览器并观察原生 runtime.onStartup 后，另行批准 finishClear。不能重启扩展 worker 或制造世代来替代真实 browser restart。不得自动重发 uncertain 的删除。网络规则暂停到期／startup 恢复与批准 mutation 共用扩展的串行队列；只消费本次读取并匹配的暂停记录，包括 operation ID，不会让旧恢复擦掉新的暂停。旧无 operation ID 记录按完整记录匹配兼容；规则已有外部改变时保留当前规则与记录；恢复读回不确定时也保留原记录继续核对。

browser schema 的 instance/operation/challenge/receipt ID 与 runtime 一致：8–80 个 ASCII 字母、数字、`-` 或 `_`。Chrome/Edge 扩展 ID 为 32 个 `a`–`p` 字符，Firefox 为固定 `lintel@lintel.local`，安装 schema 按 browser 约束对应身份。manifest helper 的 `chromium` 名称不代表已有 Chromium 注册路径；当前 installer 只接受 Chrome/Edge/Firefox。

## Network：明确的前台 owner

```sh
lintel network serve --config /absolute/reviewed-channel.json
```

配置为 [network.md](network.md) 的 Config，包含显式 default_action 和精确规则。stdout 首条 listening 给出 foreground_process owner、PID、实际 loopback address 和规范化 active_config；连接事件是 NDJSON，Ctrl-C 结束进程通道。只覆盖经过代理的连接，不强制所有进程流量。App 的通道由 App 进程持有；CLI 不查询或停止 App 内通道，不声称共享状态。

<a id="ssh-controller"></a>

## SSH：同一个有限 controller

```sh
lintel remote aliases
lintel remote control <<'JSON'
{"op":"add_host","alias":"approved-alias"}
JSON
lintel remote inspect approved-alias
lintel remote hosts
```

aliases 只解析当前 HOME 的系统 SSH config；add/remove 只改变 Lintel registry。inspect 连接已登记的字面 alias 并返回远端 discover，不修改 Claude。不接受任意 root/executable/shell script，使用固定严格 SSH options，host key 未获信任时不 bypass。

`remote request ALIAS` 从 stdin 接收 schema 允许的有限 core JSON；远端 execute 必须 `remote submit ALIAS`（stdin 必须是 execute 请求），先冻结本地 intent 再使用 runner submit。`remote job ALIAS PLAN_ID` 使用只读 `request.job`：已有记录保持原 ID、原 runner digest 与去重状态；未知 ID 不创建任务记录，不占用未提交计划的提交位置。remove alias 不抹掉历史任务，已有记录仍可查询。

只读查找未知 ID 可通过 `remote request` 发送 `{"command":"job","job_id":"<id>"}`（或互斥 `plan_id`）。已有任务记录使用原冻结 runner；没有记录时采用 alias 当前 runner，不创建提交记录，也不占用尚未提交计划的提交位置。显式恢复可用 `remote control` 的 `reconnect`；没有本地记录时，它先持久记录 query-only 意图和 alias 当前 runner digest，再查询原任务，后续 alias 升级不切换 runner。它保留 no-replay 边界，不能用于探测尚未提交的计划；普通查找使用 `remote job` 或 `request.job`。已有无 digest 的旧任务记录保留 PATH 兼容，去重／不确定提交仍不得删除或重发。

```sh
lintel remote request approved-alias <<'JSON'
{"command":"plan_policy","environment_id":"<remote-id>","preset":"reduce","keep_remote_control":false}
JSON
lintel remote submit approved-alias <<'JSON'
{"command":"execute","plan_id":"<returned-remote-plan-id>","approval":"<returned-remote-hash>"}
JSON
lintel remote job approved-alias <original-remote-plan-id>
```

远端新会话先用 `remote request ALIAS` 的 JSON stdin 准备 `plan_launch`（明确 `environment_id`、`project_cwd`、`mode:"interactive"`），再在 macOS 真实终端使用 `lintel remote launch request ALIAS REQUEST_ID HASH`。续聊先准备 `plan_resume`，再用 `lintel remote launch resume ALIAS REQUEST_ID HASH`；包口令在打开的 SSH Terminal 中再次无回显输入。重复调用只查询原 ID，runner 版本沿用预览绑定。只读恢复使用 `lintel remote launch query ALIAS REQUEST_ID` 或 `lintel remote launch list ALIAS`。

旧 `lintel remote launch ALIAS ENVIRONMENT_ID` 保留已有 caller 的 root-as-cwd 默认：stdin/stdout 都需为终端，不接受 prompt 或额外参数；先只读 launch_context preflight，再由有限 controller 请求 Terminal。JSON `remote control` 的交互 launch/request/resume 被拒绝，不能用隐藏管道发起会话；只读 query/list 不需要 TTY。

install 的 prepare_runner → install_runner → query_install 与 App 共用实现和准确计划批准，资源、原注册和 runner 绑定都再核对，提交不明时只 query_install；未核对安装会在 prepare 与上传入口同时阻止新上传，错误 diagnostic 给出原 install_id。激活要求预览冻结的 previous_binding 仍成立；superseded 表示旧安装已核实但后续绑定保留，verified 表示缺少冻结前置绑定的旧记录仅已核实。旧 preview 缺少该证据时必须重新准备；操作字段见当前 `remote` schema 和 [remote.md](remote.md)。原 Python stdlib controller 保留其已有 caller/subset，作为 legacy compatibility 入口；新增功能以共享 Rust controller 为 canonical，不再平行扩展 Python。

这些路径的合成测试不替代真实 Claude 认证、正式浏览器、native WebKit、生产 VPS 或 Linux logout/reboot 验收。当前证据与完整未交付目标继续在 [current-state.md](current-state.md) 和 [SPEC.md](SPEC.md)。

迁入的 `migration_path_conflict` 表示实际目标文件系统把包内路径视为同名或文件／目录冲突。批准执行中的空文件 preflight 在新根创建／正文写入前整批拒绝，预览不写目标。实际目标已有非目录祖先时，path guard 会在任何正文写入前返回 `path_unreadable`。已有目标根和父目录的权限保持不变，新建父目录为 `0700`、文件为 `0600`。包可继续 inspect/read；整理源内容重新归档，或选择能区分这些路径的文件系统。不要重发原任务来绕过名称冲突。

路径 preflight 在创建临时目录前持久记录 `migration_probe:{path,status}` 和执行中的 `migration_preflight` 步骤。正常清理确认后 status 为 `removed`；无法确认清理则为 `retained` 并停止正文迁入。worker 中断会保留 `executing` 与原路径；按原 job 查询核对目录，查询不删除或重跑它。App 回执显示尚需核对的检查目录。只核对原任务创建的空文件范围，额外内容保留，不把临时目录当成新配置环境。
\n## 维护者附录：构建并打包候选\n\n### 可移植候选包的生成与校验

`scripts/package-cli.mjs` 把**明确提供的** canonical 构建产物打成候选归档：一个 macOS arm64 Mach-O、两个静态 Linux musl ELF（x86_64 与 aarch64），以及 CLI 自己解析的 canonical `remote-runners` 目录。打包器**不编译、不下载、不签名**，只核对格式与字节；缺少、架构错误、带动态加载器或与 canonical runner 字节不一致的 Linux 输入会以具体错误拒绝。两个 Linux CLI 输入必须与该架构的 canonical runner 逐字节相同，旧输入／混合输入不能被标成同一个 candidate。归档是扁平的运行时布局：`bin/lintel`、`bin/remote-runners/manifest.json`、`bin/remote-runners/<triple>/lintel`（两个 Linux runner）、`candidate.json`、`SHA256SUMS`、`README.txt`；CLI 在 `dirname(executable)/remote-runners` 找 runner，所以是 `bin/remote-runners`，不是归档根。

验证与归档使用同一次读取的 manifest bytes；资源目录在打包期间重建时，不把后读的 manifest 混入已验证的 runner snapshot。

每个 canonical Linux runner 不得超过现有 SSH installer 的 32 MiB 上传上限；超限返回 `runner_too_large`，在生成任何 archive／索引之前停止。

格式检查要求 macOS 输入有完整 Mach-O 64-bit header、arm64 CPU 和 `MH_EXECUTE` 类型；Linux 输入有 ELF64 executable／static PIE 类型、位于文件支持的 executable segment 内的非零入口，load segment 的 file size 不超过 memory size，且没有 `PT_INTERP` 或 `DT_NEEDED` 动态依赖。截断 header、object／dylib／无入口 shared object 拒绝；格式检查仍不替代其它架构的实际执行。probe 与显式声明的版本都先通过相同的安全版本语法，才用于候选身份与输出路径。

Mach-O 逐条核对 load-command 数量、边界与 segment／section 结构，并要求 `LC_MAIN` 或 ARM64 `LC_UNIXTHREAD` 入口位于文件支持的 executable segment。只有 header、没有入口、未映射入口或不可执行 segment 的输入均拒绝。打包／最终索引发布报错时，回收本次已发布且 identity／大小／mtime 仍未变化的 links，保留既有文件与可检测到的外部替换或编辑，然后可重试；不承诺进程突然终止后的自动 transaction 恢复。

macOS 输入必须带 `LC_BUILD_VERSION` 的 macOS 平台标记，或 legacy `LC_VERSION_MIN_MACOSX`；其它 Apple 平台和缺少平台标记的输入拒绝。打包器移除子进程的 `TAR_OPTIONS`，并在发布前核对归档成员恰好等于声明清单，避免继承的 tar 配置漏掉或改名 executable。

先在 macOS 上用 `target/release/lintel` 作为 Mac 输入；需要跨平台输入时从源码构建。目标 Linux runner（同时作为该架构的 CLI 输入与 canonical runner）用：

```sh
rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl
cargo zigbuild --locked --release -p lintel-runner --bin lintel \
  --target x86_64-unknown-linux-musl --target aarch64-unknown-linux-musl
# 生成 canonical runner 资源（写入被忽略的 runner-bundles）
node apps/desktop/scripts/prepare-remote-runners.mjs
```

产出 `target/x86_64-unknown-linux-musl/release/lintel` 与 `target/aarch64-unknown-linux-musl/release/lintel`；macOS CLI 由 `cargo build --locked --release -p lintel-runner --bin lintel` 得到 `target/release/lintel`。生成流程见 [remote.md](remote.md#准备-app-内置资源)。在仓库根目录打包；输出目录必须是被忽略或调用方自选的临时目录。

```sh
node scripts/package-cli.mjs \
  --macos-arm64 target/release/lintel \
  --linux-x86_64 target/x86_64-unknown-linux-musl/release/lintel \
  --linux-aarch64 target/aarch64-unknown-linux-musl/release/lintel \
  --linux-runners apps/desktop/src-tauri/runner-bundles \
  --revision "$(git rev-parse HEAD)" \
  --out candidate-packages
```

candidate 的身份是 `<version>-candidate-<revision 前 12 位>`，因此裸 `0.1.0` 从来不是唯一身份；索引写成 `candidates-<identity>.json`，不会覆盖其它身份的索引。`--revision` 必须是精确的完整小写十六进制 git object id（40 或 64 位），它是调用方声明的构建身份，不由 digest 或探测证明。`candidate.json` 记录 product/version/protocol/catalog、`source_revision`、target、payload 文件的字节数与 SHA-256、`signed:false`、`release:false`、平台限制，以及**逐目标**的 `identity_source`：只有 macOS 输入成功返回身份数据时才标为 `macos_input_executed`，Linux 目标标为 `declared_static`。macOS 输入成功探测时其自报 `version` 权威并要求 manifest 一致；不能执行或未返回身份数据时（如 Linux 打包）必须显式传 `--version`，并记录 `identity_verified_executed:false`，否则拒绝。已有归档（包括悬空符号链接）或已有同身份索引都会在任何写入前被拒绝，绝不覆盖。

源码构建的 debug 入口是 `cargo build -p lintel-runner` 后的 `target/debug/lintel`。独立 Native Messaging 可执行文件另行从 `extensions/browser/native-host` 安装；在 Mac 上 `lintel browser control` 的安装预览仍需准确 `host_path`，不会假定 GUI 或 PATH 已提供它。扩展开发包从 canonical extension source 准备，步骤与平台限制见 [browser.md](browser.md)。

远端 runner 的静态资源与 App 使用同一 [生成流程](remote.md#准备-app-内置资源)。仅从明确核对的 canonical bundles 调用 `lintel remote control --bundles /absolute/runner-bundles`，或配置 `LINTEL_RUNNER_BUNDLES`；默认查当前 executable 旁的 `remote-runners`。资源缺少时明确拒绝安装，不下载、不在 VPS 编译。不因为读取此指南就安装或更新真实主机。
