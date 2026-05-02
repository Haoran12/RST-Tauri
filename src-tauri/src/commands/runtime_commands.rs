//! Tauri commands for ST runtime assembly
//!
//! 运行时组装相关命令：全局状态管理、预设加载、请求组装等。

use crate::storage::json_store::JsonStore;
use crate::storage::paths::app_data_root;
use crate::storage::st_resources::{ApiConfig, TavernCardV3, WorldInfoFile, ChatSession};
use crate::st::{
    GlobalAppState, STWorldInfoSettings, STSessionData, STChatMetadata, STChatMessage,
    RuntimeContext, WorldInfoInjectionResult, RequestAssembler, AssembledRequest,
    ProviderRequestMapper, WorldInfoInjector, WorldInfoSource,
    SamplerPreset, InstructTemplate, ContextTemplate, SystemPrompt,
    ReasoningTemplate, PromptPreset,
};
use crate::st::keyword_matcher::GlobalScanData;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use tauri::AppHandle;

/// Get the data directory path
fn get_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app_data_root(app)
}

// ============================================================================
// 全局应用状态命令
// ============================================================================

/// 获取全局应用状态
#[tauri::command]
pub async fn get_global_state(
    app: AppHandle,
) -> Result<GlobalAppState, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    // 尝试从文件加载
    match store.read("settings/global_state.json") {
        Ok(value) => {
            serde_json::from_value(value)
                .map_err(|e| format!("Failed to parse global state: {}", e))
        }
        Err(_) => {
            // 返回默认值
            Ok(GlobalAppState::default())
        }
    }
}

/// 保存全局应用状态
#[tauri::command]
pub async fn save_global_state(
    app: AppHandle,
    state: GlobalAppState,
) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = serde_json::to_value(&state)
        .map_err(|e| format!("Failed to serialize global state: {}", e))?;

    store.write("settings/global_state.json", &value)
}

/// 更新激活的 API 配置
#[tauri::command]
pub async fn set_active_api_config(
    app: AppHandle,
    api_config_id: Option<String>,
) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    // 加载当前状态
    let mut state: GlobalAppState = match store.read("settings/global_state.json") {
        Ok(value) => serde_json::from_value(value).unwrap_or_default(),
        Err(_) => GlobalAppState::default(),
    };

    // 更新
    state.active_api_config_id = api_config_id;

    // 保存
    let value = serde_json::to_value(&state)
        .map_err(|e| format!("Failed to serialize global state: {}", e))?;

    store.write("settings/global_state.json", &value)
}

/// 更新激活的预设
#[tauri::command]
pub async fn set_active_presets(
    app: AppHandle,
    sampler: Option<String>,
    instruct: Option<String>,
    context: Option<String>,
    sysprompt: Option<String>,
    reasoning: Option<String>,
    prompt: Option<String>,
) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    // 加载当前状态
    let mut state: GlobalAppState = match store.read("settings/global_state.json") {
        Ok(value) => serde_json::from_value(value).unwrap_or_default(),
        Err(_) => GlobalAppState::default(),
    };

    // 更新
    if let Some(s) = sampler { state.active_sampler_preset = s; }
    if let Some(s) = instruct { state.active_instruct_preset = s; }
    if let Some(s) = context { state.active_context_preset = s; }
    if let Some(s) = sysprompt { state.active_sysprompt_preset = s; }
    if let Some(s) = reasoning { state.active_reasoning_preset = s; }
    if let Some(s) = prompt { state.active_prompt_preset = s; }

    // 保存
    let value = serde_json::to_value(&state)
        .map_err(|e| format!("Failed to serialize global state: {}", e))?;

    store.write("settings/global_state.json", &value)
}

// ============================================================================
// 预设加载命令
// ============================================================================

/// 加载 Sampler 预设
#[tauri::command]
pub async fn load_sampler_preset(app: AppHandle, name: String) -> Result<SamplerPreset, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = store.read(&format!("presets/samplers/{}.json", name))?;
    serde_json::from_value(value).map_err(|e| format!("Failed to parse sampler preset: {}", e))
}

/// 加载 Instruct 模板
#[tauri::command]
pub async fn load_instruct_template(app: AppHandle, name: String) -> Result<InstructTemplate, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = store.read(&format!("presets/instruct/{}.json", name))?;
    serde_json::from_value(value).map_err(|e| format!("Failed to parse instruct template: {}", e))
}

/// 加载 Context 模板
#[tauri::command]
pub async fn load_context_template(app: AppHandle, name: String) -> Result<ContextTemplate, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = store.read(&format!("presets/context/{}.json", name))?;
    serde_json::from_value(value).map_err(|e| format!("Failed to parse context template: {}", e))
}

/// 加载 System Prompt
#[tauri::command]
pub async fn load_system_prompt(app: AppHandle, name: String) -> Result<SystemPrompt, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = store.read(&format!("presets/sysprompt/{}.json", name))?;
    serde_json::from_value(value).map_err(|e| format!("Failed to parse system prompt: {}", e))
}

/// 加载 Reasoning 模板
#[tauri::command]
pub async fn load_reasoning_template(app: AppHandle, name: String) -> Result<ReasoningTemplate, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = store.read(&format!("presets/reasoning/{}.json", name))?;
    serde_json::from_value(value).map_err(|e| format!("Failed to parse reasoning template: {}", e))
}

/// 加载 Prompt 预设
#[tauri::command]
pub async fn load_prompt_preset(app: AppHandle, name: String) -> Result<PromptPreset, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    let value = store.read(&format!("presets/prompts/{}.json", name))?;
    serde_json::from_value(value).map_err(|e| format!("Failed to parse prompt preset: {}", e))
}

// ============================================================================
// 运行时组装命令
// ============================================================================

/// 组装请求的输入参数
#[derive(Debug, Serialize, Deserialize)]
pub struct AssembleRequestInput {
    pub api_config_id: String,
    pub character_id: Option<String>,
    pub session_id: String,
    pub sampler_preset: Option<String>,
    pub instruct_template: Option<String>,
    pub context_template: Option<String>,
    pub system_prompt: Option<String>,
    pub reasoning_template: Option<String>,
    pub prompt_preset: Option<String>,
    pub world_info_settings: STWorldInfoSettings,
    #[serde(default)]
    pub chat_lore_id: Option<String>,
    #[serde(default)]
    pub global_lore_ids: Vec<String>,
    #[serde(default = "default_max_context")]
    pub max_context: i32,
}

/// 组装请求的输出
#[derive(Debug, Serialize, Deserialize)]
pub struct AssembleRequestOutput {
    pub request: AssembledRequest,
    pub provider_type: String,
    pub model: String,
    pub world_info_result: Option<WorldInfoInjectionResult>,
}

fn default_max_context() -> i32 {
    8192
}

/// 组装 ST 聊天请求
#[tauri::command]
pub async fn assemble_st_request(
    app: AppHandle,
    input: AssembleRequestInput,
) -> Result<AssembleRequestOutput, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    // 1. 加载 API 配置
    let api_config: ApiConfig = {
        let value = store.read(&format!("api_configs/{}.json", input.api_config_id))?;
        serde_json::from_value(value).map_err(|e| format!("Failed to parse API config: {}", e))?
    };

    // 2. 加载角色卡
    let character: Option<TavernCardV3> = if let Some(char_id) = &input.character_id {
        let value = store.read(&format!("characters/{}.json", char_id))?;
        Some(serde_json::from_value(value).map_err(|e| format!("Failed to parse character: {}", e))?)
    } else {
        None
    };

    // 3. 加载会话
    let session_value = store.read(&format!("chats/{}.json", input.session_id))?;
    let chat_session: ChatSession = serde_json::from_value(session_value)
        .map_err(|e| format!("Failed to parse chat session: {}", e))?;

    // 转换为 STSessionData
    let session = STSessionData {
        session_id: chat_session.id,
        character_id: chat_session.character_id,
        group_id: None,
        chat_metadata: STChatMetadata::default(),
        messages: chat_session.messages.into_iter().map(|m| STChatMessage {
            id: m.id,
            role: m.role,
            content: m.content,
            created_at: m.created_at,
            name: None,
        }).collect(),
    };

    // 4. 加载预设
    let sampler_preset = if !input.sampler_preset.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
        let name = input.sampler_preset.unwrap();
        let value = store.read(&format!("presets/samplers/{}.json", name))?;
        Some(serde_json::from_value(value).map_err(|e| format!("Failed to parse sampler preset: {}", e))?)
    } else {
        None
    };

    let instruct_template = if !input.instruct_template.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
        let name = input.instruct_template.unwrap();
        let value = store.read(&format!("presets/instruct/{}.json", name))?;
        Some(serde_json::from_value(value).map_err(|e| format!("Failed to parse instruct template: {}", e))?)
    } else {
        None
    };

    let context_template = if !input.context_template.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
        let name = input.context_template.unwrap();
        let value = store.read(&format!("presets/context/{}.json", name))?;
        Some(serde_json::from_value(value).map_err(|e| format!("Failed to parse context template: {}", e))?)
    } else {
        None
    };

    let system_prompt = if !input.system_prompt.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
        let name = input.system_prompt.unwrap();
        let value = store.read(&format!("presets/sysprompt/{}.json", name))?;
        Some(serde_json::from_value(value).map_err(|e| format!("Failed to parse system prompt: {}", e))?)
    } else {
        None
    };

    let reasoning_template = if !input.reasoning_template.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
        let name = input.reasoning_template.unwrap();
        let value = store.read(&format!("presets/reasoning/{}.json", name))?;
        Some(serde_json::from_value(value).map_err(|e| format!("Failed to parse reasoning template: {}", e))?)
    } else {
        None
    };

    let prompt_preset = if !input.prompt_preset.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
        let name = input.prompt_preset.unwrap();
        let value = store.read(&format!("presets/prompts/{}.json", name))?;
        Some(serde_json::from_value(value).map_err(|e| format!("Failed to parse prompt preset: {}", e))?)
    } else {
        None
    };

    // 5. 构建全局扫描数据
    let global_scan_data = GlobalScanData {
        persona_description: String::new(),
        character_description: character.as_ref().map(|c| c.data.description.clone()).unwrap_or_default(),
        character_personality: character.as_ref().map(|c| c.data.personality.clone()).unwrap_or_default(),
        character_depth_prompt: String::new(),
        scenario: character.as_ref().map(|c| c.data.scenario.clone()).unwrap_or_default(),
        creator_notes: character.as_ref().map(|c| c.data.creator_notes.clone()).unwrap_or_default(),
        trigger: None,
    };

    // 6. 执行世界书注入
    let world_info_result = {
        let mut global_lore_ids = input.global_lore_ids.clone();
        for lore_id in &input.world_info_settings.global_select {
            if !global_lore_ids.contains(lore_id) {
                global_lore_ids.push(lore_id.clone());
            }
        }
        let sources = collect_world_info_sources(
            &store,
            character.as_ref(),
            &input.world_info_settings,
            input.chat_lore_id.as_deref(),
            &global_lore_ids,
        )?;
        if sources.is_empty() {
            None
        } else {
            let mut injector = WorldInfoInjector::new();
            Some(injector.check_world_info(
                &session.messages,
                input.max_context,
                &input.world_info_settings,
                sources,
                &global_scan_data,
            ).await)
        }
    };

    // 7. 构建运行时上下文
    let context = RuntimeContext {
        api_config: api_config.clone(),
        sampler_preset,
        instruct_template,
        context_template,
        system_prompt,
        reasoning_template,
        prompt_preset,
        character,
        session,
        global_scan_data,
        world_info_result: world_info_result.clone(),
    };

    // 8. 组装请求
    let request = RequestAssembler::assemble(&context);

    Ok(AssembleRequestOutput {
        request,
        provider_type: api_config.provider.clone(),
        model: api_config.model.clone(),
        world_info_result,
    })
}

// ============================================================================
// 世界书注入命令
// ============================================================================

/// 世界书注入输入
#[derive(Debug, Serialize, Deserialize)]
pub struct WorldInfoInjectionInput {
    pub session_id: String,
    pub character_id: Option<String>,
    pub world_info_settings: STWorldInfoSettings,
    pub chat_lore_id: Option<String>,
    pub global_lore_ids: Vec<String>,
    pub max_context: i32,
}

/// 执行世界书注入
#[tauri::command]
pub async fn run_world_info_injection(
    app: AppHandle,
    input: WorldInfoInjectionInput,
) -> Result<WorldInfoInjectionResult, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    // 1. 加载会话消息
    let session_value = store.read(&format!("chats/{}.json", input.session_id))?;
    let chat_session: ChatSession = serde_json::from_value(session_value)
        .map_err(|e| format!("Failed to parse chat session: {}", e))?;

    let messages: Vec<STChatMessage> = chat_session.messages.into_iter().map(|m| STChatMessage {
        id: m.id,
        role: m.role,
        content: m.content,
        created_at: m.created_at,
        name: None,
    }).collect();

    // 2. 加载角色卡并收集世界书来源
    let character = if let Some(char_id) = &input.character_id {
        let value = store.read(&format!("characters/{}.json", char_id))?;
        Some(serde_json::from_value::<TavernCardV3>(value)
            .map_err(|e| format!("Failed to parse character: {}", e))?)
    } else {
        None
    };

    let mut global_lore_ids = input.global_lore_ids.clone();
    for lore_id in &input.world_info_settings.global_select {
        if !global_lore_ids.contains(lore_id) {
            global_lore_ids.push(lore_id.clone());
        }
    }

    let sources = collect_world_info_sources(
        &store,
        character.as_ref(),
        &input.world_info_settings,
        input.chat_lore_id.as_deref(),
        &global_lore_ids,
    )?;

    // 3. 构建全局扫描数据
    let global_scan_data = GlobalScanData {
        persona_description: String::new(),
        character_description: character.as_ref().map(|c| c.data.description.clone()).unwrap_or_default(),
        character_personality: character.as_ref().map(|c| c.data.personality.clone()).unwrap_or_default(),
        character_depth_prompt: String::new(),
        scenario: character.as_ref().map(|c| c.data.scenario.clone()).unwrap_or_default(),
        creator_notes: character.as_ref().map(|c| c.data.creator_notes.clone()).unwrap_or_default(),
        trigger: None,
    };

    // 4. 执行注入
    let mut injector = WorldInfoInjector::new();
    Ok(injector.check_world_info(
        &messages,
        input.max_context,
        &input.world_info_settings,
        sources,
        &global_scan_data,
    ).await)
}

fn collect_world_info_sources(
    store: &JsonStore,
    character: Option<&TavernCardV3>,
    settings: &STWorldInfoSettings,
    chat_lore_id: Option<&str>,
    global_lore_ids: &[String],
) -> Result<Vec<WorldInfoSource>, String> {
    let mut sources: Vec<WorldInfoSource> = Vec::new();
    let mut seen_lore_ids: HashSet<String> = HashSet::new();

    if let Some(lore_id) = chat_lore_id {
        if let Some(worldbook) = load_worldbook_by_id(store, lore_id)? {
            seen_lore_ids.insert(lore_id.to_string());
            sources.push(WorldInfoSource::ChatLore(worldbook));
        }
    }

    for lore_id in global_lore_ids {
        if !seen_lore_ids.insert(lore_id.clone()) {
            continue;
        }
        if let Some(worldbook) = load_worldbook_by_id(store, lore_id)? {
            sources.push(WorldInfoSource::GlobalLore(worldbook));
        }
    }

    if let Some(character) = character {
        for lore_id in character_lore_ids(character, settings, store)? {
            if !seen_lore_ids.insert(lore_id.clone()) {
                continue;
            }
            if let Some(worldbook) = load_worldbook_by_id(store, &lore_id)? {
                sources.push(WorldInfoSource::CharacterLore(worldbook));
            }
        }
    }

    Ok(sources)
}

fn load_worldbook_by_id(store: &JsonStore, lore_id: &str) -> Result<Option<WorldInfoFile>, String> {
    let value = store.read(&format!("lores/{}.json", lore_id))?;
    let worldbook = serde_json::from_value::<WorldInfoFile>(value)
        .map_err(|e| format!("Failed to parse worldbook {}: {}", lore_id, e))?;
    Ok(Some(worldbook))
}

fn character_lore_ids(
    character: &TavernCardV3,
    settings: &STWorldInfoSettings,
    store: &JsonStore,
) -> Result<Vec<String>, String> {
    let mut ids = Vec::new();

    if let Some(world_name) = character.data.extensions.get("world").and_then(|v| v.as_str()) {
        if let Some(lore_id) = find_lore_id_by_name(store, world_name)? {
            ids.push(lore_id);
        }
    }

    for binding in &settings.char_lore {
        if binding.name == character.data.name {
            ids.extend(binding.extra_books.iter().cloned());
        }
    }

    Ok(ids)
}

fn find_lore_id_by_name(store: &JsonStore, world_name: &str) -> Result<Option<String>, String> {
    for file in store.list("lores")? {
        if !file.ends_with(".json") {
            continue;
        }
        let id = file.strip_suffix(".json").unwrap_or(&file);
        let value = store.read(&format!("lores/{}", file))?;
        let worldbook = serde_json::from_value::<WorldInfoFile>(value)
            .map_err(|e| format!("Failed to parse worldbook {}: {}", id, e))?;
        if worldbook.name == world_name {
            return Ok(Some(id.to_string()));
        }
    }
    Ok(None)
}

// ============================================================================
// Provider 请求映射命令
// ============================================================================

/// 映射请求到 Provider 格式
#[tauri::command]
pub async fn map_request_to_provider(
    request: AssembledRequest,
    provider_type: String,
    model: String,
) -> Result<serde_json::Value, String> {
    match provider_type.as_str() {
        "openai_responses" => Ok(ProviderRequestMapper::map_to_openai_responses(&request, &model)),
        "openai_chat" => Ok(ProviderRequestMapper::map_to_openai_chat(&request, &model)),
        "deepseek" => Ok(ProviderRequestMapper::map_to_deepseek(&request, &model)),
        "anthropic" => Ok(ProviderRequestMapper::map_to_anthropic(&request, &model)),
        "gemini" => Ok(ProviderRequestMapper::map_to_gemini(&request, &model)),
        "claude_code" => Ok(ProviderRequestMapper::map_to_claude_code(&request)),
        _ => Err(format!("Unknown provider type: {}", provider_type)),
    }
}
