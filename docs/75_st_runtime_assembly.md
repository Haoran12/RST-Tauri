# 75 ST 运行时组装

本文定义 ST 模式的全局状态、会话 metadata、运行时组装流程和 Provider 差异适配。角色卡见 [71_st_character_cards.md](71_st_character_cards.md)，世界书注入见 [73_st_worldbook_injection.md](73_st_worldbook_injection.md)，预设结构见 [74_st_presets.md](74_st_presets.md)，Regex 扩展见 [76_st_regex.md](76_st_regex.md)。

## 1. 全局应用状态

API 配置与预设完全独立，用户可随时切换，不与会话绑定：

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
- 无需切换会话或保存任何设置。

## 4. 职责边界

- `PresetManager` 负责加载、保存、导入、导出、默认值回填和原始字段保留。
- `RegexEngine` 负责 ST Regex 脚本合并、权限过滤、作用点过滤和文本替换；默认脚本可写回聊天文本，`markdownOnly` / `promptOnly` 只能作用于显示或请求组装。
- `RequestAssembler` 负责把当前 preset + API config + ST prompt 组装成中立 `ChatRequest`。
- `ProviderRequestMapper` 负责把中立请求映射到具体 Provider 参数，并处理不支持字段。
- `AIProvider` 只负责真实 API 调用，不参与世界书扫描、Prompt 片段选择或预设自动选择。

## 5. Provider 差异适配

不同 Provider 对采样参数的支持不同：

| 参数 | OpenAI | Anthropic | Gemini | Ollama |
|---|---|---|---|---|
| temperature | ✓ | ✓ | ✓ | ✓ |
| top_p | ✓ | ✓ | ✓ | ✓ |
| top_k | ✗ | ✗ | ✓ | ✓ |
| frequency_penalty | ✓ | ✗ | ✗ | ✗ |
| presence_penalty | ✓ | ✗ | ✗ | ✗ |
| repetition_penalty | ✗ | ✗ | ✓ | ✓ |
| mirostat | ✗ | ✗ | ✗ | ✓ |

适配策略：

- 不支持的参数静默忽略，不报错。
- 语义相近参数可自动映射，例如 `repetition_penalty` → `frequency_penalty` 近似。
- 预设可声明 `provider_overrides` 字段，为特定 Provider 提供替代值。
