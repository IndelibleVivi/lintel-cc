# 开发候选历史证据

本页保留历次阶段与候选（包括 `a239123` 与独立 CLI 候选）的验收记录，是历史快照。下文的“当前”“本轮”均指对应旧候选；不能据此推断后来 checkout 或 App 包所含资源。正在使用的候选、资源对应关系与未完成目标以 [current-state.md](current-state.md) 为准。

2026-10-05 · 0.1.0 开发候选。完整 [SPEC](SPEC.md) 仍未交付，四条完整旅程 G01–G04 尚未通过。源码与本地 macOS App 已构建；未安装到 Applications、未正式发布、未部署真实远端。

项目仓库已于 2026-10-04 公开，保留原 main 历史；GitHub PUBLIC 与匿名 Git／README 读回已核对。当前仍未选定项目原创材料的公开复用许可证，第三方权利不由 Lintel 重新授权。源码公开与正式发行、完整验收是不同状态。

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
