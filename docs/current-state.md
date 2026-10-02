# 当前状态

2026-10-03 · 0.1.0 开发候选。完整 [SPEC](SPEC.md) 仍未交付，四条完整旅程 G01–G04 尚未通过。源码与本地 macOS App 已构建；未安装到 Applications、未正式发布、未部署真实远端。

## 已接通

- core / runner：环境登记与建立、四项外发设置计划/批准/读回、字段恢复、漂移、脱敏支持资料；有限登录修复/客户端重建/退役；加密归档、文本阅读与选择性迁入。清理范围是预览中的准确文件和可显式调用的官方认证入口，不等于全客户端清场。
- 桌面：同一套本机/SSH 环境、计划、清理、归档和任务界面；浏览器模块全操作入口、Native Messaging 注册计划与配对；本机代理启停/连接观察。所有修改先生成计划或独立确认。
- 界面：聊天式结构化首页、Clawd 像素几何与八种点按场景、Day / Night / System、可选本地字体、键盘操作及减少动态效果。字体文件不进入 Git，干净 checkout 使用系统 fallback。
- SSH：native Rust bridge 使用系统 OpenSSH、静态 alias、严格 host key 与有限 JSON 请求；`lintel submit` 持久接收后返回 ACK，独立会话 worker 执行，重连只查询原任务。Python controller 是可选 CLI，不是桌面依赖。
- 浏览器：Chromium MV3 / Firefox 独立适配、Native Messaging host、实例冲突/配对/持久操作记录与固定路径安装器。Chromium 清理先隔离、关闭目标及 iframe 宿主、注销 worker，等待完整浏览器重启，再用新确认继续删除。
- 网络：loopback CONNECT / 有限 HTTP 转发、精确域名/端口规则、上游与连接事件；仅证明经过通道的流量。界面明确规则表单是下次启动草案，当前 native status 未返回已生效的完整规则配置。

## 证据与未验证范围

| 范围 | 当前证据 |
| --- | --- |
| Core | 17 项 Rust tests 通过，包括旧计划冲突、归档导入拒绝覆盖、退役、官方注销 fake CLI、共享认证范围变化拒绝及删除窗口内替换新凭据保留 |
| CLI / submission | 实际 CLI 配置往返旅程通过；独立 worker 的 durable ACK、父进程退出后完成、原 ID 查询、去重、口令不落记录通过。PTY 中重建/批准/无回显口令/加密归档通过 |
| Egress | 3 unit + 7 localhost socket tests 通过；没有真实 Claude 公网探针或进程强约束证据 |
| Browser | 17 JS + 7 native host Rust tests 通过。旧静止 SW smoke 曾通过；活跃 SW 负例证实注销后仍可能回写。当前两阶段完整 smoke 被 `browser_restart_required` 拒绝，因为临时测试安装未观察到真实 onStartup；没有伪造启动世代，不能标为完整通过 |
| SSH | 完整 desktop Rust suite 13/13（11 remote + 2 network）与 Python fake-SSH 11/11通过，覆盖严格 host key、固定命令、丢 ACK 查询、stdin 口令不持久化；真实主机未连接，Linux logout/cgroup 与主机重启恢复未验收 |
| Web UI | 合成 root 中计划/执行/恢复、归档解锁/阅读/冲突拒绝/新环境迁入、四清理配方、退役重新启用、支持资料保存已走通。Day/Night、八种彩蛋、减少动态、900×640 四页无横向溢出通过；视觉稿仍属候选 |
| macOS | arm64 App 本地构建成功，约 13.48 MiB；已观察原生 WebKit 新首页和 Clawd。无 Developer ID、公证、正式分发或 Applications 安装验收 |

实际主机为 macOS arm64。Linux、正式 Chrome/Edge/Firefox、真实 Claude 身份与平台认证机制没有实机验收。细项证据在 [acceptance-status.json](acceptance-status.json)；局部测试不自动完成整个验收用例。

## 完整目标仍缺少

1. **真实认证与写入者控制：** Keychain/共享 profile、Desktop/IDE/service 的完整定位与生命周期，supervisor 暂停、重新登录及旧会话续用。当前仅按名称识别部分 Claude 进程；明确要求目标写入者先停，不能归属的进程阻止清理。官方 auth 命令仅有合成 CLI 证据，不能宣称生产注销已验收。
2. **完整外发策略与强约束：** 自定义/更多取舍、入口实效矩阵、macOS Network Extension 签名与权限、Linux namespace；直接 socket、UDP、DNS、NO_PROXY、子进程不能由代理覆盖证明。
3. **远程运行：** 远端安装/更新、supervisor、真实 SSH logout/cgroup 和主机重启恢复。当前 detached 本地证据不能取代远端 runtime 验收。
4. **浏览器旅程：** 首先在真实持久安装的独立测试 profile 完成 onStartup 两阶段清理验收，再验 Chrome、Edge、Firefox。Firefox 独立 CacheStorage、按站点 proxy、容器后台停写、克隆识别和专用 browser 启动仍有缺口。
5. **发行与维护：** 菜单栏、定时漂移、签名规则更新、卸载、升级、正式签名/公证、性能预算。
6. **并发与恢复：** 不合作的外部编辑器或已持有文件描述符的 writer 没有 OS 级 CAS；不确定副作用需核对。混合状态加密备份不支持自动恢复；会话/记忆迁入不证明可以续聊。

这些是原始完整目标的差距，不是缩小后的新 SPEC。下一关口是持久浏览器测试安装、真实认证/入口矩阵与独立 Linux runtime 验收；不在个人登录或生产服务上替代测试。
