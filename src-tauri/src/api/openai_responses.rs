//! OpenAI Responses API provider (new generation)

use crate::api::provider::*;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use std::pin::Pin;

pub struct OpenAIResponsesProvider {
    client: Client,
    base_url: String,
    api_key: String,
    default_model: String,
}

impl OpenAIResponsesProvider {
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
impl AIProvider for OpenAIResponsesProvider {
    fn name(&self) -> &str {
        "openai_responses"
    }

    fn models(&self) -> Vec<String> {
        vec![
            "gpt-4o".to_string(),
            "gpt-4o-mini".to_string(),
            "o1".to_string(),
            "o1-mini".to_string(),
            "o1-preview".to_string(),
        ]
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let response = self
            .client
            .post(format!("{}/responses", self.base_url))
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

        let body: ResponsesApiResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let content = body
            .output
            .iter()
            .filter_map(|o| {
                if o.output_type == "message" {
                    o.content.first().map(|c| c.text.clone())
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
            finish_reason: body.status,
        })
    }

    async fn chat_structured(
        &self,
        request: ChatRequest,
        schema: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let response = self
            .client
            .post(format!("{}/responses", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&self.build_request_body(&request, Some(schema))?)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error_text));
        }

        let body: ResponsesApiResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let content = body
            .output
            .iter()
            .filter_map(|o| {
                if o.output_type == "message" {
                    o.content.first().map(|c| c.text.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {}", e))
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, String>> + Send>>, String> {
        // Responses API uses SSE streaming
        let mut stream_request = request.clone();
        stream_request.stream = true;

        let response = self
            .client
            .post(format!("{}/responses", self.base_url))
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
                    for line in text.lines() {
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if let Ok(event) = serde_json::from_str::<ResponsesStreamEvent>(data) {
                                if event.event_type == "response.output_text.delta" {
                                    if let Some(delta) = event.delta {
                                        return Ok(StreamChunk {
                                            delta,
                                            finish_reason: None,
                                        });
                                    }
                                } else if event.event_type == "response.done" {
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

impl OpenAIResponsesProvider {
    fn build_request_body(
        &self,
        request: &ChatRequest,
        schema: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let mut body = serde_json::json!({
            "model": self.default_model,
            "input": request.messages.iter().map(|m| {
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

        if let Some(schema) = schema {
            body["text"] = serde_json::json!({
                "format": {
                    "type": "json_schema",
                    "name": "response",
                    "schema": schema,
                    "strict": true
                }
            });
        }

        if let Some(temp) = request.sampling.temperature {
            body["temperature"] = serde_json::json!(temp);
        }
        if let Some(max_tokens) = request.max_tokens {
            body["max_output_tokens"] = serde_json::json!(max_tokens);
        }

        Ok(body)
    }
}

// Responses API types
#[derive(Debug, Deserialize)]
struct ResponsesApiResponse {
    id: String,
    output: Vec<ResponsesOutput>,
    usage: Option<ResponsesUsage>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponsesOutput {
    output_type: String,
    content: Vec<ResponsesContent>,
}

#[derive(Debug, Deserialize)]
struct ResponsesContent {
    text: String,
}

#[derive(Debug, Deserialize)]
struct ResponsesUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ResponsesStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    delta: Option<String>,
}
