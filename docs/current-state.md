# 当前状态

2026-10-03 · 0.1.0 开发候选。完整 [SPEC](SPEC.md) 仍未交付，四条完整旅程 G01–G04 尚未通过。源码与本地 macOS App 已构建；未安装到 Applications、未正式发布、未部署真实远端。

## 已接通

- core / runner：环境登记与建立、版本化外发设置计划/批准/读回、字段恢复、漂移、脱敏支持资料；有限登录修复/客户端重建/退役；加密归档、文本阅读与选择性迁入。清理范围是预览中的准确文件和可显式调用的官方认证入口，不等于全客户端清场。概览统计与归档准入解耦（大文件不阻断 inspect）；二次重建会把此前迁入 lintel-imports 的工作重新计入归档；损坏 settings 不阻断不修改 settings 的保全计划；各变量分别按官方非空值或 boolean 语义判断；静态识别 native/npm 产品版本，Remote Control 按版本／Trusted Devices 条件评估，未知条件显式标注。冲突旧值通过用户选定的准确删除 diff 解除；预览冻结策略与产品版本，变更后拒绝执行。
- 桌面：同一套本机/SSH 环境、计划、清理、归档和任务界面；浏览器模块全操作入口、Native Messaging 注册计划与配对；本机代理启停/连接观察。所有修改先生成计划或独立确认。
- 界面：聊天式首页按角色招呼、操作框、一行工作入口重排，玩耍入口集中到不挤动工作区的口袋菜单。Clawd 支持摸摸／拖抱／弹飞／连续戳戳害羞与躲藏、下拉起飞；四幅重新绘制的字符风景保留清楚角色轮廓与画面比例，另有跳跃小游戏。提供 Day / Night / System、可选本地字体、键盘操作及减少动态效果。字体文件不进入 Git，干净 checkout 使用系统 fallback。
- SSH：native Rust bridge 使用系统 OpenSSH、静态 alias、严格 host key 与有限 JSON 请求；`lintel submit` 持久接收后返回 ACK，独立会话 worker 执行，重连只查询原任务。桌面可移除／撤销移除 alias；原任务与去重记录保留。新增用户批准的 Linux x86_64 / arm64 runner 探测、安装预览、内置文件上传、SHA／权限／能力核验与 alias 版本绑定；上传前持久 intent，丢 ACK 只核对原安装，原任务冻结 runner。仅写用户专用版本目录，不修改 PATH／系统服务／Claude。错误提供阶段、具体原因、排查步骤、退出码、限长 stderr 与只读核验命令，查询错误同样保留诊断。Python controller 是可选 CLI，不是桌面依赖，也未接入这一轮的结构化诊断。
- 帮助：App 内提供开发者 GitHub、Lintel 源码说明、Infra Field Guide 的 VPS 101 与 SSH 排障入口。macOS native 仅打开固定 HTTPS 文档资源；远端准备文档区分 App 批准安装与独立 CLI 的手工 PATH 准备。
- 浏览器：Chromium MV3 / Firefox 独立适配、Native Messaging host、实例冲突/配对/持久操作记录与固定路径安装器。Chromium 清理先隔离、关闭目标及 iframe 宿主、注销 worker，等待完整浏览器重启，再用新确认继续删除。
- 网络：loopback CONNECT / 有限 HTTP 转发、精确域名/端口规则、上游与连接事件；仅证明经过通道的流量。native start/status 返回实际采用的规范化 active_config；界面分开呈现当前生效配置与按环境保存的下次启动草案，支持默认动作及允许／阻止规则和端口。

## 证据与未验证范围

| 范围 | 当前证据 |
| --- | --- |
| Core | 24 项 Rust tests 通过（新增版本／变量／组织矩阵、静态版本来源、显式解除与外部旧值保护），其余包括，旧计划冲突、归档导入拒绝覆盖、退役、官方注销 fake CLI、共享认证范围变化拒绝、删除窗口内替换新凭据保留，以及本轮新增：二次重建保留 lintel-imports 原有工作、9 MiB 会话不阻断 inspect、损坏 settings 下独立保全计划可预览并执行且原字节不变 |
| CLI / submission | 实际 CLI 配置往返旅程通过；独立 worker 的 durable ACK、父进程退出后完成、原 ID 查询、去重、口令不落记录通过。PTY 中重建/批准/无回显口令/加密归档通过。新增 work_preservation journey 3/3（大文件概览、A→B→C 保留、损坏 settings 独立保全）；policy journey 覆盖版本变化拒绝、外部编辑、准确删除、恢复外部旧值、receipt 策略证据 |
| Egress | 5 unit + 7 localhost socket tests 通过；新增 inet_aton 式／IPv4-mapped IP 写法不能绕过精确规则、wire 配置缺省 default_action 直接拒绝的回归。本轮 native network tests 3/3 新鲜通过，覆盖规范化生效配置读回、精确规则命中、环境隔离、无效配置与通道结束状态。没有真实 Claude 公网探针或进程强约束证据 |
| Browser | 20 JS + 11 native host Rust tests 通过；新增：通用 control 不能放行扩展（授权仅限安装路径）、配对码 12 hex／5 次失败作废／pending 上限、browser 由调用方身份派生、running 回执移到 durable 边界之后、DNR 读回数组序不敏感。旧静止 SW smoke 曾通过；活跃 SW 负例证实注销后仍可能回写。本轮实际 smoke 的目标／iframe 隔离和邻站保留通过，但当前两阶段完整 smoke 被 `browser_restart_required` 拒绝，因为命令行临时扩展加载未观察到真实 onStartup；独立 CDP loadUnpacked 安装在重启后也消失，未取得真实持久安装证据；没有伪造启动世代，不能标为完整通过 |
| SSH | 本轮 native SSH 27/27、完整 desktop Rust suite 31/31通过，新增上传真实字节／权限／哈希、过期目标／文件／批准拒绝、丢回包只核对、外部文件保留、平台／能力拒绝、原任务版本冻结；既有覆盖移除后保留任务／去重、具体失败分类、stderr 并发排空／限长／去敏、只读排查命令、查询原错误保留；既有严格 host key、固定命令、丢 ACK 查询继续通过。固定关闭 `ProxyCommand=none` / `RemoteCommand=none`，observe 仅采用 id 匹配的回执，私有目录校验属主。Python fake-SSH 11/11 本轮入口复跑通过，未改其实现。两台真实 Linux x86_64 主机已完成严格 SSH 与生产安装脚本的只读探测，OS／架构／UID／安装条件返回有效且目标身份不同；未上传、未安装、未运行 Claude 或 runner。Linux logout/cgroup 与主机重启恢复未验收 |
| Web UI | 合成 root 中计划/执行/恢复、归档解锁/阅读/冲突拒绝/新环境迁入、四清理配方、退役重新启用、支持资料保存已走通。重排后 Home 的 Day/Night、900×640布局、工作入口、口袋展开不挤动操作框、连续戳戳／躲藏／拖甩、鼠标／滚轮／模拟触摸、焦点返回和减少动态已验证；四画收星、等比例缩放、翻页，游戏跳跃／暂停／碰撞／重开与本机最高分通过。网络面板经 synthetic native-response harness 验证延迟响应隔离和停止／编辑／重启。新增安装卡合成 native bridge 验证预览前不上传、显式批准、丢回包后的状态同步、原安装只读查询、关闭／重开／迟到响应隔离与连接管理；安装卡 Day/Night 和 900×640 长路径详情已实际渲染检查，视觉仍待用户接受。F04 界面使用真实合成 CLI 完成计划／批准／解除／回执／组织条件往返；新增 SSH 合成 native-response harness 验证具体错误／摘要复制、显式重连、移除／撤销／失败保留、当前主机回本机、原任务查询和关闭面板后的迟到响应隔离；帮助链接 ID 与 900×640 长命令排版通过。其后完成纸上晨光/夜里月光环境光、首页问候纵向重排、统一柔影刻度、clay tint 选中态的 Day/Night/窄屏截图 QA；浏览器面板移除独立「允许扩展 ID」按钮（扩展授权并入用户确认的安装路径），经合成空间复核；概览「已设置关闭」计数使用各变量解析后的 configured 状态。视觉稿仍属候选 |
| macOS | arm64 App 本地构建成功，约 16.72 MiB，含本轮策略界面与两种静态 Linux runner；typecheck、Vite 与 Tauri release 构建通过。此前已观察原生 WebKit 首页和 Clawd，本轮 SSH 界面使用浏览器合成 bridge 验证，未完成新 native runtime／真实 VPS 交互验收；没有自动重启已打开的旧窗口。无 Developer ID、公证、正式分发或 Applications 安装验收 |

统一入口 [tests/verify.py](../tests/verify.py) 聚合 root、独立 desktop/native-host Rust、JS、Python、前端与四条合成 journey；浏览器与 native bundle 显式 opt-in。macOS 本轮默认 11 个检查通过；入口证据记录检查时的 HEAD 与工作树状态。CI 提供 macOS/Linux 合成 matrix，其结果与真实目标 runtime 验收分别记录。[首次 CI](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37119853055) 的 clean HEAD `1654211` 已在 macOS 与 Ubuntu 分别完成 11/11（无跳过）；包括 Linux 源码 runner 的合成 CLI／detached submission 旅程。两种 Linux musl ELF 已交叉构建并打包，但 App 内置的这两份 musl ELF 尚未在目标 Linux 主机执行。

实际主机为 macOS arm64。App 内置 Linux musl runner 执行、正式 Chrome/Edge/Firefox、真实 Claude 身份与平台认证机制没有实机验收。细项证据在 [acceptance-status.json](acceptance-status.json)；局部测试不自动完成整个验收用例。

## 完整目标仍缺少

1. **真实认证与写入者控制：** Keychain/共享 profile、Desktop/IDE/service 的完整定位与生命周期，supervisor 暂停、重新登录及旧会话续用。当前仅按名称识别部分 Claude 进程；明确要求目标写入者先停，不能归属的进程阻止清理。官方 auth 命令仅有合成 CLI 证据，不能宣称生产注销已验收。
2. **完整外发策略与强约束：** 自定义/更多取舍、入口实效矩阵、macOS Network Extension 签名与权限、Linux namespace；直接 socket、UDP、DNS、NO_PROXY、子进程不能由代理覆盖证明。
3. **远程运行：** 用户级安装／版本绑定已接入候选，但真实 VPS 安装与任务实跑、supervisor、真实 SSH logout/cgroup 和主机重启恢复仍未验收。当前 detached 本地证据不能取代远端 runtime 验收。
4. **浏览器旅程：** 首先在真实持久安装的独立测试 profile 完成 onStartup 两阶段清理验收，再验 Chrome、Edge、Firefox。Firefox 独立 CacheStorage、按站点 proxy、容器后台停写、克隆识别和专用 browser 启动仍有缺口。
5. **发行与维护：** 菜单栏、定时漂移、签名规则更新、卸载、升级、正式签名/公证、性能预算。
6. **并发与恢复：** 不合作的外部编辑器或已持有文件描述符的 writer 没有 OS 级 CAS；不确定副作用需核对。混合状态加密备份不支持自动恢复；会话/记忆迁入不证明可以续聊。

这些是原始完整目标的差距，不是缩小后的新 SPEC。下一关口是持久浏览器测试安装、真实认证/入口矩阵与独立 Linux runtime 验收；不在个人登录或生产服务上替代测试。
