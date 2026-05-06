//! OpenAI Responses API provider (new generation)

use crate::api::openai_files::{
    invalidate_openai_file_cache, prepare_request_messages_with_file_cache,
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

pub struct OpenAIResponsesProvider {
    client: Client,
    base_url: String,
    api_key: String,
    default_model: String,
    data_dir: Option<PathBuf>,
}

impl OpenAIResponsesProvider {
    pub fn new(
        api_key: String,
        base_url: Option<String>,
        default_model: String,
        data_dir: Option<PathBuf>,
    ) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
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
    let provider = OpenAIResponsesProvider::new(
        config.api_key.clone().unwrap_or_default(),
        config.base_url.clone(),
        config.model.clone(),
        Some(data_dir.to_path_buf()),
    );
    provider.build_request_body(request, schema).await
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

    async fn list_models(&self) -> Result<Vec<ModelInfo>, String> {
        let response = self
            .client
            .get(format!("{}/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error_text));
        }

        let body: OpenAIModelsResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(body
            .data
            .into_iter()
            .map(|m| ModelInfo {
                id: m.id,
                display_name: None,
                owned_by: Some(m.owned_by),
                max_input_tokens: None,
                max_output_tokens: None,
                capabilities: None,
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

impl OpenAIResponsesProvider {
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
            .post(format!("{}/responses", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
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
        let body: ResponsesApiResponse = serde_json::from_str(&response_text)
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

        let raw_response: Option<serde_json::Value> = match serde_json::from_str(&response_text) {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::error!(
                    "[OpenAI Responses] Failed to parse raw_response: {}, response_text preview: {}",
                    e,
                    &response_text[..response_text.len().min(200)]
                );
                None
            }
        };
        tracing::info!(
            "[OpenAI Responses] raw_response parsed: {}, response_text length: {}",
            raw_response.is_some(),
            response_text.len()
        );

        Ok(ChatResponse {
            request_id: request.request_id.clone(),
            content,
            reasoning: None,
            token_usage: body.usage.map(|u| TokenUsage {
                prompt_tokens: u.input_tokens,
                completion_tokens: u.output_tokens,
                total_tokens: u.input_tokens + u.output_tokens,
            }),
            finish_reason: body.status,
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
        let request_body = self
            .build_request_body(request, Some(schema.clone()))
            .await?;
        let response = self
            .client
            .post(format!("{}/responses", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
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
        let body: ResponsesApiResponse = serde_json::from_str(&response_text)
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
            .post(format!("{}/responses", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
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
                                if let Some(chunk) = parse_responses_stream_data(&data) {
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
                                if let Some(chunk) = parse_responses_stream_data(&data) {
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
        lower.contains("file")
            && (lower.contains("not found")
                || lower.contains("invalid")
                || lower.contains("expired"))
    }

    async fn invalidate_file_caches(&self, request: &ChatRequest) -> Result<(), String> {
        for message in &request.messages {
            for part in &message.content {
                if let ContentPart::FileRef { file } = part {
                    if file.file_id.is_some() || file.file_data.is_some() {
                        invalidate_openai_file_cache(
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
        schema: Option<serde_json::Value>,
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

        let mut body = serde_json::json!({
            "model": self.default_model,
            "input": prepared_messages.iter().map(|m| {
                let role = match m.role {
                    ChatRole::System => "system",
                    ChatRole::Developer => "developer",
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                    ChatRole::Tool => "tool",
                };
                let content: Vec<_> = m.content.iter().filter_map(|c| match c {
                    ContentPart::Text { text } => Some(serde_json::json!({
                        "type": "input_text",
                        "text": text
                    })),
                    ContentPart::ImageRef { image_url } => Some(serde_json::json!({
                        "type": "input_image",
                        "image_url": image_url.url,
                        "detail": "auto"
                    })),
                    ContentPart::FileRef { file } => Some(serde_json::json!({
                        "type": "input_file",
                        "file_id": file.file_id,
                        "file_data": file.file_data,
                        "filename": file.filename
                    })),
                    ContentPart::ToolResult { .. } => None,
                }).collect::<Vec<_>>();
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
        if let Some(top_p) = request.sampling.top_p {
            body["top_p"] = serde_json::json!(top_p);
        }
        if let Some(max_tokens) = request.max_tokens {
            body["max_output_tokens"] = serde_json::json!(max_tokens);
        }
        if request.stream {
            body["stream"] = serde_json::json!(true);
        }

        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_responses_stream_data, OpenAIResponsesProvider};

    #[test]
    fn retry_missing_file_detects_expected_errors() {
        assert!(OpenAIResponsesProvider::should_retry_missing_file(
            "API error: file expired"
        ));
        assert!(OpenAIResponsesProvider::should_retry_missing_file(
            "API error: invalid file handle"
        ));
        assert!(!OpenAIResponsesProvider::should_retry_missing_file(
            "API error: rate limit exceeded"
        ));
    }

    #[test]
    fn parses_responses_stream_delta() {
        let chunk =
            parse_responses_stream_data(r#"{"type":"response.output_text.delta","delta":"hello"}"#)
                .expect("chunk")
                .expect("ok");

        assert_eq!(chunk.delta, "hello");
        assert_eq!(chunk.finish_reason, None);
    }
}

fn parse_responses_stream_data(data: &str) -> Option<Result<StreamChunk, String>> {
    let event = serde_json::from_str::<ResponsesStreamEvent>(data).ok()?;
    if event.event_type == "response.output_text.delta" {
        event.delta.map(|delta| {
            Ok(StreamChunk {
                delta,
                finish_reason: None,
            })
        })
    } else if event.event_type == "response.done" {
        Some(Ok(StreamChunk {
            delta: String::new(),
            finish_reason: Some("stop".to_string()),
        }))
    } else {
        None
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

// OpenAI Models API response types (shared with openai_chat)
#[derive(Debug, Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
}

#[derive(Debug, Deserialize)]
struct OpenAIModel {
    id: String,
    owned_by: String,
}
