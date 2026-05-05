# 20 后端契约：AI Provider 抽象

需支持三种调用模式：

- **自由对话**（SillyTavern 模式）
- **严格 JSON 输出**（Agent 模式 LLM 节点）
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

后续做 API 相关适配时，必须逐项考虑上述六类：请求字段白名单、role / content 映射、图片 / PDF 输入形态、流式事件解析、结构化输出降级、token usage、错误响应、日志脱敏和回放还原都要明确各自行为。

Claude Code Interface 的请求格式复刻参考见 [CLAUDE_CODE_REQUEST_FORMAT.md](CLAUDE_CODE_REQUEST_FORMAT.md)；该文档是参考资料，不替代本文件定义的 `AIProvider` 抽象和日志边界。具体字段白名单、协议差异、多模态输入能力和 RST 级附件策略以后端编译后的 `LlmApiContractsSnapshot` 为准，其源文件为 `config/llm_api_contracts.json`。

---

## 1. AIProvider trait

```rust
#[async_trait]
pub trait AIProvider: Send + Sync {
    fn name(&self) -> &str;
    fn models(&self) -> Vec<String>;

    /// 自由文本输出（SillyTavern 模式）
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

`ChatRequest` 是 ST 模式、Agent PromptBuilder 与 ProviderRequestMapper 之间的中立请求形态；它不保存 API key、endpoint 或连接配置，这些只来自 `api_config_id` 对应的 API 配置。ST 模式下图片 / PDF 附件的本地存储与聊天记录引用见 [77_st_multimodal_attachments.md](77_st_multimodal_attachments.md)；这里定义发送给 Provider 前的中立内容块：

```rust
pub struct ChatRequest {
    pub request_id: String,
    pub api_config_id: String,
    pub messages: Vec<ChatMessage>,
    pub sampling: SamplingParams,
    pub stop_sequences: Vec<String>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
    pub reasoning: Option<ReasoningParams>,
    pub response_format: Option<ResponseFormat>,
    pub provider_overrides: serde_json::Value,
}

pub struct ChatMessage {
    pub role: ChatRole,
    pub content: Vec<ContentPart>,
    pub name: Option<String>,
}

pub enum ChatRole {
    System,
    Developer,
    User,
    Assistant,
    Tool,
}

pub enum ContentPart {
    Text { text: String },
    ImageRef {
        attachment_id: String,
        mime_type: String,
        filename_hint: Option<String>,
        transport: BinaryTransport,
        detail: Option<ImageDetail>,
    },
    DocumentRef {
        attachment_id: String,
        mime_type: String,   // 第一版只允许 application/pdf
        filename_hint: Option<String>,
        transport: BinaryTransport,
    },
    ToolResult { tool_call_id: String, payload: serde_json::Value },
}

pub enum BinaryTransport {
    InlineBase64 { data_base64: String },
    ExternalUrl { url: String },
    ProviderFile { handle: String },
}

pub enum ImageDetail {
    Auto,
    Low,
    High,
    Original,
}

pub struct SamplingParams {
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub top_k: Option<u32>,
    pub repetition_penalty: Option<f64>,
    pub frequency_penalty: Option<f64>,
    pub presence_penalty: Option<f64>,
}

pub struct ReasoningParams {
    pub effort: Option<String>,
    pub budget_tokens: Option<u32>,
    pub exclude_reasoning_text_from_response: bool,
}

pub enum ResponseFormat {
    Text,
    JsonObject,
    JsonSchema { schema: serde_json::Value, strict: bool },
}
```

实现：`OpenAIResponsesProvider` / `OpenAIChatProvider` / `AnthropicProvider` / `GeminiProvider` / `DeepSeekProvider` / `ClaudeCodeInterfaceProvider`。其他 Provider（例如本地模型或 Ollama）可作为扩展加入，但不得替代上述一等适配目标。

后端实现要求：

- `ProviderRequestMapper`、`CapabilityResolver`、结构化输出降级判断和多模态输入判断必须以后端编译后的 `LlmApiContractsSnapshot` 为唯一契约源。
- 禁止在多个 Provider 适配器里手写散落的 allowed/forbidden 字段、图片/PDF 支持表和静默降级规则。
- 禁止在每次请求热路径重新读取 `config/llm_api_contracts.json`；必须在启动或显式 reload 时加载并编译。
- 必须按当前 API 连接维度维护 `ProviderContractCache`。cache key 至少包含 `api_config_id + provider_kind + protocol_kind + model + base_url`；Claude Code Interface 还应包含 `provider_variant`。
- cache value 应包含该连接下已经裁剪好的字段白名单、结构化输出策略、`input_capabilities`、默认 transport 策略与 fail-fast 规则，供 `RequestAssembler` / `CapabilityResolver` / `ProviderRequestMapper` 直接读取。

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

多模态补充约束：

- `ImageRef` / `DocumentRef` 中的 `attachment_id` 必须可回指本地源文件；远端 `file_id` / `file_uri` 只是 `transport=ProviderFile` 的缓存句柄，不是持久化真源。
- `DocumentRef` 第一版只允许 `application/pdf`。
- 对不支持图片 / PDF 的 Provider 或 model，运行时必须在发送前 fail fast；不得静默 OCR、抽纯文本或删除内容块。

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

Agent 模式的调用方不得手写散落 prompt。所有 Agent LLM 调用必须先由 `PromptBuilder` 生成 [13_agent_llm_io.md](13_agent_llm_io.md) 定义的 `AgentPromptBundle`；具体节点 I/O 契约按节点分布在 [13_agent_llm_io.md](13_agent_llm_io.md)、[21_agent_scene_llm_io.md](21_agent_scene_llm_io.md) 与 [22_agent_outcome_narration_io.md](22_agent_outcome_narration_io.md)：

- system：静态节点契约。
- developer / system追加：本次任务说明；Provider 无 developer role 时合并进 system。
- user：单个 JSON 对象 `{ "input": <TInput> }`。
- `chat_structured`：额外传入输出 JSON Schema。

Provider 适配层只能做消息格式映射、schema 能力映射和降级处理；不得改变节点权限、追加世界事实、读取日志或绕过 `PromptBuilder`。

### 1.1 ST 多模态输入映射

ST 模式用户消息允许混合 `Text`、`ImageRef` 与 `DocumentRef`。默认映射策略：

| Provider | 图片输入 | PDF 输入 | 第一版默认策略 |
|---|---|---|---|
| OpenAI Responses | `input_image` | `input_file` | 小文件可 inline；多轮或较大附件优先 Files API |
| OpenAI Chat Completions | `content[].image_url`（可 `data:` URL） | `content[].file`（`file_id` 或 `file_data`） | 图片优先 `data:` URL；PDF 优先 `file_id` |
| Anthropic Messages | `content[].type=image` | `content[].type=document` | 小文件可 base64；多轮或较大附件优先 Files API |
| Gemini | `parts.inline_data` 或 Files API `file_uri` | `parts.inline_data` 或 Files API `file_uri` | 小文件 inline；重复使用或大文件优先 Files API |
| DeepSeek Chat Completions | 不支持 | 不支持 | 发送前直接报能力错误 |
| Claude Code Interface | 取决于兼容后端 | 取决于兼容后端 | 必须先看 capability；未声明则报能力错误 |

`ExternalUrl` 只作为兼容传输选项，不作为默认数据源。即使 Provider 支持 URL 引用，RST 默认仍以本地源文件 + inline / Files API 为主，避免外部 URL 生命周期和权限漂移影响回放一致性。

### 1.2 契约加载与连接缓存

后端应把 `config/llm_api_contracts.json` 视为打包内置契约资源，而不是每次请求动态打开的业务文件：

```rust
pub struct LlmApiContractsSnapshot {
    pub llm_api_contracts_snapshot_id: String,
    pub schema_version: String,
    pub contracts_hash: String,
    pub provider_contracts: Arc<CompiledProviderContracts>,
    pub multimodal_policy: Arc<CompiledMultimodalPolicy>,
}

pub struct ProviderContractCacheKey {
    pub api_config_id: String,
    pub provider_kind: String,
    pub protocol_kind: String,
    pub model: String,
    pub base_url: String,
    pub provider_variant: Option<String>,
}

pub struct CompiledProviderContractView {
    pub request_whitelist: Arc<RequestWhitelist>,
    pub structured_output_strategy: Arc<StructuredOutputStrategy>,
    pub input_capabilities: Arc<InputCapabilities>,
    pub multimodal_policy: Arc<ConnectionMultimodalPolicy>,
}
```

规则：

- `LlmApiContractsSnapshot` 在应用启动时加载；开发模式或显式 reload 时可刷新。
- `ProviderContractCache` 在第一次遇到某个连接 key 时，从 `LlmApiContractsSnapshot + api_config` 编译对应视图并缓存。
- 用户切换 `active_api_config_id`、修改 model、修改 endpoint/base URL、或 Claude Code `provider_variant` 变化时，只失效对应 key。
- `RequestAssembler`、`CapabilityResolver`、`ProviderRequestMapper` 在热路径只读 `Arc<LlmApiContractsSnapshot>` 与 `ProviderContractCache`，不得直接读盘解析 JSON。

---

## 2. Agent 模式各 LLM 节点对应的调用类型

| LLM 节点 | 调用类型 | 输出 schema | 权限类型 |
|---|---|---|
| SceneInitializer | `chat_structured` | SceneInitializationDraft | 公开上下文 + 场景相关私有约束；受限 God-read，不全库读隐藏 Knowledge / GodOnly |
| SceneStateExtractor | `chat_structured` | SceneStateExtractorOutput | 场景域 God-read；只读当前 SceneModel 与场景相关私有约束 |
| CharacterCognitivePass | `chat_structured` | CharacterCognitivePassOutput | 受限：只读 L2 + prior L3 |
| OutcomePlanner | `chat_structured` | OutcomePlannerOutput | God-read；输出候选结果与候选状态更新，不能直接提交 |
| SurfaceRealizer | `chat_structured` | SurfaceRealizerOutput | 受限：只读 NarrationScope 派生输入；外层必须返回 `{ narrative_text, used_fact_ids }` |

SillyTavern 模式仅使用 `chat` / `chat_stream`，不依赖 `chat_structured`。Agent 模式所有节点，包括 SurfaceRealizer，都必须经过 `PromptBuilder` 与 schema 校验；`SurfaceRealizerOutput.narrative_text` 是允许展示给用户的自由文本叶子，`used_fact_ids` 用于 NarrativeFactCheck。

第一版不把 SurfaceRealizer 作为自由文本流式节点实现。若后续需要叙事流式体验，必须采用“流式展示草稿 + 最终结构化 `SurfaceRealizerOutput` 校验通过后确认入库”的双阶段方案，或新增 `chat_structured_stream` 契约；不得用裸 `chat_stream` 绕过 `used_fact_ids`。

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
