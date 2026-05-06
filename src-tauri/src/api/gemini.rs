//! Google Gemini API provider

use crate::api::gemini_files::{
    invalidate_gemini_file_cache, prepare_request_messages_with_file_cache,
};
use crate::api::provider::*;
use crate::api::sse::SseDecoder;
use async_trait::async_trait;
use futures::{stream, Stream, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::pin::Pin;

pub struct GeminiProvider {
    client: Client,
    base_url: String,
    api_key: String,
    default_model: String,
    data_dir: Option<PathBuf>,
}

impl GeminiProvider {
    pub fn new(
        api_key: String,
        base_url: Option<String>,
        default_model: String,
        data_dir: Option<PathBuf>,
    ) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url
                .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".to_string()),
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
    let provider = GeminiProvider::new(
        config.api_key.clone().unwrap_or_default(),
        config.base_url.clone(),
        config.model.clone(),
        Some(data_dir.to_path_buf()),
    );
    provider.build_request_body(request, schema).await
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

    async fn list_models(&self) -> Result<Vec<ModelInfo>, String> {
        let url = format!(
            "{}/models?key={}",
            self.base_url.trim_end_matches('/'),
            self.api_key
        );

        let response = self
            .client
            .get(url)
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error_text));
        }

        let body: GeminiModelsResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(body
            .models
            .into_iter()
            .map(|m| {
                let model_id = m
                    .name
                    .strip_prefix("models/")
                    .unwrap_or(&m.name)
                    .to_string();
                ModelInfo {
                    id: model_id,
                    display_name: Some(m.display_name),
                    owned_by: Some("google".to_string()),
                    max_input_tokens: Some(m.input_token_limit),
                    max_output_tokens: Some(m.output_token_limit),
                    capabilities: None,
                }
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

impl GeminiProvider {
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
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.base_url.trim_end_matches('/'),
            self.default_model,
            self.api_key
        );
        let request_body = self.build_request_body(request, None).await?;

        let response = self
            .client
            .post(url)
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
        let body: GeminiResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let content = body
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .and_then(|p| p.text.clone())
            .unwrap_or_default();

        let raw_response: Option<serde_json::Value> = match serde_json::from_str(&response_text) {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::error!(
                    "[Gemini] Failed to parse raw_response: {}, response_text preview: {}",
                    e,
                    &response_text[..response_text.len().min(200)]
                );
                None
            }
        };
        tracing::info!(
            "[Gemini] raw_response parsed: {}, response_text length: {}",
            raw_response.is_some(),
            response_text.len()
        );

        Ok(ChatResponse {
            request_id: request.request_id.clone(),
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
            raw_response,
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
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.base_url.trim_end_matches('/'),
            self.default_model,
            self.api_key
        );
        let request_body = self
            .build_request_body(request, Some(schema.clone()))
            .await?;

        let response = self
            .client
            .post(url)
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
        let body: GeminiResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let content = body
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .and_then(|p| p.text.clone())
            .unwrap_or_default();

        serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {}", e))
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
        let url = format!(
            "{}/models/{}:streamGenerateContent?key={}&alt=sse",
            self.base_url.trim_end_matches('/'),
            self.default_model,
            self.api_key
        );
        let request_body = self.build_request_body(request, None).await?;

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&request_body)
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
                                if let Some(chunk) = parse_gemini_stream_data(&data) {
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
                                if let Some(chunk) = parse_gemini_stream_data(&data) {
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

    fn should_retry_missing_file(error: &str) -> bool {
        let lower = error.to_ascii_lowercase();
        (lower.contains("file") || lower.contains("uri"))
            && (lower.contains("not found")
                || lower.contains("invalid")
                || lower.contains("expired"))
    }

    async fn invalidate_file_caches(&self, request: &ChatRequest) -> Result<(), String> {
        let upload_base_url = format!("{}/upload/v1beta", self.base_url.trim_end_matches('/'));
        for message in &request.messages {
            for part in &message.content {
                if let ContentPart::FileRef { file } = part {
                    if file.file_uri.is_some() || file.file_data.is_some() {
                        invalidate_gemini_file_cache(
                            self.data_dir.as_deref(),
                            &upload_base_url,
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
        schema: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let prepared_messages = prepare_request_messages_with_file_cache(
            &self.client,
            self.data_dir.as_deref(),
            &format!("{}/upload/v1beta", self.base_url.trim_end_matches('/')),
            &self.api_key,
            &request.api_config_id,
            &request.messages,
        )
        .await?;
        let contents: Vec<_> = prepared_messages
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
                                "mime_type": image_mime_type_from_data_url(&image_url.url),
                                "data": image_base64_from_data_url(&image_url.url)
                            }
                        })
                    },
                    ContentPart::FileRef { file } => {
                        if let Some(file_uri) = &file.file_uri {
                            serde_json::json!({
                                "file_data": {
                                    "mime_type": file.mime_type.clone().unwrap_or_else(|| "application/pdf".to_string()),
                                    "file_uri": file_uri
                                }
                            })
                        } else {
                            serde_json::json!({
                                "inline_data": {
                                    "mime_type": file.mime_type.clone().unwrap_or_else(|| "application/pdf".to_string()),
                                    "data": file.file_data.clone().unwrap_or_default()
                                }
                            })
                        }
                    }
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

        if !gen_config.is_null()
            || gen_config
                .as_object()
                .map(|o| !o.is_empty())
                .unwrap_or(false)
        {
            body["generationConfig"] = gen_config;
        }

        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_gemini_stream_data, GeminiProvider};

    #[test]
    fn retry_missing_file_detects_expected_errors() {
        assert!(GeminiProvider::should_retry_missing_file(
            "API error: file uri not found"
        ));
        assert!(GeminiProvider::should_retry_missing_file(
            "API error: invalid file handle"
        ));
        assert!(!GeminiProvider::should_retry_missing_file(
            "API error: candidate blocked"
        ));
    }

    #[test]
    fn parses_gemini_stream_delta() {
        let chunk = parse_gemini_stream_data(
            r#"{"candidates":[{"content":{"parts":[{"text":"hello"}]},"finish_reason":null}]}"#,
        )
        .expect("chunk")
        .expect("ok");

        assert_eq!(chunk.delta, "hello");
        assert_eq!(chunk.finish_reason, None);
    }
}

fn parse_gemini_stream_data(data: &str) -> Option<Result<StreamChunk, String>> {
    let chunk = serde_json::from_str::<GeminiResponse>(data).ok()?;
    let candidate = chunk.candidates.first()?;
    let part = candidate.content.parts.first()?;
    part.text.clone().map(|delta| {
        Ok(StreamChunk {
            delta,
            finish_reason: candidate.finish_reason.clone(),
        })
    })
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

// Gemini Models API response types
#[derive(Debug, Deserialize)]
struct GeminiModelsResponse {
    models: Vec<GeminiModel>,
}

#[derive(Debug, Deserialize)]
struct GeminiModel {
    name: String,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    input_token_limit: u32,
    #[serde(default)]
    output_token_limit: u32,
}
