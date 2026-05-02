//! Chat commands for Tauri

use crate::api::provider::*;
use crate::logging::context::{LogContext, LogMode, LlmNode};
use crate::storage::json_store::JsonStore;
use crate::storage::paths::app_data_root;
use crate::storage::st_resources::ApiConfig;
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, State};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequestMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponseData {
    pub request_id: String,
    pub content: String,
    pub reasoning: Option<String>,
    pub token_usage: Option<TokenUsageData>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenUsageData {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl From<TokenUsage> for TokenUsageData {
    fn from(u: TokenUsage) -> Self {
        Self {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }
    }
}

/// Get the data directory path
fn get_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app_data_root(app)
}

/// Send a chat message
#[tauri::command]
pub async fn send_chat_message(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    api_config_id: String,
    system_prompt: String,
    messages: Vec<ChatRequestMessage>,
) -> Result<ChatResponseData, String> {
    // Load API config
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);
    let value = store.read(&format!("api_configs/{}.json", api_config_id))?;
    let config: ApiConfig = serde_json::from_value(value)
        .map_err(|e| format!("Failed to parse API config: {}", e))?;

    // Create provider
    let provider = create_provider(&config)?;

    // Build request
    let request_id = Uuid::new_v4().to_string();
    let mut chat_messages: Vec<ChatMessage> = Vec::new();

    // Add system prompt if provided
    if !system_prompt.is_empty() {
        chat_messages.push(ChatMessage::system(&system_prompt));
    }

    // Add conversation messages
    for msg in messages {
        let role = match msg.role.as_str() {
            "user" => ChatRole::User,
            "assistant" => ChatRole::Assistant,
            _ => ChatRole::User,
        };
        chat_messages.push(ChatMessage {
            role,
            content: vec![ContentPart::Text { text: msg.content }],
            name: None,
        });
    }

    let request = ChatRequest {
        request_id: request_id.clone(),
        api_config_id: api_config_id.clone(),
        messages: chat_messages,
        max_tokens: Some(4096),
        sampling: SamplingParams {
            temperature: config.settings.get("temperature").and_then(|v| v.as_f64()),
            top_p: config.settings.get("top_p").and_then(|v| v.as_f64()),
            ..Default::default()
        },
        stop_sequences: Vec::new(),
        stream: false,
        reasoning: None,
        response_format: None,
        provider_overrides: serde_json::Value::Null,
    };

    // Log start
    let log_context = LogContext {
        mode: LogMode::St,
        world_id: None,
        scene_turn_id: None,
        character_id: None,
        trace_id: None,
        llm_node: LlmNode::STChat,
        api_config_id: api_config_id.clone(),
        request_id: request_id.clone(),
    };

    let store_guard = state.sqlite_store.read().await;
    if let Some(store) = store_guard.as_ref() {
        let request_json = serde_json::to_value(&request).unwrap_or(serde_json::Value::Null);
        store.llm_logger().log_start(
            &log_context,
            &request_json,
            &config.provider,
            &config.model,
            "chat",
            None,
        ).await;
    }

    // Send request
    let response = provider.chat(request).await;

    match response {
        Ok(resp) => {
            // Log success
            if let Some(store) = store_guard.as_ref() {
                store.llm_logger().log_success(
                    &request_id,
                    &serde_json::json!({"content": &resp.content}),
                    resp.token_usage.as_ref().map(|u| serde_json::json!({
                        "prompt_tokens": u.prompt_tokens,
                        "completion_tokens": u.completion_tokens,
                        "total_tokens": u.total_tokens
                    })),
                ).await;
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
            // Log failure
            if let Some(store) = store_guard.as_ref() {
                store.llm_logger().log_failure(&request_id, &e).await;
            }
            Err(e)
        }
    }
}

/// Send a structured chat message
#[tauri::command]
pub async fn send_structured_chat_message(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    api_config_id: String,
    system_prompt: String,
    messages: Vec<ChatRequestMessage>,
    schema: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // Load API config
    let data_dir = get_data_dir(&app)?;
    let store = JsonStore::new(data_dir);
    let value = store.read(&format!("api_configs/{}.json", api_config_id))?;
    let config: ApiConfig = serde_json::from_value(value)
        .map_err(|e| format!("Failed to parse API config: {}", e))?;

    // Create provider
    let provider = create_provider(&config)?;

    // Build request
    let request_id = Uuid::new_v4().to_string();
    let mut chat_messages: Vec<ChatMessage> = Vec::new();

    if !system_prompt.is_empty() {
        chat_messages.push(ChatMessage::system(&system_prompt));
    }

    for msg in messages {
        let role = match msg.role.as_str() {
            "user" => ChatRole::User,
            "assistant" => ChatRole::Assistant,
            _ => ChatRole::User,
        };
        chat_messages.push(ChatMessage {
            role,
            content: vec![ContentPart::Text { text: msg.content }],
            name: None,
        });
    }

    let request = ChatRequest {
        request_id: request_id.clone(),
        api_config_id: api_config_id.clone(),
        messages: chat_messages,
        max_tokens: Some(4096),
        sampling: SamplingParams {
            temperature: config.settings.get("temperature").and_then(|v| v.as_f64()),
            top_p: config.settings.get("top_p").and_then(|v| v.as_f64()),
            ..Default::default()
        },
        stop_sequences: Vec::new(),
        stream: false,
        reasoning: None,
        response_format: None,
        provider_overrides: serde_json::Value::Null,
    };

    let log_context = LogContext {
        mode: LogMode::St,
        world_id: None,
        scene_turn_id: None,
        character_id: None,
        trace_id: None,
        llm_node: LlmNode::STChat,
        api_config_id: api_config_id.clone(),
        request_id: request_id.clone(),
    };

    let store_guard = state.sqlite_store.read().await;
    if let Some(store) = store_guard.as_ref() {
        let request_json = serde_json::to_value(&request).unwrap_or(serde_json::Value::Null);
        store.llm_logger().log_start(
            &log_context,
            &request_json,
            &config.provider,
            &config.model,
            "chat_structured",
            Some(&schema),
        ).await;
    }

    let response = provider.chat_structured(request, schema).await;
    match response {
        Ok(value) => {
            if let Some(store) = store_guard.as_ref() {
                store.llm_logger().log_success(&request_id, &value, None).await;
            }
            Ok(value)
        }
        Err(e) => {
            if let Some(store) = store_guard.as_ref() {
                store.llm_logger().log_failure(&request_id, &e).await;
            }
            Err(e)
        }
    }
}

fn create_provider(config: &ApiConfig) -> Result<Box<dyn AIProvider>, String> {
    let api_key = config.api_key.clone().unwrap_or_default();
    let base_url = config.base_url.clone();
    let model = config.model.clone();

    match config.provider.as_str() {
        "openai_chat" => Ok(Box::new(crate::api::openai_chat::OpenAIChatProvider::new(
            api_key,
            base_url,
            model,
        ))),
        "openai_responses" => Ok(Box::new(crate::api::openai_responses::OpenAIResponsesProvider::new(
            api_key,
            base_url,
            model,
        ))),
        "anthropic" => Ok(Box::new(crate::api::anthropic::AnthropicProvider::new(
            api_key,
            base_url,
            model,
        ))),
        "gemini" => Ok(Box::new(crate::api::gemini::GeminiProvider::new(
            api_key,
            base_url,
            model,
        ))),
        "deepseek" => Ok(Box::new(crate::api::deepseek::DeepSeekProvider::new(
            api_key,
            base_url,
            model,
        ))),
        "claude_code" => Ok(Box::new(crate::api::claude_code::ClaudeCodeProvider::new(
            base_url.unwrap_or_else(|| "http://localhost:8080".to_string()),
            model,
        ))),
        _ => Err(format!("Unknown provider: {}", config.provider)),
    }
}
