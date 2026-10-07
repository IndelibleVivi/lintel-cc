# Lintel 的视觉身份

Lintel 的图形由承重横梁、两根支撑和右下方的光标构成。横梁承住配置、状态与恢复的复杂性；开口保留可用空间。光标放在开口之外，与 `lintel_` 和 CLI 呼应，避免把开口画成一张脸。

图形来源于为本项目提供的 identity kit。现有纸上晨光／夜里月光界面继续使用；这是身份资产接入，不是整套界面换皮。

## 当前源与用法

| 表面 | 权威源 | 当前行为 |
| --- | --- | --- |
| App 图标 | [`apps/desktop/src/brand.svg`](../apps/desktop/src/brand.svg) | 1024 master；暖纸容器、陶土横梁与光标、中性色支撑 |
| Tauri 图标 | [`apps/desktop/src-tauri/icons`](../apps/desktop/src-tauri/icons) | 使用提供的 PNG、ICNS、ICO exports；打包路径由 `tauri.conf.json` 指定 |
| App 内小标记 | [`Brand()`](../apps/desktop/src/ui.tsx) | 32-unit 无底色图形；横梁／光标跟随 accent，支撑跟随 text，适配 Day / Night |
| 左上角字标 | [`App.tsx`](../apps/desktop/src/App.tsx)、[`styles.css`](../apps/desktop/src/styles.css) | 保留现有 serif `lintel_`、分割线与 2.6 秒慢闪光标；reduced motion 下静止 |
| 可编辑展示资产 | [`assets/identity`](../apps/desktop/assets/identity/README.md) | 透明、单色、outlined 字标、横向组合、16/24/32 optical 与可选 dark 图标 |
| 官网 | [`apps/site`](../apps/site/README.md) | 暖纸／夜色、原生 outlined 字标与细密字符山湖；六能力索引、湖岸书页／过桥插画、四步保全流程与继续工作寄语；favicon 复用 32 px optical；页尾 Clawd 与 App 共用四腿动画／收星 runner，仅主动开始后运动 |
| Preview 展示 | [`assets/preview`](../assets/preview/README.md)、[`prepare-preview-art.mjs`](../scripts/prepare-preview-art.mjs) | 从 canonical SVG 字标／标记与所提供概念图的本地插画背景组合暖纸／夜色 banner 和 1280×640 分享图；SVG 引用仓库背景，PNG 是便携导出；未上传社交设置 |

文字中的产品名为 **Lintel**，展示字标为 **`lintel_`**，CLI 为 **`lintel`**。outlined 字标用于官网与导出材料，App 侧栏保留自己的 serif 字体。字标的橙色字脚压住最后一个 `l` 的下部，再向右伸出一小截；橙色底边与 `l` 的基线贴齐，接合处直切，不在下方露出原色字脚；单独字标按横向组合的比例修正画布和 cursor，避免旧画布留白把它推开。静态小标记的光标不闪烁；侧栏字标的慢闪是保留的交互风格。

## 保留的关系

- **纸面外壳：** 温暖纸色、serif 标题、细线与留白。
- **系统工具：** sans 控件、monospace 证据、明确的 plan／approval／receipt。
- **像素伙伴：** Clawd、风景与 playroom 保持独立层次，不作为 App 身份或信任标记。

Day 的 paper / support / beam 是 `#FAF9F5` / `#3D3D3A` / `#C16A47`；Night 为 `#24231F` / `#EAE5DA` / `#E0A485`，由现有 CSS tokens 管理。陶土色不是通用状态色；成功、警告与破坏性操作继续使用功能颜色。

保留右侧光标和空开口，不拉伸、加立体斜面／光晕、额外屋顶／盾牌／锁，不把光标移到开口中央。16/24/32 px 展示优先采用 optical 版本；更大的图标使用 master。主 App 图标在 OS 明暗主题中都保留浅色容器；dark 资产只是可选展示版本。

## 验收边界

官网与桌面保持同一身份，但官网是独立介绍页面。首屏沿用完整山湖构图，紧接六个任务的能力索引，标题直接进入对应指南，能力与限制就近说明。湖岸书页、带包过桥与原任务延续继续讲精确保全、预览批准、阅读和继续工作；中英文试用入口保持可达。原件阅读用原生 details，内容显式标为合成示例；页面没有 App 假窗口、执行模拟或编辑草稿。小游戏、阅读示例和主题不调用本机执行能力。

Capabilities 使用开放的双栏索引，手机改为单栏，沿用 serif 标题、细线与原色字脚；不把每个条目包进模拟 App 面板。How it works 用“只保全、不清理”的合成例子按四步说明对象、预览、批准执行和原任务核对，随后说明 App／CLI 共用 core 与 SSH 目标主机执行的分工。四步正文立即可读；额外的开放线稿旅程可逐步点看原件、冻结计划、加密包与原任务核对，不执行操作或产生成功回执。手机图注独立排成可读文字，不随 SVG 整体缩小。

大幅插画采用概念图的细密字符纹理、分层森林与湖面；本地 hero 背景经 imagegen 移除文字与品牌，再叠 canonical SVG。正文两幅新景是同一视觉语言的插画延展，品牌字形不由 imagegen 重画。App 原四幅 72×24 风景仍由桌面 `clawd-landscapes.ts` 维护，官网不再抽取这个小画 composer。

桌面叙事将文字排进风景留白，手机先排可读文字再裁取完整主体；不把桌面整体缩小。首屏字符光点、水纹与景深回应鼠标和触摸，原生字标与 live heading 不跟着摇动；月亮、Clawd 和字尾橙线藏着本页彩蛋，发现后可点开星图，键盘也可在首屏空白处输入 LINTEL。自然滚动只触发风景进入与细线阅读进度，不接管滚轮，正文和试用入口立即可用。无 JavaScript 可展开原生阅读示例，“停下风景”同时停止自动风景、反光、淡入与线稿动效；reduced motion 时它们静止、按钮和步骤仍直接反馈，主动小游戏保留独立的开始／暂停控制。浏览器检查、源码完成、native 验收和用户审美接受分别记录；通过检查不能替代视觉接受。

源码接入、打包、实际桌面辨识度和用户审美接受是不同事实。发布前仍需检查 macOS Dock、Finder、Spotlight、Cmd–Tab、安装器／About，以及正式 Linux launcher／window icon；同时看明暗桌面背景和小尺寸。提供 assets 或通过 build 不代表这些环境已经验收。当前实测状态见 [current-state](current-state.md)。

## 产品基线的页面构图

默认首页保留 Clawd、原创时段问候与留白，采用简洁的 composer 形态；“开始一项任务”再展开六任务选择，上次目标与待核对原任务保持轻量入口；专业工作区持续显示目标、主机和完整 root。阅读器使用索引、阅读与独立审阅稿区域，小窗口通过面板切换；Agent 页按选 CLI、核对、审阅、复制排列。批准标题／目标与操作区保持可达，长路径完整换行。身份 tokens 和资产保持上表权威。

实现入口及给 Selen 的组件索引在[桌面约定](desktop.md#本轮页面与组件索引)。当前源码、候选与实际原生观察分别记录于[基线验收](specs/product-baseline-status.md)；静态构图和构建不能替代实际界面验收。

首页问候由 `apps/desktop/src/homeGreetings.ts` 维护：清晨、午后、傍晚、深夜各十四句原创文案，统一使用 Clawd 名称。每次打开 App 选一句，在当前访问期间保持稳定；优先级为特殊日期、指定整分钟、约 1% 稀有文案、普通时段。特殊日期只在当天第一次打开显示（包括 10-13），localStorage 仅记最近一次已显示的完整本地日期；存储不可用时跳过日期彩蛋。选择在 main.tsx 的 React mount 前完成，不受 StrictMode 重复初始化影响。不读取账号、配置或活动历史，不联网；不模拟模型回复。伙伴文字不代替操作事实、影响说明或明确审批。工作区底栏独立于正文滚动，侧栏帮助与设置保留固定位置。

主题、选中状态和控件色值直接更新，不做文字／背景颜色过渡，保证动画暂停的后台原生窗口仍然可读。阴影反馈与伙伴动作保留，reduced motion 继续静止。
