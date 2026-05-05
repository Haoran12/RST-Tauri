//! Tauri commands for ST mode resources

use crate::st::{
    convert_character_book, export_character_to_json, export_character_to_png,
    parse_character_from_json, parse_character_from_png,
};
use crate::storage::attachment_upload_cache::{
    clear_remote_handles, list_remote_handles, AttachmentUploadCacheEntry,
};
use crate::storage::json_store::JsonStore;
use crate::storage::paths::{app_data_root, safe_join};
use crate::storage::st_resources::*;
use crate::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::AppHandle;
use tauri::State;
use uuid::Uuid;

/// Get the data directory path
fn get_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app_data_root(app)
}

// ===== API Config Commands =====

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiConfigList {
    pub configs: Vec<ApiConfig>,
}

#[tauri::command]
pub async fn list_api_configs(app: AppHandle) -> Result<Vec<ApiConfig>, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let files = store.list("api_configs")?;
    let mut configs = Vec::new();

    for file in files {
        if file.ends_with(".json") {
            if let Ok(value) = store.read(&format!("api_configs/{}", file)) {
                if let Ok(config) = serde_json::from_value::<ApiConfig>(value) {
                    configs.push(config);
                }
            }
        }
    }

    Ok(configs)
}

#[tauri::command]
pub async fn get_api_config(app: AppHandle, id: String) -> Result<ApiConfig, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = store.read(&format!("api_configs/{}.json", id))?;
    serde_json::from_value(value).map_err(|e| format!("Failed to parse API config: {}", e))
}

#[tauri::command]
pub async fn save_api_config(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    config: ApiConfig,
) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = serde_json::to_value(&config)
        .map_err(|e| format!("Failed to serialize API config: {}", e))?;

    store.write(&format!("api_configs/{}.json", config.id), &value)?;
    state
        .provider_contract_cache
        .invalidate_api_config(&config.id)
        .await;
    Ok(())
}

#[tauri::command]
pub async fn delete_api_config(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    store.delete(&format!("api_configs/{}.json", id))?;
    state
        .provider_contract_cache
        .invalidate_api_config(&id)
        .await;
    Ok(())
}

// ===== Character Commands =====

#[tauri::command]
pub async fn list_characters(app: AppHandle) -> Result<Vec<CharacterListItem>, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let files = store.list("characters")?;
    let mut characters = Vec::new();

    for file in files {
        if file.ends_with(".json") {
            if let Ok(value) = store.read(&format!("characters/{}", file)) {
                if let Ok(character) = serde_json::from_value::<CharacterCard>(value) {
                    let id = file.trim_end_matches(".json").to_string();
                    characters.push(CharacterListItem { id, character });
                }
            }
        }
    }

    Ok(characters)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CharacterListItem {
    pub id: String,
    pub character: CharacterCard,
}

#[tauri::command]
pub async fn get_character(app: AppHandle, id: String) -> Result<CharacterCard, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = store.read(&format!("characters/{}.json", id))?;
    serde_json::from_value(value).map_err(|e| format!("Failed to parse character: {}", e))
}

#[tauri::command]
pub async fn save_character(
    app: AppHandle,
    id: String,
    character: CharacterCard,
) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = serde_json::to_value(&character)
        .map_err(|e| format!("Failed to serialize character: {}", e))?;

    store.write(&format!("characters/{}.json", id), &value)
}

#[tauri::command]
pub async fn delete_character(app: AppHandle, id: String) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    store.delete(&format!("characters/{}.json", id))
}

// ===== Character Import/Export Commands =====

/// 角色卡导入结果
#[derive(Debug, Serialize, Deserialize)]
pub struct CharacterImportResult {
    pub id: String,
    pub character: TavernCardV3,
    pub has_embedded_worldbook: bool,
    pub avatar_filename: String,
}

/// 从 PNG 数据导入角色卡
#[tauri::command]
pub async fn import_character_from_png(
    app: AppHandle,
    png_data: Vec<u8>,
    _filename: String,
) -> Result<CharacterImportResult, String> {
    // 解析 PNG
    let character = parse_character_from_png(&png_data)?;

    // 生成 ID 和 avatar 文件名
    let id = Uuid::new_v4().to_string();
    let avatar_filename = format!("{}.png", id);

    // 保存角色卡 JSON
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir.clone());

    let value = serde_json::to_value(&character)
        .map_err(|e| format!("Failed to serialize character: {}", e))?;

    store.write(&format!("characters/{}.json", id), &value)?;

    // 保存头像 PNG
    let avatar_path = safe_join(&data_dir, &format!("characters/{}", avatar_filename))?;
    std::fs::write(&avatar_path, &png_data).map_err(|e| format!("Failed to save avatar: {}", e))?;

    // 检查是否有内嵌世界书
    let has_embedded_worldbook = character.data.character_book.is_some();

    Ok(CharacterImportResult {
        id,
        character,
        has_embedded_worldbook,
        avatar_filename,
    })
}

/// 从 JSON 数据导入角色卡
#[tauri::command]
pub async fn import_character_from_json(
    app: AppHandle,
    json_data: Vec<u8>,
    avatar_png: Option<Vec<u8>>,
    _filename: String,
) -> Result<CharacterImportResult, String> {
    // 解析 JSON
    let character = parse_character_from_json(&json_data)?;

    // 生成 ID 和 avatar 文件名
    let id = Uuid::new_v4().to_string();
    let avatar_filename = format!("{}.png", id);

    // 保存角色卡 JSON
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir.clone());

    let value = serde_json::to_value(&character)
        .map_err(|e| format!("Failed to serialize character: {}", e))?;

    store.write(&format!("characters/{}.json", id), &value)?;

    // 保存头像 PNG
    let avatar_path = safe_join(&data_dir, &format!("characters/{}", avatar_filename))?;

    if let Some(png_data) = avatar_png {
        std::fs::write(&avatar_path, &png_data)
            .map_err(|e| format!("Failed to save avatar: {}", e))?;
    } else {
        // 创建默认头像
        let default_avatar = crate::st::character::create_default_avatar_png(&character.data.name)?;
        std::fs::write(&avatar_path, &default_avatar)
            .map_err(|e| format!("Failed to save default avatar: {}", e))?;
    }

    // 检查是否有内嵌世界书
    let has_embedded_worldbook = character.data.character_book.is_some();

    Ok(CharacterImportResult {
        id,
        character,
        has_embedded_worldbook,
        avatar_filename,
    })
}

/// 导出角色卡为 PNG
#[tauri::command]
pub async fn export_character_as_png(app: AppHandle, id: String) -> Result<Vec<u8>, String> {
    // 读取角色卡
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir.clone());

    let value = store.read(&format!("characters/{}.json", id))?;
    let character: TavernCardV3 =
        serde_json::from_value(value).map_err(|e| format!("Failed to parse character: {}", e))?;

    // 读取头像 PNG
    let avatar_filename = format!("{}.png", id);
    let avatar_path = safe_join(&data_dir, &format!("characters/{}", avatar_filename))?;

    let png_data = if avatar_path.exists() {
        std::fs::read(&avatar_path).map_err(|e| format!("Failed to read avatar: {}", e))?
    } else {
        // 创建默认头像
        crate::st::character::create_default_avatar_png(&character.data.name)?
    };

    // 导出 PNG
    export_character_to_png(&png_data, &character)
}

/// 导出角色卡为 JSON
#[tauri::command]
pub async fn export_character_as_json(app: AppHandle, id: String) -> Result<Vec<u8>, String> {
    // 读取角色卡
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = store.read(&format!("characters/{}.json", id))?;
    let character: TavernCardV3 =
        serde_json::from_value(value).map_err(|e| format!("Failed to parse character: {}", e))?;

    // 导出 JSON
    export_character_to_json(&character)
}

/// 导入角色卡内嵌世界书
#[tauri::command]
pub async fn import_embedded_worldbook(
    app: AppHandle,
    character_id: String,
) -> Result<String, String> {
    // 读取角色卡
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir.clone());

    let value = store.read(&format!("characters/{}.json", character_id))?;
    let character: TavernCardV3 =
        serde_json::from_value(value).map_err(|e| format!("Failed to parse character: {}", e))?;

    // 获取内嵌世界书
    let character_book = character
        .data
        .character_book
        .as_ref()
        .ok_or_else(|| "角色卡没有内嵌世界书".to_string())?;

    // 转换为外部世界书
    let mut world_info_file = convert_character_book(character_book);

    // 生成世界书 ID 和名称
    let lore_id = Uuid::new_v4().to_string();
    let lore_name = format!("{}_lore", character.data.name);
    world_info_file.rst_lore_id = Some(lore_id.clone());
    world_info_file.name = lore_name.clone();

    // 保存世界书
    let worldbook_value = serde_json::to_value(&world_info_file)
        .map_err(|e| format!("Failed to serialize worldbook: {}", e))?;

    store.write(&format!("lores/{}.json", lore_id), &worldbook_value)?;

    // 更新角色卡的世界书绑定
    let mut updated_character = character.clone();
    updated_character.data.extensions.insert(
        "world".to_string(),
        serde_json::Value::String(lore_name.clone()),
    );
    updated_character.data.extensions.insert(
        "rst_world_lore_id".to_string(),
        serde_json::Value::String(lore_id.clone()),
    );

    let updated_value = serde_json::to_value(&updated_character)
        .map_err(|e| format!("Failed to serialize updated character: {}", e))?;

    store.write(&format!("characters/{}.json", character_id), &updated_value)?;

    Ok(lore_id)
}

/// 更新角色卡头像
#[tauri::command]
pub async fn update_character_avatar(
    app: AppHandle,
    id: String,
    png_data: Vec<u8>,
) -> Result<(), String> {
    // 读取角色卡
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir.clone());

    let value = store.read(&format!("characters/{}.json", id))?;
    let character: TavernCardV3 =
        serde_json::from_value(value).map_err(|e| format!("Failed to parse character: {}", e))?;

    // 导出新 PNG（保留角色卡 metadata）
    let new_png = export_character_to_png(&png_data, &character)?;

    // 保存新头像
    let avatar_path = safe_join(&data_dir, &format!("characters/{}.png", id))?;
    std::fs::write(&avatar_path, &new_png).map_err(|e| format!("Failed to save avatar: {}", e))?;

    Ok(())
}

/// 获取角色卡头像
#[tauri::command]
pub async fn get_character_avatar(app: AppHandle, id: String) -> Result<Vec<u8>, String> {
    let data_dir = get_data_dir(&app)?;
    let avatar_path = safe_join(&data_dir, &format!("characters/{}.png", id))?;

    if avatar_path.exists() {
        std::fs::read(&avatar_path).map_err(|e| format!("Failed to read avatar: {}", e))
    } else {
        Err("Avatar not found".to_string())
    }
}

// ===== Worldbook Commands =====

/// 世界书列表项
#[derive(Debug, Serialize, Deserialize)]
pub struct WorldbookListItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub entry_count: usize,
}

#[tauri::command]
pub async fn list_worldbooks(app: AppHandle) -> Result<Vec<WorldbookListItem>, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let files = store.list("lores")?;
    let mut worldbooks = Vec::new();

    for file in files {
        if file.ends_with(".json") {
            let id = file.strip_suffix(".json").unwrap_or(&file).to_string();
            if let Ok(value) = store.read(&format!("lores/{}", file)) {
                if let Ok(worldbook) = serde_json::from_value::<WorldInfoFile>(value) {
                    worldbooks.push(WorldbookListItem {
                        id,
                        name: worldbook.name,
                        description: worldbook.description,
                        entry_count: worldbook.entries.len(),
                    });
                }
            }
        }
    }

    Ok(worldbooks)
}

#[tauri::command]
pub async fn get_worldbook(app: AppHandle, id: String) -> Result<WorldInfoFile, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = store.read(&format!("lores/{}.json", id))?;
    serde_json::from_value(value).map_err(|e| format!("Failed to parse worldbook: {}", e))
}

/// 创建新世界书
#[tauri::command]
pub async fn create_worldbook(app: AppHandle, name: String) -> Result<String, String> {
    let id = Uuid::new_v4().to_string();
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let worldbook = WorldInfoFile {
        entries: HashMap::new(),
        original_data: None,
        rst_lore_id: Some(id.clone()),
        name,
        description: String::new(),
        extensions: serde_json::Map::new(),
        extra: serde_json::Map::new(),
    };

    let value = serde_json::to_value(&worldbook)
        .map_err(|e| format!("Failed to serialize worldbook: {}", e))?;

    store.write(&format!("lores/{}.json", id), &value)?;
    Ok(id)
}

#[tauri::command]
pub async fn save_worldbook(
    app: AppHandle,
    id: String,
    worldbook: WorldInfoFile,
) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = serde_json::to_value(&worldbook)
        .map_err(|e| format!("Failed to serialize worldbook: {}", e))?;

    store.write(&format!("lores/{}.json", id), &value)
}

#[tauri::command]
pub async fn delete_worldbook(app: AppHandle, id: String) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    store.delete(&format!("lores/{}.json", id))?;
    cleanup_deleted_worldbook_references(&store, &id)?;
    Ok(())
}

/// 更新世界书元数据（名称、描述）
#[tauri::command]
pub async fn update_worldbook_meta(
    app: AppHandle,
    id: String,
    name: String,
    description: String,
) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = store.read(&format!("lores/{}.json", id))?;
    let mut worldbook: WorldInfoFile =
        serde_json::from_value(value).map_err(|e| format!("Failed to parse worldbook: {}", e))?;

    worldbook.name = name;
    worldbook.description = description;

    let updated_value = serde_json::to_value(&worldbook)
        .map_err(|e| format!("Failed to serialize worldbook: {}", e))?;

    store.write(&format!("lores/{}.json", id), &updated_value)
}

// ===== WorldInfo Entry Commands =====

/// 创建新词条，返回生成的 UID
#[tauri::command]
pub async fn create_worldbook_entry(app: AppHandle, worldbook_id: String) -> Result<i32, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = store.read(&format!("lores/{}.json", worldbook_id))?;
    let mut worldbook: WorldInfoFile =
        serde_json::from_value(value).map_err(|e| format!("Failed to parse worldbook: {}", e))?;

    // 生成新的 UID：找最大 UID + 1
    let max_uid = worldbook
        .entries
        .keys()
        .filter_map(|k| k.parse::<i32>().ok())
        .max()
        .unwrap_or(-1);
    let new_uid = max_uid + 1;

    let entry = WorldInfoEntry::new(new_uid);
    worldbook.entries.insert(new_uid.to_string(), entry);

    let updated_value = serde_json::to_value(&worldbook)
        .map_err(|e| format!("Failed to serialize worldbook: {}", e))?;

    store.write(&format!("lores/{}.json", worldbook_id), &updated_value)?;
    Ok(new_uid)
}

/// 更新单个词条
#[tauri::command]
pub async fn update_worldbook_entry(
    app: AppHandle,
    worldbook_id: String,
    uid: i32,
    entry: WorldInfoEntry,
) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = store.read(&format!("lores/{}.json", worldbook_id))?;
    let mut worldbook: WorldInfoFile =
        serde_json::from_value(value).map_err(|e| format!("Failed to parse worldbook: {}", e))?;

    worldbook.entries.insert(uid.to_string(), entry);

    let updated_value = serde_json::to_value(&worldbook)
        .map_err(|e| format!("Failed to serialize worldbook: {}", e))?;

    store.write(&format!("lores/{}.json", worldbook_id), &updated_value)
}

/// 删除单个词条
#[tauri::command]
pub async fn delete_worldbook_entry(
    app: AppHandle,
    worldbook_id: String,
    uid: i32,
) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = store.read(&format!("lores/{}.json", worldbook_id))?;
    let mut worldbook: WorldInfoFile =
        serde_json::from_value(value).map_err(|e| format!("Failed to parse worldbook: {}", e))?;

    worldbook.entries.remove(&uid.to_string());

    let updated_value = serde_json::to_value(&worldbook)
        .map_err(|e| format!("Failed to serialize worldbook: {}", e))?;

    store.write(&format!("lores/{}.json", worldbook_id), &updated_value)
}

/// 批量更新词条顺序（用于拖拽排序）
#[tauri::command]
pub async fn reorder_worldbook_entries(
    app: AppHandle,
    worldbook_id: String,
    uid_order: Vec<i32>,
) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = store.read(&format!("lores/{}.json", worldbook_id))?;
    let mut worldbook: WorldInfoFile =
        serde_json::from_value(value).map_err(|e| format!("Failed to parse worldbook: {}", e))?;

    // 更新 display_index
    for (index, uid) in uid_order.iter().enumerate() {
        if let Some(entry) = worldbook.entries.get_mut(&uid.to_string()) {
            entry.display_index = Some(index as i32);
        }
    }

    let updated_value = serde_json::to_value(&worldbook)
        .map_err(|e| format!("Failed to serialize worldbook: {}", e))?;

    store.write(&format!("lores/{}.json", worldbook_id), &updated_value)
}

/// 导入世界书 JSON 文件
#[tauri::command]
pub async fn import_worldbook(
    app: AppHandle,
    json_data: Vec<u8>,
    filename: String,
) -> Result<String, String> {
    let id = Uuid::new_v4().to_string();
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    // 解析 JSON
    let json_str = String::from_utf8(json_data).map_err(|e| format!("Invalid UTF-8: {}", e))?;
    let json_value: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // 转换为 WorldInfoFile
    let mut worldbook: WorldInfoFile = serde_json::from_value(json_value.clone())
        .map_err(|e| format!("Failed to parse worldbook: {}", e))?;

    // 设置 ID 和名称
    worldbook.rst_lore_id = Some(id.clone());
    if worldbook.name.is_empty() {
        let name = filename
            .strip_suffix(".json")
            .or_else(|| filename.strip_suffix(".JSON"))
            .unwrap_or(&filename);
        worldbook.name = name.to_string();
    }

    let value = serde_json::to_value(&worldbook)
        .map_err(|e| format!("Failed to serialize worldbook: {}", e))?;

    store.write(&format!("lores/{}.json", id), &value)?;
    Ok(id)
}

/// 导出世界书为 JSON
#[tauri::command]
pub async fn export_worldbook(app: AppHandle, id: String) -> Result<Vec<u8>, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = store.read(&format!("lores/{}.json", id))?;
    let worldbook: WorldInfoFile =
        serde_json::from_value(value).map_err(|e| format!("Failed to parse worldbook: {}", e))?;

    // 导出时保留 ST 兼容格式
    let export_value = serde_json::to_value(&worldbook)
        .map_err(|e| format!("Failed to serialize worldbook: {}", e))?;

    serde_json::to_vec_pretty(&export_value).map_err(|e| format!("Failed to write JSON: {}", e))
}

// ===== Chat Session Commands =====

#[tauri::command]
pub async fn list_chat_sessions(app: AppHandle) -> Result<Vec<ChatSession>, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let files = store.list("chats")?;
    let mut sessions = Vec::new();

    for file in files {
        if file.ends_with(".json") {
            if let Ok(value) = store.read(&format!("chats/{}", file)) {
                if let Ok(session) = serde_json::from_value::<ChatSession>(value) {
                    sessions.push(session);
                }
            }
        }
    }

    Ok(sessions)
}

#[tauri::command]
pub async fn get_chat_session(app: AppHandle, id: String) -> Result<ChatSession, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = store.read(&format!("chats/{}.json", id))?;
    serde_json::from_value(value).map_err(|e| format!("Failed to parse chat session: {}", e))
}

#[tauri::command]
pub async fn save_chat_session(app: AppHandle, session: ChatSession) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = serde_json::to_value(&session)
        .map_err(|e| format!("Failed to serialize chat session: {}", e))?;

    store.write(&format!("chats/{}.json", session.id), &value)
}

fn cleanup_deleted_worldbook_references(
    store: &JsonStore,
    deleted_lore_id: &str,
) -> Result<(), String> {
    cleanup_global_worldbook_references(store, deleted_lore_id)?;
    cleanup_character_worldbook_references(store, deleted_lore_id)?;
    cleanup_chat_worldbook_references(store, deleted_lore_id)?;
    Ok(())
}

fn cleanup_global_worldbook_references(
    store: &JsonStore,
    deleted_lore_id: &str,
) -> Result<(), String> {
    let mut state_value = match store.read("settings/global_state.json") {
        Ok(value) => value,
        Err(_) => return Ok(()),
    };

    let mut changed = false;
    if let Some(global_select) = state_value
        .get_mut("world_info_settings")
        .and_then(|settings| settings.get_mut("global_select"))
        .and_then(|value| value.as_array_mut())
    {
        let original_len = global_select.len();
        global_select.retain(|value| value.as_str() != Some(deleted_lore_id));
        changed |= global_select.len() != original_len;
    }

    if let Some(char_lore) = state_value
        .get_mut("world_info_settings")
        .and_then(|settings| settings.get_mut("char_lore"))
        .and_then(|value| value.as_array_mut())
    {
        for binding in char_lore {
            if let Some(extra_books) = binding
                .get_mut("extra_books")
                .and_then(|value| value.as_array_mut())
            {
                let original_len = extra_books.len();
                extra_books.retain(|value| value.as_str() != Some(deleted_lore_id));
                changed |= extra_books.len() != original_len;
            }
        }
    }

    if changed {
        store.write("settings/global_state.json", &state_value)?;
    }

    Ok(())
}

fn cleanup_character_worldbook_references(
    store: &JsonStore,
    deleted_lore_id: &str,
) -> Result<(), String> {
    let files = store.list("characters")?;
    for file in files {
        if PathBuf::from(&file).extension() != Some(OsStr::new("json")) {
            continue;
        }

        let relative_path = format!("characters/{}", file);
        let mut value = match store.read(&relative_path) {
            Ok(value) => value,
            Err(_) => continue,
        };

        let mut changed = false;
        let bound_lore_id = value
            .get("data")
            .and_then(|data| data.get("extensions"))
            .and_then(|extensions| extensions.get("rst_world_lore_id"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string());
        let world_name = if let Some(world_name) = value
            .get("data")
            .and_then(|data| data.get("extensions"))
            .and_then(|extensions| extensions.get("world"))
            .and_then(|value| value.as_str())
        {
            world_name.to_string()
        } else {
            continue;
        };

        if bound_lore_id.as_deref() == Some(deleted_lore_id) {
            if let Some(extensions) = value
                .get_mut("data")
                .and_then(|data| data.get_mut("extensions"))
                .and_then(|extensions| extensions.as_object_mut())
            {
                extensions.remove("world");
                extensions.remove("rst_world_lore_id");
                changed = true;
            }
        } else if let Some(current_lore_id) = find_character_world_lore_id(store, &world_name)? {
            if current_lore_id == deleted_lore_id {
                if let Some(extensions) = value
                    .get_mut("data")
                    .and_then(|data| data.get_mut("extensions"))
                    .and_then(|extensions| extensions.as_object_mut())
                {
                    extensions.remove("world");
                    extensions.remove("rst_world_lore_id");
                    changed = true;
                }
            }
        }

        if changed {
            store.write(&relative_path, &value)?;
        }
    }

    Ok(())
}

fn find_character_world_lore_id(
    store: &JsonStore,
    world_name: &str,
) -> Result<Option<String>, String> {
    for file in store.list("lores")? {
        if PathBuf::from(&file).extension() != Some(OsStr::new("json")) {
            continue;
        }

        let lore_id = file.trim_end_matches(".json");
        let value = match store.read(&format!("lores/{}", file)) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let worldbook: WorldInfoFile = serde_json::from_value(value)
            .map_err(|e| format!("Failed to parse worldbook {}: {}", lore_id, e))?;
        if worldbook.name == world_name {
            return Ok(Some(lore_id.to_string()));
        }
    }

    Ok(None)
}

fn cleanup_chat_worldbook_references(
    store: &JsonStore,
    deleted_lore_id: &str,
) -> Result<(), String> {
    let files = store.list("chats")?;
    for file in files {
        if PathBuf::from(&file).extension() != Some(OsStr::new("json")) {
            continue;
        }

        let relative_path = format!("chats/{}", file);
        let mut value = match store.read(&relative_path) {
            Ok(value) => value,
            Err(_) => continue,
        };

        let Some(chat_metadata) = value
            .get_mut("chat_metadata")
            .and_then(|meta| meta.as_object_mut())
        else {
            continue;
        };

        let mut changed = false;
        if chat_metadata
            .get("world_info")
            .and_then(|value| value.as_str())
            == Some(deleted_lore_id)
        {
            chat_metadata.insert("world_info".to_string(), serde_json::Value::Null);
            changed = true;
        }

        if let Some(disabled) = chat_metadata
            .get_mut("disabled_world_info")
            .and_then(|value| value.as_array_mut())
        {
            let original_len = disabled.len();
            disabled.retain(|value| value.as_str() != Some(deleted_lore_id));
            changed |= disabled.len() != original_len;
        }

        if changed {
            store.write(&relative_path, &value)?;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_chat_session(app: AppHandle, id: String) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    store.delete(&format!("chats/{}.json", id))
}

// ===== Chat Attachment Commands =====

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveChatAttachmentInput {
    pub filename: String,
    pub mime_type: String,
    pub data: Vec<u8>,
}

const MAX_CHAT_ATTACHMENT_BYTES: usize = 10 * 1024 * 1024;

#[tauri::command]
pub async fn save_chat_attachment(
    app: AppHandle,
    input: SaveChatAttachmentInput,
) -> Result<ChatAttachmentRecord, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir.clone());

    if input.data.len() > MAX_CHAT_ATTACHMENT_BYTES {
        return Err(format!(
            "Attachment exceeds size limit: {} bytes > {} bytes",
            input.data.len(),
            MAX_CHAT_ATTACHMENT_BYTES
        ));
    }

    let kind = classify_attachment_kind(&input.mime_type, &input.filename, &input.data)?;
    let attachment_id = Uuid::new_v4().to_string();
    let extension = filename_extension(&input.filename, &kind);
    let blob_filename = format!("blob{}", extension);
    let attachment_dir = safe_join(&data_dir, &format!("chat_attachments/{}", attachment_id))?;
    tokio::fs::create_dir_all(&attachment_dir)
        .await
        .map_err(|e| format!("Failed to create attachment directory: {}", e))?;

    let blob_path = safe_join(&attachment_dir, &blob_filename)?;
    tokio::fs::write(&blob_path, &input.data)
        .await
        .map_err(|e| format!("Failed to write attachment blob: {}", e))?;

    let size_bytes = input.data.len() as u64;
    let record = ChatAttachmentRecord {
        attachment_id: attachment_id.clone(),
        kind,
        mime_type: input.mime_type,
        filename: input.filename,
        blob_filename,
        size_bytes,
        created_at: Utc::now().to_rfc3339(),
    };

    let value = serde_json::to_value(&record)
        .map_err(|e| format!("Failed to serialize attachment record: {}", e))?;
    store.write(
        &format!("chat_attachments/{}/meta.json", attachment_id),
        &value,
    )?;

    Ok(record)
}

#[tauri::command]
pub async fn get_chat_attachment(
    app: AppHandle,
    attachment_id: String,
) -> Result<ChatAttachmentRecord, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);
    let value = store.read(&format!("chat_attachments/{}/meta.json", attachment_id))?;
    serde_json::from_value(value).map_err(|e| format!("Failed to parse attachment metadata: {}", e))
}

#[tauri::command]
pub async fn get_chat_attachment_blob(
    app: AppHandle,
    attachment_id: String,
) -> Result<Vec<u8>, String> {
    let data_dir = get_data_dir(&app)?;
    let record = load_attachment_record(&data_dir, &attachment_id)?;
    let blob_path = safe_join(
        &data_dir,
        &format!(
            "chat_attachments/{}/{}",
            attachment_id, record.blob_filename
        ),
    )?;
    tokio::fs::read(&blob_path)
        .await
        .map_err(|e| format!("Failed to read attachment blob {}: {}", attachment_id, e))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AttachmentUploadCacheDiagnostics {
    pub attachment_id: String,
    pub entries: Vec<AttachmentUploadCacheEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClearAttachmentUploadCacheResult {
    pub attachment_id: String,
    pub removed_entries: usize,
}

#[tauri::command]
pub async fn list_attachment_upload_cache(
    app: AppHandle,
    attachment_id: String,
) -> Result<AttachmentUploadCacheDiagnostics, String> {
    let data_dir = get_data_dir(&app)?;
    Ok(AttachmentUploadCacheDiagnostics {
        attachment_id: attachment_id.clone(),
        entries: list_remote_handles(&data_dir, &attachment_id)?,
    })
}

#[tauri::command]
pub async fn clear_attachment_upload_cache(
    app: AppHandle,
    attachment_id: String,
    api_config_id: Option<String>,
) -> Result<ClearAttachmentUploadCacheResult, String> {
    let data_dir = get_data_dir(&app)?;
    let removed_entries =
        clear_remote_handles(&data_dir, &attachment_id, api_config_id.as_deref())?;
    Ok(ClearAttachmentUploadCacheResult {
        attachment_id,
        removed_entries,
    })
}

fn classify_attachment_kind(
    mime_type: &str,
    filename: &str,
    data: &[u8],
) -> Result<ChatAttachmentKind, String> {
    let mime = mime_type.to_ascii_lowercase();
    let magic_kind = classify_attachment_magic(data);

    if mime.starts_with("image/") {
        return match magic_kind {
            Some(ChatAttachmentKind::Image) => Ok(ChatAttachmentKind::Image),
            Some(ChatAttachmentKind::Pdf) => {
                Err("Attachment MIME says image but file header is PDF".to_string())
            }
            None => Err("Unsupported or unrecognized image attachment header".to_string()),
        };
    }
    if mime == "application/pdf" {
        return match magic_kind {
            Some(ChatAttachmentKind::Pdf) => Ok(ChatAttachmentKind::Pdf),
            Some(ChatAttachmentKind::Image) => {
                Err("Attachment MIME says PDF but file header is image".to_string())
            }
            None => Err("Unsupported or unrecognized PDF attachment header".to_string()),
        };
    }
    let lower_name = filename.to_ascii_lowercase();
    if lower_name.ends_with(".pdf") {
        return match magic_kind {
            Some(ChatAttachmentKind::Pdf) => Ok(ChatAttachmentKind::Pdf),
            Some(ChatAttachmentKind::Image) => {
                Err("Attachment filename says PDF but file header is image".to_string())
            }
            None => Err("Unsupported or unrecognized PDF attachment header".to_string()),
        };
    }
    Err(format!("Unsupported attachment type: {}", mime_type))
}

fn classify_attachment_magic(data: &[u8]) -> Option<ChatAttachmentKind> {
    if data.starts_with(b"%PDF-") {
        return Some(ChatAttachmentKind::Pdf);
    }
    if data.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a])
        || data.starts_with(&[0xff, 0xd8, 0xff])
        || data.starts_with(b"GIF87a")
        || data.starts_with(b"GIF89a")
        || data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WEBP")
    {
        return Some(ChatAttachmentKind::Image);
    }
    None
}

fn load_attachment_record(
    data_dir: &std::path::Path,
    attachment_id: &str,
) -> Result<ChatAttachmentRecord, String> {
    let meta_path = safe_join(
        data_dir,
        &format!("chat_attachments/{}/meta.json", attachment_id),
    )?;
    let text = std::fs::read_to_string(&meta_path).map_err(|e| {
        format!(
            "Failed to read attachment metadata {}: {}",
            attachment_id, e
        )
    })?;
    serde_json::from_str(&text).map_err(|e| {
        format!(
            "Failed to parse attachment metadata {}: {}",
            attachment_id, e
        )
    })
}

fn filename_extension(filename: &str, kind: &ChatAttachmentKind) -> String {
    std::path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| format!(".{}", ext))
        .unwrap_or_else(|| match kind {
            ChatAttachmentKind::Image => ".bin".to_string(),
            ChatAttachmentKind::Pdf => ".pdf".to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::{
        classify_attachment_kind, cleanup_deleted_worldbook_references, MAX_CHAT_ATTACHMENT_BYTES,
    };
    use crate::storage::json_store::JsonStore;
    use crate::storage::st_resources::ChatAttachmentKind;
    use serde_json::json;

    #[test]
    fn deleting_worldbook_cleans_global_and_chat_references() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let store = JsonStore::new(temp_dir.path().to_path_buf());

        store
            .write(
                "settings/global_state.json",
                &json!({
                    "world_info_settings": {
                        "global_select": ["lore-a", "lore-b"],
                        "char_lore": [
                            { "name": "Hero", "extra_books": ["lore-a", "lore-c"] }
                        ]
                    }
                }),
            )
            .expect("write global state");
        store
            .write(
                "chats/session-1.json",
                &json!({
                    "id": "session-1",
                    "name": "Test",
                    "created_at": "",
                    "updated_at": "",
                    "chat_metadata": {
                        "world_info": "lore-a",
                        "disabled_world_info": ["lore-a", "lore-z"]
                    },
                    "messages": []
                }),
            )
            .expect("write session");

        cleanup_deleted_worldbook_references(&store, "lore-a").expect("cleanup refs");

        let global_state = store
            .read("settings/global_state.json")
            .expect("read global state");
        assert_eq!(
            global_state["world_info_settings"]["global_select"],
            json!(["lore-b"])
        );
        assert_eq!(
            global_state["world_info_settings"]["char_lore"][0]["extra_books"],
            json!(["lore-c"])
        );

        let chat = store.read("chats/session-1.json").expect("read chat");
        assert!(chat["chat_metadata"]["world_info"].is_null());
        assert_eq!(
            chat["chat_metadata"]["disabled_world_info"],
            json!(["lore-z"])
        );
    }

    #[test]
    fn chat_attachment_limit_is_ten_megabytes() {
        assert_eq!(MAX_CHAT_ATTACHMENT_BYTES, 10 * 1024 * 1024);
    }

    #[test]
    fn attachment_kind_requires_matching_magic_bytes() {
        let png = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0x00];
        assert_eq!(
            classify_attachment_kind("image/png", "image.png", &png).unwrap(),
            ChatAttachmentKind::Image
        );

        assert!(classify_attachment_kind("image/png", "image.png", b"%PDF-1.7").is_err());
        assert!(classify_attachment_kind("application/pdf", "doc.pdf", b"not-a-pdf").is_err());
    }
}
