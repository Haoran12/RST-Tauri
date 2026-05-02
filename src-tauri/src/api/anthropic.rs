//! Anthropic Messages API provider

use crate::api::provider::*;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use std::pin::Pin;

pub struct AnthropicProvider {
    client: Client,
    base_url: String,
    api_key: String,
    default_model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, base_url: Option<String>, default_model: String) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| "https://api.anthropic.com/v1".to_string()),
            api_key,
            default_model,
        }
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

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let response = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&self.build_request_body(&request, None)?)
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
            request_id: request.request_id,
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

    async fn chat_structured(
        &self,
        request: ChatRequest,
        schema: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        // Anthropic uses tool use for structured output
        let response = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&self.build_structured_request(&request, schema)?)
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

        // Extract tool use result
        let tool_result = body
            .content
            .iter()
            .find(|c| c.content_type == "tool_use")
            .and_then(|c| c.input.clone());

        tool_result.ok_or_else(|| "No tool use in response".to_string())
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, String>> + Send>>, String> {
        let mut stream_request = request.clone();
        stream_request.stream = true;

        let response = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&self.build_request_body(&stream_request, None)?)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let stream = response.bytes_stream().map(move |result| {
            match result {
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
            }
        });

        Ok(Box::pin(stream))
    }
}

impl AnthropicProvider {
    fn build_request_body(
        &self,
        request: &ChatRequest,
        _schema: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        // Anthropic separates system from messages
        let system_messages: Vec<_> = request
            .messages
            .iter()
            .filter(|m| m.role == ChatRole::System)
            .collect();
        let other_messages: Vec<_> = request
            .messages
            .iter()
            .filter(|m| m.role != ChatRole::System)
            .collect();

        let system_text = system_messages
            .iter()
            .filter_map(|m| {
                m.content.iter().filter_map(|c| match c {
                    ContentPart::Text { text } => Some(text.clone()),
                    _ => None,
                }).next()
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
                        "type": "url",
                        "url": image_url.url
                    }
                }),
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

    fn build_structured_request(
        &self,
        request: &ChatRequest,
        schema: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut body = self.build_request_body(request, None)?;
        body["tools"] = serde_json::json!([{
            "name": "structured_output",
            "description": "Return structured JSON output",
            "input_schema": schema
        }]);
        Ok(body)
    }
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