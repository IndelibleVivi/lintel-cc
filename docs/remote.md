# SSH 远程控制与断线核对

桌面应用通过 Rust native `remote_request` bridge 使用系统 OpenSSH，**不要求管理端安装 Python**。远端需要独立的 Rust `lintel` runner 和用户已有的 OpenSSH 服务；无需 Node 或桌面 runtime。私钥仍由用户的 OpenSSH 配置和系统 agent 管理，Lintel 不复制私钥或创建公网管理端口。

`platform/ssh/lintel_ssh.py` 保留为可选的 Python 3.10+、stdlib-only 命令行 controller。它有独立 CLI 调用者；桌面不会启动或依赖该脚本。两个入口都只发送固定 runner 请求，不能执行任意远端 shell。

## 连接前的准备与主机登记

远端 `lintel` 应先经独立、明确批准的安装操作放到该用户 SSH 非交互 PATH 中。当前控制端没有上传、安装或升级 endpoint，也不会在“连接”时触发安装。仓库没有已完成签名 / 来源核验的远端发行资产记录；实际安装前仍需要匹配架构、来源与 release 校验，并由用户批准。

桌面可以读取当前用户 SSH config 中的 literal `Host` aliases，也可以手工添加 alias。导入过程仅静态读取当前文件：不执行 `ssh -G`、`Match exec`、ProxyCommand，不展开 `Include` 或 Host 通配符。结果列出被忽略的 directive 类型，不能据此声称枚举了全部 SSH hosts。只有字母数字开头、包含字母数字与 `.`, `_`, `-`、最多 128 字节的 literal alias 才能登记；不接受 `user@host`、选项、空格或 shell 语法。真正连接时，OpenSSH 才按用户选择的 alias 和已有配置解析 Match / Include / ProxyCommand。

登记只保存 alias，不建立连接。用户显式连接后，由远端 runner 返回环境列表和能力；环境、路径与计划始终属于所选主机。macOS 展示数据不是远端删除依据。生成与执行计划都由远端 core 核验当前目标。

已有 SSH host key 必须先通过用户正常的可信流程核对。控制端固定设置 `StrictHostKeyChecking=yes` 与 `UpdateHostKeys=no`，不会自动接受首次 key、忽略变化或更新 known_hosts。连接失败后应在用户自己的 SSH 工具中核对主机身份，再从 Lintel 连接；不能改成自动忽略模式。

## 固定请求与 native API

普通操作的远程命令固定为 `lintel request`；独立执行操作固定为 `lintel submit`，stdin JSON 仍使用 `command: "execute"`。环境 ID、路径、approval 和 archive passphrase 都只通过 stdin 传递，不拼进远端命令。协议见 [protocol.md](../contracts/protocol.md)。

SSH 同时固定开启 `BatchMode=yes`、`PermitLocalCommand=no`、`ClearAllForwardings=yes`、禁用 TTY，并限制连接建立和存活检查。stdin / stdout 上限为 2 MiB，控制端总 deadline 为 60 秒；runner 自己还执行其更窄的请求上限。SSH stderr 不存储、不回显，避免暴露 SSH 路径、账号或远端内容。SSH 中断、输出不完整或超限返回 `transport_unknown`；runner 的合法 error envelope 即使伴随非零退出也保留具体错误。没有自动重试执行。

Tauri command 接受 `remote_request({ payload })`，返回单层 Envelope：`{ok:true,data:...}` 或 `{ok:false,error:{code,message}}`。

| `payload.op` | 其他字段 | `data` / 行为 |
| --- | --- | --- |
| `aliases` | 无 | `{aliases, ignored, coverage}`；静态导入，无连接 |
| `hosts` | 无 | `{hosts:[{alias}], tasks:[{alias,plan_id,lookup_id,status}]}`；恢复本地登记与待核对任务，无连接 |
| `add_host` | `alias` | `{alias}`；幂等持久登记，无连接 |
| `connect` | `alias` | 远端 `discover` 的 `{environments,capabilities}` |
| `request` | `alias`, `request` | 允许的 core 请求原有 `data`；不再嵌套 Envelope |
| `execute` | `alias`, `plan_id`, `approval`，可选 `archive_passphrase` | 一次 `lintel submit`，返回 durable Receipt；重复调用只查原 job |
| `reconnect` | `alias`, `plan_id` | 通过保存的 lookup ID 查询原 Receipt；从不提交执行 |

除了列出 aliases、读取 hosts 和新增 host，其余操作要求 alias 已登记。普通请求有严格字段 allowlist：

- `discover`、`inspect`、`register`、`create_environment`；
- `plan_policy`、`plan_reset`、`plan_cleanup`、`plan_restore`、`plan_import`；
- `cleanup_inspect`、`auth_probe`、`archive_inspect`、`archive_read`；
- `jobs`、`job`、`drift`、`accept_drift`、`reactivate_environment`、`export_support`。

字段与 core 协议一致；未知字段或任意 shell / generic file 命令被拒绝。`execute` 不能从普通 `request` 绕过持久意图记录。归档内容只在用户明确调用 archive 操作时经响应返回；口令只进入本次请求的 stdin，不进入 SSH argv 或控制端记录。此 bridge 不提供远端 interactive launch、系统 service 管理、通用安装或强约束网络组件操作；相应能力必须由所属模块实现并独立验收。

## 持久意图、durable accept 与断线恢复

桌面在 Lintel 用户 state 下的 `remote` 子目录保存 `hosts.json` 和 `tasks/<alias>/<plan-id>.json`。沿用 core 的 `LINTEL_STATE_DIR` override；默认使用平台应用数据目录。目录和记录分别以 `0700` / `0600` 创建，记录原子替换并 fsync 文件与父目录。稳定的 lock file 串行化同一 alias / plan 的并发提交。

控制端执行流程：

1. 先持久保存原 `plan_id`、`lookup_id` 与 `submission_unknown`，再启动 SSH。记录不包含 approval、archive passphrase、远端路径、设置或完整 receipt。
2. 每个本地计划记录最多发送一次 `lintel submit`。runner 必须在远端 journal 持久接受后才返回 accepted Receipt，不能仅凭收到 stdin 宣称 accepted。连接的 `discover` capabilities 提供 detached submission 能力说明。
3. 只有有效 Envelope 与匹配 `plan_id`、`id` 的 Receipt 才更新本地观察到的状态。`accepted` 与 `completed` 是不同状态；执行结果仍以远端逐步 journal 为准。
4. ACK 丢失、deadline、SSH 断开、桌面重开或重复点击执行，都转向原 plan/job 的查询。`hosts.tasks` 让桌面重开后仍能显示这些待核对任务。查询不带口令，也不会创建第二份清理。
5. 原 job 不可获得时返回 `reconciliation_required`。查不到记录不能证明副作用没有发生；应核对远端 journal 和目标状态，不要通过删除控制端记录恢复“执行”按钮。后续操作应基于核对结果生成新计划。

本地 intent 持久化不代表远端已 durable accept。提交在发送前就失败时，同样保留保守的查询路径；工具不会为便利假设无副作用。

`lintel submit` 的 detached worker 行为和 journal 由 runner/core 所有。`setsid` 脱离当前 SSH 会话与“机器登录策略允许 worker 长期存活”是不同事实：控制端不启用 linger、不改登录策略、不安装 systemd service，不能承诺所有 VPS 在退出登录后仍允许任务继续。实际 host 的 session/cgroup 或 user-manager 行为仍需按 [G03 / J01–J10](ACCEPTANCE.md) 验证。主机重启、不可逆 action 的不确定结果与恢复冲突由 core 的 journal/reconciliation 处理，不能靠控制端重新提交修复。

## 可选 Python CLI

CLI 保留静态 alias import、普通请求、一次提交与查询。以下只使用 synthetic 标识；实际命令会连接所选 host，必须属于用户已授权的远程操作。

```sh
python3 platform/ssh/lintel_ssh.py aliases --config path/to/synthetic-ssh-config
printf '%s\n' '{"command":"discover"}' | python3 platform/ssh/lintel_ssh.py request synthetic-host
printf '%s\n' '{"command":"plan_policy","environment_id":"00000000-0000-4000-8000-000000000001","preset":"reduce","keep_remote_control":true}' | python3 platform/ssh/lintel_ssh.py request synthetic-host
python3 platform/ssh/lintel_ssh.py execute synthetic-host --plan-id 00000000-0000-4000-8000-000000000002 --approval exact-remote-plan-hash
python3 platform/ssh/lintel_ssh.py reconnect synthetic-host --plan-id 00000000-0000-4000-8000-000000000002
```

CLI 的普通请求保持已有范围：`discover`、`inspect`、`plan_policy`、`plan_reset`、`plan_restore`、`jobs`、`job`、`drift`、`export_support`。桌面的附加 cleanup/archive 表单走 Rust native bridge。CLI 的旧默认 state 目录是 `.local/state/lintel/ssh-controller`，可通过 `--state-dir` 指定其他私有目录；它与桌面 state 独立，远端 core 仍以同一 plan/job 标识作为唯一执行权威。

归档执行使用 `--ask-archive-passphrase` 进入不回显终端输入；不要把口令放进参数或 shell history。不能安全关闭回显时，CLI 拒绝提交。实际 plan ID 和 approval 必须使用 runner 返回的值。

```sh
python3 platform/ssh/lintel_ssh.py execute synthetic-host --plan-id 00000000-0000-4000-8000-000000000002 --approval exact-remote-plan-hash --ask-archive-passphrase
```

CLI 的可导入 API 是 `list_aliases(Path)`、`Controller(state_dir).request(alias,payload)`、`.execute(alias,plan_id,approval,archive_passphrase=None)`、`.reconnect(alias,plan_id)`。CLI controller error 转为相同 Envelope；没有安装和自动 replay endpoint。

## 合成验证

```sh
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml remote::
python3 -m unittest discover -s platform/ssh/tests -v
```

测试仅使用临时 home/state 和 fake SSH executable，不连接真实 host、不读取真实凭据、不改 known_hosts。native 检查 host 持久化、alias 静态扫描、严格 host checking、固定 request/submit argv、stdin 数据隔离、提交意图先于 SSH、并发只提交一次、ACK 丢失后从磁盘恢复查询、缺失 / 错配 receipt 不重放、口令不持久化、超时 / 输出上限，以及有限 cleanup/archive schema。Python 检查其现有 CLI 路径、归档口令输入、固定 submit 命令与相同的不重放边界。

这些验证证明控制端 transport 与恢复边界，不代表 runner 已部署、真实 VPS session 存活、目标 service 隔离或完整 G03 已完成验收。
