# 当前状态

2026-10-12 · **Lintel 0.1.0 Preview / 开发预览，尚无正式 Release**。最新本地 macOS arm64 App 已从 clean source `e1a2df4ade484a42a438a4bf09d225d4b7a1a78c` 构建并包装，顶部只显示实际语言 EN／中文，包含 App 简中／English、IPv4／IPv6 Leaf、遥测控制／App 更新准备与 Selen 美化；两架构 Linux runner 已按当前 catalog 重新构建。候选复制与 zip 解包逐文件字节／权限一致，包内资源与此前实际通过 host／扩展合成安装检查的资源一致；隔离 HOME／state 的真实 release WebKit 已观察 EN 入口、原生三选项与 English 首页／菜单。手动回到 System 的 native 点击仍未验证，详见下方；其余完整原生矩阵与普通用户安装保持独立。官网生产仍是 clean `47158cd` 的静态 payload，正式域名实际加载该版短直腿 runner。本轮源码另完成简中／English 官网、六项细线图标能力索引、仓库双语使用指南与 hold/release 蓄力玩法；本地完整 payload 和隔离 Chromium 检查已通过，尚未部署，现有 App 候选也未重建为新玩法。独立三平台 CLI 仍为 `bb9fa824b1a2aa509e749519c5d700eba1bb02be`。未安装到 Applications、无正式 Release／Developer ID 签名／公证，更新公钥／feed 未配置。A01–A18 与完整 SPEC 的 G01–G04 保留全部目标与具体未验证项。

## 当前交付与证据

| 对象 | 当前事实与验收边界 |
| --- | --- |
| 2026-10-09 Preview 修复 | 通用 execute／submit 在新 job／ACK 前拒绝 launch／resume／未知计划，原 ID 查询保留；Markdown 不假设 records，JSONL 选取保留真实 byte offset／content block；ARM 两侧架构名归一后 canonical 包可被 native inspector 核对；漂移检查及接受绑定主机／环境／请求世代；browser expiry 与 mutation 串行且只消费本次暂停记录；catalog 区分预览和 exact-plan／TTY 执行；独立 import coverage 依据全部逐文件发布／sync／readback 完成记录。workspace／两个独立 Rust crates、15 条 CLI journeys、Python controller、JS 与 frontend build 通过；work／baseline／archive-wait／drift／site／clawd-app UI、独立 browser restart／pairing 通过，网站检查使用实际电影。同版本 CLI 解包首用通过；`bb9fa824` 原生 CLI 核对通过，`afa98f66` 原生精确归档／会话阅读／锁定和游戏检查通过，官网未更新 |
| 共享产品合同 | 六任务 catalog；计划新增冻结目标／完整落点与用途；结构化任务结果与覆盖；旧协议、历史计划、包与原 ID 恢复保留 |
| App 多语言源码 | shared LanguageSwitch 顶部只显示实际语言 EN／中文与小箭头，点开原生菜单提供跟随系统／简体中文／English；设置保留完整选择方式与说明。本轮 frontend build、catalog／coverage 与 built-App i18n-ui 已通过；900px Day／Night、实际语言短标签、三选项、可见键盘焦点与原状态保留有合成证据，native 手动→System 选择仍未验证。最新入口已进入本地 `e1a2df4` 候选。简体中文／English，默认读取 macOS App preferred languages，手动选择本机持久保存；两套随包文案与原生菜单同步，未知语言使用 English。切换保留页面、表单、目标、任务、原 plan／hash、问候身份与工作原文；CLI／扩展／官网语言独立。验证与候选状态见下方，源码更新不等于旧候选升级 |
| App 界面 | 安静的 Clawd 首页／56 句原创时段问候与日期／整分钟／稀有彩蛋；日期只在当天首次打开显示，点击后展开六任务，保留持续目标；配置 root／项目 cwd 分离；资料阅读／交接稿；CLI 核对与 Agent。默认 1120、最低 900、宽 1440 成品 UI 检查通过；保护／保全／清理底栏独立于正文滚动 |
| 2026-10-06 连续性修复 | 本机→A／B→A 打开 A 的回执后，查询、Agent 包及启动预览均固定 A；参考指令与重名副本再次保全，不自动激活。官方注销将完整认证状态摘要冻结在私有计划，主体缺失拒绝，执行前及保全后复查；只验证 synthetic CLI，Keychain-only token 世代仍未验证 |
| 关联组件与容量预检 | core／named CLI／App 接通有限组件清单、明确 project cwd 来源、认证位置元数据、原任务 coverage 与最多 8 个原 unit 的当前核验；账号未知、Desktop／IDE 暂不支持、浏览器独立。容量预检不读正文，列出超限文件与未覆盖路径；FIFO 不打开、不计入；按类别或精确原件重新检查。有限 SSH controller 已放行这几个明确只读入口 |
| 工作选择与流式包 | 当前 runner 准入 256 MiB/文件、1 GiB 合计、10,000 文件，以 age + lintel.work/1 完整核验；App 精确选择与 named CLI 共用 core。持久 JSON 读写另共用 16 MiB 上限，过大深路径计划保存前拒绝，精确缩小选择仍可继续。发布与迁入必须完成文件／目的地父目录同步和完整读回；失败保留原路径与 ID、正常回收本次未发布临时名。release 已实跑四个 256 MiB 合成原件的 1 GiB 归档／读回、独立 inspect、首中尾页、import 与原字节摘要一致；另有 10,000 原件的完整归档／inspect／import，全部副本逐字节核对。具体成本见验证说明 |
| 会话与启动 | reader 每页 ≤256 KiB，每次先完整认证并私有暂存全包；原字节、CRLF、未知块和 opaque thinking 保全。resume 在解包后、首次 intent／复制前及复制后启动前复查冻结 target／startup；成功 exec 前明确关闭整包明文暂存，失败阻止启动并保留原 ID。真正 PTY 与 inert Claude 的清理路径通过，不等于真实认证／续聊。静态支持仍限 2.1.283／2.1.285 与 ≤8 MiB transcript |
| 历史 native runtime | 2026-10-05 unsigned arm64 Tauri App 构建通过。隔离 HOME／state 的 native WebKit 已实际创建环境、加密保全并建立新 root、逐字节读回三份资料。默认与 900×640 窗口已检查首页／任务入口，900 审批正文滚动后标题与底栏保持固定；Night→Day 控件换色已实际通过。宽窗口受当前屏幕限制，完整原生矩阵仍未逐一验收；截图为本地 QA |
| 阅读等待交互 | `archive-wait-ui` 在 built App 验首读、翻页、旧页标记、跨页面队列占用、错误释放和迟到结果隔离；1120 Day／900 Night 已目视。persistent activity 只含有限操作名、主机标签和队列状态。Chromium 合成延迟不证明 native WebKit 或解密性能 |
| Release 资源实测 | macOS arm64 release，scrypt log N 19、四个确定字节模式原件共 1 GiB：archive/readback 120.6s，inspect 70.9s，每页 59.1–66.3s，import preview 57.4s／execute 54.3s，峰值 RSS 518.2–518.5 MiB；10,000 文件 import 194.7s，峰值 RSS 550.8 MiB。整个 fixture 的已分配块采样最大约 6.51 GiB，含原件／密文／副本，不能当纯暂存峰值。部分步骤与本地 QA／build 并行；这些是此次成本观察，不是普适性能预算 |
| CLI 与分层验证 | 入口简化 `e1a2df4` 的分组 UI source CI／aggregate 已通过；本机 fresh frontend build、i18n-test／coverage／i18n-ui 通过。本轮网站收口的 canonical site-game-test／site-ui（真实影片）／clawd-app-ui 全过。native／core 源码未因入口改变，包内 Linux runner 按当前 task catalog 构建；独立 CLI 发行候选仍是 `bb9fa824`，未重发，Mac 的 Linux static 格式／字节核对不等于其它架构 runtime |
| 本机 Native 构建 | clean `e1a2df4` 的 canonical desktop-tauri-bundle 完整入口通过，152.576s，保留 beforeBuildCommand。没有改 updater／identifier／发行配置；此前 `47158cd` 与旧候选的构建观察见候选历史 |
| 资源与包装 | clean `e1a2df4` macOS arm64 App 为 30,596,543 bytes（约 29.18 MiB），zip 为 11,880,053 bytes；复制件及 zip 解包的 28 文件逐字节／权限一致。内置 x86_64／aarch64 runner 为 5,895,888／5,196,336 bytes，均嵌入当前完整 task catalog；host 789,392 bytes，Chromium／Firefox、ICNS 与 canonical resources 一致，无 woff2。全部资源也与此前 `47158cd` 包逐字节相同，沿用其 fresh 编译／2 passed 的实际包内 host framing／allowlist 与扩展合成安装证据；本轮没有冒称重跑安装检查。clean docs revision 与 bundle en／zh-Hans 已核对。只有 linker ad-hoc signature，无 Team ID／资源签封／公证；候选、报告与测试数据不进 Git |
| Preview 试用入口 | 最终 `e1a2df4` release executable 在显式 synthetic HOME／CFFIXED_USER_HOME／state 下实际显示 EN 顶部入口，偏好仍为 Follow system；原生三选项与 English 界面／菜单已观察，没有 dev server。输入工具检测到真实鼠标介入后停止，native 手动→System 选择仍未验证；三份 synthetic 原件未变、jobs 为零，QA App 已停止。旧 `47158cd`／`25a8649` 点击限制与更早归档／阅读旅程保持独立。quickstart 与双语源码入口不变，无公开 Release 下载 |
| 官网／仓库双语源码 | `/` 简中与 `/en/` English 为完整静态页面，156 stable-key paired phrases 与生成一致性、元数据、指南链接检查通过；六任务保留紧凑单／双栏索引与就近限制，标题旁加入 Lintel 原创细线图标。README／quickstart／文档 hubs 及十一使用／操作／模块指南提供维护中的双语路径，中文 canonical；28 reader files 的 447 本地链接／anchors 与 64 对 fenced blocks 核对通过，规格／当前状态／开发证据仍明确标中文。完整实际媒体 site-ui 在最后视觉修复后通过 31 groups：两路 no-JS、1440／390／320 日夜、标题图标／文字布局、same DOM／canvas／video、details／旅程／分数／主题及播放中的语言 back／forward；图景已目视。有限 payload 24 files 加 provenance manifest，影片与海报沿用原审阅字节，无 App release record。此行是 source/local candidate 证据，生产仍为下方 `47158cd` |
| 共享 Clawd 蓄力源码 | 轻点普通跳，按住跑道／按钮／Space／↑ 后松开高跳，0.4 秒封顶而不自动起跳；普通 +25，每第四颗高处 +50。暂停、失焦、hidden、pointer cancel／lost capture、resize／卸载清除待释放输入。19 个 pure tests 覆盖多帧率、不同速度、蓄力取消和 320px 看到高星后开始蓄力仍可收星越障落地；最后 product 修改后 frontend typecheck／build 与 built-App clawd-app-ui 通过，包含实际 tap／hold 弹道、键盘／触摸取消、首次挂载立即输入与蓄力途中语言更新。网站与 App 仍共用一套 runner、原成绩 keys；当前 `e1a2df4` native App 与生产 `47158cd` 网站未含此玩法，不能借此声称已安装／上线／native 验收 |
| 共享 Clawd 运动修复 | 身体保持水平，四条连续短直腿分两组交替抬起与前后移动，支撑腿贴地，无横向小脚；每 100px 行进一周期。起跳弹道与 140ms 落地收势保留，静态 BODY／电影轮廓不变。15 项游戏测试覆盖 30／60／144fps 与 210–350px/s；此前本地网站 canvas 连续 75 帧胸口高度一致、支撑脚底一致，61 种腿部像素状态。独立 `site-ui`（含真实影片）、built `clawd-app-ui`、desktop build 通过；此步态修正先进入 clean `afa98f66` App 与静态网站 payload，现已进入正式官网；原生游戏启动／计分／跳跃／暂停／继续／Escape 关闭通过，四张实际连续帧显示身体高度稳定、短直腿变化；正式官网更新证据见托管行，视觉接受独立于检查结果 |
| Preview 视觉源码 | 首屏增加留白；完整微圆角橙脚与独立慢闪下划线；独立扩散／渐隐／2.6 秒清理的水纹与三幅 Clawd 的短暂反应，纸条指导／发现计数已退休。正文夜景以柔和暖灰衬托原色角色，保留深色双眼；共享游戏恢复连续短直腿与整体动作，星星真实高低变化。最后修改后 App frontend typecheck／build、canonical site-game-test（11 tests／240 个速度、障碍、星高组合）、site-ui、clawd-app-ui 均通过；1440／390 日夜、320 布局、900 contain／手机夜色合成像素、触摸／键盘、水纹到期清空、离屏停止、无 JavaScript 正文与无外部请求已核对，最终图景已目视。后台分支为合成 visibility，App UI 为 Chromium／合成 transport；网站已在正式域名上线，证据见官网托管行；历史 `afa98f66` Native 候选已重建并完成上述游戏交互检查，视觉接受独立于检查结果 |
| 概念 intro | [独立制作入口](../scripts/preview-film/README.md)已实跑：20 秒、1920×1080、60fps／1200 帧的连续字符空间镜头，生成带英文配音与纯音乐／SFX 两份 MP4，共用编码视频。Clawd 使用共享 connected Sprite 的清晰矢量，脚沿桥拱走、前栏按深度遮挡下部、包裹随身体整体轻起伏。原创 Tone.js 音乐／SFX 与本地 Kokoro speech 保留分轨；最终 AAC 配音版 −16.97 LUFS／−1.66 dBTP，纯音乐版 −14.55 LUFS／−1.54 dBTP，为此次测量而非通用响度承诺。两份媒体完整解码通过，13 个实际编码抽帧已检查；本地播放器无 autoplay／外部请求，暂停与播放中切换音轨均保留进度。导出 manifest 如实声明 `0172ccdc`／dirty，并保留与当前 film 源码一致的快照；原版电影留在 ignored candidate 输出；网页带配音版约 16.4 MiB，完整解码通过并在正式官网实际播放，原版单独保留 |
| 官网导航验收修正 | `8cbc335` 的 [full run 38261753431](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38261753431) 为 failure：只有 macOS site-ui 在第二次原生锚点导航超时，其余两平台 frontend／native／runner／heavy、Ubuntu browser 与 Linux VM jobs 成功，aggregate 正确拒绝。受控 A/B 重现旧 fixture 在第一次 smooth scroll 尚未停止时进行 instant reset／下一次 Enter，形成 compositor 竞争；保持产品原生链接与 smooth 行为，fixture 改为位置在连续三帧稳定后才交接下一次输入，同一四秒落点标准保留。最终小图标版的本机 canonical site-ui 已通过；修正后的 source CI 待推送核对，不能把这次 full run 标绿 |
| 最新 CI 观察 | 语言入口 `e1a2df4` 的 [run 38251692348](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38251692348) 已核对 completed/success，包含 Ubuntu browser／UI 与 aggregate；这是分组检查，不表示 native／VM 重跑。双语实现 `65495b7` 的 [run 38205602479](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38205602479) 已通过两平台与 Linux VM／aggregate；后续 `47158cd`／`6a94d3b` 文档检查也通过。旧候选资源／CI 观察见候选历史 |
| 此前 CI 修复证据 | `0172ccdc974fbad2920da10dd55a06584ba22c68` 的 [run 37787248424](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37787248424) 已直接核对为 completed/success；macOS／Ubuntu defaults 与独立 browser／App pairing 均通过，Ubuntu static runner、OpenSSH、CLI 包装及 disposable Linux VM lifecycle 也通过。此前 `20e9872` 的 [run 37763131837](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37763131837) 在 Ubuntu `site-ui` 等待 reduced-motion 状态时超时，50ms polling 未单独消除问题。当前 `site-world.mjs` 在 mount／change 时缓存 media-query 状态，动画帧只读缓存，保留系统事件、暂停、静态反馈与有限生命周期；修复后本地 canonical `site-ui` 两次通过（41.287s／33.35s），未延长 timeout 或降低断言。Linux 依赖步骤保留 15 分钟总时限、30 秒连接／数据 timeout、两次文件重试与索引失败直接退出，原包名与后续 gates 保留；见[验证说明](verification.md#linux-ci-dependency-preparation)。此 run 的源码范围止于上述 SHA，独立电影工具的制作与媒体检查另行记录，不借该 CI 声称视频美学验收 |
| 官网托管 | [lintel.page](https://lintel.page/) 已更新；Direct Upload Pages project `lintel`，production deployment `f5525e87-87c7-47cb-9ed7-1cf77d992888`，preview `131dcc49-4656-4f0f-927a-a7766dc7f316`，均声明 clean source `47158cda045d03d2ffb3fb8210a2f2ac3ab503d4`。同一 21 文件有限 payload 先验 preview 再上传生产，影片／海报与原审阅字节相同，未加入下载或 feed。正式域名 HTTPS 页面正常加载，浏览器实际取得的 clawd-game.mjs 与当前源码逐字节一致：新 STRIDE_TRAVEL 短腿步态在场、旧 whole-body bounce 已退出；游戏启动／跳跃／暂停、390px Night 的继续／暂停与无横向溢出、无相关 console error 已核对；正式域名影片通过原生控件播放到 20.01s（1920×1080），关闭暂停并归零。旧实际线上脚本与 `6c4e5df` 完全相同，确认是部署落后。shell／web 直读受到 Cloudflare 1010／浏览器 manifest 直达受阻，本轮没有冒称重新读回公开 manifest、DNS／RUM 或完整首次上线矩阵。原先影片／headers／手机验收保留于候选历史；本轮实际媒体 site-ui 通过。更新程序见 [site-hosting](site-hosting.md) |
| 发行与账户 | 无正式签名／公证、Applications 安装、真实认证操作或生产 VPS 激活；官网上线和公开源码不等于正式 App 发行；未上传社交平台 |

[基线验收记录](specs/product-baseline-status.md) 保存 A01–A18 的逐项证据与未验证条件；[候选历史](candidate-history.md) 保存此前候选的资源／CI／runtime 观察。历史 Linux VM 与 browser runtime 证据仍限定于当时源码和一次性环境，不证明这轮新增路径已验收。

## App 简中 / English 源码更新

两套随包文案覆盖 App 页面、审批／结果、帮助、状态、可访问性、问候与游戏；CLI、扩展 popup、官网和外链文档保持独立。顶部与设置都可选择跟随系统／简体中文／English；macOS 只读 preferred languages，原生菜单同步。没有在线翻译或新增 runtime dependency。源码已接通，最新本地 `e1a2df4` App 候选已包含双语与 minimal 入口；旧 `47158cd`／`25a8649` 包保留，没有重发 Release 或替换安装。

本机 catalog／插值／原文投影／格式／问候检查、frontend build 与严格 CI controller self-test 已通过；新 `i18n-ui` 验首屏、持久化、迟到和乱序系统读取、手动覆盖、草稿、原审批 ID／hash及 900px Day／Night。Clawd 的切换保留同一 game／canvas、暂停状态、分数、星星、最高成绩与动画生命周期；规则错误随当前语言显示、草稿不变且不提交通道操作。相邻 App／网站／browser pairing 旅程使用独立 synthetic fixtures，详细命令见[验证说明](verification.md)。

standalone desktop Rust 48 项通过，2 项既有安装前置检查 ignored；不是通过项。新 native debug executable 在隔离 HTML／WebKit harness 实际调用 preferred languages、简中／English 菜单切换与非法语言拒绝，菜单栏文字已观察；生产 getter 还在进程内 volatile `AppleLanguages` 简中顺序下返回匹配结果，不写持久系统语言。

解锁后，源码 `65495b7` 的真实产品 WebKit／native core 在隔离 synthetic HOME／state 下完成首屏 System 模式、简中／English 手动选择、设置面板与原生菜单同步、工作保全路径草稿／分类保留，以及退出重启后保留简中／Night 的检查。三份 synthetic 原件逐字节未变，没有提交计划、任务或归档输出。后台 AX 打开下拉框未交接输入焦点时曾只改显示值；按当前截图定向聚焦后，实际页面／菜单更新与结构化表单状态已核对。手动模式回到 System 的 native 操作仍未验证：菜单首项的 AX 路径被拒投，其他后台输入路径也未产生可核对的选择。以上是选定语言旅程的 dev runtime 证据，不替代其余操作的完整 native 矩阵或已包装候选验收。测试不触碰个人 Claude、browser profile、services 或真实宿主网络。

源码 `65495b7` 的[完整 CI](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38205602479) 已通过两平台 frontend／native／browser UI／runner CLI、heavy work、Linux VM 与 aggregate；双语候选先构建为 `47158cd`，入口简化后另行构建为 clean `e1a2df4`；两者与正式发行保持区别。

## 新增 IPv4 / IPv6 Leaf

当前 main 已接通 [IPv4 / IPv6 Leaf](specs/ip-network.md)。四格实测、严格 IPv4 socket、共享 macOS IPv6 计划与完整恢复已通过本机合成／loopback 检查，以及 built App 的独立 `network-ui`；最新本地 `e1a2df4` App 已包含该 Leaf，独立 `bb9fa824` CLI 候选仍保留旧能力范围。Ubuntu hosted CI 已完成当时的默认编译／合成检查；本机现已使用既有 Zig／cargo-zigbuild 完成 x86_64／aarch64 musl runner 构建与 static 格式核对。真实 macOS 系统授权与配置往返、VPN/TUN 切换、公众回显互通与真实 Linux host IP runtime 仍未验收；本轮仅在合成 HOME／state 启动候选，不替换已有安装、不部署 VPS 或正式发行。

## 新增遥测控制与 App 更新准备

[遥测与更新 Leaf](specs/telemetry-updates.md) T01–T04／U01–U05 已接通当前源码。两个官方 Datadog exact-host 目标有 direct Anthropic API／服务端 rollout 条件；混合 API／认证／下载主机只作说明。App 合并下次启动草案，保留手写规则和其它通道字段；设置、当前配置、受控 CONNECT 与普通客户端事件分别呈现。静态目录同时支持 named CLI、raw request 与有限 SSH；当前 App 的受控测试必须匹配原 channel binding、真实 403 与完成事件，不解析或连接公网遥测目标。细粒度保护预设保留，非必要流量总开关关闭 Claude Code 自动更新的附带影响明确展示。

App 自身使用官方 Tauri updater：最新候选与源码均未配置发行公钥／feed，后台检查默认关闭；检查、下载验签、批准安装与显式重启分别执行。安装与重启排除当前 App 操作／活跃代理；原意图、中断和外部记录冲突不重放。本地 release record 联动构建 override、signed-version 校验、官网 DMG 资料和 preview／stable feed；官网投影先要求公开字节核对，二进制留在 GitHub。本轮未生成正式 key、签名／公证、Release、已激活 feed 或生产网站更新，也未安装替换已有 App。完整 operator 边界见 [App 更新](app-updates.md) 与 [遥测](telemetry.md)。

本机 workspace、standalone desktop Rust（45 通过，2 项既有安装前置检查 ignored）、frontend build、release preparation、真实合成 CLI 及有限 SSH 合同检查通过。native 官方 updater 在临时 App／loopback 上实跑 HTTP 解析、坏签名／签名版本拒绝、下载不安装和临时 bundle 替换；不是实际已分发 App 升级。独立 built-App Chromium 的更新、遥测、IP 与基线旅程已验证，invoke／feed 回复为合成；Day／Night UI 截图为本地 QA。真实公网 feed、native WebKit 更新旅程、Apple 信任与旧实际 App 升级仍未验收。[验证入口](verification.md#遥测与-app-updater) 保留检查名与证据边界；新增 selections 已由[候选历史中的 `4d3706f` 两平台 CI](candidate-history.md)核对通过。

## 日常 CI 分组

GitHub Actions 仍是主要 CI owner；`tests/ci_plan.py` 按改动选择 `docs`／`site`／`app-ui`／`browser-ext`／`core`／`full`，由 canonical `tests/verify.py --checks` 执行。纯 UI／官网／文档不等待工作包重检查和 VM；核心、未知路径、CI 控制器、缺失 diff 基线与每周／手动 full 保留跨平台完整验收。各组在 plan 后独立执行，aggregate 核对必需 job 与每个平台的原始报告，选中检查 skipped 不算通过。Cargo／官方基础 VM 镜像缓存不替代源码构建和签名核对；没有接入 Cloudflare Builds 或自动部署。

2026-10-10 核对：本机严格控制器 self-test、73 项 planner／生产 aggregate／workflow regression 与 actionlint 通过。`3d09911` 的首轮分组 [full run 37953451500](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37953451500) 中，两平台 frontend／native／工作包／CLI 与 Linux OpenSSH／VM 均通过；工作包 macOS 16m26s、Ubuntu 22m11s，VM 18m44s，三者独立并行。该 run 的两平台 `baseline-ui` 抓到新增颜色 transition 在暂停动画时保留旧主题文字的回归，aggregate 正确拒绝，所以该 full run 是 failure，不能标绿。

`fe7f136` 恢复已有即时主题配色合同，保留阴影、位移、弹窗／toast 与其余美化；本机同断言先失败、修正后通过，完整 13 项 App UI 通过。对应纯 [app-ui run 37956385539](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37956385539) completed/success：frontend 三项与 App UI 十三项全部通过，native／工作包／CLI／VM 按计划 skipped，aggregate 通过；从首个 job 开始到 aggregate 完成为 **3m33s**。这是一次日常 UI run 的观察，不是每次耗时保证或 native WebKit／正式发行验收；该 CSS 修正未重跑未改变的 core／VM。原先 `4d3706f` 全量 run 的 macOS／Ubuntu 耗时为 24m32s／54m25s，属于旧串行流程。操作和准确边界见[验证说明](verification.md#ci-workflow-github-actions)。

## 入口与权威

| 读者／任务 | 权威入口 |
| --- | --- |
| 首次合成试用与英文入口 | [中文／English 文档入口](index.md)、[English documentation](index.en.md)、[中文 quickstart](quickstart.md)、[English quickstart](quickstart.en.md)、[English README](../README.en.md)；公开源码构建，无公开 Release 下载 |
| 人类任务、保全／重建、继续工作、原任务核对 | [操作指南](operator-guide.md)、[English](operator-guide.en.md) |
| 独立 CLI、用户候选安装、stdin、schema 与 Agent | [Agent CLI](agents.md)、[English](agents.en.md) |
| 前端构图、组件入口与 Selen 后续工作 | [桌面](desktop.md)、[视觉身份](visual-language.md) |
| 关联组件、保全与完整收尾的后续实现 | [环境覆盖与使用收尾方案](specs/environment-completion-plan.md)；组件清单、容量预检、精确选择、流式包与有限启动来源已接通源码；更多资产、独立完整退场、IDE/Desktop 和发行保留依赖与验收 gates |
| App 自身更新／本地发行准备 | [App 更新](app-updates.md)；源码可用、发行配置未激活，实际首装／升级独立验收 |
| 当前 native／browser／Linux 验证边界 | [验证说明](verification.md)、[验收记录](specs/product-baseline-status.md) |
| 官网、示例与 opt-in 共享游戏 | [正式官网](https://lintel.page/)、[源码说明](../apps/site/README.md)、[托管程序](site-hosting.md)；正式域名 HTTPS 与播放已验收，无正式 App 下载 |
| core／SSH／systemd／browser／network | [core](core.md)、[remote](remote.md)、[services](services.md)、[browser](browser.md)、[network](network.md)、[telemetry](telemetry.md) |

统一验证 owner 是 `python3 tests/verify.py`；UI／browser startup／Linux runtime 为独立 selections。dirty 开发 App 的 docs revision 为 unknown；干净候选的 Lintel 文档资源指向自己的源码 revision，main 入口标为最新开发说明；独立 Infra Field Guide 使用自己的 main，标为独立指南最新版。不会开放任意 URL 或 shell。

仓库已公开；项目原创材料的公开复用许可证仍未选定，第三方权利独立。源码可见不构成通用复用许可。

## 完整目标仍缺少

1. **真实认证与写入者控制：** Keychain/共享 profile、Desktop/IDE/service 的完整定位与生命周期；有限 systemd 暂停已接通，其他 supervisor 暂停、重新登录及旧会话续用。官方注销已加入观察主体与状态绑定，并拒绝缺失主体的预览；同账号 Keychain 内部 token 更新如果不反映到 CLI 状态，仍无法识别。当前识别部分 Claude 进程及可访问 manager 的直接 root 绑定服务；明确要求目标写入者先停，不能归属的进程阻止清理。官方 auth 命令仅有合成 CLI 证据，不能宣称生产注销已验收。
2. **完整外发策略与强约束：** 七项自定义已接入；更多取舍、入口实效矩阵、macOS Network Extension 签名与权限、Linux namespace；直接 socket、UDP、DNS、NO_PROXY、子进程不能由代理覆盖证明。
3. **远程运行：** 用户级安装／版本绑定已接入候选；Ubuntu 隔离 OpenSSH 已验证安装／丢 ACK 核对／原任务查询／交互 PTY，真实 VM 已完成有限 system-manager 暂停恢复、符合既有条件的 system/user manager 严格 logout 后续跑、原任务完成与 reboot 后核对。setsid 不保证 logout 存活，托管路径也不保证 reboot 续跑；生产 VPS 安装与任务实跑、aarch64 runtime、其他 supervisor 和完整 G03 仍未验收。
4. **浏览器旅程：** 独立 Chromium 临时测试 profile 已完成真实持久安装／onStartup 两阶段清理；仍需正式 Chrome、Edge、Firefox 与 AdsPower 的安装、配对与清理验收。Firefox 独立 CacheStorage、按站点 proxy、容器后台停写、克隆识别和专用 browser 启动仍有缺口。
5. **发行与维护：** App updater 与本地 release／feed／首装资料准备已接通源码，未激活实际发行；真实首装／升级、正式签名/公证仍未验收。菜单栏、定时漂移、签名规则更新、卸载与性能预算继续保留。
6. **并发与恢复：** 不合作的外部编辑器或已持有文件描述符的 writer 没有 OS 级 CAS；不确定副作用需核对。混合状态加密备份不支持自动恢复；会话/记忆迁入不证明可以续聊。

这些是原始完整目标的差距，不是缩小后的新 SPEC。下一关口是正式浏览器／AdsPower 与非开发者安装、真实认证／入口矩阵、生产 VPS 任务验收。隔离 runtime 与合成数据能证明机制，不能替代真实产品场景；现有个人登录和生产服务不作开发 mutation fixture。
