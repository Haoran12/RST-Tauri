//! Tauri commands for ST runtime assembly
//!
//! 运行时组装相关命令：全局状态管理、预设加载、请求组装等。

use crate::api::provider::{
    ChatMessage as ProviderChatMessage, ChatRequest, ChatRole, ContentPart, FileRef, ImageUrl,
    ReasoningParams, SamplingParams,
};
use crate::commands::chat_commands::{create_provider, ChatResponseData};
use crate::config::llm_contracts::{
    connection_supports_attachments, load_llm_api_contracts_snapshot_from_str,
    CompiledProviderContractView,
};
use crate::logging::context::{LlmNode, LogContext, LogMode};
use crate::st::keyword_matcher::GlobalScanData;
use crate::st::runtime_assembly::{AssembledAttachmentRef, AssembledSamplingParams};
use crate::st::{
    AssembledRequest, ContextTemplate, GlobalAppState, InstructTemplate, MacroContext, PresetFile,
    PromptPreset, ProviderRequestMapper, ReasoningTemplate, RegexEngine, RegexPlacement,
    RegexRunOptions, RequestAssembler, RuntimeContext, STChatMessage, STChatMetadata,
    STSessionData, STWorldInfoSettings, SamplerPreset, SystemPrompt, WorldInfoInjectionResult,
    WorldInfoInjector, WorldInfoSource,
};
use crate::storage::json_store::JsonStore;
use crate::storage::paths::{app_data_root, safe_join};
use crate::storage::st_resources::{
    ApiConfig, ChatAttachmentKind, ChatAttachmentRecord, ChatSession, TavernCardV3, WorldInfoFile,
};
use crate::AppState;
use base64::{engine::general_purpose::STANDARD, Engine};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

/// Stream event payload sent to frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunkEvent {
    pub stream_id: String,
    pub delta: String,
    pub finish_reason: Option<String>,
}

/// Stream error event payload sent to frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamErrorEvent {
    pub stream_id: String,
    pub error: String,
}

/// Stream start event payload sent to frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStartEvent {
    pub stream_id: String,
    pub request_id: String,
}

async fn get_compiled_contract_view(
    state: &Arc<AppState>,
    api_config: &ApiConfig,
) -> Result<Arc<CompiledProviderContractView>, String> {
    let snapshot = {
        let guard = state.llm_api_contracts.read().await;
        guard.as_ref().cloned()
    };

    let snapshot = match snapshot {
        Some(snapshot) => snapshot,
        None => {
            let snapshot = Arc::new(load_llm_api_contracts_snapshot_from_str(include_str!(
                "../../../config/llm_api_contracts.json"
            ))?);
            *state.llm_api_contracts.write().await = Some(snapshot.clone());
            snapshot
        }
    };

    state
        .provider_contract_cache
        .get_or_insert(snapshot.as_ref(), api_config)
        .await
}

/// Get the data directory path
fn get_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app_data_root(app)
}

/// Build request URL for logging based on provider type
fn build_request_url(api_config: &ApiConfig) -> Option<String> {
    let base_url = match api_config.base_url.clone() {
        Some(url) => url,
        None => match api_config.provider.as_str() {
            "openai_chat" | "openai_responses" => "https://api.openai.com/v1".to_string(),
            "anthropic" => "https://api.anthropic.com/v1".to_string(),
            "gemini" => "https://generativelanguage.googleapis.com/v1beta".to_string(),
            "deepseek" => "https://api.deepseek.com".to_string(),
            "claude_code" => "http://localhost:8080".to_string(),
            _ => return None,
        },
    };

    let endpoint = match api_config.provider.as_str() {
        "openai_chat" => "/chat/completions",
        "openai_responses" => "/responses",
        "anthropic" => "/messages",
        "gemini" => &format!(":{}", api_config.model),
        "deepseek" => "/chat/completions",
        "claude_code" => "/v1/chat",
        _ => return Some(base_url),
    };

    Some(format!("{}{}", base_url, endpoint))
}

// ============================================================================
// 全局应用状态命令
// ============================================================================

/// 获取全局应用状态
#[tauri::command]
pub async fn get_global_state(app: AppHandle) -> Result<GlobalAppState, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);
    ensure_default_presets(&store)?;
    load_global_state(&store)
}

/// 保存全局应用状态
#[tauri::command]
pub async fn save_global_state(app: AppHandle, state: GlobalAppState) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);
    ensure_default_presets(&store)?;

    let mut state = state;
    if state.active_preset.trim().is_empty() {
        state.active_preset = default_preset_name().to_string();
    }

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

/// 更新激活的完整预设
#[tauri::command]
pub async fn set_active_preset(app: AppHandle, preset_name: String) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);
    ensure_default_presets(&store)?;

    // 加载当前状态
    let mut state: GlobalAppState = match store.read("settings/global_state.json") {
        Ok(value) => serde_json::from_value(value).unwrap_or_default(),
        Err(_) => GlobalAppState::default(),
    };

    if preset_name.trim().is_empty() {
        return Err("Preset name must not be empty".to_string());
    }
    load_combined_preset(&store, preset_name.trim())?;
    state.active_preset = preset_name.trim().to_string();

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
    ensure_default_presets(&store)?;

    let preset = load_combined_preset(&store, &name)?;
    let mut sampler = SamplerPreset::new(&name);
    sampler.temperature = preset.temperature;
    sampler.frequency_penalty = preset.frequency_penalty;
    sampler.presence_penalty = preset.presence_penalty;
    sampler.top_p = preset.top_p;
    sampler.top_k = preset.top_k;
    sampler.top_a = preset.top_a;
    sampler.min_p = preset.min_p;
    sampler.repetition_penalty = preset.repetition_penalty;
    sampler.rep_pen_range = preset.rep_pen_range;
    sampler.rep_pen_decay = preset.rep_pen_decay;
    sampler.rep_pen_slope = preset.rep_pen_slope;
    sampler.typical_p = preset.typical_p;
    sampler.tfs = preset.tfs;
    sampler.epsilon_cutoff = preset.epsilon_cutoff;
    sampler.eta_cutoff = preset.eta_cutoff;
    sampler.guidance_scale = preset.guidance_scale;
    sampler.negative_prompt = preset.negative_prompt;
    sampler.dry_allowed_length = preset.dry_allowed_length;
    sampler.dry_multiplier = preset.dry_multiplier;
    sampler.dry_base = preset.dry_base;
    sampler.dry_sequence_breakers = preset.dry_sequence_breakers;
    sampler.mirostat_mode = preset.mirostat_mode;
    sampler.mirostat_tau = preset.mirostat_tau;
    sampler.mirostat_eta = preset.mirostat_eta;
    sampler.no_repeat_ngram_size = preset.no_repeat_ngram_size;
    sampler.encoder_rep_pen = preset.encoder_rep_pen;
    sampler.sampler_priority = preset.sampler_priority;
    sampler.temperature_last = preset.temperature_last;
    sampler.source_api_id = preset.source_api_id.clone();
    Ok(sampler)
}

/// 加载 Instruct 模板
#[tauri::command]
pub async fn load_instruct_template(
    app: AppHandle,
    name: String,
) -> Result<InstructTemplate, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);
    ensure_default_presets(&store)?;

    let preset = load_combined_preset(&store, &name)?;
    Ok(preset
        .instruct
        .unwrap_or_else(|| InstructTemplate::new(&name)))
}

/// 加载 Context 模板
#[tauri::command]
pub async fn load_context_template(
    app: AppHandle,
    name: String,
) -> Result<ContextTemplate, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);
    ensure_default_presets(&store)?;

    let preset = load_combined_preset(&store, &name)?;
    Ok(preset
        .context
        .unwrap_or_else(|| ContextTemplate::new(&name)))
}

/// 加载 System Prompt
#[tauri::command]
pub async fn load_system_prompt(app: AppHandle, name: String) -> Result<SystemPrompt, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);
    ensure_default_presets(&store)?;

    let preset = load_combined_preset(&store, &name)?;
    Ok(preset.sysprompt.unwrap_or_else(|| SystemPrompt::new(&name)))
}

/// 加载 Reasoning 模板
#[tauri::command]
pub async fn load_reasoning_template(
    app: AppHandle,
    name: String,
) -> Result<ReasoningTemplate, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);
    ensure_default_presets(&store)?;

    let preset = load_combined_preset(&store, &name)?;
    Ok(preset
        .reasoning
        .unwrap_or_else(|| ReasoningTemplate::new(&name)))
}

/// 加载 Prompt 预设
#[tauri::command]
pub async fn load_prompt_preset(app: AppHandle, name: String) -> Result<PromptPreset, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);
    ensure_default_presets(&store)?;

    let preset = load_combined_preset(&store, &name)?;
    Ok(PromptPreset {
        name: preset.name.clone(),
        prompts: preset.prompts,
        prompt_order: preset.prompt_order,
        wi_format: preset.wi_format,
        scenario_format: preset.scenario_format,
        personality_format: preset.personality_format,
        new_chat_prompt: preset.new_chat_prompt,
        new_group_chat_prompt: preset.new_group_chat_prompt,
        continue_nudge_prompt: preset.continue_nudge_prompt,
        group_nudge_prompt: preset.group_nudge_prompt,
        impersonation_prompt: preset.impersonation_prompt,
        extensions: std::collections::HashMap::new(),
    })
}

// ============================================================================
// 预设管理命令
// ============================================================================

/// 预设列表项
#[derive(Debug, Serialize, Deserialize)]
pub struct PresetListItem {
    pub name: String,
    pub source_api_id: Option<String>,
}

fn load_combined_preset(store: &JsonStore, name: &str) -> Result<PresetFile, String> {
    let value = store
        .read(&format!("presets/{}.json", name))
        .map_err(|e| format!("Failed to load preset '{}': {}", name, e))?;

    let mut preset: PresetFile = serde_json::from_value(value)
        .map_err(|e| format!("Failed to parse preset '{}': {}", name, e))?;

    // 合并内置提示词条目
    merge_builtin_prompt_items(&mut preset);

    Ok(preset)
}

/// 合并内置提示词条目到预设
///
/// 合并逻辑：
/// 1. 如果 prompts 为空，使用默认 prompts
/// 2. 如果 prompt_order 为空，使用默认 prompt_order
/// 3. 如果 prompt_order 存在但缺少内置条目，补充缺失的内置条目
/// 4. 保留用户自定义的 enabled 和 position 字段
fn merge_builtin_prompt_items(preset: &mut PresetFile) {
    let defaults = create_default_preset_file(&preset.name);

    // 合并 prompts
    if preset.prompts.is_empty() {
        preset.prompts = defaults.prompts;
    } else {
        // 补充缺失的内置 prompts 条目
        let existing_ids: std::collections::HashSet<String> =
            preset.prompts.iter().map(|p| p.identifier.clone()).collect();
        for default_prompt in defaults.prompts {
            if !existing_ids.contains(&default_prompt.identifier) {
                preset.prompts.push(default_prompt);
            }
        }
    }

    // 合并 prompt_order
    if preset.prompt_order.is_empty() {
        preset.prompt_order = defaults.prompt_order;
    } else {
        // 对每个 prompt_order 条目，补充缺失的内置条目
        for order_entry in &mut preset.prompt_order {
            let existing_ids: std::collections::HashSet<String> =
                order_entry.order.iter().map(|o| o.identifier.clone()).collect();

            // 从默认 prompt_order 中获取对应 character_id 的条目
            let default_order = defaults
                .prompt_order
                .iter()
                .find(|d| d.character_id == order_entry.character_id)
                .or_else(|| defaults.prompt_order.first());

            if let Some(default_order_entry) = default_order {
                for default_item in &default_order_entry.order {
                    if !existing_ids.contains(&default_item.identifier) {
                        order_entry.order.push(default_item.clone());
                    }
                }
            }
        }
    }

    if preset.wi_format.trim().is_empty() {
        preset.wi_format = defaults.wi_format;
    }
    if preset.scenario_format.trim().is_empty() {
        preset.scenario_format = defaults.scenario_format;
    }
    if preset.personality_format.trim().is_empty() {
        preset.personality_format = defaults.personality_format;
    }
    if preset.impersonation_prompt.trim().is_empty() {
        preset.impersonation_prompt = defaults.impersonation_prompt;
    }
    if preset.new_chat_prompt.trim().is_empty() {
        preset.new_chat_prompt = defaults.new_chat_prompt;
    }
    if preset.new_group_chat_prompt.trim().is_empty() {
        preset.new_group_chat_prompt = defaults.new_group_chat_prompt;
    }
    if preset.new_example_chat_prompt.trim().is_empty() {
        preset.new_example_chat_prompt = defaults.new_example_chat_prompt;
    }
    if preset.continue_nudge_prompt.trim().is_empty() {
        preset.continue_nudge_prompt = defaults.continue_nudge_prompt;
    }
    if preset.group_nudge_prompt.trim().is_empty() {
        preset.group_nudge_prompt = defaults.group_nudge_prompt;
    }
}

/// 列出所有预设
#[tauri::command]
pub async fn list_presets(app: AppHandle) -> Result<Vec<PresetListItem>, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);
    ensure_default_presets(&store)?;

    let files = store.list("presets")?;
    let mut presets = Vec::new();

    for file in files {
        if file.ends_with(".json") {
            let name = file.strip_suffix(".json").unwrap_or(&file).to_string();
            if let Ok(value) = store.read(&format!("presets/{}", file)) {
                let source_api_id = value
                    .get("source_api_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                presets.push(PresetListItem {
                    name,
                    source_api_id,
                });
            }
        }
    }

    presets.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(presets)
}

/// 加载预设
#[tauri::command]
pub async fn load_preset(app: AppHandle, name: String) -> Result<PresetFile, String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);
    ensure_default_presets(&store)?;

    load_combined_preset(&store, &name)
}

/// 保存预设
#[tauri::command]
pub async fn save_preset(app: AppHandle, preset: PresetFile) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    if preset.name.is_empty() {
        return Err("Preset must have a name".to_string());
    }

    // 过滤内置条目，只保存用户自定义条目和启用/排序状态
    let preset_to_save = filter_builtin_items_for_save(preset);

    let value = serde_json::to_value(&preset_to_save)
        .map_err(|e| format!("Failed to serialize preset: {}", e))?;

    store.write(&format!("presets/{}.json", preset_to_save.name), &value)
}

/// 过滤内置条目，准备保存
///
/// 保存时只保留：
/// 1. 用户自定义的 prompts 条目
/// 2. 内置条目的启用状态和排序位置
fn filter_builtin_items_for_save(mut preset: PresetFile) -> PresetFile {
    preset
}

/// 删除预设
#[tauri::command]
pub async fn delete_preset(app: AppHandle, name: String) -> Result<(), String> {
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);

    if name == default_preset_name() {
        return Err("Default preset cannot be deleted".to_string());
    }

    store.delete(&format!("presets/{}.json", name))
}

// ============================================================================
// 运行时组装命令
// ============================================================================

/// 组装请求的输入参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembleRequestInput {
    pub api_config_id: String,
    pub character_id: Option<String>,
    pub session_id: String,
    /// 预设名称（一个预设文件包含所有类型）
    #[serde(default)]
    pub preset_name: Option<String>,
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
    ensure_default_presets(&store)?;
    let global_state = load_global_state(&store)?;

    // 1. 加载 API 配置
    let api_config: ApiConfig = {
        let value = store.read(&format!("api_configs/{}.json", input.api_config_id))?;
        serde_json::from_value(value).map_err(|e| format!("Failed to parse API config: {}", e))?
    };

    // 2. 加载角色卡
    let character: Option<TavernCardV3> = if let Some(char_id) = &input.character_id {
        let value = store.read(&format!("characters/{}.json", char_id))?;
        Some(
            serde_json::from_value(value)
                .map_err(|e| format!("Failed to parse character: {}", e))?,
        )
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
        chat_metadata: STChatMetadata {
            world_info: chat_session.chat_metadata.world_info.clone(),
            enabled_world_info: chat_session.chat_metadata.enabled_world_info.clone(),
            disabled_world_info: chat_session.chat_metadata.disabled_world_info.clone(),
            user_persona: chat_session
                .chat_metadata
                .user_persona
                .clone()
                .map(|persona| crate::st::runtime_assembly::STUserPersona {
                    name: persona.name,
                    description: persona.description,
                }),
            extra: chat_session.chat_metadata.extra.clone(),
        },
        messages: chat_session
            .messages
            .into_iter()
            .map(|m| STChatMessage {
                id: m.id,
                role: m.role,
                content: m.content,
                created_at: m.created_at,
                name: None,
                attachments: m.attachments,
            })
            .collect(),
    };

    // 4. 加载预设（一个文件包含所有类型）
    let active_preset_name =
        resolve_active_preset_name(input.preset_name.as_deref(), &global_state);
    let preset: Option<PresetFile> = if let Some(name) = active_preset_name.as_deref() {
        if !name.is_empty() {
            Some(load_combined_preset(&store, name)?)
        } else {
            None
        }
    } else {
        None
    };

    // 从预设中提取各类型配置
    let sampler_preset = preset.as_ref().map(|p| {
        let mut sampler = SamplerPreset::new(&p.name);
        sampler.temperature = p.temperature;
        sampler.frequency_penalty = p.frequency_penalty;
        sampler.presence_penalty = p.presence_penalty;
        sampler.top_p = p.top_p;
        sampler.top_k = p.top_k;
        sampler.top_a = p.top_a;
        sampler.min_p = p.min_p;
        sampler.repetition_penalty = p.repetition_penalty;
        sampler.rep_pen_range = p.rep_pen_range;
        sampler.rep_pen_decay = p.rep_pen_decay;
        sampler.rep_pen_slope = p.rep_pen_slope;
        sampler.typical_p = p.typical_p;
        sampler.tfs = p.tfs;
        sampler.epsilon_cutoff = p.epsilon_cutoff;
        sampler.eta_cutoff = p.eta_cutoff;
        sampler.guidance_scale = p.guidance_scale;
        sampler.negative_prompt = p.negative_prompt.clone();
        sampler.dry_allowed_length = p.dry_allowed_length;
        sampler.dry_multiplier = p.dry_multiplier;
        sampler.dry_base = p.dry_base;
        sampler.dry_sequence_breakers = p.dry_sequence_breakers.clone();
        sampler.mirostat_mode = p.mirostat_mode;
        sampler.mirostat_tau = p.mirostat_tau;
        sampler.mirostat_eta = p.mirostat_eta;
        sampler.no_repeat_ngram_size = p.no_repeat_ngram_size;
        sampler.encoder_rep_pen = p.encoder_rep_pen;
        sampler.sampler_priority = p.sampler_priority.clone();
        sampler.temperature_last = p.temperature_last;
        sampler.source_api_id = p.source_api_id.clone();
        sampler
    });
    let instruct_template = preset.as_ref().and_then(|p| p.instruct.clone());
    let context_template = preset.as_ref().and_then(|p| p.context.clone());
    let system_prompt = preset.as_ref().and_then(|p| p.sysprompt.clone());
    let reasoning_template = preset.as_ref().and_then(|p| p.reasoning.clone());
    let prompt_preset = preset.as_ref().map(|p| PromptPreset {
        name: p.name.clone(),
        prompts: p.prompts.clone(),
        prompt_order: p.prompt_order.clone(),
        wi_format: p.wi_format.clone(),
        scenario_format: p.scenario_format.clone(),
        personality_format: p.personality_format.clone(),
        new_chat_prompt: p.new_chat_prompt.clone(),
        new_group_chat_prompt: p.new_group_chat_prompt.clone(),
        continue_nudge_prompt: p.continue_nudge_prompt.clone(),
        group_nudge_prompt: p.group_nudge_prompt.clone(),
        impersonation_prompt: p.impersonation_prompt.clone(),
        extensions: std::collections::HashMap::new(),
    });

    // 5. 构建全局扫描数据
    let global_scan_data = GlobalScanData {
        persona_description: persona_scan_text(&session.chat_metadata),
        character_description: character
            .as_ref()
            .map(|c| c.data.description.clone())
            .unwrap_or_default(),
        character_personality: character
            .as_ref()
            .map(|c| c.data.personality.clone())
            .unwrap_or_default(),
        character_depth_prompt: String::new(),
        scenario: character
            .as_ref()
            .map(|c| c.data.scenario.clone())
            .unwrap_or_default(),
        creator_notes: character
            .as_ref()
            .map(|c| c.data.creator_notes.clone())
            .unwrap_or_default(),
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
            &session.chat_metadata,
            &input.world_info_settings,
            input.chat_lore_id.as_deref(),
            &global_lore_ids,
        )?;
        if sources.is_empty() {
            None
        } else {
            let mut injector = WorldInfoInjector::new();
            let macro_context =
                MacroContext::from_chat_metadata(&session.chat_metadata, character.as_ref(), "");
            Some(
                injector
                    .check_world_info(
                        &session.messages,
                        input.max_context,
                        &input.world_info_settings,
                        sources,
                        &global_scan_data,
                        &macro_context,
                    )
                    .await,
            )
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
    let mut request = RequestAssembler::assemble(&context);
    if let Some(preset) = preset.as_ref() {
        if preset.openai_max_tokens > 0 {
            request.max_tokens = Some(preset.openai_max_tokens);
        }
    }
    apply_prompt_only_regex(
        &mut request,
        &global_state.regex_settings,
        context
            .character
            .as_ref()
            .map(|character| character.data.name.as_str()),
        context
            .prompt_preset
            .as_ref()
            .map(|preset| preset.name.as_str()),
    );

    Ok(AssembleRequestOutput {
        request,
        provider_type: api_config.provider.clone(),
        model: api_config.model.clone(),
        world_info_result,
    })
}

/// 经过 ST runtime assembly gate 后发送聊天请求。
#[tauri::command]
pub async fn send_assembled_st_chat_message(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: AssembleRequestInput,
) -> Result<ChatResponseData, String> {
    let assembled = assemble_st_request(app.clone(), input.clone()).await?;
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir.clone());

    let api_config: ApiConfig = {
        let value = store.read(&format!("api_configs/{}.json", input.api_config_id))?;
        serde_json::from_value(value).map_err(|e| format!("Failed to parse API config: {}", e))?
    };
    let compiled_contract = get_compiled_contract_view(state.inner(), &api_config).await?;
    let provider = create_provider(&api_config, Some(data_dir.clone()))?;

    let request_id = Uuid::new_v4().to_string();
    let request = assembled_to_chat_request(
        &data_dir,
        &compiled_contract,
        request_id.clone(),
        input.api_config_id.clone(),
        assembled.request,
    )
    .await?;

    // Build request URL for logging
    let request_url = build_request_url(&api_config);

    let log_context = LogContext {
        mode: LogMode::St,
        world_id: None,
        session_id: Some(input.session_id.clone()),
        scene_turn_id: None,
        character_id: input.character_id.clone(),
        trace_id: None,
        llm_node: LlmNode::STChat,
        api_config_id: input.api_config_id,
        request_id: request_id.clone(),
    };

    let store_guard = state.sqlite_store.read().await;
    tracing::info!(
        "[ST Chat Log] sqlite_store present: {}, request_id: {}",
        store_guard.is_some(),
        request_id
    );
    if let Some(sqlite_store) = store_guard.as_ref() {
        // Build actual request body that will be sent to the API
        let request_json = build_provider_request_preview(
            &data_dir,
            &api_config,
            &compiled_contract.key.protocol_kind,
            request.clone(),
        )
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("[ST Chat Log] Failed to build request preview: {}, falling back to ChatRequest", e);
            serde_json::to_value(&request).unwrap_or(serde_json::Value::Null)
        });
        tracing::info!(
            "[ST Chat Log] Calling log_start for request_id: {}",
            request_id
        );
        sqlite_store
            .llm_logger()
            .log_start(
                &log_context,
                &request_json,
                request_url.as_deref(),
                &api_config.provider,
                &api_config.model,
                "chat",
                None,
            )
            .await;
    } else {
        tracing::warn!("[ST Chat Log] sqlite_store is None, log_start skipped");
    }

    match provider.chat(request).await {
        Ok(resp) => {
            tracing::info!(
                "[ST Chat Log] LLM call succeeded, request_id: {}, raw_response present: {}",
                request_id,
                resp.raw_response.is_some()
            );
            if let Some(sqlite_store) = store_guard.as_ref() {
                tracing::info!(
                    "[ST Chat Log] Calling log_success for request_id: {}",
                    request_id
                );
                let response_for_log = resp.raw_response.clone().unwrap_or_else(|| {
                    tracing::warn!(
                        "[ST Chat Log] raw_response is None for request_id: {}, using fallback",
                        request_id
                    );
                    serde_json::json!({"content": &resp.content})
                });
                sqlite_store
                    .llm_logger()
                    .log_success(
                        &request_id,
                        &response_for_log,
                        resp.reasoning.as_deref(),
                        resp.token_usage.as_ref().map(|u| {
                            serde_json::json!({
                                "prompt_tokens": u.prompt_tokens,
                                "completion_tokens": u.completion_tokens,
                                "total_tokens": u.total_tokens
                            })
                        }),
                    )
                    .await;
            } else {
                tracing::warn!("[ST Chat Log] sqlite_store is None, log_success skipped");
            }

            Ok(ChatResponseData {
                request_id,
                content: resp.content,
                reasoning: resp.reasoning,
                token_usage: resp.token_usage.map(|u| u.into()),
                finish_reason: resp.finish_reason,
            })
        }
        Err(e) => {
            if let Some(sqlite_store) = store_guard.as_ref() {
                sqlite_store.llm_logger().log_failure(&request_id, &e).await;
            }
            Err(e)
        }
    }
}

async fn assembled_to_chat_request(
    data_dir: &std::path::Path,
    compiled_contract: &CompiledProviderContractView,
    request_id: String,
    api_config_id: String,
    assembled: AssembledRequest,
) -> Result<ChatRequest, String> {
    let (_text_supported, image_supported, pdf_supported) =
        connection_supports_attachments(compiled_contract);
    let sampling = sampling_for_contract(&compiled_contract.key.protocol_kind, &assembled.sampling);
    let mut messages = Vec::new();
    if !assembled.system_prompt.is_empty() {
        messages.push(ProviderChatMessage::system(assembled.system_prompt));
    }

    for message in assembled.messages {
        let role = match message.role.as_str() {
            "system" => ChatRole::System,
            "assistant" => ChatRole::Assistant,
            "user" => ChatRole::User,
            _ => ChatRole::User,
        };
        let mut content = vec![ContentPart::Text {
            text: message.content,
        }];
        for attachment in message.attachments {
            match attachment.kind.as_str() {
                "image" if !image_supported => {
                    return Err(format!(
                        "Current API config does not support image input; attachment {} cannot be sent",
                        attachment.filename
                    ));
                }
                "pdf" if !pdf_supported => {
                    return Err(format!(
                        "Current API config does not support PDF input; attachment {} cannot be sent",
                        attachment.filename
                    ));
                }
                _ => {}
            }
            let part = assembled_attachment_to_content_part(data_dir, &attachment).await?;
            content.push(part);
        }
        messages.push(ProviderChatMessage {
            role,
            content,
            name: None,
        });
    }

    Ok(ChatRequest {
        request_id,
        api_config_id,
        messages,
        sampling,
        stop_sequences: assembled.stop_sequences,
        max_tokens: assembled.max_tokens.and_then(|v| u32::try_from(v).ok()),
        stream: false,
        reasoning: assembled.reasoning.and_then(|r| {
            if r.enabled {
                Some(ReasoningParams {
                    effort: r.effort,
                    budget_tokens: r.budget_tokens.and_then(|v| u32::try_from(v).ok()),
                    exclude_reasoning_text_from_response: false,
                })
            } else {
                None
            }
        }),
        response_format: None,
        provider_overrides: serde_json::Value::Null,
    })
}

fn sampling_for_contract(protocol_kind: &str, source: &AssembledSamplingParams) -> SamplingParams {
    let mut sampling = SamplingParams {
        temperature: source.temperature,
        top_p: source.top_p,
        top_k: source.top_k.and_then(|v| u32::try_from(v).ok()),
        repetition_penalty: source.repetition_penalty,
        frequency_penalty: source.frequency_penalty,
        presence_penalty: source.presence_penalty,
    };

    match protocol_kind {
        "openai_responses" => {
            sampling.temperature = sampling.temperature.map(|v| clamp_f64(v, 0.0, 2.0));
            sampling.top_p = sampling.top_p.map(|v| clamp_f64(v, 0.0, 1.0));
            sampling.top_k = None;
            sampling.repetition_penalty = None;
            sampling.frequency_penalty = None;
            sampling.presence_penalty = None;
        }
        "openai_chat_completions" | "deepseek_chat" => {
            sampling.temperature = sampling.temperature.map(|v| clamp_f64(v, 0.0, 2.0));
            sampling.top_p = sampling.top_p.map(|v| clamp_f64(v, 0.0, 1.0));
            sampling.top_k = None;
            sampling.repetition_penalty = None;
            if should_map_repetition_to_frequency(source, sampling.frequency_penalty) {
                sampling.frequency_penalty = source
                    .repetition_penalty
                    .map(|v| clamp_f64(v - 1.0, -2.0, 2.0));
            }
            sampling.frequency_penalty =
                sampling.frequency_penalty.map(|v| clamp_f64(v, -2.0, 2.0));
            sampling.presence_penalty = sampling.presence_penalty.map(|v| clamp_f64(v, -2.0, 2.0));
        }
        "anthropic_messages" | "claude_code_interface" => {
            sampling.temperature = sampling.temperature.map(|v| clamp_f64(v, 0.0, 1.0));
            sampling.top_p = sampling.top_p.map(|v| clamp_f64(v, 0.0, 1.0));
            sampling.repetition_penalty = None;
            sampling.frequency_penalty = None;
            sampling.presence_penalty = None;
        }
        "gemini_generate_content" => {
            sampling.temperature = sampling.temperature.map(|v| clamp_f64(v, 0.0, 2.0));
            sampling.top_p = sampling.top_p.map(|v| clamp_f64(v, 0.0, 1.0));
            sampling.top_k = sampling.top_k.filter(|v| *v > 0);
            sampling.repetition_penalty = None;
            sampling.frequency_penalty = None;
            sampling.presence_penalty = None;
        }
        _ => {}
    }

    sampling
}

fn should_map_repetition_to_frequency(
    source: &AssembledSamplingParams,
    current_frequency: Option<f64>,
) -> bool {
    let Some(repetition) = source.repetition_penalty else {
        return false;
    };

    (repetition - 1.0).abs() > f64::EPSILON
        && current_frequency
            .map(|value| value.abs() <= f64::EPSILON)
            .unwrap_or(true)
}

fn clamp_f64(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

async fn assembled_attachment_to_content_part(
    data_dir: &std::path::Path,
    attachment: &AssembledAttachmentRef,
) -> Result<ContentPart, String> {
    let record = load_attachment_record(data_dir, &attachment.attachment_id).await?;
    let blob_path = safe_join(
        data_dir,
        &format!(
            "chat_attachments/{}/{}",
            attachment.attachment_id, record.blob_filename
        ),
    )?;
    let blob = tokio::fs::read(&blob_path).await.map_err(|e| {
        format!(
            "Failed to read attachment blob {}: {}",
            attachment.attachment_id, e
        )
    })?;

    match record.kind {
        ChatAttachmentKind::Image => Ok(ContentPart::ImageRef {
            image_url: ImageUrl {
                url: format!("data:{};base64,{}", record.mime_type, STANDARD.encode(blob)),
            },
        }),
        ChatAttachmentKind::Pdf => Ok(ContentPart::FileRef {
            file: FileRef {
                attachment_id: Some(attachment.attachment_id.clone()),
                file_id: None,
                file_uri: None,
                file_data: Some(STANDARD.encode(blob)),
                filename: Some(record.filename),
                mime_type: Some(record.mime_type),
            },
        }),
    }
}

async fn load_attachment_record(
    data_dir: &std::path::Path,
    attachment_id: &str,
) -> Result<ChatAttachmentRecord, String> {
    let meta_path = safe_join(
        data_dir,
        &format!("chat_attachments/{}/meta.json", attachment_id),
    )?;
    let text = tokio::fs::read_to_string(&meta_path).await.map_err(|e| {
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

fn load_global_state(store: &JsonStore) -> Result<GlobalAppState, String> {
    match store.read("settings/global_state.json") {
        Ok(value) => {
            let mut state: GlobalAppState = serde_json::from_value(value.clone())
                .map_err(|e| format!("Failed to parse global state: {}", e))?;
            if !value.get("active_preset").is_some() || state.active_preset.trim().is_empty() {
                state.active_preset = legacy_active_preset_name(&value)
                    .unwrap_or_else(|| default_preset_name().to_string());
            }
            Ok(state)
        }
        Err(_) => Ok(GlobalAppState::default()),
    }
}

fn default_preset_name() -> &'static str {
    "Default"
}

fn resolve_active_preset_name(
    requested: Option<&str>,
    global_state: &GlobalAppState,
) -> Option<String> {
    requested
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            let active = global_state.active_preset.trim();
            if active.is_empty() {
                Some(default_preset_name().to_string())
            } else {
                Some(active.to_string())
            }
        })
}

fn legacy_active_preset_name(value: &serde_json::Value) -> Option<String> {
    [
        "active_prompt_preset",
        "active_sampler_preset",
        "active_instruct_preset",
        "active_context_preset",
        "active_sysprompt_preset",
        "active_reasoning_preset",
    ]
    .iter()
    .filter_map(|key| value.get(*key)?.as_str())
    .map(str::trim)
    .find(|name| !name.is_empty())
    .map(ToOwned::to_owned)
}

fn ensure_default_presets(store: &JsonStore) -> Result<(), String> {
    if store.read("presets/Default.json").is_ok() {
        return Ok(());
    }

    let preset = create_default_preset_file(default_preset_name());
    let value = serde_json::to_value(&preset)
        .map_err(|e| format!("Failed to serialize default preset: {}", e))?;
    store.write("presets/Default.json", &value)
}

fn create_default_preset_file(name: &str) -> PresetFile {
    use crate::st::preset::{PromptItem, PromptOrder, PromptOrderItem};

    PresetFile {
        name: name.to_string(),
        temperature: 1.0,
        frequency_penalty: 0.0,
        presence_penalty: 0.0,
        top_p: 1.0,
        top_k: 0,
        top_a: 0.0,
        min_p: 0.0,
        repetition_penalty: 1.0,
        rep_pen_range: 0,
        rep_pen_decay: 0.0,
        rep_pen_slope: 0.0,
        typical_p: 0.0,
        tfs: 0.0,
        epsilon_cutoff: 0.0,
        eta_cutoff: 0.0,
        guidance_scale: 1.0,
        negative_prompt: String::new(),
        dry_allowed_length: 0,
        dry_multiplier: 0.0,
        dry_base: 0.0,
        dry_sequence_breakers: String::new(),
        mirostat_mode: 0,
        mirostat_tau: 5.0,
        mirostat_eta: 0.1,
        no_repeat_ngram_size: 0,
        encoder_rep_pen: 0.0,
        sampler_priority: Vec::new(),
        temperature_last: false,
        prompts: vec![
            PromptItem {
                identifier: "main".to_string(),
                name: "Main Prompt".to_string(),
                role: "system".to_string(),
                content: "Write {{char}}'s next reply in a fictional chat between {{char}} and {{user}}.".to_string(),
                system_prompt: true,
                marker: false,
                enabled: None,
                injection_position: None,
                injection_depth: None,
                injection_order: None,
                forbid_overrides: None,
                injection_trigger: Vec::new(),
                builtin: false,
                editable: true,
                description: String::new(),
            },
            PromptItem {
                identifier: "nsfw".to_string(),
                name: "Auxiliary Prompt".to_string(),
                role: "system".to_string(),
                content: String::new(),
                system_prompt: true,
                marker: false,
                enabled: None,
                injection_position: None,
                injection_depth: None,
                injection_order: None,
                forbid_overrides: None,
                injection_trigger: Vec::new(),
                builtin: false,
                editable: true,
                description: String::new(),
            },
            PromptItem {
                identifier: "dialogueExamples".to_string(),
                name: "Chat Examples".to_string(),
                role: "system".to_string(),
                content: String::new(),
                system_prompt: true,
                marker: true,
                enabled: None,
                injection_position: None,
                injection_depth: None,
                injection_order: None,
                forbid_overrides: None,
                injection_trigger: Vec::new(),
                builtin: false,
                editable: true,
                description: String::new(),
            },
            PromptItem {
                identifier: "jailbreak".to_string(),
                name: "Post-History Instructions".to_string(),
                role: "system".to_string(),
                content: String::new(),
                system_prompt: true,
                marker: false,
                enabled: None,
                injection_position: None,
                injection_depth: None,
                injection_order: None,
                forbid_overrides: None,
                injection_trigger: Vec::new(),
                builtin: false,
                editable: true,
                description: String::new(),
            },
            PromptItem {
                identifier: "chatHistory".to_string(),
                name: "Chat History".to_string(),
                role: "system".to_string(),
                content: String::new(),
                system_prompt: true,
                marker: true,
                enabled: None,
                injection_position: None,
                injection_depth: None,
                injection_order: None,
                forbid_overrides: None,
                injection_trigger: Vec::new(),
                builtin: false,
                editable: true,
                description: String::new(),
            },
            PromptItem {
                identifier: "worldInfoAfter".to_string(),
                name: "World Info (after)".to_string(),
                role: "system".to_string(),
                content: String::new(),
                system_prompt: true,
                marker: true,
                enabled: None,
                injection_position: None,
                injection_depth: None,
                injection_order: None,
                forbid_overrides: None,
                injection_trigger: Vec::new(),
                builtin: false,
                editable: true,
                description: String::new(),
            },
            PromptItem {
                identifier: "worldInfoBefore".to_string(),
                name: "World Info (before)".to_string(),
                role: "system".to_string(),
                content: String::new(),
                system_prompt: true,
                marker: true,
                enabled: None,
                injection_position: None,
                injection_depth: None,
                injection_order: None,
                forbid_overrides: None,
                injection_trigger: Vec::new(),
                builtin: false,
                editable: true,
                description: String::new(),
            },
            PromptItem {
                identifier: "enhanceDefinitions".to_string(),
                name: "Enhance Definitions".to_string(),
                role: "system".to_string(),
                content: "If you have more knowledge of {{char}}, add to the character's lore and personality to enhance them but keep the Character Sheet's definitions absolute.".to_string(),
                system_prompt: true,
                marker: false,
                enabled: None,
                injection_position: None,
                injection_depth: None,
                injection_order: None,
                forbid_overrides: None,
                injection_trigger: Vec::new(),
                builtin: false,
                editable: true,
                description: String::new(),
            },
            PromptItem {
                identifier: "charDescription".to_string(),
                name: "Char Description".to_string(),
                role: "system".to_string(),
                content: String::new(),
                system_prompt: true,
                marker: true,
                enabled: None,
                injection_position: None,
                injection_depth: None,
                injection_order: None,
                forbid_overrides: None,
                injection_trigger: Vec::new(),
                builtin: false,
                editable: true,
                description: String::new(),
            },
            PromptItem {
                identifier: "charPersonality".to_string(),
                name: "Char Personality".to_string(),
                role: "system".to_string(),
                content: String::new(),
                system_prompt: true,
                marker: true,
                enabled: None,
                injection_position: None,
                injection_depth: None,
                injection_order: None,
                forbid_overrides: None,
                injection_trigger: Vec::new(),
                builtin: false,
                editable: true,
                description: String::new(),
            },
            PromptItem {
                identifier: "scenario".to_string(),
                name: "Scenario".to_string(),
                role: "system".to_string(),
                content: String::new(),
                system_prompt: true,
                marker: true,
                enabled: None,
                injection_position: None,
                injection_depth: None,
                injection_order: None,
                forbid_overrides: None,
                injection_trigger: Vec::new(),
                builtin: false,
                editable: true,
                description: String::new(),
            },
            PromptItem {
                identifier: "personaDescription".to_string(),
                name: "Persona Description".to_string(),
                role: "system".to_string(),
                content: String::new(),
                system_prompt: true,
                marker: true,
                enabled: None,
                injection_position: None,
                injection_depth: None,
                injection_order: None,
                forbid_overrides: None,
                injection_trigger: Vec::new(),
                builtin: false,
                editable: true,
                description: String::new(),
            },
        ],
        prompt_order: vec![
            PromptOrder {
                character_id: 100000,
                order: vec![
                    PromptOrderItem { identifier: "main".to_string(), enabled: true, position: None },
                    PromptOrderItem { identifier: "worldInfoBefore".to_string(), enabled: true, position: None },
                    PromptOrderItem { identifier: "personaDescription".to_string(), enabled: true, position: None },
                    PromptOrderItem { identifier: "charDescription".to_string(), enabled: true, position: None },
                    PromptOrderItem { identifier: "charPersonality".to_string(), enabled: true, position: None },
                    PromptOrderItem { identifier: "scenario".to_string(), enabled: true, position: None },
                    PromptOrderItem { identifier: "enhanceDefinitions".to_string(), enabled: false, position: None },
                    PromptOrderItem { identifier: "nsfw".to_string(), enabled: true, position: None },
                    PromptOrderItem { identifier: "worldInfoAfter".to_string(), enabled: true, position: None },
                    PromptOrderItem { identifier: "dialogueExamples".to_string(), enabled: true, position: None },
                    PromptOrderItem { identifier: "chatHistory".to_string(), enabled: true, position: None },
                    PromptOrderItem { identifier: "jailbreak".to_string(), enabled: true, position: None },
                ],
            },
            PromptOrder {
                character_id: 100001,
                order: vec![
                    PromptOrderItem { identifier: "main".to_string(), enabled: true, position: None },
                    PromptOrderItem { identifier: "worldInfoBefore".to_string(), enabled: true, position: None },
                    PromptOrderItem { identifier: "charDescription".to_string(), enabled: true, position: None },
                    PromptOrderItem { identifier: "charPersonality".to_string(), enabled: true, position: None },
                    PromptOrderItem { identifier: "scenario".to_string(), enabled: true, position: None },
                    PromptOrderItem { identifier: "enhanceDefinitions".to_string(), enabled: false, position: None },
                    PromptOrderItem { identifier: "nsfw".to_string(), enabled: true, position: None },
                    PromptOrderItem { identifier: "worldInfoAfter".to_string(), enabled: true, position: None },
                    PromptOrderItem { identifier: "dialogueExamples".to_string(), enabled: true, position: None },
                    PromptOrderItem { identifier: "chatHistory".to_string(), enabled: true, position: None },
                    PromptOrderItem { identifier: "jailbreak".to_string(), enabled: true, position: None },
                ],
            },
        ],
        wi_format: "{0}".to_string(),
        scenario_format: "{{scenario}}".to_string(),
        personality_format: "{{personality}}".to_string(),
        send_if_empty: String::new(),
        impersonation_prompt: "[Write your next reply from the point of view of {{user}}, using the chat history so far as a guideline for the writing style of {{user}}. Don't write as {{char}} or system. Don't describe actions of {{char}}.]".to_string(),
        new_chat_prompt: "[Start a new Chat]".to_string(),
        new_group_chat_prompt: "[Start a new group chat. Group members: {{group}}]".to_string(),
        new_example_chat_prompt: "[Example Chat]".to_string(),
        continue_nudge_prompt: "[Continue your last message without repeating its original content.]".to_string(),
        group_nudge_prompt: "[Write the next reply only as {{char}}.]".to_string(),
        stream_openai: true,
        use_sysprompt: false,
        assistant_prefill: String::new(),
        reasoning_effort: String::new(),
        max_context_unlocked: false,
        openai_max_context: 4095,
        openai_max_tokens: 300,
        names_behavior: 0,
        instruct: Some(InstructTemplate::new(name)),
        context: Some(ContextTemplate::new(name)),
        sysprompt: Some(SystemPrompt::new(name)),
        reasoning: Some(ReasoningTemplate::new(name)),
        source_api_id: None,
        extensions: std::collections::HashMap::new(),
    }
}

fn apply_prompt_only_regex(
    request: &mut AssembledRequest,
    settings: &crate::st::RegexExtensionSettings,
    character_name: Option<&str>,
    preset_key: Option<&str>,
) {
    if settings.regex.is_empty() && settings.regex_presets.is_empty() {
        return;
    }

    let mut engine = RegexEngine::new();
    let options = RegexRunOptions {
        is_prompt: true,
        preset_key: preset_key.map(ToOwned::to_owned),
        character_name: character_name.map(ToOwned::to_owned),
        ..Default::default()
    };

    request.system_prompt = engine.get_regexed_string(
        &request.system_prompt,
        RegexPlacement::WORLD_INFO,
        settings,
        &options,
    );

    for message in &mut request.messages {
        let placement = match message.role.as_str() {
            "assistant" => RegexPlacement::AI_OUTPUT,
            _ => RegexPlacement::USER_INPUT,
        };
        message.content =
            engine.get_regexed_string(&message.content, placement, settings, &options);
    }

    if let Some(reasoning) = &mut request.reasoning {
        if let Some(effort) = &mut reasoning.effort {
            *effort =
                engine.get_regexed_string(effort, RegexPlacement::REASONING, settings, &options);
        }
    }
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

    let messages: Vec<STChatMessage> = chat_session
        .messages
        .into_iter()
        .map(|m| STChatMessage {
            id: m.id,
            role: m.role,
            content: m.content,
            created_at: m.created_at,
            name: None,
            attachments: m.attachments,
        })
        .collect();

    // 2. 加载角色卡并收集世界书来源
    let character = if let Some(char_id) = &input.character_id {
        let value = store.read(&format!("characters/{}.json", char_id))?;
        Some(
            serde_json::from_value::<TavernCardV3>(value)
                .map_err(|e| format!("Failed to parse character: {}", e))?,
        )
    } else {
        None
    };

    let mut global_lore_ids = input.global_lore_ids.clone();
    for lore_id in &input.world_info_settings.global_select {
        if !global_lore_ids.contains(lore_id) {
            global_lore_ids.push(lore_id.clone());
        }
    }

    let chat_metadata = STChatMetadata {
        world_info: chat_session.chat_metadata.world_info.clone(),
        enabled_world_info: chat_session.chat_metadata.enabled_world_info.clone(),
        disabled_world_info: chat_session.chat_metadata.disabled_world_info.clone(),
        user_persona: chat_session
            .chat_metadata
            .user_persona
            .clone()
            .map(|persona| crate::st::runtime_assembly::STUserPersona {
                name: persona.name,
                description: persona.description,
            }),
        extra: chat_session.chat_metadata.extra.clone(),
    };

    let sources = collect_world_info_sources(
        &store,
        character.as_ref(),
        &chat_metadata,
        &input.world_info_settings,
        input.chat_lore_id.as_deref(),
        &global_lore_ids,
    )?;

    // 3. 构建全局扫描数据
    let global_scan_data = GlobalScanData {
        persona_description: persona_scan_text(&chat_metadata),
        character_description: character
            .as_ref()
            .map(|c| c.data.description.clone())
            .unwrap_or_default(),
        character_personality: character
            .as_ref()
            .map(|c| c.data.personality.clone())
            .unwrap_or_default(),
        character_depth_prompt: String::new(),
        scenario: character
            .as_ref()
            .map(|c| c.data.scenario.clone())
            .unwrap_or_default(),
        creator_notes: character
            .as_ref()
            .map(|c| c.data.creator_notes.clone())
            .unwrap_or_default(),
        trigger: None,
    };
    let macro_context = MacroContext::from_chat_metadata(&chat_metadata, character.as_ref(), "");

    // 4. 执行注入
    let mut injector = WorldInfoInjector::new();
    Ok(injector
        .check_world_info(
            &messages,
            input.max_context,
            &input.world_info_settings,
            sources,
            &global_scan_data,
            &macro_context,
        )
        .await)
}

fn collect_world_info_sources(
    store: &JsonStore,
    character: Option<&TavernCardV3>,
    chat_metadata: &STChatMetadata,
    settings: &STWorldInfoSettings,
    chat_lore_id: Option<&str>,
    global_lore_ids: &[String],
) -> Result<Vec<WorldInfoSource>, String> {
    let mut sources: Vec<WorldInfoSource> = Vec::new();
    let mut seen_lore_ids: HashSet<String> = HashSet::new();
    let disabled_world_info: HashSet<&str> = chat_metadata
        .disabled_world_info
        .iter()
        .map(String::as_str)
        .collect();

    let chat_lore_ids = if let Some(lore_id) = chat_lore_id {
        vec![lore_id.to_string()]
    } else if !chat_metadata.enabled_world_info.is_empty() {
        chat_metadata.enabled_world_info.clone()
    } else {
        chat_metadata.world_info.iter().cloned().collect()
    };

    for lore_id in chat_lore_ids {
        if disabled_world_info.contains(lore_id.as_str()) {
            continue;
        }
        if !seen_lore_ids.insert(lore_id.clone()) {
            continue;
        }
        if let Some(worldbook) = load_worldbook_by_id(store, &lore_id)? {
            sources.push(WorldInfoSource::ChatLore(worldbook));
        }
    }

    for lore_id in global_lore_ids {
        if disabled_world_info.contains(lore_id.as_str()) {
            continue;
        }
        if !seen_lore_ids.insert(lore_id.clone()) {
            continue;
        }
        if let Some(worldbook) = load_worldbook_by_id(store, lore_id)? {
            sources.push(WorldInfoSource::GlobalLore(worldbook));
        }
    }

    if let Some(character) = character {
        for lore_id in character_lore_ids(character, chat_metadata, settings, store)? {
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

fn persona_scan_text(chat_metadata: &STChatMetadata) -> String {
    let Some(persona) = chat_metadata.user_persona.as_ref() else {
        return String::new();
    };

    match (persona.name.trim(), persona.description.trim()) {
        ("", "") => String::new(),
        ("", description) => description.to_string(),
        (name, "") => name.to_string(),
        (name, description) => format!("{}\n{}", name, description),
    }
}

fn load_worldbook_by_id(store: &JsonStore, lore_id: &str) -> Result<Option<WorldInfoFile>, String> {
    let value = match store.read(&format!("lores/{}.json", lore_id)) {
        Ok(value) => value,
        Err(error) if error.contains("Failed to read") => return Ok(None),
        Err(error) => return Err(error),
    };
    let worldbook = serde_json::from_value::<WorldInfoFile>(value)
        .map_err(|e| format!("Failed to parse worldbook {}: {}", lore_id, e))?;
    Ok(Some(worldbook))
}

fn character_lore_ids(
    character: &TavernCardV3,
    chat_metadata: &STChatMetadata,
    settings: &STWorldInfoSettings,
    store: &JsonStore,
) -> Result<Vec<String>, String> {
    let mut ids = Vec::new();

    if let Some(world_name) = character
        .data
        .extensions
        .get("world")
        .and_then(|v| v.as_str())
    {
        if let Some(lore_id) = find_lore_id_by_name(store, world_name)? {
            if !chat_metadata
                .disabled_world_info
                .iter()
                .any(|disabled| disabled == &lore_id)
            {
                ids.push(lore_id);
            }
        }
    }

    for binding in &settings.char_lore {
        if binding.name == character.data.name {
            ids.extend(
                binding
                    .extra_books
                    .iter()
                    .filter(|lore_id| {
                        !chat_metadata
                            .disabled_world_info
                            .iter()
                            .any(|disabled| disabled == *lore_id)
                    })
                    .cloned(),
            );
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

#[cfg(test)]
mod tests {
    use super::{
        apply_prompt_only_regex, assembled_to_chat_request, character_lore_ids,
        ensure_default_presets, load_combined_preset, load_global_state, load_worldbook_by_id,
        sampling_for_contract,
    };
    use crate::config::llm_contracts::{CompiledProviderContractView, ProviderContractCacheKey};
    use crate::st::runtime_assembly::AssembledAttachmentRef;
    use crate::st::{AssembledMessage, AssembledRequest};
    use crate::st::{AssembledSamplingParams, STChatMetadata, STWorldInfoSettings};
    use crate::storage::json_store::JsonStore;
    use crate::storage::st_resources::{CharacterData, TavernCardV3};
    use serde_json::json;

    fn sample_character(world_name: &str) -> TavernCardV3 {
        TavernCardV3 {
            spec: "chara_card_v3".to_string(),
            spec_version: "3.0".to_string(),
            data: CharacterData {
                name: "Hero".to_string(),
                description: String::new(),
                personality: String::new(),
                scenario: String::new(),
                first_mes: String::new(),
                mes_example: String::new(),
                creator_notes: String::new(),
                system_prompt: String::new(),
                post_history_instructions: String::new(),
                alternate_greetings: Vec::new(),
                tags: Vec::new(),
                creator: String::new(),
                character_version: String::new(),
                extensions: {
                    let mut extensions = serde_json::Map::new();
                    extensions.insert("world".to_string(), json!(world_name));
                    extensions
                },
                character_book: None,
                extra: serde_json::Map::new(),
            },
            extra: serde_json::Map::new(),
        }
    }

    fn sample_worldbook(name: &str, lore_id: &str) -> serde_json::Value {
        json!({
            "entries": {},
            "rst_lore_id": lore_id,
            "name": name,
            "description": ""
        })
    }

    #[test]
    fn default_preset_is_created_as_single_combined_file() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let store = JsonStore::new(temp_dir.path().to_path_buf());

        ensure_default_presets(&store).expect("default preset");

        let files = store.list("presets").expect("list presets");
        assert_eq!(files, vec!["Default.json".to_string()]);

        let preset = load_combined_preset(&store, "Default").expect("load default preset");
        assert_eq!(preset.name, "Default");
        assert!(!preset.prompts.is_empty());
        assert!(!preset.prompt_order.is_empty());
        assert_eq!(preset.new_chat_prompt, "[Start a new Chat]");
        assert_eq!(preset.wi_format, "{0}");
        assert!(preset.instruct.is_some());
        assert!(preset.context.is_some());
        assert!(preset.sysprompt.is_some());
        assert!(preset.reasoning.is_some());
        assert!(preset.source_api_id.is_none());
    }

    #[test]
    fn global_state_without_active_preset_falls_back_to_default_or_legacy_name() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let store = JsonStore::new(temp_dir.path().to_path_buf());
        store
            .write(
                "settings/global_state.json",
                &json!({
                    "active_api_config_id": null,
                    "active_prompt_preset": "Legacy Prompt",
                    "auto_select_preset": false,
                    "world_info_settings": crate::st::STWorldInfoSettings::default(),
                    "regex_settings": crate::st::RegexExtensionSettings::default()
                }),
            )
            .expect("write global state");

        let state = load_global_state(&store).expect("load global state");
        assert_eq!(state.active_preset, "Legacy Prompt");
    }

    #[test]
    fn character_bound_worldbook_is_enabled_by_default_until_explicitly_disabled() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let store = JsonStore::new(temp_dir.path().to_path_buf());
        store
            .write(
                "lores/lore-1.json",
                &sample_worldbook("Default World", "lore-1"),
            )
            .expect("write lore");

        let character = sample_character("Default World");
        let settings = STWorldInfoSettings::default();

        let enabled = character_lore_ids(&character, &STChatMetadata::default(), &settings, &store)
            .expect("character lore ids");
        assert_eq!(enabled, vec!["lore-1".to_string()]);

        let disabled = character_lore_ids(
            &character,
            &STChatMetadata {
                world_info: None,
                enabled_world_info: Vec::new(),
                disabled_world_info: vec!["lore-1".to_string()],
                user_persona: None,
                extra: serde_json::Map::new(),
            },
            &settings,
            &store,
        )
        .expect("character lore ids");
        assert!(disabled.is_empty());
    }

    #[test]
    fn missing_worldbook_is_treated_as_absent_not_error() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let store = JsonStore::new(temp_dir.path().to_path_buf());

        let loaded =
            load_worldbook_by_id(&store, "missing-id").expect("missing lore should not error");
        assert!(loaded.is_none());
    }

    #[test]
    fn apply_prompt_only_regex_transforms_prompt_payload() {
        let mut request = AssembledRequest {
            system_prompt: "hero info".to_string(),
            messages: vec![crate::st::runtime_assembly::AssembledMessage {
                role: "user".to_string(),
                content: "hello hero".to_string(),
                attachments: Vec::new(),
            }],
            ..Default::default()
        };
        let settings = crate::st::RegexExtensionSettings {
            regex: vec![crate::st::RegexScriptData {
                id: "regex-1".to_string(),
                script_name: "prompt".to_string(),
                find_regex: "hero".to_string(),
                replace_string: "mage".to_string(),
                trim_strings: Vec::new(),
                placement: vec![
                    crate::st::RegexPlacement::USER_INPUT,
                    crate::st::RegexPlacement::WORLD_INFO,
                ],
                disabled: false,
                markdown_only: false,
                prompt_only: true,
                run_on_edit: true,
                substitute_regex: crate::st::SubstituteRegex::NONE,
                min_depth: None,
                max_depth: None,
            }],
            ..Default::default()
        };

        apply_prompt_only_regex(&mut request, &settings, Some("Hero"), None);

        assert_eq!(request.system_prompt, "mage info");
        assert_eq!(request.messages[0].content, "hello mage");
    }

    #[tokio::test]
    async fn provider_capability_fail_fast_blocks_attachments_before_blob_read() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let compiled_contract = CompiledProviderContractView {
            key: ProviderContractCacheKey {
                api_config_id: "deepseek".to_string(),
                provider_kind: "deepseek".to_string(),
                protocol_kind: "deepseek_chat".to_string(),
                model: "deepseek-chat".to_string(),
                base_url: "https://api.deepseek.com".to_string(),
                provider_variant: None,
            },
            contract: json!({}),
            input_capabilities: Some(json!({
                "text": { "supported": true },
                "image": { "supported": false },
                "pdf": { "supported": false }
            })),
            multimodal_policy: None,
        };
        let request = AssembledRequest {
            messages: vec![AssembledMessage {
                role: "user".to_string(),
                content: "see this".to_string(),
                attachments: vec![AssembledAttachmentRef {
                    attachment_id: "missing-attachment".to_string(),
                    kind: "image".to_string(),
                    mime_type: "image/png".to_string(),
                    filename: "missing.png".to_string(),
                    size_bytes: Some(10),
                }],
            }],
            ..Default::default()
        };

        let error = assembled_to_chat_request(
            temp_dir.path(),
            &compiled_contract,
            "req-1".to_string(),
            "deepseek".to_string(),
            request,
        )
        .await
        .expect_err("unsupported image should fail before reading missing blob");

        assert!(error.contains("does not support image input"));
        assert!(!error.contains("Failed to read attachment"));
    }

    #[test]
    fn sampling_contract_drops_and_maps_unsupported_fields() {
        let source = AssembledSamplingParams {
            temperature: Some(3.0),
            top_p: Some(1.5),
            top_k: Some(40),
            repetition_penalty: Some(1.4),
            frequency_penalty: Some(0.0),
            presence_penalty: Some(3.0),
        };

        let openai = sampling_for_contract("openai_chat_completions", &source);
        assert_eq!(openai.temperature, Some(2.0));
        assert_eq!(openai.top_p, Some(1.0));
        assert_eq!(openai.top_k, None);
        assert_eq!(openai.repetition_penalty, None);
        assert!((openai.frequency_penalty.unwrap() - 0.4).abs() < 0.000001);
        assert_eq!(openai.presence_penalty, Some(2.0));

        let anthropic = sampling_for_contract("anthropic_messages", &source);
        assert_eq!(anthropic.temperature, Some(1.0));
        assert_eq!(anthropic.top_k, Some(40));
        assert_eq!(anthropic.frequency_penalty, None);
        assert_eq!(anthropic.presence_penalty, None);
        assert_eq!(anthropic.repetition_penalty, None);

        let gemini = sampling_for_contract(
            "gemini_generate_content",
            &AssembledSamplingParams {
                top_k: Some(0),
                ..source
            },
        );
        assert_eq!(gemini.top_k, None);
        assert_eq!(gemini.frequency_penalty, None);
    }
}

// ============================================================================
// Provider 请求映射命令
// ============================================================================

/// 映射请求到 Provider 格式
#[tauri::command]
pub async fn map_request_to_provider(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    request: AssembledRequest,
    api_config_id: String,
    _provider_type: String,
    _model: String,
) -> Result<serde_json::Value, String> {
    let app_state = state.inner().clone();
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir.clone());
    let api_config: ApiConfig = {
        let value = store.read(&format!("api_configs/{}.json", api_config_id))?;
        serde_json::from_value(value).map_err(|e| format!("Failed to parse API config: {}", e))?
    };
    let resolved_provider_type = api_config.provider.clone();
    let compiled_contract = get_compiled_contract_view(&app_state, &api_config).await?;
    let (_text_supported, image_supported, pdf_supported) =
        connection_supports_attachments(&compiled_contract);
    let (has_image, has_pdf) = ProviderRequestMapper::request_contains_attachments(&request);
    if has_image && !image_supported {
        return Err(format!(
            "Current API config does not support image input for provider {}",
            resolved_provider_type
        ));
    }
    if has_pdf && !pdf_supported {
        return Err(format!(
            "Current API config does not support PDF input for provider {}",
            resolved_provider_type
        ));
    }

    let preview_request = assembled_to_chat_request(
        &data_dir,
        &compiled_contract,
        "preview-request".to_string(),
        api_config_id,
        request,
    )
    .await?;

    let preview = build_provider_request_preview(
        &data_dir,
        &api_config,
        &compiled_contract.key.protocol_kind,
        preview_request,
    )
    .await?;

    Ok(preview)
}

async fn build_provider_request_preview(
    data_dir: &std::path::Path,
    api_config: &ApiConfig,
    protocol_kind: &str,
    request: ChatRequest,
) -> Result<serde_json::Value, String> {
    let provider = create_provider(api_config, Some(data_dir.to_path_buf()))?;
    let provider_any = provider.as_ref();

    match protocol_kind {
        "openai_responses" => {
            let preview = crate::api::openai_responses::build_request_body_preview(
                api_config, data_dir, &request, None,
            )
            .await?;
            Ok(preview)
        }
        "openai_chat_completions" | "deepseek_chat" => {
            let preview = crate::api::openai_chat::build_request_body_preview(
                api_config, data_dir, &request, None,
            )
            .await?;
            Ok(preview)
        }
        "anthropic_messages" => {
            let preview = crate::api::anthropic::build_request_body_preview(
                api_config, data_dir, &request, None,
            )
            .await?;
            Ok(preview)
        }
        "gemini_generate_content" => {
            let preview = crate::api::gemini::build_request_body_preview(
                api_config, data_dir, &request, None,
            )
            .await?;
            Ok(preview)
        }
        "claude_code_interface" => {
            let _ = provider_any;
            Ok(crate::api::claude_code::build_request_body_preview(
                api_config, &request, None,
            )?)
        }
        _ => Err(format!("Unknown provider protocol: {}", protocol_kind)),
    }
}

/// Start a streaming chat request for ST mode.
/// Returns a stream_id immediately, then emits events:
/// - "st-stream-start" with StreamStartEvent
/// - "st-stream-chunk" with StreamChunkEvent for each chunk
/// - "st-stream-error" with StreamErrorEvent on error
/// - "st-stream-end" with stream_id when done
#[tauri::command]
pub async fn start_st_chat_stream(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: AssembleRequestInput,
) -> Result<String, String> {
    let stream_id = Uuid::new_v4().to_string();

    // Prepare everything needed for the stream
    let assembled = assemble_st_request(app.clone(), input.clone()).await?;
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir.clone());

    let api_config: ApiConfig = {
        let value = store.read(&format!("api_configs/{}.json", input.api_config_id))?;
        serde_json::from_value(value).map_err(|e| format!("Failed to parse API config: {}", e))?
    };
    let compiled_contract = get_compiled_contract_view(state.inner(), &api_config).await?;
    let provider = create_provider(&api_config, Some(data_dir.clone()))?;

    let request_id = Uuid::new_v4().to_string();
    let request = assembled_to_chat_request(
        &data_dir,
        &compiled_contract,
        request_id.clone(),
        input.api_config_id.clone(),
        assembled.request,
    )
    .await?;

    // Enable streaming on the request
    let mut stream_request = request.clone();
    stream_request.stream = true;

    // Build request URL for logging
    let request_url = build_request_url(&api_config);

    let log_context = LogContext {
        mode: LogMode::St,
        world_id: None,
        session_id: Some(input.session_id.clone()),
        scene_turn_id: None,
        character_id: input.character_id.clone(),
        trace_id: None,
        llm_node: LlmNode::STChat,
        api_config_id: input.api_config_id,
        request_id: request_id.clone(),
    };

    // Log start before spawning
    {
        let store_guard = state.sqlite_store.read().await;
        if let Some(sqlite_store) = store_guard.as_ref() {
            // Build actual request body that will be sent to the API
            let request_json = build_provider_request_preview(
                &data_dir,
                &api_config,
                &compiled_contract.key.protocol_kind,
                stream_request.clone(),
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("[ST Stream Log] Failed to build request preview: {}, falling back to ChatRequest", e);
                serde_json::to_value(&stream_request).unwrap_or(serde_json::Value::Null)
            });
            sqlite_store
                .llm_logger()
                .log_start(
                    &log_context,
                    &request_json,
                    request_url.as_deref(),
                    &api_config.provider,
                    &api_config.model,
                    "chat_stream",
                    None,
                )
                .await;
        }
    }

    // Get Arc clone for use in spawned task
    let sqlite_store_arc = state.sqlite_store.clone();

    // Emit start event
    app.emit(
        "st-stream-start",
        StreamStartEvent {
            stream_id: stream_id.clone(),
            request_id: request_id.clone(),
        },
    )
    .map_err(|e| format!("Failed to emit start event: {}", e))?;

    // Spawn background task to process stream
    let app_clone = app.clone();
    let request_id_clone = request_id.clone();
    let stream_id_for_task = stream_id.clone();

    tokio::spawn(async move {
        match provider.chat_stream(stream_request).await {
            Ok(mut stream) => {
                let mut full_content = String::new();
                let mut finish_reason: Option<String> = None;
                let mut raw_sse_events: Vec<serde_json::Value> = Vec::new();

                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            // Collect raw SSE events for logging
                            if let Some(raw_data) = &chunk.raw_sse_data {
                                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(raw_data) {
                                    raw_sse_events.push(json_value);
                                }
                            }

                            if !chunk.delta.is_empty() {
                                full_content.push_str(&chunk.delta);
                            }
                            if chunk.finish_reason.is_some() {
                                finish_reason = chunk.finish_reason.clone();
                            }

                            // Emit chunk event
                            let _ = app_clone.emit(
                                "st-stream-chunk",
                                StreamChunkEvent {
                                    stream_id: stream_id_for_task.clone(),
                                    delta: chunk.delta,
                                    finish_reason: chunk.finish_reason,
                                },
                            );
                        }
                        Err(e) => {
                            // Emit error event
                            let _ = app_clone.emit(
                                "st-stream-error",
                                StreamErrorEvent {
                                    stream_id: stream_id_for_task.clone(),
                                    error: e.clone(),
                                },
                            );
                            // Log failure
                            {
                                let store_guard = sqlite_store_arc.read().await;
                                if let Some(ref store) = store_guard.as_ref() {
                                    store.llm_logger().log_failure(&request_id_clone, &e).await;
                                }
                            }
                            let _ = app_clone.emit("st-stream-end", &stream_id_for_task);
                            return;
                        }
                    }
                }

                // Log success with raw SSE events array
                {
                    let store_guard = sqlite_store_arc.read().await;
                    if let Some(ref store) = store_guard.as_ref() {
                        // Store raw SSE events as JSON array - this is the actual data received from API
                        let response_json = serde_json::Value::Array(raw_sse_events);
                        store
                            .llm_logger()
                            .log_success(
                                &request_id_clone,
                                &response_json,
                                None,
                                None,
                            )
                            .await;
                    }
                }

                // Emit end event
                let _ = app_clone.emit("st-stream-end", &stream_id_for_task);
            }
            Err(e) => {
                // Failed to create stream
                let _ = app_clone.emit(
                    "st-stream-error",
                    StreamErrorEvent {
                        stream_id: stream_id_for_task.clone(),
                        error: e.clone(),
                    },
                );
                // Log failure
                {
                    let store_guard = sqlite_store_arc.read().await;
                    if let Some(ref store) = store_guard.as_ref() {
                        store.llm_logger().log_failure(&request_id_clone, &e).await;
                    }
                }
                let _ = app_clone.emit("st-stream-end", &stream_id_for_task);
            }
        }
    });

    Ok(stream_id)
}
