//! Anthropic Messages API provider

use crate::api::anthropic_files::{
    invalidate_anthropic_file_cache, prepare_request_messages_with_file_cache,
};
use crate::api::provider::*;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::pin::Pin;

pub struct AnthropicProvider {
    client: Client,
    base_url: String,
    api_key: String,
    default_model: String,
    data_dir: Option<PathBuf>,
}

impl AnthropicProvider {
    pub fn new(
        api_key: String,
        base_url: Option<String>,
        default_model: String,
        data_dir: Option<PathBuf>,
    ) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| "https://api.anthropic.com/v1".to_string()),
            api_key,
            default_model,
            data_dir,
        }
    }
}

pub async fn build_request_body_preview(
    config: &crate::storage::st_resources::ApiConfig,
    data_dir: &std::path::Path,
    request: &ChatRequest,
    schema: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let provider = AnthropicProvider::new(
        config.api_key.clone().unwrap_or_default(),
        config.base_url.clone(),
        config.model.clone(),
        Some(data_dir.to_path_buf()),
    );
    match schema {
        Some(schema) => provider.build_structured_request(request, schema).await,
        None => provider.build_request_body(request, None).await,
    }
}

#[async_trait]
impl AIProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn models(&self) -> Vec<String> {
        vec![
            "claude-opus-4-20250514".to_string(),
            "claude-sonnet-4-20250514".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
            "claude-3-5-haiku-20241022".to_string(),
            "claude-3-opus-20240229".to_string(),
        ]
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, String> {
        let response = self
            .client
            .get(format!("{}/models", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error_text));
        }

        let body: AnthropicModelsResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(body
            .data
            .into_iter()
            .map(|m| ModelInfo {
                id: m.id,
                display_name: Some(m.display_name),
                owned_by: Some("anthropic".to_string()),
                max_input_tokens: Some(m.max_input_tokens),
                max_output_tokens: Some(m.max_tokens),
                capabilities: m
                    .capabilities
                    .map(|c| serde_json::to_value(c).unwrap_or_default()),
            })
            .collect())
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        self.chat_once_with_retry(request).await
    }

    async fn chat_structured(
        &self,
        request: ChatRequest,
        schema: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.chat_structured_once_with_retry(request, schema).await
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, String>> + Send>>, String> {
        self.chat_stream_once_with_retry(request).await
    }
}

impl AnthropicProvider {
    async fn chat_once_with_retry(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        match self.send_chat_once(&request).await {
            Ok(response) => Ok(response),
            Err(error) if Self::should_retry_missing_file(&error) => {
                self.invalidate_file_caches(&request).await?;
                self.send_chat_once(&request).await
            }
            Err(error) => Err(error),
        }
    }

    async fn send_chat_once(&self, request: &ChatRequest) -> Result<ChatResponse, String> {
        let request_body = self.build_request_body(request, None).await?;
        let response = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error_text));
        }

        let body: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let content = body
            .content
            .iter()
            .filter_map(|c| {
                if c.content_type == "text" {
                    c.text.clone()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        Ok(ChatResponse {
            request_id: request.request_id.clone(),
            content,
            reasoning: None,
            token_usage: body.usage.map(|u| TokenUsage {
                prompt_tokens: u.input_tokens,
                completion_tokens: u.output_tokens,
                total_tokens: u.input_tokens + u.output_tokens,
            }),
            finish_reason: body.stop_reason,
        })
    }

    async fn chat_structured_once_with_retry(
        &self,
        request: ChatRequest,
        schema: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        match self.send_chat_structured_once(&request, &schema).await {
            Ok(response) => Ok(response),
            Err(error) if Self::should_retry_missing_file(&error) => {
                self.invalidate_file_caches(&request).await?;
                self.send_chat_structured_once(&request, &schema).await
            }
            Err(error) => Err(error),
        }
    }

    async fn send_chat_structured_once(
        &self,
        request: &ChatRequest,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let request_body = self
            .build_structured_request(request, schema.clone())
            .await?;
        let response = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error_text));
        }

        let body: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let tool_result = body
            .content
            .iter()
            .find(|c| c.content_type == "tool_use")
            .and_then(|c| c.input.clone());

        tool_result.ok_or_else(|| "No tool use in response".to_string())
    }

    async fn chat_stream_once_with_retry(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, String>> + Send>>, String> {
        match self.send_chat_stream_once(&request).await {
            Ok(stream) => Ok(stream),
            Err(error) if Self::should_retry_missing_file(&error) => {
                self.invalidate_file_caches(&request).await?;
                self.send_chat_stream_once(&request).await
            }
            Err(error) => Err(error),
        }
    }

    async fn send_chat_stream_once(
        &self,
        request: &ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, String>> + Send>>, String> {
        let mut stream_request = request.clone();
        stream_request.stream = true;
        let request_body = self.build_request_body(&stream_request, None).await?;

        let response = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error_text));
        }

        let stream = response.bytes_stream().map(move |result| match result {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                for line in text.lines() {
                    if line.starts_with("data: ") {
                        let data = &line[6..];
                        if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(data) {
                            if event.event_type == "content_block_delta" {
                                if let Some(delta) = event.delta {
                                    if delta.delta_type == "text_delta" {
                                        return Ok(StreamChunk {
                                            delta: delta.text.clone(),
                                            finish_reason: None,
                                        });
                                    }
                                }
                            } else if event.event_type == "message_stop" {
                                return Ok(StreamChunk {
                                    delta: String::new(),
                                    finish_reason: Some("stop".to_string()),
                                });
                            }
                        }
                    }
                }
                Ok(StreamChunk {
                    delta: String::new(),
                    finish_reason: None,
                })
            }
            Err(e) => Err(format!("Stream error: {}", e)),
        });

        Ok(Box::pin(stream))
    }

    fn should_retry_missing_file(error: &str) -> bool {
        let lower = error.to_ascii_lowercase();
        (lower.contains("file") || lower.contains("document"))
            && (lower.contains("not found")
                || lower.contains("invalid")
                || lower.contains("expired"))
    }

    async fn invalidate_file_caches(&self, request: &ChatRequest) -> Result<(), String> {
        for message in &request.messages {
            for part in &message.content {
                if let ContentPart::FileRef { file } = part {
                    if file.file_id.is_some() || file.file_data.is_some() {
                        invalidate_anthropic_file_cache(
                            self.data_dir.as_deref(),
                            &self.base_url,
                            &self.api_key,
                            &request.api_config_id,
                            file,
                        )
                        .await?;
                    }
                }
            }
        }
        Ok(())
    }

    async fn build_request_body(
        &self,
        request: &ChatRequest,
        _schema: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let prepared_messages = prepare_request_messages_with_file_cache(
            &self.client,
            self.data_dir.as_deref(),
            &self.base_url,
            &self.api_key,
            &request.api_config_id,
            &request.messages,
        )
        .await?;
        // Anthropic separates system from messages
        let system_messages: Vec<_> = prepared_messages
            .iter()
            .filter(|m| m.role == ChatRole::System)
            .collect();
        let other_messages: Vec<_> = prepared_messages
            .iter()
            .filter(|m| m.role != ChatRole::System)
            .collect();

        let system_text = system_messages
            .iter()
            .filter_map(|m| {
                m.content
                    .iter()
                    .filter_map(|c| match c {
                        ContentPart::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .next()
            })
            .collect::<Vec<_>>()
            .join("\n");

        let mut body = serde_json::json!({
            "model": self.default_model,
            "max_tokens": request.max_tokens.unwrap_or(4096),
        });

        if !system_text.is_empty() {
            body["system"] = serde_json::json!(system_text);
        }

        body["messages"] = serde_json::json!(other_messages.iter().map(|m| {
            let role = match m.role {
                ChatRole::User => "user",
                ChatRole::Assistant => "assistant",
                _ => "user",
            };
            let content: Vec<_> = m.content.iter().map(|c| match c {
                ContentPart::Text { text } => serde_json::json!({
                    "type": "text",
                    "text": text
                }),
                ContentPart::ImageRef { image_url } => serde_json::json!({
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": image_mime_type_from_data_url(&image_url.url),
                        "data": image_base64_from_data_url(&image_url.url)
                    }
                }),
                ContentPart::FileRef { file } => {
                    if let Some(file_id) = &file.file_id {
                        serde_json::json!({
                            "type": "document",
                            "source": {
                                "type": "file",
                                "file_id": file_id
                            }
                        })
                    } else {
                        serde_json::json!({
                            "type": "document",
                            "source": {
                                "type": "base64",
                                "media_type": file.mime_type.clone().unwrap_or_else(|| "application/pdf".to_string()),
                                "data": file.file_data.clone().unwrap_or_default()
                            }
                        })
                    }
                }
                ContentPart::ToolResult { tool_call_id, content } => serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": tool_call_id,
                    "content": content
                }),
            }).collect();
            serde_json::json!({
                "role": role,
                "content": content
            })
        }).collect::<Vec<_>>());

        if let Some(temp) = request.sampling.temperature {
            body["temperature"] = serde_json::json!(temp);
        }
        if let Some(top_p) = request.sampling.top_p {
            body["top_p"] = serde_json::json!(top_p);
        }

        Ok(body)
    }

    async fn build_structured_request(
        &self,
        request: &ChatRequest,
        schema: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut body = self.build_request_body(request, None).await?;
        body["tools"] = serde_json::json!([{
            "name": "structured_output",
            "description": "Return structured JSON output",
            "input_schema": schema
        }]);
        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use super::AnthropicProvider;

    #[test]
    fn retry_missing_file_detects_expected_errors() {
        assert!(AnthropicProvider::should_retry_missing_file(
            "API error: document expired"
        ));
        assert!(AnthropicProvider::should_retry_missing_file(
            "API error: file not found"
        ));
        assert!(!AnthropicProvider::should_retry_missing_file(
            "API error: overloaded"
        ));
    }
}

fn image_mime_type_from_data_url(url: &str) -> String {
    url.strip_prefix("data:")
        .and_then(|rest| rest.split(";base64,").next())
        .filter(|mime| !mime.is_empty())
        .unwrap_or("image/jpeg")
        .to_string()
}

fn image_base64_from_data_url(url: &str) -> String {
    url.split(";base64,").nth(1).unwrap_or_default().to_string()
}

// Anthropic API types
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    usage: Option<AnthropicUsage>,
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
    input: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    delta: Option<AnthropicDelta>,
}

#[derive(Debug, Deserialize)]
struct AnthropicDelta {
    #[serde(rename = "type")]
    delta_type: String,
    text: String,
}

// Anthropic Models API response types
#[derive(Debug, Deserialize)]
struct AnthropicModelsResponse {
    data: Vec<AnthropicModel>,
}

#[derive(Debug, Deserialize)]
struct AnthropicModel {
    id: String,
    display_name: String,
    max_input_tokens: u32,
    max_tokens: u32,
    capabilities: Option<AnthropicModelCapabilities>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AnthropicModelCapabilities {
    #[serde(default)]
    batch: Option<AnthropicCapabilityStatus>,
    #[serde(default)]
    citations: Option<AnthropicCapabilityStatus>,
    #[serde(default)]
    code_execution: Option<AnthropicCapabilityStatus>,
    #[serde(default)]
    image_input: Option<AnthropicCapabilityStatus>,
    #[serde(default)]
    pdf_input: Option<AnthropicCapabilityStatus>,
    #[serde(default)]
    structured_outputs: Option<AnthropicCapabilityStatus>,
    #[serde(default)]
    thinking: Option<AnthropicCapabilityStatus>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AnthropicCapabilityStatus {
    supported: bool,
}
