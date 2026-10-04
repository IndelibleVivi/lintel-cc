# 受控网络通道

`lintel-egress` 是 Rust loopback HTTP proxy。它已经实现 TCP CONNECT、有限普通 HTTP 转发、每个实例固定的环境与 hostname 规则，以及 HTTP/HTTPS 上游。它只约束实际进入此代理的连接。设置 `HTTP_PROXY` / `HTTPS_PROXY`、启动一个代理或者看到一条代理事件，均不能证明目标进程无法直连。

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

运行中的 `active_config` 是 native 校验、规范化后交给 `Proxy::bind` 的完整 Config，包含 `environment_id`、`bind`、`default_action`、`allowed`、`blocked`、`upstream` 和三个资源限制。它不是原始请求的回显：规则主机名转为小写、去掉末尾点，IP 使用规范形式；显式规则端口保留。上游 URL 经过校验但字符串不重写。native 始终把环境 ID 设为当前目标，把 `bind` 设为 `127.0.0.1:0`；OS 实际分配的端口使用单独的 `address`，不能把 Config 的端口 0 当作可连接地址。无效配置不会建立通道，也不会留下 `active_config`。

桌面面板分别展示当前生效配置和下次启动草案。重开面板先查询当前通道；停止后可基于已读回的默认动作、阻止/允许规则与端口、上游继续编辑，重新启动时保留已读回的资源限制。草案仅保留在该 webview 的内存中，按环境区分；切换环境清除旧状态展示，旧环境的延迟回复不能替换当前面板。规则文本每行一个主机，可在空格后用逗号列出端口，例如 `example.invalid 443,8443`；省略端口表示全部端口。原通道任务异常结束时必须先清除旧实例，再重新启动。

core 在 macOS 只给新启动上下文设置该 loopback URL 的 `HTTP_PROXY`、`HTTPS_PROXY`、`http_proxy`、`https_proxy`。这不会修改其他环境、全局 shell 或现有进程，也不会消除目标对 `NO_PROXY` 或其他路由机制的处理。macOS Terminal / 实际 Claude 路由没有在 synthetic 测试中运行；不据此认定应用级直接连接被阻止。其他平台若 core 返回 `terminal_required`，则没有启动目标进程。

低体积 native 验证命令：

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml
CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml network::tests
```

Synthetic bridge 测试通过 fake core 核对 inspect / launch 请求形状，通过真实 loopback socket 检查拒绝与停止。它们不调用实际 Claude、Terminal、系统代理切换或公网探针。

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

连接建立（含 DNS 与 HTTPS 上游握手）和请求头有 `connect_timeout_seconds` 上限，默认 10 秒。整个连接默认最多 3600 秒；可显式设置到 86400 秒，长 tunnel 到期会关闭。最大并发默认 64，可设置 1–1024；接入在容量满时受 backpressure。头部最多 32 KiB / 100 fields。没有自动公共网络探针、STUN、出口 IP 查询、analytics、license 请求或在线规则下载。

## 观察语义与隐私

事件只含 timestamp、环境 ID、目的地主机 / 端口、`http` / `connect` channel、allow / deny / reject、规则来源、完成或失败原因、分类限制与少量 byte counts。没有 URL path / query、Authorization、Cookie、请求 / 响应 headers、正文或原始错误输出。hostname 本身可能敏感；不要未经预览把原始事件当作可公开的支持包。

`coverage` 始终为 `proxy_connections_only`，`direct_connections_enforced` 始终为 `false`。`tunnel_content_unclassified` 表示 CONNECT 内容不检查；HTTPS 加密内容不可分类，其他 tunnel 协议也不作推断。`byte_counts_complete: false` 表示中断或错误期间 counters 不完整，零不代表没有传输。`completed` 表示本代理完成转发，不代表目标业务成功；事件是在连接关闭或中断时产生，仍在运行的 tunnel 可能尚无完成事件。

## 验证和发行限制

```sh
cargo test -p lintel-egress
```

真实 synthetic localhost sockets 验证 CONNECT allow/deny、HTTP 请求与正文、上游 absolute-form HTTP、上游 CONNECT、上游失败无直连回退、pipeline 不穿透、显式停止关闭现有 tunnel、日志不包含 synthetic secrets。另有配置 / framing 拒绝测试。测试无需公网代理、真实 Claude 数据或管理员权限。HTTPS 上游使用维护中的 [tokio-rustls](https://docs.rs/tokio-rustls/latest/tokio_rustls/) 和 [webpki-roots](https://docs.rs/webpki-roots/latest/webpki_roots/)；目前没有真实部署上游的互通验收记录。

macOS 强约束需要签名、entitlement、用户批准与真实 Network Extension runtime；此 crate 不提供这些能力。Linux namespace 强约束尚未实现或验证，见 [Linux 状态](../platform/linux/README.md)。任何平台都不能据此显示“强约束已生效”。完整验收仍须执行 [ACCEPTANCE.md 网络用例](ACCEPTANCE.md#d-网络与真实覆盖)。
