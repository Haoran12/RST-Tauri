//! Google Gemini API provider

use crate::api::provider::*;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use std::pin::Pin;

pub struct GeminiProvider {
    client: Client,
    base_url: String,
    api_key: String,
    default_model: String,
}

impl GeminiProvider {
    pub fn new(api_key: String, base_url: Option<String>, default_model: String) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".to_string()),
            api_key,
            default_model,
        }
    }
}

#[async_trait]
impl AIProvider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    fn models(&self) -> Vec<String> {
        vec![
            "gemini-2.5-pro-preview-05-06".to_string(),
            "gemini-2.0-flash".to_string(),
            "gemini-1.5-pro".to_string(),
            "gemini-1.5-flash".to_string(),
        ]
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let url = format!(
            "{}models {}:generateContent?key={}",
            self.base_url, self.default_model, self.api_key
        );

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&self.build_request_body(&request, None)?)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error_text));
        }

        let body: GeminiResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let content = body
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .and_then(|p| p.text.clone())
            .unwrap_or_default();

        Ok(ChatResponse {
            request_id: request.request_id,
            content,
            reasoning: None,
            token_usage: body.usage_metadata.map(|u| TokenUsage {
                prompt_tokens: u.prompt_token_count,
                completion_tokens: u.candidates_token_count,
                total_tokens: u.total_token_count,
            }),
            finish_reason: body
                .candidates
                .first()
                .and_then(|c| c.finish_reason.clone()),
        })
    }

    async fn chat_structured(
        &self,
        request: ChatRequest,
        schema: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let url = format!(
            "{}models {}:generateContent?key={}",
            self.base_url, self.default_model, self.api_key
        );

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&self.build_request_body(&request, Some(schema))?)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error_text));
        }

        let body: GeminiResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let content = body
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .and_then(|p| p.text.clone())
            .unwrap_or_default();

        serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {}", e))
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, String>> + Send>>, String> {
        let url = format!(
            "{}models {}:streamGenerateContent?key={}&alt=sse",
            self.base_url, self.default_model, self.api_key
        );

        let response = self
            .client
            .post(url)
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
                            if let Ok(chunk) = serde_json::from_str::<GeminiResponse>(data) {
                                if let Some(candidate) = chunk.candidates.first() {
                                    if let Some(part) = candidate.content.parts.first() {
                                        if let Some(text) = part.text.clone() {
                                            return Ok(StreamChunk {
                                                delta: text,
                                                finish_reason: candidate.finish_reason.clone(),
                                            });
                                        }
                                    }
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

impl GeminiProvider {
    fn build_request_body(
        &self,
        request: &ChatRequest,
        schema: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let contents: Vec<_> = request
            .messages
            .iter()
            .filter(|m| m.role != ChatRole::System)
            .map(|m| {
                let role = match m.role {
                    ChatRole::User => "user",
                    ChatRole::Assistant => "model",
                    _ => "user",
                };
                let parts: Vec<_> = m.content.iter().map(|c| match c {
                    ContentPart::Text { text } => serde_json::json!({ "text": text }),
                    ContentPart::ImageRef { image_url } => {
                        serde_json::json!({
                            "inline_data": {
                                "mime_type": "image/jpeg",
                                "data": image_url.url
                            }
                        })
                    },
                    _ => serde_json::json!({}),
                }).collect();
                serde_json::json!({
                    "role": role,
                    "parts": parts
                })
            })
            .collect();

        // System instructions
        let system_instruction = request
            .messages
            .iter()
            .filter(|m| m.role == ChatRole::System)
            .filter_map(|m| {
                m.content.iter().filter_map(|c| match c {
                    ContentPart::Text { text } => Some(text.clone()),
                    _ => None,
                }).next()
            })
            .collect::<Vec<_>>()
            .join("\n");

        let mut body = serde_json::json!({
            "contents": contents,
        });

        if !system_instruction.is_empty() {
            body["systemInstruction"] = serde_json::json!({
                "parts": [{ "text": system_instruction }]
            });
        }

        // Generation config
        let mut gen_config = serde_json::json!({});
        if let Some(temp) = request.sampling.temperature {
            gen_config["temperature"] = serde_json::json!(temp);
        }
        if let Some(top_p) = request.sampling.top_p {
            gen_config["topP"] = serde_json::json!(top_p);
        }
        if let Some(top_k) = request.sampling.top_k {
            gen_config["topK"] = serde_json::json!(top_k);
        }
        if let Some(max_tokens) = request.max_tokens {
            gen_config["maxOutputTokens"] = serde_json::json!(max_tokens);
        }
        if !request.stop_sequences.is_empty() {
            gen_config["stopSequences"] = serde_json::json!(request.stop_sequences);
        }

        if let Some(schema) = schema {
            gen_config["responseSchema"] = schema;
            gen_config["responseMimeType"] = serde_json::json!("application/json");
        }

        if !gen_config.is_null() || gen_config.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
            body["generationConfig"] = gen_config;
        }

        Ok(body)
    }
}

// Gemini API types
#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    usage_metadata: Option<GeminiUsage>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Deserialize)]
struct GeminiPart {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiUsage {
    prompt_token_count: u32,
    candidates_token_count: u32,
    total_token_count: u32,
}