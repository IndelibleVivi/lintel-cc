# 当前状态

2026-10-04 · 0.1.0 开发候选。完整 [SPEC](SPEC.md) 仍未交付，四条完整旅程 G01–G04 尚未通过。源码与本地 macOS App 已构建；未安装到 Applications、未正式发布、未部署真实远端。

## 浏览器首次连接与真实重启验收（候选）

App 伴随扩展文件准备已接通：非 fixture Chromium / Firefox 包随 App 构建，桌面选择浏览器后只读预览，再批准准备稳定的当前用户目录。Chrome / Edge 共用 Chromium 目录，Firefox 独立目录；受管更新保持加载路径，非受管或被改动的内容拒绝覆盖，整份计划与当前文件重新核对。准备后的 Finder 入口和路径复制只交接文件，目标 profile 仍需浏览器开发加载。安装器中断切换须新的恢复预览与批准；原批准资源改变则停止恢复，保留文件供核对，不显示成完成。

随后填入准确扩展 ID，另行预览并批准内置 Native Messaging host 安装，再复制 12 位配对短码到目标扩展提交请求，核对后在 App 批准。页面已修正复制内部 challenge 的错误；真实 framed native 请求对旧复制值返回 `invalid_pairing_code`，新复制值与显式批准通过。host 稳定目录／精确注册／owned 更新与 query-only 重复核对保留；独立 CLI 手工 installer 仍有实际用途，桌面不接受任意 host 路径。目录准备、组件注册、profile 配对与在线轮询是不同事实。

此前内置扩展轮 desktop 检查 57 passed、0 failed、0 ignored，明确过滤 1 项独立 Linux runtime；其中两项分别从 **App 包内资源** 安装实际扩展／host 到临时 home，验证扩展文件完全一致、未创建浏览器 profile 或注册，以及真实 host native frames／非授权 extension 拒绝。扩展安装器 12 项含实际资源测试通过，覆盖完整批准、资源／目标变更、fixture 拒绝、受管更新、非受管保留、symlink 和目录切换中断／部分清理恢复。TypeScript、Vite 与 Tauri App build 通过。

构建页面的合成 native bridge 检查扩展预览不安装、整份批准、有限 reveal、路径复制（合成 clipboard）、三种浏览器说明、更新／重复核对／中断继续与冲突禁用、过期／缺资源／冲突、浏览器切换撤回状态、pending 禁用、单独配对、键盘与 Day/Night 1120／900 布局；长路径可读，无横向溢出。视觉仍属候选，未证明 native WebKit、Finder 的实际展示或浏览器加载。

当前 macOS App 约 17.17 MiB，含 Chromium 47799 bytes、Firefox 47961 bytes（各 8 个文件），实际 bytes／摘要与包内 inventory 一致。独立 host 787440 bytes 与两种既有 Linux runner 的大小／SHA 也相符。默认包无 woff2，本地六份字体保留；开发／显式 local-candidate 可用本地字体。host 只支持 native 同架构构建，跨架构／universal 明确拒绝。App 未安装到 Applications、激活、公证或发布；真实 VPS 未写入。

此前 clean `67f8549` 已通过 [CI 验收](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37142533163)：macOS／Ubuntu 默认各 12/12，无跳过；Ubuntu 独立 OpenSSH runtime 1/1、0 ignored，三份产物记录同一 clean HEAD。该次 CI 没有真实浏览器检查；App 包内资源 opt-in 与渲染检查仍是独立本机证据。

2026-10-04，macOS arm64／Playwright Chromium 155.0.8059.12 的完整两阶段 `browser-smoke` 已通过，11 项原断言全部保留。测试专用原生安装恢复在独立临时 profile 持久安装 synthetic 扩展；旧浏览器进程实际退出、新进程重新启动且无扩展加载 flags，同一扩展身份保留，生产 `runtime.onStartup` 监听器产生新的世代，再单独批准 `finishClear`。活跃 SW／iframe writer、目标隔离／邻域保留、五类存储读回、定位恢复、旧操作不重删新合成登录、真实 Native Messaging 往返均通过。没有修改 profile preferences 或伪造启动世代；不是 App 自动加载扩展的功能。

新的 `browser-pairing-ui` 也通过：构建 App 页面使用真实 core／host 进程、合成 invoke／clipboard，实际复制值经 framed `pair_request` 接受并由 App 显式批准，保持“已配对／当前离线”区别；短码更新、键盘和 Day/Night 通过。TypeScript／Vite／Tauri App 已按短码修正重建。CI 已增加 macOS／Ubuntu 独立 browser smoke 和配对检查；本轮 clean HEAD CI 结果待回收，不能把本机 dirty-source 成功当作 clean CI 证明。

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
- SSH：native Rust bridge 使用系统 OpenSSH、静态 alias、严格 host key 与有限 JSON 请求；`lintel submit` 持久接收后返回 ACK，独立会话 worker 执行，重连只查询原任务。桌面可移除／撤销移除 alias；原任务与去重记录保留。新增用户批准的 Linux x86_64 / arm64 runner 探测、安装预览、内置文件上传、SHA／权限／能力核验与 alias 版本绑定；上传前持久 intent，丢 ACK 只核对原安装，原任务冻结 runner。仅写用户专用版本目录，不修改 PATH／系统服务／Claude。错误提供阶段、具体原因、排查步骤、退出码、限长 stderr 与只读核验命令，查询错误同样保留诊断。Python controller 是可选 CLI，不是桌面依赖，也未接入这一轮的结构化诊断。
- 帮助：App 内提供开发者 GitHub、Lintel 源码说明、Infra Field Guide 的 VPS 101 与 SSH 排障入口。macOS native 仅打开固定 HTTPS 文档资源；远端准备文档区分 App 批准安装与独立 CLI 的手工 PATH 准备。
- 浏览器：Chromium MV3 / Firefox 独立适配、Native Messaging host、实例冲突/配对/持久操作记录与固定路径安装器。Chromium 清理先隔离、关闭目标及 iframe 宿主、注销 worker，等待完整浏览器重启，再用新确认继续删除。
- 网络：loopback CONNECT / 有限 HTTP 转发、精确域名/端口规则、上游与连接事件；仅证明经过通道的流量。native start/status 返回实际采用的规范化 active_config；界面分开呈现当前生效配置与按环境保存的下次启动草案，支持默认动作及允许／阻止规则和端口。

## 证据与未验证范围

| 范围 | 当前证据 |
| --- | --- |
| Core | 31 项 Rust tests 通过（新增版本／变量／组织矩阵、静态版本来源、显式解除与外部旧值保护），其余包括，旧计划冲突、归档导入拒绝覆盖、退役、官方注销 fake CLI、共享认证范围变化拒绝、删除窗口内替换新凭据保留，以及本轮新增：二次重建保留 lintel-imports 原有工作、9 MiB 会话不阻断 inspect、损坏 settings 下独立保全计划可预览并执行且原字节不变 |
| CLI / submission | 实际 CLI 配置往返旅程通过；独立 worker 的 durable ACK、父进程退出后完成、原 ID 查询、去重、口令不落记录通过。PTY 中重建/批准/无回显口令/加密归档通过。新增 work_preservation journey 3/3（大文件概览、A→B→C 保留、损坏 settings 独立保全）；policy journey 覆盖版本变化拒绝、外部编辑、准确删除、恢复外部旧值、receipt 策略证据 |
| Egress | 5 unit + 7 localhost socket tests 通过；新增 inet_aton 式／IPv4-mapped IP 写法不能绕过精确规则、wire 配置缺省 default_action 直接拒绝的回归。本轮 native network tests 3/3 新鲜通过，覆盖规范化生效配置读回、精确规则命中、环境隔离、无效配置与通道结束状态。没有真实 Claude 公网探针或进程强约束证据 |
| Browser | 20 JS 与当前 12 native host Rust tests 通过；App 内置扩展安装器 12 项含包内资源测试、host 安装／更新与实际 executable 验证见上节。既有覆盖：通用 control 不能放行扩展（授权仅限安装路径）、配对码 12 hex／5 次失败作废／pending 上限、browser 由调用方身份派生、running 回执移到 durable 边界之后、DNR 读回数组序不敏感。活跃 SW 负例证实注销后仍可能回写；当前真实 Chromium 持久安装／完整退出与原生 onStartup／二次确认 smoke 已通过，详见上节。覆盖 active SW／iframe writer、五类目标存储与邻域保留、隔离／权限恢复、重复操作不重删及 Native Messaging 持久回执。正式三浏览器与 AdsPower／Firefox 容器／真实网站仍未验收 |
| SSH | 当前本机 desktop Rust suite 57 passed、0 ignored（包含两项包内资源 opt-in，过滤独立 Linux runtime；该 runtime 已在本轮 Ubuntu CI 单独 1/1 通过），保留既有 SSH 回归，包括上传真实字节／权限／哈希、过期目标／文件／批准拒绝、丢回包只核对、外部文件保留、平台／能力拒绝、原任务版本冻结；既有覆盖移除后保留任务／去重、具体失败分类、stderr 并发排空／限长／去敏、只读排查命令、查询原错误保留；既有严格 host key、固定命令、丢 ACK 查询继续通过。固定关闭 `ProxyCommand=none` / `RemoteCommand=none`，observe 仅采用 id 匹配的回执，私有目录校验属主。Python fake-SSH 12/12 通过，含有限 custom schema 与 stdin payload 回归。两台真实 Linux x86_64 主机已完成严格 SSH 与生产安装脚本的只读探测，OS／架构／UID／安装条件返回有效且目标身份不同；未上传、未安装、未运行 Claude 或 runner。Linux logout/cgroup 与主机重启恢复未验收 |
| Web UI | 合成 root 中计划/执行/恢复、归档解锁/阅读/冲突拒绝/新环境迁入、四清理配方、退役重新启用、支持资料保存已走通。重排后 Home 的 Day/Night、900×640布局、工作入口、口袋展开不挤动操作框、连续戳戳／躲藏／拖甩、鼠标／滚轮／模拟触摸、焦点返回和减少动态已验证；四画收星、等比例缩放、翻页，游戏跳跃／暂停／碰撞／重开与本机最高分通过。网络面板经 synthetic native-response harness 验证延迟响应隔离和停止／编辑／重启。新增安装卡合成 native bridge 验证预览前不上传、显式批准、丢回包后的状态同步、原安装只读查询、关闭／重开／迟到响应隔离与连接管理；安装卡 Day/Night 和 900×640 长路径详情已实际渲染检查，视觉仍待用户接受。F04 界面使用真实合成 CLI 完成计划／批准／解除／回执／组织条件往返；新增 SSH 合成 native-response harness 验证具体错误／摘要复制、显式重连、移除／撤销／失败保留、当前主机回本机、原任务查询和关闭面板后的迟到响应隔离；帮助链接 ID 与 900×640 长命令排版通过。其后完成纸上晨光/夜里月光环境光、首页问候纵向重排、统一柔影刻度、clay tint 选中态的 Day/Night/窄屏截图 QA；浏览器面板移除独立「允许扩展 ID」按钮（扩展授权并入用户确认的安装路径），经合成空间复核；概览「已设置关闭」计数使用各变量解析后的 configured 状态。视觉稿仍属候选 |
| macOS | 当前 arm64 App 本地构建成功，约 17.17 MiB，含自定义策略、内置 Chromium／Firefox 扩展、browser host 与两种静态 Linux runner；默认包排除本机可选字体。typecheck、Vite 与 Tauri release 构建通过。此前已观察原生 WebKit 首页和 Clawd，本轮扩展界面使用浏览器合成 bridge 验证，未完成新 native runtime／真实 VPS 交互验收；没有自动重启已打开的旧窗口。无 Developer ID、公证、正式分发或 Applications 安装验收 |

统一入口 [tests/verify.py](../tests/verify.py) 聚合 root、独立 desktop/native-host Rust、JS、Python、前端与五条合成 journey；浏览器与 native bundle 显式 opt-in，CI 已单独加入两项浏览器检查。此前 clean `67f8549` 的 macOS／Ubuntu 默认 12 个检查分别通过，Ubuntu 独立 Linux runtime 也已通过；当前本机 App、内置扩展与 host 独立证据见上节。入口证据记录检查时的 HEAD 与工作树状态。CI 提供 macOS/Linux 合成 matrix，其结果与真实目标 runtime 验收分别记录。[首次 CI](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37119853055) 的 clean HEAD `1654211` 已在 macOS 与 Ubuntu 分别完成 11/11（无跳过）；包括 Linux 源码 runner 的合成 CLI／detached submission 旅程。两种 Linux musl ELF 已交叉构建并打包。后续 `1276c84` 的 x86_64 musl 构建已在 Ubuntu 隔离 SSH 中执行安装、后台任务和交互启动旅程；App 内资源独立通过字节／SHA 打包核验，aarch64 runtime 仍未验收。

实际主机为 macOS arm64。Linux x86_64 musl 已有上述 CI runtime 证据；aarch64、正式 Chrome/Edge/Firefox、真实 Claude 身份与平台认证机制没有实机验收。细项证据在 [acceptance-status.json](acceptance-status.json)；局部测试不自动完成整个验收用例。

## 完整目标仍缺少

1. **真实认证与写入者控制：** Keychain/共享 profile、Desktop/IDE/service 的完整定位与生命周期，supervisor 暂停、重新登录及旧会话续用。当前仅按名称识别部分 Claude 进程；明确要求目标写入者先停，不能归属的进程阻止清理。官方 auth 命令仅有合成 CLI 证据，不能宣称生产注销已验收。
2. **完整外发策略与强约束：** 七项自定义已接入；更多取舍、入口实效矩阵、macOS Network Extension 签名与权限、Linux namespace；直接 socket、UDP、DNS、NO_PROXY、子进程不能由代理覆盖证明。
3. **远程运行：** 用户级安装／版本绑定已接入候选，但真实 VPS 安装与任务实跑、supervisor、真实 SSH logout/cgroup 和主机重启恢复仍未验收。Ubuntu 隔离 OpenSSH 已验证安装／丢 ACK 核对／detached 原任务查询／交互 PTY；它不能取代生产 VPS session/cgroup 与重启验收。
4. **浏览器旅程：** 独立 Chromium 临时测试 profile 已完成真实持久安装／onStartup 两阶段清理；仍需正式 Chrome、Edge、Firefox 与 AdsPower 的安装、配对与清理验收。Firefox 独立 CacheStorage、按站点 proxy、容器后台停写、克隆识别和专用 browser 启动仍有缺口。
5. **发行与维护：** 菜单栏、定时漂移、签名规则更新、卸载、升级、正式签名/公证、性能预算。
6. **并发与恢复：** 不合作的外部编辑器或已持有文件描述符的 writer 没有 OS 级 CAS；不确定副作用需核对。混合状态加密备份不支持自动恢复；会话/记忆迁入不证明可以续聊。

这些是原始完整目标的差距，不是缩小后的新 SPEC。下一关口是正式浏览器／AdsPower 与非开发者安装、真实认证／入口矩阵、VPS session/cgroup／重启验收。隔离 runtime 与合成数据能证明机制，不能替代真实产品场景；现有个人登录和生产服务不作开发 mutation fixture。
