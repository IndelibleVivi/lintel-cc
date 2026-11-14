# 产品与升级验收判断 — 2026-11-14

产品路径评审起于本轮首个 source `e573ce228e15ee6cb03c2705baf70a8bab1a8c75`，最终产品 source 为 `353536cc7dc3c8c63baa628955ee66332d95bcbb`（PR #8）。全部工作基于 PR #7 后的 main；以下保留初始、中间和最终版本各自的证据边界。App 与 CLI 源码版本均为 **0.2.0**，发行定位仍是 **Preview**。公开下载仍为 App 0.1.1 Preview／CLI 0.1.0。本分项没有发布、部署、操作真实账号或重新执行已经成立的测试。

本报告复核最终前端、共享控制器和验收脚本的衔接，复用已有明确 source 范围的证据。本轮 exact-head CI、最终原生安装包和升级报告的结论由[总交接](README.md)与[发行工程报告](release-engineering.md)记录；不把准备好的程序预记为运行通过。

## 1. 产品判断

**建议下一版为 0.2.0 Preview；产品层面 CONDITIONAL GO，面向普通用户的一般 macOS App 分发仍为 NO-GO。**

Lintel 已经具备一个可信工具的主要结构：按目标选择任务、批准前固定主机与环境、明确实际动作、保留原任务身份、失败后继续查询，以及不依靠删记录来“重试”。工作资料保全／阅读和任务恢复是最成熟的两条路径。认证来源界面与精确清理也已形成有意义的用户闭环，本轮修复了实际认证命名空间和返回查看损坏记录时的重要缺陷。

最终复核确认 PR #7 的版式、双语说明和任务层级改进有效，应继续保留。本轮又完成认证命名空间、损坏原记录反馈、迟到响应隔离、离线帮助和公开 CLI 下载入口等具体改进；两平台 built-App 流程及最终 App 的有限原生控件操作已有结果。三平台 CLI 最终包与旧公开版资料兼容也已实际通过。现在阻挡普通用户 App 分发的条件已经具体：下载首开信任、实际安装替换、图形授权依据；浏览器手动安装和客户端版本限定支持也要明确告知。继续增加配方或重画界面不能替代这些验收，完整 SPEC 中的未来功能也不应无限推迟范围明确的 CLI Preview。

## 2. 六类任务的验收结论

| 用户目标 | 当前体验与独立判断 | 证据与发行边界 |
| --- | --- | --- |
| 选择保护方案、管理外发 | 六任务入口直达方案；默认 reduce、具体字段编辑、实际变更预览和独立恢复减少了命令研究。适合作为 Preview 核心能力。 | `App.tsx`、`CustomPolicy.tsx` 与 Core 计划／恢复检查形成同一链路；既有 built-App 和合成 Core 验收支持它。设置写回、客户端是否采用、受控代理观察和系统强制阻断必须保持区别，不能声称所有进程、DNS、UDP 或直连都已受控。 |
| 保全、阅读、迁移工作资料 | 默认只归档；精确文件选择、容量提示、独立加密文件阅读与迁入已接通。新环境中的指令启用需要另选，不把保存资料变成自动激活。完成后有阅读入口，用户不必自己拼后续命令。**这是最接近成熟交付的模块。** | `WorkPanel.tsx`、精确选取、独立状态解密／迁入和原字节保全有真实 Core 验证；历史候选也有 native WebKit 观察。旧会话可阅读、可提取上下文，不等于当前真实 Claude 可以成功原生续聊；该限制不妨碍资料保全本身发行。 |
| 清理旧认证与环境、准备下一次使用 | 当前流程先显示来源，再选择精确移除；建议范围不默认扩大到共享 shell。快捷配方收起后，登录来源和主操作更突出。结果区把本地移除、当前进程仍可认证、其他上下文、服务端撤销分开，用户能理解为何“清理已完成”后仍可能有登录。 | `CleanupPanel.tsx`、`AuthenticationSources.tsx` 与真实合成 Core journey 检查实际文件及合成 Keychain 相邻字段。真实 Security.framework 有专门临时库 gate；它不等于个人账号 ACL／实际客户端接受。本轮共享目录选择修复已进入最终 `353536cc`；官方 logout 仍不被自动调用，写入者需按计划条件停止。 |
| 管理指定浏览器 profile | 精确配对、扩展侧确认、真正浏览器重启后的第二阶段批准，以及原 operation 查询构成有判断力的安全流程。首用仍需加载扩展、登记准确 ID 和配对，成本高于消费级即用功能。 | 真实一次性 Chromium profile 和浏览器 API 测试是有效 runtime 证据；App invoke 可能仍是合成边界。Chrome／Edge 使用开发者模式加载，Firefox 使用临时扩展；正式三浏览器与 AdsPower 安装验收尚未完成。可以作为明确标注的 Preview 模块，不能宣传成所有 profile 一键接管。 |
| 通过 App／CLI 管理远端及服务 | 对已有 SSH alias 的用户，准备 runner、冻结安装批准、绑定版本、查看原任务和独立恢复已连贯。本轮损坏历史告警使返回用户不会把“读不到记录”理解成“操作没发生”。 | `RemotePanel.tsx` 的 UI journey 使用合成 SSH／持久 registry；真正 OpenSSH、PTY、systemd、logout／reboot 由独立 runtime 证据承担。setsid 不保证注销后继续，受管任务也不保证重启后自动续跑。每种架构的可执行性必须由相应 native artifact 报告决定。 |
| 检查变化、查找任务、恢复与继续 | 原 ID、原 runner、分页历史、独立恢复批准和保留后续外部编辑已经成为产品行为。失败回执保留阶段、产物和下一步，比重新提交整个任务可靠。**这是第二个明显成熟的模块。** | `HistoryPanel.tsx` 使用 snapshot digest 读取有界详情；Core 有实际进程中断和故障注入，remote 有 ACK 丢失和并发去重。本轮修复了不完整记录被隐藏及 settings 已发布却少报 uncertainty。浏览器 1000 条 operation 上限是明确容量边界，不能通过删除去重记录绕过。 |

对应入口与实现：[任务入口](../../apps/desktop/src/Resources.tsx)、[主 App](../../apps/desktop/src/App.tsx)、[工作资料](../../apps/desktop/src/WorkPanel.tsx)、[认证清理](../../apps/desktop/src/CleanupPanel.tsx)、[浏览器](../../apps/desktop/src/BrowserPanel.tsx)、[远端](../../apps/desktop/src/RemotePanel.tsx)、[历史](../../apps/desktop/src/HistoryPanel.tsx)。完整目标与既有验收边界见[产品基线](../specs/product-baseline.md)、[基线状态](../specs/product-baseline-status.md)和[验证说明](../verification.md)。

## 3. 三类用户能否完成任务

### 首次使用者

首页有“开始一项任务”，六类任务能直达相应页面；现有目录登记与创建新环境分开。界面使用 native dialog、可命名控件、键盘页签和页面主标题焦点，语言切换保留草稿与展开状态。PR #7 的长英文按钮、窄窗口和清理配方层次调整应保留。 本轮还纠正了仍实际显示的两处离线帮助：默认只归档不会创建新环境；官方注销不是 Lintel 当前可勾选执行的步骤，MCP／插件影响需要另行核对。历史回执的旧动作翻译保留，避免改写旧事实。

新用户最可能遇到的困难已经不在“功能藏在哪里”：macOS 会不会接受下载包、怎样选择正确 Claude root、浏览器如何完成手动加载，以及 SSH 本身是否可用，才是主要门槛。App 内的有限诊断与指南能帮助有一定技术背景的 Preview 用户；它们尚不能替代普通用户首装和真实 native 可访问性验收。

第二套候选 `da7ebd2024384ea2019719f63dabef3d9db6cd32` 已从实际 DMG 复制并观察原生 WebKit 窗口：1024×681 的英文浅色界面中导航、任务入口与顶栏完整；AX 暴露有名称的导航、语言、主题控件及 WebArea。窗口截图与 AX 来自该精确 App PID、既有许可和合成 HOME，没有请求权限。协调审计者与发行审计者分别查看了截图。它补齐了该中间版本的原生显示证据；不把只读观察升级为 VoiceOver、实际完整任务或窗口尺寸矩阵。最终 `353536cc` 的独立有限 native 控件验收见第 5 节，不由这套旧截图代替。

### 存在多个 Claude 环境的用户

主机／环境选择进入草稿 key 和计划目标；批准页显示冻结 root，旧回执不随着当前环境选择重新解释。清理来源 ID、来源观察和“已停写”确认不写入可复用草稿，切换 host/environment 后重新建立。A 的迟到响应不能成为 B 的来源清单。

本轮认证修复特别重要：相同目录的显式选择和默认未设置状态可能映射到不同 Keychain 服务；独立 secure-storage 选择也不能被简单当成普通配置 root。共享的执行上下文现在保留可验证的选择语义，无法归属的跨 root 选择会拒绝执行。具体分发版本和真实 API 适用边界见[认证审查](auth-security.md)。这类用户应先使用来源检查和预览，不依靠目录名猜当前账户，也不应把独立 root 当成 OS sandbox。

### 操作中断后回来的用户

原任务和原启动 ID 是正常产品入口，不是排错后才出现的后门。远端面板能保留已移除 alias 的原任务查询；全回执进入正确的原主机和环境；恢复需要新的预览和批准。历史读取失败不再使健康记录一起消失。

新 `records_complete`／`record_errors` 连接了控制器与双语 UI：有损坏记录时明确提示不完整；空列表改为“当前没有可读取的远程任务”，不会断言任务从未发生。详情只展示有限 kind、可验证 alias／ID 和 reason code，未知身份不猜测。展开或刷新不会重发操作。

本轮进一步处理了两种真实返回故障：安装 parent／alias 被文件或 symlink 替换时，准备与批准在 SSH 前失败；合法但缺字段的安装 JSON 进入错误列表，而不会触发 `bundle.version` 等解引用使整个面板崩溃。原 bytes 与健康邻居记录都保留。相关 before/after 复现及 62 项 remote 回归结果见[执行与恢复审查](execution-recovery.md)。修复并不自动重建损坏的原记录；用户仍应保留原 ID 和文件进行核对。

## 4. 升级验收：最终 CLI 旧包到新包已通过，App 跨版本状态与中断查询已补验，安装层仍独立

| 升级问题 | 当前可以成立的结论 | 不能据此推出的结论 |
| --- | --- | --- |
| 旧状态、任务和回执 | 原任务优先查询；旧 protocol-1、工作包格式、旧 runner 绑定及有限旧记录有兼容回归。缺少必要冻结信息的未接收计划要求重新预览，已接收原任务不因此重新执行。 | 源码兼容测试不等于真实公开旧包升级到最终新包已完成。 |
| CLI 0.1.0 → 0.2.0 | 最终 `353536cc` 的 macOS arm64、Linux x86_64、Linux aarch64 三份报告均为 `passed:true`、`upgrade_tested:true`，各自在匹配的原生 OS／CPU 运行最终包。旧公开 0.1.0 先创建真实合成 state／job／密文，新包查询原 ID、保留 archive bytes，并在独立 state 读取及迁入。 | 这些结果不包含普通浏览器下载后的 macOS quarantine／信任首开、App 替换升级或旧版本反向写入新 state；一个平台的结果没有代替其他架构。 |
| 重复原批准 | 最终 CLI 程序不仅核对返回 ID/path，也比较重复前后的实际密文摘要和目录清单，防止“回旧 ID、实际又归档”的假成功。 | 这不证明所有操作在每个旧版本上都可反向写入。 |
| 回滚 | 验收保留旧可执行文件并再次核对摘要；手动选择旧版本的路径仍在。 | 未验证旧版本写入已经被新版本改变的 state；不能把“旧文件还在”表述成自动双向回滚。不得删 state、历史或密文来恢复启动。 |
| App 0.1.1 → 0.2.0 | 最终 DMG 已挂载、复制、严格验签并直接启动；旧公开 App 建立的原状态、原任务与中断原 ID 查询已由最终候选实际补验。 | 尚未建立真实 Finder／Applications 替换、普通下载 quarantine 与用户系统信任首开的成功证据。 |
| 自动更新 | 未配置 updater 的旧公开 Preview 仍需手动获取新包；新源码有更新实现，不会让旧包自动获得 feed。 | SHA-256 不是独立发行者认证；没有配置生产 key/feed 时不能宣传自动升级已可用。 |

最终候选程序：[CLI](../../scripts/accept-cli-candidate.py)、[macOS App](../../scripts/accept-macos-candidate.py)。本地 debug 试验曾在加密归档相关请求超时，未通过完整程序；该失败仍保留。后来通过的是三平台实际最终优化包，来自候选 run `43526964671`，没有把 debug 试验的部分进展补记成通过。准确 artifact provenance 由发行工程报告记录。

## 5. 证据分层与线上事实

| 层次 | 本报告使用的事实 |
| --- | --- |
| 本轮初始 source | `e573ce228e15ee6cb03c2705baf70a8bab1a8c75`，首轮 0.2.0 Preview 修复与候选；当时的 PTY／EPIPE 失败记录保留，不当作完整 gate 通过。 |
| 最终产品 source | `353536cc7dc3c8c63baa628955ee66332d95bcbb`，包含后续测试边界修正与依赖安全更新；是本报告最终结果的绑定对象，尚未公开发行。 |
| 本轮合成／本地 runtime | Core 文件操作、FIFO、实际 rename 后故障、目录替换、损坏 JSON、独立 CLI／PTY 与有限进程路径；具体结果由各审查分项记录。数据是一次性合成数据，SSH／客户端状态可能是 inert adapter。 |
| 既有完整 CI | PR #7 的完整 canonical CI 是有价值的基线；它不验证本轮新增代码。选中的 check 必须真实通过，skipped／未选中的 native 项不被补记为通过。 |
| 最终 browser／frontend CI | 同一 clean `353536cc` 的 push run `43526964668`：Linux 与 macOS browser 各 19/19，frontend 分别 14/14 与 15/15，均无失败、selected skip 或超时。四份 artifact 的 ZIP 字节数与 SHA-256 均与 authenticated GitHub metadata 一致。 |
| 既有 native | `current-state.md` 记录的 `e822d74`／`fcae6cb` 候选真实 WebKit 首用、归档、配置恢复、重开，以及旧包签名／quarantine 事实；各自只适用于所测旧 source。临时 Keychain 与真实 OpenSSH／VM 另有专门证据。 |
| 本轮 final package／installed | 最终 `353536cc` 的三平台 CLI native／跨版本资料实验通过；同源 DMG 的暂存 App 副本、严格签名、进程存活、可见窗口和有限 AX 操作分别有报告。App 仍为 ad-hoc；Gatekeeper exit 3 拒绝，无公证票据，副本没有 quarantine，`ordinary_download_install_accepted:false`。 |
| 公开／已部署 | 协调审计者在普通云浏览器实际观察生产站可用：仍展示 App 0.1.1、旧 source `ebbe237…`；没有 CLI 二进制下载卡片，旧指南没有新认证清理内容。本轮网站源码已补充指向真实公开 0.1.0 三架构资产的有限 CLI 下载区，位于 App 下载之后、源码构建之前；中英页面顺序与精确 URL 已回归。源码没有部署。上述是当前线上差距，不是网站构建失败。 |

公开网站的下一次同步应把下载版本、包摘要、对应 source、架构、信任条件、手动升级步骤和双语指南一起更新。CLI 既然独立发行，就应有可发现的下载入口；不能要求新用户从仓库和 App 文档之间自行猜测。应待最终发行资产确定后部署，不预先把候选显示成公开下载。

### 最终 UI 证据具体覆盖了什么

最终 browser jobs 为 Linux `132235079479`、macOS `132235079436`。两平台 `remote-task-ui` 明确通过原 host／environment 保留、原 ID query-only、关闭结果后抑制迟到原任务响应、关闭 SSH 面板后抑制迟到导航；`auth-sources-ui` 明确通过迟到 A 清单不能进入 B。损坏历史告警保留原 ID 与健康记录，没有新提交。`components-ui` 两平台通过，旧 EPIPE 失败仍保留在其原 source 的证据中。

更新后的离线 help catalog 经两平台 i18n 15/15、coverage 8/8、typecheck/build 与 built-App i18n-ui 检查。`site-ui` 的前置 `site_languages` suite 包含真实公开 CLI 0.1.0 精确 URL／摘要、0.2.0 未公开范围，以及 App → CLI → 源码试用的版面顺序；两平台均通过。两份 frontend `app-release-test` 组合各 24/24，通过有限 CLI allowlist，同时保留未验证 App metadata 的拒绝门槛。上述 App journey 使用各自报告声明的 Chromium／invoke／SSH／合成 Core 边界，没有被改称原生 WebKit 全流程。

最终原生 App 观察来自同一 `353536cc` 的实际 DMG，SHA-256 为 `9865fc72ee05d01b1fb579b5333b579772319debe18efe9198a136c1c86bbf18`。在原生 macOS arm64 的合成 HOME 中，程序核对复制资源并限定到启动的 App PID；使用已存在的 AX／截图许可，没有请求新权限。可见窗口为 1024×681。`window-observation/observation.json` 与 `interactions/report.json` 记录 **11 次 allowlisted AXPress 和 9 个通过的结果状态**，每个结果状态另有 AX 读回与所选窗口截图。

| 原生操作范围 | 通过的结果状态 |
| --- | --- |
| 切换主题并恢复原选择 | `theme-change`、`theme-restored`；切换 Dark Night 后恢复 Follow system |
| 页面导航与新用户目标提示 | `history`、`work-needs-target`、`home-restored`；打开历史、看到尚未选择目标的提示、返回环境页 |
| 任务选择面板 | `task-chooser`、`task-chooser-closed`；打开后确认实际面板，再关闭 |
| 切换语言并恢复原选择 | `chinese`、`language-restored`；选择简体中文后恢复跟随系统 |

这些结果证明最终 App 在真实 WebKit 中能够渲染、暴露有名称的 AX 控件并响应上述导航、主题、语言和面板操作。每个状态均核对操作后的结果，未仅凭 AXPress 返回成功记为通过。该程序没有执行归档、批准、清理、SSH 或任何业务任务；没有使用全局键盘输入、WebKit 注入或系统权限例外。

### 最终 App 的补充范围与未验证路径

- **完整原生任务与 App 升级：** 选定最终 App 的首用归档、批准、回执、重开与读包，以及旧公开 App 的跨版本状态／中断查询已补验。真实 Applications 替换、普通下载 quarantine 与旧 approval UI 承接仍未覆盖。
- **键盘、焦点与 VoiceOver：** 初始 AXPress 没有验收完整键盘路径；本机随后观察到原生 Escape 关闭 Help 并返回 Help 按钮焦点，但没有完成 Tab／Shift-Tab／Enter 矩阵、屏幕朗读顺序、VoiceOver 或窗口缩放矩阵。已有 Chromium 键盘／焦点断言及 AX 元素存在是各自独立的证据。
- **两项实际 bundle 资源 fixtures：** 常规 macOS 56 passed／2 ignored、Linux 52 passed／2 ignored 计数保持。本机 macOS 26.5.2 arm64 在 clean `353536cc` 另行显式执行两个既有 ignored 测试，消费最终 DMG 复制 App 的真实资源，各 **1 passed、0 failed、0 ignored**；host 从合成 HOME 的安装 manifest 启动并验证 native frames，扩展验证 Chrome／Edge／Firefox 选择对应 bytes、仅准备扩展文件、不改 profile。此证据不替代完整真实浏览器旅程。
- **普通下载与信任：** 临时复制、直接启动和上述有限 AX 交互未覆盖带 quarantine 的普通下载首开、Applications 替换或 Gatekeeper 接受。Developer ID／公证与含图形资产的授权条件仍由总交接单独处理。

最终原始报告保存在工程证据包的 `rc-final-ci/browser/`、`rc-final-ci/native-auth/`、`rc-final-ci/native-linux/` 和 `rc-final-candidate-ci/`；source、run、artifact metadata 与摘要相互绑定。完整 candidate gate 的最终结论以总交接为准，本分项的 browser 成功没有代替其他矩阵项。

## 6. 发布前真正需要的决定与最后验收

1. **版本定位：** 保持数字版本 0.2.0，渠道为 Preview。工作资料、配置、精确来源清理和原任务恢复可以进入候选；浏览器手动安装、有限续聊和真实认证覆盖明确标注范围。无需为未来 SPEC 功能无限延期。
2. **逐平台决定：** 最终 Linux x86_64／aarch64 CLI 已获得各自原生包与旧包升级证据，可支持范围明确的 0.2.0 Preview GO；macOS CLI 同样通过原生功能与资料兼容，普通下载信任仍是其技术 Preview 的条件。macOS App 保持有限技术 Preview 的 CONDITIONAL GO、普通用户分发 NO-GO，继续完成第 5 节的实际安装替换与下载信任条件。
3. **macOS 分发方式：** 已选保留 strict ad-hoc、由用户通过系统例外手动信任的技术 Preview，暂不 Developer ID／公证；选定候选原生归档、合成清理及跨版本状态／中断查询已补验，实际安装替换与下载首开仍未验。若目标是普通用户自然下载并信任，需 Developer ID、公证与实际下载／首开／升级证据；不能用“严格签名通过”替代。
4. **更新与上线：** 手动升级是有效的当前方案。生产 updater 的签名 key／公钥／feed 是独立决定；本轮不创建凭据或激活渠道。官网原媒体已补齐，完整离线包已准备；仍须另行批准上线。
5. **资产许可：** 包中的权利归属与未决许可问题依发行工程报告处理，功能测试和第三方 notice 完整不能替代这些决定。

本分项不建议再做大规模 UI 或状态格式重构。已通过的三平台 CLI 最终包和原生 App 有限交互应明确认可；剩余工作集中于选定 App 的实际安装替换、键盘与 VoiceOver、普通下载信任及图形授权。公开下载／指南与实际交付同步仍是发布后的独立部署动作。这些条件具体且有限，不要求先完成全部长期路线图。

## 本机最终候选补充验收

实际 `353536cc` 最终 DMG 在 macOS 26.5.2 arm64、独立合成 HOME/state 中完成空 state GUI 登记 → archive-only 精确选择三件（90 bytes）→ 批准前无 ciphertext → 一次批准、原 ID completed。六份原件 bytes 保持一致且未创建新 root。Quit 后确认进程退出，再以相同 HOME/state 重开，原记录可查询；错误口令返回 archive_locked，重新输入正确口令后三份正文及会话 structured/raw 均可读。另一独立 HOME/state 的实际 packaged CLI inspect/read/list PASS：三件，jobs=[]。旧 App 跨版本补验见下文；真实安装升级仍未覆盖。

独立合成 auth root 的 GUI 清理选择文件 OAuth、settings API、synthetic Keychain OAuth，共三 sources；一次批准的五个 selected steps 全部 done。主界面报告所选本地清理完成，原始 status 仍为 partially_completed、task_result.outcome 为 partial。四层 coverage 分别是 local done、runtime not requested、other contexts/server unverified；fresh 复查显示三来源 removed 且没有新 job。邻接 MCP/plugin/unknown 字段、item identity、work 与原六文件均保持。Keychain 使用合成 adapter，不是个人 native Keychain 库验收。

后续 native 发现促成两项 source-only 修复：四条 archive 英文显示漏投影，以及 Help 硬写 0.1.0／没有公开 release 的文案。fresh work-ui、i18n-test、i18n-coverage-test、i18n-ui 已通过，新 dirty QA App strict ad-hoc build 与复制校验通过（59 path entries、26,295,400 bytes）；该 QA App 不是原 `353536cc` 候选，尚未完成修后 native 复验。固定候选 source 身份不变，本轮也不能再描述为仅文档变化。

App updates 中英文指南的 TMPDIR 示例已改为 fixture 父目录；实际交付 CLI 相同参数修改前返回 invalid_path，修改后 auth inventory PASS，site guides 已重新生成，镜像 16 tests PASS。原媒体离线 payload 已准备，但其冻结 source 仍是 `353536cc`；含新版指南命令的网站包待另行生成，未部署。

### 旧公开 App 跨版本状态与中断查询补验

真实旧公开 0.1.1 DMG 副本 GUI 在合成 root 建立四件 134 bytes 的 archive-only completed 任务，并保留未批准 cleanup plan。第二次归档经 GUI 批准，外部合成 monitor 在原 job executing 已持久化时仅 SIGSTOP/SIGKILL 测试 App，保留 records/bytes。真实 353536cc 候选以相同 HOME/state 启动后同时显示旧 completed 与 executing 原 ID；打开中断任务并查询最新后，原 ID 转 needs_reconciliation，界面明确结果不确定、查询原任务、勿重新提交，启动禁用。jobs 仍仅两个，无第二份 ciphertext；旧 completed receipt 原字节、旧 ciphertext 大小与摘要、六份源文件 bytes 均不变。旧未执行 cleanup preview 由实际最终 CLI execute 拒绝为 stale_auth_plan，无新 job，旧 plan bytes 保留；这是 CLI 拒绝证据，不是 App 承接旧 approval UI。旧 App 密文另由独立 HOME/state 的实际最终 CLI inspect/read/list 成功读取四件及原指令，jobs=[]；这也不冒充新 App reader 原生复验。上述证明跨版本状态与中断查询保留，不覆盖真实 Applications 替换或普通下载 quarantine 首开。
