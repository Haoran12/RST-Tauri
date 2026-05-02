// ST Runtime Assembly Types
// 运行时组装相关类型定义

// ============================================================================
// 全局应用状态
// ============================================================================

/**
 * 全局应用状态
 *
 * API 配置与预设、世界书选择完全独立，用户可随时切换，不与会话绑定。
 */
export interface GlobalAppState {
  active_api_config_id: string | null;

  active_sampler_preset: string;
  active_instruct_preset: string;
  active_context_preset: string;
  active_sysprompt_preset: string;
  active_reasoning_preset: string;
  active_prompt_preset: string;

  auto_select_preset: boolean;

  world_info_settings: STWorldInfoSettings;
  regex_settings: RegexExtensionSettings;
}

/**
 * ST 世界书全局设置
 */
export interface STWorldInfoSettings {
  global_select: string[];
  world_info_depth: number;
  world_info_min_activations: number;
  world_info_min_activations_depth_max: number;
  world_info_budget: number;
  world_info_budget_cap: number;
  world_info_include_names: boolean;
  world_info_recursive: boolean;
  world_info_overflow_alert: boolean;
  world_info_case_sensitive: boolean;
  world_info_match_whole_words: boolean;
  world_info_use_group_scoring: boolean;
  world_info_character_strategy: number; // 0=evenly, 1=character_first, 2=global_first
  world_info_max_recursion_steps: number;
  char_lore: CharLoreBinding[];
}

/**
 * 角色额外世界书绑定
 */
export interface CharLoreBinding {
  name: string;
  extra_books: string[];
}

/**
 * Regex 扩展设置（简化版）
 */
export interface RegexExtensionSettings {
  global_regex: string[];
  preset_allowed_regex: string[];
  character_allowed_regex: Record<string, string[]>;
}

// ============================================================================
// 会话数据
// ============================================================================

/**
 * ST 会话数据
 */
export interface STSessionData {
  session_id: string;
  character_id: string | null;
  group_id: string | null;
  chat_metadata: STChatMetadata;
  messages: STChatMessage[];
}

/**
 * ST 聊天元数据
 */
export interface STChatMetadata {
  world_info?: string;
  [key: string]: unknown;
}

/**
 * ST 聊天消息
 */
export interface STChatMessage {
  id: string;
  role: string;
  content: string;
  created_at: string;
  name?: string;
}

// ============================================================================
// 运行时组装
// ============================================================================

/**
 * 组装后的请求
 */
export interface AssembledRequest {
  system_prompt: string;
  messages: AssembledMessage[];
  sampling: AssembledSamplingParams;
  stop_sequences: string[];
  max_tokens: number | null;
  reasoning: AssembledReasoningParams | null;
}

/**
 * 组装后的消息
 */
export interface AssembledMessage {
  role: string;
  content: string;
}

/**
 * 组装后的采样参数
 */
export interface AssembledSamplingParams {
  temperature: number | null;
  top_p: number | null;
  top_k: number | null;
  frequency_penalty: number | null;
  presence_penalty: number | null;
  repetition_penalty: number | null;
}

/**
 * 组装后的推理参数
 */
export interface AssembledReasoningParams {
  enabled: boolean;
  effort: string | null;
  budget_tokens: number | null;
}

/**
 * 世界书注入结果
 */
export interface WorldInfoInjectionResult {
  world_info_before: string;
  world_info_after: string;
  world_info_depth: Record<number, Record<number, string>>;
  em_top: string;
  em_bottom: string;
  an_top: string;
  an_bottom: string;
  outlets: Record<string, string>;
  activated_entries: number[];
  tokens_used: number;
}

// ============================================================================
// 请求组装输入
// ============================================================================

/**
 * 组装请求的输入参数
 */
export interface AssembleRequestInput {
  api_config_id: string;
  character_id: string | null;
  session_id: string;
  sampler_preset: string | null;
  instruct_template: string | null;
  context_template: string | null;
  system_prompt: string | null;
  reasoning_template: string | null;
  prompt_preset: string | null;
  world_info_settings: STWorldInfoSettings;
  chat_lore_id?: string | null;
  global_lore_ids?: string[];
  max_context?: number;
}

/**
 * 组装请求的输出
 */
export interface AssembleRequestOutput {
  request: AssembledRequest;
  provider_type: string;
  model: string;
  world_info_result: WorldInfoInjectionResult | null;
}

/**
 * 世界书注入输入
 */
export interface WorldInfoInjectionInput {
  session_id: string;
  character_id: string | null;
  world_info_settings: STWorldInfoSettings;
  chat_lore_id: string | null;
  global_lore_ids: string[];
  max_context: number;
}

// ============================================================================
// 默认值工厂
// ============================================================================

/**
 * 创建默认全局应用状态
 */
export function createDefaultGlobalAppState(): GlobalAppState {
  return {
    active_api_config_id: null,
    active_sampler_preset: '',
    active_instruct_preset: '',
    active_context_preset: '',
    active_sysprompt_preset: '',
    active_reasoning_preset: '',
    active_prompt_preset: '',
    auto_select_preset: false,
    world_info_settings: createDefaultWorldInfoSettings(),
    regex_settings: {
      global_regex: [],
      preset_allowed_regex: [],
      character_allowed_regex: {},
    },
  };
}

/**
 * 创建默认世界书设置
 */
export function createDefaultWorldInfoSettings(): STWorldInfoSettings {
  return {
    global_select: [],
    world_info_depth: 4,
    world_info_min_activations: 0,
    world_info_min_activations_depth_max: 0,
    world_info_budget: 25,
    world_info_budget_cap: 0,
    world_info_include_names: false,
    world_info_recursive: true,
    world_info_overflow_alert: true,
    world_info_case_sensitive: false,
    world_info_match_whole_words: false,
    world_info_use_group_scoring: false,
    world_info_character_strategy: 1, // character_first
    world_info_max_recursion_steps: 5,
    char_lore: [],
  };
}
