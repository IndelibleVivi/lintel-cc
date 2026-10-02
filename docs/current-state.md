# 当前状态

2026-10-03 · 0.1.0 开发候选。本地源码与 macOS App 已构建，未安装到 Applications、未发布、未部署远端。完整 [SPEC](SPEC.md) 仍未交付，四条完整旅程 G01–G04 尚未通过。

## 已接通

- core / runner：环境登记与建立、四项外发设置的计划/批准/写入/读回、持久任务、幂等查询、字段恢复、漂移与脱敏支持资料；口令加密归档及新环境资料迁入。
- 桌面：上述 core 路径已接入，Claude 式首页与可收起侧栏、每环境草案、详情/计划/结果/恢复/深浅主题；BrowserPanel 配对与提交、NetworkPanel 通道启停/观察/明确启动入口。
- 浏览器：Chromium MV3 和 Firefox 独立适配、Native Messaging host、配对/实例冲突/持久操作记录；各自限制见 [浏览器](browser.md)。
- 网络：真实 CONNECT / 有限 HTTP 转发、确切 host/port 规则、HTTP(S) 上游、typed events；覆盖始终仅受控通道，不是进程直接出站强约束。
- SSH：系统 OpenSSH 固定远端命令、JSON stdin、严格 host key、持久提交意图与原任务核对；尚未接入桌面，不保证脱离 SSH 存活。

## 已取得的证据

| 范围 | 证据及限制 |
| --- | --- |
| Core | 11 项 Rust tests 通过；包含字段冲突、stale plan、symlink、重复 execute、中断/活跃 journal 查询、归档解密与迁入检查 |
| CLI | `tests/cli_journey.py` 通过，跨真实 CLI 进程走登记 → 检查 → 预览 → 批准 → 读回 → 重放查询 → 恢复；只用临时合成文件 |
| Egress | 3 项 unit + 7 项实际 localhost socket tests 通过；没有访问真实 Claude / 公共网络探针 |
| Desktop bridge | 2 项 native tests 通过，核对通道真实监听、拒绝、事件、停止与 launch payload；实际 Claude 未运行 |
| Browser | 10 项 JS + 5 项 host Rust tests 通过；实际 Chromium 先前通过目标五类存储删除、邻域保留与重启去重，但完整 harness 末段未通过；main_frame 隔离与正式三浏览器仍未验收 |
| SSH | 10 项 fake-SSH/subprocess tests 通过；包括 passphrase 仅经 stdin、丢 ACK 后查询不重发；没有真实主机连接 |
| Web UI | 明确合成模式下实际通过计划/执行/恢复、重建口令门槛与部分完成展示；已检视亮/暗主题、小窗口、键盘 tabs。最新重排首页通过选择/详情、键盘 tabs、计划预览、深浅主题、四页 900×640 无横向溢出及侧栏收起；无 page errors，视觉认可仍待用户评价 |
| macOS App | `npm run desktop:build` 成功，arm64 `.app` 约 12.08 MiB；已观察原生窗口正常渲染；仅本地 ad-hoc linker signature，无 Developer ID/Team ID、公证或正式分发验收 |

本轮主机 macOS 26.5.2 arm64；Rust 1.98.0、Node 26.7.0。未取得 Linux、Edge、Firefox 实机证据。macOS 配置最低 12.0 是构建声明，不是全版本兼容性结论。细项证据在 [acceptance-status.json](acceptance-status.json)，局部 synthetic test 不等于对应完整用例通过。

## 尚未完成的产品范围

1. **完整清场与继续使用：** Keychain/credential fallback/共享认证定位、真实 logout、客户端/Desktop/IDE 状态、写入者与 supervisor 暂停、原地清理、有效重新登录/旧会话续用。当前 rebuild 留存旧状态，因此结果明确为部分完成。
2. **网络强约束：** macOS Network Extension 签名/entitlement/runtime 和 Linux namespace 尚未实现或验证；NO_PROXY、UDP、DNS、子进程、直接 socket 不在当前代理证明范围。
3. **远程完整旅程：** GUI SSH、远端安装/更新、detached job supervisor、跨主机重启/退出后的任务恢复与真实 Linux 运行。
4. **浏览器完整旅程：** 正式浏览器和容器实测、Firefox CacheStorage/按站点 proxy、完整 iframe 写入者控制、离线克隆识别、非开发者 native host 安装与签名分发、专用 browser 启动。
5. **发行与运行维护：** 菜单栏、定时漂移、规则签名更新、卸载、正式签名/公证与升级、完整性能预算。
6. **并发与恢复：** 外部编辑器写入尚无原子 CAS；不确定副作用需人工核对，archive 导入/资料续用尚未成为完整恢复旅程。

这些是完整目标的差距，不是缩小后的新 SPEC。下一集成关口应按真实入口/认证机制建立测试矩阵，先补清场与远程任务生命周期，再取得各平台正式 runtime 证据；不在个人登录或生产服务上用开发测试替代验收。
