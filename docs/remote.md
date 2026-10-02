# SSH 远程控制与断线核对

`platform/ssh/lintel_ssh.py` 是 Python 3.10+、stdlib-only controller，运行在管理端。远端只需要独立的 Rust `lintel` runner 和用户已有的 OpenSSH 服务；无需 Python / Node runtime。controller 使用系统 `/usr/bin/ssh` 与用户现有 agent / 配置，不复制私钥，不建立真实连接来列出 alias，也不自动安装远端 runner。

## 显式连接前的准备

远端 `lintel` 应先经独立、明确批准的安装操作放到该用户 SSH 非交互 PATH 中。当前 controller 没有上传、安装或升级 endpoint，也不会在“连接”时触发安装。仓库没有已完成签名 / 来源核验的远端发行资产记录；实际安装前仍需要匹配架构、来源与 release 校验，并由用户批准。已有 SSH host key 必须先通过用户正常的可信流程核对；controller 不替用户接受新 key 或忽略改变的 key。

只读取指定配置文件中的 literal `Host` aliases：

```sh
python3 platform/ssh/lintel_ssh.py aliases --config path/to/synthetic-ssh-config
```

不会执行 `ssh -G`、`Match exec`、ProxyCommand，也不会展开 `Include` 或 Host 通配符。输出包含忽略的 directive 类型，不能声称已枚举全部 SSH hosts。真正连接时才用用户明确选中的 alias 调用系统 SSH；OpenSSH 此时会按用户配置解析 Match / Include / ProxyCommand。只允许字母数字开头、包含字母数字与 `.`, `_`, `-` 的短 literal alias；不接受 `user@host`、选项、空格或 shell 语法。

## 请求、执行与重连

runner 的固定远程命令是 `lintel request`，协议见 [protocol.md](../contracts/protocol.md)。请求 JSON 只通过 stdin，环境 ID / path / approval 不拼进远程命令字符串。命令行固定开启 `BatchMode=yes`、`StrictHostKeyChecking=yes`、`PermitLocalCommand=no`、`ClearAllForwardings=yes`、禁用 TTY 与有限连接存活检查。stdout / stdin 上限 2 MiB，transport 默认总 deadline 60 秒。stderr 被丢弃，错误不会原样泄露 SSH paths、账号或远端内容；SSH 失败返回有解释的 `transport_unknown`。

以下示例均为 synthetic；实际调用会连接所选 host，必须属于用户已授权的远程操作。

```sh
printf '%s\n' '{"command":"discover"}' | python3 platform/ssh/lintel_ssh.py request synthetic-host
printf '%s\n' '{"command":"plan_policy","environment_id":"00000000-0000-4000-8000-000000000001","preset":"reduce","keep_remote_control":true}' | python3 platform/ssh/lintel_ssh.py request synthetic-host
python3 platform/ssh/lintel_ssh.py execute synthetic-host --plan-id 00000000-0000-4000-8000-000000000002 --approval exact-remote-plan-hash
python3 platform/ssh/lintel_ssh.py reconnect synthetic-host --plan-id 00000000-0000-4000-8000-000000000002
```

示例 UUID 仅展示有效格式；实际使用时须替换成远端 runner 返回的环境 ID、plan ID 和 approval hash。重建计划使用 age 加密工作内容，执行时需要至少 12 个字符的 `archive_passphrase`。通过以下显式选项在不回显的终端提示中输入；不要把口令写进命令参数、shell history 或配置文件：

```sh
python3 platform/ssh/lintel_ssh.py execute synthetic-host --plan-id 00000000-0000-4000-8000-000000000002 --approval exact-remote-plan-hash --ask-archive-passphrase
```

口令只放入本次 `execute` 请求的 JSON stdin，不进入 SSH argv、controller record 或日志。不能安全关闭输入回显时，controller 拒绝读取并且不提交请求。重连 / 查询原 job 不需要也不发送口令；未知结果不会因为重新提供口令而重放。

`request` 只接受 schema 中的 `discover`、`inspect`、`plan_policy`、`plan_reset`、`plan_restore`、`jobs`、`job`、`drift`、`export_support`。controller 不提供 arbitrary shell / file endpoint。变更只能通过独立 `execute` 操作提交目标 runner 返回的 plan ID 和准确 approval hash；远端 core 是计划与实际路径的权威，Mac 缓存不是删除依据。当前 controller 没有包装 remote register / launch / accept_drift 等其他命令。

执行流程：

1. 在管理端独立用户 state directory 中 fsync 保存原 `plan_id` 与 `submission_unknown`，再启动 SSH。默认是用户 home 下 `.local/state/lintel/ssh-controller`；可以显式 `--state-dir` 改到另一私有目录。锁按 alias / plan 串行化同一个本地任务。
2. 每个计划最多发送一次 `execute`。只有有效 response envelope 与匹配 receipt 才保存观察到的状态。缓存不保存 approval、archive passphrase、远端 path、settings、secret 或完整 receipt。
3. SSH 失败、deadline、ACK 丢失、controller 重开，以及重复点击 `execute`，都转为对原 plan/job 的 `job` 查询。协议要求远端 `job` 同时支持原 `plan_id` lookup；不能从是否收到 ACK 推断有没有 mutation。
4. 查不到 job 返回 `reconciliation_required`，不自动重发 execute。先由远端 journal 与目标状态确定实际结果，再明确制定新计划。不要靠删除本地 task record 来“修好”按钮。

本地 record 在调用 SSH 之前持久化不等于远端已 durable accept；`response_received` 也不等于所有步骤完成。远端 receipt 的逐步状态仍是权威。如果首次调用其实在发出前失败，保守的查询路径可能需要人工核对；controller 不会为便利假定无副作用。

## 持续运行与尚未完成的验收

controller 已实现固定命令、严格 host checking、stdin JSON、持久化本地提交意图、重连查询和禁止自动 replay。它**不保证任务脱离 SSH 会话继续执行**。是否 durable accept、journal 恢复、服务端 deadline、进程重启后的 uncertain-step 核对，必须以远端 core / runner 的实际实现与测试为准。此 controller 不启用 linger、不改登录策略、不安装 systemd service，也没有验证用户退出后 user manager 仍存活。

```sh
python3 -m unittest discover -s platform/ssh/tests -v
```

fake SSH executable 测试真实 `subprocess` 边界：固定 argv / host checking、stdin 中 shell 字符保持为数据、静态 alias import 不执行 Match/Include、ACK 丢失后从磁盘恢复查询原任务、缺失 job 不重放、deadline / 输出上限、stderr 不回显、归档口令仅经 stdin 传递且 ACK 丢失后查询不带口令。测试不连接真实 hosts。这些证据不代替 [远程验收 G03 / J01–J10](ACCEPTANCE.md)，也不代表 runner 已部署或后台任务存活已验收。

供 GUI / CLI integration 导入的 API：`list_aliases(Path) -> dict`；`Controller(state_dir).request(alias, payload)`；`.execute(alias, plan_id, approval, *, archive_passphrase=None)`；`.reconnect(alias, plan_id)`。返回 runner JSON envelope；controller 自身失败抛出 `ControllerError(code, message)`。CLI 把它转换为相同 error envelope 并以非零退出。没有任何方法会自动重试 execute 或执行安装。
