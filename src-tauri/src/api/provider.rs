//! AI Provider trait and common types

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

/// Chat role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChatRole {
    System,
    Developer,
    User,
    Assistant,
    Tool,
}

/// Content part
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageRef { image_url: ImageUrl },
    #[serde(rename = "tool_result")]
    ToolResult { tool_call_id: String, content: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: Vec<ContentPart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ChatMessage {
    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: ChatRole::System,
            content: vec![ContentPart::Text { text: text.into() }],
            name: None,
        }
    }

    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            content: vec![ContentPart::Text { text: text.into() }],
            name: None,
        }
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: vec![ContentPart::Text { text: text.into() }],
            name: None,
        }
    }
}

/// Sampling parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SamplingParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repetition_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
}

/// Reasoning parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_tokens: Option<u32>,
    #[serde(default)]
    pub exclude_reasoning_text_from_response: bool,
}

/// Response format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseFormat {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "json_object")]
    JsonObject,
    #[serde(rename = "json_schema")]
    JsonSchema { schema: serde_json::Value, strict: bool },
}

/// Chat request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub request_id: String,
    pub api_config_id: String,
    pub messages: Vec<ChatMessage>,
    pub sampling: SamplingParams,
    #[serde(default)]
    pub stop_sequences: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ReasoningParams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(default)]
    pub provider_overrides: serde_json::Value,
}

/// Chat response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub request_id: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_usage: Option<TokenUsage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Stream chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub delta: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// AI Provider trait
#[async_trait]
pub trait AIProvider: Send + Sync {
    /// Provider name
    fn name(&self) -> &str;

    /// Available models
    fn models(&self) -> Vec<String>;

    /// Free text chat (ST mode)
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, String>;

    /// Structured JSON output (Agent mode)
    async fn chat_structured(
        &self,
        request: ChatRequest,
        schema: serde_json::Value,
    ) -> Result<serde_json::Value, String>;

    /// Streaming output
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, String>> + Send>>, String>;
}

/// Provider type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderType {
    OpenAIChat,
    OpenAIResponses,
    Anthropic,
    Gemini,
    DeepSeek,
    ClaudeCode,
}

impl ProviderType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" | "openai_chat" => Some(Self::OpenAIChat),
            "openai_responses" => Some(Self::OpenAIResponses),
            "anthropic" => Some(Self::Anthropic),
            "gemini" | "google" => Some(Self::Gemini),
            "deepseek" => Some(Self::DeepSeek),
            "claude_code" => Some(Self::ClaudeCode),
            _ => None,
        }
    }
}
