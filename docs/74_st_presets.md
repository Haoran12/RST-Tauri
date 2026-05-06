# 74 ST 预设系统

本文定义 ST 模式预设类型、导入导出、自动选择，以及 RST 相对 SillyTavern 的核心偏离：Preset 与 API Provider 解耦。运行时如何把预设与 API 配置组装为请求见 [75_st_runtime_assembly.md](75_st_runtime_assembly.md)。

## 1. 设计原则

SillyTavern 原设计中，预设按 API 类型（kobold / novel / openai / textgenerationwebui）分类存储，预设与 API 配置强绑定。RST 取消此绑定：

| 概念 | 职责 | 存储位置 |
|---|---|---|
| API 配置 | Provider 类型、endpoint URL、model、API key、超时、代理等连接与鉴权参数 | `./data/api_configs/` |
| 预设 | 采样参数、提示词模板、停止符、惩罚参数等生成参数 | `./data/presets/` |

解耦规则：

- 同一预设可用于不同 API 配置。
- 切换 API 配置无需重新选择预设。
- 切换 API 配置不得触发自动选择预设、改写 active preset、改写 preset 文件、清空 provider_overrides 或改变预设内嵌 Regex 授权。
- 预设可跨 Provider 共享与迁移。
- 导入 ST 预设时保留原始 JSON 字段，额外记录 `source_api_id` / `source_preset_type` 作为迁移信息。
- 导出为 ST 兼容格式时，可按目标 `apiId` 重新写入 ST 期望目录 / 文件结构。
- RST 运行时不从 preset 读取 endpoint、key、model、connection profile；这些字段若出现在导入文件中，只作为原始扩展数据保留或在 Master Import 时剔除。

`source_api_id` 和导出目标 `apiId` 只用于 ST 兼容迁移，不参与 RST 运行时身份判断。运行时预设身份使用 RST 的稳定 `preset_key`（稳定 ID 或规范化路径）；`preset_key` 不随当前 API 配置变化。

## 2. 预设文件结构

本应用以 `E:\AIPlay\cards\夏瑾DS预设v0.40.json` 所代表的 ST 扁平 `PresetFile` 作为创建 / 编辑 / 导入 / 导出 / 运行时的实际标准。`./data/presets/` 下每个预设直接保存一个扁平 JSON 文件；RST 不再使用 `presets/samplers/`、`presets/instruct/` 等分目录作为运行时持久化来源：

```
./data/presets/
├── Default.json
├── Creative.json
├── ChatML.json
└── ...
```

预设文件内部结构：

```typescript
interface PresetFile {
  name: string;

  // 顶层采样参数（与 ST 保持一致）
  temperature?: number;
  top_p?: number;
  top_k?: number;
  top_a?: number;
  min_p?: number;
  repetition_penalty?: number;
  frequency_penalty?: number;
  presence_penalty?: number;
  openai_max_context?: number;
  openai_max_tokens?: number;
  stream_openai?: boolean;
  reasoning_effort?: string;

  // 顶层 prompt 装配字段（与 ST 保持一致）
  prompts?: PromptItem[];
  prompt_order?: PromptOrder[];
  wi_format?: string;
  scenario_format?: string;
  personality_format?: string;
  send_if_empty?: string;
  new_chat_prompt?: string;
  new_group_chat_prompt?: string;
  new_example_chat_prompt?: string;
  continue_nudge_prompt?: string;
  group_nudge_prompt?: string;
  impersonation_prompt?: string;

  // 兼容扩展：保留旧 RST 分段字段，但不再作为运行时主标准
  instruct?: InstructTemplate;
  context?: ContextTemplate;
  sysprompt?: SystemPrompt;
  reasoning?: ReasoningTemplate;

  source_api_id?: string;
  extensions?: Record<string, any>;
}
```

运行时主规则：

- `prompts + prompt_order` 是提示词组装唯一主链。
- `chatHistory` 之前且 `system_prompt=true` 的 PromptItem 合并为 provider-level system prompt。
- `chatHistory` 之后的 PromptItem 作为内联消息加入聊天消息链。
- `instruct/context/sysprompt/reasoning` 仅作兼容扩展字段保留；当前仅少量字段会被运行时消费。

## 3. 数据结构

### 3.1 Sampler Preset（采样参数）

```typescript
interface SamplerPreset {
  name?: string;
  temperature: number;
  top_p: number;
  top_k: number;
  top_a: number;
  min_p: number;
  typical_p: number;
  tfs: number;
  epsilon_cutoff: number;
  eta_cutoff: number;

  repetition_penalty: number;
  rep_pen_range: number;
  rep_pen_decay: number;
  rep_pen_slope: number;
  frequency_penalty: number;
  presence_penalty: number;
  encoder_rep_pen: number;

  dry_allowed_length: number;
  dry_multiplier: number;
  dry_base: number;
  dry_sequence_breakers: string;

  mirostat_mode: number;
  mirostat_tau: number;
  mirostat_eta: number;

  no_repeat_ngram_size: number;
  guidance_scale: number;
  negative_prompt: string;

  sampler_priority?: string[];
  temperature_last?: boolean;

  provider_overrides?: Record<string, Record<string, any>>;
}
```

Sampler 编辑规划：

- UI 分为基础生成参数、重复控制、兼容字段三组；首屏只暴露 `temperature`、`top_p`、`top_k`、`frequency_penalty`、`presence_penalty`、`repetition_penalty`、最大上下文、最大输出 tokens 与流式输出。
- `stream_openai` 在 RST 中改名显示为“流式输出”，语义为当前预设是否走 `chat_stream`。它不再只代表 OpenAI，也不再由聊天页硬编码开启。
- `openai_max_context` / `openai_max_tokens` 保留 ST 兼容字段名，但 UI 显示为通用最大上下文 / 最大输出 tokens。运行时必须把最大输出 tokens 写入中立 `ChatRequest.max_tokens`，再由 Provider adapter 映射为 `max_tokens`、`max_output_tokens` 或 `generationConfig.maxOutputTokens`。
- 不同 Provider 不支持的采样字段由 `llm_api_contract` 驱动屏蔽或转换；UI 后续应在选择了主 API 配置时标记“当前连接会忽略 / 近似映射”的字段，但保存的预设仍保持 ST 兼容字段完整。

### 3.2 Instruct / Context / SystemPrompt / Reasoning（兼容扩展）

以下字段继续保留在 `PresetFile` 中，目的是：

- 兼容旧版 RST 预设文件
- 兼容 ST master import 转换
- 为未来更完整的 ST 模板链路保留字段

它们不是创建 / 编辑 / 运行时的主标准，运行时当前只会选择性消费其中的个别字段。

#### Instruct Template（对话格式模板）

```typescript
interface InstructTemplate {
  name?: string;
  input_sequence: string;
  output_sequence: string;
  system_sequence: string;
  stop_sequence: string;

  input_suffix: string;
  output_suffix: string;
  system_suffix: string;

  first_input_sequence?: string;
  last_input_sequence?: string;
  first_output_sequence?: string;
  last_output_sequence?: string;

  story_string_prefix?: string;
  story_string_suffix?: string;

  wrap: boolean;
  macro: boolean;
  names_behavior: 'none' | 'force' | 'always';
  system_same_as_user: boolean;
  skip_examples: boolean;
  sequences_as_stop_strings: boolean;

  activation_regex?: string;
}
```

#### Context Template（上下文模板）

```typescript
interface ContextTemplate {
  name?: string;
  story_string: string;
  example_separator: string;
  chat_start: string;

  use_stop_strings: boolean;
  names_as_stop_strings: boolean;

  story_string_position: number;
  story_string_depth: number;
  story_string_role: number;

  always_force_name2: boolean;
  trim_sentences: boolean;
  single_line: boolean;
}
```

#### System Prompt（系统提示词）

```typescript
interface SystemPrompt {
  name?: string;
  content: string;
}
```

#### Reasoning Template（思维链格式）

```typescript
interface ReasoningTemplate {
  name?: string;
  prefix: string;
  suffix: string;
  separator: string;
}
```

### 3.3 Prompt Preset（提示词组装主链）

```typescript
interface PromptPreset {
  name?: string;
  prompts: PromptItem[];
  prompt_order: PromptOrderItem[];

  wi_format: string;
  scenario_format: string;
  personality_format: string;

  new_chat_prompt: string;
  new_group_chat_prompt: string;
  continue_nudge_prompt: string;
  group_nudge_prompt: string;
  impersonation_prompt: string;
}

interface PromptItem {
  identifier: string;
  name: string;
  role: 'system' | 'user' | 'assistant';
  content: string;
  system_prompt?: boolean;
  marker?: boolean;
  injection_position?: number;
  injection_depth?: number;
  injection_order?: number;
  forbid_overrides?: boolean;
  injection_trigger?: string[];
}

interface PromptOrderItem {
  character_id: number;          // 100000=默认，100001=群聊
  order: {
    identifier: string;
    enabled: boolean;
  }[];
}
```

## 4. 预设管理器

预设中的大段模板正文使用 [42_structured_text_editor.md](42_structured_text_editor.md) 定义的 Structured Text Editor：

- `SystemPrompt.content`、`PromptItem.content`、`story_string`、`wi_format`、`scenario_format`、`personality_format` 等字段默认 Plain。
- 用户可切 JSON / YAML，把 prompt 正文组织成 LLM 可读的结构化模板、规则列表、示例块或配置样式文本，但保存形态仍是 string。
- JSON / YAML 解析失败时阻止保存当前字段；Plain 的括号 / 引号不匹配只作为 warning。
- `extensions`、`provider_overrides` 和导入保留的未知字段默认使用 JSON 模式编辑；保存前必须保持 object 形态。

```typescript
interface PresetManager {
  listPresets(): Promise<string[]>;
  loadPreset(name: string): Promise<PresetFile>;
  savePreset(preset: PresetFile): Promise<void>;
  deletePreset(name: string): Promise<void>;
  exportPreset(name: string): Promise<string>;
  importPreset(data: string): Promise<string>;
  getActivePreset(): string | null;
  setActivePreset(name: string): void;
}
```

## 5. 自动选择

支持基于角色名 / 群组名自动选择预设：

```typescript
interface AutoSelectConfig {
  enabled: boolean;
  bindings: {
    characterName?: string;
    groupName?: string;
    presetName: string;
  }[];
}

function autoSelectPreset(characterName: string, group: string | null): void;
```

匹配优先级：精确 `groupName + characterName` > 精确 `characterName` > 精确 `groupName`。若同一优先级命中多条，以列表后项覆盖前项。

自动选择只由角色 / 群组上下文、用户显式启用状态和绑定列表触发。切换 API 配置不是自动选择输入，不得因为 Provider、model、endpoint 或 `source_api_id` 匹配而切换预设。

## 6. 默认预设

应用启动或首次列出 / 读取预设时，若 `./data/presets/Default.json` 缺失，存储层必须自动创建默认预设：

```

当前编辑器与运行时以这些字段为核心：

- `identifier`
- `content`
- `role`
- `system_prompt`
- `marker`
- `injection_position`
- `injection_depth`
- `injection_order`
- `forbid_overrides`
- `injection_trigger`

其中 `injection_position/depth/order/trigger` 必须在文件中原样保留；运行时现阶段已开始消费 `injection_position` 的历史前/后分段语义，其余字段先作为标准字段保存，不得在导入导出时丢失。
./data/presets/
└── Default.json
```

`Default.json` 包含完整六类配置，且不绑定任何 API 配置、model、endpoint 或鉴权字段。全局状态缺失或旧版六字段 active preset 迁移失败时，`active_preset` 回退为 `Default`；默认预设不能通过 UI 或命令删除。

## 7. 导入导出

### 7.1 RST 格式导出

导出为单一 JSON 文件，包含完整的 `PresetFile` 结构。

### 7.2 导入格式兼容

支持导入以下格式：

#### 7.2.1 RST PresetFile 格式

```json
{
  "name": "My Preset",
  "sampler": { ... },
  "instruct": { ... },
  "context": { ... },
  "sysprompt": { ... },
  "reasoning": { ... },
  "prompt": { ... }
}
```

#### 7.2.2 ST Master 格式

ST 的 Advanced Formatting 导出格式，包含多个 section：

```json
{
  "instruct": { "name": "...", "input_sequence": "...", ... },
  "context": { "name": "...", "story_string": "...", ... },
  "sysprompt": { "name": "...", "content": "..." },
  "reasoning": { "name": "...", "prefix": "...", ... },
  "preset": { "temp": 1.0, "top_p": 1.0, ... }
}
```

导入时自动合并到 `PresetFile` 结构。

#### 7.2.3 ST 单类型预设

自动检测并转换：

| 类型 | 检测字段 | 转换目标 |
|---|---|---|
| Instruct Template | `input_sequence`, `output_sequence` | `instruct` |
| Context Template | `story_string` | `context` |
| System Prompt | `content` (无 `prompts`) | `sysprompt` |
| Reasoning Template | `prefix`, `suffix` | `reasoning` |
| Text Completion | `temp`, `top_k`, `top_p`, `rep_pen` | `sampler` |

#### 7.2.4 字段名映射

ST Text Completion 预设使用缩写字段名，导入时自动映射：

| ST 字段 | RST 字段 |
|---|---|
| `temp` | `temperature` |
| `rep_pen` | `repetition_penalty` |

### 7.3 解耦原则

导入时自动剔除以下字段（API 连接相关）：

- `api_server`
- `streaming_url`
- `model` / `model_novel` / `custom_model`
- `preset_settings` / `preset_settings_novel`
- `seed`
- `server_urls`
- 其他连接配置字段

这些字段仅用于 ST 兼容迁移追踪，不参与 RST 运行时。

### 7.4 ST 兼容导出（规划中）

导出为 ST 兼容格式时，按目标 `apiId` 重新写入 ST 期望目录 / 文件结构。
