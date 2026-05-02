//! Claude Code Interface provider

use crate::api::provider::*;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use std::pin::Pin;

/// Claude Code Interface - for local Claude Code compatible endpoints
pub struct ClaudeCodeProvider {
    client: Client,
    base_url: String,
    default_model: String,
}

impl ClaudeCodeProvider {
    pub fn new(base_url: String, default_model: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            default_model,
        }
    }
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

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let response = self
            .client
            .post(format!("{}/chat", self.base_url))
            .header("Content-Type", "application/json")
            .json(&self.build_request_body(&request, None)?)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error_text));
        }

        let body: ClaudeCodeResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(ChatResponse {
            request_id: request.request_id,
            content: body.content,
            reasoning: body.reasoning,
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
        let response = self
            .client
            .post(format!("{}/chat", self.base_url))
            .header("Content-Type", "application/json")
            .json(&self.build_request_body(&request, Some(schema))?)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error_text));
        }

        let body: ClaudeCodeResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // Parse JSON from content
        serde_json::from_str(&body.content).map_err(|e| format!("Failed to parse JSON: {}", e))
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, String>> + Send>>, String> {
        let response = self
            .client
            .post(format!("{}/chat/stream", self.base_url))
            .header("Content-Type", "application/json")
            .json(&self.build_request_body(&request, None)?)
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
                            if let Ok(chunk) = serde_json::from_str::<ClaudeCodeStreamChunk>(data) {
                                return Ok(StreamChunk {
                                    delta: chunk.delta,
                                    finish_reason: chunk.finish_reason,
                                });
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

impl ClaudeCodeProvider {
    fn build_request_body(
        &self,
        request: &ChatRequest,
        schema: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let mut body = serde_json::json!({
            "model": self.default_model,
            "messages": request.messages.iter().map(|m| {
                let role = match m.role {
                    ChatRole::System => "system",
                    ChatRole::Developer => "developer",
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                    ChatRole::Tool => "tool",
                };
                let content: String = m.content.iter().filter_map(|c| match c {
                    ContentPart::Text { text } => Some(text.clone()),
                    _ => None,
                }).collect();
                serde_json::json!({
                    "role": role,
                    "content": content
                })
            }).collect::<Vec<_>>(),
        });

        if let Some(temp) = request.sampling.temperature {
            body["temperature"] = serde_json::json!(temp);
        }
        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }

        if let Some(schema) = schema {
            body["response_format"] = serde_json::json!({
                "type": "json_schema",
                "schema": schema
            });
        }

        Ok(body)
    }
}

// Claude Code API types
#[derive(Debug, Deserialize)]
struct ClaudeCodeResponse {
    content: String,
    reasoning: Option<String>,
    usage: Option<ClaudeCodeUsage>,
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeCodeUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ClaudeCodeStreamChunk {
    delta: String,
    finish_reason: Option<String>,
}
