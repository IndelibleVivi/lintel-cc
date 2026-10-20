# 当前状态

2026-10-20 · **[Lintel 0.1.0 开发者 Preview 已发布](https://github.com/IndelibleVivi/lintel-cc/releases/tag/v0.1.0)**。公开 macOS arm64 App／DMG 沿用 clean source `fcae6cb1efb50c6384a31ddce161bee6a39646e0` 的已验收产品字节；随包文档编译 revision 也是该值，包含当前客户端控制、逐项／批量保护编辑、动态开关和新版侧栏图标。DMG 13,217,299 bytes，SHA-256 `7017b1d0a88068d54956c76b50511ff9503f132ad8d796079f43572e1038cb03`，完整性校验与只读挂载后 App 全文件读回一致；取出 executable 与构建输出相同。App 为 arm64／ad-hoc linker signature，无 Developer ID Team ID，原发行验收未安装到 Applications、无公证或 updater feed。10-20 后续从官网下载并安装的副本已暴露下述签名封装缺陷。Release 于 `2026-10-19T20:50:02Z` 公开，tag 固定 `2181e72db1ddb4e468226270e05546c62175d471`；8 个资产均经无需登录的公开下载逐字节核对，manual 记录为 `public_bytes_verified`／`publication_verified:true`。构建 source revision 仍是构建者声明，字节一致不构成来源签名。

独立三平台 CLI 候选为 `0.1.0-candidate-fcae6cb1efb5`，macOS arm64、Linux x86_64／aarch64 三包及两份 static musl runner 已随同一 prerelease 公开；包名保留原候选身份，Linux 为 experimental。10-20 发行整理版在不改变任何 executable 字节的前提下，为各 CLI 包加入 18 份与已核验 App 相同的第三方声明及校验覆盖；此前未含声明的本地包保留作历史测试候选。两份 Linux runner 在 10-19 用 X9 上的隔离 cargo-zigbuild 0.23.4／Zig 0.16.0 重建；App 内 runner、browser host、16 个扩展文件均与 manifest 的大小／摘要一致。实际 macOS CLI 交付包已解出，在新 HOME／state 逐字节读取三份原文且没有来源 job；这不证明其他架构 runtime。

[官网](https://lintel.page/) 的五类路由、15 篇 paired 指南与真实手动下载入口现已上线，共 40 路语言页面。当前 payload 是 clean `815f24be349752b0242cc1fc9a35232d766e4cf3`；[production 8442c39c](https://8442c39c.lintel-aue.pages.dev/) 与 [preview d9718bd4](https://d9718bd4.lintel-aue.pages.dev/) 使用同一有限 ZIP，由既有 Pages project 的 Dashboard Direct Upload 发布。正式域名 40 路 HTTP 200；10 页原始 HTML 字节一致，30 篇指南仅由 Cloudflare 消费自有 `email_off` 标记，其余 HTML／命令字节一致，技术标识未改写且没有邮箱解码脚本。下载记录、分享图、robots／sitemap 与真正 404 已读回；两条 updater feed 均为 404。手机中英日夜无溢出；正式域名的 1920×1080／20.01 秒影片已完整播放，Range 206、seek、字幕资源和关闭复位通过。站点未发出第三方 HTTP 请求或注入 analytics；App 二进制与发行 tag 未改变。

新 `fcae6cb` 已完成真实 native 首用、归档、退出重开、独立阅读与 CLI 交接；配置恢复和残留清理的完整 native 操作先在 `e822d74` 完成，下面保留各自的 source 范围。`878482f`、`bb9fa824` 均为较早候选，见[候选历史](candidate-history.md)。

## 当前下载版的启动故障（10-20）

已确认公开 `Lintel_0.1.0_aarch64.dmg` 的 App bundle 缺少完整资源封装签名。用户经 Chrome 下载后安装，macOS 报“已损坏”；DMG 自校验通过、SHA-256 与公开资产一致，安装副本的 46 个文件与只读 DMG 完全一致，排除本次下载／拷贝损坏。`codesign --verify --deep --strict` 与 `spctl --assess` 均报告 `code has no resources but signature indicates they must be present`；原 App 只有 `adhoc,linker-signed`，Info.plist 未绑定且 Sealed Resources 为 none。

独立修复副本完成本地 ad-hoc bundle sealing 后，严格 codesign 校验通过，Info.plist 已绑定且资源封装为 v2／44 文件；Gatekeeper 仍拒绝未识别身份的候选，**不等于 Developer ID 或 Apple 公证**。经用户明确批准，原安装副本已备份，随后仅修复该 App 的本地封装签名并移除其下载隔离标记；实际运行路径已核对为 `/Applications/Lintel.app/Contents/MacOS/lintel-desktop`，真实 native 首页可见，全局 Gatekeeper 仍开启。这是本机单 App 例外，不是新下载首装的 Apple 信任证据。公开 DMG／tag／官网 payload 还未替换；canonical 构建修复与更高版本 Preview 准备中，保留旧包及历史记录。

0.1.1 的源码修复已接入 `desktop:build`、本地 Preview 与正式 release build 的同一入口；本地未配置身份才补 ad-hoc，正式入口保留 Developer ID 要求。完整签名／篡改回归、参数优先权／准确产物路径、手动发行与站点打包检查通过；真实新 DMG、下载启动及公开读回尚待本次独立验收。独立 CLI 保持原候选版本，不因 App patch 重打包。

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
| 官网与阅读路径 | 五类路由、15 篇 repo 单源指南及精确 English mirrors，共 40 路静态页面。首页六任务标题进入站内指南；语言／历史保留原游戏、短片、details 与风景状态。真实 20 秒同源影片播放、全部 30 篇中英 guide 的 390px 无 JS 布局、首页 1440／390／320 日夜、无运行错误／第三方请求通过。manual／签名记录的 finite payload 与多通道／无 feed 边界通过合成检查；正式域名最新 40 路／手机日夜／完整影片与无 feed 独立验收通过 |
| CLI 与资源 | 当前 native release CLI 的 canonical `cli-candidate` 打包、解包、实际执行、保留 state 升级及实际三平台输入 smoke 通过。两份新 musl runner 为 static ELF／正确架构；其他架构实际执行仍是独立验收，不能从格式检查推断 |
| 发行与实机边界 | App／DMG／三平台 CLI 已公开并核对 8 资产；manual 记录已核对公开字节，未生成 updater feed。官网新 payload 已上线并完成正式域名读回；下载首装已确认有上方 App bundle 签名缺陷，修复候选与公开包分开记录；真实认证、正式浏览器／生产 SSH 与 Apple 信任仍未验收 |

## 能力与验证 owner

App／CLI 共用 core、有限 SSH 与原任务查询；v5 的 27 项保护字段、客户端配置／残留／身份控制、精确工作选择、流式加密包、独立阅读、有限启动来源、systemd、browser、IPv4／IPv6 与遥测已在源码接通。六任务合同与完整目标分别由 [product baseline](specs/product-baseline.md)、[SPEC](SPEC.md) 维护，不能把源码支持扩大为所有平台实效。

GitHub Actions 是主要 CI owner；`tests/ci_plan.py` 选择有限 changed-path profiles，`tests/verify.py --checks` 执行并输出 canonical evidence，aggregate 拒绝选中但 skipped 的检查。隐式默认组保持原样；真实 browser startup、native WebKit、Linux runtime 与大包容量为各自独立证据。发行 tag `2181e72` 的 [GitHub CI 39318119246](https://github.com/IndelibleVivi/lintel-cc/actions/runs/39318119246) 已完成，14 个 jobs（含 aggregate）全部 success。当前官网 `815f24b` 的 [CI 39334691082](https://github.com/IndelibleVivi/lintel-cc/actions/runs/39334691082) 也已成功，所选 frontend／control／browser-UI／aggregate 均通过；未选中的 native／heavy／runner／VM skips 不计通过。没有接入自动部署。

逐轮 source、旧候选资源、CI 和隔离 runtime 证据已移入 [候选历史](candidate-history.md#2026-10-19-归档的阶段证据)，[基线验收](specs/product-baseline-status.md) 保留 A01–A18；具体命令和证据边界见 [验证说明](verification.md)。历史通过不会把本轮未测层标成通过。

## 入口与权威

| 读者／任务 | 权威入口 |
| --- | --- |
| 首次合成试用与英文入口 | [中文／English 文档入口](index.md)、[English documentation](index.en.md)、[中文 quickstart](quickstart.md)、[English quickstart](quickstart.en.md)、[English README](../README.en.md)；公开源码构建与 v0.1.0 开发者 Preview 手动下载 |
| 人类任务、保全／重建、继续工作、原任务核对 | [操作指南](operator-guide.md)、[English](operator-guide.en.md) |
| 独立 CLI、用户候选安装、stdin、schema 与 Agent | [Agent CLI](agents.md)、[English](agents.en.md) |
| 前端构图、组件入口与 Selen 后续工作 | [桌面](desktop.md)、[视觉身份](visual-language.md) |
| 关联组件、保全与完整收尾的后续实现 | [环境覆盖与使用收尾方案](specs/environment-completion-plan.md)；组件清单、容量预检、精确选择、流式包与有限启动来源已接通源码；更多资产、独立完整退场、IDE/Desktop 和发行保留依赖与验收 gates |
| App 自身更新／本地发行准备 | [App 更新](app-updates.md)；源码可用、发行配置未激活，实际首装／升级独立验收 |
| 当前 native／browser／Linux 验证边界 | [验证说明](verification.md)、[验收记录](specs/product-baseline-status.md) |
| 官网、示例与 opt-in 共享游戏 | [正式官网](https://lintel.page/)、[源码说明](../apps/site/README.md)、[托管程序](site-hosting.md)；本轮 40 路、HTTPS／headers、手动下载、手机日夜与真实影片已验收 |
| core／SSH／systemd／browser／network | [core](core.md)、[remote](remote.md)、[services](services.md)、[browser](browser.md)、[network](network.md)、[telemetry](telemetry.md) |

统一验证 owner 是 `python3 tests/verify.py`；UI／browser startup／Linux runtime 为独立 selections。dirty 开发 App 的 docs revision 为 unknown；干净候选的 Lintel 文档资源指向自己的源码 revision，main 入口标为最新开发说明；独立 Infra Field Guide 使用自己的 main，标为独立指南最新版。不会开放任意 URL 或 shell。

仓库已公开；项目原创材料的公开复用许可证仍未选定，第三方权利独立。源码可见不构成通用复用许可。

## 完整目标仍缺少

1. **真实认证与写入者控制：** Keychain/共享 profile、Desktop/IDE/service 的完整定位与生命周期；有限 systemd 暂停已接通，其他 supervisor 暂停、重新登录及旧会话续用。官方注销已加入观察主体与状态绑定，并拒绝缺失主体的预览；同账号 Keychain 内部 token 更新如果不反映到 CLI 状态，仍无法识别。当前识别部分 Claude 进程及可访问 manager 的直接 root 绑定服务；明确要求目标写入者先停，不能归属的进程阻止清理。官方 auth 命令仅有合成 CLI 证据，不能宣称生产注销已验收。
2. **完整外发策略与强约束：** v5 的 27 个字段与逐项编辑已接入；真实客户端入口实效矩阵、macOS Network Extension 签名与权限、Linux namespace；直接 socket、UDP、DNS、NO_PROXY、子进程不能由代理覆盖证明。
3. **远程运行：** 用户级安装／版本绑定已接入候选；Ubuntu 隔离 OpenSSH 已验证安装／丢 ACK 核对／原任务查询／交互 PTY，真实 VM 已完成有限 system-manager 暂停恢复、符合既有条件的 system/user manager 严格 logout 后续跑、原任务完成与 reboot 后核对。setsid 不保证 logout 存活，托管路径也不保证 reboot 续跑；生产 VPS 安装与任务实跑、aarch64 runtime、其他 supervisor 和完整 G03 仍未验收。
4. **浏览器旅程：** 独立 Chromium 临时测试 profile 已完成真实持久安装／onStartup 两阶段清理；仍需正式 Chrome、Edge、Firefox 与 AdsPower 的安装、配对与清理验收。Firefox 独立 CacheStorage、按站点 proxy、容器后台停写、克隆识别和专用 browser 启动仍有缺口。
5. **发行与维护：** App updater 与本地 release／feed／首装资料准备已接通源码，手动开发者 Preview 已发布，签名 updater 未激活；当前公开 App 下载启动存在已确认的 bundle signing 缺陷；修复候选、真实首装／升级、正式签名/公证分层验收。菜单栏、定时漂移、签名规则更新、卸载与性能预算继续保留。
6. **并发与恢复：** 不合作的外部编辑器或已持有文件描述符的 writer 没有 OS 级 CAS；不确定副作用需核对。混合状态加密备份不支持自动恢复；会话/记忆迁入不证明可以续聊。

这些是原始完整目标的差距，不是缩小后的新 SPEC。下一关口是正式浏览器／AdsPower 与非开发者安装、真实认证／入口矩阵、生产 VPS 任务验收。隔离 runtime 与合成数据能证明机制，不能替代真实产品场景；现有个人登录和生产服务不作开发 mutation fixture。
