//! ST mode resource types and storage
//!
//! 类型设计原则：
//! - 保留未知字段：导入时保存，导出时写回
//! - 外部世界书 entries 使用 Record<string, WorldInfoEntry> 对象形式
//! - CharacterBook entries 使用数组形式

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// TavernCard V3 - 角色卡
// ============================================================================

/// TavernCard V3 角色卡
///
/// V3 validator 只检查：
/// - spec === 'chara_card_v3'
/// - Number(spec_version) >= 3.0 && < 4.0
/// - data 是对象
///
/// 顶层未知字段必须保留。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TavernCardV3 {
    pub spec: String,
    pub spec_version: String,
    pub data: CharacterData,

    /// 顶层未知字段保留
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// 角色卡数据
///
/// V3 / ST 扩展字段必须保留（如 group_only_greetings、depth_prompt 等）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterData {
    // 基础字段
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub personality: String,
    #[serde(default)]
    pub scenario: String,
    #[serde(default)]
    pub first_mes: String,
    #[serde(default)]
    pub mes_example: String,

    // 可选基础字段
    #[serde(default)]
    pub creator_notes: String,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub post_history_instructions: String,
    #[serde(default)]
    pub alternate_greetings: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub creator: String,
    #[serde(default)]
    pub character_version: String,

    // 扩展字段
    #[serde(default)]
    pub extensions: serde_json::Map<String, serde_json::Value>,

    // 内嵌 CharacterBook
    #[serde(default)]
    pub character_book: Option<CharacterBook>,

    /// 未知字段保留（如 group_only_greetings、depth_prompt 等）
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

// ============================================================================
// CharacterBook - 角色卡内嵌世界书
// ============================================================================

/// 角色卡内嵌 CharacterBook
///
/// 注意：CharacterBook 不会被 ST 运行时直接扫描，
/// 必须先由 Import Card Lore 转换为外部世界书。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterBook {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub scan_depth: Option<i32>,
    #[serde(default)]
    pub token_budget: Option<i32>,
    #[serde(default)]
    pub recursive_scanning: Option<bool>,
    #[serde(default)]
    pub extensions: serde_json::Map<String, serde_json::Value>,

    /// CharacterBook entries 是数组形式
    pub entries: Vec<CharacterBookEntry>,
}

/// CharacterBook 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterBookEntry {
    pub keys: Vec<String>,
    pub content: String,

    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub insertion_order: i32,
    #[serde(default)]
    pub case_sensitive: Option<bool>,

    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub priority: Option<i32>,
    #[serde(default)]
    pub id: Option<i32>,
    #[serde(default)]
    pub comment: String,
    #[serde(default)]
    pub selective: Option<bool>,
    #[serde(default)]
    pub secondary_keys: Vec<String>,
    #[serde(default)]
    pub constant: Option<bool>,
    #[serde(default)]
    pub position: Option<String>,

    #[serde(default)]
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

// ============================================================================
// WorldInfoFile - 外部世界书文件
// ============================================================================

/// 外部世界书文件
///
/// ST 后端导入和保存世界书时只强制检查对象中存在 entries 字段。
/// 运行时假设 entries 是以 UID 为 key 的对象。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldInfoFile {
    /// entries 必须是对象形式
    pub entries: HashMap<String, WorldInfoEntry>,

    /// 从角色卡内嵌书导入时，保留原始 CharacterBook
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_data: Option<CharacterBook>,

    /// RST 内部稳定 ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rst_lore_id: Option<String>,

    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub extensions: serde_json::Map<String, serde_json::Value>,

    /// 未知字段保留
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// 世界书条目
///
/// 字段名使用 camelCase，与 ST newWorldInfoEntryTemplate 一致。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldInfoEntry {
    pub uid: i32,

    // 匹配
    #[serde(default)]
    pub key: Vec<String>,
    #[serde(default)]
    pub keysecondary: Vec<String>,
    #[serde(default)]
    pub selective: bool,
    #[serde(default)]
    pub selective_logic: i32,

    // 内容
    #[serde(default)]
    pub comment: String,
    #[serde(default)]
    pub content: String,

    // 状态
    #[serde(default)]
    pub constant: bool,
    #[serde(default)]
    pub vectorized: bool,
    #[serde(default)]
    pub disable: bool,
    #[serde(default)]
    pub add_memo: bool,

    // 排序与位置
    #[serde(default)]
    pub order: i32,
    #[serde(default)]
    pub position: i32,
    #[serde(default)]
    pub depth: i32,
    #[serde(default)]
    pub role: i32,
    #[serde(default)]
    pub outlet_name: String,

    // 预算
    #[serde(default)]
    pub ignore_budget: bool,

    // 递归
    #[serde(default)]
    pub exclude_recursion: bool,
    #[serde(default)]
    pub prevent_recursion: bool,
    #[serde(default)]
    pub delay_until_recursion: serde_json::Value, // number | boolean

    // 概率
    #[serde(default)]
    pub probability: i32,
    #[serde(default)]
    pub use_probability: bool,

    // 分组
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub group_override: bool,
    #[serde(default)]
    pub group_weight: i32,
    #[serde(default)]
    pub use_group_scoring: Option<bool>,

    // 扫描
    #[serde(default)]
    pub scan_depth: Option<i32>,
    #[serde(default)]
    pub case_sensitive: Option<bool>,
    #[serde(default)]
    pub match_whole_words: Option<bool>,

    // 时间控制
    #[serde(default)]
    pub sticky: Option<i32>,
    #[serde(default)]
    pub cooldown: Option<i32>,
    #[serde(default)]
    pub delay: Option<i32>,

    // 匹配目标扩展
    #[serde(default)]
    pub match_persona_description: bool,
    #[serde(default)]
    pub match_character_description: bool,
    #[serde(default)]
    pub match_character_personality: bool,
    #[serde(default)]
    pub match_character_depth_prompt: bool,
    #[serde(default)]
    pub match_scenario: bool,
    #[serde(default)]
    pub match_creator_notes: bool,

    // 自动化
    #[serde(default)]
    pub automation_id: String,
    #[serde(default)]
    pub triggers: Vec<String>,
    #[serde(default)]
    pub display_index: Option<i32>,

    // 角色过滤
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub character_filter: Option<CharacterFilter>,

    // 扩展
    #[serde(default)]
    pub extensions: serde_json::Map<String, serde_json::Value>,

    /// 未知字段保留
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// 角色过滤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterFilter {
    #[serde(default)]
    pub names: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub is_exclude: bool,
}

// ============================================================================
// 枚举默认值
// ============================================================================

impl WorldInfoEntry {
    /// 创建带默认值的新条目
    pub fn new(uid: i32) -> Self {
        Self {
            uid,
            key: Vec::new(),
            keysecondary: Vec::new(),
            selective: true,
            selective_logic: WorldInfoLogic::AND_ANY as i32,
            comment: String::new(),
            content: String::new(),
            constant: false,
            vectorized: false,
            disable: false,
            add_memo: false,
            order: 100,
            position: WorldInfoPosition::BEFORE_CHAR as i32,
            depth: 4,
            role: ExtensionPromptRole::SYSTEM as i32,
            outlet_name: String::new(),
            ignore_budget: false,
            exclude_recursion: false,
            prevent_recursion: false,
            delay_until_recursion: serde_json::json!(0),
            probability: 100,
            use_probability: true,
            group: String::new(),
            group_override: false,
            group_weight: 100,
            use_group_scoring: None,
            scan_depth: None,
            case_sensitive: None,
            match_whole_words: None,
            sticky: None,
            cooldown: None,
            delay: None,
            match_persona_description: false,
            match_character_description: false,
            match_character_personality: false,
            match_character_depth_prompt: false,
            match_scenario: false,
            match_creator_notes: false,
            automation_id: String::new(),
            triggers: Vec::new(),
            display_index: None,
            character_filter: None,
            extensions: serde_json::Map::new(),
            extra: serde_json::Map::new(),
        }
    }
}

/// 世界书逻辑枚举
pub struct WorldInfoLogic;

impl WorldInfoLogic {
    pub const AND_ANY: i32 = 0;
    pub const NOT_ALL: i32 = 1;
    pub const NOT_ANY: i32 = 2;
    pub const AND_ALL: i32 = 3;
}

/// 世界书位置枚举
pub struct WorldInfoPosition;

impl WorldInfoPosition {
    pub const BEFORE_CHAR: i32 = 0;
    pub const AFTER_CHAR: i32 = 1;
    pub const AN_TOP: i32 = 2;
    pub const AN_BOTTOM: i32 = 3;
    pub const AT_DEPTH: i32 = 4;
    pub const EM_TOP: i32 = 5;
    pub const EM_BOTTOM: i32 = 6;
    pub const OUTLET: i32 = 7;
}

/// 扩展提示角色枚举
pub struct ExtensionPromptRole;

impl ExtensionPromptRole {
    pub const SYSTEM: i32 = 0;
    pub const USER: i32 = 1;
    pub const ASSISTANT: i32 = 2;
}

// ============================================================================
// 其他资源类型
// ============================================================================

/// API 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub settings: serde_json::Map<String, serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

/// 预设类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub preset_type: PresetType,
    pub settings: serde_json::Map<String, serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PresetType {
    Sampler,
    Instruct,
    Context,
    Sysprompt,
    Reasoning,
    Prompt,
}

/// 聊天会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub name: String,
    pub character_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

// ============================================================================
// 向后兼容类型别名
// ============================================================================

/// 向后兼容：旧代码使用 CharacterCard
pub type CharacterCard = TavernCardV3;

/// 向后兼容：旧代码使用 Worldbook（数组形式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worldbook {
    #[serde(default)]
    pub entries: Vec<WorldbookEntry>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub scan_depth: Option<i32>,
    #[serde(default)]
    pub token_budget: Option<i32>,
    #[serde(default)]
    pub recursive_scanning: Option<bool>,
    #[serde(default)]
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

/// 向后兼容：旧代码使用 WorldbookEntry（数组形式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldbookEntry {
    pub keys: Vec<String>,
    pub content: String,
    #[serde(default)]
    pub extensions: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub insertion_order: i32,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub id: i32,
    #[serde(default)]
    pub comment: String,
    #[serde(default)]
    pub selective: bool,
    #[serde(default)]
    pub secondary_keys: Vec<String>,
    #[serde(default)]
    pub constant: bool,
    #[serde(default)]
    pub position: String,
}
