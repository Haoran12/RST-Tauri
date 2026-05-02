//! OpenAI Chat Completions API provider

use crate::api::provider::*;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use std::pin::Pin;

pub struct OpenAIChatProvider {
    client: Client,
    base_url: String,
    api_key: String,
    default_model: String,
}

impl OpenAIChatProvider {
    pub fn new(api_key: String, base_url: Option<String>, default_model: String) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            api_key,
            default_model,
        }
    }
}

#[async_trait]
impl AIProvider for OpenAIChatProvider {
    fn name(&self) -> &str {
        "openai_chat"
    }

    fn models(&self) -> Vec<String> {
        vec![
            "gpt-4o".to_string(),
            "gpt-4o-mini".to_string(),
            "gpt-4-turbo".to_string(),
            "gpt-4".to_string(),
            "gpt-3.5-turbo".to_string(),
        ]
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&self.build_request_body(&request, None)?)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error_text));
        }

        let body: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let content = body
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(ChatResponse {
            request_id: request.request_id,
            content,
            reasoning: None,
            token_usage: body.usage.map(|u| TokenUsage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
            finish_reason: body.choices.first().and_then(|c| c.finish_reason.clone()),
        })
    }

    async fn chat_structured(
        &self,
        request: ChatRequest,
        schema: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&self.build_request_body(&request, Some(schema.clone()))?)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error_text));
        }

        let body: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let content = body
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        // Parse JSON from content
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {}", e))
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, String>> + Send>>, String> {
        let mut stream_request = request.clone();
        stream_request.stream = true;

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&self.build_request_body(&stream_request, None)?)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let stream = response.bytes_stream().map(move |result| {
            match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    // Parse SSE format
                    for line in text.lines() {
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if data == "[DONE]" {
                                return Ok(StreamChunk {
                                    delta: String::new(),
                                    finish_reason: Some("stop".to_string()),
                                });
                            }
                            if let Ok(chunk) = serde_json::from_str::<OpenAIStreamChunk>(data) {
                                if let Some(choice) = chunk.choices.first() {
                                    let delta = choice.delta.content.clone().unwrap_or_default();
                                    return Ok(StreamChunk {
                                        delta,
                                        finish_reason: choice.finish_reason.clone(),
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

impl OpenAIChatProvider {
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
                let content: Vec<_> = m.content.iter().map(|c| match c {
                    ContentPart::Text { text } => serde_json::json!({
                        "type": "text",
                        "text": text
                    }),
                    ContentPart::ImageRef { image_url } => serde_json::json!({
                        "type": "image_url",
                        "image_url": { "url": image_url.url }
                    }),
                    ContentPart::ToolResult { tool_call_id, content } => serde_json::json!({
                        "type": "tool_result",
                        "tool_call_id": tool_call_id,
                        "content": content
                    }),
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
        if let Some(top_p) = request.sampling.top_p {
            body["top_p"] = serde_json::json!(top_p);
        }
        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }
        if !request.stop_sequences.is_empty() {
            body["stop"] = serde_json::json!(request.stop_sequences);
        }

        if let Some(schema) = schema {
            body["response_format"] = serde_json::json!({
                "type": "json_schema",
                "json_schema": {
                    "name": "response",
                    "strict": true,
                    "schema": schema
                }
            });
        }

        Ok(body)
    }
}

// OpenAI API response types
#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChunk {
    choices: Vec<OpenAIStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIDelta {
    content: Option<String>,
}
