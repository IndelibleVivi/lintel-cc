# 开发候选历史证据

## 2026-10-13 cdb0ba8 本地 DMG

后续 `878482f` 仅修正 notices 的 clean-checkout 构建接线并重封包；以下完整 native 交互仍对应本候选，未冒充后续包的逐项复验。

| 对象 | 已观察证据 |
| --- | --- |
| 本机 Native 构建 | clean `cdb0ba8` 的 `npm run desktop:build` 完整入口通过，保留 beforeBuildCommand；随后 `tauri bundle --bundles dmg --ci --no-sign` 完成 DMG。Rust 1.98.0；两种 musl runner 使用既有 Zig 0.16.0 重新构建并通过 static ELF 检查。版本仍 0.1.0，identifier／未配置 updater 合同不变 |
| 资源与包装 | App 45 文件／36,297,243 bytes；DMG 12,204,865 bytes。hdiutil verify、只读挂载、复制件文件字节／权限、native executable 与全部 canonical resources 一致；17 份 notices 含 frontend、387 个 Rust crate 的目标依赖超集和当前 Rust／Zig 工具链说明，不是 exact linked SBOM。en／zh-Hans 与 clean docs revision 核对；只有 linker ad-hoc signature，无 Team ID、资源签封或公证。候选与证据不进 Git，旧包保留 |
| Preview 试用入口 | 从本轮 DMG 取出的 release App 在隔离 HOME／CFFIXED_USER_HOME／state 下实际运行：System English → 简中 → System English，页面与原生菜单同步；版本 0.1.0、空发行配置与 exact-source 文档链接可见；共享游戏运行、碰撞结果、重试和 P 暂停已观察。jobs 为零，测试 App 已停止。此证据不替代 quarantine／Gatekeeper 首装、完整 native 操作矩阵或真实认证；quickstart 不变，无公开 Release 下载 |

本页保留历次阶段与候选（包括 `a239123` 与独立 CLI 候选）的验收记录，是历史快照。下文的“当前”“本轮”均指对应旧候选；不能据此推断后来 checkout 或 App 包所含资源。正在使用的候选、资源对应关系与未完成目标以 [current-state.md](current-state.md) 为准。

2026-10-05 · 0.1.0 开发候选。完整 [SPEC](SPEC.md) 仍未交付，四条完整旅程 G01–G04 尚未通过。源码与本地 macOS App 已构建；未安装到 Applications、未正式发布、未部署真实远端。

项目仓库已于 2026-10-04 公开，保留原 main 历史；GitHub PUBLIC 与匿名 Git／README 读回已核对。当前仍未选定项目原创材料的公开复用许可证，第三方权利不由 Lintel 重新授权。源码公开与正式发行、完整验收是不同状态。

## 本地 DMG 准备与复验（2026-10-13）

clean `cdb0ba80426bcd819106844b026e280fed769d0c` 的 App／DMG 为本轮本地候选，包含 cleanup 检查改为 metadata-only、第三方 notices 与最新共享游戏。默认验证原始 25 PASS／3 FAIL 后分别完成 workspace 串行、native 真实 target 路径与 release 大工作包复验；独立 18 项 UI／browser 也均取得通过结果。没有把原失败报告改成成功，debug 大包的30分钟运行预算仍是已观察限制。当前包装与 native 证据见 current-state。

此前 clean `e1a2df4ade484a42a438a4bf09d225d4b7a1a78c` App 的包装与观察保留如下；它已被 cdb0ba8 本地候选替代，旧文件未覆盖：

| 对象 | 当时事实 |
| --- | --- |
| 本机 Native 构建 | clean `e1a2df4` 的 canonical desktop-tauri-bundle 完整入口通过，152.576s，保留 beforeBuildCommand。没有改 updater／identifier／发行配置；此前 `47158cd` 与旧候选的构建观察见候选历史 |
| 资源与包装 | clean `e1a2df4` macOS arm64 App 为 30,596,543 bytes（约 29.18 MiB），zip 为 11,880,053 bytes；复制件及 zip 解包的 28 文件逐字节／权限一致。内置 x86_64／aarch64 runner 为 5,895,888／5,196,336 bytes，均嵌入当前完整 task catalog；host 789,392 bytes，Chromium／Firefox、ICNS 与 canonical resources 一致，无 woff2。全部资源也与此前 `47158cd` 包逐字节相同，沿用其 fresh 编译／2 passed 的实际包内 host framing／allowlist 与扩展合成安装证据；本轮没有冒称重跑安装检查。clean docs revision 与 bundle en／zh-Hans 已核对。只有 linker ad-hoc signature，无 Team ID／资源签封／公证；候选、报告与测试数据不进 Git |
| Preview 试用入口 | 最终 `e1a2df4` release executable 在显式 synthetic HOME／CFFIXED_USER_HOME／state 下实际显示 EN 顶部入口，偏好仍为 Follow system；原生三选项与 English 界面／菜单已观察，没有 dev server。输入工具检测到真实鼠标介入后停止，native 手动→System 选择仍未验证；三份 synthetic 原件未变、jobs 为零，QA App 已停止。旧 `47158cd`／`25a8649` 点击限制与更早归档／阅读旅程保持独立。quickstart 与双语源码入口不变，无公开 Release 下载 |

## 跑道扩展候选修订（2026-10-13）

clean `78a5de52eb3a18970582cabe98d8a488fed286e3` 的彩色三场景／松手蓄力方案曾通过 25 项逻辑、网站与 built-App Chromium 检查，有限网站包为 24 文件加 manifest／26,711,348 bytes；该包没有成功上传。此实现随后退休，视觉与手感接受不能从通过检查推断。

clean `a064c86580690ccd9a48a26eaa60c06e74332910` 改回原主题的单色像素山线／云／日月，树林与庭院只增加同一风格的标记；按下立即起跳，短按原高度、按住在同一次上升中加高，松手不补跳；初速由 210 调至 260px/s。26 项逻辑、实际影片网站 UI、frontend build 和 build 后 App UI 通过；隔离 Chromium 开局日／夜 canvas 与原版 `65495b7` 逐像素一致。有限包为 24 文件加 manifest／26,708,523 bytes，源码与媒体逐字节核对，同包 ZIP 已准备；本地复看已显示新版，尚未上传或重建 native App。最新候选与部署状态继续以 current-state 为准。

clean `c80a783c8755bc6a76de6870ba95ff739d9d57e9` 进一步将普通星移入轻跳轨迹，减小高度扰动，并随较高障碍上移；起跳物理、速度、背景、碰撞判定和 +50 高星不变。新增固定正常起跳窗口与连续跑道的普通星全收检查，在旧规则上能复现安全越障却漏收，修订后 27 项逻辑通过。真实键盘输入的受控 390px Day／320px Night seeded 跑道各收下连续六颗星（含高星）；网站／build 后 App UI 通过。夜景测试重载后先滚动可见画面、等 lazy raster 载入，再检查角色热区与原像素断言，避免等待隐藏热区阻止图片载入；产品懒加载与热区行为未改变。新包为 24 文件加 manifest／26,708,586 bytes；用户在本地试玩后明确认可，随后同一 25 条目 ZIP 经 Dashboard preview 后更新生产。实际部署与验证见 current-state；native App 候选仍未重建。

## 能力开场官网（fdf18b2，2026-10-12）

clean `fdf18b234c4ed45f756b4ab7d64a76ff076ce3e9` 的 24 文件 payload 加 manifest（26,702,191 bytes）曾部署为 production `e7f12076-e58e-4c67-96b3-6ee351880e91` 与 preview `d6c7a45e-1301-44ad-b623-831e2b7ea611`，保留双语、标题图标＋交错场景并协调能力区开场。两路正常缓存重载取得版本化 CSS，字节与冻结包一致；其 hold/release 跑道现已被上方 c80a783 的按下即跳／按住加高版本替代。旧包与此前同媒体的真实播放证据保留，当前部署以 current-state 为准。

## 双语组合官网（427e981，2026-10-12）

此轮随后由 clean `fdf18b2` 加入能力区开场协调与版本化样式入口；以下“尚未上线”仅是当时缓存修正的状态，最新状态见 current-state。

| 对象 | 历史证据 |
| --- | --- |
| 官网托管 | Direct Upload Pages project `lintel`，production `027679d0-6f0c-444c-921b-424c47e856bc`，preview `5302a610-62b4-45f0-914d-273328a690a7`，均声明 clean source `427e981be402e46148ca5bf5de78353acfb82c11`；同一 24 文件有限 payload 加 provenance manifest 先 preview 再生产，媒体与原审阅字节相同，无下载／feed。正式 [中文](https://lintel.page/)／[English](https://lintel.page/en/) HTTPS 返回 200，六标题 icon＋六场景、指南／canonical／无横向溢出／蓄力控件已观察；实际取得的 runner 与冻结包逐字节一致。影片通过原生控件播到 20.01s（1920×1080），关闭暂停且归零。回访浏览器仍拿旧 CSS；临时禁用缓存观察到服务器新 CSS 与冻结包逐字节一致，实际 CSS header 为 max-age=14400，debug 设置已还原。canonical 两样式入口已加版本 query 并生成 English，本地四项语言检查通过，修正尚未上线；不把诊断中的 cache-disabled 状态称为正常回访通过。此前 `47158cd` 证据见 [候选历史](candidate-history.md)；程序见 [site-hosting](site-hosting.md) |

## 官网短直腿部署（47158cd，2026-10-12）

此部署后来被 clean `427e981` 的官网双语／组合能力场景／蓄力版本替代；旧有限 payload 保留。下行只记录当时线上观察，不证明后续 revision。

| 对象 | 历史证据 |
| --- | --- |
| 官网托管 | [lintel.page](https://lintel.page/) 已更新；Direct Upload Pages project `lintel`，production deployment `f5525e87-87c7-47cb-9ed7-1cf77d992888`，preview `131dcc49-4656-4f0f-927a-a7766dc7f316`，均声明 clean source `47158cda045d03d2ffb3fb8210a2f2ac3ab503d4`。同一 21 文件有限 payload 先验 preview 再上传生产，影片／海报与原审阅字节相同，未加入下载或 feed。正式域名 HTTPS 页面正常加载，浏览器实际取得的 clawd-game.mjs 与当前源码逐字节一致：新 STRIDE_TRAVEL 短腿步态在场、旧 whole-body bounce 已退出；游戏启动／跳跃／暂停、390px Night 的继续／暂停与无横向溢出、无相关 console error 已核对；正式域名影片通过原生控件播放到 20.01s（1920×1080），关闭暂停并归零。旧实际线上脚本与 `6c4e5df` 完全相同，确认是部署落后。shell／web 直读受到 Cloudflare 1010／浏览器 manifest 直达受阻，本轮没有冒称重新读回公开 manifest、DNS／RUM 或完整首次上线矩阵。原先影片／headers／手机验收保留于候选历史；本轮实际媒体 site-ui 通过。更新程序见 [site-hosting](site-hosting.md) |

## 首次双语候选（47158cd，2026-10-12）

以下保留入口简化之前的本地 App；它被 clean `e1a2df4` 候选替代，旧包保留。当时官网使用 `47158cd` 的独立静态 payload，App 替代关系没有改变该次生产部署；后续官网更新见 current-state。最初双语实现及 dev native 的六项语言旅程属于 `65495b7`，manual→System 的 native 选择尚未验证。

| 对象 | 历史证据 |
| --- | --- |
| 本机 Native 构建 | clean `47158cd` 的 canonical desktop-tauri-bundle 完整入口两次通过：初次 252.294s；更新 runner 资源后的最终构建 112.076s，保留 beforeBuildCommand。没有改 updater／identifier／发行配置，也没有将旧候选的 timeout 标为通过；旧构建成本见候选历史 |
| 资源与包装 | clean `47158cd` macOS arm64 App 为 30,596,543 bytes（约 29.18 MiB），zip 为 11,879,150 bytes；复制件及 zip 解包的 28 文件逐字节／权限一致。内置 x86_64／aarch64 runner 为 5,895,888／5,196,336 bytes，均嵌入当前完整 task catalog；host 789,392 bytes，Chromium／Firefox、ICNS 与 canonical resources 一致，无 woff2。实际包内 host framing／allowlist 与扩展合成安装两项检查为 fresh 编译／2 passed，clean docs revision 与 bundle en／zh-Hans 已核对。只有 linker ad-hoc signature，无 Team ID／资源签封／公证；候选、报告与测试数据不进 Git |
| Preview 试用入口 | 最终 `47158cd` release executable 已在显式 synthetic HOME／CFFIXED_USER_HOME／state 下观察 System／English 首页与原生菜单；没有 dev server。后台控制接口曾返回空图／空树，另一个原生观察接口能读到完整 UI，不能据此判为产品启动故障。native 手动→System 点击尚未验证；旧 `25a8649` 点击限制及更早归档／阅读旅程在候选历史，不归给新候选。quickstart 与双语源码入口不变，无公开 Release 下载 |

## Selen 美化与本地 App（25a8649，2026-10-11）

以下保留被双语候选 `47158cd` 替代的 App／资源观察，以及更新之前的官网部署；各行的“本轮”仅指当时检查。旧包仍保留，未覆盖安装。`25a8649fb313eeac159503801a392ee6cb969e12` 包含 IPv4／IPv6 Leaf、遥测控制／App 更新准备与 Selen 美化，没有后来的 App 简中／English。新候选和实际官网以 [当前状态](current-state.md) 为准。

| 对象 | 历史证据 |
| --- | --- |
| CLI 与分层验证 | 已包装 App 候选源码 `25a8649` 的分组 CI 与 aggregate 已通过；本机 fresh frontend build、选中主题／主按钮的真实 Tab 与旧样式反例、moon／snack、reduced motion、Escape／卸载和短脚风景检查通过（built Chromium／合成 invoke）。独立 CLI 发行候选仍是 `bb9fa824`，其历史解包／首次试用证据见候选历史；本轮没有重发 CLI 包。App 内两架构 Linux runner 已从 clean `f611911` 重建，与最新 App 的相关 Rust source 一致；格式与包内字节一致不等于其它架构 runtime |
| 最新 CI 观察 | Selen 三条源码 `7a1f975`／`7b3e0a8`／`f611911` 的 runs 37961931825／37962431346／37967621633 已核对 success。主按钮焦点修复 `759a9a0` 的 [run 38104607685](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38104607685) completed/success；最终选中态修复 `25a8649` 的 [run 38110577397](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38110577397) completed/success，触发至完成约 2m55s。这是分组 source CI，不能代替 native WebKit 点击／安装或正式发行；更早 CI 的成功与失败快照保存在候选历史 |
| 本机 Native 构建 | canonical desktop-tauri-bundle 首次在 1800s 时限内未完成，记录为失败。确认进程已停后，使用仅跳过已完成 beforeBuildCommand 的本地 override 复用已核对的前端／host／扩展／runner 资源；两次 raw Tauri 构建均 exit 0，最终 clean `25a8649` release 编译 73m05s。没有改 updater／identifier／发行配置；这是本机构建成本，不是分组 CI 的耗时或 canonical timeout 已修复的声明 |
| 资源与包装 | clean `25a8649` macOS arm64 App 为 30,465,374 bytes（约 29.06 MiB），zip 为 11,806,522 bytes；复制件及 zip 解包的 28 个文件逐字节／权限一致。内置 x86_64／aarch64 runner 为 5,891,792／5,196,336 bytes，host 789,392 bytes，Chromium／Firefox 资源与 canonical generated resources 全部一致。实际包内 host framing／allowlist 与扩展合成安装两个检查通过；复用相关 Rust source 未改变的既有测试 executable，不宣称重新编译测试。ICNS 一致、无 woff2、clean docs revision 已核对。仅 linker ad-hoc signature，无 Team ID／资源签封／公证；候选、报告与测试数据不进 Git |
| Preview 试用入口 | 最终 `25a8649` App 在显式合成 Foundation HOME／core HOME／state 下启动，真实 WebKit 首页显示 Synthetic native Preview；WebKit cache／local storage 位于合成 HOME，六份原件／控制文件未变。本次 QA 进程已停止。原生点击接口连续报 elementHasNoFrame，刷新同一候选绑定未消除，未完成 native 归档／阅读／设置／日夜点击旅程；历史 `afa98f66` 的合成 native 旅程独立保留。quickstart 与双语源码构建入口不变，无公开 Release 下载 |
| 官网托管 | [lintel.page](https://lintel.page/) 已上线；Direct Upload Pages project `lintel`，平台 hostname `lintel-aue.pages.dev`，生产 deployment `291cc34b`。公开 `site-build.json` 读回 clean source `6c4e5df562550b37e9445b2d4fffe98a46be4493`；apex proxied CNAME、Pages domain／验证状态 active、正常 TLS 与 HTTPS 200 已核对。1440／390 隔离 Chromium 的正式域名检查通过：CSP 下 ES modules 挂载、日夜无溢出、初始及打开 dialog 不下载／不播放影片，主动播放实际前进、1920×1080／20.01s、14.4s seek、四条 captions、关闭复位与返回焦点、无 JS 正文及普通短片链接可用，页面错误与外部资源请求均为零。另核对 MJS／VTT MIME、分享图、robots／sitemap、真实 404 与视频 Range 206。项目无 Functions／Web Analytics 配置，zone RUM 已在 dashboard 完全禁用并复验无 beacon 注入；托管平台仍处理普通 HTTP 请求。不是 Safari／真实手机验收，不提供正式 App 下载；更新程序见 [site-hosting](site-hosting.md) |

## 八项修复与短直腿候选（afa98f66，2026-10-09）

以下保存被 `25a8649` 本地 App 替代之前的交付与资源身份。独立三平台 CLI 仍保留 `bb9fa824`；旧 App 的 native 归档／阅读／游戏验收不能自动归给新 App。旧段落中的“当前”“本轮”只指当时观察。

2026-10-09 · **Lintel 0.1.0 Preview / 开发预览**；main 源码新增遥测控制与 App 更新准备，最新已分发 App／CLI 仍是下述旧候选；新 Leaf 不由旧候选包含。本地 macOS arm64 App 已从 clean source `afa98f6634151324e27dd13ea354e8e72db6d689` 重建，包含本批八项 Preview 修复与连续短直腿步态；配套三平台 CLI 保持 `bb9fa824b1a2aa509e749519c5d700eba1bb02be`。源码分层回归、实际解包 CLI 首次试用与 `afa98f66` App 的合成 native WebKit 归档／会话阅读／锁定／游戏旅程通过；此前 `bb9fa824` App 已核对实际解包 CLI。后台启动曾出现真实空白，用户切到前台后恢复，同一未修改候选随后完成旅程；底层原因未完全隔离，不标为代码修复或完整原生矩阵通过。官网仍运行 `6c4e5df`；新静态 payload 已准备，未生产更新。没有正式 Release／签名／公证。A01–A18 保留 [产品基线](specs/product-baseline.md) 与 [完整 SPEC](SPEC.md) 的 G01–G04，真实认证与完整原生矩阵仍有具体未验证项。

| 对象 | 历史证据 |
| --- | --- |
| CLI 与分层验证 | 本批默认 26 项按 canonical category 分批通过：Rust 4、Python 3、JS 3、CLI journeys 15、desktop build 1（含 typecheck）。另有 built App 的 work／baseline／archive-wait／drift，site（实际电影）／clawd-app，以及独立 browser restart／pairing 通过。实际 release CLI 的 `cli-candidate` 包装／解包／独立 state import／升级保留原 ID 检查通过；另对最终 `bb9fa824` macOS archive 解包，在 repo 外且 PATH 仅系统工具时完成精确选择 3/4 原件、只归档、查询原 ID、独立 state inspect／逐字节阅读；六份原件／控制文件未变。Linux 本地为 static 格式／字节证据，不算 Linux runtime |
| 资源与包装 | clean `afa98f66` 的 macOS arm64 App 为 25,903,350 bytes（约 24.70 MiB）；复制件及最终 zip 解包的 31 个文件逐字节一致、executable mode 保留。双架构 runner 为 5,271,064／4,643,664 bytes，host 789,392 bytes，Chromium／Firefox 各 8 文件共 53,302／53,464 bytes，与 canonical resources 一致；ICNS 一致、无 woff2、clean docs revision 已核对。只有 linker ad-hoc signature，无 distribution Team ID／公证。三平台 CLI identity 为 `0.1.0-candidate-bb9fa824b1a2`，macOS／Linux x86_64／aarch64 archive 为 6,958,150／7,163,410／6,982,223 bytes；输入格式、canonical runner 与 checksums 通过。候选及测试数据不进 Git |
| Preview 试用入口 | 中英文 README／quickstart、官网 Preview 能力与试用导航、有限反馈表已接通源码。中文命令以真实 PTY 实跑：精确选择 3/4 原件、只归档、原 ID 查询、独立 state inspect/read，六份合成源文件字节未变、口令未回显、父 shell 不变。双语 17 个 shell blocks 一致，英文未重复独立 PTY 实跑。此前 clean `8e6448c` App 在独立 HOME/state 的真实 native WebKit 中选中三份原件共 572 bytes，批准并生成 age 工作包，App 解锁 manifest／会话并实际重新锁定，口令与正文清空。密文复制到另一独立 HOME/state 后 inspect/read，三份读回逐字节一致、六份源／控制文件未变，独立 state 未产生任务。30 秒 1600×1000 MP4 为八张真实 native 截图剪辑，标有 synthetic data／edited；无音轨，非连续实机录屏。本次 `bb9fa824` App 已在原生窗口核对 canonical macOS CLI：架构 aarch64、candidate bytes_match、App／CLI state 一致。`afa98f66` App 在同一合成 HOME/state 精确批准 3/4 原件共 406 bytes，实际完成 age 归档；App 清单三件、会话两条消息与字节位置 0／95、EOF 已观察；Markdown 指令原文 96 bytes 直接显示，结构化切换禁用；实际锁定后口令／索引／正文清除。密文复制到另一独立 HOME/state 后由配套 CLI inspect/read，三份逐字节一致、独立 jobs 为零，六份源／控制文件未变。后台启动空白经用户切前台恢复，未修改业务代码；临时诊断副本不作为交付包。未做真实认证、清理、迁入或 resume |
| 最新 CI 观察 | 遥测／updater source `4d3706fce1224a1a8b8ca6bf409561b2a1bce6c5` 的 [run 37926493932](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37926493932) 已直接核对 completed/success：macOS／Ubuntu 默认 28 项、独立 UI／browser 14 项、CLI 包装与 Linux static runner／OpenSSH／disposable VM 全部通过；整个 job 分别 24m32s／54m25s。IPv4／IPv6 source `e20e2c2988c28bd33e03d2fc45b6085eefb0637a` 的 [run 37908877220](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37908877220) 已直接核对 completed/success：macOS／Ubuntu defaults、独立 UI／browser、CLI 包装、Linux static runner／OpenSSH／disposable VM lifecycle 全部通过；该历史 run 仅覆盖 IPv4／IPv6 当时源码。修复 producer `bb9fa824` 的 [run 37877119482](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37877119482) 已 completed/success：macOS／Ubuntu 的默认检查、独立 UI／browser、Linux static runner／OpenSSH／CLI 包装／VM 生命周期均通过；docs `d90f221` 的 [run 37878435430](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37878435430) 也为 success。新步态 `afa98f66` 的 [run 37881691142](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37881691142) 已 completed/success：macOS／Ubuntu 两组全部通过，包含独立 UI／browser、CLI 包装、Linux OpenSSH 与 disposable VM 生命周期验收。`73b77cd` 的 [run 37812513290](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37812513290) 为 completed/failure：macOS／Ubuntu 的功能检查均通过，失败为 macOS `Upload evidence` 的 ArtifactService/CreateArtifact 五次超时。此前 docs `d3a6d7c` 的 [run 37812214395](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37812214395) completed/success。官网 producer `6c4e5df562550b37e9445b2d4fffe98a46be4493` 的 [run 37808126477](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37808126477) 已核对 completed/failure：macOS job 全部通过；Ubuntu defaults、static runner、OpenSSH、CLI 包装与独立浏览器组九项通过（含 `site-ui` 35.331s），`baseline-ui` 的测试子进程 stdin 写入报 `EPIPE`，该 job 为 failure、后续 VM 被跳过，原子进程早退原因仍未确定。本轮测试 helper 已处理 stdin error、检查非零退出并保留诊断，早退／有效 envelope 但 exit 17／ENOENT 回归正确拒绝；这不证明历史失败原因已修复。此前 `125d677b` 的 [run 37798378798](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37798378798) 为 completed/failure：Ubuntu 全部通过；macOS `site-ui` 在键盘导航至 `#how-it-works` 的滚动位置断言等待 4 秒后超时，其余九项与 defaults／CLI 包装通过。本地先前最终源码／真实媒体 `site-ui` 37.181s 与正式官网检查通过，不据此宣称此前 hosted CI 的导航失败已修复；本轮仅为相同 4 秒断言添加失败时 hash／滚动坐标／focus 诊断，不放宽条件 |

## Preview 视觉与本地首次试用（8e6448c，2026-10-08）

以下两行保存被 `bb9fa824` 候选替代之前的资源与验证身份；不代表新候选验收。该 App 的合成 native WebKit 首次归档／阅读、锁定清空与独立读回通过，三份原件 572 bytes，六份原件／控制文件保持不变。30 秒界面视频为八张 native 截图剪辑，明确标注 synthetic／edited，不是连续录屏。

| 对象 | 历史证据 |
| --- | --- |
| CLI 与分层验证 | 最终默认 synthetic suite 25/25 通过（RUST_TEST_THREADS=4），包括深路径计划预算、真正 PTY cleanup、共享 workspace 与两个独立 Rust crates；补充原密文逐字节不变断言后的 PTY journey 再次通过。built App 的 work／components／baseline／archive-wait／remote-task 五条相关 UI 旅程通过。扩充的 native `cli-candidate` 通过解包后保全、独立 state inspect／import、原字节核对及升级后原任务查询，PATH 仅系统工具；真实三平台输入包装 smoke 通过。2026-10-07 直接核对 GitHub：producer `57e8eb9` 的 [CI run 37523781020](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37523781020) 与记录源码 `3afaa7b` 的 [CI run 37524901927](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37524901927) 均为 completed / success；对应 SHA 已核对。CI 不替代真实认证或生产 runtime 验收 |
| 资源与包装 | clean `8e6448c` 的 macOS arm64 App 构建通过，约 24.66 MiB；候选复制件与 build output 全部文件字节一致。内置 x86_64／aarch64 runner 5,279,664／4,614,728 bytes 与 canonical static outputs 一致，host 789,392 bytes，非 fixture Chromium／Firefox 各 8 文件（52,522／52,684 bytes）与生成 manifest／canonical resources 一致；ICNS 供应图标一致、无 woff2。编译 docs revision 已核对为该 clean source。仅 linker ad-hoc executable signature，无 distribution Team ID、资源签封或公证；产物不进 Git。三平台 CLI archives 保留 `57e8eb9` 的独立解包／checksums／macOS 静态调用证据，未借此声称 Linux archives 本机执行 |

## 流式工作包与有限启动来源（7e23371，2026-10-07）

该源码阶段默认 23/23 synthetic checks（`RUST_TEST_THREADS=4`）通过，built work／components／baseline／remote-task UI 及后续 startup/components 相关旅程有分层证据。macOS／Ubuntu [CI run 37507737481](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37507737481) 已完成成功；这个结论只属于 `7e2337160d77969935d6ed0e07a8307613179e18`。

4 份原件共 36 MiB（24 MiB 非 UTF-8、12 MiB JSONL 及小参考文件）的归档、独立 state inspect／页面／import／preserve 与旧 v1 等 8 场景通过。macOS arm64 debug 的 age 实测 scrypt log N 14，各进程峰值 RSS 24.5–26.3 MiB；36 MiB inspect 35.4s、每页 36.6–53.7s、preserve 131.3s。元数据 1 GiB+1 阻塞有证据，当时未实跑 1 GiB 完整链路。这些 debug 数值不能推广到后来的 release KDF 或容量实测。native／CLI 候选当时仍是 2026-10-05 的构建，有限来源投影不证明实际加载、认证或 OS 隔离。

## 源码连续性与认证修复（628ce84，2026-10-06）

`093ca73` 合并 Clawd 问候与首次日期彩蛋；`cacd5ef` 绑定原回执主机、保全重名参考指令并冻结官方注销的认证状态；`628ce84` 保存后续方案。当轮默认 20 项首次 19 项通过，workspace 两项已有安装测试触发 4 秒 transport deadline；12 项安装测试串行诊断与原样 workspace 补跑均通过，未放宽产品 timeout。独立 baseline-ui／remote-task-ui 通过。该证据为源码与 synthetic 旅程；native／CLI 产物仍限于 2026-10-05，真实 Keychain token 世代未验证。

## 独立安装与双架构资源候选（25fdb43，2026-10-05）

2026-10-05 · **0.1.0 开发候选**。完整 [SPEC](SPEC.md) 与四条完整旅程 G01–G04 尚未交付。本轮补齐独立安装候选与双架构 runner，整合原任务恢复、批准一致性和浏览器跨入口接续修复；不代表正式发行、真实认证或生产 VPS 激活。

## 这份候选包含什么

本轮通过 [PR #2](https://github.com/IndelibleVivi/lintel-cc/pull/2) 合并独立安装候选，并整合 `e276f67` 的恢复／审批修复和官网／共享游戏。tested source 为 clean `25fdb43`，本地候选身份为 `0.1.0-candidate-25fdb4305785`。产品版本仍为 0.1.0，不能只凭版本号认定两个候选相同；candidate 身份与字节核验另行记录。

| 对象 | 当前证据与边界 |
| --- | --- |
| 源码 | clean `25fdb43` 已合并；包含恢复／审批／浏览器修复、官网／共享游戏与独立 CLI 打包 |
| CLI | Mac arm64、Linux x86_64／aarch64 三份实际归档已核对输入、manifest 与全部文件 checksum；本机新 release CLI 的完整 native 安装／升级旅程通过，旧版本、state 和原 ID 保留。CI 另在 Mac／Ubuntu 执行其 native 包；aarch64 runtime 未验证 |
| macOS App | arm64 `Lintel.app` 22.63 MiB，包含本轮修复与共享 Clawd；路径 `apps/desktop/src-tauri/target/release/bundle/macos/Lintel.app` |
| Linux runner | 新构建 x86_64／aarch64 musl；App `remote-runners` 的 bytes／SHA-256 与各自产物一致，aarch64 runtime 未验证 |
| 浏览器资源 | Chromium／Firefox 非 fixture 扩展与当前 arm64 native host 已入包；包内文件与 source／manifest 核对一致，直接使用包内资源的两个 synthetic 安装测试通过 |
| 安装与发行 | 仅 linker ad-hoc signature；无 Applications 安装、生产远端部署、Developer ID 签名、公证或正式发布 |

## 独立安装候选与双架构 runner

独立 CLI 打包入口为 `scripts/package-cli.mjs`：明确提供 macOS arm64 Mach-O、Linux x86_64／aarch64 static ELF 与 canonical runner-bundles，核对 Linux CLI 与对应 runner 字节相同，生成三份 archive、`SHA256SUMS` 与候选身份索引。checksum 包含 payload、`candidate.json` 和 `README.txt`，只排除自身；source_revision 是调用方声明，不是签名或认证 provenance。每个 runner 遵守 SSH installer 的 32 MiB 上限。

[安装与升级指南](agents.md) 使用系统 checksum 工具，目标端不需要 Rust、GUI 或 Node runtime。安装选择尚不存在的新版本目录，保留旧 executable、state 和原 job ID；archive／索引／版本目录不覆盖。打包或索引发布报错时只回收本次未被外部替换或编辑的 owned 文件，不承诺骤然进程死亡后的自动恢复。其它架构的格式 fixtures 不构成实际 runtime 验收。

`cli-candidate` 在 macOS／Ubuntu CI 通过同一 canonical entrypoint 执行 native 解包、version／capabilities／schema 和原 state/job 保留旅程。本轮 fresh Mac release／双架构 Linux cross-build／App 构建及 actual App 两项 synthetic 资源安装均通过。Ubuntu CI 实际运行其同源码原生 x86_64 static musl build；本地 Linux cross-built archives 只核对包内字节／checksum，未在 Mac 执行，CI 的另一份构建不能证明它们的 runtime。正式发布、Applications 激活和真实 VPS 安装仍是独立步骤。

当前本地候选 App 的包内资源身份可按下表核对；SHA-256 为完整摘要，来自本地构建和 source/resource 比对，不表示这些二进制已经发行。路径相对于 `Lintel.app/Contents`。

| 对象 | 路径 | SHA-256 |
| --- | --- | --- |
| macOS arm64 App executable | `MacOS/lintel-desktop` | `274c34b4c895c610967ede5cd6af296f74da83d41759e8457f15eaf2fe07cde5` |
| Linux x86_64 runner | `Resources/remote-runners/x86_64-unknown-linux-musl/lintel` | `9eb09f30bec1660914440aa1959f527a53d8afde5426a4322611036248018da6` |
| Linux aarch64 runner | `Resources/remote-runners/aarch64-unknown-linux-musl/lintel` | `907e122d9d897335606bfa8ec4400df9c68224417bf455f10b8108f0806a13a0` |

## 阶段修复的合同

- **配置中断恢复：** settings 原子发布前持久记录具体 staged 对象与冻结计划。原任务查询核对当前文件；确认已发布才开放独立恢复预览。未写入或被外部替换保留具体说明，不从意图推断成功。正常完成的字段恢复继续保留无关后续编辑。
- **清理批准：** 注销前比较计划冻结 executable、当前 inventory 与实时发现；重新 discover 不能改变旧批准含义。工作归档／迁入后，无论是否选择官方注销，都在第一破坏性动作前再次检查写入者与冻结范围。
- **迁入审批：** CLI `plan show` 与 App 展示同一受限 `import_manifest`，列出准确包身份、选中文件来源、最终相对位置、类别、大小与摘要；同名分配已经包含在批准中，不暴露正文或口令。
- **远端安装：** 预览冻结前置 binding（包括 install ID）；上传入口在锁内重查原未确认安装。旧安装查询可核实文件与运行能力，但不能覆盖后来绑定；历史记录可只核验而不自动启用。原任务 runner pin 与 no-replay 保留。
- **浏览器：** ACK 元数据与操作消费标记共用 Engine 的串行更新入口；App 清理经 popup 继续时保留原／子任务关联。过期与取消预览退出待批准队列，新尝试需要新 ID 与独立批准。DNR 恢复按规则集合比较，不依赖 API 数组顺序。
- **界面：** 最终迁入清单、配置核对说明和后续步骤进入审批／回执；记录页可筛选待处理任务，未决安装可直接查询原 ID。弹窗正文独立滚动，操作区随实际高度排布。暖纸／月光、Clawd 与侧栏字标保留。

状态更新仍由各领域 owner 负责：core 管文件／journal，remote controller 管安装／任务绑定，browser Engine 管 profile 操作，native host 管桥接回执。查询核对事实，不重新执行副作用。

## 官网与共享 Clawd 游戏（源码／前端候选）

[`apps/site`](../apps/site/README.md) 是独立静态官网：产品介绍、显式示例的计划／回执、日夜主题、真实开发状态与页尾游戏。无新增生产依赖、native/core 通道、analytics 或远程字体请求；未绑定域名或部署。网页的品牌与能力文案依据现有产品合同，视觉仍待作者验收。

官网与 App 共用 `apps/site/clawd-game.mjs`／CSS：四腿交替、收脚／落地反馈、石头与书本、星星计分、暂停／重试；旧 App runner 已移除，风景册和原成绩保留。浏览器与 App webview 分别存储主题／成绩。前端 build、7 项游戏机制检查、官网响应式与交互、App 隔离 Chromium 的挂载／卸载／主题旅程分别验收；共享游戏现已进入本节列明的 App 构建；前端检查仍不证明 native WebKit、安装或正式发行。测试入口是 `site-game-test`、独立 `site-ui` 与 `clawd-app-ui`。

## 已有能力与入口

| 范围 | 当前入口与权威说明 |
| --- | --- |
| 环境、七项保护设置、字段恢复、漂移 | [core](core.md)、[人类操作指南](operator-guide.md#protect) |
| 独立 archive-only／preserve、portable work 与清理保全顺序 | [工作流程](operator-guide.md#work)、[Agent CLI](agents.md) |
| 精确 systemd 暂停／恢复与原任务托管 | [服务](services.md)、[远程](remote.md) |
| App 内置扩展与 host 准备、配对、两阶段清理 | [浏览器](browser.md) |
| loopback 通道、明确生效配置与连接观察 | [网络](network.md)；仅覆盖经过代理的连接 |
| Day／Night／System、Clawd、任务帮助与审批 | [桌面](desktop.md)、[视觉身份](visual-language.md) |

## 验证与历史

统一入口是 `python3 tests/verify.py`；浏览器启动、App 渲染、Linux OpenSSH／VM 和 App 包内资源是独立检查。每份报告的源码 HEAD／dirty、fixture、runtime 和跳过状态分别记录；构建成功不代表实际安装或 runtime 验收。详细验收用例继续见 [acceptance-status.json](acceptance-status.json)，局部通过不自动关闭完整用例。

本轮 clean `25fdb43` 的 [push CI37267381234](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37267381234) 与 [PR CI37267385529](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37267385529) 均通过。八份入口报告记录同一 HEAD、`dirty=false`；忽略项不计通过。

| 检查 | macOS arm64 | Ubuntu x86_64 |
| --- | --- | --- |
| 默认合成验证 | 18/18 | 18/18 |
| native CLI candidate 安装／升级 | 1/1 | 1/1 |
| 独立 browser／App／官网 UI | 7/7 | 7/7 |
| 静态 musl runner 的真实 OpenSSH | 不适用 | 1/1 |
| 一次性 Linux VM 生命周期 | 不适用 | 1/1 |

两平台 Chromium 报告均有完整 12 项 smoke，包含 App clear → 实际进程退出／新 onStartup → popup 继续 → 原 App 最终回执；提交一次，取消预览回传也通过。VM 报告为 `evidence_complete`，覆盖真实 systemd/PAM/logout/cgroup/reboot 与服务恢复，证据限于一次性 guest。本地实际候选归档核对、提取后 named CLI／finite-adapter 旅程、完整 native 安装／升级和 34 项验证入口自检通过；实际 App 包内 host／extension 的 synthetic 安装检查为 2/2。

此前 `e276f67` 阶段的本地默认 18/18、29 项 Engine 单测及 UI／配置恢复证据仍有其边界；本轮 fresh CI 另验整合源码。较早候选与 CI 引用保留在 [候选历史](candidate-history.md) 和 [验证说明](verification.md)，不替代本次源码验证。

仓库已公开；项目原创材料的公开复用许可证仍未选定，源码可见不构成通用复用许可，第三方权利保持独立。

## 完整目标仍缺少

1. **真实认证与写入者控制：** Keychain/共享 profile、Desktop/IDE/service 的完整定位与生命周期；有限 systemd 暂停已接通，其他 supervisor 暂停、重新登录及旧会话续用。当前识别部分 Claude 进程及可访问 manager 的直接 root 绑定服务；明确要求目标写入者先停，不能归属的进程阻止清理。官方 auth 命令仅有合成 CLI 证据，不能宣称生产注销已验收。
2. **完整外发策略与强约束：** 七项自定义已接入；更多取舍、入口实效矩阵、macOS Network Extension 签名与权限、Linux namespace；直接 socket、UDP、DNS、NO_PROXY、子进程不能由代理覆盖证明。
3. **远程运行：** 用户级安装／版本绑定已接入候选；Ubuntu 隔离 OpenSSH 已验证安装／丢 ACK 核对／原任务查询／交互 PTY，真实 VM 已完成有限 system-manager 暂停恢复、符合既有条件的 system/user manager 严格 logout 后续跑、原任务完成与 reboot 后核对。setsid 不保证 logout 存活，托管路径也不保证 reboot 续跑；生产 VPS 安装与任务实跑、aarch64 runtime、其他 supervisor 和完整 G03 仍未验收。
4. **浏览器旅程：** 独立 Chromium 临时测试 profile 已完成真实持久安装／onStartup 两阶段清理；仍需正式 Chrome、Edge、Firefox 与 AdsPower 的安装、配对与清理验收。Firefox 独立 CacheStorage、按站点 proxy、容器后台停写、克隆识别和专用 browser 启动仍有缺口。
5. **发行与维护：** 菜单栏、定时漂移、签名规则更新、卸载、升级、正式签名/公证、性能预算。
6. **并发与恢复：** 不合作的外部编辑器或已持有文件描述符的 writer 没有 OS 级 CAS；不确定副作用需核对。混合状态加密备份不支持自动恢复；会话/记忆迁入不证明可以续聊。

这些是原始完整目标的差距，不是缩小后的新 SPEC。下一关口是正式浏览器／AdsPower 与非开发者安装、真实认证／入口矩阵、生产 VPS 任务验收。隔离 runtime 与合成数据能证明机制，不能替代真实产品场景；现有个人登录和生产服务不作开发 mutation fixture。

---

## 官网（本地网页候选）

[`apps/site`](../apps/site/README.md) 提供中文产品介绍、显式示例的计划／回执交互、日夜主题、开发状态与页尾 Clawd 跳跃游戏。独立静态 HTML/CSS/ES modules，无新增生产依赖，无 native/core 通道、外部字体或 analytics 请求；主题与最高分仅存于当前浏览器。桌面与 390 px 页面、示例切换、主题保存、游戏开始／暂停／重试、焦点离开与离屏暂停、存储不可用路径、reduced motion 已在隔离 Chromium 验证。`site-game-test` 与独立 `site-ui` 纳入统一验证入口。不表示 App 发行或官网已经上线；域名、托管部署及用户视觉验收尚未完成。

## 人类任务、agent CLI 与独立工作保全（源码已合并，开发候选）

完整 A/B/C 已通过 [PR #1](https://github.com/IndelibleVivi/lintel-cc/pull/1) 合并到 main。已接入六个 App 任务帮助入口及[人类指南](operator-guide.md)、[Agent CLI 指南](agents.md)。工作保全独立于清理：archive-only 只生成加密包；preserve 归档后建立新根并迁入，成功按本任务显示完成，旧登录／设置／服务绑定保留。旧 plan_reset/rebuild 与历史部分完成回执仍兼容。工作包保留 lintel.work/1 格式，可用目标主机的显式 archive_path 独立检查、阅读、选择性迁入，不依赖原 job/state；密文由明确的系统工具传输，已有文件不覆盖，包内容与目标在批准执行时再核对。

现有 lintel runner 提供静态 version/capabilities/describe/schema、named core 操作、冻结 plan show 和原 ID job wait。GUI 与 CLI 共用 crates/remote 的有限 SSH controller；Mac CLI 接现有 browser-host control，浏览器人工确认／真正重启仍由目标 profile 拥有。network serve 与既有 egress 共用前台生命周期，明确 owner/PID/实际 active_config；不能查询或停止 App 进程内通道。discover 两入口的 runner capabilities 一致，submit 拒绝退出非零。清理预览与执行都先保全／新根迁入，再批准的注销与精确删除；接受后失败回执保留 code/message/真实 phase/step/不确定副作用与原任务恢复指引。

Review 修复纳入共同执行路径：创建新 root 前分配并持久保存准确 new_root／new_environment_id，登记失败或中断仍可按原任务定位，写回执失败不创建目录；意图不证明登记完成，App 标记待核对路径且仅对已登记环境开放保护方案。显式归档输出冻结父目录的 device/inode，在接受与发布前核对，目录替换或旧计划缺身份返回 `stale_plan` 并要求重新预览；已完成原任务仍只查询。已有目录权限保留，新建父目录为 `0700`，迁入文件为 `0600`；整批实际目标在预览与执行前检查当前 UID 归属、有效访问／写入权限和 path guard；已知权限、归属或非目录祖先问题在任何正文写入前拒绝，不自动 chmod/chown。新文件通过 macOS/Linux 原子不覆盖 rename 发布，不产生双链接清理窗口；不支持时返回 atomic_publication_unsupported，已有内容保留。归档／状态备份的未读回路径标为待核对，App 查看按钮与任务归档列表共用读回证据；活跃与已迁入工作同名／祖先路径统一分配；批准后在真实目标文件系统以空文件检查整批路径，仍有大小写或规范化等价冲突时在新根／正文写入前停止。state/work 归档与 migration probe 在写入前保存准确目标与执行中步骤，读回／清理核验后才标记完成；原任务查询保留中断产物，不删除或重跑。job 包读取绑定已记录的准确密文，显式 path 保持独立；包内 generator 是自声明。App 重建／退役必须至少选择一种工作类别才能预览，修复登录保持独立，strict named/SSH schema 允许省略或空工作类别，reset/retire 仍要求非空；strict named/finite（包括 wait 轮询前）与外层 remote.execute/reconnect/launch 的 core ID 共用 UUID 检查，错误格式在 state/SSH 前拒绝；named service schema/core 共用有限名称规则，拒绝未实例化模板和命令。交互 launch 必须使用专用入口及真实 TTY。 strict named execute 与有限远端新提交共用 64 位 lowercase hex plan-hash 格式；错误格式在 named core 初始化或远端任务 intent／SSH 前拒绝，准确批准仍由 core 核对，已有任务仍只查询。 远端回执刷新走原任务 reconnect；手填 ID 与 CLI remote job 使用只读 job lookup，共同 controller 对已有任务保留冻结 runner，未知 ID 不创建任务记录、不占未提交计划的提交位置。已移除 alias 的已有任务仍可查询；新建显式 reconnect 记录在首次查询前冻结当前 runner，alias 升级不切换该记录。已有无 digest 的旧记录保留 PATH 兼容；原提交去重不变。

最终源码 clean `d0851c7` 的 [CI37249445342](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37249445342) 已通过 macOS／Ubuntu 默认各 17/17、独立 browser/UI 各 5/5（真实启动、App 配对、服务、原任务、工作保全），Ubuntu OpenSSH 1/1 与真实 Linux VM 1/1。六份入口报告记录同一 clean source HEAD；两平台 root workspace 检查通过；本机 core 80/80、operations 6/6；本机 shared remote 41 passed／2 independent ignored。ignored 不计为通过。详细 Chromium 启动报告保留全部 11 个断言、旧进程退出／新进程、生产 onStartup 世代与无扩展加载 flags；VM 报告 evidence_complete，原 PAM logout、准确 system/user unit cgroup、原任务完成、真实 reboot 后核对、精确 service 恢复均通过。全部使用合成 roots、账户与 inert Claude，不证明生产 VPS 或真实认证。

arm64 App 从最终源码 `d0851c7` 构建。独立临时版本目录的 release CLI 已从 `d0851c7` 安装，完整 named plan/submit/原 ID wait/restore、有限 adapter 与 portable 5/5 journeys 通过；App 与 CLI 均对应最终源码。不依赖 GUI。App 约 14.46 MiB，含 canonical browser host 与扩展；实际 App 包内 host/扩展资源两项独立 synthetic 安装检查通过。未安装到 Applications、激活、发行签名、公证或正式分发。此候选没有本轮新 Linux runner bundles，远端安装明确显示 bundle_unavailable；CI 的 x86_64 静态 runner 构建与隔离运行是独立证据，不能当作 App 已包含或生产主机已安装。aarch64 runtime、正式浏览器／真实认证与生产 VPS 仍未验收。

工作旅程使用两个隔离 HOME 的真实 core，检查归档／保全／独立导入、错误口令、键盘和 Day/Night，以及明确标记的中断 probe 回执。App 配对使用真实 host。invoke/clipboard/SSH/service seams 为合成，不证明 native WebKit 或生产传输。下方保留此前 clean CI 与 App 的历史证据；后续文档提交不改变上面的 tested source HEAD。

## SSH 原任务与恢复入口（候选）

主机面板查询原任务后可直接查看完整回执与恢复，进入准确 alias／环境的记录；已移除主机保留 query-only。恢复依旧独立预览和批准。TypeScript/Vite 与 `remote-task-ui`、`service-ui` 本机渲染检查通过，涵盖丢 ACK 后 App reload、一次原提交、查询更新、关闭面板后的迟到响应、后续编辑冲突、独立恢复和 Day/Night 1120／900 布局。clean `7a05291` 的 [CI37216354079](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37216354079) 也已通过 macOS／Ubuntu 默认各 13/13、独立 browser/UI 各 4/4（启动、配对、服务、原任务）。`remote-task-ui` 的 SSH/invoke/持久 registry 为合成；不代表 native WebKit、真实 SSH 或生产 VPS。

## SSH 任务托管与生命周期（候选，隔离 runtime 已验证）

clean `200aaed` 的 [CI37209110177](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37209110177) 已通过 macOS／Ubuntu 默认各 13/13、浏览器／界面各 3/3、Ubuntu OpenSSH 与真实 VM。正常等待／SIGSTOP × 两种有效 `KillUserProcesses` 的四次 logout 都观察到 worker 消失、原 session scope 停止；`setsid` 虽脱离进程 session，仍留在原登录 cgroup。原任务均进入 `needs_reconciliation`，每次只提交一次。正常运行也失败，因此不能把 SIGSTOP 当唯一原因；未捕获终止信号，不归因为某个 signal 或仅由 logind policy 导致。

新 runner 已接入有限单任务 transient service：already-root system manager，或已有 `Linger=yes` 且当前 UID user bus 可用的 user manager；不改主机政策，不提权。worker 经同机私有 pipe 保留原环境与工作目录，认证来源／proxy 复查不失真；环境值不进入 unit 属性、argv、回执或磁盘。实际 cgroup 核验与 core 的持久 `execution` 在 ACK 前完成；manager 启动不确定时不 fallback。macOS／无 systemd／条件不满足的用户保留 setsid，并显示明确续跑限制，重启存活不作承诺。

前轮 clean `7a05291` 的 CI 已完成真实 Ubuntu 24.04.4／systemd 255 VM：有效 `KillUserProcesses=yes` 下，原 OpenSSH/PAM session 已 logout 后，system/user 两种 manager 的 worker 都在准确单任务 unit cgroup 中保持运行；释放合成 barrier 后，原 policy job 完成。不符合条件的 setsid worker 消失，查询成为原任务的 `needs_reconciliation`。各只提交一次，没有重新 execute。真实 reboot 改变 kernel boot ID 后，system-managed 和 setsid 的 accepted job 都重复查询为同一原任务的 `needs_reconciliation`，不重放。原 system-manager 服务暂停／邻居保留／外部编辑冲突／持久 hold 重启恢复套件继续通过。

同一真实 VM 的共享认证 fixture 验证 caller 环境在 system manager 切换后保留：预览后新增共享范围，执行在 accept 前返回 `shared_auth_scope`，没有调用注销、删除合成凭据或持久保存环境值。只读清理预览实测 55.279 秒；该完整 fixture 预览预算 180 秒，普通请求仍为 60 秒。macOS／Ubuntu core 各 52、runner 各 8、VM 控制流程各 26 项通过；六份默认／browser／OpenSSH／VM 入口报告记录同一 clean HEAD。上述 runtime 使用合成用户、临时 roots 与 inert Claude，不证明生产 VPS 或真实认证行为。

## 精确服务生命周期与视觉身份（候选）

Linux runner、App 和 CLI/TUI 已接通明确 root 绑定的 systemd service 检查、批准暂停与独立批准恢复。持久 owned drop-in 阻止重新启动，实际加载 condition、inactive/MainPID/cgroup 读回才认定暂停；清理预览和执行复查 live hold，手工 stopped 确认不能替代它。恢复核对原 root 对象与 unit 来源，只移除原任务拥有的 blocker，原 active 才启动，原 inactive 保持停止；外部编辑、共享停止传播和可能影响邻居的启动依赖拒绝。当前 UID user manager 或已为 root 的 system manager 有限支持，不自动 sudo、不改 login policy。见 [服务指南](services.md)。

clean `8c9d85c` 的 [CI37177180828](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37177180828) 已在 macOS／Ubuntu 通过默认各 12/12、独立浏览器／界面各 3/3，Ubuntu OpenSSH 1/1 与真实 VM 1/1；六份入口报告记录同一 clean HEAD。双方 core 52/52 通过（含 21 项服务状态／批准／冲突／中断回归），desktop 默认 56 passed、3 independent ignored（不计入通过），Python SSH 13 项通过。构建页面 `service-ui` 与真实 host 的 `browser-pairing-ui` 各通过：服务 manager/invoke 为合成，覆盖检查／取消不变更、独立暂停／恢复批准、丢 ACK 原任务查询、外部编辑冲突和 Day/Night。界面检查不能提供 systemd 或 native WebKit 证据。

`linux-vm-runtime` 的详细报告为 `evidence_complete`：disposable Ubuntu 24.04.4、systemd 255 PID 1、OpenSSH 9.6 的真实 PAM session／logind／cgroup。root system-manager 完整 service suite 及 prepare → 真 reboot → recover 均通过：精确暂停后 inactive/dead、MainPID 为零、cgroup 为空；Restart=always／timer／手动启动不能回写，邻居继续运行且 InvocationID 不变；外部 unit 编辑返回 `service_restore_conflict` 并保留修改和 blocker；独立恢复、原 inactive 状态与重复执行不重启通过。真实 kernel boot ID 已变化，持久 hold 在重启后实际加载，查询同一 quiesce job 后独立批准恢复。此前实际 wire／condition 路径及 Requires 成员排列问题已修正；成员增删、源文件变化和 pending/failed 状态仍拒绝。VM、overlay、seed 与测试密钥已清理，宿主 login policy 未更改。

上述 `8c9d85c` 的旧 setsid 路径在两种有效 `KillUserProcesses=no`／`yes` policy 下都被 logout 终止；原任务与另一个 reboot 后的 accepted job 均保持原 ID、`needs_reconciliation` 和一次提交。该历史证据只证明旧路径的中断发现。新 system/user manager 的严格 logout 存活与完成证据见前节；生产 VPS 与完整 G03 仍未验收。

提供的视觉资产已接入 canonical SVG master、主题内联 mark、五个 Tauri icon exports，并保留可编辑展示 variants；仍保持纸上晨光／夜里月光、侧栏 `lintel_` 与分割线、2.6 秒慢闪（reduced motion 静止）。前轮本机已重建 `7a05291` 的 arm64 App，约 17.87 MiB，包含新托管路径、caller 环境保留与共享认证前置拒绝，以及对应 x86_64/aarch64 静态 runner；包内实际字节、大小和摘要与 manifest 一致，icon 与 supplied export 字节相同。可执行文件仅有 linker ad-hoc signature，无发行证书；尚未安装、激活、发行签名、公证或正式分发。见 [视觉身份](visual-language.md)。

## 浏览器首次连接与真实重启验收（候选）

App 伴随扩展文件准备已接通：非 fixture Chromium / Firefox 包随 App 构建，桌面选择浏览器后只读预览，再批准准备稳定的当前用户目录。Chrome / Edge 共用 Chromium 目录，Firefox 独立目录；受管更新保持加载路径，非受管或被改动的内容拒绝覆盖，整份计划与当前文件重新核对。准备后的 Finder 入口和路径复制只交接文件，目标 profile 仍需浏览器开发加载。安装器中断切换须新的恢复预览与批准；原批准资源改变则停止恢复，保留文件供核对，不显示成完成。

随后填入准确扩展 ID，另行预览并批准内置 Native Messaging host 安装，再复制 12 位配对短码到目标扩展提交请求，核对后在 App 批准。页面已修正复制内部 challenge 的错误；真实 framed native 请求对旧复制值返回 `invalid_pairing_code`，新复制值与显式批准通过。host 稳定目录／精确注册／owned 更新与 query-only 重复核对保留；独立 CLI 手工 installer 仍有实际用途，桌面不接受任意 host 路径。目录准备、组件注册、profile 配对与在线轮询是不同事实。

此前内置扩展轮 desktop 检查 57 passed、0 failed、0 ignored，明确过滤 1 项独立 Linux runtime；其中两项分别从 **App 包内资源** 安装实际扩展／host 到临时 home，验证扩展文件完全一致、未创建浏览器 profile 或注册，以及真实 host native frames／非授权 extension 拒绝。扩展安装器 12 项含实际资源测试通过，覆盖完整批准、资源／目标变更、fixture 拒绝、受管更新、非受管保留、symlink 和目录切换中断／部分清理恢复。TypeScript、Vite 与 Tauri App build 通过。

构建页面的合成 native bridge 检查扩展预览不安装、整份批准、有限 reveal、路径复制（合成 clipboard）、三种浏览器说明、更新／重复核对／中断继续与冲突禁用、过期／缺资源／冲突、浏览器切换撤回状态、pending 禁用、单独配对、键盘与 Day/Night 1120／900 布局；长路径可读，无横向溢出。视觉仍属候选，未证明 native WebKit、Finder 的实际展示或浏览器加载。

此前浏览器轮 macOS App 约 17.17 MiB，含 Chromium 47799 bytes、Firefox 47961 bytes（各 8 个文件），实际 bytes／摘要与包内 inventory 一致。独立 host 787440 bytes 与两种既有 Linux runner 的大小／SHA 也相符。默认包无 woff2，本地六份字体保留；开发／显式 local-candidate 可用本地字体。host 只支持 native 同架构构建，跨架构／universal 明确拒绝。App 未安装到 Applications、激活、公证或发布；真实 VPS 未写入。

此前 clean `67f8549` 已通过 [CI 验收](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37142533163)：macOS／Ubuntu 默认各 12/12，无跳过；Ubuntu 独立 OpenSSH runtime 1/1、0 ignored，三份产物记录同一 clean HEAD。该次 CI 没有真实浏览器检查；App 包内资源 opt-in 与渲染检查仍是独立本机证据。

2026-10-04，macOS arm64／Playwright Chromium 155.0.8059.12 的完整两阶段 `browser-smoke` 已通过，11 项原断言全部保留。测试专用原生安装恢复在独立临时 profile 持久安装 synthetic 扩展；旧浏览器进程实际退出、新进程重新启动且无扩展加载 flags，同一扩展身份保留，生产 `runtime.onStartup` 监听器产生新的世代，再单独批准 `finishClear`。活跃 SW／iframe writer、目标隔离／邻域保留、五类存储读回、定位恢复、旧操作不重删新合成登录、真实 Native Messaging 往返均通过。没有修改 profile preferences 或伪造启动世代；不是 App 自动加载扩展的功能。

新的 `browser-pairing-ui` 也通过：构建 App 页面使用真实 core／host 进程、合成 invoke／clipboard，实际复制值经 framed `pair_request` 接受并由 App 显式批准，保持“已配对／当前离线”区别；短码更新、键盘和 Day/Night 通过。TypeScript／Vite／Tauri App 已按短码修正重建。本轮 clean `93c3f74` 已通过 [CI 验收](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37163091952)：macOS arm64／Ubuntu x86_64 默认各 12/12，无跳过；独立浏览器与配对各 2/2，Chromium 151.0.7922.34 的完整 11 项断言、进程退出／更换、原生启动世代、无扩展加载 flags 与配对报告均核对通过。Ubuntu 独立 OpenSSH runtime 1/1、0 ignored。五份入口产物都记录同一 clean HEAD，另有两套详细 startup／native receipt／App 配对报告；App 包内资源 opt-in 与本机构建仍是独立本机证据。

正式 Chrome/Edge/Firefox 与 AdsPower 尚未验收。AdsPower 有官方扩展加载文档，但当前 host 安装器仅提供 Chrome/Edge/Firefox 固定路径；选择 Chrome 不等于安装到 AdsPower。其 Native Messaging 注册位置与实际行为未核实，未操作现有 profile。扩展仍需开发加载，Firefox 临时加载退出后移除，签名／商店分发和 U01 非开发者旅程仍未验收。真实 Claude／claude.ai 的认证、设置效果和重新登录也未验证。

## 已交付的自定义保护（前轮证据）

新增第三种方案 `custom`：七项既有控制逐项 keep / disable / remove，未选字段保持原值；关闭语义由 core 判断，移除只撤掉当前 user settings 覆盖。全选保持不重写 settings 字节、权限或缺失文件。准确 diff 与选择冻结进计划/回执，后续编辑冲突会拒绝执行/恢复；`keep_remote_control` 不自动改写明确选择。规则 v3 要求旧待执行计划重新预览，旧回执与恢复保留。inspect 展示七项来源与 `next_launch`，运行效果仍未验证。

App 本机/SSH 表单、首页入口、按主机/环境保存的草案、当前值/影响/来源、预览/批准/回执/恢复已接入。旧 runner 通过缺少 `supported_presets: custom` 明确禁用并提示更新；TUI 逐项选择也走同一 core。七项编辑器仍未包含 WebFetch/marketplace 控制，完整策略与入口实效矩阵仍欠缺。

本轮 core 31 项、native 有限 custom schema、Python SSH 12 项通过；真实 CLI policy 与 TUI PTY journey 通过。App 合成 native bridge 使用真实隔离 core，完成 RC blocked/compatible 预览、执行/准确恢复、未选 root 保留、草案重开/隔离、旧 runner 提示与恢复、1120×800 Day / 900×640 Night 和键盘检查；视觉为候选。独立 Linux OpenSSH journey 已改为 custom subset（保持外部 false、关闭 boolean 字段、移除 GrowthBook，含丢 ACK/原任务查询），该轮源码 clean `1fb9ab6` 已通过 [CI 验收](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37125403657)：macOS/Ubuntu 默认入口分别 12/12，无跳过；Ubuntu 独立 OpenSSH custom journey 1/1、0 ignored，通过真实 x86_64 musl runner 的保持/关闭/移除、冻结回执、丢 ACK 原任务查询和 PTY 启动。此前已有安装/启动证据继续保留。两种新静态 musl runner 与 macOS App 候选已重建（约 16.81 MiB），App 资源大小/摘要与 manifest 相符；未安装、激活或发布。aarch64 runtime、真实 Claude 设置效果与生产 VPS 仍未验收。本机默认入口也 12/12 通过（desktop 35 passed＋独立 Linux test ignored；ignored 不计入通过）。

## 远端会话能力（源码已接入）

远端环境详情与回执“打开 Claude”已接通：只读 preflight 后请求 macOS Terminal，以固定严格 SSH、PTY 与绑定 runner 启动所选环境；CLI/TUI 同一配置根启动，隐藏管道和 prompt 参数拒绝。发现增加当前用户 native `~/.local/bin/claude` fallback，保留 PATH 优先级。原生 SSH launch 3 项回归、完整 desktop 35 项与 core 31 项通过；独立 CLI/TUI PTY journey 使用 inert Claude 通过。App 合成 native bridge 入口/键盘/失败恢复已验证，保留现有界面样式；真实 macOS Terminal 已在临时本机配置根执行 inert 程序，核验 cwd/config root、TTY 与零参数；完整 macOS→真实 VPS 交互仍未验收。accepted 回执须先查询到结束状态再开放启动按钮，失败反馈滚入可见范围；对应 App 合成 bridge 检查通过。

Ubuntu 独立 OpenSSH runtime 已通过[CI 验收](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37123017551)（clean `1276c84`）：实际 x86_64 静态 musl runner 上传、丢安装 ACK 核对、丢 submit ACK 原任务查询与真实 PTY 启动，上传和提交各一次。使用临时 loopback sshd、合成 home/state 与 inert Claude，无登录或模型请求；不证明生产 VPS、logout/cgroup 或重启存活。macOS/Ubuntu 默认入口分别 12/12 已通过，含五条合成 journey；ignored 的独立 Linux test 不计入通过。两种 Linux runner 与 macOS App 已按新源码重建，候选约 16.81 MiB，未安装／激活／正式发行。

## 已接通

- core / runner：环境登记与建立、版本化外发设置计划/批准/读回、字段恢复、漂移、脱敏支持资料；有限登录修复/客户端重建/退役；加密归档、文本阅读与选择性迁入。清理范围是预览中的准确文件和可显式调用的官方认证入口，不等于全客户端清场。概览统计与归档准入解耦（大文件不阻断 inspect）；二次重建会把此前迁入 lintel-imports 的工作重新计入归档；损坏 settings 不阻断不修改 settings 的保全计划；各变量分别按官方非空值或 boolean 语义判断；静态识别 native/npm 产品版本，Remote Control 按版本／Trusted Devices 条件评估，未知条件显式标注。冲突旧值通过用户选定的准确删除 diff 解除；预览冻结策略与产品版本，变更后拒绝执行。
- 桌面：同一套本机/SSH 环境、计划、清理、归档和任务界面；浏览器模块全操作入口、Native Messaging 注册计划与配对；本机代理启停/连接观察。所有修改先生成计划或独立确认。
- 界面：聊天式首页按角色招呼、操作框、一行工作入口重排，玩耍入口集中到不挤动工作区的口袋菜单。Clawd 支持摸摸／拖抱／弹飞／连续戳戳害羞与躲藏、下拉起飞；四幅重新绘制的字符风景保留清楚角色轮廓与画面比例，另有跳跃小游戏。提供 Day / Night / System、可选本地字体、键盘操作及减少动态效果。字体文件不进入 Git，干净 checkout 使用系统 fallback。
- SSH：native Rust bridge 使用系统 OpenSSH、静态 alias、严格 host key 与有限 JSON 请求；`lintel submit` 持久接收后返回 ACK，符合既有条件的 Linux 使用有限单任务 system/user transient service，其他路径保留 setsid 与明确续跑限制，重连只查询原任务。桌面可移除／撤销移除 alias；原任务与去重记录保留。新增用户批准的 Linux x86_64 / arm64 runner 探测、安装预览、内置文件上传、SHA／权限／能力核验与 alias 版本绑定；上传前持久 intent，丢 ACK 只核对原安装，原任务冻结 runner。安装仅写用户专用版本目录，不修改 PATH／系统服务／Claude。错误提供阶段、具体原因、排查步骤、退出码、限长 stderr 与只读核验命令，查询错误同样保留诊断。正式 agent CLI 与桌面共用 Rust controller。Python controller 保留旧 raw request/submit 等兼容子集；不再承接新特性，两套本地提交记录不能混用。
- 帮助：App 内提供开发者 GitHub、Lintel 源码说明、Infra Field Guide 的 VPS 101 与 SSH 排障入口。macOS native 仅打开固定 HTTPS 文档资源，并增加六个场景入口、人类操作与 agent 指南；新文档的公开链接随源码发布生效。远端准备文档区分 App 批准安装与独立 CLI 的手工 PATH 准备。
- 浏览器：Chromium MV3 / Firefox 独立适配、Native Messaging host、实例冲突/配对/持久操作记录与固定路径安装器。Chromium 清理先隔离、关闭目标及 iframe 宿主、注销 worker，等待完整浏览器重启，再用新确认继续删除。
- 网络：loopback CONNECT / 有限 HTTP 转发、精确域名/端口规则、上游与连接事件；仅证明经过通道的流量。native start/status 返回实际采用的规范化 active_config；界面分开呈现当前生效配置与按环境保存的下次启动草案，支持默认动作及允许／阻止规则和端口。

## 证据与未验证范围

| 范围 | 当前证据 |
| --- | --- |
| Core | 最终 clean `d0851c7` macOS／Ubuntu CI root workspace 检查通过；本机 core 80/80。覆盖既有策略／有限服务／官方注销合成 CLI／共享范围／删除替换保护，以及独立 archive/preserve、portable import、同名和祖先路径分配、原 job 密文绑定、真实目标文件系统等价路径准入、发布 intent、整批目标父目录归属／有效权限准入与中断原任务产物定位；完整真实认证与并发写入者控制仍未验收 |
| CLI / submission | 实际 CLI 配置往返旅程通过；独立 worker 的 durable ACK、父进程退出后完成、原 ID 查询、去重、口令不落记录通过。PTY 中重建/批准/无回显口令/加密归档通过。新增 work_preservation journey 3/3（大文件概览、A→B→C 保留、损坏 settings 独立保全）；policy journey 覆盖版本变化拒绝、外部编辑、准确删除、恢复外部旧值、receipt 策略证据 |
| Egress | 5 unit + 7 localhost socket tests 通过；新增 inet_aton 式／IPv4-mapped IP 写法不能绕过精确规则、wire 配置缺省 default_action 直接拒绝的回归。本轮 native network tests 3/3 新鲜通过，覆盖规范化生效配置读回、精确规则命中、环境隔离、无效配置与通道结束状态。没有真实 Claude 公网探针或进程强约束证据 |
| Browser | 20 JS 与当前 13 native host Rust tests 通过；App 内置扩展安装器 12 项含包内资源测试、host 安装／更新与实际 executable 验证见上节。既有覆盖：通用 control 不能放行扩展（授权仅限安装路径）、配对码 12 hex／5 次失败作废／pending 上限、browser 由调用方身份派生、running 回执移到 durable 边界之后、DNR 读回数组序不敏感。活跃 SW 负例证实注销后仍可能回写；当前真实 Chromium 持久安装／完整退出与原生 onStartup／二次确认 smoke 已通过，详见上节。覆盖 active SW／iframe writer、五类目标存储与邻域保留、隔离／权限恢复、重复操作不重删及 Native Messaging 持久回执。正式三浏览器与 AdsPower／Firefox 容器／真实网站仍未验收 |
| SSH | 本轮共享 crates/remote 默认 41 passed、2 independent ignored，standalone desktop 默认 24 passed、2 bundle-resource ignored（ignored 不计为通过；本机实际包资源另选 2/2 通过）；此前内置浏览器轮 57 passed、0 ignored（包含两项包内资源 opt-in，过滤独立 Linux runtime），Ubuntu 独立 OpenSSH runtime 已通过，保留既有 SSH 回归，包括上传真实字节／权限／哈希、过期目标／文件／批准拒绝、丢回包只核对、外部文件保留、平台／能力拒绝、原任务版本冻结；既有覆盖移除后保留任务／去重、具体失败分类、stderr 并发排空／限长／去敏、只读排查命令、查询原错误保留；既有严格 host key、固定命令、丢 ACK 查询继续通过。固定关闭 `ProxyCommand=none` / `RemoteCommand=none`，observe 仅采用 id 匹配的回执，私有目录校验属主。Python fake-SSH 13/13 通过，含有限 custom schema 与 stdin payload 回归。两台真实 Linux x86_64 主机已完成严格 SSH 与生产安装脚本的只读探测，OS／架构／UID／安装条件返回有效且目标身份不同；未上传、未安装、未运行 Claude 或 runner。最新隔离 VM 已验证符合条件的 system/user manager 在严格 PAM logout 后继续并完成原任务，不符合条件的 setsid 路径明确受限；真实 reboot 后原任务核对且未重发。生产 VPS 与 aarch64 runtime 未验收 |
| Web UI | 合成 root 中计划/执行/恢复、归档解锁/阅读/冲突拒绝/新环境迁入、四清理配方、退役重新启用、支持资料保存已走通。重排后 Home 的 Day/Night、900×640布局、工作入口、口袋展开不挤动操作框、连续戳戳／躲藏／拖甩、鼠标／滚轮／模拟触摸、焦点返回和减少动态已验证；四画收星、等比例缩放、翻页，游戏跳跃／暂停／碰撞／重开与本机最高分通过。网络面板经 synthetic native-response harness 验证延迟响应隔离和停止／编辑／重启。新增安装卡合成 native bridge 验证预览前不上传、显式批准、丢回包后的状态同步、原安装只读查询、关闭／重开／迟到响应隔离与连接管理；安装卡 Day/Night 和 900×640 长路径详情已实际渲染检查，视觉仍待用户接受。F04 界面使用真实合成 CLI 完成计划／批准／解除／回执／组织条件往返；新增 SSH 合成 native-response harness 验证具体错误／摘要复制、显式重连、移除／撤销／失败保留、当前主机回本机、原任务查询和关闭面板后的迟到响应隔离；帮助链接 ID 与 900×640 长命令排版通过。其后完成纸上晨光/夜里月光环境光、首页问候纵向重排、统一柔影刻度、clay tint 选中态的 Day/Night/窄屏截图 QA；浏览器面板移除独立「允许扩展 ID」按钮（扩展授权并入用户确认的安装路径），经合成空间复核；概览「已设置关闭」计数使用各变量解析后的 configured 状态。视觉稿仍属候选 |
| macOS | 当前 `d0851c7` arm64 App 本地构建成功，约 14.46 MiB，含六场景帮助／独立工作保全／统一 CLI 共用 core/remote、Chromium／Firefox 扩展与 browser host；没有本轮 Linux runner bundles，缺资源明确拒绝安装。默认包排除本机可选字体，typecheck、Vite 与 Tauri release 构建通过；实际包内 host/extension 2/2 synthetic 安装测试通过。本轮界面使用真实 core/host 与浏览器合成 bridge，未完成新 native WebKit／真实 VPS 交互验收；没有自动重启旧窗口。未 Developer ID 签名、公证、正式分发或 Applications 安装验收 |

统一入口 [tests/verify.py](../tests/verify.py) 聚合 root、独立 desktop/native-host Rust、JS、Python、前端与八条合成 journey；浏览器、Linux runtime 与 native bundle 显式选择。最终 clean `d0851c7` 的 [CI37249445342](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37249445342) 默认各 17/17、独立浏览器／界面各 5/5、Ubuntu OpenSSH 1/1 与真实 VM 1/1 全部通过；六份入口报告 clean HEAD 一致，详细 startup／PAM／service／reboot／共享认证报告已核对。当前本机 App 与独立 installed CLI 均来自 `d0851c7`；本机构建／安装与包内资源核验是独立证据，后续文档提交不重跑未变更的源码验证。CI 构建并运行 x86_64 静态 musl runner；当前 App 未打包本轮 Linux runner，aarch64 runtime 仍未验收。历史与局部用例证据保留在 [acceptance-status.json](acceptance-status.json)。

实际主机为 macOS arm64。Linux x86_64 musl 已有上述 CI runtime 证据；aarch64、正式 Chrome/Edge/Firefox、真实 Claude 身份与平台认证机制没有实机验收。细项证据在 [acceptance-status.json](acceptance-status.json)；局部测试不自动完成整个验收用例。

## 完整目标仍缺少

1. **真实认证与写入者控制：** Keychain/共享 profile、Desktop/IDE/service 的完整定位与生命周期；有限 systemd 暂停已接通，其他 supervisor 暂停、重新登录及旧会话续用。当前识别部分 Claude 进程及可访问 manager 的直接 root 绑定服务；明确要求目标写入者先停，不能归属的进程阻止清理。官方 auth 命令仅有合成 CLI 证据，不能宣称生产注销已验收。
2. **完整外发策略与强约束：** 七项自定义已接入；更多取舍、入口实效矩阵、macOS Network Extension 签名与权限、Linux namespace；直接 socket、UDP、DNS、NO_PROXY、子进程不能由代理覆盖证明。
3. **远程运行：** 用户级安装／版本绑定已接入候选；Ubuntu 隔离 OpenSSH 已验证安装／丢 ACK 核对／原任务查询／交互 PTY，真实 VM 已完成有限 system-manager 暂停恢复、符合既有条件的 system/user manager 严格 logout 后续跑、原任务完成与 reboot 后核对。setsid 不保证 logout 存活，托管路径也不保证 reboot 续跑；生产 VPS 安装与任务实跑、aarch64 runtime、其他 supervisor 和完整 G03 仍未验收。
4. **浏览器旅程：** 独立 Chromium 临时测试 profile 已完成真实持久安装／onStartup 两阶段清理；仍需正式 Chrome、Edge、Firefox 与 AdsPower 的安装、配对与清理验收。Firefox 独立 CacheStorage、按站点 proxy、容器后台停写、克隆识别和专用 browser 启动仍有缺口。
5. **发行与维护：** 菜单栏、定时漂移、签名规则更新、卸载、升级、正式签名/公证、性能预算。
6. **并发与恢复：** 不合作的外部编辑器或已持有文件描述符的 writer 没有 OS 级 CAS；不确定副作用需核对。混合状态加密备份不支持自动恢复；会话/记忆迁入不证明可以续聊。

这些是原始完整目标的差距，不是缩小后的新 SPEC。下一关口是正式浏览器／AdsPower 与非开发者安装、真实认证／入口矩阵、生产 VPS 任务验收。隔离 runtime 与合成数据能证明机制，不能替代真实产品场景；现有个人登录和生产服务不作开发 mutation fixture。

## 旧 machine-readable 汇总（7a05291）

2026-10-05: Development candidate, full SPEC/G01-G04/U01 incomplete. Repository remains PUBLIC; original-material public-reuse license is unselected and no release is implied. Latest clean 7a05291 CI37216354079 passed macOS/Ubuntu defaults 13/13 each, independent browser/UI 4/4 each, Ubuntu static-musl OpenSSH 1/1 and real Linux VM 1/1; all six entrypoint reports identify the same clean HEAD. Core 52/52, runner 8/8 and VM controls 26/26 passed on both platforms; desktop defaults 56 passed/3 independent ignored, Python SSH 13 passed. Runner selects one transient unit for already-root systemd or an existing lingering current-UID user manager, otherwise setsid with an explicit limitation. Private worker stdin preserves caller environment and cwd without credential-bearing unit properties/argv/journal/spool; exact cgroup and execution facts persist before ACK. Strict KillUserProcesses=yes VM logout left both eligible manager workers alive and their original policy tasks completed; ineligible setsid reconciled. Actual reboot changed kernel boot ID and both managed/setsid accepted jobs reconciled on repeated original query without resubmission. Managed shared-auth changes rejected before accept without logout, deleting synthetic credentials or persisting environment values. Complete root service live-hold/timer/manual-start/neighbor/external-conflict/independent-resume/inactive-state and persistent hold reboot recovery passed. The App reopen/original-query/full-receipt/separate-recovery journey passed in rendered Chromium with synthetic invoke/SSH/registry. Existing full persistent Chromium two-stage smoke retains all 11 assertions and production onStartup evidence; App pairing uses real core/host processes with synthetic invoke/clipboard. Bundled non-fixture extension and host preparation/owned updates remain approved current-user operations; formal browsers and developer-free installation remain unverified. Local 7a05291 arm64 App candidate is 17.87 MiB with exact bundled musl runner bytes/digests and supplied icon verified; it has only linker ad-hoc signing, no distribution certificate, notarization, Applications installation or activation. Sidebar lintel_ and 2.6-second slow blink/reduced-motion static behavior remain. Formal Chrome/Edge/Firefox/AdsPower, native WebKit/OS clipboard, real Claude/auth/model effects, production VPS, aarch64 runtime and full product journeys remain unverified. Latest CI: https://github.com/IndelibleVivi/lintel-cc/actions/runs/37216354079

## 2026-10-19 归档的阶段证据

以下保留 10-19 收束前的逐轮观察。其“本轮”“当前”“最新”均属于各段当时的 source／candidate，不覆盖 [当前状态](current-state.md) 的新证据；历史 CI、旧包与 runtime 范围不扩大。

### 交付与证据（归档时文字）

| 对象 | 当前事实与验收边界 |
| --- | --- |
| 2026-10-17 配方个人默认标注修复 | “减少正文记录”槽位曾把任意已保存意图都显示为“关闭内容记录”承诺。现按已保存意图与内置预置的语义比较（无关对象键序、仅计入有效普通字段）选择名称：不同于预置的个人默认显示“个人默认 · 方案名”与按实际启用／关闭／移除计数生成的说明，不再承诺记录已关闭；匹配预置或未保存时保留原名。编辑保存与“把当前方案存为个人默认”后标签即时更新，切换主机／环境仍按各自意图恢复；两语言一致。不改 recipe slot／键、不新增 personal_default 持久化、不迁移 schema、不限制合法选择。证据：recipe-test 新增语义比较／开关记录／双语标注断言（在干净基线源码上失败、候选通过），client-control-ui 新增开启记录保存、双语标注、环境切换与行内编辑刷新断言并在 built App＋真实 core 通过；recipe-test、i18n-test、i18n-coverage-test、desktop-typecheck 均通过。主环境 canonical entrypoint 的 recipe-test、i18n-test、i18n-coverage-test、desktop-typecheck、client-control-ui 五项全部通过，UI prerequisite 重新构建 runner 与前端；原生 WebKit／安装态 App、Release 与网站仍未更新 |
| 2026-10-17 保护方案与独立试玩 | 27 项按用途分组，普通行直接选动作、显示本环境配置与待预览状态；批量编辑独立进入，只提供共同适用动作并计数隐藏选择，一次完整更新草案，支持逐项／整体撤销。API 正文以关闭／内联／本机目录呈现，目录只填路径；长原值与规则按需展开。预览不会把开启／指定值误标为保持；masked 接收端仍可替换普通地址，请求头不写新凭据。保存默认报告被省略的地址／目录字段，保留当前草案与非秘密移除意图；存储失败明确提示。built App＋真实 core 的多项同一计划、撤销、类型提示、空值、内联／目录、masked 地址替换、未执行时原件不变、900／1120 日夜与 English 已通过本机检查；i18n、baseline、drift 与 recipe 相邻检查通过。clean `3769d52` 已经完整 beforeBuild 资源入口重建独立未签名 debug 试玩，沿用原合成 HOME／state；真实 WebKit 的普通行／批量入口、模式切换、API 目录输入、空值阻止预览与撤销已实际观察，检查用草案已撤销，窗口留在完整逐项列表。完整批准执行、长原值两尺寸日夜与语言矩阵仍以 Chromium＋真实 core 证据为限，不扩大为完整原生操作验收。未替换发行准备候选或生产网站，审美接受仍由用户判断 |
| 2026-10-17 App 界面收尾 | 顶栏保留主机／环境，左侧橙色 `>_` 无外框或附加箭头、光标默认静止；准确路径、ID 与客户端向下展开，正文顺势下移，无浮卡或阴影。界面动画、Logo 光标与终端光标各有本机独立开关，默认只保留 Logo 慢闪，系统减少动态优先；跑道关闭装饰保留玩法。外观图标保留可访问名称。快捷配方把主应用与次级编辑／保存分层；会话启动记录与本地范围统一字号／间距，文件完整路径按需展开。两尺寸 Day／Night 实际 Chromium 截图已目视；准确路径／复制、键盘关闭、modal 优先、三项偏好持久化／系统优先、安静跑道与正常文档流通过合成检查。clean `ae49d1c` 已通过完整 beforeBuild 资源入口构建独立、未签名的 debug 试玩，沿用原合成 HOME／state；真实 WebKit 首页、顶栏与向下展开已目视，三项原生控件的独立值变化／恢复及静态设置布局已核对。光标动画时序、系统减少动态与重启持久化仍以 Chromium 证据为限，不扩大为完整 native 操作矩阵。`878482f` 发行准备 App／DMG 未重建，用户审美接受不由测试推定 |
| 2026-10-16 客户端控制增强源码 | 已接通 [客户端控制增强 mini spec](specs/client-control.md) 的四组能力：v5 规则 `claude-privacy-v5-2026-10-16` 的 27 个字段、五类动作／按字段限制、凭据遮蔽与显式高级 JSON 编辑；有限本地数据清单（已知位置＋明确 `extra_roots`、分页摘要、独立 `lintel.residue/1` 加密备份／精确清理／原环境原路径恢复、`userID` 按需显示／复制／加密保全后重置且不注销）；脱敏语义快照、逐项接受比较基线、打开环境／Lintel 启动时的轻量配置检查；三条可编辑配方，每次重新预览。App／CLI／有限 SSH 使用同一 core 与计划／批准／原 job 链；观察和基线接受不创建 mutation job。**本机证据：**workspace Rust、独立 desktop Rust（50 passed／2 既有 ignored）、真实 CLI 合成旅程、有限 SSH、catalog／配方／TypeScript／frontend build 通过；built App＋真实 core 的一次合成批准、所选原字段恢复、独立残留备份恢复、语义接受、跨环境实际写入、新配方 ID 与 900×640／1120×760 Day／Night 通过。展开的下拉／口令／数字／JSON 表单已目视，长表单保留固定页眉与操作栏；相邻 baseline／drift／i18n UI 通过。**边界：**这是源码与 Chromium 合成 HOME 的验收，native locale／clipboard／Finder 为明确建模，尚未完成本轮新增操作的 native WebKit／Finder 旅程；Claude exporter／身份重新生成、真实账号／生产 SSH 仍未验收。没有重建／安装 App 发行候选、Release／feed 或网站部署；新增三个独立检查不改变隐式默认组，命令见 [验证说明](verification.md#客户端控制增强) |
| 2026-10-15 独立试用接续 | 已完整核对提供的 10-14 Linux 试用报告与六文件脚本：报告基于 `eb9bdd6`，记录非 systemd x86_64 Linux 上精确三件归档、原 ID 完成、六份源文件不变、独立 state 读包，以及 portable-work／execution-admission 回归通过；原云主机与 fixture 本轮未复核，属于报告证据。现将精确选择、独立 inspect/read、全部原件字节、口令／暂存清理断言合入既有 `journey-agent-cli`，不新增检查入口或更改默认组。当前本机 Darwin arm64 实跑结果见 [验证说明](verification.md#独立试用接续)；没有产品行为或候选二进制变更 |
| 2026-10-15 原生首用复核 | 同一 `878482f` DMG 只读取出、原 SHA256SUMS 核对通过；macOS 26.5.2 arm64／全新合成 HOME／state 的真实 release WebKit 已完成登记、精确三件选择／预检／预览、一次批准归档与 completed 回执。六份源 bytes 不变，只有一个原 job／一个环境，无新 root。原生 Quit 后重开，Records 保留原 ID completed；从独立文件入口重新解锁并阅读指令／记忆／会话，结构化与 raw JSONL 均已观察。另一独立 HOME／state 的 CLI 对同一密文核对三份完整 bytes／摘要，无源归档 job；口令未进入隔离持久文件，正常结束无解密 staging，测试 App 已退出。早期 AX／无窗口失败保留，后续经 LaunchServices、明确窗口／截图绑定及经用户允许的前台操作完成。双语 [首用程序](app-updates.md#候选首次使用验收) 已对齐；真实下载／quarantine／Gatekeeper、干净 Mac 首装、签名／公证仍独立未验收 |
| 2026-10-15 CI 收口 | `1bf82b9f06a72a3c2a9389a1337a2f8f50f5819b` 的 [full run 38626316607](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38626316607) 已直接核对 completed/success，14 jobs 全部 success：两平台 frontend／native／browser／runner／heavy、controller、Linux VM 与 aggregate。包含增强后的 CLI 回归；这是该源码的 CI 证据，本地 `878482f` App 的原生观察分别成立，不表示候选已重建或正式发行 |
| 2026-10-09 Preview 修复 | 通用 execute／submit 在新 job／ACK 前拒绝 launch／resume／未知计划，原 ID 查询保留；Markdown 不假设 records，JSONL 选取保留真实 byte offset／content block；ARM 两侧架构名归一后 canonical 包可被 native inspector 核对；漂移检查及接受绑定主机／环境／请求世代；browser expiry 与 mutation 串行且只消费本次暂停记录；catalog 区分预览和 exact-plan／TTY 执行；独立 import coverage 依据全部逐文件发布／sync／readback 完成记录。workspace／两个独立 Rust crates、15 条 CLI journeys、Python controller、JS 与 frontend build 通过；work／baseline／archive-wait／drift／site／clawd-app UI、独立 browser restart／pairing 通过，网站检查使用实际电影。同版本 CLI 解包首用通过；`bb9fa824` 原生 CLI 核对通过，`afa98f66` 原生精确归档／会话阅读／锁定和游戏检查通过，官网未更新 |
| 共享产品合同 | 六任务 catalog；计划新增冻结目标／完整落点与用途；结构化任务结果与覆盖；旧协议、历史计划、包与原 ID 恢复保留 |
| App 多语言源码 | shared LanguageSwitch 顶部只显示实际语言 EN／中文与小箭头，点开原生菜单提供跟随系统／简体中文／English；设置保留完整选择方式与说明。本轮 frontend build、catalog／coverage 与 built-App i18n-ui 已通过；900px Day／Night、实际语言短标签、三选项、可见键盘焦点与原状态保留有合成证据，native 手动→System 已在本轮实际 release WebKit 中观察。最新入口已进入本地 `878482f` 候选。简体中文／English，默认读取 macOS App preferred languages，手动选择本机持久保存；两套随包文案与原生菜单同步，未知语言使用 English。切换保留页面、表单、目标、任务、原 plan／hash、问候身份与工作原文；CLI／扩展／官网语言独立。验证与候选状态见下方，源码更新不等于旧候选升级 |
| App 界面 | 安静的 Clawd 首页／56 句原创时段问候与日期／整分钟／稀有彩蛋；日期只在当天首次打开显示，点击后展开六任务，保留持续目标；配置 root／项目 cwd 分离；资料阅读／交接稿；CLI 核对与 Agent。默认 1120、最低 900、宽 1440 成品 UI 检查通过；保护／保全／清理底栏独立于正文滚动 |
| 2026-10-06 连续性修复 | 本机→A／B→A 打开 A 的回执后，查询、Agent 包及启动预览均固定 A；参考指令与重名副本再次保全，不自动激活。官方注销将完整认证状态摘要冻结在私有计划，主体缺失拒绝，执行前及保全后复查；只验证 synthetic CLI，Keychain-only token 世代仍未验证 |
| 关联组件与容量预检 | core／named CLI／App 接通有限组件清单、明确 project cwd 来源、认证位置元数据、原任务 coverage 与最多 8 个原 unit 的当前核验；账号未知、Desktop／IDE 暂不支持、浏览器独立。容量预检不读正文，列出超限文件与未覆盖路径；FIFO 不打开、不计入；按类别或精确原件重新检查。有限 SSH controller 已放行这几个明确只读入口 |
| 工作选择与流式包 | 当前 runner 准入 256 MiB/文件、1 GiB 合计、10,000 文件，以 age + lintel.work/1 完整核验；App 精确选择与 named CLI 共用 core。持久 JSON 读写另共用 16 MiB 上限，过大深路径计划保存前拒绝，精确缩小选择仍可继续。发布与迁入必须完成文件／目的地父目录同步和完整读回；失败保留原路径与 ID、正常回收本次未发布临时名。release 已实跑四个 256 MiB 合成原件的 1 GiB 归档／读回、独立 inspect、首中尾页、import 与原字节摘要一致；另有 10,000 原件的完整归档／inspect／import，全部副本逐字节核对。具体成本见验证说明 |
| 会话与启动 | reader 每页 ≤256 KiB，每次先完整认证并私有暂存全包；原字节、CRLF、未知块和 opaque thinking 保全。resume 在解包后、首次 intent／复制前及复制后启动前复查冻结 target／startup；成功 exec 前明确关闭整包明文暂存，失败阻止启动并保留原 ID。真正 PTY 与 inert Claude 的清理路径通过，不等于真实认证／续聊。静态支持仍限 2.1.283／2.1.285 与 ≤8 MiB transcript |
| 历史 native runtime | 2026-10-05 unsigned arm64 Tauri App 构建通过。隔离 HOME／state 的 native WebKit 已实际创建环境、加密保全并建立新 root、逐字节读回三份资料。默认与 900×640 窗口已检查首页／任务入口，900 审批正文滚动后标题与底栏保持固定；Night→Day 控件换色已实际通过。宽窗口受当前屏幕限制，完整原生矩阵仍未逐一验收；截图为本地 QA |
| 阅读等待交互 | `archive-wait-ui` 在 built App 验首读、翻页、旧页标记、跨页面队列占用、错误释放和迟到结果隔离；1120 Day／900 Night 已目视。persistent activity 只含有限操作名、主机标签和队列状态。Chromium 合成延迟不证明 native WebKit 或解密性能 |
| Release 资源实测 | macOS arm64 release，scrypt log N 19、四个确定字节模式原件共 1 GiB：archive/readback 120.6s，inspect 70.9s，每页 59.1–66.3s，import preview 57.4s／execute 54.3s，峰值 RSS 518.2–518.5 MiB；10,000 文件 import 194.7s，峰值 RSS 550.8 MiB。整个 fixture 的已分配块采样最大约 6.51 GiB，含原件／密文／副本，不能当纯暂存峰值。部分步骤与本地 QA／build 并行；这些是此次成本观察，不是普适性能预算 |
| CLI 与分层验证 | 本轮默认 canonical 28 selections 首次为 25 PASS／3 FAIL，原报告保留。workspace 因合成 SSH 并行 deadline 失败后，RUST_TEST_THREADS=1 的完整 workspace 复验通过；desktop updater 拒绝软链接 target，改用真实 target 路径后 48 passed／2 既有 ignored。debug large-work 达 1800s 上限；同源码 native release runner 的独立 8 项大包旅程通过。18 项 UI／browser selections 均有最终通过证据，首次失败与修正等待后的结果分开记录；notices 9 项与 CI controller self-test 通过。Linux VM 与隔离 OpenSSH 后由下方 clean `878482f` 完整 CI 重新通过；生产 SSH／其他架构 runtime 未在本轮验收 |
| 本机 Native 构建 | clean `878482f` 的 `npm run desktop:build` 完整入口通过，保留 beforeBuildCommand；随后 `tauri bundle --bundles dmg --ci --no-sign` 完成新 DMG。Rust 1.98.0；两种 musl runner 在本轮用既有 Zig 0.16.0 重新构建并通过 static ELF 检查，其产品代码在本次 notices 接线修复中未变。版本仍 0.1.0，identifier／未配置 updater 合同不变 |
| 资源与包装 | 新 App 46 文件／36,297,881 bytes；DMG 12,205,266 bytes。hdiutil verify、只读挂载、复制件文件字节／权限、native executable 与全部 canonical resources 一致；18 份 notices 资源含新增阅读说明、frontend、387 个 Rust crate 的目标依赖超集和当前 Rust／Zig 工具链说明，不是 exact linked SBOM。只有 linker ad-hoc signature，无 Team ID、资源签封或公证。候选与证据不进 Git，旧包保留 |
| Preview 试用入口 | 从新 `878482f` DMG 取出的 release App 在隔离 HOME／CFFIXED_USER_HOME／state 实际运行，观察到 System English 首屏与真实 core 模块设置页；Info.plist 核对版本 0.1.0。后续版本面板自动化受 elementHasNoFrame／ScreenCaptureKit 错误影响，未计为通过。完整 System English → 简中 → System English、原生菜单、exact-source 链接和游戏运行／碰撞／重试／P 暂停对应同日 `cdb0ba8` 包，详见候选历史；两包产品代码未变。两次均无 mutation jobs，测试 App 已停止。不替代 quarantine／Gatekeeper 首装、完整 native 操作矩阵或真实认证；无公开 Release 下载 |
| 官网／仓库双语源码 | `/` 简中与 `/en/` English 为完整静态页面，175 stable-key paired phrases 与生成一致性、元数据、指南链接检查通过；六任务共用一份文字与就近限制；标题旁加入 Lintel 原创细线小图标，另配交错 SVG 解释场景，手机先文字再图。README／quickstart／文档 hubs 及十一使用／操作／模块指南提供维护中的双语路径，中文 canonical；28 reader files 的 447 本地链接／anchors 与 64 对 fenced blocks 核对通过，规格／当前状态／开发证据仍明确标中文。完整实际媒体 site-ui 在最后视觉修复后通过 31 groups：两路 no-JS、1440／390／320 日夜、标题小图标／交错场景／SVG 标签与文字布局、same DOM／canvas／video、details／旅程／分数／主题及播放中的语言 back／forward；图景已目视。有限 payload 24 files 加 provenance manifest，影片与海报沿用原审阅字节，无 App release record。源码／本地检查与正式部署分别记录；双语、组合图景、开场与跑道修订已进入下方 clean `c80a783` 生产 |
| 共享 Clawd 跑道源码 | 按下跑道／按钮／Space／↑ 立即起跳，短按保留原弹道；按住只在同一次上升中加高，0.26 秒封顶，松手不补跳、持续按住不自动连跳。初速 260px/s，连续渐增至 350px/s。背景恢复原主题单色像素／字符风格山线、云与日月，树林／庭院只增加同一墨色的像素标记；普通 +25 星沿轻跳轨迹排列，较高障碍会带着普通星上移，避免越障后漏收；每第四颗高处 +50，四类障碍、疏密间距与恢复窗口保留。27 个 pure tests 验证 30／60／144fps、260／280／350px/s 的输入／弹道／最大障碍／连续跑道与多个正常起跳时机；连续字段核对每颗已过身边的普通星均被正常轻跳收下；canonical game 2.503s、实际影片 site-ui 256.917s、frontend build 2.674s 与 build 后 clawd-app-ui 11.932s 全通过。独立 Chromium 开局日／夜 canvas 与原版逐像素相同，此前三场景图保留；本轮隔离 Chromium 的 390px Day／320px Night 受控 seeded 跑道经真实键盘输入各收下连续六颗星，包含 +50 高星，截图已检查。原 score keys、单 RAF、暂停／取消／locale／卸载保留，仍只有一套 runner。clean c80a783 完整本地版本经用户试玩认可并已更新生产；新 cdb0ba8 App 已包含同一 runner 并观察 native 运行／暂停；未安装到 Applications |
| 共享 Clawd 运动修复 | 身体保持水平，四条连续短直腿分两组交替抬起与前后移动，支撑腿贴地，无横向小脚；每 100px 行进一周期。起跳弹道与 140ms 落地收势保留，静态 BODY／电影轮廓不变。15 项游戏测试覆盖 30／60／144fps 与 210–350px/s；此前本地网站 canvas 连续 75 帧胸口高度一致、支撑脚底一致，61 种腿部像素状态。独立 `site-ui`（含真实影片）、built `clawd-app-ui`、desktop build 通过；此步态修正先进入 clean `afa98f66` App 与静态网站 payload，现已进入正式官网；原生游戏启动／计分／跳跃／暂停／继续／Escape 关闭通过，四张实际连续帧显示身体高度稳定、短直腿变化；正式官网更新证据见托管行，视觉接受独立于检查结果 |
| Preview 视觉源码 | 首屏增加留白；完整微圆角橙脚与独立慢闪下划线；独立扩散／渐隐／2.6 秒清理的水纹与三幅 Clawd 的短暂反应，纸条指导／发现计数已退休。正文夜景以柔和暖灰衬托原色角色，保留深色双眼；共享游戏恢复连续短直腿与整体动作，星星真实高低变化。最后修改后 App frontend typecheck／build、canonical site-game-test（11 tests／240 个速度、障碍、星高组合）、site-ui、clawd-app-ui 均通过；1440／390 日夜、320 布局、900 contain／手机夜色合成像素、触摸／键盘、水纹到期清空、离屏停止、无 JavaScript 正文与无外部请求已核对，最终图景已目视。后台分支为合成 visibility，App UI 为 Chromium／合成 transport；网站已在正式域名上线，证据见官网托管行；历史 `afa98f66` Native 候选已重建并完成上述游戏交互检查，视觉接受独立于检查结果 |
| 概念 intro | [独立制作入口](../scripts/preview-film/README.md)已实跑：20 秒、1920×1080、60fps／1200 帧的连续字符空间镜头，生成带英文配音与纯音乐／SFX 两份 MP4，共用编码视频。Clawd 使用共享 connected Sprite 的清晰矢量，脚沿桥拱走、前栏按深度遮挡下部、包裹随身体整体轻起伏。原创 Tone.js 音乐／SFX 与本地 Kokoro speech 保留分轨；最终 AAC 配音版 −16.97 LUFS／−1.66 dBTP，纯音乐版 −14.55 LUFS／−1.54 dBTP，为此次测量而非通用响度承诺。两份媒体完整解码通过，13 个实际编码抽帧已检查；本地播放器无 autoplay／外部请求，暂停与播放中切换音轨均保留进度。导出 manifest 如实声明 `0172ccdc`／dirty，并保留与当前 film 源码一致的快照；原版电影留在 ignored candidate 输出；网页带配音版约 16.4 MiB，完整解码通过并在正式官网实际播放，原版单独保留 |
| 官网导航验收修正 | `8cbc335` 的 [full run 38261753431](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38261753431) 为 failure：只有 macOS site-ui 在第二次原生锚点导航超时，其余两平台 frontend／native／runner／heavy、Ubuntu browser 与 Linux VM jobs 成功，aggregate 正确拒绝。受控 A/B 重现旧 fixture 在第一次 smooth scroll 尚未停止时进行 instant reset／下一次 Enter，形成 compositor 竞争；保持产品原生链接与 smooth 行为，fixture 改为位置在连续三帧稳定后才交接下一次输入，同一四秒落点标准保留。修正后的 `47f8723` [site-scope run 38266721447](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38266721447) completed/success，包含 Ubuntu frontend／browser、controller 与 aggregate；标题小图标＋交错场景的最终本机 Darwin canonical site-ui 也已通过。不能把原 full run 标绿 |
| 最近已完成 CI 观察 | 普通星高度／障碍配合与懒加载测试前置顺序修正、候选记录的 `eecbcfc5b6bcd2863414dc3e0fe05495053e8a6a` [run 38325613669](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38325613669) exact SHA completed/success。此前 `c4c3239` [run 38321362355](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38321362355)、`78a5de5` [run 38283216546](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38283216546) 与 `fdf18b2` [run 38272491765](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38272491765) 各保持其 source scope；CI 不部署官网 |
| 此前 CI 修复证据 | `0172ccdc974fbad2920da10dd55a06584ba22c68` 的 [run 37787248424](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37787248424) 已直接核对为 completed/success；macOS／Ubuntu defaults 与独立 browser／App pairing 均通过，Ubuntu static runner、OpenSSH、CLI 包装及 disposable Linux VM lifecycle 也通过。此前 `20e9872` 的 [run 37763131837](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37763131837) 在 Ubuntu `site-ui` 等待 reduced-motion 状态时超时，50ms polling 未单独消除问题。当前 `site-world.mjs` 在 mount／change 时缓存 media-query 状态，动画帧只读缓存，保留系统事件、暂停、静态反馈与有限生命周期；修复后本地 canonical `site-ui` 两次通过（41.287s／33.35s），未延长 timeout 或降低断言。Linux 依赖步骤保留 15 分钟总时限、30 秒连接／数据 timeout、两次文件重试与索引失败直接退出，原包名与后续 gates 保留；见[验证说明](verification.md#linux-ci-dependency-preparation)。此 run 的源码范围止于上述 SHA，独立电影工具的制作与媒体检查另行记录，不借该 CI 声称视频美学验收 |
| 已部署官网 payload | clean source `c80a783c8755bc6a76de6870ba95ff739d9d57e9` 的 24 文件有限包加 provenance manifest，26,708,586 bytes；所有源码与该 commit、影片／海报与原审阅字节逐字节相同，无 App release record／feed。25 条目 Dashboard ZIP 逐字节等于冻结包，同一 ZIP 先 preview 后 production。Dashboard 未填写 Git commit metadata；来源声明来自原包 manifest，实际 runner 与 CSS 已在正常缓存浏览器读回核对。前一本地 a064c86 基线包保留，78a5de5 彩色／松手蓄力包退休 |
| 官网托管 | Direct Upload Pages project `lintel`，production `0b6e2ece-ef40-4767-a6a6-52a7c4e31868`（[实际 deployment](https://0b6e2ece.lintel-aue.pages.dev/)），preview `27a23001-9ac6-437b-80dd-161cfc28dce1`（[实际 deployment](https://27a23001.lintel-aue.pages.dev/)）；同包 Dashboard 上传均成功。正式 [中文](https://lintel.page/)／[English](https://lintel.page/en/) HTTPS 200；正常缓存回访取得 styles.css?v=20261012 与 clawd-game.css?v=20261013，实际两份 CSS 与 runner 均与冻结 c80 包逐字节一致。正式网站启动／Space 轻跳／暂停、暂停分数在中英切换后保留、320px English／390px 中文夜景无横向溢出已观察；实际影片由原生控件播放至 20.01s（1920×1080），MP4 206／video/mp4 与字幕 text/vtt 已观察，关闭暂停并归零；无相关 console error 或页面外部请求，CSP 仍生效。CSS 实际 max-age 为 14400，继续使用版本入口；未修改账号缓存、DNS 或权限。Wrangler Keychain 读取失败后改用已有 Dashboard 正常登录，没有提取凭据或修改钥匙串。旧 fdf18b2 及更早部署证据见 [候选历史](candidate-history.md)；程序见 [site-hosting](site-hosting.md) |
| 发行与账户 | 无正式签名／公证、Applications 安装、真实认证操作或生产 VPS 激活；官网上线和公开源码不等于正式 App 发行；未上传社交平台 |

发行准备的 clean-checkout 回归：`4a5a635` 的 [run 38343526714](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38343526714) 原生 macOS job 因不存在的 frontend notice 资源失败，其余该 job 的 14 项检查通过。资源现统一从带 tracked README 的 notices 目录收集，完整 App 构建仍先生成 frontend 并拒绝缺失正文；本机临时移开 dist 与全部 generated notices 后，原生 48 项通过、2 项既有 ignored，9 项 notices 回归通过。该修正不把原 run 标绿，也不把只含 README 的开发目录当作发行包。clean `878482f` 的 [run 38344346775](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38344346775) 已核对 exact SHA completed/success：两平台 frontend／native／browser UI／runner CLI／heavy work、controller、隔离 Linux VM 与最终 aggregate 全部通过。此 CI 覆盖源码与一次性测试环境，不替代生产主机、签名或真实下载首装。

[基线验收记录](specs/product-baseline-status.md) 保存 A01–A18 的逐项证据与未验证条件；[候选历史](candidate-history.md) 保存此前候选的资源／CI／runtime 观察。历史 Linux VM 与 browser runtime 证据仍限定于当时源码和一次性环境，不证明这轮新增路径已验收。

保护方案重做代码 `3769d52f3299bbd43ba2ffc0007b3f792f021204` 的 [app-ui run 38953026480](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38953026480) 已直接核对 exact SHA 为 completed/success：Ubuntu frontend、15 项 browser/UI 组、controller 与 aggregate 通过；native Rust、runner/CLI、heavy 与 VM 按 profile skipped。本机独立 debug 试玩的 WebKit 观察范围见上表，CI 不替代原生完整操作或正式发行验收。

客户端控制源码 `8c3af4328b177428726108b4790897ff8be1646e` 的 [full run 38912008173](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38912008173) 为 completed/failure：Ubuntu 的合成操作 CLI 在 45 秒子进程预算耗尽，客户端控制 UI 在 12 秒弹窗等待到期，aggregate 正确拒绝；该 run 不标为通过。当前测试对三次独立加密保全提供有限 120 秒执行预算，UI 等实际 core 回复后继续核对原 ID／字节与回执，并保留有限错误代码诊断；本机合成 CLI 与 UI 已通过；`3308aca` 的 [后续 run 38935245688](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38935245688) 中 Ubuntu `journey-client-control` 已通过（161.174 秒，整条旅程），两平台 native／frontend／runner 和 macOS browser 组通过。Ubuntu browser 继续抓到新增范围截图等待 `cleanup_inspect` 的 12 秒 UI 等待到期；该截图改为先等实际只读回复，保留正常 core 命令预算。`3308aca` run 最终 cancelled，不能声称 full aggregate 通过；`5e6336a7f5c8541413e08dcaa6f959b1e7da51bc` 的 [app-ui run 38937377887](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38937377887) 已直接核对 completed/success。包含独立动态开关与图标左移的 `ae49d1cd0d25cee1ab69ccc5d5d2697b935013ff` [app-ui run 38939449187](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38939449187) 也已直接核对 completed/success：Ubuntu frontend／browser、controller 与 aggregate 通过，native／runner／heavy／VM 按计划 skipped，不借用此次 UI CI 扩大 native 或完整核心验收。

### App 简中 / English 源码更新

两套随包文案覆盖 App 页面、审批／结果、帮助、状态、可访问性、问候与游戏；CLI、扩展 popup、官网和外链文档保持独立。顶部与设置都可选择跟随系统／简体中文／English；macOS 只读 preferred languages，原生菜单同步。没有在线翻译或新增 runtime dependency。源码已接通，当前 `e822d74` App 候选包含双语与 minimal 入口；旧 `47158cd`／`25a8649` 包保留，没有重发 Release 或替换安装。

本机 catalog／插值／原文投影／格式／问候检查、frontend build 与严格 CI controller self-test 已通过；新 `i18n-ui` 验首屏、持久化、迟到和乱序系统读取、手动覆盖、草稿、原审批 ID／hash及 900px Day／Night。Clawd 的切换保留同一 game／canvas、暂停状态、分数、星星、最高成绩与动画生命周期；规则错误随当前语言显示、草稿不变且不提交通道操作。相邻 App／网站／browser pairing 旅程使用独立 synthetic fixtures，详细命令见[验证说明](verification.md)。

standalone desktop Rust 48 项通过，2 项既有安装前置检查 ignored；不是通过项。新 native debug executable 在隔离 HTML／WebKit harness 实际调用 preferred languages、简中／English 菜单切换与非法语言拒绝，菜单栏文字已观察；生产 getter 还在进程内 volatile `AppleLanguages` 简中顺序下返回匹配结果，不写持久系统语言。

解锁后，源码 `65495b7` 的真实产品 WebKit／native core 在隔离 synthetic HOME／state 下完成首屏 System 模式、简中／English 手动选择、设置面板与原生菜单同步、工作保全路径草稿／分类保留，以及退出重启后保留简中／Night 的检查。三份 synthetic 原件逐字节未变，没有提交计划、任务或归档输出。后台 AX 打开下拉框未交接输入焦点时曾只改显示值；按当前截图定向聚焦后，实际页面／菜单更新与结构化表单状态已核对。当时手动模式回到 System 的 native 操作未验证；本轮 `cdb0ba8` release App 已实际选择并观察 System English 与原生菜单恢复，补齐这一项，未扩展为完整 native 矩阵。以上是选定语言旅程的 dev runtime 证据，不替代其余操作的完整 native 矩阵或已包装候选验收。测试不触碰个人 Claude、browser profile、services 或真实宿主网络。

源码 `65495b7` 的[完整 CI](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38205602479) 已通过两平台 frontend／native／browser UI／runner CLI、heavy work、Linux VM 与 aggregate；双语候选先构建为 `47158cd`，入口简化后另行构建为 clean `e1a2df4`；两者与正式发行保持区别。

### 新增 IPv4 / IPv6 Leaf

当前 main 已接通 [IPv4 / IPv6 Leaf](specs/ip-network.md)。四格实测、严格 IPv4 socket、共享 macOS IPv6 计划与完整恢复已通过本机合成／loopback 检查，以及 built App 的独立 `network-ui`；当前 `e822d74` App 已包含该 Leaf，独立 `bb9fa824` CLI 候选仍保留旧能力范围。Ubuntu hosted CI 已完成当时的默认编译／合成检查；本机现已使用既有 Zig／cargo-zigbuild 完成 x86_64／aarch64 musl runner 构建与 static 格式核对。真实 macOS 系统授权与配置往返、VPN/TUN 切换、公众回显互通与真实 Linux host IP runtime 仍未验收；本轮仅在合成 HOME／state 启动候选，不替换已有安装、不部署 VPS 或正式发行。

### 新增遥测控制与 App 更新准备

[遥测与更新 Leaf](specs/telemetry-updates.md) T01–T04／U01–U05 已接通当前源码。两个官方 Datadog exact-host 目标有 direct Anthropic API／服务端 rollout 条件；混合 API／认证／下载主机只作说明。App 合并下次启动草案，保留手写规则和其它通道字段；设置、当前配置、受控 CONNECT 与普通客户端事件分别呈现。静态目录同时支持 named CLI、raw request 与有限 SSH；当前 App 的受控测试必须匹配原 channel binding、真实 403 与完成事件，不解析或连接公网遥测目标。细粒度保护预设保留，非必要流量总开关关闭 Claude Code 自动更新的附带影响明确展示。

App 自身使用官方 Tauri updater：最新候选与源码均未配置发行公钥／feed，后台检查默认关闭；检查、下载验签、批准安装与显式重启分别执行。安装与重启排除当前 App 操作／活跃代理；原意图、中断和外部记录冲突不重放。本地 release record 联动构建 override、signed-version 校验、官网 DMG 资料和 preview／stable feed；官网投影先要求公开字节核对，二进制留在 GitHub。本轮未生成正式 key、签名／公证、Release、已激活 feed 或生产网站更新，也未安装替换已有 App。完整 operator 边界见 [App 更新](app-updates.md) 与 [遥测](telemetry.md)。

本机 workspace、standalone desktop Rust（45 通过，2 项既有安装前置检查 ignored）、frontend build、release preparation、真实合成 CLI 及有限 SSH 合同检查通过。native 官方 updater 在临时 App／loopback 上实跑 HTTP 解析、坏签名／签名版本拒绝、下载不安装和临时 bundle 替换；不是实际已分发 App 升级。独立 built-App Chromium 的更新、遥测、IP 与基线旅程已验证，invoke／feed 回复为合成；Day／Night UI 截图为本地 QA。真实公网 feed、native WebKit 更新旅程、Apple 信任与旧实际 App 升级仍未验收。[验证入口](verification.md#遥测与-app-updater) 保留检查名与证据边界；新增 selections 已由[候选历史中的 `4d3706f` 两平台 CI](candidate-history.md)核对通过。

### 日常 CI 分组

GitHub Actions 仍是主要 CI owner；`tests/ci_plan.py` 按改动选择 `docs`／`site`／`app-ui`／`browser-ext`／`core`／`full`，由 canonical `tests/verify.py --checks` 执行。纯 UI／官网／文档不等待工作包重检查和 VM；核心、未知路径、CI 控制器、缺失 diff 基线与每周／手动 full 保留跨平台完整验收。各组在 plan 后独立执行，aggregate 核对必需 job 与每个平台的原始报告，选中检查 skipped 不算通过。Cargo／官方基础 VM 镜像缓存不替代源码构建和签名核对；没有接入 Cloudflare Builds 或自动部署。

2026-10-10 核对：本机严格控制器 self-test、73 项 planner／生产 aggregate／workflow regression 与 actionlint 通过。`3d09911` 的首轮分组 [full run 37953451500](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37953451500) 中，两平台 frontend／native／工作包／CLI 与 Linux OpenSSH／VM 均通过；工作包 macOS 16m26s、Ubuntu 22m11s，VM 18m44s，三者独立并行。该 run 的两平台 `baseline-ui` 抓到新增颜色 transition 在暂停动画时保留旧主题文字的回归，aggregate 正确拒绝，所以该 full run 是 failure，不能标绿。

`fe7f136` 恢复已有即时主题配色合同，保留阴影、位移、弹窗／toast 与其余美化；本机同断言先失败、修正后通过，完整 13 项 App UI 通过。对应纯 [app-ui run 37956385539](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37956385539) completed/success：frontend 三项与 App UI 十三项全部通过，native／工作包／CLI／VM 按计划 skipped，aggregate 通过；从首个 job 开始到 aggregate 完成为 **3m33s**。这是一次日常 UI run 的观察，不是每次耗时保证或 native WebKit／正式发行验收；该 CSS 修正未重跑未改变的 core／VM。原先 `4d3706f` 全量 run 的 macOS／Ubuntu 耗时为 24m32s／54m25s，属于旧串行流程。操作和准确边界见[验证说明](verification.md#ci-workflow-github-actions)。
