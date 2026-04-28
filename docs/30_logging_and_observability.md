# 30 日志与可观测性

本文档承载：

- Agent Trace 与运行 Logs 的边界
- 日志存储位置与生命周期
- LLM 请求 / 响应 / 流式 chunk 的还原规则
- Agent 模式各环节判断数据的记录规则
- 应用异常事件与清理机制

日志默认面向开发调试：本地完整保留 LLM 原始请求 / 响应，保证基本可读性；API Key、Authorization header、Provider secret、代理认证等凭证永远在写入前脱敏。

---

## 1. 两类记录

### 1.1 Agent Trace

Agent Trace 是 Agent 世界的回合决策追踪，回答"这个回合为什么这样演化"。

- 以 `scene_turn_id` 为主轴。
- 随 World 保存，属于世界调试 / 回放 / 回滚定位数据。
- 记录 Active Set、Dirty Flags、Layer 2 派生、CognitivePass、验证、结果规划、提交等 Agent 运行步骤。
- 不作为 LLM 输入来源，不参与剧情状态判断。

### 1.2 运行 Logs

运行 Logs 是应用观测日志，回答"应用运行时发生了什么"。

- 以 `request_id` / `event_id` 为主轴。
- 记录 LLM 调用、Provider 错误、异常事件、性能、存储错误、清理任务等运行事实。
- ST 模式只写运行 Logs，不写 Agent Trace。
- Agent 模式的运行 Logs 可关联 `world_id` / `scene_turn_id` / `trace_id`，但不替代 Agent Trace。

### 1.3 关系规则

- 一次 Agent LLM 调用同时产生：
  - `llm_call_logs`：还原该次 Provider 请求 / 响应。
  - `agent_step_traces` 引用：说明该调用属于哪个回合、角色、Agent 步骤。
- 程序化判断优先写 Agent Trace；异常和 Provider 运行状态优先写运行 Logs。
- 回滚 Agent 回合时，世界状态回退；运行 Logs 默认保留为审计记录，不随剧情回滚物理删除。
- 日志系统只能观察系统，不得驱动 Simulation / Cognitive / Presentation 的业务逻辑。

---

## 2. 存储位置

采用"全局 + 世界内"布局：

```text
./data/
├── logs/
│   ├── app_logs.sqlite
│   └── archives/
└── worlds/
    └── <world_id>/
        ├── world.sqlite
        ├── traces/
        └── assets/
```

规则：

- `./data/logs/app_logs.sqlite` 保存全局运行 Logs：应用启动、设置变更、ST 模式 LLM 调用、Provider 错误、异常事件、清理任务。
- `./data/worlds/<world_id>/world.sqlite` 保存 Agent 世界状态、Agent Trace、世界内 LLM 调用日志。
- `./data/worlds/<world_id>/traces/` 预留给大型 trace 附件、导出文件或压缩归档；第一版不强制使用。
- `./data/logs/archives/` 预留给全局日志归档；第一版清理以删除旧运行 Logs 为主。
- 应用启动时由存储层创建缺失目录；业务模块不得自行拼接外部日志路径。

---

## 3. LLM 调用日志

所有 `chat` / `chat_structured` / `chat_stream` 调用必须经过 Provider logging wrapper 写日志。Provider 实现只负责真实 API 调用，不直接写日志。

### 3.1 LogContext

```rust
pub struct LogContext {
    pub mode: LogMode,                 // st / agent
    pub world_id: Option<String>,
    pub scene_turn_id: Option<String>,
    pub character_id: Option<String>,
    pub trace_id: Option<String>,
    pub llm_node: LlmNode,             // STChat / SceneInitializer / SceneStateExtractor / CharacterCognitivePass / OutcomePlanner / SurfaceRealizer
    pub api_config_id: String,
    pub request_id: String,
}
```

### 3.2 保存内容

- `request_json`：Provider 发送前的真实请求体；凭证字段必须脱敏。
- `response_json`：Provider 返回的原始响应、结构化结果或错误响应。
- `api_config_id`、provider、model：本次调用实际使用的 API 配置与模型。
- `schema_json`：`chat_structured` 使用的 JSON Schema。
- `stream_chunks`：流式输出的原始 chunk，按序保存。
- `assembled_text`：按 chunk 顺序直接拼接后的完整文本。
- `readable_text`：在不改写内容含义的前提下整理为可读段落；只用于查看，不替代原始响应。
- `status`：`started` / `succeeded` / `failed` / `cancelled`。
- `latency_ms`、`token_usage`、`retry_count`、`error_summary`。

流式日志必须尽量还原原貌：原始 chunk 永远保留顺序；可读文本只是派生展示层，不允许回写为业务响应。

---

## 4. Agent Trace 记录点

Agent Trace 记录 Agent 模式下"程序如何判断"与"模型如何输出"。

每个步骤至少记录：

- `step_name`：运行步骤名。
- `step_status`：`started` / `skipped` / `succeeded` / `failed` / `fallback_used`。
- `input_summary`：结构化输入摘要。
- `output_summary`：结构化输出摘要。
- `decision_json`：关键判定值，例如 dirty flag、active set、observable facet、validation failure。
- `linked_request_id`：如该步骤调用 LLM，则关联 `llm_call_logs.request_id`。
- `error_event_id`：如该步骤触发异常事件，则关联 `app_event_logs.event_id`。

典型记录点：

- SceneInitializer 请求、响应、假设列表、阻止项、确认需求、解析失败与重试。
- SceneStateExtractor 请求、响应、解析失败与重试。
- UserInputDelta 应用结果。
- 身体 / 资源 / 状态 / 冷却的机械演化摘要。
- Active Set + Dirty Flags 的触发项与跳过原因。
- EmbodimentResolver、SceneFilter、KnowledgeAccess、InputAssembly 的关键 Layer 2 派生摘要。
- CharacterCognitivePass 输入输出、schema 校验、程序修复、OutcomePlanner 兜底触发。
- Validator 各规则结果。
- OutcomePlanner 的 God-read 输入域、候选 StateUpdatePlan、EffectValidator 裁剪摘要与物理后果。
- SurfaceRealizer 请求、流式响应拼接、NarrativeFactCheck 结果。
- StateCommitter 的提交记录、rollback patch 和 trace 关联。

---

## 5. 异常事件日志

应用运行异常统一写 `app_event_logs`。事件级别：

- `debug`
- `info`
- `warn`
- `error`
- `fatal`

事件必须包含：

- `event_id`
- `level`
- `event_type`
- `message`
- `source_module`
- `created_at`

有上下文时必须关联：

- `request_id`
- `world_id`
- `scene_turn_id`
- `trace_id`
- `character_id`

典型事件：

- Provider 请求失败、限流、超时、取消。
- LLM 输出 schema 校验失败。
- 程序容错修复失败。
- OutcomePlanner 启用、兜底使用、God-read 权限域异常或失败。
- NarrativeFactCheck 失败。
- SQLite 写入失败或事务回滚。
- Agent 世界回滚。
- 日志清理触发、完成、失败或需要用户确认。

---

## 6. 定期删除机制

默认开启按大小清理运行 Logs，上限为 **1GB**。清理只针对运行 Logs；Agent Trace 默认随 World 保留。

### 6.1 触发时机

- 应用启动后后台检查一次。
- 应用运行中最多每日检查一次。
- 写入日志后如果超过 1GB，只标记需要清理，由后台任务执行，禁止阻塞聊天或 Agent 回合。

### 6.2 清理范围

自动清理：

- 全局 `debug` / `info` 级运行事件。
- 全局旧流式 chunk。
- 全局 LLM 原始 request / response 中最旧且无 Agent Trace 关联的记录。
- 已完成的清理任务、性能统计、非关键状态事件。

默认不自动清理：

- Agent Trace。
- Agent 世界内与 `scene_turn_id` 关联的关键回合记录。
- `warn` / `error` / `fatal` 事件，除非空间仍超限且这些事件已经足够旧。
- 仍被 `state_commit_records.trace_ids` 引用的记录。

### 6.3 清理顺序

1. 删除全局 Logs 中最旧的 `debug` / `info` 事件。
2. 删除全局流式 chunk，仅保留 `assembled_text` / `readable_text`。
3. 删除全局 LLM 原始 request / response，保留元数据、耗时、状态、错误摘要。
4. 删除最旧的 `warn` 事件。
5. 如仍超限，只记录 `retention_limit_exceeded` 事件并提示用户处理；不自动删除 Agent Trace。

### 6.4 长期未更新 World 提示

如果某个 World 超过 30 天未更新，且其日志 / trace 体积较大，系统只产生提示事件，询问用户是否清理或导出，不自动删除。

提示必须包含：

- `world_id`
- 最后更新时间
- 当前日志 / trace 估算体积
- 可选操作：打开日志管理、导出、手动清理非关键运行 Logs

---

## 7. 脱敏规则

写入日志前必须脱敏：

- `api_key`
- `Authorization`
- `Proxy-Authorization`
- `x-api-key`
- Provider secret/token 字段
- 用户配置中的代理用户名 / 密码

脱敏发生在日志写入前，不能只依赖 UI 展示层隐藏。脱敏结果保留字段名和值类型，例如 `"Authorization": "[REDACTED]"`。

用户 prompt、角色卡内容、世界书、Agent 输入输出默认视为本地调试资料，不做内容级裁剪；后续可增加 `none` / `metadata` / `summary` / `full` 日志级别。

---

## 8. 验证要求

- ST 模式 LLM 调用只写全局运行 Logs。
- Agent 模式任意 `scene_turn_id` 能查到完整 Agent Trace。
- Agent Trace 能跳转到对应 LLM request / response。
- 流式响应能查看原始 chunk，也能查看拼接后的可读文本。
- API Key、Authorization header、Provider secret 不会进入数据库。
- 全局 Logs 超过 1GB 后后台清理旧运行日志。
- 普通清理任务不会删除 Agent Trace。
- 30 天未更新且日志较大的 World 只产生提示，不自动删除。
