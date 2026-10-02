# 桌面使用与界面约定

Lintel 桌面使用 Tauri 2 + React。`src/App.tsx` 管理环境选择、草案与计划展示，`src/api.ts` 调用 shared core；BrowserPanel / NetworkPanel 走各自有限的 native commands。主数据来自执行器，界面不合成保护成功、网络强约束或凭据清空状态。

## 使用路径

1. 首页选择环境与“减少外发 / 保持功能”，点击橙色箭头生成预览。需要调整 Remote Control 时，先进入“调整方案”。
2. 对照预览中的“将修改 / 将保留 / 实际步骤”，批准一次执行。
3. 结果逐项显示；可以进入记录、预览恢复，或在找到 Claude 程序时请求打开它。恢复会重新检查后续编辑。
4. 首页“工作内容”展开本地状态；“环境详情”还包含外发、网络通道与启动来源。完整路径在详情中查看和复制。
5. “清理与重建”当前是新根建立与工作内容加密迁入。必须输入并确认归档口令；旧登录与旧目录未清理，结果会明确显示部分完成。
6. 点击侧栏底部“本地工作空间”打开设置、深浅主题和浏览器配对。浏览器 mutation 仍需要扩展内确认；开发安装见 [浏览器指南](browser.md)。

草案按环境 ID 分开保存在当前桌面 webview 的 localStorage；切换环境不会串用方案。顶部/操作区与确认页显示明确目标。任务记录位于 core state，关闭界面不删除记录；关闭应用会停止它启动的代理通道。未完成任务需要查询原 ID，GUI 不自动再次提交。

## 当前视觉方向

这一阶段采用 Claude 式布局关系：轻量侧栏与环境列表、居中的首页标题、单一操作框及紧凑快捷入口。暖纸背景、陶土色主动作、衬线首页标题与普通 sans 控件共同构成当前主题。Lintel 保留自己的名称与门楣 mark，未使用 Claude 的字体文件、logo 或图片资产。

首页操作框是结构化环境/方案选择器，不是聊天输入框，也不调用模型。技术诊断在可展开详情中，保留发现路径。不要重新引入大插画 hero、指标卡墙或重复的环境展示块；新的功能应进入相应对象和操作路径。

`src/styles.css` 是颜色、尺寸、布局与交互状态的唯一样式真源。`src/brand.svg` 是原始应用 icon，使用 Tauri icon generator 生成 `src-tauri/icons`；这些 generated assets 不代表其他平台已支持。亮色/暗色/跟随系统可选，尊重 reduced motion，焦点以轻量内侧指示显示。新视觉候选的渲染、功能验证与用户审美认可分别记录。

## 开发与验证

```sh
cargo build -p lintel-runner
cd apps/desktop
npm ci
npm run dev:synthetic
npm run build
npm run desktop:build
```

合成模式将所有 core 路径指向新建临时目录，界面持续显示“测试空间”；native browser/network 不在网页里伪造成功。实际 Tauri 应用才具有两个模块的 native command。测试桥只允许 loopback、匹配 Origin/Host 与明确 command，不接受任意路径登记到测试根之外。

主窗口 1120×760，最小 900×640；可收起侧栏。browser render 不能代替 WebKit/native runtime 验收。当前 macOS arm64 本地 `.app` 构建成功，正式签名、公证、升级、菜单栏、定时漂移提醒、GUI SSH 与非开发者扩展安装仍未完成。
