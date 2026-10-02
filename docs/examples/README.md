# 合成领域样例

两个 JSON 文件用于理解方案与操作计划怎样表达，不是生产配置、正式 JSON Schema、已验证规则或可以执行的脚本。

`policy.synthetic.json` 表达“减少外发”的独立控制、保留集合、功能依赖、生效验证和字段级恢复。实际的支持版本没有填造，适配状态为 `not-tested`。

`reset-plan.synthetic.json` 表达一个本地 Claude Code 目标与浏览器 profile 的重建计划。使用 `example.test` 合成站点和符号目标，不含真实凭据、路径或主机身份。计划需要明确授权；已经成功但失去 ACK 的不可逆步骤先核对结果，不盲目重放。

所有例子均设置 `executable: false` 与 `authorization.granted: false`。实现者应据主 Spec 建立严格 schema、版本规则和真实 planner；不要写一个把这些示例直接翻译为 shell 命令的解释器。
