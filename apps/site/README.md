# Lintel website

Lintel 的独立静态官网源码。页面是一片**开阔的字符风景**加**简短的产品叙事**：首屏是本地 raster 插画与原生字标，正文用中文讲清“把工作收好，再往前走”，并保留中英文 quickstart 与文档入口。合成原件用原生阅读示例展示，不嵌入模拟 App。它仍是本地网页候选，尚未部署、未绑定域名、不提供正式 App 下载；产品阶段、页面合同和实际验收分别说明。

## 本地预览

在 repository 根目录运行：

```sh
python3 -m http.server 4317 --bind 127.0.0.1 --directory apps/site
```

打开 `http://127.0.0.1:4317/`。不需要 npm install 或 build。浏览器通过 HTTP 加载原生 ES modules；不要直接双击 HTML 作为完整交互的启动方式。停止预览用 Ctrl+C。

## 页面合同

- **hero `#top`** — 本地 raster 风景（`assets/lintel-landscape.png`，夜色是 `assets/lintel-landscape-night.png`），叠加**原生** `.brand-wordmark-svg` 字标（header／hero／footer 共三处）与梁柱标记。字标末尾的橙色小尾巴压在最后一个 `l` 的字脚上，底线与 `l` 对齐，再向右伸出很短一截。header／footer 的字标光标缓慢闪动，hero 里静止；`prefers-reduced-motion` 下全部静止。中文标语、产品说明与中英文试用入口紧接画面，正文没有覆盖风景的卡片。
- **`#journey`** — 正文起点。
- **`#keep`、`#review`、`#continue`** — 三段中文产品叙事，讲清精确保全、预览批准（**accepted ≠ completed**；原 ID 查询只核对、不重放）与继续工作，内容不等待滚动动画。
- **`#sample-pages`** — 阅读示例本身就是原生 `<details>`（`id="sample-pages"`），带 `<summary>`，可用鼠标或键盘开合；正文链接会直接展开并定位到原件，内含**显式合成**的 `CLAUDE.md`、`MEMORY.md`、`session.jsonl` 片段。它是叙事阅读示例：没有 reader 编辑／写入、剪贴板、crypto，也没有“生成工作包”按钮。
- **`#try`** — 作用范围与限制、尚未提供正式下载等边界，以及可用的中文／English quickstart 与六个 docs 真链接。
- **`#clawd-game`** — 主动开始的小游戏，仅在跑道聚焦时接管按键，离屏／失焦／页面隐藏会暂停。

## 内容与边界

- `index.html`：中文产品介绍、三段叙事、阅读示例、边界与文档链接。三处字标与三处标记是 identity source 的**生成投影**，由 [`prepare-preview-art.mjs`](../../scripts/prepare-preview-art.mjs) 写入带标记的 projection blocks；调整 canonical identity 后重新运行该脚本，不手写重复字形。
- `styles.css`：响应式暖纸／夜色主题、整宽首屏风景与叙事版式。系统字体，无远程字体请求。**正文、链接与所有文本默认即可阅读**；JS 只作用于风景插画——在允许 motion 时让 `.story-art` 随滚动淡入（`scene-motion`／`arrived`），reduced motion 和无 JavaScript 时风景直接显示；无论是否滚到，正文和链接都不会隐藏或延迟。
- `site.mjs`：主题切换、风景插画进入视口时的淡入，并挂载共享小游戏。它不隐藏正文或链接，不读取本机数据、不执行实际操作、仅加载同一静态站点的模块与图形，不发起业务 API 请求。
- `clawd-game.mjs`、`clawd-game.css`：官网与 App 共用的**唯一**跳跃小游戏实现，主动开始后四腿交替迈步，起跳收脚、落地压低，跳过石头／书本并收集星星（每颗 +25 分）。空格／↑ 跳跃、P 暂停，仅在跑道聚焦时接管这些按键；也可点击／触摸跑道或按钮。移出焦点、窗口失焦、页面隐藏或跑道离屏会暂停。不新增第二个 runner。
- `assets/lintel-landscape.png`、`assets/lintel-landscape-night.png`：从所提供概念图编辑的本地插画背景；只有风景，没有文字或品牌几何。身份插画与分享图的来源见 [Preview assets](../../assets/preview/README.md)。
- `assets/lintel-keep.png`、`assets/lintel-crossing.png`：叙事章节使用的本地风景插画。
- `assets/favicon.svg`：从 [32 px optical mark](../desktop/assets/identity/small/lintel-32.svg) 复制，品牌变化时一起更新。Clawd 身体几何沿用桌面 `Clawd.tsx`，四腿独立绘制；形象属于 Anthropic。

只有主题与游戏最高分保存在当前浏览器的 localStorage（`lintel.site.theme`、`lintel.site.clawd.best`）。存储不可用仍可浏览和玩游戏，成绩仅保留在当前页面。没有账号、表单、analytics、CDN、后端或 native bridge；点击外部文档后才导航到 GitHub。页面不采集或上报任何数据，也不把本机状态上传。reduced motion 下字标不闪、风景静止（不淡入、直接显示）；主动游戏的迈腿／跳跃保留，未开始与暂停时画面静止。

页面文案以 [README](../../README.md)、[current-state](../../docs/current-state.md) 与[人类操作指南](../../docs/operator-guide.md)为事实依据。更新产品能力、版本、发行下载或验证范围时，同步核对官网文案；不把 SPEC 目标直接写成已支持。

## 验证

```sh
python3 tests/verify.py --checks site-game-test,site-ui
```

`site-game-test` 使用 Node 内置 runner，无依赖，覆盖四腿动作、起跳、落地输入缓冲、碰撞、星星计分与不同帧率。独立 `site-ui`（`tests/site_ui_journey.mjs`）使用仓库既有 Playwright／Chromium，启动临时 loopback 静态服务并使用隔离 browser context，覆盖：hero 原生字标与尾巴几何、日夜 hero 背景加载、`#keep`／`#review` 两张正文风景 `img` 滚入后 decode（`complete`／`naturalWidth`）、`#journey` 起点与 `#keep`／`#review`／`#continue`／`#try`、原生 `details#sample-pages` 的鼠标／键盘开合与合成片段、无 JavaScript 可读性、保留的主题与主动游戏（开始／暂停／继续／失焦暂停；不含重开）、390 与 320 宽度无溢出、reduced motion、无错误／无外部请求，并断言旧操作 demo 路径已退场。它需要先具备 [browser tests](../../extensions/browser/package.json) 的开发依赖与浏览器，或通过 `PLAYWRIGHT_MODULE` 指向已有安装。不接触真实 Claude 或个人 browser profile。

可选设置 `LINTEL_SITE_QA_DIR=qa/preview-review` 保留本地 desktop（1440）与 mobile（390）的 day／night、full-page，以及三个 story section 在 1440 与 390 下的截图，便于人工验收；full-page 先滚过全页、等两张 lazy 正文风景 decode 且淡入完成后才截，避免未加载区域留透明空白；`qa/` 不进入 Git。

App 的 `ClawdPlayroom.tsx` 直接导入本目录的 `clawd-game.mjs` 与 CSS，不复制另一套 runner；传入原 App 的成绩 key，React effect cleanup 调用返回的 disposer，停止 RAF、移除监听／observer 并保存当前成绩。独立 `clawd-app-ui` 检查需要先完成 `npm --prefix apps/desktop run build`，覆盖原入口、风景册切换、卸载／重开、焦点和 App 三种主题：

```sh
python3 tests/verify.py --checks clawd-app-ui
```

本地通过的 browser journey 证明该浏览器里的布局与交互，不代表 Safari、真实手机或公开托管环境已验收，也不代表 owner 对视觉的审美接受。网站源文件可由普通静态服务器托管；具体 hosting、域名和公开上线在独立授权后确定。
