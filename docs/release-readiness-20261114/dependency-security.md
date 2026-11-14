# Dependency security: release-candidate review

委托日期：2026-11-14。实际查询时间：2026-11-13 23:07–23:24 UTC。初次扫描固定源码 `da7ebd2024384ea2019719f63dabef3d9db6cd32`；其后只刷新下表列出的 TLS、I/O 安全依赖及必要的兼容配对，最终提交与重新构建产物由本目录发行交付总表关联。

## 结论与实际修复

本轮依赖检查发现了一个**实际位于发行运行路径上的 TLS 信任问题**，不能用原有绿灯 CI 代替修复：`webpki-roots 1.0.9` 包含受影响的 emSign 信任根，`rustls-webpki 0.103.15` 也没有拒绝已公开私钥对应的失信中间证书。Lintel 的 HTTPS upstream proxy 连接和主动 TLS 探测使用这组根与验证器。具备相应网络控制能力的攻击者可影响这些连接的服务器认证，用户可能得到不可信的代理连接或探测结果。这不是对所有 Claude HTTPS 流量进行 TLS 解密的断言；普通 CONNECT 隧道中的端到端 TLS 仍由客户端负责。

该问题对应 [CVE-2026-111943 / RUSTSEC-2026-0381](https://rustsec.org/advisories/RUSTSEC-2026-0381.html) 和 [RUSTSEC-2026-0382](https://rustsec.org/advisories/RUSTSEC-2026-0382.html)。生产调用点为 `crates/egress/src/lib.rs::connect_transport`（约 715–740 行，`roots.extend` 和 `.with_root_certificates`），以及 `crates/egress/src/probe.rs::tls_config`（约 423–431 行）。两者确实从 `webpki_roots::TLS_SERVER_ROOTS` 建立根集。

| 依赖 | 原锁定版本 | 修复版本 | 修改范围与目的 |
| --- | --- | --- | --- |
| `webpki-roots` | 1.0.9 | 1.0.10 | root + App lock；移除受影响信任根 |
| `rustls-webpki` | 0.103.15 | 0.103.16 | root + App lock；拒绝受影响中间公钥 |
| `rustls` | 0.23.45 | 0.23.46 | root + App lock；同一 TLS 栈的兼容补丁，修复下述四个已公告问题 |
| `webpki-root-certs` | 1.0.9 | 1.0.10 | 仅 App lock；相同失信根数据集的修复。这里由 platform-verifier 的 wasm32 目标依赖引入，不是额外的 macOS 暴露 |
| `tokio` | 1.53.1 | 1.53.3 | root + App lock；AsyncFd 不安全 FD 所有权 API 的兼容修补 |
| `rustix` | 1.1.5 | 1.1.6 | root + App lock；零长度 xattr 读取缓冲的安全修补 |
| `xattr` | 1.6.1 | 1.7.0 | 仅 App lock；配合 rustix 新 size-query API，防止升级后大 xattr 被静默读空 |

没有修改 Cargo manifest、启用新 feature、更换 TLS provider 或升级其他包。root lock 只有五个包的版本和 checksum 改变（10 行换 10 行）；App lock 只有七个包改变（14 行换 14 行）；独立 browser native-host lock 不变。Cargo 曾为已有宽范围约束重选 syn/windows-sys 依赖边；核对精确版本 registry manifest 后恢复了原选择，避免与补丁无关的依赖行为漂移，保存于 `refresh/resolver-edge-*.json`。工作流和 candidate 必须从包含本次 lockfile 修复的新提交重新运行，先前 `da7ebd…` 的构建结果不能重标为新产物。

## 审查方法和证据范围

扫描了三个已提交 Cargo lockfile，共 **566 个唯一 crates.io 名称／版本组合**：root 232 个 registry entries，App 550，独立 native-host 19。没有其他 git registry 来源需要另行判定。`cargo-audit` 在此环境未安装，未安装新工具；使用公开 OSV `POST /v1/querybatch` 对每个精确版本查询，并保存完整请求、响应、HTTP 时间、SHA-256 和每个匹配 advisory 原文。原扫描 11 个包匹配 25 个 advisory ID；其中有 GHSA/RustSec 别名，**不能把 25 当成 25 个独立漏洞**。

补丁后用相同 566 个精确版本重新查询：剩余 **5 个包匹配 12 个 ID**，上表修补的包不再匹配该查询时点的公告。剩余项的真实使用条件见下节；本报告没有给仓库盖“零漏洞”章。

另外对 App 和 browser extension 各执行了完整及 `--omit=dev` 两种 `npm audit`：四次均成功，报告 0 个已知漏洞。Node 为 v24.22.1，npm 为 11.9.0；命令使用 `--package-lock-only --ignore-scripts --registry=https://registry.npmjs.org`，没有安装、运行生命周期脚本或改写 npm lockfile。这里的结论仅是对应 feed 和 lockfile 的结果。

root Linux 的 `cargo metadata --locked --offline --filter-platform x86_64-unknown-linux-gnu` 成功，用于核对实际 feature 和依赖路径。macOS 离线 metadata 因本地未缓存的 `core-foundation-sys` / `minisign-verify` 不能完成，不记作成功；目标条件改用**精确锁定版本**的 crates.io dependency metadata 及 upstream tag 的 Cargo manifest 核对。App macOS 重新编译由最终 native CI 验证，不能由此 Linux 环境代签。

完整原始材料在交付 evidence 的 `dependency-audit/`：

- `npm-audit-manifest.json`、每次 stdout/stderr：版本、命令、退出码、lock digest、实际查询时间。
- `osv-cargo-query.json` / `response.json` / `manifest.json`，`advisories/`：原锁定图与公告。
- `crate-metadata/`：精确版本的目标依赖条件；误查的相邻版本保留为检索记录，结论只使用 2.12.1 / 0.13.5 等实际锁定版本。
- `pre-refresh/`、`refresh/package-delta.json`、`refresh/commands.json`：补丁前 lock、有限更新命令与实际差异。
- `refresh/osv-*.json` / `validation.json`：首轮 TLS 补丁检查；`final-refresh/*.json` 和 `refresh/final-validation.json`：加入 Tokio/rustix/xattr 兼容修复后的最终查询和测试。
- `upstream-source/`、`xattr-compatibility/`：精确 crate 源码与 checksum、独立临时文件的坏配对复现／正确配对验收。
- `query-cargo-locks.py`：可再次运行的只读精确版本查询程序。

OSV 最初响应服务时间为 2026-11-13 23:07:55 GMT；最终查询时间、HTTP Date 和准确 lock digest 见 `final-refresh/manifest.json`。OSV 查询是公告 feed 匹配，不是二进制可达性分析，也不评估未披露问题；下列判断另外核对了 Lintel 使用方式和目标依赖。

## 剩余公告与排除理由

| 包与公告 | 在 Lintel 中的路径 | 当前发行判断 |
| --- | --- | --- |
| `crypto-bigint 0.5.5`，[RUSTSEC-2026-0387](https://rustsec.org/advisories/RUSTSEC-2026-0387.html) | `age → p256/hpke → elliptic-curve → crypto-bigint`。公告针对秘密整数的非恒时运算。Lintel 归档只构造 `age::Encryptor::with_user_passphrase` 和 `age::scrypt::Identity`（`package.rs` 约 176、353、411、626 行；`archive.rs`、`residue_archive.rs` 同样），未构造 P-256/tag recipient 或 identity | 当前密码归档没有发现进入受影响整数运算的路径；不宣称上游库本身已修复。以后增加 recipient/passkey 等算法时必须重新审查；上游修复版本 >=0.7.6，不能直接把 age 的间接 0.5 依赖强换到不兼容 minor |
| `zip 4.6.1`，RUSTSEC-2026-0367/0368/0369/0429；GHSA-8hxf-qr8p-62x4 / GHSA-m665-v7pm-7grj / GHSA-q7v9-x93c-xqv3 | App lock 的唯一直接引入者是 `tauri-plugin-updater 2.13.2`。其[精确 tag manifest](https://github.com/tauri-apps/plugins-workspace/blob/updater-v2.13.2/plugins/updater/Cargo.toml)把 zip 放在 `cfg(target_os = "windows")`；macOS 解包使用 tar/flate2。独立 CLI 不依赖 zip | 包括 HIGH/CRITICAL 的公告是真实的，但不在本次 macOS App、macOS/Linux CLI 的目标路径。不能为了压平扫描数字给本轮引入 ZIP 4→9/Tauri 大升级；以后支持 Windows App 必须先处理 |
| `glib 0.18.5`，[RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html) / GHSA-wrw7-89jp-8q8g | App 的 GTK/WebKitGTK Linux/BSD 目标依赖。公告为 VariantStrIter 的不正确 mutability/NULL 读取。root CLI 与独立 native-host 无 glib | 不在本次 macOS App 和 Linux CLI 分发。不能据此声称 Linux Native App 已通过发行安全验收；若将来分发，须处理 GTK 链并测原生路径 |
| `proc-macro-error 1.0.4`，[RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html) | GTK/glib procedural macro build 链 | 公告为停止维护，不是本轮确认的运行时漏洞；跟随上游 GTK/Tauri 迁移处理 |
| `basic-toml 0.1.10`，[RUSTSEC-2026-0374](https://rustsec.org/advisories/RUSTSEC-2026-0374.html) | `age → i18n-embed-fl / i18n-embed-impl → i18n-config`；前两者是 `proc-macro = true`，解析构建期 i18n 配置 | 停止维护公告；不是 Lintel 接收用户任意 TOML 的运行入口。跟随 i18n 上游替换，不阻断本次 Preview |

### Tokio / rustix 的调用检查与兼容修补

[Tokio 公告 RUSTSEC-2026-0386](https://rustsec.org/advisories/RUSTSEC-2026-0386.html)针对 `AsyncFd` 的 safe constructor 接受没有稳定所有权的 `AsRawFd`。Lintel 源码没有使用 `AsyncFd`，检查的 socket 路径使用拥有资源的 `tokio::net::TcpStream/TcpListener`；`tokio-rustls` 也没有该构造。1.53.3 保留旧 API 并弃用它，增加明确 unsafe 的注册入口；这不要求 Lintel 改用裸 FD。兼容补丁已经纳入，并重新验证真实 TLS/socket、超时、关闭与 runner 链接。

[rustix 公告 RUSTSEC-2026-0486](https://rustsec.org/advisories/RUSTSEC-2026-0486.html)针对 `getxattr/listxattr` 系列误将零长度缓冲区的查询返回值作为已初始化长度。root 的 rustix 来自 test-only `tempfile`，其 inspected source 不使用这些读取；App 还通过 updater/tar 引入 `xattr`。已验证 checksum 的 tar 0.4.46 在 `entry.rs:879` 只调用 `xattr::set`，默认 `unpack_xattrs = false`，没有调用受影响的 get/list。这使本次修补的实际优先级低于 TLS，但同一兼容补丁可以合理收敛风险。

**不能只更新 rustix 而保留 xattr 1.6.1。** 老 xattr 的 `src/sys/linux_macos.rs:51–59` 用 `&mut []` 调用旧 getter 查询大小；`src/util.rs:23–34` 在 4 KiB buffer 不足后使用该大小重分配。rustix 1.1.6 改为返回空读取并提供独立 `*_query` API，因此老调用者会把大属性静默读成空值。这个问题在独立临时文件上实际复现：8192 bytes 写入后读出 0 bytes，保留完整字节的断言失败。

[精确 xattr 1.7.0 分发](https://static.crates.io/crates/xattr/xattr-1.7.0.crate)将六处查询改成对应 `*_query`，并要求 rustix >=1.1.6；源码其他变化仅移除一个 Clippy allow，未改变公开 API。我们同步锁定这对版本。相同 Linux OS API 程序随后成功保留 0、1、4096、8192 bytes 的属性值，并完整列出超过 4096 bytes 的 80 个属性名。证据包含两个隔离 Cargo manifest/lock、程序、失败与成功输出，所有文件和属性均为临时合成数据，正常结束和断言失败均清理自建目录。

本实验检验了**升级本身是否引入数据丢失**；它不是对真实用户属性、macOS updater 安装或 Gatekeeper 的验收。最终 macOS native/package gate 仍必须使用新 lockfile。

### 同时修补的 rustls 公告

0.23.46 修补 [0346](https://rustsec.org/advisories/RUSTSEC-2026-0346.html)、[0347](https://rustsec.org/advisories/RUSTSEC-2026-0347.html)、[0348](https://rustsec.org/advisories/RUSTSEC-2026-0348.html)、[0349](https://rustsec.org/advisories/RUSTSEC-2026-0349.html)：自定义 session store 的 Debug 泄密、ECH rejection 前已缓存应用数据、Brotli 证书解压内存放大、未认证 ServerHello 对后续混合密钥协商偏好的影响。

Lintel inspected egress 配置没有自定义 session store / ECH；Linux 实际 rustls feature 为 `ring, std, tls12`，没有 Brotli；指定的 ring provider 只列 X25519 / SECP256R1 / SECP384R1，没有公告中的 PQ 混合组。无需把这些不同前提都误报成实际用户故障，但同一兼容 patch 可以消除相应依赖公告，因此一并锁定修复。未增加全面日志或连接行为来“验证”不存在的场景。

## 独立宣传影片工具

`scripts/preview-film` 是独立本地制作工具，不参与 App、CLI 或官网构建。其完整和 production-only npm audit 都返回代理 HTTP 403 / allowlist 错误，**未成功获得 npm 审计结果**。替代 OSV 查询对 318 个精确 npm 版本成功完成，发现：

| 包 | 发现 | 后续处理 |
| --- | --- | --- |
| `sharp 0.34.5` | 3 个 HIGH 公告，涉及其捆绑 libvips/libheif/librsvg；GHSA-f88m-g3jw-g9cj、GHSA-rgj7-g3m4-5g8c、GHSA-wq5f-xc86-pv6w | 下次影片工具维护升级到 >=0.35.5 或上游已修补依赖，测试真实素材与渲染。恶意图片输入下属于制作工作站风险；没有进入当前分发程序 |
| `fast-uri 3.1.8` | 4 个 LOW/MODERATE URI scheme/host/path 解析公告 | 同一 major 修复 >=3.1.9；核对调用者后更新制作工具 lock |
| `zod 4.5.4` | HIGH：过大受限 collection 的验证可耗尽内存，GHSA-mrhx-c7xc-m3w5 | >=4.6.6；核对影片输入是否可由不可信用户提供 |

这些结果保存在 `osv-film-*.json` 和 `advisories/`。它们不是 App 运行时漏洞，也不是整个仓库“全干净”的证明。本轮没有运行影片工具、处理外部媒体或修改独立制作依赖。

## 补丁验证与交接

补丁后 `cargo test --locked -p lintel-egress` 在 Linux 成功：29 个 unit tests、9 个 proxy integration tests、6 个 telemetry tests，合计 **44 passed、0 failed、0 ignored**。包括 pinned certificate 仍要求目标 hostname、真实 HTTPS upstream 开始 TLS 且失败不能降级为明文、CONNECT 失败不直连、真实代理隧道、超时和确定的关闭行为。这些是真实 loopback socket / TLS 输入，不是仅检查依赖版本字符串。

在完整 TLS/Tokio/rustix/xattr 修补后重新执行的 `cargo test --locked -p lintel-egress` 同样为 44 passed；`cargo build --locked -p lintel-runner` 成功，结果记录于 `refresh/final-validation.json`。首轮 TLS-only 验证单独保留，未冒充最终依赖图结果。App macOS 构建、资源许可搜集、签名与 candidate provenance 必须由新最终提交的 native/package gate 确认。这里没有运行公开攻击站点、使用真实账户或激活 updater feed。

可复现的有限更新命令如下；当前 lockfile 已含这些结果，**无需再次做全图更新**：

```sh
cargo update -p webpki-roots --precise 1.0.10
cargo update -p rustls-webpki --precise 0.103.16
cargo update -p rustls --precise 0.23.46
cargo update -p tokio --precise 1.53.3
cargo update -p rustix --precise 1.1.6
# 在 apps/desktop/src-tauri 重复以上五条，并额外：
cargo update -p webpki-root-certs --precise 1.0.10
cargo update -p xattr --precise 1.7.0
cargo test --locked -p lintel-egress
cargo build --locked -p lintel-runner
```

`cargo-audit` 可在具备该工具的维护环境对三个 lockfile分别补跑，但须使用上述目标／调用路径判断解释剩余项，不能把 unsupported target 的大版本迁移当成当前发行的必需工作。
