# Lintel 文档

简体中文 · [English](index.en.md) · [仓库首页](../README.md)

按眼前要做的事情读。第一次使用不需要先读完整 SPEC，也不需要掌握协议字段。

## 第一次来

[第一次使用](start-here.md)带你打开界面并完成一次合成资料归档；[基本概念](concepts.md)解释主机、配置目录、计划与工作包。需要命令行的完整可核对示例时，再走 [CLI 完整试用](quickstart.md)。

## 已经知道要做什么

| 目标或问题 | 去哪里 |
| --- | --- |
| 设置保护方案（含 27 字段逐项／批量编辑）、工作保全、清理重建、恢复结果与配方 | [按目标操作](operator-guide.md) |
| 使用 CLI、安装独立候选、交接给 Agent | [Agent CLI](agents.md) |
| App 页面、语言、桌面使用 | [桌面指南](desktop.md) |
| 扩展安装、配对、浏览器重启后的清理收尾 | [浏览器](browser.md) |
| SSH 主机、远端 runner、断线后找回任务 | [远程](remote.md) |
| IPv4／IPv6 出口测试与宿主网络配置 | [网络](network.md) |
| 遥测目标、通道规则与测试结果 | [遥测](telemetry.md) |
| 暂停和恢复具体 systemd 服务 | [服务](services.md) |
| App 版本检查、更新与发行准备 | [App 更新](app-updates.md) |
| 不知道结果该怎么看、遇到错误或入口不可用 | [排错指南](troubleshooting.md) |

## 理解实现或参与开发

先看[架构与源码地图](architecture.md)，再按需要进入 [Core / CLI](core.md)、[协议](../contracts/protocol.md)和[统一验证](verification.md)。[视觉语言](visual-language.md)维护身份与展示资产；[官网托管](site-hosting.md)维护独立网站的发布流程。

目标与证据分开查：[SPEC](SPEC.md)和[产品基线](specs/product-baseline.md)描述目标；[当前状态](current-state.md)列出当前交付和已验证范围；[基线证据](specs/product-baseline-status.md)、[完整验收](ACCEPTANCE.md)与[候选历史](candidate-history.md)保留详细依据。

中文为产品文档的主版本，配套英文页维护相同命令、默认值和操作边界。工程规格与验收记录尚有中文专页；英文入口会明确标注。

当前是 **0.1.0 Preview**：本地发行准备候选、源码能力、公开下载与已启用更新通道要分开核对。不存在公开 Release 下载或正式签名版本。

页面语言与方向保持稳定；如果结果是等待、部分完成或 `needs_reconciliation`，从保留的原 ID 进入[排错指南](troubleshooting.md)，不要为了重新阅读而删除 Lintel state 或工作包。
