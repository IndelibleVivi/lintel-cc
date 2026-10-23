# 当前状态

## 2026-10-24：未发布的恢复闭环与有界历史

当前源码新增下列修复；公开 App 仍为下文的 0.1.1，独立 CLI 仍为原 0.1.0 候选。本轮没有发布新 Release、激活 updater 或更新官网部署。

| 范围 | 当前源码事实 |
| --- | --- |
| CLI / DNS 等待 | CLI 使用 nonblocking bounded stdout；系统 DNS 由最多 32 个 slot 的独立 owner 管理，取消后的迟到结果不连接；不承诺能终止 OS DNS 或逃逸进程 |
| 浏览器恢复 | App 重开后从只读 pending IPC 发现原 host 安装批准并继续；未决 intent 拦住新安装。Popup 能完成原 owner 的隔离释放收尾；旧回执绑定原 instance，reset／重配对／worker 重开不借新身份发送。未知 legacy 归属保留并显式报限制，外部变更保留 |
| 残留／标识保全 | 保持 `lintel.residue/1` 原编码，共享有限预算、删除前生产 reader 校验、路径资格及逐原件隔离定位；同步不确定不记完成 |
| 任务与诊断 | App／named CLI 改用 256 KiB 有界摘要与分段详情，环境发现独立；cursor 绑定元数据快照／筛选／搜索，坏记录与超大单项明确说明。原 journal／去重记录不删，旧 full RPC 兼容；I/O reason 在 accepted 前后、落盘与重开查询一致。Bridge 错形 JSON 返回 invalid_state 并保留原字节，32 MiB 读写准入一致 |
| 首用／构建／CI | App/CLI 当前指引分开维护；显式 locked dependency/rust-docs 准备并检查前置工具；打包脚本、网站指南输入与 macOS signature fixture 接入对应 CI，frontend 执行与证据同用按平台过滤的清单；日常 push 不取消上一轮验证。Ubuntu native 临时 runner 优先官方 Canonical HTTPS mirror，保留依赖与严格 gate。Vite 开发依赖更新到 8.3.5 安全补丁。完整冷缓存 native 构建尚未独立验收 |
| 本地候选 gate | 新 `prepare-candidate.py` 固定 clean SHA、fresh source／target、三平台 CLI 与可选 App／DMG，并核对最终包字节、组件／许可与同 SHA canonical full CI。当前只完成 source 与合成回归，实际新候选／冷构建未验收；本地 receipt／CI JSON 来源未独立认证 |
| 分发与许可 | tar inspector 检查传递／循环 symlink，正式签名环境约束到达最终 gate；新 App/CLI 候选携带独立项目许可。原创功能代码 SUL-1.0、原创文档／独立图示 CC BY-NC-SA 4.0，范围／排除见[授权边界](../LICENSING.md)；项目为 source-available Preview |

本轮定向证据包含：App host 三发布阶段丢弃前端对象后从公开 IPC 重开恢复；真实 framed native-host 与生产 background／popup 的 lost ACK→reset→重配对→重开组合；逃逸进程 readiness 与实际 operation lock 释放；32 个真实阻塞 resolver owner 加 100 次饱和拒绝及迟到结果无连接；完整 Engine 残留 rename／sync 故障重开；accepted 前后有限 I/O reason 与 bridge 错形保留。旧轮的 24 MiB 生产残留归档→reader→恢复、逐原件隔离定位证据继续按其 source 范围保留。公开旧版 CLI → 当前不同 binary 的原 job／plan、旧加密包精确正文、选中字段恢复及实际 kill 后原 ID 不重放已通过。它们均使用 synthetic roots／adapter，不是个人认证、正式浏览器或 clean-Mac 验收。本轮检查范围见[验证说明](verification.md#2026-10-24-恢复与有界历史接续)；旧轮证据见同页 10-23 节。

上一轮准确源码 `4de7e54122f2e8dd350058c3944e9e9e5a5fbdeb` 的 [CI 39839654207](https://github.com/IndelibleVivi/lintel-cc/actions/runs/39839654207) 已完成 full profile，14 个 jobs（含 aggregate、两平台 native／frontend／heavy／browser UI／CLI 与 Linux VM）全部 success。下载的 12 份 canonical evidence 均为该 clean revision，所有选中 checks 为 passed；Ubuntu native 的官方依赖下载和后续 16 项检查均实际通过。本段仅是上一轮源码验证记录，不能覆盖本轮新改动，也不形成新 App／CLI 发行或官网部署。

有界历史的独立 debug 观察：10k 项首页约 225 ms，100k 项约 5.54 s，响应均约 15 KiB；100k 测试进程（含 fixture 建立）的最大 RSS 约 110 MiB。保留元数据扫描上限与显式加载状态，没有据此宣称恒定时间或无限容量。

仍需独立建设／验收：实际新候选的完整构建／产物 gate、正式 RC 跨平台升级／回退及 clean-Mac 信任／首装；物理 journal 归档先满足旧版去重／downgrade 兼容政策。Browser host 仍保留 1000 操作上限，满额可查原 ID，尚无安全归档／清理入口。完整健康／恢复中心、Client Footprint 的来源解释→精确修改→中断核对→快照→恢复旅程，以及外发／权限来源解释保留为下一产品建设；受管强隔离与可恢复退场独立试点，不阻塞当前维护修复。私密安全报告渠道与可复现演示仍未交付；完整 SPEC 不因此缩减。


2026-10-20 · **[Lintel 0.1.1 开发者 Preview 已发布](https://github.com/IndelibleVivi/lintel-cc/releases/tag/v0.1.1)**，修复 0.1.0 DMG 的 App bundle 签名封装。App／文档编译 revision 与 tag target 均为 clean `ebbe237cd236a2b299281e070b4758a9e9e522b2`。DMG 13,123,475 bytes，SHA-256 `867ea626ceb5101006e15c071da9627db38a555212208a4fffd577a5f5faf6d3`。Release 于 `2026-10-20T09:39:22Z` 公开，4 个上传资产均已核对公开字节；manual 记录为 `public_bytes_verified`／`publication_verified:true`。Chrome 从公开 Release 下载的副本保留 quarantine，只读挂载后的 App 通过严格签名校验。完整 ad-hoc 签名不提供 Developer ID 或公证，也没有 updater feed；干净 Mac 首装与用户信任确认仍独立。字节一致不构成源码认证。

独立三平台 CLI 候选为 `0.1.0-candidate-fcae6cb1efb5`，macOS arm64、Linux x86_64／aarch64 三包及两份 static musl runner 仍由原 [v0.1.0 prerelease](https://github.com/IndelibleVivi/lintel-cc/releases/tag/v0.1.0) 提供；包名保留原候选身份，Linux 为 experimental。10-20 发行整理版在不改变任何 executable 字节的前提下，为各 CLI 包加入 18 份与已核验 App 相同的第三方声明及校验覆盖；此前未含声明的本地包保留作历史测试候选。两份 Linux runner 在 10-19 用 X9 上的隔离 cargo-zigbuild 0.23.4／Zig 0.16.0 重建；App 内 runner、browser host、16 个扩展文件均与 manifest 的大小／摘要一致。实际 macOS CLI 交付包已解出，在新 HOME／state 逐字节读取三份原文且没有来源 job；这不证明其他架构 runtime。

[官网](https://lintel.page/) 于 2026-10-21 上线双语 SEO 与 Search Console 验证标签，保留 0.1.1 下载与安装指南。当前 payload 来自 clean `8f8d7d2113b2e1dcb1789d9effe5e43a6c5fb155`；[production 79f556f9](https://79f556f9.lintel-aue.pages.dev/) 与 [preview f4ee45e3](https://f4ee45e3.lintel-aue.pages.dev/) 使用同一 67 项有限 ZIP，经现有 Pages project 的 Dashboard Direct Upload 发布。它只比同日 `46cf22e` SEO 包在中英首页 `<head>` 各多一行 Google 提供的公开所有权标签；全部 40 页 body 与其余 64 个公开文件逐字节保留。

正式域名 40 路均为 HTTP 200：10 页 HTML 原样一致，30 篇指南仅消费两条自有 `email_off` 注释，正文与技术标识保持原字节、无邮箱解码脚本。首页验证标签、下载／来源记录、robots／40 项 sitemap、实际脚本、CSS 与媒体均匹配冻结包；未知路径及两条 updater feed 为真正 404，视频 Range 返回 206。0.1.1 下载记录与既有公开 URL 原字节保留，本轮没有重发 Release 或重新下载 DMG；此前公开下载与 quarantine 验收仍属于 10-20 的证据，没有启用 updater。

390px 中英日夜、语言／history、首页 29 个请求无第三方 HTTP／加载失败，以及两环境影片完整播放到 20.01 秒、原生 seek、字幕加载／可见 cue、关闭暂停／归零／焦点返回的浏览器证据来自同日 `46cf22e` 首轮发布。本次追加验证标签后，本机 canonical `app-release-test` 与真实媒体 `site-ui` 再次通过，且线上 HTML／资源字节复核通过；不把前一包的手动浏览器观察冒认为新包重测。

先前 `fcae6cb` 已完成真实 native 首用、归档、退出重开、独立阅读与 CLI 交接；配置恢复和残留清理的完整 native 操作先在 `e822d74` 完成，下面保留各自的 source 范围。`878482f`、`bb9fa824` 均为较早候选，见[候选历史](candidate-history.md)。

## 官网 SEO（2026-10-21，已上线）

40 个中英静态页面的 title／description、Open Graph／Twitter 字段与分享图说明已统一维护；首页搜索标题明确 Claude Code 环境与资料管理身份，原可见标语与正文保持。首页仅投影真实 `WebSite`／本语言 `WebPage` JSON-LD，语言／history 切换同步；sitemap 的 40 项均由同一路由表生成，并保留中英及 x-default 配对。PNG favicon 逐字节复用既有 App identity export。生成检查覆盖页面、JSON-LD、favicon 与 sitemap 漂移，次级页继续走完整静态 mirrors。

本机 canonical `site-game-test`、`app-release-test` 已通过；更新搜索标题的既有断言后，`site-ui` 完整通过，使用现有真实影片，包含无 JavaScript 阅读、语言／history 元数据、原有实例与交互。完整 CI gate 通过后，按当前授权依次发布冻结包到 preview 与 production，并在正式域名复验全部 40 路 canonical／语言配对、准确标题／摘要、分享 metadata 与首页 JSON-LD；preview 保留 `noindex`，正式域名没有继承它。Search Console 接入与 Google 观察见下节；新 SEO 标题在实际搜索中的展示与排名仍未验证。生成与上线程序见[网站源码说明](../apps/site/README.md)及[搜索入口核对](site-hosting.md#搜索入口与上线核对)。

## Google Search Console（2026-10-21）

按本轮明确授权，已在既有账号添加 `https://lintel.page/` URL-prefix property，经 canonical 首页的 HTML meta tag 验证；Google 设置页确认“您是经过验证的所有者”。[sitemap.xml](https://lintel.page/sitemap.xml) 已成功提交，Google 的站点地图报告显示“成功”、已发现 40 个网页。所有权标签须随以后的 payload 保留；本次没有添加 analytics、写 DNS 或修改用户／服务权限。

| 已检查 URL | Google 已存索引报告 | Google 选择的 canonical | Google 实时测试 |
| --- | --- | --- | --- |
| `/` | 已收录 | 所检查的网址 | 抓取成功，允许抓取与索引 |
| `/en/` | 已收录 | 所检查的网址 | 抓取成功，允许抓取与索引 |
| `/guide/quickstart/` | 已收录 | 所检查的网址 | 抓取成功，允许抓取与索引 |
| `/en/guide/quickstart/` | 尚未收录；Google 当时无法识别此网址 | 尚未确定 | 抓取成功，允许抓取与索引；声明 canonical 为自身 |

以上只覆盖这四个 URL；sitemap 发现 40 页不等于全部已收录。已存索引报告与实时测试是不同数据，不能据此前三条认定新 SEO 标题已经刷新搜索结果。中文首页的手动“请求编入索引”连续两次未被接受：Google 页面提示无法连接 reCAPTCHA 服务，按提示完整刷新后仍同样返回。本次没有解决 CAPTCHA、改浏览器／网络安全设置或继续对其他 URL 重复提交；该手动入口受阻，所有权与 sitemap 的成功结果独立保留。新 property 的汇总报告仍在处理数据。

## 0.1.1 修复与下载验收

0.1.0 的下载／安装副本与原 DMG 字节一致，但只有 `adhoc,linker-signed`、未绑定 Info.plist、没有资源封装；严格校验报 `code has no resources but signature indicates they must be present`。这是包装缺陷，原 tag 与 8 个资产保留，旧 Release 顶部已指向修复版；详细旧字节与发行事实见[候选历史](candidate-history.md#preview-0-1-0-release-history)。

0.1.1 在 canonical `desktop:build`、本地 Preview 和正式 release build 的同一入口完成组装、签名与 App gate；本地无配置才补 ad-hoc，正式入口要求 Developer ID。新 DMG 自校验、只读挂载及 App 47 个文件／链接与冻结候选一致；严格校验通过，Info.plist 已绑定，资源封装 v2／44 文件。参数／签名优先权、准确 target 输出、旧 linker-only、资源／plist 篡改、有限 symlink 与发行打包回归通过。

从新 DMG 取出的真实 native App 在独立 synthetic HOME／state 打开：首页、0.1.1 版本、clean docs revision 和 updater 未配置状态均已观察。Chrome 的本机 HTTP 下载保留 quarantine，macOS 随后因未公证拒绝启动；系统设置明确显示 Apple 无法验证和 Open Anyway，未点击信任覆盖。公开 GitHub 下载另行核对了实际字节、quarantine 与内部 App 严格签名，未冒认为干净 Mac 启动通过。旧安装副本已在明确的单 App 信任例外下恢复运行，全局 Gatekeeper 仍开启；该本机例外不随发行包传播。

## 当前候选验收

| 范围／所测 source | 10-19／10-20 fresh 观察 |
| --- | --- |
| native 首用与重开（`e822d74`） | macOS 26.5.2 arm64；新 synthetic HOME／state 的真实 release WebKit 空白登记、精确三件选择（排除第四份）、无密文时预览、批准一次、原 job completed；六份原件逐字节不变，无新 root。Quit 后原进程退出，同 state 重开保留中文与环境；记录页仍有同一原 ID completed |
| 工作包与正文（`e822d74`） | 重开后从独立加密文件入口重新输入口令，恰有指令／记忆／会话三件；原文、结构化消息、真实 byte offset／content block 与本地派生交接稿已实际观察。另一独立 HOME／state 的 CLI 无来源 job，三份完整原文逐字节相同 |
| 配置修改与恢复（`e822d74`） | native 自定义空草案显示零待改且禁用预览；一项关闭的 before/after、批准、completed 与 settings 读回通过。原任务的独立恢复将该项从 0 恢复到 1，保留两处后来编辑；未运行 Claude、不证明客户端实际加载 |
| 有限本地状态与 Finder（`e822d74`） | native 元数据扫描与 Finder 实际定位合成日志通过；精确 1 文件／71 B 先加密备份、再清理 completed，独立文件入口重新解锁，单独预览并批准恢复原路径，字节一致；工作资料、占位凭据和后续配置编辑保留。身份文件不存在时明确说明 |
| 最新 native 首用与阅读（`fcae6cb`，10-20） | 新 synthetic HOME／state、含空格路径：空白登记、精确三件／521 B、批准前无密文、一次批准 archive-only completed；六原件不变。Quit／重开保留环境与中文，原 ID 仍 completed；独立文件入口读回三件原文、会话结构与 byte offset 0／198。实际交付 CLI 在另一空 state 逐字节读回三件，无来源 job |
| 新包资源与日常入口（`fcae6cb`，10-20） | DMG／全 App／executable 与资源字节核对通过；帮助中的 clean docs revision 及实际打开链接均为 fcae6cb。中英原生菜单、Day／Night／System、动态默认、风景册及跑道跳跃／暂停／重试通过；手动更新检查明确返回 updater_unconfigured，默认后台检查关闭 |
| 原生 CLI 交接（`fcae6cb`，10-20） | 实际交付 CLI 的本机临时副本字节相同；静态核对返回候选匹配、同 HOME／UID／state，审阅后复制成功且无新 job。原外置宗卷路径读取曾停在 macOS 可移动宗卷隐私授权；未更改权限，该路径未验收，不把系统等待当作 CLI 子进程超时 |
| 本机回归 | desktop Rust 通过，2 项既有 ignored 不算通过；App matrix 20 项原为 19 passed／1 failed。失败来自 client-control-ui 在 React reinspection 重挂前跳过清草案，补真实挂载／清空等待后完整检查两次通过；保留原 empty-preview 断言并增加零改动行断言。原失败报告仍保留，不标为全绿 |
| 官网与阅读路径（`d4a92dd`） | 五类路由、15 篇 paired 指南／40 locale 页；正式域名 HTML、元数据与实际 CSS／脚本读回通过，技术 @ 标识保留。中英 390px 日夜、真实官网按钮下载与新 DMG 摘要通过；同 payload 的 Preview 影片完整播放、关闭复位通过。前次网站完整交互与字幕／Range 验收仍保留其原 source 范围 |
| CLI 与资源 | 当前 native release CLI 的 canonical `cli-candidate` 打包、解包、实际执行、保留 state 升级及实际三平台输入 smoke 通过。两份新 musl runner 为 static ELF／正确架构；其他架构实际执行仍是独立验收，不能从格式检查推断 |
| 发行与实机边界 | 0.1.1 App 的 4 资产公开并读回，官网 production 已切到同一新 DMG；原 0.1.0 的 8 资产与 CLI 保留，旧 App DMG 标注替代。完整 ad-hoc 封装与 native synthetic 启动通过；Preview 的单 App 信任、干净 Mac、真实认证与生产 browser／SSH 验收分别保留 |

## 能力与验证 owner

App／CLI 共用 core、有限 SSH 与原任务查询；v5 的 27 项保护字段、客户端配置／残留／身份控制、精确工作选择、流式加密包、独立阅读、有限启动来源、systemd、browser、IPv4／IPv6 与遥测已在源码接通。六任务合同与完整目标分别由 [product baseline](specs/product-baseline.md)、[SPEC](SPEC.md) 维护，不能把源码支持扩大为所有平台实效。

GitHub Actions 是主要 CI owner；`tests/ci_plan.py` 选择有限 changed-path profiles，`tests/verify.py --checks` 执行并输出 canonical evidence，aggregate 拒绝选中但 skipped 的检查。隐式默认组保持原样；真实 browser startup、native WebKit、Linux runtime 与大包容量为各自独立证据。0.1.1 App source／tag target `ebbe237` 的 [GitHub CI 39399056140](https://github.com/IndelibleVivi/lintel-cc/actions/runs/39399056140) 已完成，14 个 jobs（含 aggregate、两平台 heavy 和 Linux VM）全部 success。SEO source `46cf22e` 的 [CI 39567599162](https://github.com/IndelibleVivi/lintel-cc/actions/runs/39567599162) 也已 14 个 jobs 全部 success。当前验证标签 payload source `8f8d7d2` 的 [CI 39583906342](https://github.com/IndelibleVivi/lintel-cc/actions/runs/39583906342) 已完成；选中的 plan、control、Ubuntu frontend／browser UI 与 aggregate 全部 success，native／heavy／runner／Linux VM 未选中而跳过，不计为通过。没有接入自动部署，以上 production 为本次明确授权的手动激活。

逐轮 source、旧候选资源、CI 和隔离 runtime 证据已移入 [候选历史](candidate-history.md#2026-10-19-归档的阶段证据)，[基线验收](specs/product-baseline-status.md) 保留 A01–A18；具体命令和证据边界见 [验证说明](verification.md)。历史通过不会把本轮未测层标成通过。

## 入口与权威

| 读者／任务 | 权威入口 |
| --- | --- |
| 首次合成试用与英文入口 | [中文／English 文档入口](index.md)、[English documentation](index.en.md)、[中文 quickstart](quickstart.md)、[English quickstart](quickstart.en.md)、[English README](../README.en.md)；公开源码构建、v0.1.1 App 与原 v0.1.0 CLI 手动下载 |
| 人类任务、保全／重建、继续工作、原任务核对 | [操作指南](operator-guide.md)、[English](operator-guide.en.md) |
| 独立 CLI、用户候选安装、stdin、schema 与 Agent | [Agent CLI](agents.md)、[English](agents.en.md) |
| 前端构图、组件入口与 Selen 后续工作 | [桌面](desktop.md)、[视觉身份](visual-language.md) |
| 关联组件、保全与完整收尾的后续实现 | [环境覆盖与使用收尾方案](specs/environment-completion-plan.md)；组件清单、容量预检、精确选择、流式包与有限启动来源已接通源码；更多资产、独立完整退场、IDE/Desktop 和发行保留依赖与验收 gates |
| App 自身更新／本地发行准备 | [App 更新](app-updates.md)；源码可用、发行配置未激活，实际首装／升级独立验收 |
| 当前 native／browser／Linux 验证边界 | [验证说明](verification.md)、[验收记录](specs/product-baseline-status.md) |
| 官网、示例与 opt-in 共享游戏 | [正式官网](https://lintel.page/)、[源码说明](../apps/site/README.md)、[托管程序](site-hosting.md)；本轮正式域名 40 路、HTTPS／headers、手动下载与手机日夜已验收；同包 Preview 影片完整播放 |
| core／SSH／systemd／browser／network | [core](core.md)、[remote](remote.md)、[services](services.md)、[browser](browser.md)、[network](network.md)、[telemetry](telemetry.md) |

统一验证 owner 是 `python3 tests/verify.py`；UI／browser startup／Linux runtime 为独立 selections。dirty 开发 App 的 docs revision 为 unknown；干净候选的 Lintel 文档资源指向自己的源码 revision，main 入口标为最新开发说明；独立 Infra Field Guide 使用自己的 main，标为独立指南最新版。不会开放任意 URL 或 shell。

仓库已公开；当前源码的原创功能代码适用 SUL-1.0，原创文档／独立原创图示适用 CC BY-NC-SA 4.0；品牌、Clawd 与未核实素材排除，第三方保留原条款。此前发行包不自动改授许可，具体范围和生效边界见 [LICENSING](../LICENSING.md)。

## 完整目标仍缺少

1. **真实认证与写入者控制：** Keychain/共享 profile、Desktop/IDE/service 的完整定位与生命周期；有限 systemd 暂停已接通，其他 supervisor 暂停、重新登录及旧会话续用。官方注销已加入观察主体与状态绑定，并拒绝缺失主体的预览；同账号 Keychain 内部 token 更新如果不反映到 CLI 状态，仍无法识别。当前识别部分 Claude 进程及可访问 manager 的直接 root 绑定服务；明确要求目标写入者先停，不能归属的进程阻止清理。官方 auth 命令仅有合成 CLI 证据，不能宣称生产注销已验收。
2. **完整外发策略与强约束：** v5 的 27 个字段与逐项编辑已接入；真实客户端入口实效矩阵、macOS Network Extension 签名与权限、Linux namespace；直接 socket、UDP、DNS、NO_PROXY、子进程不能由代理覆盖证明。
3. **远程运行：** 用户级安装／版本绑定已接入候选；Ubuntu 隔离 OpenSSH 已验证安装／丢 ACK 核对／原任务查询／交互 PTY，真实 VM 已完成有限 system-manager 暂停恢复、符合既有条件的 system/user manager 严格 logout 后续跑、原任务完成与 reboot 后核对。setsid 不保证 logout 存活，托管路径也不保证 reboot 续跑；生产 VPS 安装与任务实跑、aarch64 runtime、其他 supervisor 和完整 G03 仍未验收。
4. **浏览器旅程：** 独立 Chromium 临时测试 profile 已完成真实持久安装／onStartup 两阶段清理；仍需正式 Chrome、Edge、Firefox 与 AdsPower 的安装、配对与清理验收。Firefox 独立 CacheStorage、按站点 proxy、容器后台停写、克隆识别和专用 browser 启动仍有缺口。
5. **发行与维护：** App updater 与本地 release／feed／首装资料准备已接通源码，手动开发者 Preview 已发布，签名 updater 未激活；0.1.0 bundle signing 缺陷已由 0.1.1 修复并公开；未公证 Preview 的用户信任确认、干净 Mac 首装／升级与正式签名/公证分层验收。菜单栏、定时漂移、签名规则更新、卸载与性能预算继续保留。
6. **并发与恢复：** 不合作的外部编辑器或已持有文件描述符的 writer 没有 OS 级 CAS；不确定副作用需核对。混合状态加密备份不支持自动恢复；会话/记忆迁入不证明可以续聊。

这些是原始完整目标的差距，不是缩小后的新 SPEC。下一关口是正式浏览器／AdsPower 与非开发者安装、真实认证／入口矩阵、生产 VPS 任务验收。隔离 runtime 与合成数据能证明机制，不能替代真实产品场景；现有个人登录和生产服务不作开发 mutation fixture。
