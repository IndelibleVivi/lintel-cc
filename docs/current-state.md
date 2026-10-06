# 当前状态

2026-10-06 · **0.1.0 产品基线的源码可靠性后续**；本地 native／CLI 候选仍是 2026-10-05 的构建。产品基线从 main@2f29b59 接续 [产品基线 v1.0](specs/product-baseline.md)，并保留 [完整 SPEC](SPEC.md) 的 G01–G04。A01–A18 已接通源码及分层验证；真实认证与完整原生窗口矩阵仍有具体未验证项，不等于原完整产品已经发行。

## 当前交付与证据

| 对象 | 当前事实与验收边界 |
| --- | --- |
| 共享产品合同 | 六任务 catalog；计划新增冻结目标／完整落点与用途；结构化任务结果与覆盖；旧协议、历史计划、包与原 ID 恢复保留 |
| App 界面 | 安静的 Clawd 首页／56 句原创时段问候与日期／整分钟／稀有彩蛋；日期只在当天首次打开显示，点击后展开六任务，保留持续目标；配置 root／项目 cwd 分离；资料阅读／交接稿；CLI 核对与 Agent。默认 1120、最低 900、宽 1440 成品 UI 检查通过；保护／保全／清理底栏独立于正文滚动 |
| 2026-10-06 连续性修复 | 本机→A／B→A 打开 A 的回执后，查询、Agent 包及启动预览均固定 A；参考指令与重名副本再次保全，不自动激活。官方注销将完整认证状态摘要冻结在私有计划，主体缺失拒绝，执行前及保全后复查；只验证 synthetic CLI，Keychain-only token 世代仍未验证 |
| 会话与启动 | bounded reader、未知块与 thinking 原件保全；普通启动／独立 resume／原启动 query/list 接入。持久 intent 先于私有副本／Terminal，未知后只查询原 ID。有限 adapter 仅静态识别 2.1.283／2.1.285；inert 参数、cwd、配置根与副本机制通过，真实认证 resume 尚未验收 |
| macOS native 候选 | 2026-10-05 unsigned arm64 Tauri App 构建通过。隔离 HOME／state 的 native WebKit 已实际创建环境、加密保全并建立新 root、逐字节读回三份资料。默认与 900×640 窗口已检查首页／任务入口，900 审批正文滚动后标题与底栏保持固定；Night→Day 控件换色已实际通过。宽窗口受当前屏幕限制，完整原生矩阵仍未逐一验收；截图为本地 QA |
| CLI 与静态检查 | context/tasks/深层嵌套帮助和原启动 query/list；App 明确 executable 的有限 native 探测。2026-10-06 默认 20 项已覆盖通过：首次 19 项通过，workspace 的两项已有安装测试触发 4 秒 transport deadline；12 项安装测试串行诊断及原样 workspace 补跑均通过。独立 baseline-ui／remote-task-ui 通过；2026-10-05 的完整 UI／browser 与 native 证据仍限于当时候选 |
| 资源与包装 | 2026-10-05 macOS arm64 CLI 与两架构 Linux musl runner 已构建，canonical App bundles 已更新；native host／非 fixture Chromium 与 Firefox 扩展由原 source 生成。CLI candidate 包装含真实输入 smoke 通过；生成物不进 Git。cross-build 不证明其他架构 runtime，checksums 不证明签名或来源 |
| 发行与账户 | 无正式签名／公证、Applications 安装、真实认证操作、官网托管或生产 VPS 激活；公开源码不等于正式发行 |

[基线验收记录](specs/product-baseline-status.md) 保存 A01–A18 的逐项证据与未验证条件；[候选历史](candidate-history.md) 保存 25fdb43 与更早候选的 CI／runtime 观察。历史 Linux VM 与 browser runtime 证据仍限定于当时源码和一次性环境，不证明这轮新增路径已验收。

## 入口与权威

| 读者／任务 | 权威入口 |
| --- | --- |
| 人类任务、保全／重建、继续工作、原任务核对 | [操作指南](operator-guide.md) |
| 独立 CLI、用户候选安装、stdin、schema 与 Agent | [Agent CLI](agents.md) |
| 前端构图、组件入口与 Selen 后续工作 | [桌面](desktop.md)、[视觉身份](visual-language.md) |
| 关联组件、容量与完整收尾的后续实现 | [环境覆盖与使用收尾方案](specs/environment-completion-plan.md)；包含依赖、有限范围、验收场景和发行 gates，状态为方案 |
| 当前 native／browser／Linux 验证边界 | [验证说明](verification.md)、[验收记录](specs/product-baseline-status.md) |
| 官网、示例与 opt-in 共享游戏 | [网站](../apps/site/README.md)；尚未部署、无正式下载 |
| core／SSH／systemd／browser／network | [core](core.md)、[remote](remote.md)、[services](services.md)、[browser](browser.md)、[network](network.md) |

统一验证 owner 是 `python3 tests/verify.py`；UI／browser startup／Linux runtime 为独立 selections。dirty 开发 App 的 docs revision 为 unknown；干净候选的 Lintel 文档资源指向自己的源码 revision，main 入口标为最新开发说明；独立 Infra Field Guide 使用自己的 main，标为独立指南最新版。不会开放任意 URL 或 shell。

仓库已公开；项目原创材料的公开复用许可证仍未选定，第三方权利独立。源码可见不构成通用复用许可。

## 完整目标仍缺少

1. **真实认证与写入者控制：** Keychain/共享 profile、Desktop/IDE/service 的完整定位与生命周期；有限 systemd 暂停已接通，其他 supervisor 暂停、重新登录及旧会话续用。官方注销已加入观察主体与状态绑定，并拒绝缺失主体的预览；同账号 Keychain 内部 token 更新如果不反映到 CLI 状态，仍无法识别。当前识别部分 Claude 进程及可访问 manager 的直接 root 绑定服务；明确要求目标写入者先停，不能归属的进程阻止清理。官方 auth 命令仅有合成 CLI 证据，不能宣称生产注销已验收。
2. **完整外发策略与强约束：** 七项自定义已接入；更多取舍、入口实效矩阵、macOS Network Extension 签名与权限、Linux namespace；直接 socket、UDP、DNS、NO_PROXY、子进程不能由代理覆盖证明。
3. **远程运行：** 用户级安装／版本绑定已接入候选；Ubuntu 隔离 OpenSSH 已验证安装／丢 ACK 核对／原任务查询／交互 PTY，真实 VM 已完成有限 system-manager 暂停恢复、符合既有条件的 system/user manager 严格 logout 后续跑、原任务完成与 reboot 后核对。setsid 不保证 logout 存活，托管路径也不保证 reboot 续跑；生产 VPS 安装与任务实跑、aarch64 runtime、其他 supervisor 和完整 G03 仍未验收。
4. **浏览器旅程：** 独立 Chromium 临时测试 profile 已完成真实持久安装／onStartup 两阶段清理；仍需正式 Chrome、Edge、Firefox 与 AdsPower 的安装、配对与清理验收。Firefox 独立 CacheStorage、按站点 proxy、容器后台停写、克隆识别和专用 browser 启动仍有缺口。
5. **发行与维护：** 菜单栏、定时漂移、签名规则更新、卸载、升级、正式签名/公证、性能预算。
6. **并发与恢复：** 不合作的外部编辑器或已持有文件描述符的 writer 没有 OS 级 CAS；不确定副作用需核对。混合状态加密备份不支持自动恢复；会话/记忆迁入不证明可以续聊。

这些是原始完整目标的差距，不是缩小后的新 SPEC。下一关口是正式浏览器／AdsPower 与非开发者安装、真实认证／入口矩阵、生产 VPS 任务验收。隔离 runtime 与合成数据能证明机制，不能替代真实产品场景；现有个人登录和生产服务不作开发 mutation fixture。
