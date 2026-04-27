# 20 后端契约：AI Provider 抽象

需支持三种调用模式：

- **自由对话**（SillyTavern 模式 / SurfaceRealizer）
- **严格 JSON 输出**（Agent 模式各 LLM 节点）
- **流式输出**（聊天 UI 体验）

所有调用必须经过日志包装层记录请求、响应、流式 chunk 与异常。日志结构与清理规则见 [30_logging_and_observability.md](30_logging_and_observability.md)。

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

实现：`OpenAIProvider` / `AnthropicProvider` / `GeminiProvider` / `OllamaProvider` / `DeepSeekProvider`。

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
    pub llm_node: LlmNode,             // STChat / SceneStateExtractor / CharacterCognitivePass / ArbitrationFallback / SurfaceRealizer
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
| OpenAI | Structured Outputs（`json_schema` / typed `text.format`，strict schema）；旧模型才降级为 JSON mode + schema 校验 |
| Anthropic | Tool use（声明一个返回该 schema 的虚拟工具，让模型调用） |
| Gemini | `response_schema` 字段直接传 JSON Schema |
| Ollama | `format=json` 参数 + system prompt 中嵌 schema + 返回后 schema 校验 |
| DeepSeek | JSON/object 模式或兼容格式 + system prompt 中嵌 schema + 返回后 schema 校验 |

`chat_structured` 的统一后置条件：返回值必须通过传入的 JSON Schema 校验；未通过时由 Provider 层执行有限重试，仍失败则向上返回错误并触发运行时容错路径。JSON mode 只能保证 JSON 可解析，不能替代 schema adherence。


---

## 2. Agent 模式各 LLM 节点对应的调用类型

| LLM 节点 | 调用类型 | 输出 schema |
|---|---|---|
| SceneStateExtractor | `chat_structured` | UserInputDelta |
| CharacterCognitivePass | `chat_structured` | CharacterCognitivePassOutput |
| Arbitration 兜底 | `chat_structured` | IntentPlan |
| SurfaceRealizer | `chat` 或 `chat_stream` | 自由文本叙事 |

SillyTavern 模式仅使用 `chat` / `chat_stream`，不依赖 `chat_structured`。

ST 模式的 LLM 调用只写全局 `./data/logs/app_logs.sqlite`。Agent 模式的 LLM 调用写入对应 World 的 `world.sqlite`，并通过 `scene_turn_id` / `trace_id` / `request_id` 与 Agent Trace 关联。
