# 审查基线、外部事实与证据可信范围

本报告属于 2026-11-14 委托；执行记录使用 UTC，部分查询发生于 2026-11-13。日期标签不替代 Git SHA、workflow run、包摘要或实际观察时间。总决策与本轮最终结果见同目录交接文件。

## 1. 实际审查的产品

起点为 main `73f34ed06b5d630d2a36910baf88ff5713aede66`，已包含 PR #7；不是公开 0.1.1 App 的源码。本轮同时检查了完整目标 SPEC、当前实施、历史验证、现有包和线上网站，沿源码数据流复核，未将设计描述视作完成证明。

| 对象 | 已核对事实 | 对本轮的意义 |
| --- | --- | --- |
| 当前 main | 上述 SHA，PR #4–#7 均已合并 | 作为缺陷复现与改动基线 |
| PR #4 | [认证来源与复查](https://github.com/IndelibleVivi/lintel-cc/pull/4)；source `9a7ae90eaad76c52e500f557b42da4e91aac607c`，merge `57b23c41a59d81952f676b24c2695c09cf1e880a` | 建立精确来源、一次批准、四层复查；独立执行器身份冻结修复已包含 |
| PR #5 | [Keychain 接续](https://github.com/IndelibleVivi/lintel-cc/pull/5)；source `07d3322ee937b6a43307c79c55a7b90fb4bef4d0`，merge `0422cba5f9d5ef76e64d6819be20848bcb045c26` | 原 item data-only 修改与原生临时 Keychain API 测试，未证明每个未来 Claude 版本 |
| PR #6 | [实际使用收尾](https://github.com/IndelibleVivi/lintel-cc/pull/6)；source `a563cd7901af3630a610067ed2043bf5de99430e`，merge `cad1a90e74849014f7eb7e1b5a075811de7b0938` | 登记错误可恢复、归档专属结果准确；已有受控 WebKit 旅程，但部分后续画面未观察 |
| PR #7 | [App 版式与双语文档](https://github.com/IndelibleVivi/lintel-cc/pull/7)；source `41276561743c86a535c576548f970e26115995d0`，merge `d3ba15ecd75f2e5a5275c9a926f71ea368d16fbd` | 改善完整标签、页签、操作栏和清理入口；其源码、合并与 CI checkout tree 一致 |
| 公开 App | [v0.1.1](https://github.com/IndelibleVivi/lintel-cc/releases/tag/v0.1.1)，Preview，2026-10-20；source `ebbe237cd236a2b299281e070b4758a9e9e522b2` | 不包含后续认证、Keychain 与本轮修复 |
| 公开 CLI | [v0.1.0](https://github.com/IndelibleVivi/lintel-cc/releases/tag/v0.1.0)，2026-10-19；source `fcae6cb1efb50c6384a31ddce161bee6a39646e0` | 三平台旧包用于本轮兼容实验，不把 App/CLI 版本混为一谈 |

实际读取了 PR 元数据、改动及讨论／验证记录。未发现尚未解决的实质人工 review thread；自动摘要或赞同不能替代独立复核。PR #7 没有改 Core，不能由其界面通过推导后来发行包、Keychain 用户授权或当前所有认证版本通过。

### 公开资产

0.1.1 DMG `Lintel_0.1.1_aarch64.dmg` 为 13,123,475 bytes，SHA-256 `867ea626ceb5101006e15c071da9627db38a555212208a4fffd577a5f5faf6d3`。同一 release 有双语安装说明和 SHA256SUMS。认证 GitHub asset 元数据、校验文件与页面声明一致。

0.1.0 CLI 三包 SHA-256 已分别固定于 [发行工程报告](release-engineering.md)。本轮实际下载的 Linux x86_64 旧包为 9,286,789 bytes；新严格检查器确认 22 个声明文件、执行位、内部校验和及许可材料有效。这只说明该实际旧包通过所述检查，不把新发现的包装漏洞追认为旧公开包已经损坏。

## 2. 线上网站与生成源码

本轮普通浏览器实际打开了 [中文首页](https://lintel.page/)、[中文下载](https://lintel.page/download/)、[英文下载](https://lintel.page/en/download/)、[英文指南](https://lintel.page/en/guide/)及[英文发行记录](https://lintel.page/en/releases/)。页面正常呈现，双语下载均指向 0.1.1 DMG，并显示上述大小、摘要和旧源码。早先非浏览器请求遭 Cloudflare 拒绝，不构成网站宕机证据。

已部署站点仍是先前 40 页结构；本轮源码为 16 篇成对指南、42 个 locale URL。线上指南缺少新认证清理入口，发行原始说明仍按旧布局展开；新源代码中的认证说明和折叠发行说明尚未部署。页面中的 CLI 入口主要引导源码构建，没有直接呈现现有三个 CLI 二进制下载选项。这是下次发布应补齐的产品入口问题，不能靠“仓库里已有 tar.gz”代替普通用户可发现性。

本轮源码已补齐三个精确公共 CLI 0.1.0 下载入口（含架构、旧版本身份和现有摘要），顺序为 App 下载、CLI、源码构建；两语言均通过回归。它没有把尚未公开的 0.2.0 显示成可下载。

站点源码严格区分示意演示与真实能力，不调用 native/Core；中英文页面可无 JavaScript 阅读，下载元数据只接受有限、已验证的发行记录。维护者应保留这些选择。下一次公开发布需先将实际授权资产读回核对，再生成下载记录与站点 payload，最后独立验收部署；本次没有部署。

## 3. 既有 CI 究竟证明了什么

### PR #7 的完整成功不是一句 badge

独立读取 [run 43461405010](https://github.com/IndelibleVivi/lintel-cc/actions/runs/43461405010) 最新 attempt 的全部 14 个 jobs，下载 13 个 artifacts（含 plan），核对 GitHub artifact 摘要，并读取 12 份 canonical reports。报告绑定 clean checkout `ec634d4c733d0e30d6056476b39e0b5668c1698f`，与 PR source／merge tree 一致。120 次按平台计数的选中检查全部 passed；本轮再次执行生产 `ci_plan.py --aggregate`，没有问题。

| 所选执行组 | macOS arm64 | Ubuntu x86_64 |
| --- | --- | --- |
| Frontend | 15/15 | 14/14 |
| Native/Core/CLI | 19/19 | 18/18 |
| Browser/UI | 19/19 | 19/19 |
| Heavy work | 6/6 | 6/6 |
| CLI candidate | 1/1 | 1/1 |
| 独立 OpenSSH | — | 1/1 |
| 独立 Linux VM | — | 1/1 |

这里按平台计数，不把同一用例执行两次称为两项不同能力。CLI candidate 内部 `real_input_smoke` 因未供给正式输入而 skipped；父检查成功并不让这个内部路径变成最终包验收。真正原生 macOS 临时 Keychain、真实 Chromium API、loopback OpenSSH、VM 的 systemd/PAM/logout/reboot 都有独立入口，可信范围明显强于纯 mock。

main 后续 [完整 run 43468273189](https://github.com/IndelibleVivi/lintel-cc/actions/runs/43468273189) 则在 Ubuntu 系统包阶段失败；原因是固定重写 Azure apt mirror 与新的 hosted image 不匹配，不是 Core 测试失败。本轮移除该镜像重写，使用官方 image 的仓库配置、有限重试与超时，保留 update 失败立即停止的行为及回归。后续文档专用 [run 43468440296](https://github.com/IndelibleVivi/lintel-cc/actions/runs/43468440296) 只有 plan/control/aggregate，不是最新 main 的另一份完整跨平台成功。

### 哪些证据共享了假设

- 多个 auth fixture 曾将显式 `CLAUDE_CONFIG_DIR` 当作默认行为；产品和 inert status stub 可以一起错。本轮修正默认/显式命名空间，也修正 fixture，使未设置变量时使用合成 HOME 的默认根。
- release gate 曾只比较自洽的 manifest/receipt/文件 bytes；无执行位或坏 SHA256SUMS 仍可自洽。新回归在其他外层证据保持一致时单独破坏这些消费者条件。
- Linux writer unit 路径跳过生产进程枚举。本轮 fresh-process journey 以惰性 `claude` 进程实测 ps/proc 与物理目录别名，不再仅依赖独立谓词。
- built-App Chromium 可证明真实组件、Core 和键盘交互，但 invoke/SSH transport 和部分 Keychain/status 为显式 synthetic。它不证明 WebKit、系统授权提示或当前用户登录。
- 一个截图文件名曾声称英文 Day/1120，实际继承先前 Night/900 状态。本轮截图前设置并断言 theme/viewport，不能由旧文件名推断视觉验收。
- 原 late-response 测试的请求数量和固定延时不能证明迟到响应真正返回。本轮增加响应完成观察，再检查两帧后的目标状态。

测试体系本身有价值：严格 canonical selection 拒绝所选 skip，生产 aggregate 重新计算 plan，状态与任务恢复有实际故障注入，archive 的 exact-byte 检查保留独立 oracle。应继续按真实边界增补用例，避免另起一套与生产行为脱节的大框架。

## 4. 本轮外部机制核对

官方 [Authentication](https://code.claude.com/docs/en/authentication) 仍明确 `setup-token` 生成长期 OAuth token，并输出给用户自行配置；移除某个凭据文件不能清掉已继承的 token。认证选择还受云 provider、bearer/API key、helper、profile/federation、gateway 及交互式采用状态影响，不能从库存顺序直接宣称实际优先级。官方 [安装及登录排错](https://code.claude.com/docs/en/troubleshoot-install) 明确 macOS Keychain 失败可回退明文文件，以及 `/logout` 会清除 MCP 登录和 plugin 敏感值；当前拒绝宽范围官方 logout 的决定有实际依据。

本轮对官方 npm 分发的 `@anthropic-ai/claude-code-darwin-arm64@2.1.325` 进行完整性校验及静态检查。新独立安全存储目录变量、默认/显式 namespace 与 account 规则有具体分发依据；没有运行该客户端登录，也没有把内部结构当作永久合同。包、binary 摘要与定位证据保存在 [认证报告](auth-security.md)。报告只保留必要结构和摘要，研究包与第三方二进制不进入本项目发行资产。

Apple 的 [Keychain 更新说明](https://developer.apple.com/documentation/security/updating-and-deleting-keychain-items) 支持以精确 query 更新指定字段；[SecItemUpdate](https://developer.apple.com/documentation/security/secitemupdate(_:_:)) 会更新匹配项且是阻塞 API。Lintel 的 private persistent reference、data-only 修改、邻接字段与 identity 读回是必要保护；这些 API 不是与任意第三方写入者之间的 payload compare-and-swap。用户仍需停止会写目标的上下文，冲突/unknown 不能被包装为已退出所有账号。

Apple 对 [短版本号](https://developer.apple.com/documentation/bundleresources/information-property-list/cfbundleshortversionstring)和 [build 版本号](https://developer.apple.com/documentation/bundleresources/information-property-list/cfbundleversion) 的当前定义为数字组件。固定 Tauri CLI 2.12.1 实现直接投影版本；本轮统一使用数字 `0.2.0`，Preview 放在渠道和发布标签，避免 `0.2.0-preview.1` 被写进 plist。

Apple 的 [安全打开 App 说明](https://support.apple.com/en-us/102445) 与 [Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/) 区分 Developer ID、公证和用户例外。完整 ad-hoc 签名解决 bundle 完整性，不建立 Developer ID 身份或首装信任。SHA-256 则只核对选定 bytes；同源校验文件不产生独立发布者认证。

## 5. 许可与资产决策

仓库 [LICENSING.md](../../LICENSING.md) 为原创功能代码和原创文档分别保留 SUL 1.0 与 CC BY-NC-SA 4.0，前向生效，不能称为 OSI 开源许可证，也不重写早期公开包的授权。第三方 notice 与原项目 terms 分开封装；Rust/Zig runtime 与锁定依赖的 notice 清单不等于精确链接 SBOM。

当前 Clawd/Anthropic 相关图形与名称被明确排除在原创授权之外。仓库中未找到独立再分发授权证据；注明归属或自行描绘相似图形都不能作为许可凭据。当前 [Anthropic 法律中心](https://www.anthropic.com/legal)所链接商标指南也将品牌资产使用与允许的文字指称分开。这是扩大 App/网站分发前需要所有者解决的具体材料缺口；本轮不自行作出授权已存在的结论，也不擅自改品牌。仅含功能代码和许可说明的 CLI 应按实际包内容单独判断。

## 6. SPEC 与本次发布门槛

SPEC 的完整目标保留，包括更广的系统外发约束、长期环境维护和更方便的浏览器分发。当前 Preview 的门槛是：已经承诺的明确任务准确执行、给出真实范围、保存原任务、失败后可找到并继续，下载包可运行且说明符合实际。未实现系统级全流量约束、浏览器商店分发或自动 updater 不自动阻断一个诚实定界的 CLI Preview；它们会限制“面向所有普通用户的完整 Stable”这一定位。

应保留当前最有价值的设计：对原任务的责任、精确授权、保全优先、原生浏览器 API、无全局清扫、没有“本地清理等于服务端解绑”的假结论。真正阻断项是确认的数据/执行错误或实际分发不能兑现使用条件，不能用未来功能清单无限推迟当前版本。
