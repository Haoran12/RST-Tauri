// ST Mode Resource Types
// 类型设计原则：
// - 保留未知字段：导入时保存，导出时写回
// - 外部世界书 entries 使用 Record<string, WorldInfoEntry> 对象形式
// - CharacterBook entries 使用数组形式

// ============================================================================
// TavernCard V3 - 角色卡
// ============================================================================

/**
 * TavernCard V3 角色卡
 *
 * V3 validator 只检查：
 * - spec === 'chara_card_v3'
 * - Number(spec_version) >= 3.0 && < 4.0
 * - data 是对象
 *
 * 顶层未知字段必须保留。
 */
export interface TavernCardV3 {
  spec: 'chara_card_v3'
  spec_version: string // Number(value) >= 3.0 && < 4.0
  data: CharacterData

  // 顶层未知字段保留
  [key: string]: unknown
}

/**
 * 角色卡数据
 *
 * V3 / ST 扩展字段必须保留（如 group_only_greetings、depth_prompt 等）。
 */
export interface CharacterData {
  // 基础字段
  name: string
  description?: string
  personality?: string
  scenario?: string
  first_mes?: string
  mes_example?: string

  // 可选基础字段
  creator_notes?: string
  system_prompt?: string
  post_history_instructions?: string
  alternate_greetings?: string[]
  tags?: string[]
  creator?: string
  character_version?: string

  // 扩展字段
  extensions?: Record<string, unknown>

  // 内嵌 CharacterBook
  character_book?: CharacterBook

  // 未知字段保留（如 group_only_greetings、depth_prompt 等）
  [key: string]: unknown
}

// ============================================================================
// CharacterBook - 角色卡内嵌世界书
// ============================================================================

/**
 * 角色卡内嵌 CharacterBook
 *
 * 注意：CharacterBook 不会被 ST 运行时直接扫描，
 * 必须先由 Import Card Lore 转换为外部世界书。
 */
export interface CharacterBook {
  name?: string
  description?: string
  scan_depth?: number
  token_budget?: number
  recursive_scanning?: boolean
  extensions?: Record<string, unknown>

  // CharacterBook entries 是数组形式
  entries: CharacterBookEntry[]
}

/**
 * CharacterBook 条目
 */
export interface CharacterBookEntry {
  keys: string[]
  content: string

  enabled?: boolean
  insertion_order?: number
  case_sensitive?: boolean

  name?: string
  priority?: number
  id?: number
  comment?: string
  selective?: boolean
  secondary_keys?: string[]
  constant?: boolean
  position?: 'before_char' | 'after_char'

  extensions?: Record<string, unknown>
}

// ============================================================================
// WorldInfoFile - 外部世界书文件
// ============================================================================

/**
 * 外部世界书文件
 *
 * ST 后端导入和保存世界书时只强制检查对象中存在 entries 字段。
 * 运行时假设 entries 是以 UID 为 key 的对象。
 */
export interface WorldInfoFile {
  // entries 必须是对象形式
  entries: Record<string, WorldInfoEntry>

  // 从角色卡内嵌书导入时，保留原始 CharacterBook
  original_data?: CharacterBook

  // RST 内部稳定 ID
  rst_lore_id?: string

  name?: string
  description?: string
  extensions?: Record<string, unknown>

  // 未知字段保留
  [key: string]: unknown
}

/**
 * 世界书条目
 *
 * 字段名使用 camelCase，与 ST newWorldInfoEntryTemplate 一致。
 */
export interface WorldInfoEntry {
  uid: number

  // 匹配
  key?: string[]
  keysecondary?: string[]
  selective?: boolean
  selective_logic?: WorldInfoLogic

  // 内容
  comment?: string
  content?: string

  // 状态
  constant?: boolean
  vectorized?: boolean
  disable?: boolean
  add_memo?: boolean

  // 排序与位置
  order?: number
  position?: WorldInfoPosition
  depth?: number
  role?: ExtensionPromptRole
  outlet_name?: string

  // 预算
  ignore_budget?: boolean

  // 递归
  exclude_recursion?: boolean
  prevent_recursion?: boolean
  delay_until_recursion?: number | boolean

  // 概率
  probability?: number
  use_probability?: boolean

  // 分组
  group?: string
  group_override?: boolean
  group_weight?: number
  use_group_scoring?: boolean | null

  // 扫描
  scan_depth?: number | null
  case_sensitive?: boolean | null
  match_whole_words?: boolean | null

  // 时间控制
  sticky?: number | null
  cooldown?: number | null
  delay?: number | null

  // 匹配目标扩展
  match_persona_description?: boolean
  match_character_description?: boolean
  match_character_personality?: boolean
  match_character_depth_prompt?: boolean
  match_scenario?: boolean
  match_creator_notes?: boolean

  // 自动化
  automation_id?: string
  triggers?: string[]
  display_index?: number

  // 角色过滤
  character_filter?: CharacterFilter

  // 扩展
  extensions?: Record<string, unknown>

  // 未知字段保留
  [key: string]: unknown
}

/**
 * 角色过滤
 */
export interface CharacterFilter {
  names?: string[]
  tags?: string[]
  is_exclude?: boolean
}

// ============================================================================
// 枚举
// ============================================================================

export const enum WorldInfoLogic {
  AND_ANY = 0,
  NOT_ALL = 1,
  NOT_ANY = 2,
  AND_ALL = 3,
}

export const enum WorldInfoPosition {
  BEFORE_CHAR = 0,
  AFTER_CHAR = 1,
  AN_TOP = 2,
  AN_BOTTOM = 3,
  AT_DEPTH = 4,
  EM_TOP = 5,
  EM_BOTTOM = 6,
  OUTLET = 7,
}

export const enum ExtensionPromptRole {
  SYSTEM = 0,
  USER = 1,
  ASSISTANT = 2,
}

// ============================================================================
// WorldInfoEntry 默认值工厂
// ============================================================================

/**
 * 创建带默认值的新 WorldInfoEntry
 */
export function createWorldInfoEntry(uid: number): WorldInfoEntry {
  return {
    uid,
    key: [],
    keysecondary: [],
    selective: true,
    selective_logic: WorldInfoLogic.AND_ANY,
    comment: '',
    content: '',
    constant: false,
    vectorized: false,
    disable: false,
    add_memo: false,
    order: 100,
    position: WorldInfoPosition.BEFORE_CHAR,
    depth: 4,
    role: ExtensionPromptRole.SYSTEM,
    outlet_name: '',
    ignore_budget: false,
    exclude_recursion: false,
    prevent_recursion: false,
    delay_until_recursion: 0,
    probability: 100,
    use_probability: true,
    group: '',
    group_override: false,
    group_weight: 100,
    use_group_scoring: null,
    scan_depth: null,
    case_sensitive: null,
    match_whole_words: null,
    sticky: null,
    cooldown: null,
    delay: null,
    match_persona_description: false,
    match_character_description: false,
    match_character_personality: false,
    match_character_depth_prompt: false,
    match_scenario: false,
    match_creator_notes: false,
    automation_id: '',
    triggers: [],
  }
}

// ============================================================================
// 其他资源类型
// ============================================================================

export interface ApiConfig {
  id: string
  name: string
  provider: string
  model: string
  base_url?: string
  api_key?: string
  enabled: boolean
  settings: Record<string, unknown>
  created_at: string
  updated_at: string
}

export interface ChatSession {
  id: string
  name: string
  character_id?: string
  created_at: string
  updated_at: string
  messages: ChatMessage[]
}

export interface ChatMessage {
  id: string
  role: string
  content: string
  created_at: string
}

// ============================================================================
// 向后兼容类型别名
// ============================================================================

/** 向后兼容：旧代码使用 CharacterCard */
export type CharacterCard = TavernCardV3

/** 向后兼容：旧代码使用 Worldbook（数组形式） */
export interface Worldbook {
  entries: WorldbookEntry[]
  name?: string
  description?: string
  scan_depth?: number
  token_budget?: number
  recursive_scanning?: boolean
  extensions?: Record<string, unknown>
}

/** 向后兼容：旧代码使用 WorldbookEntry（数组形式） */
export interface WorldbookEntry {
  keys: string[]
  content: string
  extensions?: Record<string, unknown>
  enabled?: boolean
  insertion_order?: number
  case_sensitive?: boolean
  name?: string
  priority?: number
  id?: number
  comment?: string
  selective?: boolean
  secondary_keys?: string[]
  constant?: boolean
  position?: string
}
