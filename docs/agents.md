# Lintel Agent CLI

此指南面向已有操作授权的 coding agent 和脚本作者。当前为 0.1.0 source candidate，协议 1 / catalog 1；没有正式签名发行。CLI 独立于 GUI，所有配置、计划、执行、归档和恢复调用同一个 core。命令自描述是当前二进制的合同；目标适用性要继续检查，不从“实现存在”推断“这个目标可以执行”。

## 独立安装、选定版本与升级

从已核对的源码 checkout 安装到你明确选择的用户目录（需要 Rust stable 与本地平台 toolchain；CLI 不需要 Node、Tauri、GUI 或已安装 App）：

```sh
cargo install --path apps/runner --locked --root "$HOME/.local/share/lintel-cli/chosen-candidate"
"$HOME/.local/share/lintel-cli/chosen-candidate/bin/lintel" version --json
"$HOME/.local/share/lintel-cli/chosen-candidate/bin/lintel" capabilities --json
```

脚本优先使用这个绝对 executable 路径；要使用 `lintel` 简写，把该目录的 `bin` 明确加入自己的 PATH。安装不自动修改 shell rc、系统 service、Claude、浏览器 profile 或既有 App。Linux runner 不编译 Tauri / browser；macOS CLI 包含有限 browser-host control 的库适配。

升级时在核对新 checkout 后选择**另一个版本目录**重新运行安装，检查 `version`、`capabilities` 和所需 schema，再让调用方选择新的 executable。不要仅凭相同的 `0.1.0` 推断两个候选构建相同；当前候选的已支持操作以 catalog 为准。旧任务仍使用原 Lintel state 与原 ID 查询；SSH controller 给新绑定任务冻结 runner digest，查询旧任务使用其记录的 runner，不因升级改绑。

源码构建的 debug 入口是 `cargo build -p lintel-runner` 后的 `target/debug/lintel`。独立 Native Messaging 可执行文件另行从 `extensions/browser/native-host` 安装；在 Mac 上 `lintel browser control` 的安装预览仍需准确 `host_path`，不会假定 GUI 或 PATH 已提供它。扩展开发包从 canonical extension source 准备，步骤与平台限制见 [browser.md](browser.md)。

远端 runner 的静态资源与 App 使用同一 [生成流程](remote.md#准备-app-内置资源)。仅从明确核对的 canonical bundles 调用 `lintel remote control --bundles /absolute/runner-bundles`，或配置 `LINTEL_RUNNER_BUNDLES`；默认查当前 executable 旁的 `remote-runners`。资源缺少时明确拒绝安装，不下载、不在 VPS 编译。不因为读取此指南就安装或更新真实主机。

## 先发现接口，再发现目标

```sh
lintel --help
lintel version --json
lintel capabilities --json
lintel describe plan_policy --json
lintel schema plan_policy
lintel env list --json
lintel env inspect <environment-id> --json
lintel capabilities --environment <environment-id>
```

静态 help/version/capabilities/describe/schema 不运行 Claude、不发现个人目录、不初始化 state。`describe` 返回稳定 operation ID、有限 transports、JSON Schema、平台/目标条件、目标与 Lintel state 副作用、批准方式、秘密字段、结果和恢复语义。显式目标 capabilities 附带该环境的真实 inspection；service 的准确 unit/manager 继续用 `service_inspect`，browser 继续用 profile inspection。

Service unit 只接受准确 `.service` 名称，例如 `claude.service` 或 `claude@synthetic.service`；模板 `claude@.service`、路径、glob 与命令不受支持。schema 与 named CLI/core 共用的名称规则保持一致；named 输入在初始化目标 state 前检查，实际 unit 身份、绑定与 manager 状态仍由 core 核对。

Named CLI 与有限 SSH 的 `environment_id`、`plan_id`、`job_id` 按 schema 的 UUID 格式检查，接受大小写十六进制的 `8-4-4-4-12` 连字符形式。错误格式返回 `invalid_request`，在本机 state 初始化或 SSH 调用前拒绝；始终使用执行器返回的原 ID。旧 raw protocol-1 仍由 core 按既有 ID 规则处理。

外层 `remote.execute` / `remote.reconnect` 的 `plan_id` 与 `remote.launch` 的 `environment_id` 也共用这条 UUID 规则，在本地记录或 SSH 前检查。`install_id` 属于安装 controller 的独立受限标识规则，使用安装预览返回值。

`env list` 和 `discover` 可能登记已发现的默认根并保存 inventory；`job` 查询可能持久标记中断。`auth_probe` 会显式运行官方认证状态命令，`archive_read` 返回工作正文。不要把它们都当成无副作用元数据操作。`discover` 快捷入口与 `request` JSON discover 返回同一 runner capability 集合。

普通命令 stdout 只有一个 JSON envelope，`ok:false` 退出非零，诊断走 stderr。`network serve` 是唯一这里明确使用 NDJSON stream 的长期入口。`ok:true` 说明请求处理成功；任务是否完成由 `data.status`、steps、coverage 和 error 判断。

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

`job submit` 返回 durable ACK，通常 status 为 accepted。ACK 后任务仍可能失败；另一独立进程使用同一个 HOME/state 和原 plan ID 可以查询。`job wait` 默认 30s，支持 0–3600s 或毫秒值；超时返回非零、`error.code=wait_timeout`、原 `plan_id` 和最新 `data`，**不会重新提交**。needs_reconciliation/interrupted 是需要核对的终态，wait 不替你执行恢复。

接受后的失败在持久回执 `error` 中记录 code/message/phase/recovery；已完成步骤和 archive/new_root 等产物保留。查询原记录，不从 warnings 中文文本猜 code、不自动再 execute。还要检查实际托管 `execution`：SSH 断线、父进程退出、logout、reboot 是不同事件，ACK 不保证 reboot survival。当前 runner 不 sudo、不启用 linger、不改变登录策略。

## 所有 core 操作与旧调用者

命名入口涵盖常见任务；其余 core 请求使用有限 `call OPERATION`，参数对象从 stdin 进入，同一 schema 校验。交互启动统一使用 `lintel launch ID` 的真实 TTY 入口；`call launch` 在进入 core 前返回 `interactive_launch_required`。例如：

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

output_path 可省略以保存在 Lintel state；填写路径时父目录须存在且目的地尚未使用。archive-only 不建立环境、不处理登录、不改变源文件。`work preserve plan --environment ID --categories ... [--name NAME]` 另行表示归档并准备新环境，成功为完成，outcome=preserved；旧登录保留，接着选新环境保护方案、正常登录与启动。归档仍为 `lintel.work/1`，包含类别、相对路径、原字节和文件 digest，不复制 credentials/settings/hooks/MCP。

把**密文**复制到新机器/独立安装后，不需要源 inventory 或源 job：

```sh
python3 -c 'import getpass,json; print(json.dumps({"archive_passphrase":getpass.getpass("Archive passphrase: ")}))' |
  lintel work archive inspect --archive-path /absolute/carried-work-package.age
python3 -c 'import getpass,json; print(json.dumps({"archive_passphrase":getpass.getpass("Archive passphrase: ")}))' |
  lintel work archive read --archive-path /absolute/carried-work-package.age --path CLAUDE.md
python3 -c 'import getpass,json; print(json.dumps({"archive_passphrase":getpass.getpass("Archive passphrase: ")}))' |
  lintel work import plan --environment <destination-id> --archive-path /absolute/carried-work-package.age --categories instructions,memory
```

inspect/read/import 必须恰好一个 source：`--job ID`（本安装记录）或 `--archive-path PATH`（目标主机已有文件）。import 计划冻结密文字节 digest 与目标文件状态；approved execute 再通过秘密 stdin 给同一口令，包改变、损坏、错误口令、超限或同名目标冲突返回具体错误，不覆盖原文件。跨主机传输由显式系统 SSH/SFTP/scp 或可移动存储完成，见 [人类迁移步骤](operator-guide.md#work)；Lintel 不自动跨主机 transfer，也不上传整个 state。

<a id="browser-adapter"></a>

`categories` 在 named/finite 请求中必须至少选择一项；空 JSON 数组和 `--categories ''` 都拒绝。protocol-1 raw request 保留历史兼容默认。

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

profile 未配对、离线、等待浏览器批准、awaiting-browser-restart、uncertain/rejected/completed 分别处理。clear 先隔离准备；真正关闭整个浏览器并观察原生 runtime.onStartup 后，另行批准 finishClear。不能重启扩展 worker 或制造世代来替代真实 browser restart。不得自动重发 uncertain 的删除。

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

`remote request ALIAS` 从 stdin 接收 schema 允许的有限 core JSON；远端 execute 必须 `remote submit ALIAS`（stdin 必须是 execute 请求），先冻结本地 intent 再使用 runner submit。`remote job ALIAS PLAN_ID` 为 query-only reconnect：保留原 ID、原 runner digest、去重状态，remove alias 不抹掉历史任务。

```sh
lintel remote request approved-alias <<'JSON'
{"command":"plan_policy","environment_id":"<remote-id>","preset":"reduce","keep_remote_control":false}
JSON
lintel remote submit approved-alias <<'JSON'
{"command":"execute","plan_id":"<returned-remote-plan-id>","approval":"<returned-remote-hash>"}
JSON
lintel remote job approved-alias <original-remote-plan-id>
```

远端交互会话只在 macOS 的真实 TTY 使用 `lintel remote launch approved-alias <remote-environment-id>`：stdin/stdout 都需为终端，不接受 prompt 或额外参数；先只读 launch_context preflight，再由有限 controller 请求 Terminal。JSON `remote control` 的 launch 被拒绝，不能用隐藏管道发起会话。

install 的 prepare_runner → install_runner → query_install 与 App 共用实现和准确计划批准，资源、原注册和 runner 绑定都再核对，提交不明时只 query_install；操作字段见当前 `remote` schema 和 [remote.md](remote.md)。原 Python stdlib controller 保留其已有 caller/subset，作为 legacy compatibility 入口；新增功能以共享 Rust controller 为 canonical，不再平行扩展 Python。

这些路径的合成测试不替代真实 Claude 认证、正式浏览器、native WebKit、生产 VPS 或 Linux logout/reboot 验收。当前证据与完整未交付目标继续在 [current-state.md](current-state.md) 和 [SPEC.md](SPEC.md)。

迁入的 `migration_path_conflict` 表示实际目标文件系统把包内路径视为同名或文件／目录冲突。批准执行中的空文件 preflight 在新根创建／正文写入前整批拒绝，预览不写目标。实际目标已有非目录祖先时，path guard 会在任何正文写入前返回 `path_unreadable`。已有目标根和父目录的权限保持不变，新建父目录为 `0700`、文件为 `0600`。包可继续 inspect/read；整理源内容重新归档，或选择能区分这些路径的文件系统。不要重发原任务来绕过名称冲突。

路径 preflight 在创建临时目录前持久记录 `migration_probe:{path,status}` 和执行中的 `migration_preflight` 步骤。正常清理确认后 status 为 `removed`；无法确认清理则为 `retained` 并停止正文迁入。worker 中断会保留 `executing` 与原路径；按原 job 查询核对目录，查询不删除或重跑它。App 回执显示尚需核对的检查目录。只核对原任务创建的空文件范围，额外内容保留，不把临时目录当成新配置环境。
