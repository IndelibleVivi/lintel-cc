# 当前状态

2026-10-05 · **0.1.0 开发候选**。完整 [SPEC](SPEC.md) 与四条完整旅程 G01–G04 尚未交付。本轮围绕原任务恢复、批准一致性、浏览器跨入口接续和审批可见性修复；不代表正式发行、真实认证或生产 VPS 激活。

## 这份候选包含什么

本轮候选来自同一工作树的阶段修复与官网／共享游戏源码。版本仍为 0.1.0，不能只凭版本号认定两个候选相同。阶段修复源码为 `e276f67`；独立 CLI 打包与该源码已经整合，整合候选的重建、安装和跨平台 CI 完成后在本节记录准确引用。下表已有构建事实属于此前阶段候选。

| 对象 | 当前证据与边界 |
| --- | --- |
| 源码 | `e276f67` 十项恢复／审批／浏览器修复；官网与共享游戏 `896d38a`；两者均已 push |
| CLI | 当前 debug runner 已重建，独立命令、portable work、原任务查询／恢复与七项策略旅程通过 |
| macOS App | arm64 `Lintel.app` 22.63 MiB，包含本轮修复与共享 Clawd；路径 `apps/desktop/src-tauri/target/release/bundle/macos/Lintel.app` |
| Linux runner | 新构建 x86_64／aarch64 musl；App `remote-runners` 的 bytes／SHA-256 与各自产物一致，aarch64 runtime 未验证 |
| 浏览器资源 | Chromium／Firefox 非 fixture 扩展与当前 arm64 native host 已入包；包内文件与 source／manifest 核对一致，直接使用包内资源的两个 synthetic 安装测试通过 |
| 安装与发行 | 仅 linker ad-hoc signature；无 Applications 安装、生产远端部署、Developer ID 签名、公证或正式发布 |

## 独立安装候选与双架构 runner

独立 CLI 打包入口为 `scripts/package-cli.mjs`：明确提供 macOS arm64 Mach-O、Linux x86_64／aarch64 static ELF 与 canonical runner-bundles，核对 Linux CLI 与对应 runner 字节相同，生成三份 archive、`SHA256SUMS` 与候选身份索引。checksum 包含 payload、`candidate.json` 和 `README.txt`，只排除自身；source_revision 是调用方声明，不是签名或认证 provenance。每个 runner 遵守 SSH installer 的 32 MiB 上限。

[安装与升级指南](agents.md) 使用系统 checksum 工具，目标端不需要 Rust、GUI 或 Node runtime。安装选择尚不存在的新版本目录，保留旧 executable、state 和原 job ID；archive／索引／版本目录不覆盖。打包或索引发布报错时只回收本次未被外部替换或编辑的 owned 文件，不承诺骤然进程死亡后的自动恢复。其它架构的格式 fixtures 不构成实际 runtime 验收。

`cli-candidate` 在 macOS／Ubuntu CI 通过同一 canonical entrypoint 执行 native 解包、version／capabilities／schema 和原 state/job 保留旅程。当前整合源码的 CI 与重新构建资源尚待读回；旧候选的本机 Mac 安装旅程通过，不能替代这次 runtime 的新验证。正式发布、Applications 激活和真实 VPS 安装仍是独立步骤。

此前 `e276f67` 阶段 App 的包内资源身份可按下表核对；SHA-256 为完整摘要，来自本地构建和 source/resource 比对，不表示这些二进制已经发行。路径相对于 `Lintel.app/Contents`。

| 对象 | 路径 | SHA-256 |
| --- | --- | --- |
| macOS arm64 App executable | `MacOS/lintel-desktop` | `f3f867248a200956f5a6f3a0bdf09cd8c1bf26437421768f65f1bfe89c0d36c5` |
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

此前阶段 clean `e276f67` 的 [CI 37263003720](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37263003720) 全部通过；下载核对的六份入口 JSON 均记录同一 HEAD、`dirty=false`。

| 检查 | macOS arm64 | Ubuntu x86_64 |
| --- | --- | --- |
| 默认合成验证 | 18/18 | 18/18 |
| 独立 browser／App／官网 UI | 7/7 | 7/7 |
| 静态 musl runner 的真实 OpenSSH | 不适用 | 1/1 |
| 一次性 Linux VM 生命周期 | 不适用 | 1/1 |

两平台 Chromium 报告均有完整 12 项 smoke：保留原存储／写入者／邻域／启动检查，新增 App clear → 实际进程退出／新 onStartup → popup 继续 → 原 App 最终回执；提交一次，取消预览回传也通过。VM 报告为 `evidence_complete`，覆盖真实 systemd/PAM/logout/cgroup/reboot 与服务恢复，证据限于一次性 guest。

本地默认 18/18、29 项 Engine 单测及最新工作 UI 也通过；实际使用本节 App 包内 host／extension 资源的独立安装检查为 2/2。配置 crash recovery 用真实 child 进程在发布前／后退出，重开查询、准确归属、独立恢复与后续编辑拒绝有回归。旧版本报告与旧 App 资源保留在 [候选历史](candidate-history.md)，不替代本次源码验证。

仓库已公开；项目原创材料的公开复用许可证仍未选定，源码可见不构成通用复用许可，第三方权利保持独立。

## 完整目标仍缺少

1. **真实认证与写入者控制：** Keychain/共享 profile、Desktop/IDE/service 的完整定位与生命周期；有限 systemd 暂停已接通，其他 supervisor 暂停、重新登录及旧会话续用。当前识别部分 Claude 进程及可访问 manager 的直接 root 绑定服务；明确要求目标写入者先停，不能归属的进程阻止清理。官方 auth 命令仅有合成 CLI 证据，不能宣称生产注销已验收。
2. **完整外发策略与强约束：** 七项自定义已接入；更多取舍、入口实效矩阵、macOS Network Extension 签名与权限、Linux namespace；直接 socket、UDP、DNS、NO_PROXY、子进程不能由代理覆盖证明。
3. **远程运行：** 用户级安装／版本绑定已接入候选；Ubuntu 隔离 OpenSSH 已验证安装／丢 ACK 核对／原任务查询／交互 PTY，真实 VM 已完成有限 system-manager 暂停恢复、符合既有条件的 system/user manager 严格 logout 后续跑、原任务完成与 reboot 后核对。setsid 不保证 logout 存活，托管路径也不保证 reboot 续跑；生产 VPS 安装与任务实跑、aarch64 runtime、其他 supervisor 和完整 G03 仍未验收。
4. **浏览器旅程：** 独立 Chromium 临时测试 profile 已完成真实持久安装／onStartup 两阶段清理；仍需正式 Chrome、Edge、Firefox 与 AdsPower 的安装、配对与清理验收。Firefox 独立 CacheStorage、按站点 proxy、容器后台停写、克隆识别和专用 browser 启动仍有缺口。
5. **发行与维护：** 菜单栏、定时漂移、签名规则更新、卸载、升级、正式签名/公证、性能预算。
6. **并发与恢复：** 不合作的外部编辑器或已持有文件描述符的 writer 没有 OS 级 CAS；不确定副作用需核对。混合状态加密备份不支持自动恢复；会话/记忆迁入不证明可以续聊。

这些是原始完整目标的差距，不是缩小后的新 SPEC。下一关口是正式浏览器／AdsPower 与非开发者安装、真实认证／入口矩阵、生产 VPS 任务验收。隔离 runtime 与合成数据能证明机制，不能替代真实产品场景；现有个人登录和生产服务不作开发 mutation fixture。
