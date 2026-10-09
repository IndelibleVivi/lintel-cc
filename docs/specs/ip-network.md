# IPv4 / IPv6 网络路径 Leaf

状态：源码已接通，合成与 loopback 验收通过；尚未进入安装候选或真实系统 mutation 验收。此 Leaf 延伸 [SPEC](../SPEC.md) 的网络诊断与明确批准边界，不替代完整目标或产品基线。

## 产品契约

在现有「外发与权限」中，用户从所选主机测试实际出口，预览该 Mac 上某个网络服务的 IPv6 变更，批准后自动用同一目标复测，并保留原配置恢复入口。GUI、CLI、远端 runner 使用同一 core 与探测执行器。

| ID | 完整交付要求 | 验收证据 |
| --- | --- | --- |
| N01 | 一张表呈现主机默认网络／当前 Lintel 通道 × IPv4／IPv6 的真实请求结果、回显 IP、耗时、执行主机与时间。通道未提供时明确未测试。默认路径保留系统 VPN／TUN 的作用。 | 有限探测器的真实 synthetic sockets、CLI 与 App 旅程 |
| N02 | 离线读取本机／远端接口、IPv4／IPv6 状态；配置、运行观察与请求结果分开表达。网络变化使原观察过期，新增路径可见。 | 配置／动态状态变更、跨环境迟到回复检查 |
| N03 | macOS 对明确 service identity 关闭 IPv6 或设为链路本地；影响范围为本机共享网络服务，持续到明确恢复。完整原 IPv6 配置与 enabled 状态冻结；批准前后核对身份与相关配置，系统写入后读回。 | synthetic 原生 adapter 往返、身份变化、外部修改、权限失败／中断与原 ID 查询；真实系统写入另行验收 |
| N04 | 恢复原自动／手动／链路本地／关闭配置，拒绝覆盖后来发生的外部修改。变更和恢复沿用持久计划／原任务记录，不重放不确定写入。 | 原配置往返、恢复冲突与重复 execute 查询 |
| N05 | Lintel 创建的目标／上游连接支持跟随系统和严格 IPv4；IPv4 不可用时明确失败，无 IPv6 或绕开上游回退。准确声明该约束只覆盖 Lintel 建立的连接段。 | IPv6-only 拒绝、真实 IPv4 sockets、上游无直连回退与配置 readback |
| N06 | 变更预览包含可见探测目标与自动后测；仍有效的同目标、同路径前测可复用。配置变更成功与探测失败可同时呈现；前后结果可比较与复制。 | 冻结探测、有效前测复用／过期重测、四格结果与恢复旅程 |

## 实现与边界

- `crates/egress` 维护唯一的有限 HTTPS IP 回显探测器与连接地址族约束；不解密业务 TLS，不记录探测响应正文以外的业务内容，不运行后台公网探针。
- `crates/core` 维护宿主机观察、冻结计划、系统配置变更、原任务记录和恢复。macOS 使用 SystemConfiguration 的 set／service／interface identity 与完整 IPv6 protocol 配置，不通过显示名称定位，不修改 IPv4、系统代理、DNS 或别的服务。
- App 原生通道 owner 核对正在运行的通道地址；CLI／远端显式选择目标主机的 loopback 通道。远端 probe 在该 runner 执行。
- 测试目标默认采用 ipify 的 IPv4-only 与 IPv6-only HTTPS 回显端点；用户可显式指定自己的 HTTPS 回显端点。响应必须为有限的 IP 值且与测试地址族一致。HTTP redirect、任意协议、凭据 URL 与无界响应不接受。
- 超时、DNS 失败、无路由、连接拒绝、TLS／HTTP 失败均保留各自原因；失败不会变成“已阻止泄漏”。单次请求不证明所有应用流量、DNS、UDP、QUIC 或 WebRTC 的覆盖。
- 宿主机关闭 IPv6 可能影响 IPv6-only／DNS64／NAT64 网络、VPN 连接和其他应用；明确说明影响后，按完整计划批准。开发不修改真实主机网络或生产 VPS。
- Linux 交付只读观察、显式出口实测及 Lintel 地址族控制。远端整机关闭、systemd／container 出口强约束不纳入本 Leaf 的系统 mutation：它们需要独立管理连接恢复与工作负载后端。原 SPEC 的强约束目标继续保留。

## 交付记录

2026-10-09：N01–N06 已接通 canonical core／egress、App 现有外发页与批准／记录流程、named CLI 和 finite SSH。六个 task catalog ID 保持不变。

- 完整 workspace、standalone desktop Rust、runner／frontend build、network／Agent CLI／adapter／原 settings restore journeys、shared remote transport 通过；独立 `network-ui` 验证 built App 的四格结果、复制、完整 Manual 预览与恢复、取消、丢 ACK 原 ID 查询、通道替换、跨环境共享记录、空清单与 Linux readonly 目标。
- core 使用 synthetic 配置 adapter 验证 off／LinkLocal／Manual 往返、外部修改冲突、授权拒绝、读回不一致、原记录中断与过期前测；macOS CoreFoundation 的实际构造／转换在本机离线检查嵌套配置、bool／integer／float、不可保真值和预算，不打开系统配置会话。
- egress 的 IPv4／IPv6 TLS 与 CONNECT 在真实 loopback sockets 验证证书／主机名、请求 deadline、HTTP framing、IP 地址族、四格路径以及 IPv4-only 无回退。这些是合成端点，不是公网互通证明。

本机 Linux cross-check 因缺少 `x86_64-linux-musl-gcc` 停在 ring build；不把它当作 Linux 通过或网络实现失败。Linux 原生编译／runtime、真实 macOS 系统授权与配置往返、真实 VPN／接口切换、公众回显互通、安装候选与正式发行仍未验证。源码 push 不更新现有 App／CLI 候选，也不激活宿主网络设置。
