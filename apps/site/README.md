# Lintel website

Lintel 的独立静态官网源码。页面介绍 Lintel 0.1.0 Preview / 开发预览，提供中英文源码试用入口；网站本身仍是本地网页候选，尚未部署、绑定域名或提供正式 App 下载。产品阶段、示例交互、构建候选与实际验收分别说明。

## 本地预览

在 repository 根目录运行：

```sh
python3 -m http.server 4317 --bind 127.0.0.1 --directory apps/site
```

打开 `http://127.0.0.1:4317/`。不需要 npm install 或 build。浏览器通过 HTTP 加载原生 ES modules；不要直接双击 HTML 作为完整交互的启动方式。停止预览用 Ctrl+C。

## 内容与边界

- `index.html`：中文产品介绍、示例工作流程、产品边界、Preview 状态与文档链接；主导航、首屏与页尾提供首次试用入口，手机宽度也保留导航。开发手记链接中文／英文 quickstart 和 English README，页尾保留“终端与 Agent”。main 链接标为最新开发说明，当前验收指向 current-state。
- `styles.css`：响应式暖纸／夜色主题。系统字体，无远程字体请求。
- `site.mjs`：三个工作流程的明确示例、计划／回执切换和主题偏好。工作示例按选中原件生成只归档的计划／结果，并说明独立阅读、交接稿与迁入的后续边界；不读取本机数据、不执行实际计划。
- `clawd-game.mjs`、`clawd-game.css`：官网与 App 共用的唯一跳跃小游戏实现，主动开始后四腿交替迈步，起跳收脚、落地压低，跳过石头／书本并收集星星（每颗 +25 分）。空格／↑ 跳跃、P 暂停，仅在跑道聚焦时接管这些按键；也可点击／触摸跑道或按钮。移出焦点、窗口失焦、页面隐藏或跑道离屏会暂停。
- `assets/favicon.svg`：从 [32 px optical mark](../desktop/assets/identity/small/lintel-32.svg) 复制，品牌变化时一起更新。首屏图形遵循[视觉身份](../../docs/visual-language.md)。Clawd 身体几何沿用桌面 `Clawd.tsx`，四腿独立绘制；形象属于 Anthropic。

只有主题与游戏最高分保存在当前浏览器的 localStorage（`lintel.site.theme`、`lintel.site.clawd.best`）。存储不可用仍可浏览和玩游戏，成绩仅保留在当前页面。没有账号、表单、analytics、CDN、后端或 native bridge；点击外部文档后才导航到 GitHub。reduced motion 下字标不闪，游戏背景不移动、无尘点与落地缩放；主动游戏的迈腿／跳跃保留，未开始与暂停时画面静止。

页面文案以 [README](../../README.md)、[current-state](../../docs/current-state.md) 与[人类操作指南](../../docs/operator-guide.md)为事实依据。更新产品能力、版本、发行下载或验证范围时，同步核对官网文案；不把 SPEC 目标直接写成已支持。当前未生成社交分享图片、未设置尚不存在的 canonical 域名。

## 验证

```sh
python3 tests/verify.py --checks site-game-test,site-ui
```

`site-game-test` 使用 Node 内置 runner，无依赖，覆盖四腿动作、起跳、落地输入缓冲、碰撞、星星计分与不同帧率，纳入默认 synthetic 检查。独立 `site-ui` 使用仓库既有 Playwright／Chromium，启动临时 loopback 静态服务并使用隔离 browser context；需要先具备 [browser tests](../../extensions/browser/package.json) 的开发依赖与浏览器，或通过 `PLAYWRIGHT_MODULE` 指向已有安装。不接触真实 Claude 或个人 browser profile。

App 的 `ClawdPlayroom.tsx` 直接导入本模块与 CSS，不复制另一套 runner；传入原 App 的成绩 key，React effect cleanup 调用返回的 disposer，停止 RAF、移除监听／observer 并保存当前成绩。其独立 `clawd-app-ui` 检查需要先完成 `npm --prefix apps/desktop run build`，覆盖原入口、风景册切换、卸载／重开、焦点和 App 三种主题。

本地通过的 browser journey 证明该浏览器里的布局与交互，不代表 Safari、真实手机或公开托管环境已验收。网站源文件可由普通静态服务器托管；具体 hosting、域名和公开上线在独立授权后确定。
