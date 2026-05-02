//! Tauri commands for ST mode resources

use crate::storage::json_store::JsonStore;
use crate::storage::paths::{app_data_root, safe_join};
use crate::storage::st_resources::*;
use crate::st::{parse_character_from_png, parse_character_from_json, export_character_to_png, export_character_to_json, convert_character_book};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::AppHandle;
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
pub async fn save_api_config(app: AppHandle, config: ApiConfig) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = serde_json::to_value(&config)
        .map_err(|e| format!("Failed to serialize API config: {}", e))?;

    store.write(&format!("api_configs/{}.json", config.id), &value)
}

#[tauri::command]
pub async fn delete_api_config(app: AppHandle, id: String) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    store.delete(&format!("api_configs/{}.json", id))
}

// ===== Character Commands =====

#[tauri::command]
pub async fn list_characters(app: AppHandle) -> Result<Vec<CharacterCard>, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let files = store.list("characters")?;
    let mut characters = Vec::new();

    for file in files {
        if file.ends_with(".json") {
            if let Ok(value) = store.read(&format!("characters/{}", file)) {
                if let Ok(character) = serde_json::from_value::<CharacterCard>(value) {
                    characters.push(character);
                }
            }
        }
    }

    Ok(characters)
}

#[tauri::command]
pub async fn get_character(app: AppHandle, id: String) -> Result<CharacterCard, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = store.read(&format!("characters/{}.json", id))?;
    serde_json::from_value(value).map_err(|e| format!("Failed to parse character: {}", e))
}

#[tauri::command]
pub async fn save_character(app: AppHandle, id: String, character: CharacterCard) -> Result<(), String> {
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
    std::fs::write(&avatar_path, &png_data)
        .map_err(|e| format!("Failed to save avatar: {}", e))?;

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
pub async fn export_character_as_png(
    app: AppHandle,
    id: String,
) -> Result<Vec<u8>, String> {
    // 读取角色卡
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir.clone());

    let value = store.read(&format!("characters/{}.json", id))?;
    let character: TavernCardV3 = serde_json::from_value(value)
        .map_err(|e| format!("Failed to parse character: {}", e))?;

    // 读取头像 PNG
    let avatar_filename = format!("{}.png", id);
    let avatar_path = safe_join(&data_dir, &format!("characters/{}", avatar_filename))?;

    let png_data = if avatar_path.exists() {
        std::fs::read(&avatar_path)
            .map_err(|e| format!("Failed to read avatar: {}", e))?
    } else {
        // 创建默认头像
        crate::st::character::create_default_avatar_png(&character.data.name)?
    };

    // 导出 PNG
    export_character_to_png(&png_data, &character)
}

/// 导出角色卡为 JSON
#[tauri::command]
pub async fn export_character_as_json(
    app: AppHandle,
    id: String,
) -> Result<Vec<u8>, String> {
    // 读取角色卡
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = store.read(&format!("characters/{}.json", id))?;
    let character: TavernCardV3 = serde_json::from_value(value)
        .map_err(|e| format!("Failed to parse character: {}", e))?;

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
    let character: TavernCardV3 = serde_json::from_value(value)
        .map_err(|e| format!("Failed to parse character: {}", e))?;

    // 获取内嵌世界书
    let character_book = character.data.character_book
        .as_ref()
        .ok_or_else(|| "角色卡没有内嵌世界书".to_string())?;

    // 转换为外部世界书
    let world_info_file = convert_character_book(character_book);

    // 生成世界书 ID 和名称
    let lore_id = Uuid::new_v4().to_string();
    let lore_name = format!("{}_lore", character.data.name);

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
    let character: TavernCardV3 = serde_json::from_value(value)
        .map_err(|e| format!("Failed to parse character: {}", e))?;

    // 导出新 PNG（保留角色卡 metadata）
    let new_png = export_character_to_png(&png_data, &character)?;

    // 保存新头像
    let avatar_path = safe_join(&data_dir, &format!("characters/{}.png", id))?;
    std::fs::write(&avatar_path, &new_png)
        .map_err(|e| format!("Failed to save avatar: {}", e))?;

    Ok(())
}

/// 获取角色卡头像
#[tauri::command]
pub async fn get_character_avatar(
    app: AppHandle,
    id: String,
) -> Result<Vec<u8>, String> {
    let data_dir = get_data_dir(&app)?;
    let avatar_path = safe_join(&data_dir, &format!("characters/{}.png", id))?;

    if avatar_path.exists() {
        std::fs::read(&avatar_path)
            .map_err(|e| format!("Failed to read avatar: {}", e))
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
pub async fn save_worldbook(app: AppHandle, id: String, worldbook: WorldInfoFile) -> Result<(), String> {
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

    store.delete(&format!("lores/{}.json", id))
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
    let mut worldbook: WorldInfoFile = serde_json::from_value(value)
        .map_err(|e| format!("Failed to parse worldbook: {}", e))?;

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
    let mut worldbook: WorldInfoFile = serde_json::from_value(value)
        .map_err(|e| format!("Failed to parse worldbook: {}", e))?;

    // 生成新的 UID：找最大 UID + 1
    let max_uid = worldbook.entries.keys()
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
    let mut worldbook: WorldInfoFile = serde_json::from_value(value)
        .map_err(|e| format!("Failed to parse worldbook: {}", e))?;

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
    let mut worldbook: WorldInfoFile = serde_json::from_value(value)
        .map_err(|e| format!("Failed to parse worldbook: {}", e))?;

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
    let mut worldbook: WorldInfoFile = serde_json::from_value(value)
        .map_err(|e| format!("Failed to parse worldbook: {}", e))?;

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
    let json_str = String::from_utf8(json_data)
        .map_err(|e| format!("Invalid UTF-8: {}", e))?;
    let json_value: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

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
    let worldbook: WorldInfoFile = serde_json::from_value(value)
        .map_err(|e| format!("Failed to parse worldbook: {}", e))?;

    // 导出时保留 ST 兼容格式
    let export_value = serde_json::to_value(&worldbook)
        .map_err(|e| format!("Failed to serialize worldbook: {}", e))?;

    serde_json::to_vec_pretty(&export_value)
        .map_err(|e| format!("Failed to write JSON: {}", e))
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

#[tauri::command]
pub async fn delete_chat_session(app: AppHandle, id: String) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    store.delete(&format!("chats/{}.json", id))
}
