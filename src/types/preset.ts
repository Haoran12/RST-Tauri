/**
 * ST Preset system
 *
 * 预设系统以 ST 扁平 PresetFile JSON 作为唯一主格式。
 * instruct/context/sysprompt/reasoning 仅作为兼容扩展保留，不是运行时主标准。
 * 参考: docs/74_st_presets.md
 */

// ============================================================================
// Sampler Preset - 采样参数预设
// ============================================================================

export interface SamplerPreset {
  name?: string

  // 基础采样参数
  temperature?: number
  top_p?: number
  top_k?: number
  top_a?: number
  min_p?: number
  typical_p?: number
  tfs?: number
  epsilon_cutoff?: number
  eta_cutoff?: number

  // 重复惩罚
  repetition_penalty?: number
  rep_pen_range?: number
  rep_pen_decay?: number
  rep_pen_slope?: number
  frequency_penalty?: number
  presence_penalty?: number
  encoder_rep_pen?: number

  // DRY
  dry_allowed_length?: number
  dry_multiplier?: number
  dry_base?: number
  dry_sequence_breakers?: string

  // Mirostat
  mirostat_mode?: number
  mirostat_tau?: number
  mirostat_eta?: number

  // 其他
  no_repeat_ngram_size?: number
  guidance_scale?: number
  negative_prompt?: string

  // 采样顺序
  sampler_priority?: string[]
  temperature_last?: boolean

  // Provider 覆盖
  provider_overrides?: Record<string, Record<string, unknown>>
}

export function createDefaultSamplerPreset(name = ''): SamplerPreset {
  return {
    name,
    temperature: 1.0,
    top_p: 1.0,
    top_k: 0,
    top_a: 0,
    min_p: 0,
    typical_p: 1.0,
    tfs: 1.0,
    epsilon_cutoff: 0,
    eta_cutoff: 0,
    repetition_penalty: 1.0,
    rep_pen_range: 0,
    rep_pen_decay: 0,
    rep_pen_slope: 0,
    frequency_penalty: 0,
    presence_penalty: 0,
    encoder_rep_pen: 1.0,
    dry_allowed_length: 0,
    dry_multiplier: 0,
    dry_base: 0,
    dry_sequence_breakers: '',
    mirostat_mode: 0,
    mirostat_tau: 5.0,
    mirostat_eta: 0.1,
    no_repeat_ngram_size: 0,
    guidance_scale: 1.0,
    negative_prompt: '',
    sampler_priority: [],
    temperature_last: false,
    provider_overrides: {},
  }
}

// ============================================================================
// Instruct Template - 对话格式模板
// ============================================================================

export interface InstructTemplate {
  name?: string

  // 序列定义
  input_sequence?: string
  output_sequence?: string
  system_sequence?: string
  stop_sequence?: string

  // 后缀
  input_suffix?: string
  output_suffix?: string
  system_suffix?: string

  // 特殊序列
  first_input_sequence?: string
  last_input_sequence?: string
  first_output_sequence?: string
  last_output_sequence?: string

  // 故事字符串
  story_string_prefix?: string
  story_string_suffix?: string

  // 行为设置
  wrap?: boolean
  macro?: boolean
  names_behavior?: 'none' | 'force' | 'always'
  system_same_as_user?: boolean
  skip_examples?: boolean
  sequences_as_stop_strings?: boolean

  // 激活正则
  activation_regex?: string
}

export function createDefaultInstructTemplate(name = ''): InstructTemplate {
  return {
    name,
    input_sequence: '',
    output_sequence: '',
    system_sequence: '',
    stop_sequence: '',
    input_suffix: '',
    output_suffix: '',
    system_suffix: '',
    first_input_sequence: '',
    last_input_sequence: '',
    first_output_sequence: '',
    last_output_sequence: '',
    story_string_prefix: '',
    story_string_suffix: '',
    wrap: false,
    macro: true,
    names_behavior: 'none',
    system_same_as_user: false,
    skip_examples: false,
    sequences_as_stop_strings: false,
    activation_regex: '',
  }
}

// ============================================================================
// Context Template - 上下文组装模板
// ============================================================================

export interface ContextTemplate {
  name?: string

  // 模板内容
  story_string?: string
  example_separator?: string
  chat_start?: string

  // 停止字符串
  use_stop_strings?: boolean
  names_as_stop_strings?: boolean

  // 故事字符串位置
  story_string_position?: number
  story_string_depth?: number
  story_string_role?: number

  // 其他设置
  always_force_name2?: boolean
  trim_sentences?: boolean
  single_line?: boolean
}

export function createDefaultContextTemplate(name = ''): ContextTemplate {
  return {
    name,
    story_string: '',
    example_separator: '',
    chat_start: '',
    use_stop_strings: true,
    names_as_stop_strings: false,
    story_string_position: 0,
    story_string_depth: 4,
    story_string_role: 0,
    always_force_name2: false,
    trim_sentences: false,
    single_line: false,
  }
}

// ============================================================================
// System Prompt - 系统提示词
// ============================================================================

export interface SystemPrompt {
  name?: string
  content?: string
}

export function createDefaultSystemPrompt(name = ''): SystemPrompt {
  return {
    name,
    content: '',
  }
}

// ============================================================================
// Reasoning Template - 思维链格式模板
// ============================================================================

export interface ReasoningTemplate {
  name?: string
  prefix?: string
  suffix?: string
  separator?: string
}

export function createDefaultReasoningTemplate(name = ''): ReasoningTemplate {
  return {
    name,
    prefix: '',
    suffix: '',
    separator: '',
  }
}

// ============================================================================
// Prompt Preset - 完整提示词组装配置
// ============================================================================

/**
 * 提示词条目（运行时）
 *
 * 兼容 SillyTavern 预设格式，支持注入位置控制。
 */
export interface PromptItem {
  identifier: string
  name: string
  role: 'system' | 'user' | 'assistant'
  content?: string
  system_prompt?: boolean
  marker?: boolean
  /** 是否启用（ST 格式，本项目优先使用 prompt_order 中的 enabled） */
  enabled?: boolean
  /** 注入位置：0=chat_history 之前，1=chat_history 之后 */
  injection_position?: number
  /** 注入深度（从底部往上数） */
  injection_depth?: number
  /** 注入顺序（同位置内的排序） */
  injection_order?: number
  /** 禁止被角色卡覆盖 */
  forbid_overrides?: boolean
  /** 触发条件列表 */
  injection_trigger?: string[]
}

export interface PromptOrderItem {
  identifier: string
  enabled?: boolean
  /** 用户自定义排序位置（仅对内置条目有效，用于覆盖默认位置） */
  position?: number
}

export interface PromptOrder {
  character_id?: number // 100000=默认，100001=群聊
  order?: PromptOrderItem[]
}

export interface PromptPreset {
  name?: string

  // 提示词列表
  prompts?: PromptItem[]
  prompt_order?: PromptOrder[]

  // 格式化模板
  wi_format?: string
  scenario_format?: string
  personality_format?: string

  // 特殊提示词
  new_chat_prompt?: string
  new_group_chat_prompt?: string
  continue_nudge_prompt?: string
  group_nudge_prompt?: string
  impersonation_prompt?: string
}

export function createDefaultPromptPreset(name = ''): PromptPreset {
  return {
    name,
    prompts: [],
    prompt_order: [],
    wi_format: '',
    scenario_format: '',
    personality_format: '',
    new_chat_prompt: '',
    new_group_chat_prompt: '',
    continue_nudge_prompt: '',
    group_nudge_prompt: '',
    impersonation_prompt: '',
  }
}

// ============================================================================
// Preset File - 预设文件（SillyTavern 兼容格式）
// ============================================================================

/**
 * 预设文件
 *
 * 采用 SillyTavern 扁平格式，采样参数和提示词配置都在顶层。
 * 参考: E:\AIPlay\SillyTavern\public\scripts\presets.js
 */
export interface PresetFile {
  name: string

  // ========================================
  // 采样参数（顶层，与 ST 一致）
  // ========================================
  temperature?: number
  frequency_penalty?: number
  presence_penalty?: number
  top_p?: number
  top_k?: number
  top_a?: number
  min_p?: number
  repetition_penalty?: number
  rep_pen_range?: number
  rep_pen_decay?: number
  rep_pen_slope?: number
  typical_p?: number
  tfs?: number
  epsilon_cutoff?: number
  eta_cutoff?: number
  guidance_scale?: number
  negative_prompt?: string

  // DRY
  dry_allowed_length?: number
  dry_multiplier?: number
  dry_base?: number
  dry_sequence_breakers?: string

  // Mirostat
  mirostat_mode?: number
  mirostat_tau?: number
  mirostat_eta?: number

  // 其他采样
  no_repeat_ngram_size?: number
  encoder_rep_pen?: number
  sampler_priority?: string[]
  temperature_last?: boolean

  // ========================================
  // 提示词配置（顶层，与 ST 一致）
  // ========================================
  prompts?: PromptItem[]
  prompt_order?: PromptOrder[]

  // 格式化模板
  wi_format?: string
  scenario_format?: string
  personality_format?: string

  // 特殊提示词
  send_if_empty?: string
  impersonation_prompt?: string
  new_chat_prompt?: string
  new_group_chat_prompt?: string
  new_example_chat_prompt?: string
  continue_nudge_prompt?: string
  group_nudge_prompt?: string

  // ========================================
  // ST 兼容字段
  // ========================================
  /** 流式输出 */
  stream_openai?: boolean
  /** 使用系统提示词 */
  use_sysprompt?: boolean
  /** 助手预填充 */
  assistant_prefill?: string
  /** 推理强度 */
  reasoning_effort?: string
  /** 最大上下文解锁 */
  max_context_unlocked?: boolean
  /** OpenAI 最大上下文 */
  openai_max_context?: number
  /** OpenAI 最大 tokens */
  openai_max_tokens?: number
  /** 名称行为 */
  names_behavior?: number

  // ========================================
  // RST 扩展字段
  // ========================================
  /** 对话格式模板（RST 扩展） */
  instruct?: InstructTemplate
  /** 上下文组装模板（RST 扩展） */
  context?: ContextTemplate
  /** 系统提示词（RST 扩展，已弃用，使用 prompts 中的 Main Prompt） */
  sysprompt?: SystemPrompt
  /** 思维链格式（RST 扩展） */
  reasoning?: ReasoningTemplate

  // 元数据
  source_api_id?: string
  extensions?: Record<string, unknown>
}

/**
 * 应用内置默认预设模板
 *
 * 基于 debug 产物中的 Default.json，作为新建预设的基础模板。
 */
const DEFAULT_PRESET_TEMPLATE: Omit<PresetFile, 'name'> = {
  // 采样参数默认值
  temperature: 1.0,
  top_p: 1.0,
  top_k: 0,
  top_a: 0,
  min_p: 0,
  typical_p: 0,
  tfs: 0,
  epsilon_cutoff: 0,
  eta_cutoff: 0,
  repetition_penalty: 1.0,
  rep_pen_range: 0,
  rep_pen_decay: 0,
  rep_pen_slope: 0,
  frequency_penalty: 0,
  presence_penalty: 0,
  encoder_rep_pen: 0,
  dry_allowed_length: 0,
  dry_multiplier: 0,
  dry_base: 0,
  dry_sequence_breakers: '',
  mirostat_mode: 0,
  mirostat_tau: 5.0,
  mirostat_eta: 0.1,
  no_repeat_ngram_size: 0,
  guidance_scale: 1.0,
  negative_prompt: '',
  sampler_priority: [],
  temperature_last: false,
  // 提示词配置
  prompts: [
    { identifier: 'main', name: 'Main Prompt', role: 'system', content: 'Write {{char}}\'s next reply in a fictional chat between {{char}} and {{user}}.', system_prompt: true },
    { identifier: 'nsfw', name: 'Auxiliary Prompt', role: 'system', content: '', system_prompt: true },
    { identifier: 'dialogueExamples', name: 'Chat Examples', role: 'system', system_prompt: true, marker: true },
    { identifier: 'jailbreak', name: 'Post-History Instructions', role: 'system', content: '', system_prompt: true },
    { identifier: 'chatHistory', name: 'Chat History', role: 'system', system_prompt: true, marker: true },
    { identifier: 'worldInfoAfter', name: 'World Info (after)', role: 'system', system_prompt: true, marker: true },
    { identifier: 'worldInfoBefore', name: 'World Info (before)', role: 'system', system_prompt: true, marker: true },
    { identifier: 'enhanceDefinitions', name: 'Enhance Definitions', role: 'system', content: 'If you have more knowledge of {{char}}, add to the character\'s lore and personality to enhance them but keep the Character Sheet\'s definitions absolute.', system_prompt: true },
    { identifier: 'charDescription', name: 'Char Description', role: 'system', system_prompt: true, marker: true },
    { identifier: 'charPersonality', name: 'Char Personality', role: 'system', system_prompt: true, marker: true },
    { identifier: 'scenario', name: 'Scenario', role: 'system', system_prompt: true, marker: true },
    { identifier: 'personaDescription', name: 'Persona Description', role: 'system', system_prompt: true, marker: true },
  ],
  prompt_order: [
    {
      character_id: 100000,
      order: [
        { identifier: 'main', enabled: true },
        { identifier: 'worldInfoBefore', enabled: true },
        { identifier: 'personaDescription', enabled: true },
        { identifier: 'charDescription', enabled: true },
        { identifier: 'charPersonality', enabled: true },
        { identifier: 'scenario', enabled: true },
        { identifier: 'enhanceDefinitions', enabled: false },
        { identifier: 'nsfw', enabled: true },
        { identifier: 'worldInfoAfter', enabled: true },
        { identifier: 'dialogueExamples', enabled: true },
        { identifier: 'chatHistory', enabled: true },
        { identifier: 'jailbreak', enabled: true },
      ],
    },
    {
      character_id: 100001,
      order: [
        { identifier: 'main', enabled: true },
        { identifier: 'worldInfoBefore', enabled: true },
        { identifier: 'charDescription', enabled: true },
        { identifier: 'charPersonality', enabled: true },
        { identifier: 'scenario', enabled: true },
        { identifier: 'enhanceDefinitions', enabled: false },
        { identifier: 'nsfw', enabled: true },
        { identifier: 'worldInfoAfter', enabled: true },
        { identifier: 'dialogueExamples', enabled: true },
        { identifier: 'chatHistory', enabled: true },
        { identifier: 'jailbreak', enabled: true },
      ],
    },
  ],
  wi_format: '{0}',
  scenario_format: '{{scenario}}',
  personality_format: '{{personality}}',
  send_if_empty: '',
  impersonation_prompt: '[Write your next reply from the point of view of {{user}}, using the chat history so far as a guideline for the writing style of {{user}}. Don\'t write as {{char}} or system. Don\'t describe actions of {{char}}.]',
  new_chat_prompt: '[Start a new Chat]',
  new_group_chat_prompt: '[Start a new group chat. Group members: {{group}}]',
  new_example_chat_prompt: '[Example Chat]',
  continue_nudge_prompt: '[Continue your last message without repeating its original content.]',
  group_nudge_prompt: '[Write the next reply only as {{char}}.]',
  // ST 兼容字段
  stream_openai: true,
  use_sysprompt: false,
  max_context_unlocked: false,
  openai_max_context: 4095,
  openai_max_tokens: 300,
  names_behavior: 0,
  // RST 扩展
  instruct: {
    name: 'Default',
    input_sequence: '',
    output_sequence: '',
    system_sequence: '',
    stop_sequence: '',
    input_suffix: '',
    output_suffix: '',
    system_suffix: '',
    first_input_sequence: '',
    last_input_sequence: '',
    first_output_sequence: '',
    last_output_sequence: '',
    story_string_prefix: '',
    story_string_suffix: '',
    wrap: false,
    macro: true,
    names_behavior: 'none',
    system_same_as_user: false,
    skip_examples: false,
    sequences_as_stop_strings: false,
    activation_regex: '',
  },
  context: {
    name: 'Default',
    story_string: '',
    example_separator: '',
    chat_start: '',
    use_stop_strings: true,
    names_as_stop_strings: false,
    story_string_position: 0,
    story_string_depth: 4,
    story_string_role: 0,
    always_force_name2: false,
    trim_sentences: false,
    single_line: false,
  },
  sysprompt: {
    name: 'Default',
    content: '',
  },
  reasoning: {
    name: 'Default',
    prefix: '',
    suffix: '',
    separator: '',
  },
  extensions: {},
}

export function createDefaultPresetFile(name: string): PresetFile {
  return {
    name,
    ...DEFAULT_PRESET_TEMPLATE,
    instruct: { ...DEFAULT_PRESET_TEMPLATE.instruct, name },
    context: { ...DEFAULT_PRESET_TEMPLATE.context, name },
    sysprompt: { ...DEFAULT_PRESET_TEMPLATE.sysprompt, name },
    reasoning: { ...DEFAULT_PRESET_TEMPLATE.reasoning, name },
  }
}

// ============================================================================
// Auto Select Config
// ============================================================================

export interface AutoSelectBinding {
  character_name?: string
  group_name?: string
  preset_name: string
}

export interface AutoSelectConfig {
  enabled?: boolean
  bindings?: AutoSelectBinding[]
}

export function createAutoSelectConfig(): AutoSelectConfig {
  return {
    enabled: false,
    bindings: [],
  }
}
