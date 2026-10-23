# Lintel App 更新与发行准备

简体中文（canonical） · [English](app-updates.en.md) · [文档入口](index.md)

本指南说明当前源码中的 macOS App 手动安装、updater 与发行流程。[v0.1.1 开发者 Preview](https://github.com/IndelibleVivi/lintel-cc/releases/tag/v0.1.1) 已修复 0.1.0 DMG 的 bundle 签名封装缺陷；新 DMG、双语安装说明与校验清单共四个公开资产已核对实际字节，Chrome 下载副本也已只读挂载并通过内部 App 的严格签名校验。旧 0.1.0 DMG 保留为历史，不应继续安装。App 使用完整 ad-hoc 签名，没有 Developer ID 或 Apple 公证，**没有发行公钥或已激活 feed**。未公证下载仍可能被 Gatekeeper 拒绝；本次观察到“仍要打开”入口，未把单 App 信任例外当作干净 Mac 验收。准确证据见 [current-state](current-state.md)。CLI 仍使用 [Agent CLI](agents.md) 的独立流程与原候选版本。

## 手动安装开发者 Preview

1. 从 [官网](https://lintel.page/download/) 或当前 GitHub Preview Release 下载对应架构的 DMG，并阅读同一 Release 的安装说明；目前 App 仅提供 Apple Silicon。先将 DMG 的 `shasum -a 256` 结果与该 Release 的 `SHA256SUMS` 对照。校验和只证明下载字节一致，不证明 Apple 认证或可安装。
2. 退出其它 Lintel 副本，打开 DMG，将 `Lintel.app` 拖入“应用程序”。需要替换旧版本时先保留旧 App；用户资料和任务记录独立于 App bundle，不要删除它们来重试启动。
3. 首次打开可能遇到未认证开发者／未公证提示。仅当你已核对来源并决定信任这份 Preview 时，按 [Apple 的单 App 例外说明](https://support.apple.com/102445)，到“系统设置 → 隐私与安全性”查看“仍要打开”，再由你确认。这个选择不关闭全局 Gatekeeper；不同系统或管理策略可能不提供该选项。
4. 如果仍提示“已损坏”，先取消，不反复重装或修改全局安全设置。核对版本与下载校验和，并运行下面的只读检查；保留完整错误供报告。严格签名通过仍不代表 Developer ID、公证或 Gatekeeper 接受。

   ```sh
   codesign --verify --deep --strict --verbose=4 /Applications/Lintel.app
   codesign -dvvv /Applications/Lintel.app
   ```

本机移除单个 App 下载隔离标记属于另一项明确的信任例外，构建、安装器和 App 不会自动执行。项目不会提供关闭全局 Gatekeeper 的安装步骤。

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

<a id="本地未签名-dmg-与第三方说明"></a>

## 本地 Preview DMG 与第三方说明

没有 Developer ID 时仍可在本机构建 DMG，便于保存和检查。Preview 仍必须有完整的 bundle 级 ad-hoc 签名：`Info.plist` 要进入签名，`Contents/Resources` 要被封签。它不同于 Developer ID，也不证明 Gatekeeper 首装或公证通过；构建本身不会发布下载或激活 updater。不要使用 `--no-sign`；统一入口在 App 资源组装完成后签名，再生成 DMG。

先准备两种 Linux runner（如需内置安装能力），并在 clean source 上运行受控入口。它沿用调用者的 Tauri/Cargo target 缓存和工具链环境；只有 `tauri.conf.json` 的非空 `signingIdentity`、`APPLE_SIGNING_IDENTITY` 与 `APPLE_CERTIFICATE` 均不存在时，才为本次本地构建增加 ad-hoc fallback：

```sh
cd apps/desktop
npm run desktop:build:macos-preview -- \
  --out "/path/to/new/lintel-preview-$(date +%Y%m%d-%H%M%S)"
```

该入口要求候选目录尚不存在，不覆盖已有内容；同一次 `tauri build --bundles app,dmg` 完成 App 封装签名与 DMG 生成，然后调用 `scripts/verify-macos-bundle.mjs`。gate 会拒绝 linker-only、plist 未进入签名、没有资源封签或严格验证失败的 App，并通过 `codesign -dvvv` 读取 `Info.plist entries`、`Sealed Resources version/files`，不靠忽略 `codesign --verify` 输出来放大成功范围。资源或 plist 在签名后被改动会失败。复制到候选目录的 App 会再次通过 gate；DMG 只复制，不声称已有 DMG 签名、只读挂载或内部 App 读回证据。

如果只需要检查已存在的受控 bundle：

```sh
node scripts/verify-macos-bundle.mjs /path/to/Lintel.app
```

记录 source revision、架构、实际检查结果并保留旧包；不自动覆盖 Applications，不绕过 Gatekeeper。Apple Developer ID、公证、真实首装、GitHub Release、官网下载和 feed 仍是独立步骤。`macos-bundle-signature-test` 用 synthetic App 证明旧 linker-only 失败、完整 bundle ad-hoc 通过以及 resource/plist tamper 被拒绝；它不运行 native App，也不替代真实候选构建。

`build:desktop-resources` 自动运行 `scripts/prepare-rust-notices.mjs`，从三个锁定 Cargo manifest 离线收集目标相关的 normal/build 依赖和当前 Rust 标准库许可；需要 `rust-docs` 与已缓存的依赖，缺少正文或精确 upstream 证据会拒绝准备。它包含 build/proc-macro 依赖超集，不是 exact linked SBOM。`contracts/third-party-notices` 保存版本绑定的补充正文与来源；旧 runner 必须保留其原 toolchain 说明，不能把当前 toolchain 说明当成旧二进制的构建证明。

Vite 使用内置 `build.license` 生成 frontend 的许可与版权正文。完整构建先运行 Vite，再由同一 notices 生成器验证并收集 frontend 正文，缺失／空正文会拒绝；原生测试可使用只含 tracked README 的资源目录，不能把它当作发行包。两者随 App 放在 `Contents/Resources/third-party-notices`；保留子目录与 HTML 相对引用。生成正文不进入 Git。第三方声明不改变 Lintel 原创代码、文档或独立资产的许可选择；Clawd 等第三方身份／资产的授权仍需独立核对。

独立 CLI 分发包也必须随包保留第三方声明。`package-cli.mjs --notices` 复用上述完整目录，冻结其有限正文与 license 文件并纳入各 archive 的 metadata／checksum；应先核对声明对应实际 executable 的构建 toolchain，不能拿新的声明补证未知旧构建。未传声明的既有合成打包调用仍只用于本地测试。

## 候选首次使用验收

使用与候选架构相符的 Mac，先记录确切 DMG、source revision、App 版本与 macOS 版本。以下是可重复执行的原生操作流程；最新候选的实际结果见 [当前状态](current-state.md)，已有 CLI／Chromium 检查不代替 native 验收。

1. **核对所测的包。** 本地候选只读挂载后，将 App 复制到新的测试目录，保留原 DMG 与旧安装。真实下载／Gatekeeper 另在干净 Mac 上检查；记录原始系统提示，遇到阻止时停止，不移除 quarantine 或关闭安全检查。未来官网下载验收须先有公开下载授权，再核对实际下载字节与这份已验收候选一致。
2. **准备六份假原件。** 在独立子 shell 中执行 [quickstart 第 1 步](quickstart.md#1-一次明确的子-shell-边界)，只创建 fixture，不继续 CLI 登记或提交。保留打印出的 `ROOT`，不要指向真实 Claude root。将下列 `APP` 替换为刚复制的候选绝对路径，在同一子 shell 经 macOS LaunchServices 后台启动；完全退出测试 App 后用同一命令重开，保持相同 HOME／state：

   ```sh
   APP="/path/to/test-copy/Lintel.app"
   env -i PATH=/usr/bin:/bin:/usr/sbin:/sbin /usr/bin/open -g -n -a "$APP" \
     --env "HOME=$ROOT/home" --env "CFFIXED_USER_HOME=$ROOT/home" \
     --env "LINTEL_TEST_HOME=$ROOT/home" --env "LINTEL_STATE_DIR=$ROOT/state" \
     --env "LINTEL_BROWSER_STATE=$ROOT/browser-state" --env "TMPDIR=$ROOT"
   ```

   `open` 返回只说明启动请求已发出，须另外观察真实窗口与所选环境；`-g` 不主动切前台。不要同时启动同一测试 state 的多个副本。这些变量隔离本次指定的 HOME／state，不构成 OS sandbox，也不代替干净 Mac 的首次安装验收。quickstart 的 `ROOT` 已解析为实际路径；输出路径也保留这一形式，不能改填指向符号链接的简写路径。
3. **从空白工作区登记。** 添加已有环境，名称使用 `Synthetic native trial`，配置目录填写 `$ROOT/claude-root` 展开的绝对路径；只登记，不运行 Claude。确认当前主机是本机、环境是这份 fixture。
4. **精确预览。** 进入“工作保全”，选择“只生成工作归档”，启用“精确到原件”，仅保留 `CLAUDE.md`、`projects/example/memory/MEMORY.md` 与 `projects/example/session.jsonl`，取消 `NOT_SELECTED.md`。输出使用 `$ROOT/carried.age` 的绝对路径。容量预检与批准前计划都应恰好三件；计划阶段密文尚不存在，settings 与占位凭据不在选择中。
5. **批准一次并核对原任务。** 输入只用于此次合成试用的口令，批准当前计划；保存原 ID／hash。等待原任务真正完成，不能把已接收当完成；结果不确定时只查询原 ID，不重新提交。回执应只有归档结果，无新环境／新 root；按 [quickstart 第 6 步](quickstart.md#6-确认源原件保留没有新建根) 的字节检查核对全部六份原件，包括未选文件。
6. **退出后重开读包。** 完全退出 App，再以同一 HOME／state 启动。在“记录与恢复”核对原任务，再从“会话与资料”的“独立加密包”入口填入密文绝对路径并重新输入口令。应恰好列出三份所选原件，指令、记忆和会话原文均可阅读；不批准迁入、启动或清理。独立 HOME／state 读包另用 [quickstart 第 7 步](quickstart.md#7-在另一份独立-state-里-inspect--read) 验证，不能把同一 state 重开当作独立安装。
7. **分别记结果。** 记录启动、登记、精确选择、预览、批准、原任务完成、退出重开、独立文件读包及六份原件核对的实际结果。自动化无窗口、无法输入或截图失败时标未验证，并保存错误；不据此改写为产品成功或失败。原始截图、fixture、state、口令与内部记录留在本地，不上传整个候选 evidence 目录。

完成主路径后，另外观察中英菜单、Day／Night／System 与动态默认，打开帮助核对文档 revision／实际链接，并手动检查一次更新；未配置 feed 应明确显示限制，不能报“已是最新版”。从“终端与 Agent”选择本平台交付 CLI，核对候选与 HOME／UID／state，审阅后复制交接包并确认无新 job。外置宗卷的系统访问授权单独记录，处理方式见 [CLI 入口](agents.md#从-app-接到终端或-agent)。

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

### 手动下载形态（无 updater feed）

未启用 App 内更新的 Preview 使用 `lintel.app-release-manual/1` 记录：字段与签名记录相同，但**没有** `pubkey`、`archive_path`、`signature_path` 与 `archive_name`，每个平台只声明 `dmg_path` 与 `dmg_name`。`configure`／`build` 拒绝手动记录（它们固定签名构建配置）；`prepare` 按记录 schema 分流，对手动形态只做 DMG 的冻结身份与 SHA-256 字节核验，输出 `release.json`（schema `lintel.app-release-manual-public/1`），**不生成** `updates/` feed。`verify-public` 同样按 schema 分流，只核对公开 DMG 字节。

`prepare-site.mjs --app-manual <release.json>` 要求公开字节已核验，投影下载按钮与“未启用 App 内更新”说明，但不写任何 updater feed；`--app-release` 与 `--app-manual` 不能同时使用，一个站点 payload 只承载一种下载叙事。手动形态不关闭或放宽任何现有签名校验；办完签名后仍走原 `lintel.app-release/1` 路径启用 updater。

```sh
node scripts/app-release.mjs prepare \
  --record candidate-packages/app-release-manual-input.json \
  --out candidate-packages/app-prepared-manual
```

## 有发行授权时的激活顺序

每次外部发行与托管仍需精确授权。签名 updater 发行先完成签名／公证与候选安装验收，再创建相应版本的 GitHub Release，按记录中的公开名字上传各架构 DMG、`.app.tar.gz` 和 `.sig`；保持版本／tag／文件名不可变，不复用已发行版本。明确获准的未签名开发者 Preview 则走上方 manual 记录与手动下载路径，公开说明 Apple 信任与干净 Mac 首装的未验收状态，不生成 updater feed。

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

## 统一候选构建与一致性 gate（未发布）

`scripts/prepare-candidate.py` 需要 Python 3.11+（标准库 `tomllib`），提供 `build` 与 `verify` 两个本地模式。先提交并核对准确的 clean 40 位 SHA；输出目录必须尚不存在且位于源码 checkout 外。当前 matrix 入口只接受 native macOS arm64；Rust 三个 targets、Node、cargo-zigbuild、Zig 0.16.0 与当前 toolchain 的完整 rust-docs 声明须预先可用。Linux 两目标复用既有 cargo zigbuild 路径，Zig 版本须与受审 native-runtime 声明匹配；脚本在创建输出前检查并拒绝缺项，不安装工具链。示例路径和 SHA 均须替换为实际选定值：

```sh
python3 scripts/prepare-candidate.py build \
  --revision REPLACE_WITH_EXACT_CLEAN_40_HEX_SHA \
  --out ../lintel-candidate-build
# 需要 macOS App/DMG 时，在上述 build 命令加 --app。
python3 scripts/prepare-candidate.py verify \
  --receipt ../lintel-candidate-build/build-receipt.json \
  --ci-plan ../lintel-ci/plan.json \
  --ci-results ../lintel-ci/results.json \
  --ci-evidence ../lintel-ci/evidence \
  --out ../lintel-candidate-verified.json
```

`build` 使用 fresh isolated source clone 与 fresh target，固定 locked 构建三个 CLI 平台和配套组件，记录输入与实际最终归档。`verify` 重新读取实际 tar member bytes，不信任包内自报 checksum；核对完整三平台、组件、版本、准确 SHA 与 canonical full CI plan／结果／证据一致性（包含 clean checkout）。它不接受把 reduced plan 改名为 full。带 App 的构建另核对实际 DMG 中的 App 与候选资源。

这些是本地一致性证据：build receipt 和 CI JSON 的来源没有独立认证，不能充当可信 CI 身份证明。`c7680f0` 已实际完成 fresh clone／fresh target 的三平台 CLI（macOS arm64、Linux x86_64／arm64 musl）及 macOS App／DMG 构建，依赖下载缓存复用，并非全空缓存构建。最终候选的完整 source SHA、matrix 与 full CI 状态由[当前状态](current-state.md)按准确 SHA 记录。后续 proof-only 文档提交不改变已构建候选的 source revision。App 安装、干净 Mac 信任与 Release 验收仍未完成。公开 App 0.1.1／CLI 0.1.0 仍为旧发布线，本轮 `main` 未发布。构建、校验、原生安装验收、签名／公证、Release、下载与 feed 激活仍是独立步骤，均不由此脚本自动授予或执行。
