/**
 * ST Preset system
 *
 * 预设系统，一个预设文件包含六类配置：Sampler/Instruct/Context/SystemPrompt/Reasoning/Prompt。
 * 预设与 API 配置解耦，可跨 Provider 共享。
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

// ============================================================================
// Built-in Prompt Items - 内置预设提示词条目
// ============================================================================

/**
 * 内置提示词条目内容来源类型
 */
export type BuiltinPromptSource = 'static' | 'generated'

/**
 * 内置提示词条目定义
 *
 * 内置条目由系统提供，不可删除，部分内容不可编辑。
 * - static: 静态内容，用户可查看但不可编辑内容
 * - generated: 系统动态生成内容，每次加载时由系统生成
 */
export interface BuiltinPromptItemDefinition {
  /** 内置条目标识符，以 'builtin:' 开头 */
  identifier: string
  /** 显示名称 */
  name: string
  /** 角色 */
  role: 'system' | 'user' | 'assistant'
  /** 内容来源类型 */
  source: BuiltinPromptSource
  /** 静态内容（source 为 static 时使用） */
  content?: string
  /** 内容生成器名称（source 为 generated 时使用） */
  generator?: string
  /** 默认是否启用 */
  defaultEnabled?: boolean
  /** 默认排序位置（数字越小越靠前） */
  defaultPosition?: number
  /** 是否为系统提示词 */
  system_prompt?: boolean
  /** 是否为标记条目 */
  marker?: boolean
  /** 描述说明 */
  description?: string
}

// ============================================================================
// Prompt Item - 提示词条目
// ============================================================================

/**
 * 提示词条目（运行时）
 *
 * 包含内置条目的元信息，用于前端区分显示和交互。
 */
export interface PromptItem {
  identifier: string
  name: string
  role: 'system' | 'user' | 'assistant'
  content?: string
  system_prompt?: boolean
  marker?: boolean
  /** 是否为内置条目 */
  builtin?: boolean
  /** 内置条目是否可编辑内容 */
  editable?: boolean
  /** 内置条目描述 */
  description?: string
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

/**
 * 默认内置提示词条目定义列表
 *
 * 这些条目在每个预设中都存在，不可删除，但可以禁用或调整顺序。
 * 部分条目内容由系统动态生成，用户无法直接编辑。
 */
export const BUILTIN_PROMPT_DEFINITIONS: BuiltinPromptItemDefinition[] = [
  {
    identifier: 'builtin:main_prompt',
    name: 'Main Prompt',
    role: 'system',
    source: 'static',
    content: '',
    defaultEnabled: true,
    defaultPosition: 0,
    system_prompt: true,
    description: '预设级系统提示词，定义 AI 的基本行为模式',
  },
  {
    identifier: 'builtin:character_description',
    name: 'Character Description',
    role: 'system',
    source: 'generated',
    generator: 'character_description',
    defaultEnabled: true,
    defaultPosition: 10,
    description: '角色描述，从当前角色卡动态提取',
  },
  {
    identifier: 'builtin:character_personality',
    name: 'Character Personality',
    role: 'system',
    source: 'generated',
    generator: 'character_personality',
    defaultEnabled: true,
    defaultPosition: 20,
    description: '角色性格，从当前角色卡动态提取',
  },
  {
    identifier: 'builtin:scenario',
    name: 'Scenario',
    role: 'system',
    source: 'generated',
    generator: 'scenario',
    defaultEnabled: true,
    defaultPosition: 30,
    description: '场景设定，从当前角色卡动态提取',
  },
  {
    identifier: 'builtin:world_info',
    name: 'World Info',
    role: 'system',
    source: 'generated',
    generator: 'world_info',
    defaultEnabled: true,
    defaultPosition: 40,
    description: '世界书内容，根据触发条件动态注入',
  },
  {
    identifier: 'builtin:chat_history',
    name: 'Chat History',
    role: 'system',
    source: 'generated',
    generator: 'chat_history',
    defaultEnabled: true,
    defaultPosition: 100,
    marker: true,
    description: '聊天历史记录占位标记',
  },
]

/**
 * 检查标识符是否为内置条目
 */
export function isBuiltinPrompt(identifier: string): boolean {
  return identifier.startsWith('builtin:')
}

/**
 * 获取内置条目定义
 */
export function getBuiltinDefinition(identifier: string): BuiltinPromptItemDefinition | undefined {
  return BUILTIN_PROMPT_DEFINITIONS.find((d) => d.identifier === identifier)
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
// Preset File - 预设文件（包含所有类型）
// ============================================================================

export interface PresetFile {
  name: string

  // 采样参数
  sampler?: SamplerPreset

  // 对话格式模板
  instruct?: InstructTemplate

  // 上下文组装模板
  context?: ContextTemplate

  // 系统提示词
  sysprompt?: SystemPrompt

  // 思维链格式
  reasoning?: ReasoningTemplate

  // 完整提示词组装
  prompt?: PromptPreset

  // 元数据
  source_api_id?: string
  extensions?: Record<string, unknown>
}

export function createDefaultPresetFile(name: string): PresetFile {
  return {
    name,
    sampler: createDefaultSamplerPreset(name),
    instruct: createDefaultInstructTemplate(name),
    context: createDefaultContextTemplate(name),
    sysprompt: createDefaultSystemPrompt(name),
    reasoning: createDefaultReasoningTemplate(name),
    prompt: createDefaultPromptPreset(name),
    extensions: {},
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
