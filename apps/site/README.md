# Lintel website

Lintel 的独立静态官网源码，面向 Claude Code 用户，以简洁的个人工具说明环境整理与工作保全。页面是一片**开阔的字符风景**加**简短的产品叙事**：首屏是本地 raster 插画与原生字标；正文以 [六任务映射](../../contracts/task-catalog.json) 对应的六项能力索引开始，通过插画与原生合成阅读示例引出四步流程和边界说明，并保留中英文 quickstart 与文档入口，不嵌入模拟 App。完整站点已在 [lintel.page](https://lintel.page/) 上线，由 Cloudflare Pages 同源托管；不提供正式 App 下载。准确部署状态见 [current-state](../../docs/current-state.md)，更新程序见[官网托管说明](../../docs/site-hosting.md)。

## 本地预览

在 repository 根目录运行：

```sh
python3 -m http.server 4317 --bind 127.0.0.1 --directory apps/site
```

打开 `http://127.0.0.1:4317/`。预览源页面不需要 npm install 或 frontend build；实际短片与分享图由 [`prepare-site.mjs`](../../scripts/prepare-site.mjs) 供应到完整静态目录，命令见[官网托管说明](../../docs/site-hosting.md)。浏览器通过 HTTP 加载原生 ES modules；不要直接双击 HTML 作为完整交互的启动方式。停止预览用 Ctrl+C。

## 页面合同

- **header 导航** — `#capabilities`、`#how-it-works`、`#try` 三个页内锚点与 GitHub 链接；每个 `href="#…"` 都指向存在的 id。
- **hero `#top`** — 本地 raster 山湖叠加原生 `.brand-wordmark-svg` 和梁柱标记；缩小字标、拉开字标／Preview／标题关系、下移风景，保留较多安静纸面。完整橙脚承住最后一个 `l`，微圆角下面的原色字脚由 canonical mask 去掉；向右仍只伸出短截。header／footer 用同一 glyph 配独立、较细的 `_`，按 2.6 秒节奏慢闪，hero 橙脚静止；reduced motion 下全部静止。中文字句、产品说明与中英文试用入口紧接画面，正文没有覆盖风景的卡片。
- **hero 短片入口 `#film-open`** — 既有 CTA 旁一个安静的 `.film-open` 链接（“看 20 秒短片 · 英文”）。无 JavaScript（或模块加载失败）时它就是一个指向同源 `media/lintel-intro.mp4` 的普通链接；`film.mjs` 挂载后由同一链接经 `showModal()` 打开 `#film-dialog`。短片为英文，无中文配音，打开后再按原生播放控件开始。
- **`#film-dialog`** — 原生 `<dialog>` 全屏深色 cinematic 表面：真实原生 `<video controls playsinline preload="none" poster>`（初始不下载、不开播）、可见的 `.film-close`、accessible `aria-label`，焦点移入并由原生 modal 保证被遮住的页面不可点击；`Esc`／关闭按钮停止声音并把焦点还给入口，重开从干净状态开始。含一条同源英文 `<track kind="captions">`（`film.vtt`，四句，时间取自生产源）。桌面 1440 与手机 390／320 均保持 16:9、无溢出、close 可见。不自动滚动、不加 storage／analytics／外部 CDN／YouTube iframe。
- **`#capabilities`（正文第一段）** — 首屏之后是六项能力索引：六个开放的 `article[data-capability]` 条目（无外框卡片），其 `data-capability` 必须等于 [task-catalog](../../contracts/task-catalog.json) 的六个稳定任务 id（`reduce_egress`、`preserve_work`、`repair_cleanup_retire`、`browser_profile`、`ssh_remote`、`recover_results`）。每个条目标题直接链接对应权威文档／anchor（保护方案→`docs/operator-guide.md#protect`，保全→`#work`，清理→`#cleanup`，浏览器→`docs/browser.md`，SSH→`docs/remote.md`，恢复→`#recovery`），正文给出具体结果，另有一行“就近限制”。旧 `.capability-line` 已退休，不再出现。
- **`#journey`** — 插画旅程的起点（正文叙事由此展开）。
- **`#keep`、`#sample-pages`、`#review`** — 中文产品叙事与合成阅读示例：精确保全、原生 `<details id="sample-pages">` 阅读示例（带 `<summary>`，鼠标／键盘开合，链接会直接展开并定位到原件，内含**显式合成**的 `CLAUDE.md`、`MEMORY.md`、`session.jsonl` 片段，无 reader 编辑／写入、剪贴板、crypto 或“生成工作包”按钮）、预览→批准→原任务结果，内容不等待滚动动画。
- **`#how-it-works`** — 用“只保全、不清理”的三份合成原件示例解释四步（明确对象 → 预览 → 批准执行 → 原任务核对），`.execution-entries` 说明 macOS App／独立 CLI／SSH 三个入口（App 与 CLI 共用执行核心；SSH 由目标主机 runner 执行），`.execution-boundary` 说明浏览器与官网的独立有限边界；末尾 `.thread-line` 保留 `accepted ≠ completed · 只核对，不重放`。四步 `li` 与全部文本是**服务器渲染**的权威说明；其上的交互式“工作包旅程”（`#workflow-trail`）为近旁解释，缺 JavaScript 时保持隐藏。
- **`#continue`** — 结束寄语（往前走，不必丢掉来路）。
- **`#try`** — 作用范围与限制、尚未提供正式下载等边界、0.1.0 Preview 状态，以及可用的中文／English quickstart 与六个 docs 真链接。
- **`#clawd-game`** — 主动开始的小游戏，仅在跑道聚焦时接管按键，离屏／失焦／页面隐藏会暂停。

阅读顺序即为页面顺序：`#top` → `#capabilities`（六项能力索引）→ `#journey` → `#keep`（含 `#sample-pages`）→ `#review` → `#how-it-works`（四步流程）→ `#continue` → `#try` → `#clawd-game`。

## 内容与边界

- `index.html`：中文产品介绍、六项能力、四步流程、三段叙事、阅读示例、边界与文档链接。三处字标与三处标记是 identity source 的**生成投影**，由 [`prepare-preview-art.mjs`](../../scripts/prepare-preview-art.mjs) 写入带标记的 projection blocks；调整 canonical identity 后重新运行该脚本，不手写重复字形。
- `styles.css`：响应式暖纸／夜色主题、整宽首屏风景与叙事版式。系统字体，无远程字体请求。夜色风景投影为较柔和的暖灰；两幅正文的 `.story-character` 用原 raster 的轮廓 clip 保留 Clawd 的暖橙身体和黑眼睛，避免整张反色把眼睛翻白。该层复用原素材，不另生一张插画；`site-world.mjs` 将 SVG 的 slice／meet 同步到风景的 cover／contain，手机裁景亦保持对齐。**正文、链接与所有文本默认即可阅读**；JS 只作用于风景插画——在允许 motion 时让 `.story-world` 的图景与 hit areas 一起随滚动淡入（`scene-motion`／`arrived`），reduced motion 和无 JavaScript 时风景直接显示；无论是否滚到，正文和链接都不会隐藏或延迟。
- `site.mjs`：唯一页面入口，维护主题、原生阅读示例，并挂载风景交互、工作包旅程和共享小游戏；离开页面时释放各自监听与绘制。页面不连接 native bridge 或业务 API，模块与素材均从同一个静态站点加载。
- `film.mjs`：官网短片播放入口的唯一 owner（`export function mountFilmPlayer(root)`）。它把 markup 里的原生链接升级为 `#film-dialog` 触发：复用入口文案作 dialog 名称、只按需（`preload="none"`）使影片仅在显式 `play()` 时下载、按 `open` 同步维护 `aria-hidden`（关闭时 inert，打开时绝不因异步 `toggle` 而停留在隐藏态）、聚焦对话框，并在 `close`／`cancel` 停止暂停、复位播放位置、把焦点还给入口。所有监听（入口 click、close click、dialog `cancel`／`close`）都是具名回调，disposer 会逐一移除并关闭仍打开的 dialog；`pagehide`（非 bfcache）经 `site.mjs` 调用该 disposer。它不读写 storage／clipboard，不自动播放、不预取，只认同源 `media/lintel-intro.mp4`；`showModal` 不可用时保留真实原生链接入口并返回 no-op。`film.vtt` 是四句英文 captions（时间 4.95／9.9／13.85／17s，与生产源一致）。
- `site-world.mjs`：首屏本地字符光点、轻微 pointer 景深，logo 与 live heading 固定；月亮与 header 共用主题。点水面／Enter 产生三圈逐步扩散、渐隐的细水纹，每次都有独立出生时间，2.6 秒后清理；最多保留六组，一个到期 timer，暂停、离屏与 reduced motion 不会使水纹永久堆积。三幅画里的 Clawd 可主动摸摸：湖边一个小心、书桌旁一只纸船、桥上几颗小光点；字尾会短暂露出小眼睛，所有反应自行收掉，重复点击替换旧反应。没有可见彩蛋说明、发现计数、纸条引导或游戏招募。首屏空白处 `LINTEL` 切换隐藏星图，避开链接、控件、编辑区与小游戏。正文 hit areas 按原图坐标与实际 cover／contain 及手机裁景对齐。低调的暂停按钮停止自动动效，reduced motion 时只给静态反应；离屏／hidden 停止 ambient RAF，回来后继续。彩蛋／暂停只在本页内存；正文与试用入口立即可用。
- `clawd-game.mjs`、`clawd-game.css`：官网与 App 共用的**唯一**跳跃小游戏实现，主动开始后沿用 App 的连续四条短直腿轮廓，不单独画脚；整体轻微弹跳、起跳伸展与落地压低，跳过石头／书本并收集实际高低变化、可跳到的星星（每颗 +25 分）。空格／↑ 跳跃、P 暂停，仅在跑道聚焦时接管这些按键；也可点击／触摸跑道或按钮。移出焦点、窗口失焦、页面隐藏或跑道离屏会暂停。不新增第二个 runner。
- `workflow-trail.mjs`：`#how-it-works` 中四步“工作包旅程”解释交互的 canonical owner（`export function mountWorkflowTrail(host)`）。host 为 `#workflow-trail`（初始 `hidden`），内含四个 `button[data-trail-step="0".."3"]`（原生 click／Enter／Space 选定，`aria-pressed` 始终只有一个 `true`）、一个 `button[data-trail-next]`（下一步，末步回到第 1 步并改文案）与 `p[data-trail-caption][role=status]`（清晰中文解释）。`host.dataset.step`（`"0".."3"`）驱动 SVG／CSS 图景。说明始终**显式合成**：不读取电脑、不生成计划、不接受批准、不执行，也不把点击渲染成真实成功回执或权限批准。模块不加 timer／RAF／storage／clipboard／network／native bridge；SVG 状态与过渡由 `styles.css` 所有，reduced motion 与风景暂停时步切换仍可用。初始化完成后取消 host 的 `hidden` 并返回 disposer（页面卸载时移除全部监听并复位）。四个 `li` 可被加上 `data-highlight="true"` 作为近旁解释，文本本身不变。
- `assets/lintel-landscape.png`、`assets/lintel-landscape-night.png`：从所提供概念图编辑的本地插画背景；只有风景，没有文字或品牌几何。身份插画与分享图的来源见 [Preview assets](../../assets/preview/README.md)。
- `assets/lintel-keep.png`、`assets/lintel-crossing.png`：叙事章节使用的本地风景插画。
- `assets/favicon.svg`：从 [32 px optical mark](../desktop/assets/identity/small/lintel-32.svg) 复制，品牌变化时一起更新。Clawd 身体几何沿用桌面 `Clawd.tsx`，四条短直腿与身体连成同一个轮廓；形象属于 Anthropic。
- `media/lintel-intro.mp4`、`media/lintel-film-poster.jpg`：首屏短片与海报，**不进 Git**，由 [`prepare-site.mjs`](../../scripts/prepare-site.mjs) 从明确指定的媒体目录供应到完整 payload 的 `media/`。缺媒体的源码预览只能检查页面，实际播放另以 `LINTEL_SITE_MEDIA_DIR` 指定真实媒体验证。

只有主题与游戏最高分保存在当前浏览器的 localStorage（`lintel.site.theme`、`lintel.site.clawd.best`）。存储不可用仍可浏览和玩游戏，成绩仅保留在当前页面。没有账号、表单、应用 analytics、外部 CDN 脚本、业务后端或 native bridge；点击外部文档后才导航到 GitHub。站点与电影由 Cloudflare Pages 同源提供，托管平台处理普通 HTTP 请求；页面不上传本机状态或用户工作。工作包旅程与风景彩蛋不写入持久存储，风景暂停不阻止主动开始小游戏。reduced motion 下字标不闪、风景静止（不淡入、直接显示），首屏“世界”画面静止但按钮仍有反馈，`#workflow-trail` 仍可切换；主动游戏的手动跳跃保留，装饰性弹跳／伸展停用，未开始与暂停时画面静止。

页面文案以 [README](../../README.md)、[current-state](../../docs/current-state.md) 与[人类操作指南](../../docs/operator-guide.md)为事实依据。更新产品能力、版本、发行下载或验证范围时，同步核对官网文案；不把 SPEC 目标直接写成已支持。

## 验证

```sh
python3 tests/verify.py --checks site-game-test,site-ui
```

`site-game-test` 使用 Node 内置 runner，无依赖，覆盖连通轮廓／整体动作、起跳、落地输入缓冲、碰撞、真实星星高低与可达性、一次计分和不同帧率。独立 `site-ui`（`tests/site_ui_journey.mjs`）使用仓库既有 Playwright／Chromium，启动临时 loopback 静态服务并使用隔离 browser context，覆盖：header 导航锚点与“无死锚点”、**真实**点击与键盘 Enter 导航（断言 hash 与目标 section 落到正常滚动位置）、hero 原生字标与尾巴几何、日夜 hero 背景加载、`#capabilities` 六个条目的 `data-capability` 与 [task-catalog](../../contracts/task-catalog.json) 六个 id **动态**对齐、每个条目链接到真实存在的文档／anchor（并读盘确认文件存在）且就近限制包含预期边界词、`#how-it-works` 四步与 App／CLI／SSH 三个 `execution-entries`（共用 core、目标主机执行、官网不执行）与 `accepted ≠ completed`／只核对不重放、**`#workflow-trail` 四步交互**（初始第 1 步、click 第 3 步后迅速改选的 latest state、键盘 Enter／Space、next 前进并在末步回到第 1 步、说明中的合成与“原 ID／不重放／原件保留”边界、绝无伪造批准或成功回执）、`#keep`／`#review` 两张正文风景 `img` 滚入后 decode（`complete`／`naturalWidth`）、真实浏览器合成截图中的橙色身体／深色双眼（1440 cover、900 contain、390／320 手机裁景），回到日光时额外角色层隐藏、`#journey` 起点与 `#keep`／`#review`／`#continue`／`#try`、原生 `details#sample-pages` 的鼠标／键盘开合与合成片段、无 JavaScript 时六个能力与四步仍**实际可见且有正常布局盒**（不只是 DOM 计数）且 `#workflow-trail` **保持 hidden**、键盘焦点可达、390 与 320 宽度无溢出且六个条目与四步的标题＋正文保持可见布局且 `#workflow-trail` 仍可切换、reduced motion 下 `#workflow-trail` 仍可用、无错误／无外部请求，并断言旧操作 demo 路径与旧 `.capability-line` 已退场。有限 asset map 仅服务网站素材与这两个新模块。它需要先具备 [browser tests](../../extensions/browser/package.json) 的开发依赖与浏览器，或通过 `PLAYWRIGHT_MODULE` 指向已有安装。不接触真实 Claude 或个人 browser profile。首屏检查使用真实鼠标、Enter／Space 和触摸：昼夜共享、三幅画中的 Clawd 与字尾短暂回应、触发点焦点保持、星图快捷键、水纹真实像素扩散／渐隐／到期清空、实际 canvas 随时间变化、暂停／reduced motion 时像素静止、离开首屏停止与返回继续。后台暂停分支通过合成 hidden／visible 输入覆盖；headless 多 tab 仍报告 visible，不将它表述为真实系统切后台证据。

`site-ui` 另含**短片播放入口**检查，且这些都发生在同一个**仍存活**的 desktop context 内、`context.close()` **之前**：无 JavaScript 时入口仍是可见、可点、指向同源 `media/lintel-intro.mp4`（`autoplay=false`）的原生链接且无预开 dialog；增强后 dialog 初始关闭且 `aria-hidden="true"`、`<video>` 具 `controls`／`playsinline`／`preload="none"`、无 `autoplay`、有一条 captions `<track>`、close 可见且有 `aria-label`；**初始页面加载**（尚未点入口）不请求影片；显式 Enter 打开后 dialog `open`、同步清除 `aria-hidden`、仍 `paused` 且仍未请求影片（脚本化 open/close 不会触发下载，只有真实 `play()` 才会）；`Esc` 与 close 按钮停止、复位并把焦点还给入口、恢复 `aria-hidden="true"`；被遮住的页面在 modal 打开时不可点击；无 iframe／第三方视频主机；1440／390／320 下 video 保持 16:9、close 在视口内且无横向溢出。**非 bfcache 的 `pagehide` 是该 context 最后一次功能操作**（shared `site.mjs` 据此 dispose 播放器），其后不再操作已卸载的播放器。真实媒体字节独立：当且仅当设置 `LINTEL_SITE_MEDIA_DIR`（内含 main 构建的 `lintel-intro.mp4` 与 `lintel-film-poster.jpg`）时，在独立 context 里断言打开不自动播放、不下载，随后**显式 `play()`** 才发出真实影片请求并前进，且元数据为 **1920×1080、约 20s**（`19.9–20.3`，容许单 AAC frame padding），关闭后暂停且复位。未提供时有限 map 用**显式合成**字节仅验证请求时机；这**不**等于真实播放，也不把假视频写成真影片。

可选设置 `LINTEL_SITE_QA_DIR=qa/preview-review` 保留本地 desktop（1440）与 mobile（390）的 day／night、full-page，以及 `section-capabilities`／`section-how-it-works`／`section-keep`／`section-review`／`section-continue` 在 1440 与 390 下的截图，便于人工验收；full-page 先滚过全页、等两张 lazy 正文风景 decode 且淡入完成后才截，避免未加载区域留透明空白；`qa/` 不进入 Git。

App 的 `ClawdPlayroom.tsx` 直接导入本目录的 `clawd-game.mjs` 与 CSS，不复制另一套 runner；传入原 App 的成绩 key，React effect cleanup 调用返回的 disposer，停止 RAF、移除监听／observer 并保存当前成绩。独立 `clawd-app-ui` 检查需要先完成 `npm --prefix apps/desktop run build`，覆盖原入口、风景册切换、卸载／重开、焦点和 App 三种主题：

```sh
python3 tests/verify.py --checks clawd-app-ui
```

本地 browser journey 证明隔离 Chromium 中的布局与交互，不代表 Safari 或真实手机验收。托管后的 HTTP／CSP／真实播放另行检查，具体结果见 current-state；owner 的视觉接受独立于这些 checks。每次生产更新仍需当前部署授权。

`index.html` 的 `<head>` 声明 canonical／`og:url` `https://lintel.page/`、同源分享图 `https://lintel.page/assets/lintel-social-preview.png` 与 `twitter:summary_large_image`；分享图由打包入口供应。`robots.txt`／`sitemap.xml` 使用正式域名，`404.html` 为未知路径提供独立错误页面，`_headers` 维护托管 CSP 与响应 headers。

## 未来 App 下载与更新资料

[`prepare-site.mjs`](../../scripts/prepare-site.mjs) 可从已核对公开二进制字节的 `lintel.app-release-public/1` 记录生成首装 DMG 链接、`app-downloads.json` 和 `updates/preview.json`／`stable.json`。版本与 feed 来自同一记录；二进制留在固定 GitHub Releases 仓库，Pages payload 不包含 App archives。无记录时保持源码 Preview 入口。两个已激活通道需同次传入两个记录，feed 使用 no-cache；具体命令、签名区别与先核对字节后激活的顺序由 [App 更新指南](../../docs/app-updates.md) 管理。本次只是源码准备，未创建正式下载或更改线上站点。
