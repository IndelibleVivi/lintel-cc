# Lintel 0.2.0 Preview — 工程审查与发行交接

**最终产品 source：`353536cc7dc3c8c63baa628955ee66332d95bcbb`，tree `7c12bee277e84c85052d146738bbb395bd9b25b7`。** 本轮基于 PR #7 后 main `73f34ed06b5d630d2a36910baf88ff5713aede66`；修复进入 [PR #8](https://github.com/IndelibleVivi/lintel-cc/pull/8)。公开下载仍为 App 0.1.1 Preview／CLI 0.1.0，未创建 Release、激活 production updater 或部署官网。最终 canonical 与 candidate gate：已通过（完整 push CI 43526964668；candidate 43526964671 的强制 macOS gate）。

**发行建议：0.2.0 Preview，按平台分别决定。** Linux x86_64/aarch64 CLI：GO；macOS arm64 CLI：CONDITIONAL GO，补首下载隔离／信任路径；面向普通用户的 macOS App：NO-GO；有限技术 App Preview：在明确手动信任、完成所选包的 native 完整任务／升级及图形权利条件后 CONDITIONAL GO。这里的 GO 是工程建议，不代替所有者公开发布授权。

源码已通过 [PR #8](https://github.com/IndelibleVivi/lintel-cc/pull/8) 合并为 **`448d92bced6e87d34842bb5c54407e675e741653`**；合并 tree 与固定产品 source 相同。除指定的 push CI 外，[PR CI 43526971709](https://github.com/IndelibleVivi/lintel-cc/actions/runs/43526971709) 也已 14/14 jobs success。本文和相关最终说明在其后以 **docs-only** 提交记录结果，不能改写候选的实际构建 source 或代替新代码验收。

## 阅读顺序

Faye 先读最终 Release Decision Brief。维护者从本文的证据矩阵和剩余任务进入；需要复现时再读对应专项报告。

| 材料 | 负责的问题 |
| --- | --- |
| [基线与证据](baseline-and-evidence.md) | PR #4–#7、公开包、线上网站、既有 CI 与外部机制的真实状态 |
| [依赖安全](dependency-security.md) | 当前锁定图、实际 shipped 路径、TLS 修补及不适用告警的依据 |
| [认证安全](auth-security.md) | 认证 namespace、Keychain 精确字段、Linux writers、实际 Claude 分发、旧批准兼容 |
| [执行与恢复](execution-recovery.md) | 原 ID、损坏记录、路径／管道替换、发布不确定、远端安装去重 |
| [产品验收](product-acceptance.md) | 六类任务、新用户、多环境、返回用户、界面与 native 证据边界 |
| [发行工程](release-engineering.md) | 实际包、权限／内部校验和、版本投影、原生升级、签名、最终 gate |
| [中文发行说明](release-notes.zh-CN.md) / [English](release-notes.en.md) | 可审阅、尚未公开的 0.2.0 Preview 说明 |

所有测试修改均限定合成 HOME/state、临时 Keychain、惰性客户端或 disposable SSH/VM。没有操作个人认证、生产 VPS、真实浏览器 profile、生产 updater、正式 GitHub Release 或官网部署。

## 1. 产品质量判断

Lintel 已有可保留的产品基础：六类工作有入口，GUI/CLI 共用 Core，实际动作有计划、准确批准、持久任务与回执；工作资料可以独立离线读取；发生不确定结果后继续沿原 ID 核对。它没有通过删全目录、重提任务或把所有结果标成绿色来简化流程。

本轮最严重的已确认问题集中于**选择对象与恢复信息**：默认认证目录被显式变量改成另一凭据命名空间；Linux writer 的字符串比较漏掉同一物理目标；远端损坏记录被当作不存在；设置已经改写后的不确定性被低估。发行检查也存在自洽但不可用的包能通过的问题。后续依赖审查还确认了 HTTPS 上游代理和探测使用的信任根包含已被撤销信任的证书链，需要限定范围的安全补丁更新。详细触发条件、修复和可区分的证据见下表及专项报告。

当前适合推进 0.2.0 Preview。普通用户的一般 macOS App 分发还要跨过真实首装信任、最后 native 交互及资产授权三道明确边界。未来网络约束、浏览器商店分发、更多系统支持和容量退休策略是后续演进，不用来无限延期一个范围明确的 CLI Preview。

## 2. 可追溯发现与修复

| 发现 | 触发与用户后果 | 修复位置与可区分的证据 |
| --- | --- | --- |
| AUTH-01 默认/显式目录混淆 | 未设置目录变量的默认环境被强制设置，当前 Claude 会访问另一 Keychain service；表现为意外退出或不同登录 | `keychain.rs::execution_context/configure_execution`、`launch.rs`、Core/runner 启动入口共用描述；原生分发静态规则与 PTY default/explicit 回归 |
| AUTH-02 独立安全存储目录 | `CLAUDE_SECURESTORAGE_CONFIG_DIR` 跨 root，或批准前后目录选择改变；可能核对另一个环境 | 观察选择、拒绝无法对应的 root、冻结并在接收/启动前复查；`auth_cleanup.rs`、launch/resume；缺旧绑定先新预览，原已接收任务不重放 |
| AUTH-03 活跃 writer 漏判 | 两个绝对路径字符串不同却指同一物理目录，或普通 config 在邻居而 secure storage 指向目标 | `cleanup.rs` 只在两个目标都明确无关时排除；fresh process + 实际 Linux ps/proc 实验 |
| AUTH-04 account 规则漂移 | 异常 USER 与新客户端 account 规则不一致，可能误报不存在 | 原生 account 派生对未知选择返回不可核对；不扫所有 account、不改原 frozen identity |
| ER-01/02 损坏原记录静默消失 | 安装/任务/launch JSON 不可读，历史看似完整，新的安装可能跨过原不确定状态 | shared remote `records_complete/record_errors`；健康原记录保留，选定 alias 阻止新 intent/SSH；App 双语 warning、精确 ID、仍可查询 |
| ER-03 FIFO 阻塞 | 常规文件被替换成管道，阻塞 open，字节限制无效 | Core/remote descriptor-first `O_NONBLOCK` + 类型检查；两秒有界 baseline 反例；保留 symlink 与大小规则 |
| ER-04 清理说明落后 | TUI 把现已支持的 Keychain 一概归为外部 | 同步共享能力与覆盖语义，无扩大清理范围 |
| ER-05 已落盘但低估不确定 | rename 成功、父目录 fsync 失败，原记录虽需 reconciliation，辅助标志却 false | Core 执行错误分支保留 prepared/written publication 不确定；真实 rename 后 EIO 注入，原 ID/bytes/重复批准无重写 |
| ER-06/07 安装目录或记录结构异常 | 父目录 symlink 绕过 leaf 检查；合法 JSON 缺 bundle 字段可弄崩 UI | parent/leaf 检查及 shared `valid_install_record`；五种目录替换、九种部分 JSON，不触发 SSH，健康邻居与原 bytes 保留 |
| RE-01/02 包自洽却不可用 | 二进制 mode 0644、unsafe modes 或坏 SHA256SUMS，旧 gate 仍 true | `prepare-candidate.py::read_tar_members` 检查执行权限、安全模式与完整内部映射；其他 receipt 保持一致的 before/after 反例 |
| RE-03/04 App 版本投影 | release override 未传 Vite；预发行字符串直接落入数字 plist | formal build 显式 `LINTEL_APP_VERSION`；数字 0.2.0 + Preview channel；实际 DMG 两字段一致；不破坏旧 SemVer 比较 |
| CI 系统包前提 | 固定 Azure 镜像重写遇到 hosted image 变化，原完整 CI 在产品测试前失败 | 保留原 image 官方仓库，有限重试/超时、update 失败不 install；实际后续 Ubuntu 原生／VM 路径已运行 |
| DEP-01 失信 TLS 根与已公告依赖 | HTTPS upstream proxy 与主动 TLS probe 使用受影响的根及验证器；简单单包更新另会破坏旧 xattr size query | 精确锁定 TLS、Tokio、rustix 与配套 xattr；真实 TLS 回归、8192 字节扩展属性及大 name-list 红／绿对照；见依赖安全报告 |
| ER-08 停止检查误认端口身份 | 已关闭监听后，同端口可被其他监听器或 outgoing self-connect 使用；数字端口连接不能识别原代理 | 生产 shutdown 路径无需修改；直接断言原 owner 释放、真实双向 tunnel 在 stop 返回前结束，取消后原 channel 保留；旧 macOS 具体原因不猜测 |
| QA 采集前提 | macOS 最后 PTY slave 关闭丢失输出；argv-only child 早退遇空 stdin 写入 EPIPE | 保留 slave 到 nonblocking drain；无 stdin 合同不建 pipe，JSON transport 验退出码/信号/信封；不放宽产品断言，须新 native CI 通过 |

### 没有被“修掉”的可靠设计

- Keychain 针对原 persistent reference 做 data-only 更新，保留 service/account、MCP/plugin/未知字段；没有凭据恢复副本。官方 logout 的实际范围过宽，继续拒绝。
- durable acceptance 先于 ACK；已有 original job ID 走查询路径；别名移除不删除去重状态；迟到响应不能更换当前主机/环境的结果。
- settings 恢复核对原 staged object 的 identity/bytes，保留之后的无关字段；同内容不等于所有权。
- 加密归档和迁入采用 no-replace publication，独立 state 能检验密文原文；归档不默认激活会话或修改凭据。
- browser 通过原生 API 和真实 startup generation 完成两阶段清理；不把 worker 重启当浏览器重启，不重放 uncertain ID。
- SSH 使用有限 transport、严格主机配置、原 runner 绑定；systemd 只使用已有资格，不 sudo、不启用 linger、不改生产登录策略。
- 来源 inventory 不运行 helper；另行选择的官方 status 可能调用客户端配置的 helper 或重新生成凭据，随后再做最终本地读回。原始 status 只在内存中解析，输出保留有限来源/范围；状态 schema 不认识时说明不可核对。

这些保护有明确边界：停止其他 writer 仍是运行条件，Keychain API 不提供跨任意写入者的 payload CAS；路径选择冻结不是整个后续进程的 OS sandbox；Terminal 异步交接只报告 `launch_requested`；systemd/setsid/reboot 能力分开。报告保留这些实际条件，不扩大“完成”的含义。

## 3. 证据层与版本身份

| 层 | 最终观察与实际范围 | 不能由这一层推导的结论 |
| --- | --- | --- |
| Source | 最新 main 全路径审查；三个工程提交 `e573ce2` → `da7ebd2` → `353536cc`；最后一个为固定产品 source | 用户已安装新版、线上已经更新 |
| Local synthetic/native Linux | 实际文件、PTY、Linux ps/proc、FIFO／rename-fsync 故障、路径替换、独立 TLS／xattr／TCP 反例；记录红／绿结果 | 其他 OS／CPU 或个人账户已验收 |
| 历史完整 CI | PR #7 14 jobs、120 次按平台计的选中 checks；12 份报告重新聚合通过 | 本轮新改动或最终 DMG 已通过 |
| Final canonical CI | [push 43526964668](https://github.com/IndelibleVivi/lintel-cc/actions/runs/43526964668)，exact clean `353536cc`；14/14 jobs success；12 份 canonical reports，120 次按平台计的选中 check 执行全部 passed；root 下载读回并独立复跑 production aggregate，problems=[] | 一份 aggregate 自动取消内部 ignored 项或模型边界 |
| Final native auth | macOS 19/19、Linux 18/18 checks；auth 各 49 cases；临时 Security.framework Keychain、Linux 8 个活 writer 场景实际运行 | 个人账号 ACL、未来客户端版本、真实账户登录 |
| Final UI/browser | 两平台各 19/19；frontend Mac 15/15、Linux 14/14；真实 Core 与 browser，invoke／SSH 边界明确建模 | Chromium 等同于 native WebKit、生产远端已操作 |
| Final CLI packages | 最终三份 tar.gz 在 native macOS arm64、Linux x86_64、Linux aarch64 各自通过，`upgrade_tested:true`；输入为真实公开 0.1.0 | 浏览器下载 quarantine、旧程序写未来 state、生产 SSH |
| Final DMG/package | 完整 64 文件 App／39,934,188 bytes；strict ad-hoc 签名、两 plist 版本 0.2.0；DMG mount/copy inventory 一致 | Developer ID 身份、公证、普通用户首开 |
| Installed copy | macOS Darwin 24.6.0 arm64 临时复制 App 原生存活；Gatekeeper exit 3 rejected、stapler exit 65 无票据、无 quarantine | Finder/Applications 升级或真实下载信任 |
| Native WebKit/AX | 同一最终 DMG：11 次有限 AX 控件操作，9 个结果状态检查通过，10 张目标窗口图；Night／系统、中文／系统、历史、需环境提示、任务框开关 | 完整归档／清理／SSH 执行、native 升级、键盘导航或 VoiceOver 验收 |
| Deployed/public | 实际浏览器观察旧 App 0.1.1 官网；旧三平台 CLI 0.1.0 真实资产用于升级；旧 DMG 读回并重新验证发布记录 | 新 0.2.0 已公开或 updater 已可用 |

macOS **源码测试**使用 macOS 26.7.2 arm64；最终 **包／安装副本／窗口**使用 macOS 15 系列、Darwin 24.6.0 arm64。两者分列；声明的最低 macOS 12 没有因此获得原生覆盖。两个 desktop bundle resource fixtures 在普通组中仍 ignored：Mac 是 **56 passed + 2 ignored**，Linux 是 **52 passed + 2 ignored**。当前 source/browser host 单测和包资源 inventory 不能冒充这两个最终资源消费者的执行，下面给出直接补跑命令。

最终 VM 的实际结果也保留了失败语义：普通 setsid 在四个 logout 场景均观察到 worker 结束，没有将其说成跨 logout 保证；具备既有 system／user manager 资格的两组观察到存活，fallback setsid 仍结束。管理认证实验先返回 `submission_uncertain`，随后 **一次 submit、24 次原 ID query**，中间真实观察到 `job_not_found`，最终拿到同 ID 的 `partially_completed`：本地清理完成，超范围官方 logout 拒绝、凭据与 caller 环境保留。真实 reboot 前后原 ID 可查询，`submitted_once:true`、`reexecuted:false`。这是 canonical 源码构建的 disposable Ubuntu/PAM/systemd 证据，不冒充最终 tar.gz 在生产 VPS 的执行。

### 固定构建与完整候选

[Candidate run 43526964671](https://github.com/IndelibleVivi/lintel-cc/actions/runs/43526964671) 从 clean exact source fresh clone／fresh target 构建。记录 Rust 1.100.0、cargo-zigbuild 0.23.4、Zig 0.16.0、Node 22.23.4、npm 10.9.9 和锁定输入；这不是 bit-for-bit reproducible build 声明。

| 最终资产 | Bytes | SHA-256 |
| --- | ---: | --- |
| `lintel-cli-0.2.0-candidate-353536cc7dc3-macos-arm64.tar.gz` | 9,556,992 | `ac62ded7cdbe377d517068ffb55d4db5db0f69245e5177fe45e1c01733579a9e` |
| `lintel-cli-0.2.0-candidate-353536cc7dc3-linux-x86_64.tar.gz` | 9,807,758 | `a611f5ec8a8067f0c983a93af6756827bdb20aa31b8f786b0a2bd3795f04d98b` |
| `lintel-cli-0.2.0-candidate-353536cc7dc3-linux-aarch64.tar.gz` | 9,542,757 | `6ac7aa1308c485dbabb89305eba126ff8e8ed9f210c622e29f0bc55abbf77793` |
| `Lintel_0.2.0_aarch64.dmg` | 13,753,858 | `9865fc72ee05d01b1fb579b5333b579772319debe18efe9198a136c1c86bbf18` |

原 GitHub artifact **14085474075** ZIP 保持不改包：45,199,043 bytes，SHA-256 `ebd9f53edae5b7cdf88bf0acab0f941c7d2b6ace7dc6891f07ec0cafa2b52230`。另保留 Linux aarch64 native report artifact **14086765725**（`c8ddcf0945cf07aa88a81fe1e3eb75003cb567aa246bdc97c351d4f305c2f9c9`）、x86_64 **14086815445**（`abc49063cf4c97bbea9ca2ec69ae4da94b03c67cb1e1fdf4cb1b4765962b00d4`）。下载字节与 GitHub 已认证 artifact metadata digest 一致；root 另外重读四个实际包 SHA，并复核 native 报告与选定 native 图像。每个 CLI 有 28 个 regular members，执行位、安全 modes、完整内部 SHA256SUMS 均验证。

生产候选 gate 绑定最终 source、14-job full plan、12 reports 与同一套包／DMG：artifact **14087520922**，8,860,119 bytes，SHA-256 `44110d76798336ce72c92152b9823cd2ee262ee7e3e985ce165bcb16bd119f1f`；实际 `candidate-verified.json` 为 `ci_profile:full`、`consistency_passed:true`、`release_authorized:false`，其 1,722 bytes／SHA-256 `5082b063ae9362345ad3c7098bd643266f8d951eb0581f6f892a144389db0ed6` 与 provenance 相符。`build-receipt.json` 的 `origin_authenticated:false`、local verifier 的同类边界仍保留；独立的 GitHub run／artifact metadata 是另一层 provenance，不把本地 JSON 自称的身份升级成签名。SHA-256 也不是 Developer ID 或独立分发来源认证。

### 失败历史没有被新的绿灯覆盖

第一套 `e573ce228e15ee6cb03c2705baf70a8bab1a8c75` 的三个 native 包通过，但 canonical 在 macOS PTY 输出采集和 Linux argv-only 空 stdin EPIPE 失败。它们都修正了测试 transport，不放宽产品断言。

第二套 `da7ebd2024384ea2019719f63dabef3d9db6cd32` 三平台包、browser/UI 及 native WebKit 窗口通过，Mac desktop numeric-port assertion 失败。candidate mandatory gate 实际拒绝了该失败 source，未借用其他提交绿灯。原数字端口断言具有已在真实 TCP 复现的 self-connect 假阳性边界，但那次 macOS 成功连接的具体原因未知；最终测试直接核对原 listener owner、双向真实 tunnel 终止及取消后仍保有原 owner，没有发现需要改变生产 shutdown 的证据。第二套也早于 TLS 安全刷新，两套都不作为本次发行候选。

独立证据目录保留 baseline、三套 runs、原始 reports、job logs、artifact digests、故障反例和最终机器可读记录。历史实际包以原 artifact 标识追溯，不把 intermediate bytes 放进最终安装清单。
## 4. 升级、保留与回退

0.1.0 CLI 到候选的实验使用**实际旧公开二进制**创建合成 policy job 和 encrypted archive。新包查询原 ID/计划/回执，再重复原批准，并比较真实密文摘要与归档目录清单不变；另一份独立 HOME/state 读取并迁入选定原文，逐字节一致。旧 executable 摘要保持原值。这个实验检验了查询与资料兼容，也排除了“回同一 ID 却暗中重做”的假成功。

没有依靠一次破坏性 state migration 升级。本轮主要响应字段为 additive；旧已接受任务保持 query-only。未执行、缺新增认证目录绑定的 launch/resume/runtime cleanup 预览先拒绝并要求重新预览；旧已接受任务仍可查，本地复查与无法沿用的 runtime 部分分开。更早缺逐来源 schema 的历史任务可读原回执，新式带 job auth verify 返回明确的 `legacy_auth_coverage`。

CLI 安装沿用包内 README：创建尚不存在的独立 identity 目录，解压后检查 SHA256SUMS，再显式调用 `bin/lintel version --json` 和 `capabilities --json`。不自动改 PATH、shell rc、App、服务或 browser registration。升级选择另一目录，保留旧 binary、state、归档和原 ID；不中途覆盖使用中的版本。

App 手动升级前退出旧 App 并保留旧副本；资料/state 不在 bundle 中，不删除它们解决安装失败。新版的查询兼容不等于旧程序可以安全写入全部新状态，回退优先使用原 ID 只读核对，保留证据；旧副本的继续修改需确认格式与行为兼容。正在运行的旧 App 替换、安装中断与正式 updater 替换尚未由上述 CLI 实验证明。

## 5. 发布资产与网站交接

候选包保留 receipt、component inventory、三个 CLI archives、一个 DMG、三份 native CLI JSON、App trust report、canonical gate/provenance、双语 release notes 与安装说明。项目 terms 和第三方 notice 不从包中拆走。候选 build/source provenance 和 SHA256SUMS 都不是新的发布签名体系。

在获得公开发布决定后，维护者应采用下列顺序：

1. 选择已经通过的**同一组 bytes**，确认 source、版本、架构、notice、安装说明、摘要和发行渠道。发生重签名／公证／重新打包时产生新摘要，并重新核对最终 DMG；不能沿用旧 bytes 的结果。
2. 发布经批准的平台，明确 Preview、实际信任条件和未包含的平台。App 和 CLI 可以分开发行，不让未满足 App 条件阻塞已通过的 CLI。
3. 从公开资产地址实际下载读回，比较大小/摘要，并检查包内身份；随后执行既有 `app-release.mjs prepare/verify-public` 对应的手动分发记录流程。未公开时保留 `publication_verified:false`，不制造可用下载或 feed。
4. 中英文网站均核对 App/CLI version、source、实际下载地址、安装限制与许可。源码新增的 CLI 下载入口只指向真实公开 0.1.0，0.2.0 发布后必须用核实的新资产同步更新。生成 payload 与部署是独立动作；本轮不部署。
5. 旧 App 没有配置 update key/feed，先采用清楚的手动升级。以后启用 updater 时再确定密钥持有者、public key、签名 archive、固定 feed、旧安装中断恢复与手动 fallback；SHA-256 不替代更新签名。

## 6. 剩余工作：可直接交给本地 Codex

### A. 正式 macOS 分发身份与首次下载

**需要 Faye 决定：** 是否投入 Developer ID/公证，或明确继续限制为了解手动信任操作的技术 Preview。签名身份与私钥只由所有者的受控环境提供。工程人员可以完成构建、签名校验、公证提交/票据处理和重新打包，但本轮没有这项授权或凭据。

**执行与通过标准：** 在专用测试 Mac 记录 OS/CPU、DMG SHA、source；按项目构建入口产出完整 bundle。严格 codesign、Developer ID identity、hardened runtime、公证与 stapler 的实际结果全部保存。通过浏览器从经批准的真实下载来源取得文件，保留 quarantine，用正常 Finder/Applications 路径安装并首次打开；记录系统提示和最终结果。不能将 CLI curl 下载、直接 binary launch 或手工取消隔离当成此项成功。

### B. 当前候选的 native 操作与升级

**不需要新的产品决定。** 在可交互的受控 Mac 执行 [App 首次使用验收](../app-updates.md#候选首次使用验收) 的合成六文件流程：登记、恰好三件选择、批准前无密文、批准一次、原 ID 完成、六份原 bytes、退出重开、独立读包、错误口令、中英主题与焦点。使用本轮候选本身，不用另一个 QA rebuild 冒充。

再用旧公开 App 在独立 state 建立已完成和未确定任务，退出后切换新 App；检查原记录与归档、未执行旧认证预览提示、失败后保持旧副本和原 state。对升级中断，只查询原记录；不删除 deduplication 状态。AX/window/screenshot 可辅助定位，但不能代替实际完整任务。此项可由有原生桌面的 Codex 执行；当前 Linux 工作环境已提供自动 native 包/窗口路径，缺完整可交互桌面时保留具体结果。


另外在同一受控 Mac、clean `353536cc` checkout 用**实际 DMG 复制 App 的资源**执行两项已存在但普通组忽略的消费者测试（路径替换成已核对的复制 App）：

```sh
LINTEL_TEST_BROWSER_HOST_RESOURCES=/absolute/copied/Lintel.app/Contents/Resources/browser-host \
  cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked \
  built_host_runs_from_installed_manifest_and_returns_native_frames -- --ignored
LINTEL_TEST_BROWSER_EXTENSION_RESOURCES=/absolute/copied/Lintel.app/Contents/Resources/browser-extensions \
  cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked \
  actual_packaged_extensions_install_from_bundle_with_no_profile_mutation -- --ignored
```

两者使用自己的合成 HOME；第一项由所装 manifest 执行真实 host 并核对有限 native frames，第二项消费三类扩展实际 bytes、只准备注册文件且不改 profile。通过标准是测试真实执行且通过，不接受 ignored／零测试或换成源码 fixture。这是可执行的 package resource 补充验收，不是完整实际浏览器用户旅程。

### C. 第三方图形与品牌许可

**需要 Faye 决定：** 提供 Clawd/相关品牌资产的可核实再分发许可，或选择不依赖该资产的原创替代。仓库的 attribution 与项目许可不产生该许可；不能通过测试补齐。按实际 App/site 资产检查；CLI 若未包含相关图形，不应被一并视为同一阻断。

### D. 最终公开资产与部署

**需要明确发布／部署授权。** 发布之前审阅本文选择的平台、版本与双语说明。授权后再创建正式 Release、读回公开 bytes、生成已验证网站 payload、部署并逐语言核对。updater 继续关闭是当前有效方案，不必为了发 Preview 先引入新的密钥运营。


当前网站源码已生成 **42 个中英文页面**，最终两平台 site-ui 通过；部署仍是本轮实际浏览器观察到的旧页面，不能混称。完整可部署 payload 还缺审核过的 `lintel-intro.mp4` 和 `lintel-film-poster.jpg`：当前工作区／可用资料中未找到这两个原件，apex 与文档列出的 deployment origin 的普通 HTTP 读取均返回 403。浏览器可以看到线上页面，所以这被记为取回媒体的环境边界，**不是官网故障**；没有用合成替代品伪造原媒体验收。

已重新下载核对旧公开 0.1.1 DMG，并用真实 `app-release.mjs prepare`／`verify-public` 产生 `evidence/site-staging-inputs/verified-public-0.1.1/release.json`，状态 `public_bytes_verified`。最终 0.2.0 DMG 则只生成 `release-preparation/app-manual-prepared/release.json`，状态 `prepared_local`、`publication_verified:false`，不含 updater feed。草稿中的未来 URL 不是可用下载声明，发布时间在正式批准时重新确定。

拿到两份原媒体后可直接执行（所有路径是明确的本地输入；out 必须不存在）：

```sh
node scripts/prepare-site-languages.mjs --check
LINTEL_SITE_MEDIA_DIR=/absolute/reviewed-media \
  python3 tests/verify.py --checks site-game-test,site-ui --require-passed
node scripts/prepare-site.mjs --media-dir /absolute/reviewed-media \
  --app-manual /absolute/verified-public-0.1.1/release.json \
  --out /absolute/absent-site-payload
```

发布 0.2.0 后再换成经实际公开读回的新 release record，并更新网站有限 CLI asset allowlist／三平台摘要。未批准发布之前，用旧公开 record 生成只会更新说明／CLI 入口的准备包；**不部署**。完整媒体 payload、部署和部署后逐页读回仍须分别记载。
## 7. 发布后演进，保持有限范围

可继续提高浏览器非 developer 安装、安装/升级便利性、跨版本客户端认证适配、真实账户/ACL 场景、macOS 最低版本覆盖、长期任务容量与 journaling 退休策略。browser 1000-operation admission cap 当前会明确拒绝新任务并保留原查询，不能通过删除记录绕过；需要后续兼容退休方案。

后续测试优先分配给会区分真实结果的路径：外部 writer 冲突、权限拒绝、掉电/持久化、客户端版本差异、下载隔离与操作系统实际授权、升级中断和真实 UI 错误回路。已足够验证的 exact-selection、原 ID 去重及加密原文路径保留既有证据，避免用重复数量替代新增可信范围。
