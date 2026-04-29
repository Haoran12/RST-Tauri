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

## 2. 预设类型

| 类型 | 文件夹 | 用途 |
|---|---|---|
| Sampler Preset | `./data/presets/samplers/` | 采样参数（temperature、top_p、repetition_penalty 等） |
| Instruct Template | `./data/presets/instruct/` | 对话格式模板（input/output/system sequence 等） |
| Context Template | `./data/presets/context/` | 上下文组装模板（story_string、chat_start 等） |
| System Prompt | `./data/presets/sysprompt/` | 系统提示词模板 |
| Reasoning Template | `./data/presets/reasoning/` | 思维链格式模板（prefix、suffix、separator） |
| Prompt Preset | `./data/presets/prompts/` | 完整提示词组装配置（Main Prompt、Jailbreak、WI Format 等） |

## 3. 数据结构

### 3.1 Sampler Preset

```typescript
interface SamplerPreset {
  name: string;
  source_api_id?: string;

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

  extensions?: Record<string, any>;
  provider_overrides?: Record<string, Record<string, any>>;
}
```

### 3.2 Instruct Template

```typescript
interface InstructTemplate {
  name: string;

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
  extensions?: Record<string, any>;
}
```

### 3.3 Context Template

```typescript
interface ContextTemplate {
  name: string;

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

  extensions?: Record<string, any>;
}
```

### 3.4 System Prompt

```typescript
interface SystemPrompt {
  name: string;
  content: string;
  extensions?: Record<string, any>;
}
```

### 3.5 Reasoning Template

```typescript
interface ReasoningTemplate {
  name: string;
  prefix: string;
  suffix: string;
  separator: string;
  extensions?: Record<string, any>;
}
```

### 3.6 Prompt Preset

```typescript
interface PromptPreset {
  name: string;

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

  extensions?: Record<string, any>;
}

interface PromptItem {
  identifier: string;
  name: string;
  role: 'system' | 'user' | 'assistant';
  content: string;
  system_prompt?: boolean;
  marker?: boolean;
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

```typescript
interface PresetManager {
  listPresets(type: PresetType): Promise<string[]>;
  loadPreset(type: PresetType, name: string): Promise<Preset>;
  savePreset(type: PresetType, preset: Preset): Promise<void>;
  deletePreset(type: PresetType, name: string): Promise<void>;
  exportPreset(type: PresetType, name: string): Promise<string>;
  importPreset(type: PresetType, data: string): Promise<string>;
  getActivePreset(type: PresetType): string | null;
  setActivePreset(type: PresetType, name: string): void;
}

type PresetType =
  | 'sampler'
  | 'instruct'
  | 'context'
  | 'sysprompt'
  | 'reasoning'
  | 'prompt';
```

## 5. 自动选择

支持基于角色名 / 群组名自动选择预设：

```typescript
interface AutoSelectConfig {
  enabled: boolean;
  bindings: {
    characterName?: string;
    groupName?: string;
    presetType: PresetType;
    presetName: string;
  }[];
}

function autoSelectPreset(characterName: string, group: string | null): void;
```

匹配优先级：精确 `groupName + characterName` > 精确 `characterName` > 精确 `groupName`。若同一优先级命中多条，以列表后项覆盖前项。

自动选择只由角色 / 群组上下文、用户显式启用状态和绑定列表触发。切换 API 配置不是自动选择输入，不得因为 Provider、model、endpoint 或 `source_api_id` 匹配而切换预设。

## 6. 默认预设

应用启动时在 `./data/presets/` 下创建默认预设：

```
./data/presets/
├── samplers/
│   ├── Default.json
│   ├── Neutral.json
│   ├── Deterministic.json
│   └── Universal-Creative.json
├── instruct/
│   ├── ChatML.json
│   ├── Alpaca.json
│   ├── Llama 3 Instruct.json
│   └── Vicuna 1.1.json
├── context/
│   ├── Default.json
│   ├── ChatML.json
│   └── NovelAI.json
├── sysprompt/
│   └── Default.json
├── reasoning/
│   └── Default.json
└── prompts/
    └── Default.json
```

## 7. Master Import / Export

支持将所有预设类型打包导出为单一 JSON 文件：

```typescript
interface MasterPresetExport {
  sampler?: SamplerPreset;
  instruct?: InstructTemplate;
  context?: ContextTemplate;
  sysprompt?: SystemPrompt;
  reasoning?: ReasoningTemplate;
  prompt?: PromptPreset;
}
```

导入时自动检测类型并分发到对应预设管理器，仍然剔除其中的 API Provider / URL / Key / model 信息。
