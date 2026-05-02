//! DeepSeek API provider (OpenAI Chat compatible)

use crate::api::openai_chat::OpenAIChatProvider;
use crate::api::provider::*;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// DeepSeek uses OpenAI Chat Completions compatible API
pub struct DeepSeekProvider {
    inner: OpenAIChatProvider,
}

impl DeepSeekProvider {
    pub fn new(api_key: String, base_url: Option<String>, default_model: String) -> Self {
        let base_url = base_url.unwrap_or_else(|| "https://api.deepseek.com/v1".to_string());
        Self {
            inner: OpenAIChatProvider::new(api_key, Some(base_url), default_model),
        }
    }
}

#[async_trait]
impl AIProvider for DeepSeekProvider {
    fn name(&self) -> &str {
        "deepseek"
    }

    fn models(&self) -> Vec<String> {
        vec![
            "deepseek-chat".to_string(),
            "deepseek-coder".to_string(),
            "deepseek-reasoner".to_string(),
        ]
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        self.inner.chat(request).await
    }

    async fn chat_structured(
        &self,
        request: ChatRequest,
        schema: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        // DeepSeek supports JSON mode
        self.inner.chat_structured(request, schema).await
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, String>> + Send>>, String> {
        self.inner.chat_stream(request).await
    }
}