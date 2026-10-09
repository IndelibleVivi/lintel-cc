# Lintel App 更新与发行准备

本指南说明当前源码中的 macOS App updater 与未来发行流程。当前公开入口仍是 **0.1.0 开发预览，没有正式 Release 下载、发行公钥或已激活 feed**。源码接通、合成测试、构建、安装、公开 Release 和生产 feed 分别成立；准确证据见 [current-state](current-state.md)。CLI 版本安装仍使用 [Agent CLI](agents.md) 的独立流程。

## 在 App 中使用

打开“设置与模块 → Lintel 版本与更新”。版本来自 native App 的 package metadata；通道由构建固定，不能通过 Webview 请求任意 URL、公钥、降级或安装器参数。

1. **检查更新。** 手动检查访问固定的 `https://lintel.page/updates/preview.json` 或 `stable.json`。未配置构建明确显示不可用；离线、无 feed、响应不合规均是错误，不显示“已经最新”。成功检查只说明该通道没有更高版本或有候选。
2. **下载并核验。** 使用官方 Tauri updater 验证下载字节与签名，再核对签名 trusted comment 中的版本。下载完成事件还不是核验通过。候选冻结版本、签名、URL 与本次 ID；重新检查会使旧候选失效。
3. **审阅并安装。** 勾选当前候选并批准后才替换 App。native 层排除本 App 正在执行的 core、browser、SSH 和 network 操作，并拒绝仍有活跃受控代理的安装。下载与检查可以独立进行。安装前持久化原意图；写入未确认则不进入安装。
4. **保存好工作后重启。** 安装不自动重启。再次检查 native activity 与代理后才接受显式重启；请先保存编辑中的交接稿。外部 Claude 会话和已 detached 的远端任务不由 updater 杀掉，原任务仍按原 ID 查询。

后台检查默认关闭。显式启用后仅在 App 打开且可见时、至多每小时检查一次；关闭 Settings 不停止已选择的检查，取消选择立即停止。它不自动下载、安装或重启，也不是菜单栏／系统后台服务。偏好只存当前用户的 App data；不初始化 Claude/core state。

请求会向 Lintel feed／GitHub 托管方暴露出口 IP 和普通 HTTP 信息；不上传 Claude root、账号、会话、环境清单或设备标识。它与 Claude Code 自身的遥测设置分别管理。下载包 admission 为 512 MiB、下载总时限五分钟；检查总时限25秒。检查响应解析使用官方 updater，成功返回后再限制 notes／signature 与候选来源；不是通用 feed 或任意 HTTP 客户端。

## 不确定结果与版本边界

更新记录保留原 ID、起止版本与尝试结果。重开时未完成的安装意图显示 interrupted，失败或记录发布未确认显示 uncertain，不自动重放。旧版本尚在运行且有未确认安装时，下一次安装被拒绝；核对 App 位置与版本，必要时用官方 DMG 手动修复。读到目标或更高运行版本可以解除对未来升级的阻挡，但不把原 interrupted 记录改写成已完成。

同用户多个 App 副本共用记录时，锁与读回比较阻止旧副本覆盖新意图。activity 与代理检查只覆盖当前 App 进程，不能证明其它 App 副本、CLI 或外部程序已停；安装前关闭其它 Lintel 副本。记录损坏或外部改写会明确拒绝，保留文件供核对。

取消操作等待不会提前释放仍在运行的 blocking core 检查；通道停止等待被取消时，未结束的原通道仍挡住安装，后续显式停止继续核对同一任务。

SemVer 只按 precedence 比较：`0.1.0-preview.1` 低于 `0.1.0`，`+build` 不提高优先级，相同版本不能作为升级。下一次 preview 可使用高于当前版本的 `0.2.0-preview.1`。preview 与 stable feed 分开，stable 不接收 prerelease；不使用 GitHub `releases/latest` 作为 preview 选择。旧的、未带 updater／公钥的已安装 App 需要手动升级一次。

## 一份记录准备三种输出

[`app-release.mjs`](../scripts/app-release.mjs) 是本地发行准备入口。它不创建 key、不签名、不上传、不创建 GitHub Release、不部署 Pages。输入是本地 `lintel.app-release/1` 记录；包含实际 artifact 路径的输入留在 Git 外或 ignored 目录，公开输出不含本机路径。

先选择精确 source revision、严格升序的新版本、通道、架构与已审阅的 **公开**更新公钥。例如下列是字段示例，不能作为真实发行数据：

```json
{
  "schema": "lintel.app-release/1",
  "version": "0.2.0-preview.1",
  "previous_version": "0.1.0",
  "channel": "preview",
  "source_revision": "<40-character-source-commit>",
  "pubkey": "<base64-encoded-public-key-file-content>",
  "notes": "本次公开变化与兼容说明",
  "pub_date": "2026-10-09T00:00:00Z",
  "platforms": {
    "darwin-aarch64": {
      "archive_path": "/path/to/Lintel.app.tar.gz",
      "signature_path": "/path/to/Lintel.app.tar.gz.sig",
      "dmg_path": "/path/to/Lintel.dmg",
      "archive_name": "Lintel_0.2.0-preview.1_aarch64.app.tar.gz",
      "dmg_name": "Lintel_0.2.0-preview.1_aarch64.dmg"
    }
  }
}
```

`darwin-x86_64` 是独立 Intel macOS 目标。当前 inspector 验收单架构 Mach-O；universal archive 不在此准备路径。每个公开文件名必须含精确版本，DMG 与 updater archive 分开。

```sh
# 先审阅构建配置；输出目录必须不存在。
node scripts/app-release.mjs configure \
  --record candidate-packages/app-release-input.json \
  --out candidate-packages/app-config

# 在已准备所需静态资源与 toolchain 的 clean source 上构建。
node scripts/app-release.mjs build \
  --record candidate-packages/app-release-input.json \
  --platform darwin-aarch64

# 对已构建、已签名的实际文件做离线核验，输出仍是 local candidate。
node scripts/app-release.mjs prepare \
  --record candidate-packages/app-release-input.json \
  --out candidate-packages/app-prepared
```

`build` 校验当前 clean Git revision 与声明相同、previous_version 与当前 source App version 相同，固定 Tauri target、`app`／`dmg`、`createUpdaterArtifacts`、公钥与 feed，并传递同一版本给 frontend。Tauri 更新签名秘密使用 Tauri 已支持的本地 secret／environment 机制；不写在输入记录、命令行、源码、前端或公开 manifest 中。这个命令可能调用本机签名工具，只在对应的构建／签名授权下运行。

`prepare` 实际校验 `.app.tar.gz` 的有限路径、条目类型／容量、Bundle ID、Info.plist 版本、`lintel-desktop` executable 的 Mach-O 架构、Minisign artifact／trusted-comment 签名与精确版本，再计算用于上传字节核对的 SHA-256。不执行 archive 或 DMG。DMG 的字节核对不证明能安装，source_revision 是声明而非源码认证；Apple Developer ID／公证仍标 unverified。输出有：

- `release.json`：公开候选记录、平台、首次下载资料与字节证据；没有本机路径。
- `updates/preview.json` 或 `updates/stable.json`：该通道的静态 Tauri feed 候选。
- configure 的 `tauri-release.json`：可审阅的构建 override；正式构建建议用上面的 build 入口确保 frontend/native 版本一致。

更新签名保障 updater 接受的包与版本；Apple Developer ID code signing 与 notarization 是 macOS 安装信任的另一层，不由更新签名代替。正式非开发者分发须独立完成 [Apple notarization](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)，不绕过 Gatekeeper。

## 有发行授权时的激活顺序

本轮不执行这些外部动作。未来有精确发行与托管授权时：先完成签名／公证与候选安装验收，再创建相应版本的 GitHub Release，按记录中的公开名字上传各架构 DMG、`.app.tar.gz` 和 `.sig`；保持版本／tag／文件名不可变，不复用已发行版本。

公开 artifact 上传完成后先逐字节核对，**feed 最后激活**：

```sh
node scripts/app-release.mjs verify-public \
  --record candidate-packages/app-prepared/release.json \
  --out candidate-packages/app-public-verified

node scripts/prepare-site.mjs \
  --media-dir candidate-packages/site-media \
  --app-release candidate-packages/app-public-verified/release.json \
  --out candidate-packages/site-with-app
```

`verify-public` 只读取记录里固定仓库／tag 的 HTTPS URLs，核对实际字节数与 SHA-256，输出另一个目录，不改旧记录。需要大文件网络下载；可达性与字节一致仍不是 Apple 签名、公证或 native 使用证明。`prepare-site` 必须拿到已核对公开字节的记录才生成下载入口与 feed，二进制始终留在 GitHub，不装入 Pages 的25MiB文件 payload。未提供记录时官网继续显示源码预览，无假下载。

有两个已激活通道时，**同次 payload 传入两个 `--app-release`**，各自保留原记录，防止整站更新使另一条 feed 消失。官网首装优先显示 stable，否则显示 preview；版本、下载资料与 feed 来自同一记录。`updates/*` 与下载元数据禁用 immutable 缓存。按 [site-hosting](site-hosting.md) 的 preview／生产／TLS／读回流程上传，再从各通道已有旧 App 验收检查、下载、安装与原任务恢复；源代码 push 不激活这一切。

feed 故障保留当前 App；签名或版本不匹配拒绝安装。误发候选先停止该 feed 的新推荐，修复后用更高版本发布；不打开降级选项或通过更改版本号重放旧包。保留此前公开文件与原更新记录供人工恢复。

浏览器 extension/native host、远端 runner 不在 App updater 的自动安装边界。新 App 可能带新资源，但必须继续各自的 frozen preview、明确批准与原安装核对；旧远端任务仍使用自己冻结的 runner。

协议与 API 依据 [Tauri updater](https://v2.tauri.app/plugin/updater/)；运行行为以锁定的 `tauri-plugin-updater`、当前 CLI 的 `signer sign --app-version` 和本仓库检查为准。源码验收标准见 [遥测与更新 Leaf](specs/telemetry-updates.md)。
