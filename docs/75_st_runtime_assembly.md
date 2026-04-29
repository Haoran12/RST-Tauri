# 75 ST 运行时组装

本文定义 ST 模式的全局状态、会话 metadata、运行时组装流程和 Provider 差异适配。角色卡见 [71_st_character_cards.md](71_st_character_cards.md)，世界书注入见 [73_st_worldbook_injection.md](73_st_worldbook_injection.md)，预设结构见 [74_st_presets.md](74_st_presets.md)，Regex 扩展见 [76_st_regex.md](76_st_regex.md)。

## 1. 全局应用状态

API 配置与预设、世界书选择完全独立，用户可随时切换，不与会话绑定：

```typescript
interface GlobalAppState {
  active_api_config_id: string | null;

  active_sampler_preset: string;
  active_instruct_preset: string;
  active_context_preset: string;
  active_sysprompt_preset: string;
  active_reasoning_preset: string;
  active_prompt_preset: string;

  auto_select_preset: boolean;

  // ST 世界书全局设置（对应 SillyTavern settings.world_info_settings / world_info）
  world_info_settings: STWorldInfoSettings;

  // ST Regex 扩展全局设置（对应 SillyTavern extension_settings.regex*）
  regex_settings: STRegexExtensionSettings;
}
```

```typescript
interface STWorldInfoSettings {
  globalSelect: string[];                 // selected_world_info
  world_info_depth: number;
  world_info_min_activations: number;
  world_info_min_activations_depth_max: number;
  world_info_budget: number;              // max context 百分比
  world_info_budget_cap: number;
  world_info_include_names: boolean;
  world_info_recursive: boolean;
  world_info_overflow_alert: boolean;
  world_info_case_sensitive: boolean;
  world_info_match_whole_words: boolean;
  world_info_use_group_scoring: boolean;
  world_info_character_strategy: 0 | 1 | 2;
  world_info_max_recursion_steps: number;

  // 角色额外书，按角色文件名绑定；结构与 ST world_info.charLore 兼容
  charLore?: {
    name: string;
    extraBooks: string[];
  }[];
}
```

`STRegexExtensionSettings` 结构见 [76_st_regex.md](76_st_regex.md)，包含全局脚本、Regex Preset、角色内嵌脚本 allow list 和预设内嵌脚本 allow list。

`active_api_config_id` 只选择连接配置。它不得作为 preset、world_info、charLore、Regex allow list、聊天 metadata 或角色卡扩展字段的命名空间，也不得参与这些资源的自动选择和持久化身份。

## 2. 会话数据

会话数据存储聊天记录、角色卡引用和 ST 兼容的聊天元数据，不存储 API 配置或预设引用：

```typescript
interface SessionData {
  session_id: string;

  character_id: string;
  group_id?: string;
  chat_metadata: STChatMetadata;

  messages: ChatMessage[];

  // 不存储 API 配置或预设引用。
  // API 配置和预设由全局状态管理，用户随时可切换。
  // chat_metadata.world_info 是会话自己的世界书绑定，不随 API 配置切换。
}

interface STChatMetadata {
  // Chat lore：当前聊天绑定的单本世界书。对应 SillyTavern chat_metadata.world_info。
  world_info?: string;

  // Author's Note、变量、脚本注入、书签等扩展继续保留原始 key。
  [key: string]: any;
}
```

## 3. 运行时组装流程

```
用户发起生成请求
       ↓
读取全局应用状态
       ↓
┌──────────────────────────────────────┐
│ 1. 加载 API 配置（连接参数）          │
│    - Provider 类型、endpoint、model   │
│    - API key、超时、代理              │
│    - 来源：active_api_config_id       │
└──────────────────────────────────────┘
       ↓
┌──────────────────────────────────────┐
│ 2. 加载预设（生成参数）               │
│    - Sampler Preset → 采样参数        │
│    - Instruct Template → 消息格式     │
│    - Context Template → 上下文组装    │
│    - System Prompt → 系统提示词       │
│    - Reasoning Template → 思维链格式  │
│    - Prompt Preset → 提示词片段       │
│    - 来源：active_*_preset            │
└──────────────────────────────────────┘
       ↓
┌──────────────────────────────────────┐
│ 3. 加载会话内容                       │
│    - 角色卡 → 角色信息                │
│    - chat_metadata.world_info → Chat lore │
│    - 角色卡 extensions.world → Character lore │
│    - world_info.globalSelect → Global lore │
│    - 聊天记录 → 对话上下文            │
└──────────────────────────────────────┘
       ↓
┌──────────────────────────────────────┐
│ 4. 运行 ST 世界书注入                  │
│    - checkWorldInfo → before/after/depth/AN/EM/outlet │
└──────────────────────────────────────┘
       ↓
┌──────────────────────────────────────┐
│ 5. 运行 Regex prompt-only 变换         │
│    - 聊天历史 USER_INPUT / AI_OUTPUT  │
│    - 世界书 WORLD_INFO                 │
│    - reasoning REASONING               │
└──────────────────────────────────────┘
       ↓
┌──────────────────────────────────────┐
│ 6. 组装请求                           │
│    - API 配置 → 连接信息              │
│    - 预设参数 → 请求体参数            │
│    - 会话内容 + 注入结果 → 消息内容   │
└──────────────────────────────────────┘
       ↓
调用 AIProvider.chat() 或 chat_stream()
```

用户切换 API 配置或预设时：

- 立即更新全局应用状态。
- 下次生成请求自动使用新配置。
- 无需切换会话。

切换 API 配置的副作用边界：

- 可以改变：Provider 类型、endpoint、model、鉴权、代理、超时、Provider 字段映射、不支持参数的忽略 / 降级方式。
- 不可以改变：`active_*_preset`、自动预设选择结果、`world_info_settings`、`chat_metadata.world_info`、角色卡 `data.extensions.world`、`world_info.charLore`、Regex allow list、Regex Preset、世界书文件内容、预设文件内容。
- 不应保存资源文件；只保存 `active_api_config_id` 本身或用户显式编辑的 API 配置。

## 4. 职责边界

- `PresetManager` 负责加载、保存、导入、导出、默认值回填和原始字段保留。
- `RegexEngine` 负责 ST Regex 脚本合并、权限过滤、作用点过滤和文本替换；默认脚本可写回聊天文本，`markdownOnly` / `promptOnly` 只能作用于显示或请求组装。
- `RequestAssembler` 负责把当前 preset + API config + ST prompt 组装成中立 `ChatRequest`。
- `ProviderRequestMapper` 负责把中立请求映射到具体 Provider 参数，并处理不支持字段。
- `AIProvider` 只负责真实 API 调用，不参与世界书扫描、Prompt 片段选择或预设自动选择。

## 5. Provider 差异适配

### 5.0 一等适配范围

ST 模式和共享 API 配置池必须把以下 Provider / 协议作为一等适配目标：

- OpenAI Responses API
- OpenAI Chat Completions API
- Google Gemini GenerateContent / streamGenerateContent
- Anthropic Messages API
- DeepSeek Chat Completions 兼容接口
- Claude Code Interface

后续任何 API 相关改动都必须同时评估这六类：中立 `ChatRequest` 到 Provider 请求的字段映射、消息 role/content 形态、流式事件解析、token usage、错误响应、结构化输出降级、日志脱敏与回放还原。矩阵中写作 `OpenAI` 的参数若两种 OpenAI 协议行为不同，必须在实现中拆成 `openai_responses` 与 `openai_chat_completions` 两条适配路径；DeepSeek 虽兼容 OpenAI Chat 形态，也必须保留独立能力表与错误处理。

### 5.1 采样参数支持矩阵

不同 Provider 对采样参数的支持不同，DeepSeek 与 OpenAI 同为高优先级支持：

| 参数 | OpenAI Responses | OpenAI Chat | DeepSeek | Anthropic | Gemini | Claude Code Interface |
|---|---|---|---|---|---|---|
| temperature | ✓ (0-2) | ✓ (0-2) | ✓ (0-2) | ✓ (0-1) | ✓ | 取决于后端变体 |
| top_p | ✓ (0-1) | ✓ (0-1) | ✓ (0-1) | ✓ (0-1) | ✓ | 取决于后端变体 |
| top_k | ✗ | ✗ | ✗ | ✓ | ✓ | 取决于后端变体 |
| frequency_penalty | ✗ | ✓ (-2~2) | ✓ (-2~2) | ✗ | ✗ | 取决于后端变体 |
| presence_penalty | ✗ | ✓ (-2~2) | ✓ (-2~2) | ✗ | ✗ | 取决于后端变体 |
| repetition_penalty | ✗ | ✗ | ✗ | ✗ | ✓ | 取决于后端变体 |
| stop | 视模型支持 | ✓ | ✓ (最多16个) | ✓ (stop_sequences) | ✓ (stopSequences) | 取决于后端变体 |

### 5.2 流式传输设置

| Provider | 字段 | 类型 | 说明 |
|---|---|---|---|
| OpenAI Responses | `stream` | boolean | 启用 Responses SSE 事件流 |
| OpenAI Chat | `stream` | boolean | 启用 Chat Completions SSE chunk |
| OpenAI Chat | `stream_options.include_usage` | boolean | 流式返回 token 用量 |
| DeepSeek | `stream` | boolean | 启用 SSE 流式传输 |
| DeepSeek | `stream_options.include_usage` | boolean | 流式返回 token 用量 |
| Anthropic | `stream` | boolean | 启用 SSE 流式传输 |
| Gemini | 端点切换 | - | 使用 `streamGenerateContent` 端点 |
| Claude Code Interface | 接口事件流 | - | 按 Claude Code 兼容事件/消息循环解析，不直接复用普通 SSE chunk parser |

### 5.3 推理/思维链设置

| Provider | 字段 | 类型 | 取值 | 说明 |
|---|---|---|---|---|
| OpenAI Responses | `reasoning.effort` | string | "low", "medium", "high" | 推理强度，仅推理模型支持 |
| OpenAI Chat | `reasoning_effort` | string | "low", "medium", "high" | 推理强度，仅推理模型支持 |
| DeepSeek | `thinking.type` | string | "enabled", "disabled" | 推理开关 |
| DeepSeek | `thinking.reasoning_effort` | string | "high", "max" | 推理强度 |
| Anthropic | `thinking.type` | string | "enabled", "disabled", "adaptive" | 思维链模式 |
| Anthropic | `thinking.budget_tokens` | integer | ≥1024 | 思维链 token 预算 |
| Anthropic | `thinking.display` | string | "summarized", "omitted" | 思维链显示方式 |
| Claude Code Interface | 取决于后端变体 | - | - | 不假定可用；必须显式探测或配置 |

### 5.4 语义相近参数映射

当用户设置的参数在当前 Provider 不支持时，可尝试映射到语义相近的参数：

| 源参数 | 目标参数 | 映射方向 | 近似程度 | 说明 |
|---|---|---|---|---|
| `repetition_penalty` | `frequency_penalty` | → OpenAI/DeepSeek | 中等 | 都惩罚重复，但机制不同 |
| `repetition_penalty` | `presence_penalty` | → OpenAI/DeepSeek | 较弱 | presence 只惩罚出现与否 |
| `top_k` | - | 无映射 | - | Anthropic/DeepSeek/OpenAI 无等价参数 |

**映射规则：**
- `repetition_penalty` (通常 1.0-2.0) → `frequency_penalty` (0-2)：`frequency_penalty ≈ repetition_penalty - 1.0`
- 映射为近似值，用户应针对不同 Provider 单独调参

### 5.5 适配策略

- 不支持的参数静默忽略，不报错。
- 语义相近参数可自动映射（需用户确认或预设配置）。
- 预设可声明 `provider_overrides` 字段，为特定 Provider 提供替代值。
- 推理参数仅在支持的模型上生效，否则忽略。

### 5.6 请求组装示例

**OpenAI Responses:**
```json
{
  "model": "gpt-<model>",
  "input": [...],
  "temperature": 0.7,
  "top_p": 0.9,
  "stream": true
}
```

**OpenAI Chat Completions:**
```json
{
  "model": "gpt-4o",
  "messages": [...],
  "temperature": 0.7,
  "top_p": 0.9,
  "frequency_penalty": 0.5,
  "stream": true,
  "stream_options": { "include_usage": true }
}
```

**DeepSeek Chat:**
```json
{
  "model": "deepseek-v4-pro",
  "messages": [...],
  "temperature": 0.7,
  "top_p": 0.9,
  "frequency_penalty": 0.5,
  "thinking": { "type": "enabled", "reasoning_effort": "high" },
  "stream": true
}
```

**Anthropic Messages:**
```json
{
  "model": "claude-sonnet-4-6",
  "max_tokens": 4096,
  "messages": [...],
  "temperature": 0.7,
  "top_p": 0.9,
  "top_k": 50,
  "thinking": { "type": "enabled", "budget_tokens": 2048 },
  "stream": true
}
```

**Gemini GenerateContent:**
```json
{
  "contents": [...],
  "generationConfig": {
    "temperature": 0.7,
    "topP": 0.9,
    "topK": 50,
    "maxOutputTokens": 4096
  }
}
```

**Claude Code Interface:**
```json
{
  "system": "...",
  "messages": [...],
  "tools": [...],
  "max_tokens": 4096,
  "stream": true
}
```
