# 20 后端契约：AI Provider 抽象

需支持三种调用模式：

- **自由对话**（SillyTavern 模式 / SurfaceRealizer）
- **严格 JSON 输出**（Agent 模式的部分 LLM 节点）
- **流式输出**（聊天 UI 体验）

所有调用必须经过日志包装层记录请求、响应、流式 chunk 与异常。日志结构与清理规则见 [30_logging_and_observability.md](30_logging_and_observability.md)。

第一版 AI Provider / 协议适配范围必须覆盖：

| 适配目标 | 协议 / 端点形态 | 说明 |
|---|---|---|
| OpenAI Responses API | `/v1/responses` | OpenAI 新一代响应接口；结构化输出优先使用 `text.format` / JSON Schema |
| OpenAI Chat Completions API | `/v1/chat/completions` | OpenAI 兼容消息接口；同时作为部分兼容 Provider 的基础协议形态 |
| Google Gemini | `models.generateContent` / `streamGenerateContent` | 使用 `contents` 与 `generationConfig` 组装请求 |
| Anthropic | Messages API | 原生 `system` + `messages` 结构；结构化输出优先使用官方 schema / tool 能力 |
| DeepSeek | Chat Completions 兼容接口 | OpenAI Chat 兼容形态，但推理、JSON mode、限制和错误处理按 DeepSeek 独立适配 |
| Claude Code Interface | Claude Code 风格消息 / 工具 / 环境接口 | 面向 Claude Code 兼容网关或本地接口；不得简单等同于普通 Anthropic Messages passthrough |

后续做 API 相关适配时，必须逐项考虑上述六类：请求字段白名单、role / content 映射、流式事件解析、结构化输出降级、token usage、错误响应、日志脱敏和回放还原都要明确各自行为。

---

## 1. AIProvider trait

```rust
#[async_trait]
pub trait AIProvider: Send + Sync {
    fn name(&self) -> &str;
    fn models(&self) -> Vec<String>;

    /// 自由文本输出（SillyTavern 模式 / SurfaceRealizer）
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, String>;

    /// 严格 schema 输出（Agent 模式各 LLM 节点）
    /// 优先使用 Provider 的 structured output / tool schema 能力；
    /// JSON mode 只能作为降级路径，返回后仍必须 schema 校验。
    async fn chat_structured(
        &self,
        request: ChatRequest,
        schema: serde_json::Value,    // JSON Schema
    ) -> Result<serde_json::Value, String>;

    /// 流式输出（聊天 UI 体验）
    async fn chat_stream(&self, request: ChatRequest)
        -> Result<Box<dyn Stream<Item = String>>, String>;
}
```

实现：`OpenAIResponsesProvider` / `OpenAIChatProvider` / `AnthropicProvider` / `GeminiProvider` / `DeepSeekProvider` / `ClaudeCodeInterfaceProvider`。其他 Provider（例如本地模型或 Ollama）可作为扩展加入，但不得替代上述一等适配目标。

Provider 实现只负责真实 API 调用，不直接写日志。调用方必须通过 `LoggingAIProvider` 或等价 wrapper 注入 `LogContext`：

```rust
pub struct LoggingAIProvider<P: AIProvider> {
    inner: P,
    logger: LlmCallLogger,
}

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

Wrapper 的后置条件：

- 调用开始时写入 `llm_call_logs(status=started)`。
- 成功时写入脱敏后的 `request_json`、`response_json`、耗时、token usage 与 `status=succeeded`。
- 失败或取消时写入错误摘要、耗时、Provider 原始错误响应（如有）与 `status=failed/cancelled`。
- `chat_structured` 必须保存 `schema_json`。
- `chat_stream` 必须按顺序写入 `llm_stream_chunks.raw_chunk`，同时生成 `assembled_text` 与 `readable_text`。
- API Key、Authorization header、Provider secret、代理认证等字段必须在落库前脱敏。
- 日志写入失败不得改写 Provider 调用结果；应额外写 `app_event_logs` 或降级为内存错误计数。

各 Provider 通过自身能力实现 `chat_structured`：

| Provider | structured 输出机制 |
|---|---|
| OpenAI Responses | Structured Outputs（typed `text.format` / JSON Schema，strict schema）；不支持时才降级为工具或 JSON mode + schema 校验 |
| OpenAI Chat Completions | Structured Outputs（`response_format.type=json_schema`）；旧模型才降级为 JSON mode + schema 校验 |
| Anthropic | 原生 structured output / tool use（声明一个返回该 schema 的虚拟工具，让模型调用） |
| Gemini | `response_schema` 字段直接传 JSON Schema |
| DeepSeek | JSON/object 模式或兼容格式 + system prompt 中嵌 schema + 返回后 schema 校验 |
| Claude Code Interface | 优先沿用接口暴露的 tool/schema 能力；若后端仅提供 Claude Code 兼容消息循环，则用受控工具调用或 JSON 降级，并在本地 schema 校验 |

`chat_structured` 的统一后置条件：返回值必须通过传入的 JSON Schema 校验；未通过时由 Provider 层执行有限重试，仍失败则向上返回错误并触发运行时容错路径。JSON mode 只能保证 JSON 可解析，不能替代 schema adherence。

Agent 模式的调用方不得手写散落 prompt。所有 Agent LLM 调用必须先由 `PromptBuilder` 生成 [13_agent_llm_io.md](13_agent_llm_io.md) 定义的 `AgentPromptBundle`：

- system：静态节点契约。
- developer / system追加：本次任务说明；Provider 无 developer role 时合并进 system。
- user：单个 JSON 对象 `{ "input": <TInput> }`。
- `chat_structured`：额外传入输出 JSON Schema。

Provider 适配层只能做消息格式映射、schema 能力映射和降级处理；不得改变节点权限、追加世界事实、读取日志或绕过 `PromptBuilder`。

---

## 2. Agent 模式各 LLM 节点对应的调用类型

| LLM 节点 | 调用类型 | 输出 schema | 权限类型 |
|---|---|---|
| SceneInitializer | `chat_structured` | SceneInitializationDraft | 公开上下文受控补全；默认不读隐藏 Knowledge / GodOnly |
| SceneStateExtractor | `chat_structured` | SceneStateExtractorOutput | 场景域 God-read；默认不读隐藏 Knowledge / GodOnly |
| CharacterCognitivePass | `chat_structured` | CharacterCognitivePassOutput | 受限：只读 L2 + prior L3 |
| OutcomePlanner | `chat_structured` | OutcomePlannerOutput | God-read；输出候选结果与候选状态更新，不能直接提交 |
| SurfaceRealizer | `chat` 或 `chat_stream` | 自由文本叙事 | 受限：只读 NarrationScope 派生输入 |

SillyTavern 模式仅使用 `chat` / `chat_stream`，不依赖 `chat_structured`。

ST 模式的 LLM 调用只写全局 `./data/logs/app_logs.sqlite`。Agent 模式的 LLM 调用写入对应 World 的 `world.sqlite`，并通过 `scene_turn_id` / `trace_id` / `request_id` 与 Agent Trace 关联。

## 3. Agent LLM 节点 API 配置绑定

第一版复用 ST 模式的 API 配置池（`./data/api_configs/`）。用户可以为五类 Agent LLM 节点分别选择不同的 API 配置；未显式配置的节点继承全局默认 Agent 配置。

```rust
pub enum AgentLlmNode {
    SceneInitializer,
    SceneStateExtractor,
    CharacterCognitivePass,
    OutcomePlanner,
    SurfaceRealizer,
}

pub struct AgentLlmConfigBinding {
    pub node: AgentLlmNode,
    pub api_config_id: String,      // 指向 ST/API 配置池中的配置
    pub enabled: bool,
}

pub struct AgentLlmProfile {
    pub profile_id: String,
    pub name: String,
    pub default_api_config_id: String,
    pub bindings: Vec<AgentLlmConfigBinding>,
}
```

约束：

- API 配置只定义 Provider、model、base URL、鉴权、采样参数、超时、代理等调用参数；不得改变节点权限。
- 节点权限由 `AgentLlmNode` 决定，不能因为用户选择了更强模型而提升 Knowledge 访问权限或叙事披露范围。
- `chat_structured` 节点必须校验所选 API 配置支持结构化输出；不支持时只允许走文档定义的 JSON 降级路径。
- 每次调用必须把 `api_config_id`、provider、model 写入 `llm_call_logs`，便于回放与问题定位。
- World 可以保存自己的 `AgentLlmProfile` 引用或覆盖项；删除 API 配置前必须检查是否被 Agent profile / World 引用。
