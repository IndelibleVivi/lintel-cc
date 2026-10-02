# 执行权威与状态

本图是 SPEC 的目标架构。具体已接通与未验收部分见 [当前状态](current-state.md)，图中组件存在不代表整条旅程已通过。

```mermaid
flowchart LR
    UI[Mac 桌面界面] -->|计划请求| Core[Rust core · 唯一计划权威]
    CLI[Linux CLI / TUI] -->|同一协议| Core
    Core -->|持久记录后执行| Journal[目标主机的计划与任务 journal]
    Core -->|批准的精确字段| FS[配置与工作文件]
    Core -->|实例与任务 ID| Native[Native Messaging host]
    Native -->|浏览器发起连接| Ext[已配对 profile 扩展]
    Ext -->|原生 API| Storage[该 profile 站点数据与权限]
    Ext -->|执行前后状态| BJ[扩展持久 journal]
    UI -->|OpenSSH / JSON stdin| Remote[远端 runner · 独立 core 与 journal]
    Launch[明确启动上下文] -->|仅选择的网络路径| Proxy[Loopback CONNECT 通道]
    Core --> Launch
    Proxy -->|不解密 TLS| Destination[批准的目的地 / 上游代理]
```

界面持有草案与展示状态，目标 core 拥有计划与文件操作授权；浏览器扩展拥有原生浏览器操作结果。SSH 连接状态与业务任务状态分别保存。网络通道只证明经过它的连接；进程直接出站约束是独立的平台能力。

关键状态为 `planned → accepted → executing → verifying → completed / partially_completed / failed / needs_reconciliation`。`accepted` 只在 journal 可持久查询后成立。不确定的副作用查询原任务，不重放破坏性动作；恢复以字段归属与当前值为依据。
