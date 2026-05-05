//! Chat commands for Tauri

use crate::api::provider::*;
use crate::storage::paths::app_data_root;
use crate::storage::st_resources::ApiConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, State};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelInfoData {
    pub id: String,
    pub display_name: Option<String>,
    pub owned_by: Option<String>,
    pub max_input_tokens: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub capabilities: Option<serde_json::Value>,
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

impl From<ModelInfo> for ModelInfoData {
    fn from(m: ModelInfo) -> Self {
        Self {
            id: m.id,
            display_name: m.display_name,
            owned_by: m.owned_by,
            max_input_tokens: m.max_input_tokens,
            max_output_tokens: m.max_output_tokens,
            capabilities: m.capabilities,
        }
    }
}

pub(crate) fn create_provider(
    config: &ApiConfig,
    data_dir: Option<PathBuf>,
) -> Result<Box<dyn AIProvider>, String> {
    let api_key = config.api_key.clone().unwrap_or_default();
    let base_url = config.base_url.clone();
    let model = config.model.clone();

    match config.provider.as_str() {
        "openai_chat" => Ok(Box::new(crate::api::openai_chat::OpenAIChatProvider::new(
            api_key, base_url, model, data_dir,
        ))),
        "openai_responses" => Ok(Box::new(
            crate::api::openai_responses::OpenAIResponsesProvider::new(
                api_key, base_url, model, data_dir,
            ),
        )),
        "anthropic" => Ok(Box::new(crate::api::anthropic::AnthropicProvider::new(
            api_key, base_url, model, data_dir,
        ))),
        "gemini" => Ok(Box::new(crate::api::gemini::GeminiProvider::new(
            api_key, base_url, model, data_dir,
        ))),
        "deepseek" => Ok(Box::new(crate::api::deepseek::DeepSeekProvider::new(
            api_key, base_url, model, data_dir,
        ))),
        "claude_code" => Ok(Box::new(crate::api::claude_code::ClaudeCodeProvider::new(
            base_url.unwrap_or_else(|| "http://localhost:8080".to_string()),
            model,
        ))),
        _ => Err(format!("Unknown provider: {}", config.provider)),
    }
}

/// List available models from a provider
#[tauri::command]
pub async fn list_models(app: AppHandle, config_id: String) -> Result<Vec<ModelInfoData>, String> {
    let data_dir = app_data_root(&app)?;

    // Load the API config
    let store = crate::storage::json_store::JsonStore::new(data_dir.clone());
    let value = store.read(&format!("api_configs/{}.json", config_id))?;
    let config: ApiConfig =
        serde_json::from_value(value).map_err(|e| format!("Failed to parse API config: {}", e))?;

    let provider = create_provider(&config, Some(data_dir))?;
    let models = provider.list_models().await?;

    Ok(models.into_iter().map(ModelInfoData::from).collect())
}
