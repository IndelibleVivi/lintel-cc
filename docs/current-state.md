# 当前状态

2026-10-15 · **Lintel 0.1.0 Preview / 开发预览，尚无正式 Release**。最新本地 macOS arm64 App／未签名 DMG 仍为 2026-10-13 构建的 clean `878482fa14f656776ef99b995c6f177ac52921d3`，包含最新共享跑道、双语入口、cleanup metadata-only 修复、完整第三方许可与 clean-checkout notices 接线修复。两个 Linux musl runner 和 native browser host 已在该轮重新构建；DMG 只读挂载与资源字节核对通过。新包隔离 HOME／state 的实际 release WebKit 首屏和原生设置已观察；完整语言／游戏操作对应 10-13 同日产品代码未变的 `cdb0ba8` 候选，分别留存。未安装到 Applications、无 Developer ID／公证、无公开二进制或 feed 激活。

官网生产仍是 clean `c80a783` 的独立静态 payload，简中／English、六项能力场景、共享跑道与概念短片保持已验证版本，本轮未重新部署。独立三平台 CLI 发行候选仍为 `bb9fa824`，10-13 构建的 macOS CLI 仅用于当轮 release 大工作包复验。10-15 接续合成回归与现成候选的原生首用验收，没有产品行为／二进制变化。A01–A18 与完整 SPEC／G01–G04 的未验收范围继续保留。

## 当前交付与证据

| 对象 | 当前事实与验收边界 |
| --- | --- |
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

## App 简中 / English 源码更新

两套随包文案覆盖 App 页面、审批／结果、帮助、状态、可访问性、问候与游戏；CLI、扩展 popup、官网和外链文档保持独立。顶部与设置都可选择跟随系统／简体中文／English；macOS 只读 preferred languages，原生菜单同步。没有在线翻译或新增 runtime dependency。源码已接通，最新本地 `878482f` App 候选已包含双语与 minimal 入口；旧 `47158cd`／`25a8649` 包保留，没有重发 Release 或替换安装。

本机 catalog／插值／原文投影／格式／问候检查、frontend build 与严格 CI controller self-test 已通过；新 `i18n-ui` 验首屏、持久化、迟到和乱序系统读取、手动覆盖、草稿、原审批 ID／hash及 900px Day／Night。Clawd 的切换保留同一 game／canvas、暂停状态、分数、星星、最高成绩与动画生命周期；规则错误随当前语言显示、草稿不变且不提交通道操作。相邻 App／网站／browser pairing 旅程使用独立 synthetic fixtures，详细命令见[验证说明](verification.md)。

standalone desktop Rust 48 项通过，2 项既有安装前置检查 ignored；不是通过项。新 native debug executable 在隔离 HTML／WebKit harness 实际调用 preferred languages、简中／English 菜单切换与非法语言拒绝，菜单栏文字已观察；生产 getter 还在进程内 volatile `AppleLanguages` 简中顺序下返回匹配结果，不写持久系统语言。

解锁后，源码 `65495b7` 的真实产品 WebKit／native core 在隔离 synthetic HOME／state 下完成首屏 System 模式、简中／English 手动选择、设置面板与原生菜单同步、工作保全路径草稿／分类保留，以及退出重启后保留简中／Night 的检查。三份 synthetic 原件逐字节未变，没有提交计划、任务或归档输出。后台 AX 打开下拉框未交接输入焦点时曾只改显示值；按当前截图定向聚焦后，实际页面／菜单更新与结构化表单状态已核对。当时手动模式回到 System 的 native 操作未验证；本轮 `cdb0ba8` release App 已实际选择并观察 System English 与原生菜单恢复，补齐这一项，未扩展为完整 native 矩阵。以上是选定语言旅程的 dev runtime 证据，不替代其余操作的完整 native 矩阵或已包装候选验收。测试不触碰个人 Claude、browser profile、services 或真实宿主网络。

源码 `65495b7` 的[完整 CI](https://github.com/IndelibleVivi/lintel-cc/actions/runs/38205602479) 已通过两平台 frontend／native／browser UI／runner CLI、heavy work、Linux VM 与 aggregate；双语候选先构建为 `47158cd`，入口简化后另行构建为 clean `e1a2df4`；两者与正式发行保持区别。

## 新增 IPv4 / IPv6 Leaf

当前 main 已接通 [IPv4 / IPv6 Leaf](specs/ip-network.md)。四格实测、严格 IPv4 socket、共享 macOS IPv6 计划与完整恢复已通过本机合成／loopback 检查，以及 built App 的独立 `network-ui`；最新本地 `878482f` App 已包含该 Leaf，独立 `bb9fa824` CLI 候选仍保留旧能力范围。Ubuntu hosted CI 已完成当时的默认编译／合成检查；本机现已使用既有 Zig／cargo-zigbuild 完成 x86_64／aarch64 musl runner 构建与 static 格式核对。真实 macOS 系统授权与配置往返、VPN/TUN 切换、公众回显互通与真实 Linux host IP runtime 仍未验收；本轮仅在合成 HOME／state 启动候选，不替换已有安装、不部署 VPS 或正式发行。

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
