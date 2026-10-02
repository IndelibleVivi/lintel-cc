# Linux runner 与网络能力边界

Linux 上的 core / `lintel request` 使用 Rust，与 macOS 调用相同的 planner、journal 与 receipt。`lintel-egress` 提供 loopback TCP proxy，操作见 [网络文档](../../docs/network.md)；SSH 管理端见 [远程文档](../../docs/remote.md)。

此目录当前没有已实现、可部署的 privileged namespace helper，也没有在获授权 Linux root 环境中的强约束验证。不能把 proxy 运行中、`HTTPS_PROXY` 已设置、`PrivateNetwork` 出现在 service 配置中或某条回显请求成功，显示为“进程无法直连”。

真正交付 namespace 强约束需要：精确选定受控服务、独立 network namespace、只通向 owned proxy 与显式批准的必要例外、无网络管理 capabilities 的目标、独立于 GUI 的 owned-rule 恢复入口；同时证明不修改宿主默认路由、不清空宿主 firewall、不影响 SSH 或其他服务。已有服务的迁入 / 重启须显式授权，不能假定任意运行中的 PID 已迁入。

必须在独立获授权环境补齐 IPv4、IPv6、UDP、DNS、子进程、继承 sockets、权限逃逸、proxy 崩溃、GUI/SSH 断开与卸载恢复的实际负例。当前 source 没有执行这些操作，也没有安装用户 service、启用 linger 或改变真实主机网络策略。
