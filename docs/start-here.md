# 第一次使用 Lintel

简体中文 · [English](start-here.en.md) · [文档入口](index.md)

第一次先完成一件小事：**选择两份合成资料，生成加密工作包，再把它打开。** 你会看见 Lintel 如何展示目标、预览变更和记录结果；不用先配置自己的 Claude 或浏览器。

## 选择入口

| 你手里有什么 | 使用方式 |
| --- | --- |
| 源码，想先看界面 | 按下面的“合成界面试用”启动 |
| 已由自己或可信提供方构建的 macOS App | 打开 App；用你明确选定的测试目录登记环境。核对候选来源与版本，见[当前状态](current-state.md) |
| Linux，或只需要 CLI | [Agent CLI](agents.md)负责安装；[CLI 完整试用](quickstart.md)提供全程合成示例 |

当前仍为 0.1.0 Preview；尚无公开 Release 下载、正式签名或已启用的更新 feed。最新本地 macOS arm64 发行准备候选是 10-19 的 `e822d74`，已包含当前客户端控制与界面更新。普通 `npm run dev` 只启动前端，没有下面这条本机执行通道。

## 合成界面试用

准备 Git、Rust stable 与兼容仓库依赖的 Node.js／npm：

```sh
git clone https://github.com/IndelibleVivi/lintel-cc.git
cd lintel-cc
cargo build -p lintel-runner --locked
cd apps/desktop
npm ci
npm run dev:synthetic
```

打开终端报告的地址，确认页面显示**测试空间**。这个入口启动 Vite 合成模式，通过真实 CLI 使用一份新建 HOME／state。脚本准备了 `CLAUDE.md`、`projects/synthetic/memory/notes.md` 和合成 settings；目录路径会打印在终端。

这是浏览器中的开发界面，不能据此判断原生 WebKit、真实认证或正式浏览器集成。你自己的登录与 browser profile 不在这条试用路径里。`e822d74` 原生 App 已在另一份隔离 HOME／state 中完成精确三件归档、退出重开、原任务核对与独立文件阅读；独立 CLI state 已核对三份原文字节。它仍不等于真实个人认证或干净 Mac 下载首装验收。

### 做一次“只生成工作归档”

1. 选择合成环境，打开“工作保全”，选择“只生成工作归档”。
2. 选中合成指令与记忆。查看原件清单、数量和最终归档位置；本次不需要清理或创建新配置根。
3. 生成预览，核对主机、完整配置目录、两份选中原件和归档目标。输入至少 12 字符的临时归档口令，审阅后批准执行。
4. 查看原任务结果，等待归档步骤完成。保存原任务 ID 和工作包位置，再用同一口令解锁并阅读选中的资料。

**这次完成的标志：** 原任务显示 `completed`，归档步骤成功；工作包能打开并读到所选指令和记忆；源文件仍在，没有因为只归档而建立新配置环境。`accepted` 只说明任务已经被接收，仍要继续查看完成结果。

需要证明独立 state 也能打开、未选文件没有混入，以及源文件逐字节不变时，继续 [CLI 完整试用](quickstart.md)。这份较长文档保留完整验证过程。

### 结束试用

在启动 Vite 的终端按 `Ctrl-C`。合成目录会留下供检查；确认不再需要、也没有相关进程使用它后，再删除终端显示的那一份临时目录。归档口令丢失后无法通过 Lintel 找回包内内容。

<a id="build-native"></a>

## 构建原生 macOS App

在 macOS 上准备 Rust、Node.js／npm 与 Xcode Command Line Tools。当前构建还需要当前 Rust toolchain 的 `rust-docs` 和锁定依赖，以生成第三方许可正文。先按上文完成 `npm ci`，然后在 `apps/desktop` 目录运行：

```sh
mkdir -p src-tauri/runner-bundles
npm run desktop:build
```

空 `runner-bundles` 允许不附带 Linux runner 的本地 App 构建；远端安装入口会说明资源缺失。需要该能力时，先按[远程资源准备](remote.md#准备-app-内置资源)准备两种静态 Linux runner，再构建。

产物位于：

```text
apps/desktop/src-tauri/target/release/bundle/macos/Lintel.app
```

构建同时准备当前平台 browser host、Chromium／Firefox 扩展和第三方许可资源。它不会自动安装到 Applications，也不会替你完成 Developer ID 签名、公证或正式发行。当前仅支持匹配 Rust host 的原生构建。更多说明见[桌面指南](desktop.md)与[App 更新](app-updates.md)。

本地构建来自你检出的源码。候选版本号相同不代表文件相同：核对 source revision、包内资源与实际操作范围；当前候选与更早候选的原生、Chromium 和 CLI 证据按[当前状态](current-state.md)分别理解。

## 接入自己的环境

打开 App 后，用“添加现有环境”登记明确的 Claude Code 配置目录；登记记录位置，不修改该目录设置。先点顶栏橙色 `>_` 查看主机、配置根与环境 ID，再在“环境详情”核对配置来源，然后选择自己的任务。

想从新的本地状态开始，可建立空白环境；要带走指令、记忆或会话资料，使用工作保全。配置根、项目 cwd、浏览器 profile 分开选择，参见[基本概念](concepts.md)。

下一次不必重走试用：[操作指南](operator-guide.md)直接按六种目标进入。出错时保留原 ID，查[排错指南](troubleshooting.md)。
