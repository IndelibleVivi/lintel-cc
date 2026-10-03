# Lintel

让 Claude 使用环境清晰、可控。Lintel 将外发设置、工作内容、浏览器操作和变更记录放在同一个本地工具中，提供 macOS 桌面界面与独立 CLI。

**当前是 0.1.0 开发候选，完整 SPEC 尚未交付，未正式发布。** 可以在独立测试环境中使用已接通的功能；已接入有限本地清理、官方注销入口、工作归档迁入与 SSH 控制；真实认证、完整浏览器旅程、进程级网络强约束和正式远端运行仍未验收。具体证据与缺口见 [当前状态](docs/current-state.md)。

## 现在可以做什么

- 登记 Claude Code 配置目录，或创建专用环境。首页选择环境与方案，先预览具体变更，再批准执行。
- 选择“保持功能”“减少外发”或“自定义”。自定义对七项已识别控制逐项选择保持原值、关闭或移除本环境覆盖，显示当前值、来源、影响与恢复方式；其余 settings、通用代理与自设 OTel 保留。静态识别已安装版本，按版本与 Trusted Devices 条件评估 Remote Control；冲突通过准确 diff 明确处理，未知条件标注。配置读回与实际运行分开。
- 查看持久任务结果、检查设置漂移、接受当前值，以及按字段恢复 Lintel 的配置修改。后续编辑冲突会阻止恢复。
- 将选定的指令、记忆和会话文件以口令加密归档，创建新配置根并迁入工作内容。**此操作不注销旧登录或清除旧目录**；会话与记忆以资料形式保留，不宣称可以续聊。
- 解锁既有工作归档、阅读文件并生成迁入计划；同名内容不覆盖。清理页提供修复本地登录、清理并重建和退役路径，精确文件与认证范围先预览；本地文件处理不等于服务端撤销。
- 首页以聊天式操作框组织真实环境操作，提供 Day / Night / System、可摸摸／拖抱／弹飞的 Clawd、连续戳戳害羞与躲藏彩蛋。下拉放飞可打开口袋菜单，进入四幅点阵风景和跳跃小游戏；不调用聊天模型。
- 在桌面环境详情中启动 loopback 代理、设置默认允许／阻止与确切主机／端口规则、读回当前生效配置、查看通道连接，再明确请求通过此通道打开 Claude。它只覆盖经过代理的连接。
- 在设置中预览 Native Messaging 注册、配对扩展和查看实例。站点清理分隔离准备、完整浏览器重启、再次确认删除两步；当前完整持久安装 smoke 尚未通过。扩展开发安装与各浏览器限制见 [浏览器指南](docs/browser.md)。

桌面可登记、移除和撤销移除 SSH alias，复用同一环境、清理、归档和任务界面。移除只影响 Lintel 主机列表，系统 SSH 配置与原任务保留。连接失败会区分 SSH、远端 runner 和响应问题，显示排查步骤、退出码及可展开的错误片段；排查摘要可手动复制。远端需要 Lintel runner。Linux x86_64 / arm64 可在主机面板“检查并准备运行器”，先预览，再批准安装 App 内置的静态 runner；只写目标用户的专用版本目录，不需要 VPS 上的编译环境或 sudo。文件与运行能力核验后再连接；中断后只核对原安装，更新后原任务保留原 runner。已有 PATH runner 仍可使用。连接后可从环境详情或任务回执“打开 Claude”，Terminal 会用同一严格 SSH alias 和选定配置根建立交互会话；不会自动发送 prompt。runner 在持久接收后返回 ACK，由独立会话中的 worker 执行。已验证本地父进程退出后的完成与去重，真实 Linux logout/cgroup 行为仍需验收，见 [远程指南](docs/remote.md)。

App 的“帮助”包含开发者 GitHub、项目源码说明及 [Infra Field Guide](https://github.com/IndelibleVivi/infra-field-guide) 的 VPS 101 / SSH 排障入口。链接由用户点击后在系统浏览器打开，不附带环境或诊断数据；私有源码仓库需相应访问权限。

## 本地构建与试用

需要 Rust stable、Node.js 与 npm；macOS 桌面构建还需要 Xcode Command Line Tools。当前实际构建与测试主机为 macOS arm64，Linux 源码已在 Ubuntu CI 通过合成旅程；独立 OpenSSH runtime 检查见[验证指南](docs/verification.md)，真实认证和生产 VPS 状态分别验收。

```sh
cargo build --workspace
cd apps/desktop
npm ci
npm run dev:synthetic
```

打开开发服务器报告的本地地址。`dev:synthetic` 创建独立临时 home/state 并调用真实 CLI，界面明确显示“测试空间”；不会使用你的 Claude 登录或浏览器资料。生成的临时目录会保留便于检查。普通 `npm run dev` 不提供浏览器到本机的执行通道。

构建桌面应用（若要包含 Linux 安装功能，先按[远程资源准备](docs/remote.md#准备-app-内置资源)构建并打包两种静态 runner；缺少资源时 App 会明确提示）：

```sh
cd apps/desktop
npm run desktop:build
```

输出 `apps/desktop/src-tauri/target/release/bundle/macos/Lintel.app`。这是本地构建候选，未经过 Developer ID 签名、公证或非开发者安装验收，不会自动复制到 Applications。

独立 CLI：

```sh
./target/debug/lintel --help
./target/debug/lintel discover
./target/debug/lintel tui
```

`discover` 可登记已发现的默认根，只检查已知目录，不运行 Claude、shell rc、hooks 或 MCP。修改通过 `lintel request` 的 JSON plan/approval 流程完成，详见 [core 与 CLI](docs/core.md) 和 [协议](contracts/protocol.md)。

## 数据与边界

无需 Lintel 账号、模型 key 或卡密。应用不默认发送 analytics、崩溃上传或远程字体请求；没有在线激活服务。浏览器 host 与 core 只暴露各自有限的操作，不提供任意 shell/文件接口。支持资料本地预览、手动复制，没有自动上传。

配置目录隔离不等于 OS sandbox，也不证明登录凭据相互独立。代理环境变量不等于进程网络强约束。浏览器删除不能撤销；设置恢复会核对当前值，不能“恢复全部”掩盖后续改动。Lintel 不承诺改变服务端账户状态或解除账号关联。

当前 core 使用本地锁、冻结快照与写入前复查，但不能把外部编辑器的并发写入称为已获得原子 CAS。未暂停实际 Claude 写入者；真实使用前需评估这些 [具体限制](docs/core.md#并发与恢复边界)。

## 开发与文档

```sh
python3 tests/verify.py
python3 tests/verify.py --list
python3 tests/verify.py --json /tmp/lintel-verify/evidence.json
```

- [产品目标](docs/SPEC.md)、[86 项完整验收](docs/ACCEPTANCE.md)、[验收证据索引](docs/acceptance-status.json)
- [统一验证与独立 runtime 关口](docs/verification.md)
- [当前实现状态](docs/current-state.md)、[桌面使用与视觉约定](docs/desktop.md)
- [共享 core / CLI](docs/core.md)、[浏览器](docs/browser.md)、[网络](docs/network.md)、[远程](docs/remote.md)
- [目标架构](docs/architecture.md)、[协议](contracts/protocol.md)、[研究依据](docs/RESEARCH.md)

源码：`crates/core` 为计划与文件操作权威，`apps/runner` 提供 CLI，`apps/desktop` 为 Tauri + React，`extensions/browser` 为扩展和 Native Messaging host，`crates/egress` 为受控代理，桌面的 `src-tauri/src/remote.rs` 是 native SSH bridge；`platform/ssh` 保留可选 Python stdlib CLI controller。

Lintel 是独立工具，与 Anthropic 无官方关联。Clawd 形象属于 Anthropic；可选本地字体不随仓库分发。产品名 **Lintel**，仓库暂名 **lintel-cc**。尚未选定公开发行许可证；第三方依赖保留各自许可。
