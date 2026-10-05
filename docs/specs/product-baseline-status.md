# 产品基线实施与验收

接受合同：[产品基线 v1.0](product-baseline.md)，2026-10-05；施工起点为最新 `main@2f29b59`（起初 checkout 的 `b667851` 已 fast-forward 同步）。原完整产品 SPEC 的 G01–G04 与未交付事项继续保留。此前恢复、审批、浏览器接续和 runner binding 修复属于保留基础。

实施顺序为 A 共享合同 → B 人类操作／C 会话工作流／D CLI 与帮助 → E 集成交付。A01–A18 均已接入源码与相应验证；真实认证和生产操作仍受独立授权限制。用户同日修订首页观感：安静的 Clawd／原创时段问候，明确点击后展开六任务，保持原任务目标；固定操作栏不随正文滚动。未缩小完整 SPEC。

| 验收 | 责任与依赖 | 状态 | 证据或待完成条件 |
| --- | --- | --- | --- |
| A01 | B 首次入口 | 源码／UI 通过 | `baseline-ui`：无 root、stale selection 不误查；创建／登记、主机、六任务和 CLI 可达。默认首页不铺满任务卡 |
| A02 | A/B 冻结目标与草案 | 合成通过 | `baseline-ui`、`remote-task-ui`：同名 root 草案隔离，批准使用冻结主机／完整 root；切换环境不重贴原回执 |
| A03 | B 本机浏览器范围 | UI／浏览器通过 | 本机 scope 明示；`browser-pairing-ui` 使用真实 core／host 与一次性 Chromium profile。Tauri invoke／clipboard 是合成 transport |
| A04 | A/C cwd 与有限启动 | core／PTY 通过 | `journey-product-baseline`：inert Claude 读回独立 cwd 与配置 root，无 prompt；非 TTY JSON 请求不能打开客户端 |
| A05 | A 冻结新 root／清单 | core／UI／native 通过 | 新 root、命名、用途与全部文件落点先冻结；碰撞／父目录更换拒绝旧计划。native preserve 的三份文件逐字节读回一致 |
| A06 | A 原任务 intent／恢复 | core／UI 通过 | 部分迁入、私有副本失败、重复启动／丢回复保留原 intent、位置和阶段；原 ID 查询，不再建 root 或打开第二次 Terminal |
| A07 | C 原件与私有阅读 | core／资料旅程通过 | 非 ASCII、未知字段、signature、特殊换行包字节不变；提取排除 opaque thinking／signature；正文不入操作回执和 Agent 包 |
| A08 | C 有界分页／来源身份 | core／UI 通过 | 6000 条记录完整有界分页，损坏／未知格式明确展示；path、package／entry digest 与记录索引绑定，源变化拒绝旧选择 |
| A09 | A/B 资料用途与启用 | core／UI／native 通过 | App 默认参考资料；启用指令另行选择，批准与落盘一致。旧 raw import 默认激活语义保留；新工作交接稿不自动写入指令位置 |
| A10 | B/C 审阅／复制／启动 | core／UI 通过 | 编辑使审阅失效，复制失败不启动；启动失败／未知只查原请求，阶段不冒充模型接收；普通新上下文与原生 resume 各自明确选择 |
| A11 | C 有限 resume／支持证据 | 静态／inert 通过；真实认证未验证 | 仅静态识别的 2.1.283／2.1.285 与受支持 JSONL shape 开放；私有 byte-identical 副本、`--resume` 与 `--fork-session`、cwd／config、原件不变受测试覆盖。真实 Claude 接受和实际写入／原路径读取尚未验收 |
| A12 | D 静态 CLI／state 核对 | native checker／UI 通过 | 明确 executable；错误架构、协议、资源缺失／摘要和 state／UID 各自报告；固定 argv、有界输出／时限，静态检查不初始化 state。bytes 一致不等于源认证 |
| A13 | D 原任务 Agent 交接 | core／remote／UI 通过 | accepted／uncertain job 和原启动 ID 均保留准确查询命令、runner binding／PATH 兼容事实；移除 alias 后保留核对，secret sentinel 不进包、argv 或普通日志 |
| A14 | D 帮助／任务映射 | CLI 通过 | 顶层及深层 `--help` 提前只读返回；六任务／operations／schema 可解析；context、version、tasks 静态调用不建立 state |
| A15 | A/B 结果与覆盖 | core／UI 通过 | task_result 与 coverage 分开，旧 status 保留；本地清理、部分失败、unknown 和恢复冲突分别展示，下一步沿原任务 |
| A16 | B/E 成品与原生视觉 | UI 通过；native 部分通过 | Chromium 成品检查 1120×760／900×640／1440×900、Day／Night、长路径、侧栏、reduced motion、键盘焦点返回；默认首页无滚动、固定底栏坐标不随正文改变。native WebKit 已检查默认窗口首页／批准／结果／底栏；原生全部窗口矩阵未逐一完成 |
| A17 | A/C/D 旧合同兼容 | core／既有旅程通过 | protocol-1、lintel.work/1、旧 import 指令目的地、旧 launch cwd、旧回执与 runner binding 保留。旧未接收的新 root 计划需重新预览，已接收旧任务先返回原回执 |
| A18 | D/E 版本帮助／官网 | 源码／build 通过 | 固定 13 个文档资源共享表；clean docs revision 固定，dirty／unknown 标为最新开发说明。官网保持 illustrative static demo，无第三方请求，未部署、无正式下载 |

源码、合成 transport、候选构建、native WebKit、真实浏览器、Linux runtime 和真实 Claude resume 分别验收。不得用某一层通过替代其他层。

## 验证汇总与复现

- `python3 tests/verify.py` 默认 19 项通过；最终后续机制修改分别补跑相关 checks，没有把更早候选的 CI 当成本次证明。
- 独立 8 项 UI／browser checks 通过；首页和底栏修订后再次运行 desktop-build、baseline-ui、work-ui、remote-task-ui、clawd-app-ui、browser-pairing-ui、service-ui，7 项通过。
- `cargo test -p lintel-core baseline_tests -- --test-threads=1`：16 项通过；包含旧 portable import 的真实目的地、原回执查询与重复批准兼容。
- macOS arm64 CLI、Linux x86_64／aarch64 musl runner 已 cross-build；native Tauri App 已构建。Linux cross-build 不构成 Linux runtime 证据。
- native 证据仅使用一次性 HOME／state、inert Claude 和合成工作包。截图留在本地 QA，不包含个人环境或发布进仓库。真实账号／正式浏览器／生产 VPS 仍是独立验收关口。
