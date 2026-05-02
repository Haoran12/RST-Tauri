//! WorldBook conversion
//!
//! CharacterBook 到 WorldInfoEntry 的转换逻辑。
//! 按照 ST convertCharacterBook 实现。

use std::collections::HashMap;

use crate::storage::st_resources::{
    CharacterBook, CharacterBookEntry, WorldInfoEntry, WorldInfoFile,
    WorldInfoLogic, WorldInfoPosition, ExtensionPromptRole,
};

/// 将 CharacterBook 转换为外部世界书
///
/// 转换规则必须按 ST 当前逻辑实现。
pub fn convert_character_book(book: &CharacterBook) -> WorldInfoFile {
    let mut entries = HashMap::new();

    for (index, entry) in book.entries.iter().enumerate() {
        let uid = entry.id.unwrap_or(index as i32);
        let converted = convert_character_book_entry(entry, index);
        entries.insert(uid.to_string(), converted);
    }

    WorldInfoFile {
        entries,
        original_data: Some(book.clone()),
        rst_lore_id: None,
        name: book.name.clone(),
        description: book.description.clone(),
        extensions: book.extensions.clone(),
        extra: serde_json::Map::new(),
    }
}

/// 转换单个 CharacterBookEntry 为 WorldInfoEntry
fn convert_character_book_entry(entry: &CharacterBookEntry, index: usize) -> WorldInfoEntry {
    let uid = entry.id.unwrap_or(index as i32);

    // 从 extensions 中提取 ST 扩展字段
    let ext = &entry.extensions;

    WorldInfoEntry {
        uid,

        // 匹配
        key: entry.keys.clone(),
        keysecondary: entry.secondary_keys.clone(),
        selective: entry.selective.unwrap_or(false), // 注意：默认值与新建 WorldInfoEntry 不同
        selective_logic: get_ext_i32(ext, "selectiveLogic", WorldInfoLogic::AND_ANY as i32),

        // 内容
        comment: entry.comment.clone(),
        content: entry.content.clone(),

        // 状态
        constant: entry.constant.unwrap_or(false),
        vectorized: get_ext_bool(ext, "vectorized", false),
        disable: !entry.enabled,
        add_memo: !entry.comment.is_empty(),

        // 排序与位置
        order: entry.insertion_order,
        position: convert_position(entry.position.as_deref(), ext),
        depth: get_ext_i32(ext, "depth", 4),
        role: get_ext_i32(ext, "role", ExtensionPromptRole::SYSTEM as i32),
        outlet_name: get_ext_string(ext, "outlet_name", ""),

        // 预算
        ignore_budget: get_ext_bool(ext, "ignore_budget", false),

        // 递归
        exclude_recursion: get_ext_bool(ext, "exclude_recursion", false),
        prevent_recursion: get_ext_bool(ext, "prevent_recursion", false),
        delay_until_recursion: get_ext_delay_until_recursion(ext),

        // 概率
        probability: get_ext_i32(ext, "probability", 100),
        use_probability: get_ext_bool(ext, "useProbability", true),

        // 分组
        group: get_ext_string(ext, "group", ""),
        group_override: get_ext_bool(ext, "group_override", false),
        group_weight: get_ext_i32(ext, "group_weight", 100),
        use_group_scoring: get_ext_opt_bool(ext, "use_group_scoring"),

        // 扫描
        scan_depth: get_ext_opt_i32(ext, "scan_depth"),
        case_sensitive: entry.case_sensitive.or(get_ext_opt_bool(ext, "case_sensitive")),
        match_whole_words: get_ext_opt_bool(ext, "match_whole_words"),

        // 时间控制
        sticky: get_ext_opt_i32(ext, "sticky"),
        cooldown: get_ext_opt_i32(ext, "cooldown"),
        delay: get_ext_opt_i32(ext, "delay"),

        // 匹配目标扩展
        match_persona_description: get_ext_bool(ext, "match_persona_description", false),
        match_character_description: get_ext_bool(ext, "match_character_description", false),
        match_character_personality: get_ext_bool(ext, "match_character_personality", false),
        match_character_depth_prompt: get_ext_bool(ext, "match_character_depth_prompt", false),
        match_scenario: get_ext_bool(ext, "match_scenario", false),
        match_creator_notes: get_ext_bool(ext, "match_creator_notes", false),

        // 自动化
        automation_id: get_ext_string(ext, "automation_id", ""),
        triggers: get_ext_vec_string(ext, "triggers"),
        display_index: get_ext_opt_i32(ext, "display_index").or(Some(index as i32)),

        // 角色过滤
        character_filter: None,

        // 扩展
        extensions: entry.extensions.clone(),
        extra: serde_json::Map::new(),
    }
}

/// 转换 position 字段
fn convert_position(position: Option<&str>, ext: &serde_json::Map<String, serde_json::Value>) -> i32 {
    // 优先使用 extensions.position
    if let Some(pos) = ext.get("position").and_then(|v| v.as_i64()) {
        return pos as i32;
    }

    // 否则使用 entry.position
    match position {
        Some("before_char") => WorldInfoPosition::BEFORE_CHAR as i32,
        Some("after_char") => WorldInfoPosition::AFTER_CHAR as i32,
        _ => WorldInfoPosition::BEFORE_CHAR as i32,
    }
}

// ============================================================================
// 辅助函数：从 extensions 中提取值
// ============================================================================

fn get_ext_bool(ext: &serde_json::Map<String, serde_json::Value>, key: &str, default: bool) -> bool {
    ext.get(key)
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

fn get_ext_opt_bool(ext: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<bool> {
    ext.get(key).and_then(|v| v.as_bool())
}

fn get_ext_i32(ext: &serde_json::Map<String, serde_json::Value>, key: &str, default: i32) -> i32 {
    ext.get(key)
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .unwrap_or(default)
}

fn get_ext_opt_i32(ext: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<i32> {
    ext.get(key).and_then(|v| v.as_i64()).map(|v| v as i32)
}

fn get_ext_string(ext: &serde_json::Map<String, serde_json::Value>, key: &str, default: &str) -> String {
    ext.get(key)
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())
        .unwrap_or_else(|| default.to_string())
}

fn get_ext_vec_string(ext: &serde_json::Map<String, serde_json::Value>, key: &str) -> Vec<String> {
    ext.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn get_ext_delay_until_recursion(ext: &serde_json::Map<String, serde_json::Value>) -> serde_json::Value {
    ext.get("delay_until_recursion")
        .cloned()
        .unwrap_or(serde_json::json!(false))
}

// ============================================================================
// 反向转换：WorldInfoEntry -> CharacterBookEntry
// ============================================================================

/// 将 WorldInfoEntry 转换回 CharacterBookEntry
///
/// 用于导出角色卡时重建 character_book。
pub fn convert_world_info_entry_to_character_book(entry: &WorldInfoEntry) -> CharacterBookEntry {
    let mut extensions = entry.extensions.clone();

    // 写入 ST 扩展字段
    extensions.insert("position".to_string(), serde_json::json!(entry.position));
    extensions.insert("exclude_recursion".to_string(), serde_json::json!(entry.exclude_recursion));
    extensions.insert("prevent_recursion".to_string(), serde_json::json!(entry.prevent_recursion));
    extensions.insert("delay_until_recursion".to_string(), entry.delay_until_recursion.clone());
    extensions.insert("display_index".to_string(), serde_json::json!(entry.display_index));
    extensions.insert("probability".to_string(), serde_json::json!(entry.probability));
    extensions.insert("useProbability".to_string(), serde_json::json!(entry.use_probability));
    extensions.insert("depth".to_string(), serde_json::json!(entry.depth));
    extensions.insert("selectiveLogic".to_string(), serde_json::json!(entry.selective_logic));
    extensions.insert("outlet_name".to_string(), serde_json::json!(entry.outlet_name));
    extensions.insert("group".to_string(), serde_json::json!(entry.group));
    extensions.insert("group_override".to_string(), serde_json::json!(entry.group_override));
    extensions.insert("group_weight".to_string(), serde_json::json!(entry.group_weight));
    extensions.insert("automation_id".to_string(), serde_json::json!(entry.automation_id));
    extensions.insert("role".to_string(), serde_json::json!(entry.role));
    extensions.insert("vectorized".to_string(), serde_json::json!(entry.vectorized));
    extensions.insert("ignore_budget".to_string(), serde_json::json!(entry.ignore_budget));

    if let Some(v) = entry.scan_depth {
        extensions.insert("scan_depth".to_string(), serde_json::json!(v));
    }
    if let Some(v) = entry.case_sensitive {
        extensions.insert("case_sensitive".to_string(), serde_json::json!(v));
    }
    if let Some(v) = entry.match_whole_words {
        extensions.insert("match_whole_words".to_string(), serde_json::json!(v));
    }
    if let Some(v) = entry.use_group_scoring {
        extensions.insert("use_group_scoring".to_string(), serde_json::json!(v));
    }
    if let Some(v) = entry.sticky {
        extensions.insert("sticky".to_string(), serde_json::json!(v));
    }
    if let Some(v) = entry.cooldown {
        extensions.insert("cooldown".to_string(), serde_json::json!(v));
    }
    if let Some(v) = entry.delay {
        extensions.insert("delay".to_string(), serde_json::json!(v));
    }

    extensions.insert("match_persona_description".to_string(), serde_json::json!(entry.match_persona_description));
    extensions.insert("match_character_description".to_string(), serde_json::json!(entry.match_character_description));
    extensions.insert("match_character_personality".to_string(), serde_json::json!(entry.match_character_personality));
    extensions.insert("match_character_depth_prompt".to_string(), serde_json::json!(entry.match_character_depth_prompt));
    extensions.insert("match_scenario".to_string(), serde_json::json!(entry.match_scenario));
    extensions.insert("match_creator_notes".to_string(), serde_json::json!(entry.match_creator_notes));
    extensions.insert("triggers".to_string(), serde_json::json!(entry.triggers));

    CharacterBookEntry {
        keys: entry.key.clone(),
        content: entry.content.clone(),
        enabled: !entry.disable,
        insertion_order: entry.order,
        case_sensitive: entry.case_sensitive,
        name: entry.comment.clone(),
        priority: None,
        id: Some(entry.uid),
        comment: entry.comment.clone(),
        selective: Some(entry.selective),
        secondary_keys: entry.keysecondary.clone(),
        constant: Some(entry.constant),
        position: Some(convert_position_to_string(entry.position)),
        extensions,
    }
}

fn convert_position_to_string(position: i32) -> String {
    match position {
        p if p == WorldInfoPosition::BEFORE_CHAR as i32 => "before_char".to_string(),
        p if p == WorldInfoPosition::AFTER_CHAR as i32 => "after_char".to_string(),
        _ => "before_char".to_string(),
    }
}
