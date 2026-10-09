# Telemetry 目标目录与受控规则测试

本文件拥有 telemetry 目标目录这一项能力的实现边界：静态目录、App 草案合并、CLI／有限 SSH 的只读目录，以及受控的规则测试。产品立场与既有受控通道（`docs/network.md`）保持不变。这里没有任何在线规则、第三方历史清单导入、通配符扩展或全域名黑名单。

## 目的与事实分离

“哪些主机承载可选 telemetry”“客户端能否真的连上它”“当前是否被显式阻止”“本机原生设置读回值”是四个不同事实：

- **目录（proposed/informational）**：来自官方文档的静态目标清单，含稳定 ID、用途、客户端、版本适用性（按当前文档诚实界定，不虚构最低版本）、官方来源 URL、核对日期、附带影响、精确 host/port，以及是否 `blockable`。
- **configured**：用户把某个 `blockable` 目标选进受阻通道草案并显式启动后，该 host 出现在通道的 `blocked` 规则里。这是草案／配置事实。
- **observed blocked**：真实 loopback 代理对一次实际 CONNECT 请求返回 `explicit_block` 的事件；这是唯一能说“当前这个通道阻止了它”的证据。
- **not observed**：没有连接、通道已停止／被替换、或规则只是 `default_deny` 命中时，不能当作“已阻止该 telemetry 目标”。

原生设置读回值只说明某个冻结设置值已写入并读出；它不是全局强制，也不等同于浏览器或系统级阻止。

## 静态目录来源与内容

唯一来源是 `contracts/telemetry-destinations.json`（`lintel.telemetry-destinations/1`）。它由 `crates/egress/src/telemetry.rs` 在编译期嵌入，App、root CLI 和 runner CLI 共用同一份文件。

官方文档（核对日期 2026-10-09，`https://code.claude.com/docs/en/network-config`）列出两个可选 Datadog 运营 telemetry 入口，它们是当前唯一 `blockable` 的目标：

| ID | host:port | 用途与启用条件 |
| --- | --- | --- |
| `datadog_logs_intake` | `http-intake.logs.us5.datadoghq.com:443` | Claude Code 运营 telemetry（Datadog log intake）。**仅**用于使用 **direct Anthropic API** 的 Claude Code；Bedrock、Google Agent Platform 与 Foundry 部署不经过此入口。 |
| `datadog_browser_intake` | `browser-intake-us5-datadoghq.com:443` | 运营错误回报（Datadog browser intake）。用于 direct Anthropic API 的 Claude Code，另受 Anthropic 服务端 rollout gate 控制；原生 `DISABLE_ERROR_REPORTING` 或 `DISABLE_TELEMETRY` 会将其关闭。 |

`provider` 字段同时标明 telemetry sink（Datadog）与模型 provider（direct Anthropic API）两个不同事实，不把 sink 当成模型 provider。

下列混合／必要主机只作为 **informational**，永不提供一键 telemetry 阻止：

| host:port | 用途 |
| --- | --- |
| `api.anthropic.com:443` | 模型请求、安全预检查、feature flag 与 telemetry 混合 |
| `claude.ai:443`、`claude.com:443`、`platform.claude.com:443` | 认证或混合用途 |
| `downloads.claude.ai:443` | 安装器／更新、版本检查、插件可执行文件 |

旧 Sentry / Statsig 规则是未核验的历史线索，**不是**当前 `blockable` 条目；目录里不存在它们。目录只接受确切 hostname：没有通配符、路径或 scheme。未知 ID、未知字段与通配符一律拒绝。

Provider 与 enablement 条件可能随官方文档变化；修改目录条目前必须重新核对上表来源与核对日期。

## App：合并进已有的受阻通道草案

入口仍是“环境详情 → 外发与权限”的受控通道面板，没有第二个代理或执行器。

`TelemetryPanel` 通过共享目录显示每个目标。勾选一个 `blockable` 目标，只是把它的确切 host 合并进**下次启动**的受阻通道草案的 `blocked` 规则：

- 保留用户已有的 `blocked`／`allowed` 规则、默认动作、上游、地址族与三个资源限制；
- 同 host 的全部端口规则或包含 443 的规则才视为覆盖；只有 80 的规则不覆盖 443；
- 从不选中 informational／混合主机；
- 只改草案。启动仍是显式动作；不会就地修改正在运行的通道。

草案按环境保留在该 webview 内存中，与其它连接详情一样不写入磁盘。待合并的选择与手写草案分别保留；取消选择不删除手写规则。`start` 时合并确切 host/443 并交给原生 bridge；代理是否采用由 `active_config` 读回证明，实际阻止由真实连接事件证明。

远端映射：管理远端环境（SSH alias）时，目录来自远端 runner 的只读目录请求；勾选和受控测试不可用。旧 runner 未提供目录时明确显示失败，不用本地目录冒充远端支持。远端 UI 不得声称它的本地 App 代理在远端生效——本地通道只覆盖经过该本地通道的连接。

### 已观察的隐私设置（读回值）

`TelemetryPanel` 接受可选的 `settings?: Inspection['settings']` 与 `onPrivacySettings?: () => void`（由 `App.tsx` 提供）。有真实的 `inspect` 读回时：

- 单独显示 `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` 的**实际观测值**，并附官方 env-vars 的更新影响说明（非空即生效；同时关闭自动更新、release notes 与 feature flag 获取）；
- 细粒度开关（产品指标、错误回报、质量调查、主动反馈、跨工具 telemetry、feature flag）逐项分列；
- 没有读回时显示 `not_read`，**不**显示为已关闭或已强制。读回值 ≠ 全局强制，也 ≠ 当前运行证明。

`onPrivacySettings` 只是“查看现有隐私设置”的入口，不会自动改写设置或正在运行的代理。

## CLI 与有限 SSH

只读静态目录，不初始化私有 state、不访问真实宿主网络：

```sh
lintel telemetry catalog          # 与 lintel network catalog 相同
lintel network catalog
```

三条入口（root CLI、runner CLI、`lintel request` 的 `telemetry_catalog` 操作）都返回同一份 `lintel.telemetry-destinations/1`。有限 SSH 把 `telemetry_catalog` 当作只读 request 命令：它不创建任务记录、不重新提交、不需要 runner 绑定字段，也不接受除 `command` 之外任何字段。

### 受控规则测试：`--test-telemetry`

```sh
lintel network serve --config policy.synthetic.json --test-telemetry datadog_logs_intake
```

`--test-telemetry ID[,ID...]` 是有限、去重、已知目录 ID 的列表（最多 8 个）。启动前，每个 ID 解析为确切 host，并只在**已经显式阻止**该 host 时放行：

- 任一目标不是 `explicit_block`（例如只有 `default_deny` 命中）→ 整个启动被拒绝，`telemetry_target_not_explicitly_blocked:<host>`；不会静默成功。
- 未知或通配 ID → `invalid_telemetry_test_ids` / `unknown_telemetry_destination`。
- 拒绝发生在**绑定监听之前**：非法的 `--test-telemetry` 不会打开任何 listener，也不连接目标。
- 通过后，前台 server 先在 `listening` 事件之后对外服务，再对**自己的**监听地址发起一次真实 loopback CONNECT（不是伪造事件）；只有真实 403 与完成的原事件一致（`provenance:explicit_block`、`outcome:blocked`）时，才输出成功的 `rule_test` 结果，字段 `owner_origin: lintel_egress_foreground`、`outcome.result: blocked_explicit`、`outcome.connection_attempted: false`。**从不连接目标**，也不发送任何 telemetry 正文。

受控测试由该 Proxy 自己的 `RuleTestRegistry` 约束：注册绑定**私有 loopback 客户端本地端点 + 确切 host/443 + 一次性 test ID**，只在该 Proxy 实例内、按实例累计有界（最多 16 次、并发 ≤ 2、单次 5 秒 deadline）。接受到的连接必须匹配该私有 peer 与确切目标才会被标记为受控来源；一次性消费；取消/错误只清理自己的注册。任意客户端 header 或 UI 时间推断都不能制造这个来源。

### 原生受控测试（App）

App 通过原生 `network_request` 的 `rule_test` op 在同一活动通道上做同样的受控证明：

```json
{"op":"rule_test","environment_id":"<ID>","telemetry_id":"datadog_logs_intake","channel_binding":"<CURRENT_CHANNEL_BINDING>"}
```

- 只接受目录 ID（`telemetry_id`），不接受任意 host／port／通配；未知 ID 返回 `rule_test_failed`。
- 无活动通道、已停止或原 `channel_binding` 不匹配→`channel_missing` / `channel_stopped` / `channel_changed`，不冒充成功。
- 走的是这个**正在服务的**通道自身的 `Proxy::rule_test`：它先做 `explicit_block` 准入（否则不发起任何网络活动），再绑定私有 loopback 客户端、注册定向 test ID、向该通道自己的监听地址发送真实 `CONNECT host:443`。返回携带 `owner_origin: lintel_app_proxy` 与 `outcome.test_id` 作为 owner-origin 标记；普通 Claude／代理流量永远不带该标记（对应事件 `origin`/`test_id` 为 null）。
- 成功必须同时收到 403，并匹配已完成事件的原 test ID、目标、`decision:deny`、`provenance:explicit_block` 与 `outcome:blocked`。结果为 `blocked_explicit` 或 `failed`。若一次被误放行，服务端会把受控请求**失败关闭**（拒绝且不连接），报 `failed`，绝不伪造成 `blocked_explicit`。从不解析公网 DNS、不连接目标、不经过上游。
- 通道停止或被替换后，旧实例的 `rule_test` 不再可验证；UI 会清掉旧实例的结果并要求对新实例重测。

CLI 与 App 共用同一份目录与同一种受控证明（`serve_config_with_tests` 与 App 的原生 `rule_test` op 都经由该 Proxy 实例的 `RuleTestRegistry` 发起真实 loopback CONNECT）。

## 观察语义与区分

真实客户端连接事件与规则测试严格区分：

- 客户端事件：`provenance: explicit_block`（来自 `Config::decide` 的显式阻止）、`channel: connect`／`http`、`outcome: blocked`。这是 observed blocked。
- `default_deny` 命中的拒绝：`provenance: default_deny`，**不是**显式阻止证据。
- 规则测试：`kind: rule_test`、`provenance: rule_test`、`connection_attempted: false`。它证明的是“冻结草案把该确切目标列为显式阻止”，而不是客户端真的尝试过。

`coverage` 始终为 `proxy_connections_only`，`direct_connections_enforced` 始终为 `false`。目录与规则测试都不构成系统级、浏览器级或桌面级强约束，也不覆盖其它进程的直连。

## 与策略评估的关系

`crates/core/src/policy.rs` 的 `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` 仍是 **nonempty** 语义（`0`／`false` 也会关闭）。按官方 `https://code.claude.com/docs/en/env-vars`，它还同时关闭自动更新、release notes 与 feature flag 获取；该附带影响现在记录在每个相关 blocker 的 `collateral` 字段中。这不改变已批准的 settings 语义，也不改变 Remote Control 保证。`reduce` 预设继续使用细粒度的 `FLAGS`，不会因这条目改变既有做法。

## 验证与限制

```sh
python3 tests/verify.py --checks cargo-workspace-test,desktop-rust-test,journey-agent-adapters
# frontend 先构建，再做独立的合成 invoke UI 旅程
python3 tests/verify.py --checks desktop-build,telemetry-ui
```

受控规则测试使用**真实既有 loopback 代理**：一个显式阻止的目录目标必须被拒绝（403）且不发生任何目标连接；一个不在阻止列表里的 loopback origin 仍可连接（证明块是精确的）。测试从不解析或连接公网目标，也不发送 telemetry 正文。当前沙箱可能缺少 loopback 权限：这类测试会以精确的 `PermissionDenied`／`listen_failed` 报告环境失败，而不是伪装通过。

真实 host 系统设置、真实 macOS 授权、VPN 切换、Linux 强约束与公众回显互通仍需各自独立验收；目录内容只随官方文档变化重新核对。
