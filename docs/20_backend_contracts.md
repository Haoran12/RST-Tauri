# 20 后端契约：AI Provider 抽象

需支持三种调用模式：

- **自由对话**（SillyTavern 模式 / SurfaceRealizer）
- **严格 JSON 输出**（Agent 模式各 LLM 节点）
- **流式输出**（聊天 UI 体验）

---

## 1. AIProvider trait

```rust
#[async_trait]
pub trait AIProvider: Send + Sync {
    fn name(&self) -> &str;
    fn models(&self) -> Vec<String>;

    /// 自由文本输出（SillyTavern 模式 / SurfaceRealizer）
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, String>;

    /// 严格 JSON 输出（Agent 模式各 LLM 节点）
    /// 使用 Provider 的 JSON mode / structured output 能力。
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

实现：`OpenAIProvider` / `AnthropicProvider` / `GeminiProvider` / `OllamaProvider`。

各 Provider 通过自身能力实现 `chat_structured`：

| Provider | structured 输出机制 |
|---|---|
| OpenAI | JSON mode (`response_format: json_object`) + 在 system prompt 中嵌 schema |
| Anthropic | Tool use（声明一个返回该 schema 的虚拟工具，让模型调用） |
| Gemini | `response_schema` 字段直接传 JSON Schema |
| Ollama | `format=json` 参数 + system prompt 中嵌 schema |

---

## 2. Agent 模式各 LLM 节点对应的调用类型

| LLM 节点 | 调用类型 | 输出 schema |
|---|---|---|
| SceneStateExtractor | `chat_structured` | UserInputDelta |
| CharacterCognitivePass | `chat_structured` | CharacterCognitivePassOutput |
| Arbitration 兜底 | `chat_structured` | IntentPlan |
| SurfaceRealizer | `chat` 或 `chat_stream` | 自由文本叙事 |

SillyTavern 模式仅使用 `chat` / `chat_stream`，不依赖 `chat_structured`。
