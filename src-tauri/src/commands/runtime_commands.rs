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
use crate::st::runtime_assembly::AssembledAttachmentRef;
use crate::st::{
    AssembledRequest, ContextTemplate, GlobalAppState, InstructTemplate, PromptPreset,
    ProviderRequestMapper, ReasoningTemplate, RegexEngine, RegexPlacement, RegexRunOptions,
    RequestAssembler, RuntimeContext, STChatMessage, STChatMetadata, STSessionData,
    STWorldInfoSettings, SamplerPreset, SystemPrompt, WorldInfoInjectionResult, WorldInfoInjector,
    WorldInfoSource,
};
use crate::storage::json_store::JsonStore;
use crate::storage::paths::{app_data_root, safe_join};
use crate::storage::st_resources::{
    ApiConfig, ChatAttachmentKind, ChatAttachmentRecord, ChatSession, TavernCardV3, WorldInfoFile,
};
use crate::AppState;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, State};
use uuid::Uuid;

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
    Ok(preset.sampler.unwrap_or_else(|| SamplerPreset::new(&name)))
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
    Ok(preset.context.unwrap_or_else(|| ContextTemplate::new(&name)))
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
    Ok(preset.prompt.unwrap_or_else(|| PromptPreset::new(&name)))
}

// ============================================================================
// 预设管理命令
// ============================================================================

/// 预设文件结构（包含所有类型）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetFile {
    pub name: String,
    pub sampler: Option<SamplerPreset>,
    pub instruct: Option<InstructTemplate>,
    pub context: Option<ContextTemplate>,
    pub sysprompt: Option<SystemPrompt>,
    pub reasoning: Option<ReasoningTemplate>,
    pub prompt: Option<PromptPreset>,
    pub source_api_id: Option<String>,
    #[serde(default)]
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

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
fn merge_builtin_prompt_items(preset: &mut PresetFile) {
    use crate::st::preset::{
        get_builtin_prompt_definitions, BuiltinPromptSource, PromptItem, PromptOrder,
        PromptOrderItem,
    };

    let builtin_defs = get_builtin_prompt_definitions();

    // 确保 prompt 字段存在
    if preset.prompt.is_none() {
        preset.prompt = Some(crate::st::preset::PromptPreset::new(&preset.name));
    }
    let prompt = preset.prompt.as_mut().unwrap();

    // 确保 prompt_order 存在
    if prompt.prompt_order.is_empty() {
        prompt.prompt_order.push(PromptOrder {
            character_id: 100000,
            order: Vec::new(),
        });
    }

    // 获取现有的 order map
    let order = &mut prompt.prompt_order[0].order;
    let order_map: std::collections::HashMap<String, (bool, Option<i32>)> = order
        .iter()
        .map(|item| {
            (
                item.identifier.clone(),
                (item.enabled, item.position),
            )
        })
        .collect();

    // 清空并重建 prompts 列表，先添加内置条目
    let mut new_prompts: Vec<PromptItem> = Vec::new();
    let mut new_order: Vec<PromptOrderItem> = Vec::new();

    for def in builtin_defs {
        let (enabled, position) = order_map
            .get(&def.identifier)
            .copied()
            .unwrap_or((def.default_enabled, None));

        let editable = def.source == BuiltinPromptSource::Static;

        new_prompts.push(PromptItem {
            identifier: def.identifier.clone(),
            name: def.name.clone(),
            role: def.role.clone(),
            content: def.content.clone(),
            system_prompt: def.system_prompt,
            marker: def.marker,
            enabled: Some(enabled),
            injection_position: None,
            injection_depth: None,
            injection_order: None,
            forbid_overrides: None,
            injection_trigger: Vec::new(),
            builtin: true,
            editable,
            description: def.description.clone(),
        });

        new_order.push(PromptOrderItem {
            identifier: def.identifier.clone(),
            enabled,
            position,
        });
    }

    // 添加用户自定义条目（非内置）
    for item in &prompt.prompts {
        if !item.identifier.starts_with("builtin:") {
            let (enabled, position) = order_map
                .get(&item.identifier)
                .copied()
                .unwrap_or((true, None));

            new_prompts.push(item.clone());
            new_order.push(PromptOrderItem {
                identifier: item.identifier.clone(),
                enabled,
                position,
            });
        }
    }

    prompt.prompts = new_prompts;
    prompt.prompt_order[0].order = new_order;
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
    if let Some(prompt) = &mut preset.prompt {
        // 过滤 prompts，只保留非内置条目
        prompt.prompts.retain(|item| !item.identifier.starts_with("builtin:"));

        // 过滤 prompt_order，只保留启用状态和排序位置
        for order in &mut prompt.prompt_order {
            for item in &mut order.order {
                // 内置条目只保留 enabled 和 position，清除其他字段
                if item.identifier.starts_with("builtin:") {
                    // 保持 enabled 和 position 即可
                }
            }
        }
    }

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
            disabled_world_info: chat_session.chat_metadata.disabled_world_info.clone(),
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
    let active_preset_name = resolve_active_preset_name(input.preset_name.as_deref(), &global_state);
    let preset: Option<PresetFile> = if let Some(name) = active_preset_name.as_deref() {
        if !name.is_empty() {
            let value = store.read(&format!("presets/{}.json", name))?;
            Some(
                serde_json::from_value(value)
                    .map_err(|e| format!("Failed to parse preset '{}': {}", name, e))?,
            )
        } else {
            None
        }
    } else {
        None
    };

    // 从预设中提取各类型配置
    let sampler_preset = preset.as_ref().and_then(|p| p.sampler.clone());
    let instruct_template = preset.as_ref().and_then(|p| p.instruct.clone());
    let context_template = preset.as_ref().and_then(|p| p.context.clone());
    let system_prompt = preset.as_ref().and_then(|p| p.sysprompt.clone());
    let reasoning_template = preset.as_ref().and_then(|p| p.reasoning.clone());
    let prompt_preset = preset.as_ref().and_then(|p| p.prompt.clone());

    // 5. 构建全局扫描数据
    let global_scan_data = GlobalScanData {
        persona_description: String::new(),
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
            Some(
                injector
                    .check_world_info(
                        &session.messages,
                        input.max_context,
                        &input.world_info_settings,
                        sources,
                        &global_scan_data,
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
    if let Some(sqlite_store) = store_guard.as_ref() {
        let request_json = serde_json::to_value(&request).unwrap_or(serde_json::Value::Null);
        sqlite_store
            .llm_logger()
            .log_start(
                &log_context,
                &request_json,
                &api_config.provider,
                &api_config.model,
                "chat",
                None,
            )
            .await;
    }

    match provider.chat(request).await {
        Ok(resp) => {
            if let Some(sqlite_store) = store_guard.as_ref() {
                sqlite_store
                    .llm_logger()
                    .log_success(
                        &request_id,
                        &serde_json::json!({"content": &resp.content}),
                        resp.token_usage.as_ref().map(|u| {
                            serde_json::json!({
                                "prompt_tokens": u.prompt_tokens,
                                "completion_tokens": u.completion_tokens,
                                "total_tokens": u.total_tokens
                            })
                        }),
                    )
                    .await;
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
        sampling: SamplingParams {
            temperature: assembled.sampling.temperature,
            top_p: assembled.sampling.top_p,
            top_k: assembled.sampling.top_k.and_then(|v| u32::try_from(v).ok()),
            repetition_penalty: assembled.sampling.repetition_penalty,
            frequency_penalty: assembled.sampling.frequency_penalty,
            presence_penalty: assembled.sampling.presence_penalty,
        },
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
    let mut prompt = PromptPreset::new(name);
    prompt.wi_format = "{{wi}}".to_string();
    prompt.scenario_format = "Scenario: {{scenario}}".to_string();
    prompt.personality_format = "Personality: {{personality}}".to_string();
    prompt.new_chat_prompt = String::new();

    PresetFile {
        name: name.to_string(),
        sampler: Some(SamplerPreset::new(name)),
        instruct: Some(InstructTemplate::new(name)),
        context: Some(ContextTemplate::new(name)),
        sysprompt: Some(SystemPrompt::new(name)),
        reasoning: Some(ReasoningTemplate::new(name)),
        prompt: Some(prompt),
        source_api_id: None,
        extensions: serde_json::Map::new(),
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

    let sources = collect_world_info_sources(
        &store,
        character.as_ref(),
        &STChatMetadata {
            world_info: chat_session.chat_metadata.world_info.clone(),
            disabled_world_info: chat_session.chat_metadata.disabled_world_info.clone(),
            extra: chat_session.chat_metadata.extra.clone(),
        },
        &input.world_info_settings,
        input.chat_lore_id.as_deref(),
        &global_lore_ids,
    )?;

    // 3. 构建全局扫描数据
    let global_scan_data = GlobalScanData {
        persona_description: String::new(),
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

    // 4. 执行注入
    let mut injector = WorldInfoInjector::new();
    Ok(injector
        .check_world_info(
            &messages,
            input.max_context,
            &input.world_info_settings,
            sources,
            &global_scan_data,
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

    if let Some(lore_id) = chat_lore_id {
        if !disabled_world_info.contains(lore_id) {
            if let Some(worldbook) = load_worldbook_by_id(store, lore_id)? {
                seen_lore_ids.insert(lore_id.to_string());
                sources.push(WorldInfoSource::ChatLore(worldbook));
            }
        }
    } else if let Some(lore_id) = chat_metadata.world_info.as_deref() {
        if !disabled_world_info.contains(lore_id) {
            if let Some(worldbook) = load_worldbook_by_id(store, lore_id)? {
                seen_lore_ids.insert(lore_id.to_string());
                sources.push(WorldInfoSource::ChatLore(worldbook));
            }
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
    };
    use crate::config::llm_contracts::{CompiledProviderContractView, ProviderContractCacheKey};
    use crate::st::runtime_assembly::AssembledAttachmentRef;
    use crate::st::{AssembledMessage, AssembledRequest};
    use crate::st::{STChatMetadata, STWorldInfoSettings};
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
        assert!(preset.sampler.is_some());
        assert!(preset.instruct.is_some());
        assert!(preset.context.is_some());
        assert!(preset.sysprompt.is_some());
        assert!(preset.reasoning.is_some());
        assert!(preset.prompt.is_some());
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
                disabled_world_info: vec!["lore-1".to_string()],
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
