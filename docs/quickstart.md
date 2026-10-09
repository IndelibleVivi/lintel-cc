# 快速上手：从公开源码做一次本地试用

中文 · [English](quickstart.en.md)

这份文档带新访客在**一次性合成 fixture** 里走完一条小旅程：登记一个 Claude
配置环境，生成合成的指令／记忆／会话原件（另加一个**故意不选**的原件），
用精确选择预览范围，审阅原计划与其 hash，提交一次**只归档**
（archive-only）保全，查询原 ID 和最终结果，再从明确的 `archive_path` 在
**另一份独立 state** 里重新检查与阅读，最后逐字节确认源原件、settings 与
占位凭据都没有被动过。

目标读者：已经拿到 Lintel 源码、想在本机确认 CLI 真实行为的人。

> 本文覆盖的都是**Lintel 0.1.0 Preview / 开发预览**能力。真实认证／native resume、正式
> Chrome/Edge/Firefox/AdsPower、生产 VPS 与签名发行都**未验收**。当前证据与
> 缺口见 [当前状态](current-state.md)。

本文的合成 home 不开放真实宿主网络观察或修改，也不进行公网出口测试。另行明确选择的网络功能及其隐私边界见 [网络指南](network.md)。

---

## 0. 前置条件

- Rust stable（编译 CLI 与 workspace）。
- Python 3（解析 JSON、无回显读取口令）。**不依赖 `jq`**。
- 全程**不需要** Node、Tauri、GUI 或已安装 App。

先取得源码并构建 CLI。`cargo build` 会写入 `target/` 并可能拉取依赖，属于
正常本地构建；它**不**运行 Claude、不发现个人目录、不初始化 Lintel state。

```sh
git clone https://github.com/IndelibleVivi/lintel-cc.git
cd lintel-cc
cargo build -p lintel-runner
```

产物是 `target/debug/lintel`。下面的命令都以 **repository 根目录为当前目录**
运行。

## 1. 一次明确的子 shell 边界

先运行 `sh` 打开一份独立的交互 shell，再逐块执行下面的命令。每块会立即执行，
可以在批准前停下来阅读实际计划；最后用 `exit` 回到原来的 shell。
这里定义一份**一次性 fixture** 和只给每次 CLI 调用注入环境变量的 `lintel`
函数。原 shell 的 `HOME`、`PATH` 保持不变，已安装的 Python 3 仍可用。

```sh
sh
set -eu

# 一次性合成 fixture：完全假的原件，不是你的真实 ~/.claude
ROOT="$(python3 -c 'import os,tempfile;print(os.path.realpath(tempfile.mkdtemp(prefix="lintel-quickstart.")))')"
echo "fixture = $ROOT"
mkdir -p "$ROOT/home" "$ROOT/state" "$ROOT/claude-root/projects/example/memory"

printf '%s\n' 'Synthetic instruction only.'                 > "$ROOT/claude-root/CLAUDE.md"
printf '%s\n' 'Synthetic memory only.'                      > "$ROOT/claude-root/projects/example/memory/MEMORY.md"
printf '%s\n' 'This synthetic memory file is NOT selected.' > "$ROOT/claude-root/projects/example/memory/NOT_SELECTED.md"
printf '%s\n' '{"type":"synthetic","message":"hello"}'      > "$ROOT/claude-root/projects/example/session.jsonl"
printf '%s\n' '{"env":{"SYNTHETIC_KEEP":"1"}}'              > "$ROOT/claude-root/settings.json"
printf '%s\n' 'SYNTHETIC_CREDENTIAL_DO_NOT_COPY'            > "$ROOT/claude-root/.credentials.json"

# CLI 绝对路径（对你的 checkout 有效）；引号保证含空格的路径也能工作
LINTEL="$PWD/target/debug/lintel"

# 只给每次调用注入合成 HOME/state，不改动当前 shell
lintel() { env HOME="$ROOT/home" LINTEL_TEST_HOME="$ROOT/home" LINTEL_STATE_DIR="$ROOT/state" "$LINTEL" "$@"; }
```

整篇后面的代码块**继续在这份交互 shell 内执行**；第 8 步用 `exit` 关闭它。
若任一步报错，先保存显示的原 ID 与 fixture 路径，停止后续步骤并核对，不能重发。

> 本旅程的操作只针对这个合成 root，不调用 Claude、认证、浏览器或 service。
> HOME 分离**不是** OS sandbox，也不证明其它进程或 managed sources 一定访问
> 不到你的真实环境；它只说明本旅程不指向它们。这是**合成测试**，不是生产
> 流程。

## 2. 登记环境

`env register` 把一份已存在的配置根登记进 Lintel，返回一个环境 `id`
（UUID）。后续命令都用这个 id。

```sh
lintel env register --name 'Synthetic quickstart' --root "$ROOT/claude-root" > "$ROOT/register.json"
ID="$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["data"]["id"])' "$ROOT/register.json")"
echo "environment id = $ID"
```

登记只记录 inventory，不运行 Claude、不写目标文件。

## 3. 只读扫描：先看有什么

`work inventory` 是**只读、分页、仅元数据**的清单：它列出所选类别的相对路径、
类别与字节数，返回一个绑定这次扫描的 `digest`。它不读取文件正文。

```sh
lintel work inventory --environment "$ID" --categories instructions,memory,sessions --json
```

你会看到**四条**合成原件：

| path |
| --- |
| `CLAUDE.md` |
| `projects/example/memory/MEMORY.md` |
| `projects/example/memory/NOT_SELECTED.md` |
| `projects/example/session.jsonl` |

以及一个 `digest`。注意 `NOT_SELECTED.md` 出现在清单里但**稍后不会被选**。

可选的容量预检只读元数据，并标出超限文件或扫描缺口。这里先用重复的
`--path` 明确只选三条：

```sh
lintel work preflight --environment "$ID" --categories instructions,memory,sessions \
  --path CLAUDE.md \
  --path projects/example/memory/MEMORY.md \
  --path projects/example/session.jsonl \
  --json
```

返回的 `work_selection.mode` 为 `paths`，`totals.files` 为 3，`eligible` 为
true。当前 runner 准入为**单文件 256 MiB、所选合计 1 GiB、10,000 文件**，
版本化持久记录另有 **16 MiB** 上限。预检通过只代表元数据准入，正式计划仍会
完整读取所选原件。

## 4. 生成 archive-only 计划并审阅

`work archive plan` 生成一份**冻结计划**：它把选择的原件与完整内容摘要冻结
下来。`--output-path` 可省略（默认存进 Lintel state），这里显式写到 fixture
里的新文件。它**不创建新配置根、不注销、不删除、不改 settings**。

```sh
lintel work archive plan --environment "$ID" --categories instructions,memory,sessions \
  --path CLAUDE.md \
  --path projects/example/memory/MEMORY.md \
  --path projects/example/session.jsonl \
  --output-path "$ROOT/carried.age" --json > "$ROOT/plan.json"
```

核对计划里的 `kind=archive`、`outcome=archive_only`、`file_count=3`，以及
`work_selection` 恰好列出这三条路径（**不含** `NOT_SELECTED.md`）。

把 id 与 hash 取出来，再回读冻结计划：

```sh
PLAN_ID="$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["data"]["id"])' "$ROOT/plan.json")"
PLAN_HASH="$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["data"]["hash"])' "$ROOT/plan.json")"
lintel plan show "$PLAN_ID" --json | python3 -m json.tool
```

`plan show` 只读回公共范围：`kind`、`hash`、`output_path`、选择范围，**不含**
文件正文或口令。

> **批准必须匹配这一份计划的 hash**。计划生成后若所选原件或冻结目标身份改变，批准会失效，
> 需要重新预览。没有通用 `--yes`。

## 5. 明确提交（口令只经 stdin）

提交时必须给出**原计划的 hash**。归档口令通过 JSON stdin 传入，**绝不放在
argv、shell history、请求文件或计划／回执里**。下面的 getpass 从控制终端
无回显读取口令；`inspect`／`read` 要求口令至少 12 个字符。

请**自己输入一个至少 12 字符的测试口令**（例如一串临时字串），并在第 7 步
**重复输入完全相同**的口令。它是本次合成 fixture 的临时口令，不是预先存在
的固定值；不要把它用于真实场景。

```sh
python3 -c 'import getpass,json; print(json.dumps({"archive_passphrase":getpass.getpass("Archive passphrase: ")}))' \
  | lintel job submit --plan "$PLAN_ID" --approval "$PLAN_HASH" > "$ROOT/receipt.json"
```

`job submit` 返回的是 **durable ACK**，`status` 通常是 `accepted`。

> **accepted 不等于 completed。** ACK 只说明请求已被持久接收；任务之后仍可能
> 失败。`ok:true` 只表示请求处理成功，任务是否完成要看 `data.status`、steps
> 与 coverage。

要等结果，用 `job wait`（它**只轮询、不重发**，超时返回非零且不重新提交）：

```sh
lintel job wait "$PLAN_ID" --timeout 30s --json | python3 -m json.tool
```

完成后 `status` 为 `completed`，`steps` 中的 `archive` 项为 `completed`，`archive_path` 指向
`$ROOT/carried.age`，并给出 `archive_digest`。archive-only **不会**出现
`new_root`。

任何时刻都可以用原 ID 查询：

```sh
lintel job show "$PLAN_ID" --json | python3 -m json.tool
```

> **query-only**：查询原任务只做**核对、不重放**。它不会自动恢复、迁入或
> 启动，也不需要口令；但查询本身**可能更新持久的中断核对记录**，因此不是
> “纯只读”。中断后只按原 ID 核对，不自动再执行。

## 6. 确认源原件保留、没有新建根

archive-only 保全**不触碰**源目录，也不建立新环境。逐字节核对四条原件
（包括未选的 `NOT_SELECTED.md`）、settings 与占位凭据：

```sh
python3 - "$ROOT" <<'PYTHON'
from pathlib import Path
import sys
root = Path(sys.argv[1])
expected = {
    "CLAUDE.md": b"Synthetic instruction only.\n",
    "projects/example/memory/MEMORY.md": b"Synthetic memory only.\n",
    "projects/example/memory/NOT_SELECTED.md": b"This synthetic memory file is NOT selected.\n",
    "projects/example/session.jsonl": b'{"type":"synthetic","message":"hello"}\n',
    "settings.json": b'{"env":{"SYNTHETIC_KEEP":"1"}}\n',
    ".credentials.json": b"SYNTHETIC_CREDENTIAL_DO_NOT_COPY\n",
}
for relative, original in expected.items():
    assert (root / "claude-root" / relative).read_bytes() == original, relative
ciphertext = (root / "carried.age").read_bytes()
assert ciphertext.startswith(b"age-encryption.org/")
assert expected["CLAUDE.md"].strip() not in ciphertext
print("PASS: 6 source files unchanged; age ciphertext created")
PYTHON
```

再确认只登记了一个 environment，且密文包不含明文、可被识别为 age 包：

```sh
lintel env list --json | python3 -c 'import sys,json;print("environments:",len(json.load(sys.stdin)["data"]["environments"]))'
head -c 19 "$ROOT/carried.age"; echo
```

environment 数量应为 1（只登记了合成根），输出以 `age-encryption.org/` 开头，
且 `Synthetic instruction` 不会以明文出现在包里。

## 7. 在另一份独立 state 里 inspect / read

这是关键一步：把**密文**带到**另一份独立 HOME/state**（模拟另一台机器），
在**没有源 inventory、没有源 job** 的情况下重新解锁它。仍用无回显输入口令，
**不要**把口令放进 shell history。

```sh
mkdir -p "$ROOT/independent/home" "$ROOT/independent/state"

lintel_dest() { env HOME="$ROOT/independent/home" LINTEL_TEST_HOME="$ROOT/independent/home" LINTEL_STATE_DIR="$ROOT/independent/state" "$LINTEL" "$@"; }

```

```sh
# inspect：只读清单，恰好一个来源（这里用 --archive-path）
python3 -c 'import getpass,json; print(json.dumps({"archive_passphrase":getpass.getpass("Archive passphrase: ")}))' \
  | lintel_dest work archive inspect --archive-path "$ROOT/carried.age"

```

```sh
# read：读取包内一条明确路径的正文
python3 -c 'import getpass,json; print(json.dumps({"archive_passphrase":getpass.getpass("Archive passphrase: ")}))' \
  | lintel_dest work archive read --archive-path "$ROOT/carried.age" --path CLAUDE.md

```

```sh
# 这份独立 state 自己没有记录任何工作包
lintel_dest work archive list --json
```

用**与第 5 步完全相同**的测试口令。inspect 会返回恰好三条文件（含 `sha256`
digest，**不含** `NOT_SELECTED.md`）；read 返回该文件正文；最后
`work archive list` 返回 `{"jobs":[]}`，证明阅读**不依赖**源 job。

密文跨主机传输由**显式的系统文件传输**（SSH/SFTP/scp 或可移动存储）完成，
Lintel 不自动跨主机 transfer，也不上传整个 state。

## 8. 关闭子 shell

旅程结束，用 `exit` 关闭第 1 步打开的交互 shell；原 shell 环境保持不变。
合成 fixture 会保留在临时目录，便于检查；没有安装或激活真实环境。

```sh
exit
```

## 9. 连回独立的工作包（可选）

要在独立 state 里**迁入**选中的内容，用 `work import plan`（同样以
`--archive-path` 作为来源）预览最终落点，再另行批准执行。它冻结密文字节
digest 与目标文件状态，同名内容不覆盖。会话与记忆保留为资料，**不宣称可以
续聊**。完整流程见 [Agent CLI 指南的 portable work 一节](agents.md#portable-work)。

---

## 还想继续用 App？

桌面开发路径见仓库根目录 [README 的本地构建章节](../README.md)，其中
`npm run dev:synthetic` 会创建一份独立临时 home/state 并调用真实 CLI，界面明确
标注“测试空间”，不会使用你的 Claude 登录或浏览器资料。普通 `npm run dev`
不提供浏览器到本机的执行通道。

## 只想装 CLI？

公开发布为空、**没有正式下载链接**。请从已核对的源码独立安装，或走候选包
流程（核对身份、平台与字节，再解到新的版本目录，不改 PATH／shell rc／App／
service）。见 [Agent CLI 指南](agents.md#用户安装候选包) 与
[从源码独立安装](agents.md#从源码独立安装)。

## 你应该分清的三件事

| 说法 | 不等于 |
| --- | --- |
| `accepted`（durable ACK） | 任务已完成 |
| 查询原 job（query-only，不重放） | 自动恢复／迁入／启动 |
| 资料完整保全 | 原会话可以续聊 |

## 平台与未验收边界

- **macOS desktop GUI**：Tauri，当前按 unsigned arm64 候选构建。
- **macOS arm64 CLI** 与 **Linux headless CLI**（x86_64／aarch64）：独立 CLI；
  Linux 源码以合成旅程构建与检查。三平台候选包记录存在，**不代表可下载**、
  也不代表 Linux runtime 全通过。
- 真实认证与 native resume、正式浏览器／AdsPower、生产 VPS 与签名发行**尚未
  验收**。
- 每次读取都会先完整解密并私有暂存**整个认证包**；请留出临时磁盘与时间。
- 独立包 inspect 返回文件清单；read 输出私有正文，**不进入普通 job／诊断／
  Agent 元数据**。请自行控制正文的终端显示与后续复制。

## 相关文档

当前源码另有 [遥测控制](telemetry.md) 和 [App 更新与发行准备](app-updates.md)。首次合成试用不访问更新 feed，也不发送遥测测试到公网；App 后台检查默认关闭，正式公钥／feed 未配置，旧安装候选不因此升级。

- [当前状态](current-state.md)：候选／资源／证据表与未交付目标。
- [人类操作指南](operator-guide.md)：GUI 里按目标选入口。
- [Agent CLI 指南](agents.md)：接口发现、计划／批准／查询、portable work 与候选安装。
- [统一验证](verification.md)：默认 synthetic 入口与独立 runtime 关口。
- [预览反馈模板](../.github/ISSUE_TEMPLATE/preview-feedback.yml)：只附合成／脱敏示例。
