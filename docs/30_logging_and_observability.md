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

SQLite schema 边界：

- `world.sqlite` 使用 [14_agent_persistence.md](14_agent_persistence.md) 中的完整 Agent schema，`llm_call_logs` / `app_event_logs` 可通过外键或逻辑 ID 关联 `agent_sessions`、`world_turns` 与 `turn_traces`。
- 全局 `app_logs.sqlite` 只创建日志相关表：`config_snapshots`、`llm_call_logs`、`llm_stream_chunks`、`app_event_logs`、`log_retention_state`。这些表与 World 内同名表保持字段兼容，但不得设置指向 `agent_sessions`、`world_turns`、`turn_traces` 的外键。
- 全局日志表中的 `world_id` / `session_id` / `scene_turn_id` / `trace_id` 只是脱敏索引字段，用于从全局错误跳转到某个 World，不表示全局库拥有该 World 的 Agent 状态。

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

多模态补充：

- `request_json` 可记录发送给 Provider 的 `image` / `document` 内容块结构、`attachment_id`、`mime_type`、传输方式与文件句柄类型。
- 日志中不得保存本地绝对文件路径。
- 对 inline base64 发送的图片 / PDF，日志默认不保存完整 base64 正文；只保存 `attachment_id`、`mime_type`、字节数、sha256、`transport=inline_base64` 与必要的 Provider 字段壳。
- 对 `ProviderFile` 发送的图片 / PDF，可记录远端 `file_id` / `file_uri`，但它只是运行时句柄，不代表本地真源。
- 若用户最初通过 URL 导入附件，日志中 `original_source_url` 只可写脱敏或截断后的 provenance，不得把带签名的原始 URL 原样落库。

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
- AttributeResolver 的属性修正来源、effective/displayed、ManaExpressionTendency、运行时 ManaExpressionMode、intentionality、presence pressure、tier/delta 与异常修正摘要。
- EmbodimentResolver、SceneFilter、KnowledgeAccess、InputAssembly 的关键 Layer 2 派生摘要。
- CharacterCognitivePass 输入输出、schema 校验、程序修复、OutcomePlanner 兜底触发。
- Validator 各规则结果。
- OutcomePlanner 的 God-read 输入域、候选 StateUpdatePlan、EffectValidator 裁剪摘要与物理后果。
- SurfaceRealizer 请求、结构化响应、used_fact_ids 与 NarrativeFactCheck 结果。第一版 SurfaceRealizer 不使用裸 `chat_stream`；未来若引入 `chat_structured_stream`，再记录结构化流式片段。
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

默认开启按大小清理全局运行 Logs，默认上限为 **1GB**，实际值来自 `./data/settings/app_runtime.yaml` 编译后的 `RuntimeConfigSnapshot.log_retention`。自动清理只针对全局运行 Logs；Agent Trace 与 World 内回合相关 LLM Logs 默认随 World 保留，只能在用户确认后清理或导出。

### 6.1 触发时机

- 应用启动后后台检查一次。
- 应用运行中最多每日检查一次。
- 写入日志后如果超过当前快照中的 `global_size_limit_bytes`，只标记需要清理，由后台任务执行，禁止阻塞聊天或 Agent 回合。
- 日志写入路径只读内存中的 retention 快照和近似体积计数，不扫描文件、不解析配置、不打开额外配置查询。

### 6.2 清理范围

自动清理范围仅限全局 `app_logs.sqlite`：

- 全局 `debug` / `info` 级运行事件。
- 全局旧流式 chunk。
- 全局 LLM 原始 request / response 中最旧且无 Agent Trace 关联的记录。
- 已完成的清理任务、性能统计、非关键状态事件。

默认不自动清理：

- Agent Trace。
- Agent 世界内与 `scene_turn_id` 关联的关键回合记录。
- Agent 世界内与 `scene_turn_id` / `trace_id` 关联的 LLM 调用日志。
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

`30 天` 和“体积较大”的阈值同样来自 `RuntimeConfigSnapshot.log_retention.world_stale_prompt`，默认值分别为 30 天和全局上限的一定比例。修改这些值后从下一次后台 retention 检查开始生效。

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

## 8. 日志页面规划

日志页面是只读调试工具，入口为 `/logs`，对应前端 `src/views/LogsView.vue`。页面只能读取、筛选、展示、导出或触发经过用户确认的清理任务；不得修改 ST 会话、Agent World、Trace、canonical Truth、Provider 配置或运行时分支。

### 8.1 页面目标

日志页面首版回答四类问题：

- 某次 LLM 调用实际向 Provider 发送了什么、收到了什么、耗时和错误是什么。
- 某个 Agent 回合为什么这样运行，Trace 中哪些步骤关联了哪些 LLM 请求。
- 全局运行 Logs 与 World 内 Logs 各占用多少空间，是否触发清理提示。
- Provider 错误、schema 校验失败、SQLite 错误、清理任务等异常事件发生在哪个上下文。

页面不作为业务控制台：不能重放请求、不能把日志响应写回聊天、不能从 Trace 直接修复状态、不能绕过 ST request assembly 或 Agent runtime。

### 8.2 信息架构

页面采用三栏工作区：

1. 左侧筛选栏：日志来源、类型、级别、状态、时间范围、Provider、模型、World、Session、Turn、Trace 与 request 搜索。
2. 中央结果列表：按时间倒序展示日志项、LLM 调用项、Trace 项和清理提示项。
3. 右侧详情面板：展示当前选中项的脱敏详情、关联跳转、JSON / stream chunk 查看器和安全操作。

顶部动作栏固定展示：

- 当前范围：`全局 Logs` / `World Logs` / `Agent Trace` / `全部索引`
- 搜索框：支持 `request_id`、`event_id`、`trace_id`、`scene_turn_id`、`world_id` 精确搜索
- 时间范围选择：最近 1 小时、24 小时、7 天、自定义
- 刷新按钮
- 导出按钮
- 清理管理入口

日志来源必须明确标记：

- `global`：`./data/logs/app_logs.sqlite`
- `world`：`./data/worlds/<world_id>/world.sqlite` 中的运行 Logs
- `trace`：`world.sqlite` 中的 `turn_traces` / `agent_step_traces`

### 8.3 筛选与列表

筛选条件分为基础筛选和上下文筛选。

基础筛选：

- `record_kind`：LLM 调用、stream chunk、应用事件、Agent Trace、清理任务、配置快照
- `level`：debug / info / warn / error / fatal
- `status`：started / succeeded / failed / cancelled / skipped / fallback_used
- `mode`：ST / Agent / app
- `provider`、`model`、`api_config_id`
- 时间范围与关键词

上下文筛选：

- `world_id`
- `session_id`
- `scene_turn_id`
- `trace_id`
- `request_id`
- `character_id`
- `llm_node`

列表项最少展示：

- 创建时间
- 类型图标与来源标签
- 主标识：`request_id` / `event_id` / `trace_id`
- 摘要：Provider、模型、节点、事件类型或 step name
- 状态 / 级别徽标
- 耗时、token usage、chunk 数量或 step 数量
- 关联上下文：World、Turn、Trace、Character

列表滚动必须虚拟化或分页，避免一次性读取大量 JSON 正文。默认只取元数据和摘要；request / response / chunk 正文按需加载。

### 8.4 详情面板

详情面板按记录类型切换 tabs：

- `摘要`：状态、上下文、耗时、token、错误摘要、配置快照 ID。
- `Request`：脱敏后的 `request_json`，支持 JSON tree 与 raw text。
- `Response`：脱敏后的 `response_json`、`assembled_text`、`readable_text`。
- `Stream`：按序展示 `llm_stream_chunks`，支持 chunk index、created_at、raw JSON 与 delta 摘要。
- `Schema`：`chat_structured` 的 `schema_json` 与解析状态。
- `Trace`：关联 `agent_step_traces` 的 step 列表、输入输出摘要、decision_json 与跳转。
- `事件`：关联 `app_event_logs`，展示异常事件、清理事件或 retention 提示。

敏感内容展示规则：

- 日志写入层已经完成凭证脱敏；UI 仍必须把 Request / Response / Stream 视为本地敏感调试资料。
- 首次展开原始 JSON 时显示轻量提示，说明其中可能包含用户 prompt、角色卡、世界书和 Agent 私有上下文。
- 复制按钮只复制当前已显示的脱敏内容。
- 不提供“显示未脱敏内容”的入口。

### 8.5 Trace 跳转

Trace 与 Logs 的双向跳转规则：

- 从 `agent_step_traces.linked_request_id` 跳到对应 `llm_call_logs.request_id`。
- 从 LLM 调用详情跳回 `trace_id` / `scene_turn_id` 所在 Trace step。
- 从 `error_event_id` 跳到对应异常事件。
- 从 World / Turn 上下文跳到 Agent 工作区或 Agent 聊天页时，只改变查看位置，不触发回放、回滚或重新生成。

Trace 详情按运行顺序展示 step：

1. SceneInitializer
2. SceneStateExtractor
3. UserInputDelta
4. Simulation / Layer 2 派生
5. Active Set / Dirty Flags
6. CharacterCognitivePass
7. ReactionWindow
8. Validation
9. OutcomePlanner
10. SurfaceRealizer
11. StateCommitter

每个 step 展示 `step_status`、`input_summary`、`output_summary`、`decision_json`、`linked_request_id` 和 `error_event_id`。大 JSON 默认折叠。

### 8.6 容量、清理与导出

日志页面的容量管理分三层：

- 全局容量概览：`app_logs.sqlite` 当前估算大小、默认 1GB 上限、下一次 retention 检查时间、最近清理结果。
- World 容量概览：每个 World 的 `world.sqlite` 日志 / Trace 估算大小、最后更新时间、是否满足 30 天未更新提示条件。
- 选中范围详情：当前筛选结果的记录数、估算大小、可导出范围。

自动清理只作用于全局运行 Logs，并遵守第 6 节顺序。页面可提供手动入口：

- 立即运行全局 retention 检查。
- 导出当前筛选结果。
- 对长期未更新 World 发起“导出”或“清理非关键运行 Logs”确认流程。

手动清理必须显示影响摘要：

- 将删除的记录类别与数量。
- 是否会删除 stream chunks。
- 是否会删除原始 request / response，只保留元数据。
- 是否涉及 World 内日志。
- 明确说明 Agent Trace、被 `state_commit_records.trace_ids` 引用的记录、关键回合 LLM Logs 不会自动删除。

### 8.7 后端命令边界

日志页面建议使用独立 Tauri 命令，避免前端直接理解 SQLite 文件路径：

- `query_log_records(filter, page)`：查询全局 / World / Trace 元数据列表。
- `get_log_record_detail(record_ref)`：按需读取 request / response / schema / readable text。
- `get_stream_chunks(request_id, source_ref, page)`：分页读取 stream chunks。
- `get_trace_detail(trace_id, world_id)`：读取 turn trace 与 step traces。
- `get_log_storage_summary()`：读取全局与 World 日志容量摘要。
- `export_logs(filter, format)`：导出当前筛选范围。
- `run_log_retention_now(scope)`：触发允许范围内的 retention 检查。
- `preview_log_cleanup(scope, policy)` 与 `confirm_log_cleanup(plan_id)`：先预览、后确认清理。

命令必须通过 `storage::paths` 定位日志库，不接收前端传入的任意文件路径。`world_id`、`trace_id`、`request_id` 等只作为查询参数，不能参与未校验路径拼接。

### 8.8 首版实现切片

第一版按以下顺序落地：

1. 元数据列表：全局 Logs、World Logs、Agent Trace 统一查询与筛选。
2. LLM 调用详情：request / response / schema / readable_text 查看。
3. stream chunks 分页查看。
4. Trace step 查看与 request 双向跳转。
5. 容量摘要与 30 天未更新 World 提示。
6. 当前筛选结果导出。
7. 手动清理预览与确认。

前四项构成日志页面 MVP；后三项构成日志管理增强。

### 8.9 验收要求

- `/logs` 能区分全局运行 Logs、World 内 Logs 与 Agent Trace。
- 默认列表不加载大 JSON 正文，详情按需加载。
- 能按 `request_id` 精确找到 LLM 调用，并查看脱敏 request / response。
- 流式请求能查看原始 chunk 顺序、`assembled_text` 和 `readable_text`。
- Agent Trace 能跳转到对应 LLM request，LLM request 能跳回对应 Trace step。
- ST 模式记录只出现在全局运行 Logs；Agent 回合相关记录能在对应 World 范围查到。
- 容量摘要不会扫描或暴露任意外部路径。
- 手动清理必须先生成预览，不允许一键直接删除 World Trace。
- 导出内容不包含未脱敏凭证。

---

## 9. 验证要求

- ST 模式 LLM 调用只写全局运行 Logs。
- Agent 模式任意 `scene_turn_id` 能查到完整 Agent Trace。
- Agent Trace 能跳转到对应 LLM request / response。
- 流式响应能查看原始 chunk，也能查看拼接后的可读文本。
- API Key、Authorization header、Provider secret 不会进入数据库。
- 全局 Logs 超过当前配置上限（默认 1GB）后后台清理旧运行日志。
- 普通清理任务不会删除 Agent Trace。
- 30 天未更新且日志较大的 World 只产生提示，不自动删除。
