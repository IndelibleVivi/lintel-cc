# Lintel

中文 · [English](README.en.md) · [官网](https://lintel.page/)

![Lintel Preview：末尾橙色 l 字脚与字符山湖](assets/preview/lintel-banner.png)

一个简洁的 Claude Code 个人工具，把环境与工作收好。审阅配置变化，保全选中的指令、记忆和会话，并核对操作结果。提供 macOS App 与独立 CLI，两者共用执行核心。

**Lintel 0.1.0 Preview / 开发预览。** 源码已公开，可从源码在独立测试环境中试用；目前没有公开 Release 下载或正式签名、公证分发。完整 SPEC 尚未交付；真实认证、正式 Chrome/Edge/Firefox/AdsPower、进程级网络强约束和生产远端环境仍未验收。当前候选来源与分层证据见 [当前状态](docs/current-state.md)。源码公开不构成通用复用许可，原创材料的许可仍待选定。

首次试用从 [中文 quickstart](docs/quickstart.md) 或 [English quickstart](docs/quickstart.en.md) 开始：在一次性测试目录中选择合成原件、审阅归档计划、批准保全、查询原任务，再用独立 state 打开工作包。整个旅程保留源原件，不使用个人登录或浏览器资料。想直接看 App，可用下文的 `dev:synthetic`。

三个可以先体验的场景：

- **收好工作：** 按具体原件选择，加密保全后独立阅读；选择片段整理可编辑交接稿，迁入另行预览批准。
- **审阅变化：** 先看准确目标、范围和 diff，再批准；配置读回与实际运行效果分别核对。
- **核对结果：** 接收与完成分开；中断后查询原任务 ID，恢复时保留后续编辑。

试用遇到卡点，可通过 [Issues](https://github.com/IndelibleVivi/lintel-cc/issues) 反馈目标、步骤与实际结果；请使用合成示例，不附凭据、原始会话、工作包或未脱敏的 state。

## 从眼前的目标开始

| 想做的事 | 当前入口 | 操作说明 |
| --- | --- | --- |
| 减少外发，保留需要的功能 | 保护方案 | [选择方案与核对效果](docs/operator-guide.md#protect) |
| 收好指令、记忆和会话，或准备新环境 | 工作保全 | [只归档、保全与 portable work](docs/operator-guide.md#work) |
| 修复登录、清理客户端状态或退役 | 清理与重建 | [准确范围与先保全的顺序](docs/operator-guide.md#cleanup) |
| 管理具体 browser profile | 首页浏览器 | [加载、配对与两阶段清理](docs/browser.md) |
| 管理 SSH 主机与明确绑定的服务 | 主机与 SSH 连接 | [连接、原任务恢复和服务](docs/remote.md) |
| 核对变化、恢复设置或找回中断结果 | 记录与恢复 | [原任务查询与冲突处理](docs/operator-guide.md#recovery) |

第一次使用读 [人类操作指南](docs/operator-guide.md)；侧栏“终端与 Agent”可选择准确 CLI、核对 HOME/state 与执行用户、审阅并复制当前任务的操作交接包。[Agent CLI 指南](docs/agents.md) 覆盖取得候选、独立安装和自动化。首页、帮助和 `lintel tasks --json` 共用 [六任务映射](contracts/task-catalog.json)。

App 发起的浏览器清理可在浏览器完整重启后从 popup 续办，最终结果回到原 App 任务。未执行预览可取消、五分钟后过期；重新尝试须新预览与批准，旧 ID 不重做。

## 现在可以做什么

- 登记 Claude Code 配置目录，或创建专用环境。顶部选择环境，进入保护方案后预览具体变更，再批准执行。
- 选择“保持功能”“减少外发”或“自定义”。自定义对七项已识别控制逐项选择保持原值、关闭或移除本环境覆盖，显示当前值、来源、影响与恢复方式；其余 settings、通用代理与自设 OTel 保留。静态识别已安装版本，按版本与 Trusted Devices 条件评估 Remote Control；冲突通过准确 diff 明确处理，未知条件标注。配置读回与实际运行分开。
- 查看持久任务结果、检查设置漂移、接受当前值，以及按字段恢复 Lintel 的配置修改。中断后查询原任务会核对具体 settings 发布证据，确认已写入才开放独立恢复预览；后续编辑冲突会阻止恢复。
- 工作保全可只生成加密归档，也可归档后准备新配置根并迁入所选内容。两者成功都按自己的目标显示完成；旧目录、登录与设置保留。独立 `.age` 包脱离原 job/state 仍能阅读和选择性迁入，会话与记忆以资料形式保留，不宣称可以续聊。
- 保全前可按整类，或按项目路径组、会话文件组及单个原件选择；超限会话可明确排除，其余原件完整保全。冻结计划另有 16 MiB 持久记录预算；深路径与大量落点可能在正文限额内仍被拒绝，此时可缩小精确选择。分页文件清单只读取元数据；批准冻结精确路径和完整内容摘要，未选文件不会加入。当前源码支持单文件 256 MiB／合计 1 GiB／10,000 文件；保持 `lintel.work/1` 原字节语义，以流式读写和私有临时磁盘完整核验，资料不截断。旧候选和旧 runner 保留各自限额，升级状态见当前状态。每页阅读也先完整解密全包，须留出临时磁盘与时间；阅读与跨页面等待显示持续状态，翻页时明确标注仍在显示上一页。离开资料页会清除页面正文与口令，已提交的读取仍继续，同一队列请求随后执行。远端阅读继续受有限 SSH 请求边界约束。
- 环境详情的“关联组件”按准确主机／root 展示有限 CLI、配置来源、认证位置、服务原记录和独立浏览器入口；未知与暂不支持明确区分。工作保全提供只读容量预检，指出超限文件与扫描缺口；真实计划仍重新读取全部所选原件。界面按当前 runner 报告的容量排除超限原件，缺少容量报告时不猜阈值。
- 解锁当前任务归档或目标主机上的独立工作包，按需分页阅读消息、工具与未知记录，选择片段生成可编辑的工作交接稿，并另行生成迁入计划；批准界面与 CLI 展示选中文件的来源、最终迁入位置与摘要；密文包由明确的系统文件传输带到目标，同名内容不覆盖。新文件发布要求目标文件系统／内核支持原子不覆盖 rename；不支持时明确拒绝，已有内容保留。清理页提供修复本地登录、清理并重建和退役路径，精确文件与认证范围先预览；官方注销须能观察登录主体，批准冻结的认证状态在执行前及保全后重新核对，变化时停止；本地文件处理不等于服务端撤销。
- 在独立项目 cwd 中打开所选配置环境；启动／resume 预览展示有限 root、项目／上级候选及系统 managed 配置来源，标明 hooks／MCP／helper 声明和认证未知项，首次请求前复查变化。它不执行配置内容或读取凭据正文，也不证明实际加载或隔离。审阅工作稿后复制并打开空白会话，粘贴与发送由用户完成。原生 resume 是另行批准的有限入口，使用 byte-identical 私有 transcript 副本、静态版本支持检查和原请求核对；未知版本明确拒绝，真实 Claude 认证及实际恢复仍未验收。
- 默认首页保留 Clawd、56 句原创时段问候与留白，另有日期／整分钟／稀有彩蛋；特殊日期仅在当天第一次打开显示，本地只记最后显示的特殊日期；“开始一项任务”展开六个用户目标，专业工作区持续显示当前目标、主机与完整配置 root，空环境也能选择主机、浏览器与 CLI。提供 Day / Night / System、可摸摸／拖抱／弹飞的 Clawd、连续戳戳害羞与躲藏彩蛋。下拉放飞可打开口袋菜单，进入四幅点阵风景和跳跃小游戏；Clawd 保留连着身体的四条短直腿，以轻微弹跳跳过障碍，收集不同高度的星星，游戏与官网共用实现、成绩各自留在本机；不调用聊天模型。
- 在桌面环境详情中启动 loopback 代理、设置默认允许／阻止与确切主机／端口规则、读回当前生效配置、查看通道连接，再明确请求通过此通道打开 Claude。它只覆盖经过代理的连接。
- 在首页“浏览器”入口准备伴随扩展、安装本地连接、配对和查看实例。App 自带 Chromium / Firefox 扩展与 Native Messaging host，无需源码或终端构建。选择浏览器，预览并批准准备固定扩展目录，按页面说明在目标 profile 加载；随后填入准确扩展 ID，另行预览并批准本地连接。受管版本更新也需核对并批准，冲突目录不覆盖。扩展仍需开发加载，Firefox 临时加载会在退出后移除，签名或商店发布尚未完成；站点清理分隔离准备、完整浏览器重启、再次确认删除两步，独立 Chromium 的真实持久安装／完整重启 smoke 已通过，正式 Chrome/Edge/Firefox 与 AdsPower 尚未验收。安装流程与各浏览器限制见 [浏览器指南](docs/browser.md)。

桌面可登记、移除和撤销移除 SSH alias，复用同一环境、清理、归档和任务界面。移除只影响 Lintel 主机列表，系统 SSH 配置与原任务保留。连接失败会区分 SSH、远端 runner 和响应问题，显示排查步骤、退出码及可展开的错误片段；排查摘要可手动复制。远端需要 Lintel runner。Linux x86_64 / arm64 可在主机面板“检查并准备运行器”，先预览，再批准安装 App 内置的静态 runner；只写目标用户的专用版本目录，不需要 VPS 上的编译环境或 sudo。文件与运行能力核验后再连接；中断后只核对原安装；旧安装核对不会覆盖后续版本绑定，已有未决安装会阻止另一份上传，更新后原任务保留原 runner。已有 PATH runner 仍可使用。连接后可从环境详情或任务回执“打开 Claude”，Terminal 会用同一严格 SSH alias 和选定配置根建立交互会话；不会自动发送 prompt。runner 在持久接收后返回 ACK；Linux PID 1 为 systemd 且当前用户已经是 root 时，由单任务 system transient service 执行；普通用户只有已有 `Linger=yes` 和可用 user bus 时才使用 user transient service。其他主机继续使用 `setsid`，回执明确显示退出登录后的续跑限制。Lintel 不启用 linger、不执行 sudo、不调整登录策略。App 重开后可查询原任务、查看完整回执，再独立批准恢复；ACK 丢失不会重新提交。托管事实在接收前持久写入，不承诺重启后继续执行。独立 Linux VM 已实际验证符合条件的 system/user manager 在严格 logout 后继续并完成原任务；不符合条件的 setsid 路径以及真实重启后的任务进入原任务核对，没有重新提交。生产 VPS 行为仍需独立验收，见 [远程指南](docs/remote.md)。

桌面重开后，可在主机面板的持久提交记录“查询原任务”，再选择“查看完整回执与恢复”，进入该 alias 和环境的结果页。配置恢复与服务恢复仍各自预览并批准；移除主机后只保留原任务查询，重新登记后才可准备恢复。

App 的“帮助”包含六个任务入口、人类指南、Agent CLI 指南、开发者 GitHub、项目源码说明及 [Infra Field Guide](https://github.com/IndelibleVivi/infra-field-guide) 的 VPS 101 / SSH 排障入口。链接由用户点击后在系统浏览器打开，不附带环境或诊断数据；项目仓库保留完整目标、实际验收证据与当前限制。

独立项目 cwd 的新 launch／resume 预览需要文件系统提供目录创建时间；缺少时明确返回限制，资料阅读仍可用。旧未启动预览需重新核对，已经尝试的启动继续按原 ID 查询。普通项目文件新增／删除不使目录身份失效。

## 本地构建与试用

需要 Rust stable、Node.js 与 npm；macOS 桌面构建还需要 Xcode Command Line Tools。当前本地构建与测试主机为 macOS arm64，Linux 源码已在 Ubuntu CI 通过合成旅程、独立 OpenSSH 与真实 systemd/PAM/reboot VM 检查；具体范围见[验证指南](docs/verification.md)，真实认证和生产 VPS 状态分别验收。

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
# 若不包含 Linux runner，建立空资源目录；安装入口会明确显示缺资源
mkdir -p src-tauri/runner-bundles
npm run desktop:build
```

构建会自动准备 Chromium / Firefox 扩展并编译、打包当前平台的 browser host，目前只支持匹配 Rust host 的 native 构建，跨架构／universal App 会明确拒绝；默认前端构建排除仅供本机使用的可选字体，使用系统 fallback。输出 `apps/desktop/src-tauri/target/release/bundle/macos/Lintel.app`。这是本地构建候选，未经过 Developer ID 签名、公证或非开发者安装验收，不会自动复制到 Applications。App 用户可直接从内置文件准备扩展目录；独立开发 ZIP 与各浏览器加载限制见 [浏览器指南](docs/browser.md)。

独立 CLI（在 repository 根目录执行）：

```sh
./target/debug/lintel --help
./target/debug/lintel context --json
./target/debug/lintel tasks --json
./target/debug/lintel work --help
./target/debug/lintel capabilities --json
./target/debug/lintel describe plan_policy
./target/debug/lintel env list
./target/debug/lintel tui
```

`discover` 可登记已发现的默认根，只检查已知目录，不运行 Claude、shell rc、hooks 或 MCP。命名命令或兼容 `lintel request` 共用计划/批准/执行/查询/恢复。静态 version/schema/describe/capabilities 不初始化 state；submit 拒绝退出非零，accepted 与任务完成分开。交互启动使用真实 TTY 的 `lintel launch ID`，generic `call launch` 明确拒绝。远端交互使用 `lintel remote launch ALIAS ENVIRONMENT_ID`，同样需要真实 TTY；JSON `remote control` 不启动交互会话。

独立 CLI 候选可打包为 macOS arm64、Linux x86_64 与 aarch64 archives，每包携带两种 static Linux remote runners、候选身份及文件校验清单。取得匹配候选后，无需 Rust 或 GUI；核验包和文件，解包到尚不存在的新版本目录，再选择准确的 `bin/lintel`。升级保留旧 executable、state 与原任务 ID，不自动修改 PATH、shell rc 或服务。打包命令、安装／升级和源码构建见 [Agent CLI 指南](docs/agents.md)；完整 core 与兼容合同见 [core 与 CLI](docs/core.md) 和 [协议](contracts/protocol.md)。

## 数据与边界

无需 Lintel 账号、模型 key 或卡密。应用不默认发送 analytics、崩溃上传或远程字体请求；没有在线激活服务。浏览器 host 与 core 只暴露各自有限的操作，不提供任意 shell/文件接口。支持资料本地预览、手动复制，没有自动上传。

配置目录隔离不等于 OS sandbox，也不证明登录凭据相互独立。代理环境变量不等于进程网络强约束。浏览器删除不能撤销；设置恢复会核对当前值，不能“恢复全部”掩盖后续改动。Lintel 不承诺改变服务端账户状态或解除账号关联。

Linux systemd 服务可在“清理与重建”展开“目标后台服务”，核对准确 unit 与配置目录绑定，再独立批准暂停或恢复。暂停添加只属于原任务的持久启动阻止项并停止目标，恢复核对外部编辑、移除该项，按原先状态启动；不改变邻居服务或 enablement。当前用户管理器与 root 登录下的系统管理器均有有限范围，其他 supervisor / 容器 / macOS 服务不支持；见 [服务指南](docs/services.md)。

当前 core 使用本地锁、冻结快照与写入前复查，但不能把外部编辑器的并发写入称为已获得原子 CAS。有限 systemd 暂停不能代表所有 Claude、IDE 或交互写入者已停；真实使用前需评估这些 [具体限制](docs/core.md#并发与恢复边界)。

## 开发与文档

官网源码在 [`apps/site`](apps/site/README.md)：独立静态页面，以六个任务说明当前能力，用合成保全旅程解释预览、批准执行与原任务核对；首屏在字标与山湖之间留出安静空间，完整橙脚承住末尾 `l`，header 的独立 `_` 慢闪。点湖面会产生扩散、渐隐并自行消失的水纹，三幅画中的 Clawd 藏着短暂回应，夜景以柔和暖灰衬托橙色身体与黑眼睛；彩蛋不显示提示或发现计数；四步工作包旅程可点击探索，提供风景暂停与 reduced motion。原件阅读示例、中英文试用入口与 opt-in Clawd 跳跃小游戏保持可达。按需打开的 20 秒英文概念短片保留配音、音乐与音效。完整静态站点已在 [lintel.page](https://lintel.page/) 上线，使用 Cloudflare Pages 同源托管；不连接本机执行核心，不提供正式 App 下载。启动方式见[官网说明](apps/site/README.md)，更新程序见[托管说明](docs/site-hosting.md)。

```sh
python3 tests/verify.py
python3 tests/verify.py --list
python3 tests/verify.py --json /tmp/lintel-verify/evidence.json
```

- [产品目标](docs/SPEC.md)、[本轮产品基线与 18 项行为验收](docs/specs/product-baseline-status.md)、[86 项完整验收](docs/ACCEPTANCE.md)、[验收证据索引](docs/acceptance-status.json)
- [统一验证与独立 runtime 关口](docs/verification.md)
- [当前实现状态](docs/current-state.md)、[桌面使用与视觉约定](docs/desktop.md)、[视觉身份与源资产](docs/visual-language.md)
- [首次试用](docs/quickstart.md)、[English quickstart](docs/quickstart.en.md)、[人类操作指南](docs/operator-guide.md)、[Agent CLI 与安装](docs/agents.md)
- [共享 core / CLI](docs/core.md)、[浏览器](docs/browser.md)、[网络](docs/network.md)、[远程](docs/remote.md)、[Linux 服务暂停与恢复](docs/services.md)
- [当前实现架构](docs/architecture.md)、[协议](contracts/protocol.md)、[研究依据](docs/RESEARCH.md)

源码：`crates/core` 为计划与文件操作权威，`apps/runner` 提供 CLI，`apps/desktop` 为 Tauri + React，`extensions/browser` 为扩展和 Native Messaging host，`crates/egress` 为受控代理，`crates/operations` 为静态操作/schema 与严格字段合同，`crates/remote` 是 GUI/CLI 共用有限 SSH controller，桌面 `src-tauri/src/remote.rs` 只定位 App 资源并适配 native command；`platform/ssh` 保留已有 Python caller 的兼容子集，新增能力只由共享 Rust 路径拥有。

Lintel 是独立工具，与 Anthropic 无官方关联。Clawd 形象属于 Anthropic；可选本地字体不随仓库分发。产品名 **Lintel**，仓库暂名 **lintel-cc**。尚未选定项目原创代码、文档与身份资产的公开复用许可证；源码可见性不构成这些材料的通用复用许可，也不表示正式发布。第三方依赖与 Clawd 形象保留各自权利，未由本项目重新授权。
