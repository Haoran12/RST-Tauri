/**
 * ST Preset system
 *
 * 预设系统，支持 Sampler/Instruct/Context/SystemPrompt/Reasoning/Prompt 六类预设。
 * 预设与 API 配置解耦，可跨 Provider 共享。
 * 参考: docs/74_st_presets.md
 */

// ============================================================================
// Sampler Preset - 采样参数预设
// ============================================================================

export interface SamplerPreset {
  name: string
  source_api_id?: string

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

  // 扩展
  extensions?: Record<string, unknown>
  provider_overrides?: Record<string, Record<string, unknown>>
}

export function createSamplerPreset(name: string): SamplerPreset {
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
    extensions: {},
    provider_overrides: {},
  }
}

// ============================================================================
// Instruct Template - 对话格式模板
// ============================================================================

export interface InstructTemplate {
  name: string

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

  // 扩展
  extensions?: Record<string, unknown>
}

export function createInstructTemplate(name: string): InstructTemplate {
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
    extensions: {},
  }
}

// ============================================================================
// Context Template - 上下文组装模板
// ============================================================================

export interface ContextTemplate {
  name: string

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

  // 扩展
  extensions?: Record<string, unknown>
}

export function createContextTemplate(name: string): ContextTemplate {
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
    extensions: {},
  }
}

// ============================================================================
// System Prompt - 系统提示词
// ============================================================================

export interface SystemPrompt {
  name: string
  content?: string
  extensions?: Record<string, unknown>
}

export function createSystemPrompt(name: string): SystemPrompt {
  return {
    name,
    content: '',
    extensions: {},
  }
}

// ============================================================================
// Reasoning Template - 思维链格式模板
// ============================================================================

export interface ReasoningTemplate {
  name: string
  prefix?: string
  suffix?: string
  separator?: string
  extensions?: Record<string, unknown>
}

export function createReasoningTemplate(name: string): ReasoningTemplate {
  return {
    name,
    prefix: '',
    suffix: '',
    separator: '',
    extensions: {},
  }
}

// ============================================================================
// Prompt Preset - 完整提示词组装配置
// ============================================================================

export interface PromptItem {
  identifier: string
  name: string
  role: 'system' | 'user' | 'assistant'
  content?: string
  system_prompt?: boolean
  marker?: boolean
}

export interface PromptOrderItem {
  identifier: string
  enabled?: boolean
}

export interface PromptOrder {
  character_id?: number // 100000=默认，100001=群聊
  order?: PromptOrderItem[]
}

export interface PromptPreset {
  name: string

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

  // 扩展
  extensions?: Record<string, unknown>
}

export function createPromptPreset(name: string): PromptPreset {
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
    extensions: {},
  }
}

// ============================================================================
// Preset Type
// ============================================================================

export type PresetType =
  | 'sampler'
  | 'instruct'
  | 'context'
  | 'sysprompt'
  | 'reasoning'
  | 'prompt'

export const PresetTypeFolder: Record<PresetType, string> = {
  sampler: 'samplers',
  instruct: 'instruct',
  context: 'context',
  sysprompt: 'sysprompt',
  reasoning: 'reasoning',
  prompt: 'prompts',
}

// ============================================================================
// Auto Select Config
// ============================================================================

export interface AutoSelectBinding {
  character_name?: string
  group_name?: string
  preset_type: PresetType
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
