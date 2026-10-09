# IPv4 / IPv6 与受控网络通道

`lintel-egress` 是 Rust loopback HTTP proxy。它已经实现 TCP CONNECT、有限普通 HTTP 转发、每个实例固定的环境与 hostname 规则，以及 HTTP/HTTPS 上游。它只约束实际进入此代理的连接。设置 `HTTP_PROXY` / `HTTPS_PROXY`、启动一个代理或者看到一条代理事件，均不能证明目标进程无法直连。

## IPv4 / IPv6 Leaf：宿主与通道分别实测

App 入口是“环境详情 → 外发与权限”，无 Claude 环境时首页可“查看这台主机的网络”。CLI 使用 `network inspect`、`network probe`、`network ipv6 plan` 和 `network restore plan`。这四个操作共用 `crates/core/src/network.rs`；请求 schema 来自 `crates/operations`，有限 HTTPS 请求只由 `crates/egress/src/probe.rs` 实现。完整目标见 [Leaf](specs/ip-network.md)。

`network_inspect` 离线读取所选主机的服务／接口与地址观察；网络 revision 包含配置、接口 flags、地址与路由／VPN 动态值，用于前测有效性；观察不完整明确列为 limitation，不复用该结果。未关联到配置服务的新增接口（含 VPN/TUN）也独立可见。界面在进入、重新取得焦点及可见期间每 30 秒更新 metadata，不会因此访问公网。配置、观察、实际请求与强约束是分别报告的事实。

点“测试实际出口”才产生四项请求：主机默认网络 × IPv4/IPv6、所选 Lintel 通道 × IPv4/IPv6。主机默认路径保留系统 VPN/TUN 的作用；未选择通道时对应两格为 `not_tested`。回显必须是匹配族的有限 IP，结果包含所选目标、执行进程实际观察的主机名、时间、耗时与失败原因，IP 可复制；主机名不可读时不伪造。`peer_family` 只描述实际 TCP 一跳，不证明上游之后的路由。默认 `https://api.ipify.org` 与 `https://api6.ipify.org`，可显式换成自己的 HTTPS 回显端点；端点会观察本次出口 IP。拒绝 credentials/query/fragment、redirect、自定义 CA 与无界响应。每格 1–15 秒完整 deadline（默认 10），头部／正文分别有上限，DNS、连接、CONNECT、TLS、HTTP 与地址族失败不混成安全结论。

macOS 操作准确的当前 SCNetworkSet／service UUID／接口，不按 Wi-Fi 等显示名猜测。预览冻结完整 IPv6 protocol 配置、协议 enabled、服务自身 enabled、服务身份及前测；关闭禁用该协议，仅链路本地选 LinkLocal。批准后只修改该服务 IPv6，保留 IPv4、DNS、系统代理与其他服务，在系统授权下写入、提交／应用、独立读回，再自动沿用冻结端点和路径复测。它是**宿主共享设置**，影响使用服务的所有应用，不随 Claude 环境切换或 App 退出撤销；IPv6-only／VPN 可能失连。有效前测要求同 revision／端点／通道实例／deadline 且不超过五分钟，否则重新前测；批准前若前测过期或网络变化，原计划拒绝新任务，须重新预览。

完整配置恢复另行预览批准，包含手动地址、前缀与 router；后续外部修改、服务／接口变化拒绝覆盖。App 原通道停止后，可在新恢复预览中把该路径明确列为未测试。原任务 ID 持久保存，响应丢失或中断只查询原任务，不重写或重新复测；intent 或相同可读值不能单独证明完成。读回未确认时保留不确定性，不能据此自动恢复；配置已读回而后测中断或未完成时，任务标为 `partially_completed` 并保留已确认的恢复入口；后测取得有限失败结果则如实列出每格原因。后测失败独立显示，不回滚已确认配置，也不变成“零泄漏”。

Linux 的 `network inspect/probe` 在所选 runner 执行，不用 Mac 的通道代替远端。只接受远端 loopback HTTP 通道；`network serve` 仍由独立前台进程持有。Linux 整机 IPv6 变更不支持，不尝试 sudo 或改变管理连接；namespace／systemd 工作负载强约束仍是独立 SPEC 目标。`LINTEL_TEST_HOME` 和开发 fixture 禁止接触真实宿主网络，本 Leaf 的系统配置验收只使用 cfg(test) adapter 与 synthetic invoke。真实 macOS 授权、系统写入／恢复、VPN 切换、公众回显互通与 Linux native runtime 保持未验证。

## 启动和停用

每个环境启动一个独立实例，并将该环境的受控启动入口指向输出的 loopback 地址。配置例子全部为 synthetic 值：

```sh
lintel network serve --config crates/egress/examples/policy.synthetic.json
# Independent existing executable uses the same serve_config owner:
cargo run -p lintel-egress -- --config crates/egress/examples/policy.synthetic.json
```

两条 CLI 共用 `serve_config`，stdout 明确为 NDJSON。首条 `listening` 报告 `owner=foreground_process`、PID、规范化 `active_config` 与实际监听地址，例如 `127.0.0.1:49152`。端口 `0` 由 OS 分配；只接受 IPv4/IPv6 loopback 绑定，不监听公网，也不自动改 shell、浏览器、服务、路由或防火墙。没有配置好的客户端不会自动经过此代理。其他同一台机器上的进程也能够连接这个端口；独立端口是配置作用域，**不是进程身份验证**。

退出 CLI 使用 Ctrl-C。它会先停止接入，再关闭已有 tunnel / HTTP 连接。重新加载规则采用显式停止旧实例、等待停止完成、再启动新实例的流程；已经建立的 CONNECT tunnel 不会绕过新规则继续存活。代理停止或上游失败后，本代理不会尝试绕开指定上游。客户端是否自行回退直连仍需分别验证。

GUI 与 CLI 使用同一 Config/Proxy crate，但 GUI 通道由 App 进程持有；CLI 不读取或停止 App 通道。两者不共享跨进程 lifecycle。

GUI/runner integration 使用同一 crate：

```rust,ignore
let proxy = lintel_egress::Proxy::bind(config, observer).await?;
let address = proxy.local_addr()?;
// shutdown 可以是 oneshot receiver 包装成的 Future<Output = ()>。
proxy.serve_until(shutdown).await?;
// 只有返回后，调用者才能报告该实例及既有连接已停用。
```

`Observer = Arc<dyn Fn(Event) + Send + Sync>` 接收下述固定 metadata schema。回调应迅速返回；GUI 应只保留有上限的内存事件列表。crate 默认不创建日志文件。CLI 将 metadata 写到 stdout，调用者需要持续消费输出；使用 shell 重定向后，日志的容量、保留和删除由该调用者负责。`serve()` 是没有停止信号的常驻形式；需要控制生命周期时应使用 `serve_until` 并等待它结束，不要直接丢弃 future。

## Desktop native bridge

Tauri 注册 `network_request` command，调用形式为 `invoke("network_request", { payload })`。它返回标准 `{ "ok": true, "data": ... }` / `{ "ok": false, "error": { "code", "message" } }` envelope。此处描述 native command 的 source contract；是否已在某个界面暴露需以该界面的实际实现为准。

| `payload.op` | 其他字段 | 行为与返回 |
| --- | --- | --- |
| `start` | `environment_id`；可选 object `config` | 先通过 core `inspect` 核对已登记环境，再建立该环境的独立代理。`config` 使用前述 Config schema；native 强制覆盖环境 ID 和 `127.0.0.1:0` bind，调用者不能选择公网监听。已有实例时返回 `channel_exists`，需要先停止再修改规则。 |
| `status` | `environment_id` | 返回当前任务是否仍运行、地址、实际采用的 `active_config` 与最多 200 条连接 metadata。只保存在该应用进程内，不写入日志文件。任务已结束时 `running` 为 false、`active_config` 为 null，即使尚有旧地址 / 事件，也不显示为运行中。 |
| `stop` | `environment_id` | 关闭监听及已建立连接，等待代理结束后返回 stopped 状态；释放该环境的内存事件。没有实例时返回相同 stopped 状态。不会终止已经启动的 Claude 进程，也不改系统代理。 |
| `launch` | `environment_id` | 必须有运行中的通道。向共享 core 发送 `launch` 与该通道的 `proxy_url`，原样返回 core envelope。该操作属于用户明确选择的启动动作，不能在 `start` / `status` 时自动调用。 |

新 App 的“通过通道打开 Claude”进入独立启动预览：输入项目 cwd，`request` bridge 的 `plan_launch` 冻结配置 root、cwd 和当前通道地址；批准后使用同一 `launch_request`。`NetworkState::dispatch_core` 在预览及首次尝试时核对这份地址确属该环境的 live App 通道，并串行化与 stop 的竞争。已存在的原启动 record 只查询，通道停用不妨碍找回它；仅有 `planned` 预览不能跳过 live 通道核验。旧 `network_request.launch` 为已有调用者保留。

`start` / `status` / `stop` 的 `data` 固定包括：

```json
{
  "running": false,
  "address": null,
  "active_config": null,
  "events": [],
  "coverage": "proxy_connections_only",
  "direct_connections_enforced": false
}
```

运行中 `address` 是 OS 分配的 `127.0.0.1:port` 字符串；`events` 是上述 typed Event 的数组，部分操作附带解释性 `message`。显式停止后地址和 `active_config` 为 null、事件清空。重新启动通道可能分配不同端口，已启动客户端的代理上下文不会自动更新，需要明确重新启动客户端。`launch` 使用 core 的返回 schema，macOS 的正常响应是 `status: "launch_requested"` 与 `message`；这表示 Terminal 接受了启动请求，不是已观察到目标流量。

运行中的 `active_config` 是 native 校验、规范化后交给 `Proxy::bind` 的完整 Config，包含 `environment_id`、`bind`、`default_action`、`allowed`、`blocked`、`upstream`、`address_family` 和三个资源限制。它不是原始请求的回显：规则主机名转为小写、去掉末尾点，IP 使用规范形式；显式规则端口保留。上游 URL 经过校验但字符串不重写。native 始终把环境 ID 设为当前目标，把 `bind` 设为 `127.0.0.1:0`；OS 实际分配的端口使用单独的 `address`，不能把 Config 的端口 0 当作可连接地址。无效配置不会建立通道，也不会留下 `active_config`。

桌面面板分别展示当前生效配置和下次启动草案。重开面板先查询当前通道；停止后可基于已读回的默认动作、阻止/允许规则与端口、上游继续编辑，重新启动时保留已读回的资源限制。草案仅保留在该 webview 的内存中，按环境区分；切换环境清除旧状态展示，旧环境的延迟回复不能替换当前面板。规则文本每行一个主机，可在空格后用逗号列出端口，例如 `example.invalid 443,8443`；省略端口表示全部端口。原通道任务异常结束时必须先清除旧实例，再重新启动。

core 在 macOS 只给新启动上下文设置该 loopback URL 的 `HTTP_PROXY`、`HTTPS_PROXY`、`http_proxy`、`https_proxy`。这不会修改其他环境、全局 shell 或现有进程，也不会消除目标对 `NO_PROXY` 或其他路由机制的处理。macOS Terminal / 实际 Claude 路由没有在 synthetic 测试中运行；不据此认定应用级直接连接被阻止。其他平台若 core 返回 `terminal_required`，则没有启动目标进程。

低体积 native 验证命令：

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml
CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml network::tests
```

Synthetic bridge 测试核对 inspect／启动请求形状，区分 `planned` 与已尝试 record，覆盖冻结地址变化、停止后拒绝新尝试及原请求查询；真实 loopback socket 检查拒绝与停止。它们不调用实际 Claude、Terminal、系统代理切换或公网探针。

## 规则与实际支持范围

配置字段参见 [synthetic 配置](../crates/egress/examples/policy.synthetic.json)。`environment_id` 只接受短的字母数字 / `-` / `_` 标识，不接受路径或账号名。`allowed`、`blocked` 中每项为 `{ "host": "example.invalid", "ports": [443] }`；域名不区分大小写，使用精确 hostname / IP，不支持通配符。IP 按规范形式比较：`::ffff:127.0.0.1` 这类 IPv4-mapped IPv6 归一到其内嵌 IPv4 参与匹配，规则写一侧即可覆盖两种写法；`127.1`、纯十进制整数或 `0x` 十六进制等 inet_aton 式 IP 写法（规则或请求目的地都一样）被直接拒绝，不能借此绕过精确 IP 规则。空 `ports` 代表该主机的所有端口。显式 `blocked` 优先，其次 `allowed`，最后 `default_action`（`allow` 或 `deny`）。`default_action` 在配置 JSON 中是必填字段：省略即配置无效、拒绝启动，不会静默变成全放行。规则在建立每个请求 / tunnel 前检查，域名判断不等于对 TLS 内部 URL 的检查。

产品立场上的缺省动作是 `allow`，没有未经证明的内置 telemetry 黑名单；但这不是配置缺省——如上一节所述，wire 配置必须显式写出 `default_action`。`api.anthropic.com` 是用途混合域名，默认保留；CONNECT 从不把其加密内容分类成 telemetry。若用户明确把任何主机加入 `blocked` 或选择 `deny` allowlist，它会按该显式规则阻止整个主机 / 端口，可能同时影响必要功能。减少产品可选外发应同时依赖对应产品支持的原生配置。

| 能力 | 当前行为 |
| --- | --- |
| CONNECT | 支持 TCP 双向字节转发；不解密 TLS，不安装根证书 |
| 普通 HTTP | HTTP/1.1 absolute-form URL；转给 origin 时改为 origin-form，并核对 Host 与 URL authority |
| HTTP 请求正文 | 支持单一 Content-Length，最多 64 MiB；不保存正文 |
| HTTP keepalive / pipelining | 每个客户端连接只转发一个请求，强制 `Connection: close`；剩余请求不转发 |
| chunked upload / Upgrade / Expect | 明确返回 501；需要这些协议时由客户端通过适当 CONNECT/TLS 通道传输 |
| HTTP response | 原样流式返回；服务端需遵守 `Connection: close`，否则到连接寿命上限关闭 |
| HTTP / HTTPS 上游 | `http://proxy.example.invalid:8080` 或 `https://proxy.example.invalid:8443`；HTTPS 用 rustls 校验服务器证书与 hostname，使用 Mozilla root bundle |
| 上游认证 / 自定义 CA | 当前不支持；URL userinfo 被拒绝，客户端 Proxy-Authorization 不转发给 origin |
| SOCKS / PAC | 不支持，配置 SOCKS URL 返回明确错误；需要单独、已验证的转换适配器 |
| IPv6 | 可绑定 `::1`，可解析 IPv6 authority；不代表进程其他 IPv6 出站受控 |
| UDP / QUIC / DNS / WebRTC / 直接 socket | 不在本代理约束范围；没有自动测试或“零泄漏”结论 |

连接建立（含 DNS 与 HTTPS 上游握手）和请求头有 `connect_timeout_seconds` 上限，默认 10 秒。整个连接默认最多 3600 秒；可显式设置到 86400 秒，长 tunnel 到期会关闭。最大并发默认 64，可设置 1–1024；接入在容量满时受 backpressure。头部最多 32 KiB / 100 fields。没有后台公共网络探针、STUN、analytics、license 请求或在线规则下载。显式 IP 回显及批准后的自动复测遵循上面的有限目标合同。

`address_family` 为 additive Config 字段：缺少时 `system`；`ipv4_only` 仅允许 Lintel 自己向目标或上游建立 IPv4 socket，DNS 返回的 IPv6 不尝试，不进行 IPv6 或上游失败后的直连回退。IPv4-mapped IPv6 归一为 IPv4。没有可用 IPv4 返回连接失败；它不控制上游的后续连接、客户端绕过、DNS 请求地址族或其他进程。事件的 `peer_family` 来自实际 socket，建立前为 null。

## 观察语义与隐私

事件只含 timestamp、环境 ID、目的地主机 / 端口、`http` / `connect` channel、allow / deny / reject、规则来源、完成或失败原因、分类限制与少量 byte counts。没有 URL path / query、Authorization、Cookie、请求 / 响应 headers、正文或原始错误输出。hostname 本身可能敏感；不要未经预览把原始事件当作可公开的支持包。

`coverage` 始终为 `proxy_connections_only`，`direct_connections_enforced` 始终为 `false`。`tunnel_content_unclassified` 表示 CONNECT 内容不检查；HTTPS 加密内容不可分类，其他 tunnel 协议也不作推断。`byte_counts_complete: false` 表示中断或错误期间 counters 不完整，零不代表没有传输。`completed` 表示本代理完成转发，不代表目标业务成功；事件是在连接关闭或中断时产生，仍在运行的 tunnel 可能尚无完成事件。

## 验证和发行限制

```sh
cargo test -p lintel-egress
```

真实 synthetic localhost sockets 验证 CONNECT allow/deny、HTTP 请求与正文、上游 absolute-form HTTP、上游 CONNECT、上游失败无直连回退、pipeline 不穿透、显式停止关闭现有 tunnel、日志不包含 synthetic secrets。另有配置 / framing 拒绝测试。测试无需公网代理、真实 Claude 数据或管理员权限。HTTPS 上游使用维护中的 [tokio-rustls](https://docs.rs/tokio-rustls/latest/tokio_rustls/) 和 [webpki-roots](https://docs.rs/webpki-roots/latest/webpki_roots/)；目前没有真实部署上游的互通验收记录。

macOS 强约束需要签名、entitlement、用户批准与真实 Network Extension runtime；此 crate 不提供这些能力。Linux namespace 强约束尚未实现或验证，见 [Linux 状态](../platform/linux/README.md)。任何平台都不能据此显示“强约束已生效”。完整验收仍须执行 [ACCEPTANCE.md 网络用例](ACCEPTANCE.md#d-网络与真实覆盖)。
