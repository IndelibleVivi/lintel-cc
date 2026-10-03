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

清理分两步。第一次确认安装持久 DNR 隔离规则（包含 `main_frame`/`sub_frame`），按需申请 `webNavigation`，关闭目标顶层页及嵌入目标 iframe 的宿主标签，再注销所选 Service Worker。只在此次操作读取 frame URL 用于匹配，不保存页面正文或浏览记录。第一次回执为 `awaiting-browser-restart` / `preparation-only`，其余存储尚未删除。Chromium Cookie 扩大到可注册域时，隔离和标签范围同时扩大。容器清理只删除所选 store 数据、关闭该 store 的标签，但 DNR 暂时隔离整个 profile 对目标主机的请求；预览明确披露。容器清理不支持按 store 注销 Service Worker，因此结果标记后台回写未验证。

隔离在清理后保留，用户在回执中单独点击“已核对结果，解除站点隔离”，之后可以自己重访站点。不会自动重访并把新 Cookie 误报成旧数据残留。隔离绑定准备/删除 operation 的所有权；旧回执不能解除新任务隔离。

用户完整退出并重启该浏览器后，扩展记录原生 `runtime.onStartup` 的新启动世代。点击“预览重启后继续删除”，再次确认原范围，才通过新的 `finishClear` operation 删除存储。扩展 worker 重启不算浏览器重启，隔离规则缺失或变化会拒绝继续。准备记录只能消费一次；断线不会自动再删。此流程来自真实负例：已注销的活跃 Service Worker 仍能完成 `waitUntil` 事件并重建 CacheStorage；仅等删除 API 成功不足以证明停写。工具不会自行退出、强杀或重启用户浏览器。

第二次确认的浏览器 callback/Promise 成功记为 `browser-acknowledged`。Firefox 容器无法按 store 注销 Service Worker，仍明确标记 `container-frames-closed-service-workers-not-verified`，不宣称容器后台写入已完全停止。容器操作需要可选 `cookies` 权限，用于验证 store 存在并识别其标签；未知 store 在修改前拒绝。没有通用存储枚举能力时不显示“零残留”。可选 `cookies` 权限已授予时仅返回剩余数量，Cookie 值不离开扩展执行上下文，也不进入 journal。正式包默认不申请该权限，合成测试包单独预授权。

## App 内准备与开发加载

macOS App 已带 Chromium / Firefox 伴随扩展，不要求桌面用户安装 Node、拿源码或编译 host。首页“浏览器”→“连接一个新的 profile”选择目标浏览器，按三步进行：

1. “预览扩展目录”：只读核对版本、大小、文件清单和目标目录；批准后才准备文件。Chrome / Edge 共用当前用户 `~/Library/Application Support/Lintel/browser-extensions/chromium/`，Firefox 使用同级 `firefox/`。准备完成后可在 Finder 打开，或复制目录／manifest 路径。
2. 在目标 profile 加载扩展。Chrome 打开 `chrome://extensions`，Edge 打开 `edge://extensions`，启用开发者模式，选择“Load unpacked / 加载已解压的扩展”和已准备的目录。Firefox 打开 `about:debugging#/runtime/this-firefox`，选择“Load Temporary Add-on”和目录里的 `manifest.json`。随后将管理页中的准确扩展 ID 填到 App 第二步，预览并另行批准本地连接。
3. 生成配对请求，复制 App 显示的 12 位配对短码，在目标扩展输入并提交请求；核对两端短码，再在 App 批准这份 profile。目录准备、本地连接注册、profile 配对、最近轮询在线是四个不同事实。

准备只复制 App 内置文件，不写浏览器 profile、偏好、扩展加载登记、native manifest 或 allowlist。目录保持稳定，移动 App 不影响浏览器加载路径；原有受管文件可预览后批准更新，随后要在浏览器重新加载。非受管目录、被改动的文件或 ownership 冲突会拒绝覆盖；批准前重新核对整份计划。上次准备中断时，新的预览会列出待完成的受管目录切换；重新批准后才继续，不把备份或暂存目录当成已完成。未完成切换期间 App 资源版本改变会阻止恢复；需先核对原版本资源和目录变化，保留现有状态，不自动改用新版本继续。已准备的相同版本可重复核对，无须重写。

这仍是开发加载：Chrome / Edge 需要开发者模式，Firefox 临时扩展在退出后移除，长期 Firefox 安装需要签名。商店／签名分发与非开发者首次安装验收尚未完成。文件准备成功不代表浏览器已经加载。独立 Chromium 的两阶段清理有下述真实 runtime 证据，正式浏览器与非开发者安装仍需各自验收。

### AdsPower 与其他 Chromium 浏览器

AdsPower 的 [profile 文档](https://help.adspower.com/docs/creating_browser_profiles) 区分基于 Chrome 的 SunBrowser 与基于 Firefox 的 FlowerBrowser；[扩展文档](https://help.adspower.com/docs/extensions) 说明 Chrome 商店／ZIP 与受管扩展的安装方式。通过 AdsPower 管理扩展可能涉及云端上传、缓存或扩展数据同步，应按其实际设置判断范围，Lintel 不会自动上传。扩展 API 兼容性不能证明 Native Messaging 可用；本轮没有找到可核实的 AdsPower host 注册路径，也没有操作 AdsPower 的现有 profile。

当前 App 安装器只提供 Chrome、Edge、Firefox 的固定注册路径；选择 Chrome 不代表安装到了 AdsPower。AdsPower 的 Native Messaging、完整重启与清理尚未验收，不列为已支持平台。需要在独立新建 profile 验证该浏览器实际读取的注册与调用行为后，才能实现它的安装入口。

## 独立构建与开发安装

以下供开发者或独立 operator 使用；App 用户可直接走上面的目录准备。仓库根目录：

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

**桌面用户：** 在首页“浏览器”打开“连接一个新的 profile”，选择 Chrome / Edge / Firefox，输入目标扩展的准确 ID，点击“预览本地连接安装”。App 自带当前平台的 host，桌面无需构建或指定可执行路径。预览只读；显示版本、大小、组件摘要、安装路径、注册内容和既有注册。批准安装后，组件放到当前用户的 `~/Library/Application Support/Lintel/browser-host/<version>-<sha256>/`，注册文件指向其中 `lintel-browser-host`，不会随 App 移动而改变。

桌面安装目前支持 macOS。相同组件／注册可核对；本安装器管理的旧版本可在对照旧注册后批准更新，旧二进制保留。其他来源的注册、修改过的组件或失配的 ownership 记录会阻止覆盖；未把它们自动认领。整个批准计划在写入前重查，过期时需重新预览。安装会授权该准确 extension ID，**短码配对仍是下一次独立批准**；成功不表示 profile 在线。扩展可由 App 准备固定目录后按上节开发加载；内置组件不代表扩展已签名、上架或两阶段清理已验收。

`npm run desktop:build` 自动构建非 fixture Chromium / Firefox 扩展并打包独立 host，目前要求与 Rust host 相同架构的 native 构建；跨架构与 universal App 构建会明确拒绝，避免打入错误组件。开发资源可在 `apps/desktop` 运行 `npm run prepare:browser-extension` 与 `npm run prepare:browser-host` 准备；这里只生成 App 资源，不注册个人浏览器。资源缺失时 App 返回具体 `bundle_unavailable` 提示。Linux 独立 CLI 仍按以下路径使用手工安装器；桌面不再开放手填 host 路径的安装操作。

**独立 CLI：** 以下命令仍供直接使用 host 的 operator 使用。

以下命令仅生成 manifest 到 stdout，不注册、不修改任何个人浏览器。用真实、已审查的绝对 host 二进制路径替换占位符：

```sh
lintel-browser-host manifest chrome EXTENSION_ID /absolute/path/to/lintel-browser-host
lintel-browser-host manifest firefox lintel@lintel.local /absolute/path/to/lintel-browser-host
```

已有可执行 native host 后，可使用内置安装器生成固定浏览器路径的计划；没有 `--apply` 时不写入：

```sh
lintel-browser-host register chrome CHROME_EXTENSION_ID /absolute/path/to/lintel-browser-host
lintel-browser-host register edge EDGE_EXTENSION_ID /absolute/path/to/lintel-browser-host
lintel-browser-host register firefox lintel@lintel.local /absolute/path/to/lintel-browser-host
```

审查该计划后，对所选浏览器重复同一命令并添加 `--apply`，注册 manifest 并将精确 extension ID 加入 host allowlist。相同内容重复运行返回 `already-registered`；已有不同 manifest 返回 `manifest_conflict`，不会覆盖。未找到 host、非可执行 host、错误扩展 ID 均拒绝安装。安装不代替短码配对。`--home /absolute/synthetic-home` 只用于指定独立测试 home；本轮未向个人浏览器注册目录安装。Chrome、Edge、Firefox 的 macOS/Linux 路径均有临时目录 contract tests；正式浏览器安装验收仍需分别执行。

审查后，用户显式安装生成的 JSON，文件名 `app.lintel.browser.json`。浏览器规定的当前用户目录如下（`~` 指实际用户 home）：

| 系统 | Chrome | Edge | Firefox |
| --- | --- | --- | --- |
| macOS | `~/Library/Application Support/Google/Chrome/NativeMessagingHosts/` | `~/Library/Application Support/Microsoft Edge/NativeMessagingHosts/` | `~/Library/Application Support/Mozilla/NativeMessagingHosts/` |
| Linux | `~/.config/google-chrome/NativeMessagingHosts/` | `~/.config/microsoft-edge/NativeMessagingHosts/` | `~/.mozilla/native-messaging-hosts/` |

本 smoke 使用的 Chromium / Chrome for Testing 从临时 user-data-dir 的 `NativeMessagingHosts/` 子目录读取 host manifest；其他发行版的注册位置需分别核实，不能据此推断 AdsPower。自动 smoke 只向自己新建的临时 user-data-dir 注册，并把 host DB 指向同一临时根；不使用个人注册目录。浏览器 profile 与 host 注册的作用域不同，配对实例身份仍然必须独立确认。

host 只接受已授权的精确扩展 ID，授权**只**发生在上述用户确认的安装路径：独立 `register --apply`，或桌面「批准安装本地连接／批准更新本地连接」（`install_bundled_host`）在安装流程内把该 ID 加入 host allowlist。通用 control 通道上的 `allow_extension` 不再接受写入，直接返回 `allow_extension_requires_installer`，防止任何同用户进程绕过已确认的安装动作自行放行任意扩展。

桌面只在准确预览获批准后安装和注册。卸载前在扩展中逐项恢复仍归本扩展控制的设置并解除清理隔离。移除 native manifest 和 host 后连接会显示离线；浏览器卸载扩展会移除其 DNR/content/privacy/proxy 控制。浏览器删除的数据不能恢复，bridge 回执不会假装提供“撤销全部”。

## Native bridge 接口

`extensions/browser/native-host` 有自己的 Cargo workspace。桌面可直接依赖 package `lintel-browser-host`，不需要 shell 或 subprocess：

```rust
let response = lintel_browser_host::control(serde_json::json!({"op":"instances"}));
```

所有控制调用返回 `{ "ok": true, "data": ... }`，或者 `{ "ok": false, "error": { "code": "...", "message": "..." } }`。CLI `lintel-browser-host control` 从 stdin 读取同样的 JSON，并把同样的 envelope 输出到 stdout。

| 输入 | data |
| --- | --- |
| `{"op":"bundled_extension_plan","browser":"chrome"}` | **仅桌面**：只读 `{schema,browser,package,version,sha256,bytes,files,extension_path,manifest_path,resource_path,platform,effect,existing_installation,existing_files,recovery,status,install_action}`；Chrome / Edge 映射到 chromium，Firefox 到 firefox。status 为 ready / already-installed / conflict，install_action 为 install / upgrade / none / blocked；recovery 为 null 或上一批准准备的 `{phase,action,message,transaction,stage_path,previous_path,stage_files,stage_record,previous_installation,previous_files}` |
| `{"op":"install_bundled_extension","approved_plan":{...}}` | **仅桌面**：批准并复查完整计划，返回 `{status,plan,loading:"required",pairing:"required"}`；status 为 installed / updated / already-installed。只准备固定目录；发生中断不能返回成功，需新的恢复预览与批准 |
| `{"op":"reveal_bundled_extension","browser":"chrome"}` | **仅桌面**：核对受管且完整的固定扩展目录后在 Finder 打开，返回 `{status:"revealed",browser,package,extension_path}`；不接受 path 或 URL；没有已完成的受管目录时拒绝 |
| `{"op":"bundled_host_plan","browser":"chrome","extension_id":"EXTENSION_ID"}` | **仅桌面**：只读 `{schema,browser,extension_id,version,sha256,bytes,host_path,manifest_path,manifest,state_path,effect,platform,architecture,existing_manifest,existing_registration,existing_host,installed_host,status,install_action}`。status 为 ready / already-registered / conflict；install_action 为 install / upgrade / none / blocked |
| `{"op":"install_bundled_host","approved_plan":{...}}` | **仅桌面**：完整 frozen plan 批准与复查，返回 `{status,plan,pairing:"required"}`。status 为 registered / updated / already-registered；相同已成功批准只查询，不再次写入或授权；blocked 拒绝 |
| `{"op":"installation_plan","browser":"chrome","extension_id":"EXTENSION_ID","host_path":"/absolute/path/to/lintel-browser-host"}` | `{browser,extension_id,manifest_path,manifest,state_path,effect}`；只预览当前用户固定路径 |
| `{"op":"install_native_host","browser":"chrome","extension_id":"EXTENSION_ID","host_path":"/absolute/path/to/lintel-browser-host"}` | `{status,plan,pairing:"required"}`；用户批准后安装。browser 也可为 edge/firefox；以上两个手工路径操作仅供独立 CLI local control，桌面返回 unsupported_browser_operation，扩展管道不开放 |
| `{"op":"pair_create"}` | `{challenge, code, expires_at}`；12 位大写 hex 短码，5 分钟有效；同时未决挑战上限 8 个，超出逐出最旧 |
| `{"op":"pair_pending"}` | `[{challenge, code, instance_id, label, browser, extension_id}]` |
| `{"op":"pair_approve","challenge":"..."}` | 已配对实例记录；必须由桌面用户显式批准 |
| `{"op":"instances"}` | `[{instance_id,label,browser,extension_id,paired,conflict,last_seen,online}]` |
| `{"op":"submit","instance_id":"...","operation_id":"UUID","action":{...}}` | `{instance_id,id,action,phase,created_at}` |
| `{"op":"query","instance_id":"...","operation_id":"UUID"}` | 同一操作及收到的 `receipt`；不重新执行 |
| `{"op":"allow_extension","extension_id":"..."}` | 不再开放：始终返回 `allow_extension_requires_installer`；授权只走安装路径（见上节） |

`expires_at`、`created_at`、`last_seen` 使用 Unix 秒。extension receipt 的 `createdAt`、`startedAt`、`completedAt` 使用 epoch 毫秒。`paired` 是持久配对；`online` 表示最近 20 秒收到本地 native poll，两者不能互相替代。Native port 断开、权限撤销、重复实例冲突都不能显示为有效保护。

例：提交可逆 WebRTC 操作。

```json
{"op":"submit","instance_id":"INSTANCE_UUID","operation_id":"OPERATION_UUID","action":{"kind":"webrtc","setting":"disable_non_proxied_udp"}}
```

初始 phase 为 `awaiting-browser-confirmation`。用户在扩展中批准后变成 `running`；一般终态是 `completed`、`uncertain` 或 `rejected`。站点清理准备会进入 `awaiting-browser-restart`，还不是删除成功；重启后单独提交 `finishClear`，新操作产生删除结果。成功 receipt 示例：

```json
{"id":"OPERATION_UUID","phase":"completed","result":{"verification":"effective-readback","configured":"disable_non_proxied_udp","effective":"disable_non_proxied_udp","controller":"controlled_by_this_extension","scope":"current-profile"},"completedAt":1790985600000}
```

恢复使用新 operation ID，action 为 `{"kind":"restore","receiptId":"OPERATION_UUID"}`，仍需预览确认。有效值或控制者变化时返回 `restore_conflict`，不盖回旧快照。原先由其他层控制的 BrowserSetting 使用 `clear` 撤销本扩展 override，使当前底层值重新显现，不把旧有效值写成永久 override。contentSettings 没有完整控制者查询；恢复仅清理本扩展所写规则、重建先前本扩展的规则，并读取当前有效值；保留其他受管设置，读回与应恢复的自有规则不符时拒绝。存在控制来源可见性限制。

其余允许的 action：

```json
{"kind":"clear","origins":["https://claude.ai"],"types":["cookies","localStorage","indexedDB","serviceWorkers","cacheStorage"]}
{"kind":"finishClear","receiptId":"PREPARATION_OPERATION_UUID"}
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

- `pair_request` 附加 `{code,label}`：短码挑战与扩展实例绑定，返回 `{paired:false,pending:true,challenge,code}`；不能自行批准。`browser` 字段由 host 根据 argv 校验过的调用方扩展 ID 派生（`lintel@lintel.local` → firefox，32 位 Chromium ID → chromium），请求里自报的 browser 一律忽略。短码连续 5 次错误尝试会把所有存活中的待配对短码作废，需要重新生成。
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
npm ci
npx playwright install chromium
npm run test:browser
```

需要已有可构建的 Rust toolchain，smoke 会构建 debug native host。可复用已安装的 Playwright：`PLAYWRIGHT_MODULE=/absolute/path/to/playwright/index.mjs npm run test:browser`。Playwright 与浏览器 revision 应匹配；如指定 `PLAYWRIGHT_CHROMIUM_EXECUTABLE`，仅允许 `ms-playwright/chromium-*` 缓存中的可执行文件，不接受 `/Applications` 的普通 Chrome。测试只创建临时 profile/native DB，并启动固定端口 18765 的合成 localhost HTTP fixture；端口冲突会失败，不改现有服务。不会复用个人 Chrome profile、读取 Cookie DB 或访问真实 Claude 站点。

合成构建 `node scripts/build.mjs --fixture` 生成显眼命名的 fixture 包，只接受 `http://localhost:18765`。其 Cookie/DNR/loopback 权限是测试预授权，与正式包分开。不要将 fixture 包发布给普通用户。

2026-10-04 验证：20 项 JS contract tests 与 12 项 native host Rust tests 有既有通过证据。macOS arm64 / Playwright Chromium 155.0.8059.12 的当前完整两阶段 smoke 已通过，保留原有 11 项断言：真正活跃的 SW `waitUntil` CacheStorage writer 与 iframe writer、目标／宿主关闭、隔离保持、五类存储删除／邻域保留、定位权限目标级 block/restore、旧 operation ID 不重删新的合成登录，以及真实 `connectNative` 短码请求／本地批准／浏览器确认 WebRTC／host 持久回执。

持久安装由测试专用 [persistent-install.mjs](../extensions/browser/tests/persistent-install.mjs) 调用 Chromium 原生管理页安装器完成：仅临时破坏自己复制的 fixture manifest，以取得浏览器生成的加载失败恢复凭据，随后逐字节恢复，再由原生安装器重试。这遵循 [Chromium 原生 reload/loadUnpacked 实现](https://chromium.googlesource.com/chromium/src/+/main/chrome/browser/extensions/api/developer_private/developer_private_functions.cc)；不是产品的自动扩展安装入口。测试确认旧浏览器进程已退出、新进程不同、重新启动没有 `--load-extension` 或 `--disable-extensions-except`，同一扩展身份保留，生产监听器实际收到新的 `runtime.onStartup` 世代，再单独确认 `finishClear`。测试不直接写入 profile preferences 或启动世代，也不伪造启动事件。详细证据写入本地 `extensions/browser/artifacts/browser-smoke.json`，失败则写 `browser-smoke-failure.json`；这些生成文件不纳入 Git。

此前的活跃 SW 负例证明“注销成功”仍可能回写；命令行临时加载和独立 CDP 临时安装又未产生持久启动世代，执行器正确拒绝 `browser_restart_required`。这些失败推动了现在的两步流程和原生持久安装 harness，不能用旧静止 worker 成功替代当前检查。macOS headless fixture 在种植／读取存储前需 `bringToFront()`，浏览器与 Playwright revision 要匹配；30 秒阶段 deadline 未放宽。

App 配对也有可重复运行的页面到真实 host 检查。旧页面复制的是 64 位内部 challenge，被 `pair_request` 拒绝为 `invalid_pairing_code`；现在显示并复制真正的 12 位短码，随后核对并批准内部 pending challenge。仓库根目录构建前端并运行：

```sh
npm --prefix apps/desktop run build
python3 tests/verify.py --checks browser-smoke,browser-pairing-ui --json /tmp/lintel-browser-runtime.json
```

这需要上面的 Playwright 依赖；`browser-pairing-ui` 使用真实 core／host 进程，但 invoke 和 clipboard 是合成 transport。它验证复制值被实际 framed native 请求接受、显式批准、离线状态、短码更新与键盘／Day/Night 布局；不证明 native WebKit、OS clipboard 或非开发者首次安装。CI 在 macOS／Ubuntu 单独选取两项，不将其混入默认合成检查；具体结果见 [当前状态](current-state.md) 与 [验证指南](verification.md)。

未完成的产品范围：正式 Chrome/Edge/Firefox 与 AdsPower 各版本验收、Firefox 容器实测／独立 CacheStorage 清理／按站点代理、离线克隆识别、native host 与扩展的签名分发、专用浏览器启动与防进程接管、主应用 deep link、journal 用户归档工具。独立 Chromium 使用真实 runtime 与合成数据，不访问 claude.ai、不证明真实认证或重新登录；开发包与局部通过不代表整个浏览器模块交付。

## 官方能力依据

2026-10-03 核对：

- [Chrome browsingData](https://developer.chrome.com/docs/extensions/reference/api/browsingData)：origin 过滤、Cookie 可注册域范围与分类 Promise。
- [Firefox RemovalOptions](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/browsingData/RemovalOptions)：hostname 和 cookieStoreId 的适用类别不同。
- [Firefox removeCache](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/browsingData/removeCache)：全 profile HTTP cache 边界。
- [Firefox privacy.network](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/privacy/network)：WebRTC 策略值与 BrowserSetting。
- [Chrome contentSettings](https://developer.chrome.com/docs/extensions/reference/api/contentSettings)：站点模式、有效值与 extension-owned clear。
- [Chrome proxy](https://developer.chrome.com/docs/extensions/reference/api/proxy)：PAC/profile 设置，并不涵盖全部流量。
- [ServiceWorkerRegistration.unregister](https://developer.mozilla.org/en-US/docs/Web/API/ServiceWorkerRegistration/unregister)：注销与结束正在运行的事件是不同事实。
- [Chrome runtime.onStartup](https://developer.chrome.com/docs/extensions/reference/api/runtime#event-onStartup)：profile 启动事件用于区分浏览器重启与 worker 重启。
- [Chrome webNavigation](https://developer.chrome.com/docs/extensions/reference/api/webNavigation)：getAllFrames 用于目标 iframe 匹配。
- [Firefox DataTypeSet](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/browsingData/DataTypeSet)：没有独立 CacheStorage 类别；不把全缓存或 serviceWorkers 字段冒充可单独定向的 CacheStorage API。
- [Firefox background](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/manifest.json/background)：Firefox 使用 scripts/event page，Chrome 使用 service worker。
- [Firefox Native Messaging](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Native_messaging) 与 [Chrome Native Messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging)：安装 manifest、native framing 与 caller identity。
