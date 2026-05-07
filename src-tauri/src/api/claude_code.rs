//! Claude Code Interface provider

use crate::api::provider::*;
use crate::api::sse::SseDecoder;
use async_trait::async_trait;
use futures::{stream, Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::pin::Pin;

/// Claude Code Interface - Anthropic Messages-compatible surface used by Claude Code.
pub struct ClaudeCodeProvider {
    client: Client,
    base_url: String,
    api_key: String,
    default_model: String,
}

impl ClaudeCodeProvider {
    pub fn new(base_url: String, api_key: String, default_model: String) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            default_model,
        }
    }

    fn messages_url(&self) -> String {
        if self.base_url.ends_with("/v1") {
            format!("{}/messages", self.base_url)
        } else {
            format!("{}/v1/messages", self.base_url)
        }
    }
}

pub fn build_request_body_preview(
    config: &crate::storage::st_resources::ApiConfig,
    request: &ChatRequest,
    schema: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let provider = ClaudeCodeProvider::new(
        config
            .base_url
            .clone()
            .unwrap_or_else(|| "http://localhost:8080".to_string()),
        config.api_key.clone().unwrap_or_default(),
        config.model.clone(),
    );
    provider.build_request_body(request, schema)
}

#[async_trait]
impl AIProvider for ClaudeCodeProvider {
    fn name(&self) -> &str {
        "claude_code"
    }

    fn models(&self) -> Vec<String> {
        vec![
            "claude-opus-4-20250514".to_string(),
            "claude-sonnet-4-20250514".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
        ]
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, String> {
        Ok(self
            .models()
            .into_iter()
            .map(|id| ModelInfo {
                id: id.clone(),
                display_name: Some(id),
                owned_by: Some("claude_code".to_string()),
                max_input_tokens: None,
                max_output_tokens: None,
                capabilities: None,
            })
            .collect())
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let request_body = self.build_request_body(&request, None)?;
        let response = self
            .client
            .post(self.messages_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
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

        let response_text = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;
        let body: ClaudeCodeResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let raw_response: Option<serde_json::Value> = match serde_json::from_str(&response_text) {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::error!(
                    "[Claude Code] Failed to parse raw_response: {}, response_text preview: {}",
                    e,
                    &response_text[..response_text.len().min(200)]
                );
                None
            }
        };
        tracing::info!(
            "[Claude Code] raw_response parsed: {}, response_text length: {}",
            raw_response.is_some(),
            response_text.len()
        );

        Ok(ChatResponse {
            request_id: request.request_id,
            content: body.text_content(),
            reasoning: body.reasoning_content(),
            token_usage: body.usage.map(|u| TokenUsage {
                prompt_tokens: u.input_tokens,
                completion_tokens: u.output_tokens,
                total_tokens: u.input_tokens + u.output_tokens,
            }),
            finish_reason: body.stop_reason,
            raw_response,
        })
    }

    async fn chat_structured(
        &self,
        request: ChatRequest,
        schema: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let request_body = self.build_request_body(&request, Some(schema))?;
        let response = self
            .client
            .post(self.messages_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
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

        let response_text = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;
        let body: ClaudeCodeResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(input) = body.tool_input("structured_output") {
            return Ok(input);
        }

        serde_json::from_str(&body.text_content())
            .map_err(|e| format!("Failed to parse JSON: {}", e))
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, String>> + Send>>, String> {
        let response = self
            .client
            .post(self.messages_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&{
                let mut body = self.build_request_body(&request, None)?;
                body["stream"] = serde_json::Value::Bool(true);
                body
            })
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error_text));
        }

        let byte_stream = response.bytes_stream();
        let stream = stream::unfold(
            (
                byte_stream,
                SseDecoder::default(),
                VecDeque::<Result<StreamChunk, String>>::new(),
            ),
            |(mut byte_stream, mut decoder, mut pending)| async move {
                loop {
                    if let Some(item) = pending.pop_front() {
                        return Some((item, (byte_stream, decoder, pending)));
                    }

                    match byte_stream.next().await {
                        Some(Ok(bytes)) => {
                            let text = String::from_utf8_lossy(&bytes);
                            for data in decoder.push_str(&text) {
                                if let Some(chunk) = parse_claude_code_stream_data(&data) {
                                    pending.push_back(chunk);
                                }
                            }
                        }
                        Some(Err(e)) => {
                            return Some((
                                Err(format!("Stream error: {}", e)),
                                (byte_stream, decoder, pending),
                            ));
                        }
                        None => {
                            for data in decoder.finish() {
                                if let Some(chunk) = parse_claude_code_stream_data(&data) {
                                    pending.push_back(chunk);
                                }
                            }
                            if let Some(item) = pending.pop_front() {
                                return Some((item, (byte_stream, decoder, pending)));
                            }
                            return None;
                        }
                    }
                }
            },
        );

        Ok(Box::pin(stream))
    }
}

impl ClaudeCodeProvider {
    fn build_request_body(
        &self,
        request: &ChatRequest,
        schema: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let mut system_blocks = Vec::new();
        let mut messages = Vec::new();

        for message in &request.messages {
            match message.role {
                ChatRole::System | ChatRole::Developer => {
                    for part in &message.content {
                        if let ContentPart::Text { text } = part {
                            if !text.is_empty() {
                                system_blocks.push(serde_json::json!({
                                    "type": "text",
                                    "text": text
                                }));
                            }
                        }
                    }
                }
                ChatRole::User | ChatRole::Assistant | ChatRole::Tool => {
                    let role = match message.role {
                        ChatRole::Assistant => "assistant",
                        _ => "user",
                    };
                    messages.push(serde_json::json!({
                        "role": role,
                        "content": self.message_content_blocks(&message.content)
                    }));
                }
            }
        }

        let mut body = serde_json::json!({
            "model": self.default_model,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "system": system_blocks,
            "messages": messages,
            "metadata": {
                "user_id": format!(
                    "{{\"device_id\":\"rst-local\",\"account_uuid\":\"\",\"session_id\":\"{}\"}}",
                    request.request_id
                )
            }
        });

        if let Some(temp) = request.sampling.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        if let Some(schema) = schema {
            body["tools"] = serde_json::json!([{
                "name": "structured_output",
                "description": "Return the requested JSON object.",
                "input_schema": schema
            }]);
            body["tool_choice"] = serde_json::json!({
                "type": "tool",
                "name": "structured_output"
            });
        }

        Ok(body)
    }

    fn message_content_blocks(&self, parts: &[ContentPart]) -> serde_json::Value {
        let blocks: Vec<_> = parts
            .iter()
            .map(|part| match part {
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
                ContentPart::FileRef { file } => serde_json::json!({
                    "type": "document",
                    "source": {
                        "type": "base64",
                        "media_type": file.mime_type.clone().unwrap_or_else(|| "application/pdf".to_string()),
                        "data": file.file_data.clone().unwrap_or_default()
                    }
                }),
                ContentPart::ToolResult {
                    tool_call_id,
                    content,
                } => serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": tool_call_id,
                    "content": content
                }),
            })
            .collect();

        serde_json::Value::Array(blocks)
    }
}

fn image_mime_type_from_data_url(data_url: &str) -> String {
    data_url
        .strip_prefix("data:")
        .and_then(|rest| rest.split_once(';'))
        .map(|(mime, _)| mime.to_string())
        .unwrap_or_else(|| "image/png".to_string())
}

fn image_base64_from_data_url(data_url: &str) -> String {
    data_url
        .split_once(',')
        .map(|(_, data)| data.to_string())
        .unwrap_or_else(|| data_url.to_string())
}

// Claude Code Interface response types follow Anthropic Messages.
#[derive(Debug, Deserialize, Serialize)]
struct ClaudeCodeResponse {
    content: Vec<ClaudeCodeContentBlock>,
    usage: Option<ClaudeCodeUsage>,
    stop_reason: Option<String>,
}

impl ClaudeCodeResponse {
    fn text_content(&self) -> String {
        self.content
            .iter()
            .filter_map(|block| match block {
                ClaudeCodeContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    fn reasoning_content(&self) -> Option<String> {
        let reasoning = self
            .content
            .iter()
            .filter_map(|block| match block {
                ClaudeCodeContentBlock::Thinking { thinking } => Some(thinking.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");
        if reasoning.is_empty() {
            None
        } else {
            Some(reasoning)
        }
    }

    fn tool_input(&self, tool_name: &str) -> Option<serde_json::Value> {
        self.content.iter().find_map(|block| match block {
            ClaudeCodeContentBlock::ToolUse { name, input, .. } if name == tool_name => {
                Some(input.clone())
            }
            _ => None,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
enum ClaudeCodeContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    #[serde(rename = "redacted_thinking")]
    RedactedThinking {},
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize, Serialize)]
struct ClaudeCodeUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClaudeCodeStreamEvent {
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { delta: ClaudeCodeStreamDelta },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: ClaudeCodeMessageDelta },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct ClaudeCodeStreamDelta {
    #[serde(rename = "type")]
    _delta_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeCodeMessageDelta {
    stop_reason: Option<String>,
}

fn parse_claude_code_stream_data(data: &str) -> Option<Result<StreamChunk, String>> {
    let event = serde_json::from_str::<ClaudeCodeStreamEvent>(data).ok()?;
    match event {
        ClaudeCodeStreamEvent::ContentBlockDelta { delta, .. } => delta.text.map(|text| {
            Ok(StreamChunk {
                delta: text,
                finish_reason: None,
                raw_sse_data: Some(data.to_string()),
            })
        }),
        ClaudeCodeStreamEvent::MessageDelta { delta, .. } => Some(Ok(StreamChunk {
            delta: String::new(),
            finish_reason: delta.stop_reason,
            raw_sse_data: Some(data.to_string()),
        })),
        _ => {
            // For other event types, still return raw data for logging
            Some(Ok(StreamChunk {
                delta: String::new(),
                finish_reason: None,
                raw_sse_data: Some(data.to_string()),
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider() -> ClaudeCodeProvider {
        ClaudeCodeProvider::new(
            "https://maas-coding-api.cn-huabei-1.xf-yun.com/anthropic".to_string(),
            "secret".to_string(),
            "astron-code-latest".to_string(),
        )
    }

    fn request() -> ChatRequest {
        ChatRequest {
            request_id: "req-1".to_string(),
            api_config_id: "api-1".to_string(),
            messages: vec![
                ChatMessage::system("system prompt"),
                ChatMessage::user("hello"),
                ChatMessage::assistant("hi"),
            ],
            sampling: SamplingParams {
                temperature: Some(1.0),
                top_p: Some(1.0),
                ..Default::default()
            },
            stop_sequences: vec![],
            max_tokens: Some(128),
            stream: false,
            reasoning: None,
            response_format: None,
            provider_overrides: serde_json::Value::Null,
        }
    }

    #[test]
    fn claude_code_uses_anthropic_messages_shape() {
        let body = provider().build_request_body(&request(), None).unwrap();

        assert_eq!(body["model"], "astron-code-latest");
        assert_eq!(body["max_tokens"], 128);
        assert_eq!(
            body["system"],
            serde_json::json!([{ "type": "text", "text": "system prompt" }])
        );
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"][0]["type"], "text");
        assert_eq!(body["messages"][1]["role"], "assistant");
        assert!(body.get("top_p").is_none());
        assert!(body.get("stop_sequences").is_none());
        assert!(body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .all(|message| message["role"] != "system"));
    }

    #[test]
    fn claude_code_structured_output_uses_tool_schema() {
        let body = provider()
            .build_request_body(
                &request(),
                Some(serde_json::json!({
                    "type": "object",
                    "properties": { "answer": { "type": "string" } },
                    "required": ["answer"]
                })),
            )
            .unwrap();

        assert!(body.get("response_format").is_none());
        assert_eq!(body["tools"][0]["name"], "structured_output");
        assert_eq!(
            body["tool_choice"],
            serde_json::json!({
                "type": "tool",
                "name": "structured_output"
            })
        );
    }

    #[test]
    fn claude_code_appends_v1_messages_to_anthropic_client_base_url() {
        assert_eq!(
            provider().messages_url(),
            "https://maas-coding-api.cn-huabei-1.xf-yun.com/anthropic/v1/messages"
        );
        let with_v1 = ClaudeCodeProvider::new(
            "https://example.test/anthropic/v1".to_string(),
            "secret".to_string(),
            "model".to_string(),
        );
        assert_eq!(
            with_v1.messages_url(),
            "https://example.test/anthropic/v1/messages"
        );
    }

    #[test]
    fn parses_claude_code_text_delta() {
        let chunk = parse_claude_code_stream_data(
            r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"hello"}}"#,
        )
        .expect("chunk")
        .expect("ok");

        assert_eq!(chunk.delta, "hello");
        assert_eq!(chunk.finish_reason, None);
    }
}
