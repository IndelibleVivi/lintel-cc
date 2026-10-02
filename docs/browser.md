# 浏览器扩展与本地桥接

状态：0.1.0 开发候选。Chrome/Edge 的 MV3 包和 Firefox 独立 event-page 适配可构建；正式浏览器安装、商店发布和完整三浏览器验收尚未完成。本文对应 [完整目标](SPEC.md#9-模块-c浏览器能力必须真实可操作) 与 [B01–B15 验收](ACCEPTANCE.md#c-浏览器实测)，不是完成整版目标的声明。

## 使用入口与实际范围

扩展 popup 可以独立预览并确认站点清理、WebRTC、Chromium 站点权限、站点阻断规则及 Chromium 站点代理。桌面应用通过同一 native host 提交操作；用户还需在目标扩展内查看范围并确认。网页没有消息入口，扩展没有 content script、debugger、远程代码或默认全站 host 权限。

默认站点是 `https://claude.ai`；`https://console.anthropic.com` 需要勾选。没有邮箱、Google、Apple 或其他 SSO 站点。此版本站点 allowlist 是编译内置的，不接受任意网站清理输入。

| 能力 | Chromium / Chrome / Edge adapter | Firefox adapter |
| --- | --- | --- |
| Cookie | origins 输入，实际扩大到完整可注册域，预览列明 | hostnames 输入；可选 cookieStoreId |
| localStorage、IndexedDB | 精确 origins | 精确 hostnames（包括该主机各 origin）；可选 cookieStoreId |
| Service Worker | 原生注销，先于其他存储删除 | hostnames 注销；不能限定 cookieStoreId |
| CacheStorage | 原生 `browsingData` 类别 | 独立按站点删除接口未交付，明确拒绝 |
| HTTP cache | 可随 origins 清理 | 默认拒绝站点级请求；独立“整个当前 profile HTTP cache”动作需要新预览与确认 |
| WebRTC | profile 级原生 BrowserSetting，检查控制权并读回 | 独立调用 Firefox 同名 BrowserSetting；实际浏览器验收未运行 |
| 定位、摄像头、麦克风、通知 | 所选站点写入 `block` 并读回；不默认全关 | contentSettings API 不存在，明确提示使用浏览器权限面板 |
| 本机代理 | 仅批准站点的 PAC，固定 `127.0.0.1` 用户端口；无 DIRECT fallback | 按站点 proxy 适配未交付；不会改成全 profile 代理 |
| DNR | 明确阻止选定站点的全部请求；独立可选权限 | 独立 manifest，使用原生 DNR；实际浏览器验收未运行 |

WebRTC 不是设备级无泄漏证明。站点代理不代表 DNS、WebRTC、QUIC、其他程序或全部浏览器后台流量均受约束。DNR 站点阻断会影响该站点全部功能，不称为遥测精确分类。无公共 IP、ASN、STUN 或 DNS 诊断请求。

清理前安装本扩展的持久 DNR 隔离规则，明确包含 `main_frame`；关闭所选范围的标签，注销所选 Service Worker，再清理其他类别。Chromium Cookie 扩大到可注册域时，隔离和标签范围同时扩大。容器清理只删除所选 store 数据、关闭该 store 的标签，但 DNR 暂时隔离整个 profile 对目标主机的请求；预览明确披露。容器清理不支持按 store 注销 Service Worker，因此结果标记后台回写未验证。

隔离在清理后保留，用户在回执中单独点击“已核对结果，解除站点隔离”，之后可以自己重访站点。不会自动重访并把新 Cookie 误报成旧数据残留。当前还没有跨无关顶层页枚举目标 iframe 写入者的能力；不能宣称完全冻结所有可能的本地写入。

浏览器 callback/Promise 成功记为 `browser-acknowledged`。没有通用存储枚举能力时不显示“零残留”。可选 `cookies` 权限已授予时仅返回剩余数量，Cookie 值不离开扩展执行上下文，也不进入 journal。正式包默认不申请该权限，合成测试包单独预授权。

## 构建与开发安装

仓库根目录：

```sh
node extensions/browser/scripts/build.mjs
cargo build --release --manifest-path extensions/browser/native-host/Cargo.toml
node extensions/browser/scripts/package.mjs
```

扩展构建和打包无需 npm 运行时依赖；打包使用系统 `zip`。输出：

- `extensions/browser/dist/chromium/`：Chrome/Edge Load unpacked 开发目录。
- `extensions/browser/dist/firefox/`：Firefox 独立 MV3 manifest 与模块 event page。
- `extensions/browser/artifacts/lintel-chromium-0.1.0-dev.zip` 与 Firefox 对应 ZIP：开发分发归档，不是已签名/商店发布的扩展。
- `extensions/browser/native-host/target/release/lintel-browser-host`：native host 独立二进制。

Chrome/Edge：打开其扩展管理页，启用开发者模式，Load unpacked 上面的 Chromium 目录，记录该浏览器显示的扩展 ID。不同安装目录可能得到不同 ID。

Firefox：`about:debugging` → This Firefox → Load Temporary Add-on → 选择 Firefox 目录中的 `manifest.json`。开发临时安装会在浏览器退出后移除。正式 Firefox 长期安装需要 Mozilla 签名；本仓库未完成该发布步骤。扩展 ID 为 `lintel@lintel.local`。不要把临时安装称为非开发者正式交付。

### Native host manifest 与注册

以下命令仅生成 manifest 到 stdout，不注册、不修改任何个人浏览器。用真实、已审查的绝对 host 二进制路径替换占位符：

```sh
lintel-browser-host manifest chromium EXTENSION_ID /absolute/path/to/lintel-browser-host
lintel-browser-host manifest firefox lintel@lintel.local /absolute/path/to/lintel-browser-host
```

审查后，用户显式安装生成的 JSON，文件名 `app.lintel.browser.json`。浏览器规定的当前用户目录如下（`~` 指实际用户 home）：

| 系统 | Chrome | Edge | Firefox |
| --- | --- | --- | --- |
| macOS | `~/Library/Application Support/Google/Chrome/NativeMessagingHosts/` | `~/Library/Application Support/Microsoft Edge/NativeMessagingHosts/` | `~/Library/Application Support/Mozilla/NativeMessagingHosts/` |
| Linux | `~/.config/google-chrome/NativeMessagingHosts/` | `~/.config/microsoft-edge/NativeMessagingHosts/` | `~/.mozilla/native-messaging-hosts/` |

Chrome for Testing/Chromium 的目录由实际构建决定；不要把 Chrome 的注册路径当作测试 profile 自动隔离。自动测试不在这些目录注册。浏览器 profile 与 host 注册的作用域不同，配对实例身份仍然必须独立确认。

显式授权精确扩展 ID 后 host 才接受该扩展：

```sh
printf '%s\n' '{"op":"allow_extension","extension_id":"EXTENSION_ID"}' | lintel-browser-host control
```

桌面端可将以上步骤包装成用户发起的安装流程；不得偷偷注册。卸载前在扩展中逐项恢复仍归本扩展控制的设置并解除清理隔离。移除 native manifest 和 host 后连接会显示离线；浏览器卸载扩展会移除其 DNR/content/privacy/proxy 控制。浏览器删除的数据不能恢复，bridge 回执不会假装提供“撤销全部”。

## Native bridge 接口

`extensions/browser/native-host` 有自己的 Cargo workspace。桌面可直接依赖 package `lintel-browser-host`，不需要 shell 或 subprocess：

```rust
let response = lintel_browser_host::control(serde_json::json!({"op":"instances"}));
```

所有控制调用返回 `{ "ok": true, "data": ... }`，或者 `{ "ok": false, "error": { "code": "...", "message": "..." } }`。CLI `lintel-browser-host control` 从 stdin 读取同样的 JSON，并把同样的 envelope 输出到 stdout。

| 输入 | data |
| --- | --- |
| `{"op":"pair_create"}` | `{challenge, code, expires_at}`；8 位 hex 短码，5 分钟有效 |
| `{"op":"pair_pending"}` | `[{challenge, code, instance_id, label, browser, extension_id}]` |
| `{"op":"pair_approve","challenge":"..."}` | 已配对实例记录；必须由桌面用户显式批准 |
| `{"op":"instances"}` | `[{instance_id,label,browser,extension_id,paired,conflict,last_seen,online}]` |
| `{"op":"submit","instance_id":"...","operation_id":"UUID","action":{...}}` | `{instance_id,id,action,phase,created_at}` |
| `{"op":"query","instance_id":"...","operation_id":"UUID"}` | 同一操作及收到的 `receipt`；不重新执行 |
| `{"op":"allow_extension","extension_id":"..."}` | `{allowed: extensionId}`；仅供明确批准的本地安装流程 |

`expires_at`、`created_at`、`last_seen` 使用 Unix 秒。extension receipt 的 `createdAt`、`startedAt`、`completedAt` 使用 epoch 毫秒。`paired` 是持久配对；`online` 表示最近 20 秒收到本地 native poll，两者不能互相替代。Native port 断开、权限撤销、重复实例冲突都不能显示为有效保护。

例：提交可逆 WebRTC 操作。

```json
{"op":"submit","instance_id":"INSTANCE_UUID","operation_id":"OPERATION_UUID","action":{"kind":"webrtc","setting":"disable_non_proxied_udp"}}
```

初始 phase 为 `awaiting-browser-confirmation`。用户在扩展中批准后变成 `running`；终态是 `completed`、`uncertain` 或 `rejected`。成功 receipt 示例：

```json
{"id":"OPERATION_UUID","phase":"completed","result":{"verification":"effective-readback","configured":"disable_non_proxied_udp","effective":"disable_non_proxied_udp","controller":"controlled_by_this_extension","scope":"current-profile"},"completedAt":1790985600000}
```

恢复使用新 operation ID，action 为 `{"kind":"restore","receiptId":"OPERATION_UUID"}`，仍需预览确认。有效值或控制者变化时返回 `restore_conflict`，不盖回旧快照。原先由其他层控制的 BrowserSetting 使用 `clear` 撤销本扩展 override，使当前底层值重新显现，不把旧有效值写成永久 override。contentSettings 没有完整控制者查询；恢复仅清理本扩展所写规则、保留其他受管设置，读回不符时拒绝。存在控制来源可见性限制。

其余允许的 action：

```json
{"kind":"clear","origins":["https://claude.ai"],"types":["cookies","localStorage","indexedDB","serviceWorkers","cacheStorage"]}
{"kind":"clearProfileCache"}
{"kind":"sitePermission","origins":["https://claude.ai"],"setting":"location"}
{"kind":"proxy","origins":["https://claude.ai"],"port":8080}
{"kind":"blockSites","origins":["https://claude.ai"]}
{"kind":"pauseRules","minutes":10}
```

`sitePermission.setting` 允许 `location/camera/microphone/notifications`；`webrtc.setting` 允许 `default/default_public_interface_only/disable_non_proxied_udp`。暂停 1–60 分钟，浏览器 alarm 恢复本扩展自己的持久阻断规则；不会解除清理隔离。到期时浏览器不运行则在下次启动 reconciliation 恢复，不承诺离线时刻准时触发。

### Native 管道与信任边界

浏览器通过 `connectNative("app.lintel.browser")` 发起连接。host 只接受 manifest 中精确扩展 origin/ID、host allowlist 和已配对本地实例 token。每帧是 native-endian 32 位长度加 UTF-8 JSON，上限 64 KiB。host 没有 exec/read_file/delete/export、URL fetch 或任意 path 操作。

内部请求共同字段 `{op, request_id, instance_id, token}`；token 为本地生成的 256 位随机数，仅存于扩展 `storage.local` 与用户私有 bridge DB，不使用 sync。

- `pair_request` 附加 `{code,label,browser}`：短码挑战与扩展实例绑定，返回 `{paired:false,pending:true,challenge,code}`；不能自行批准。
- `poll`：返回 `{paired:true,proposals:[...]}`；绑定当前 native connection lease。
- `receipt` 附加 `{receipt:{id,phase,result?,error?,completedAt?}}`：只报告结果；不会请求执行。

回复 envelope 加同一 `request_id`。桥接目录权限 0700，状态与 lock 0600；独占文件锁、写临时文件、fsync、rename、目录 fsync。默认 macOS 为用户 Application Support 下 `Lintel/browser-bridge`，Linux 为用户 `.local/state/lintel/browser-bridge`。合成环境可用 `LINTEL_BROWSER_STATE` 指定临时根；这是进程配置，不接受来自扩展的路径。无网络 listener。

两个同时在线的 native 连接若持同一实例凭据，host 持久标记 `duplicate_instance_conflict`，两个都停止接受操作。扩展提供“生成新身份重新配对”。浏览器没有可信的普通 profile 身份证明，**离线复制后从不同时连接的克隆仍无法被证明区分**；这部分 B02 不算完成。不能把连接冲突检测描述为任意 profile 克隆完整检测。

### 不重复执行的边界

扩展在 native mutation 前把 `running` journal 写入 `storage.local`。执行完成先持久结果，再报告 host。重新启动时 `running` 改为 `uncertain`；相同 ID 无论 completed/uncertain 都不重新执行。ACK 丢失只重发 receipt，用户的新登录不会因为传输重试被再次清理。

Host 操作存储与 extension journal 相互补充。相同 operation ID 绑定原 action，换 action 拒绝。断线或 host 只有 running 记录时，GUI 必须查询/等待扩展结果，不生成新 ID 自动重做。Journal 达 1000 个 host 操作时拒绝新提交；目前尚无用户可用的归档/清理工具，不能称为长期无人维护完成。

## 验证与已知剩余范围

```sh
node --test extensions/browser/tests/*.test.mjs
cargo test --manifest-path extensions/browser/native-host/Cargo.toml
cd extensions/browser
npm install
npx playwright install chromium
npm run test:browser
```

可复用已安装的 Playwright：`PLAYWRIGHT_MODULE=/absolute/path/to/playwright/index.mjs npm run test:browser`。测试只创建临时 profile，并启动固定端口 18765 的合成 localhost HTTP fixture；端口冲突会失败，不改现有服务。不会复用个人 Chrome profile、读取 Cookie DB 或访问真实 Claude 站点。

合成构建 `node scripts/build.mjs --fixture` 生成显眼命名的 fixture 包，只接受 `http://localhost:18765`。其 Cookie/DNR/loopback 权限是测试预授权，与正式包分开。不要将 fixture 包发布给普通用户。

2026-10-03 验证：10 项 JS contract tests 与 5 项 Rust tests 通过；两个开发 ZIP 已构建。macOS arm64 / 缓存 Chromium 155.0.8059.12 的先前实测通过了 popup 确认、五类存储删除、邻域保留、目标标签关闭，以及整个浏览器/worker 重启后新合成登录不被相同 ID 重删的断言；随后截图等待超时。加入 `main_frame` 隔离断言后的最后一轮，浏览器 network service crash 并在 fixture 读取时停住，已结束该合成进程。因此完整 browser harness 尚未通过，`main_frame` 实际阻断及正式 Chrome/Edge/Firefox 验收仍未验证。观察记录在本地生成的 `extensions/browser/artifacts/verification-observations.json`；完整 harness 将在通过时输出 `browser-smoke.json`，都不纳入 Git。JS contract tests 覆盖拒绝路径、journal 恢复、配对要求、Cookie 扩大范围、Firefox scope 拒绝和设置恢复冲突；Rust tests 覆盖挑战配对、双连接冲突、持久回执、非法输入、帧上限与 host 异常退出后的连接恢复。

未完成的产品范围：正式 Chrome/Edge/Firefox 各版本验收、Firefox 容器实测/独立 CacheStorage 清理/按站点代理、完整 iframe 写入者控制、离线克隆识别、native host 非开发者自动安装与签名分发、专用浏览器启动与防进程接管、主应用 deep link、journal 用户归档工具。上述边界需要继续实现或验收，不能因为开发包构建成功而宣布整个浏览器模块交付。

## 官方能力依据

2026-10-03 核对：

- [Chrome browsingData](https://developer.chrome.com/docs/extensions/reference/api/browsingData)：origin 过滤、Cookie 可注册域范围与分类 Promise。
- [Firefox RemovalOptions](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/browsingData/RemovalOptions)：hostname 和 cookieStoreId 的适用类别不同。
- [Firefox removeCache](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/browsingData/removeCache)：全 profile HTTP cache 边界。
- [Firefox privacy.network](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/privacy/network)：WebRTC 策略值与 BrowserSetting。
- [Chrome contentSettings](https://developer.chrome.com/docs/extensions/reference/api/contentSettings)：站点模式、有效值与 extension-owned clear。
- [Chrome proxy](https://developer.chrome.com/docs/extensions/reference/api/proxy)：PAC/profile 设置，并不涵盖全部流量。
- [Firefox background](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/manifest.json/background)：Firefox 使用 scripts/event page，Chrome 使用 service worker。
- [Firefox Native Messaging](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Native_messaging) 与 [Chrome Native Messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging)：安装 manifest、native framing 与 caller identity。
