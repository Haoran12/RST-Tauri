//! OpenAI Chat Completions API provider

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

pub struct OpenAIChatProvider {
    client: Client,
    base_url: String,
    api_key: String,
    default_model: String,
    data_dir: Option<PathBuf>,
}

impl OpenAIChatProvider {
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
    let provider = OpenAIChatProvider::new(
        config.api_key.clone().unwrap_or_default(),
        config.base_url.clone(),
        config.model.clone(),
        Some(data_dir.to_path_buf()),
    );
    provider.build_request_body(request, schema).await
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

impl OpenAIChatProvider {
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
            .post(format!("{}/chat/completions", self.base_url))
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
            request_id: request.request_id.clone(),
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
            .post(format!("{}/chat/completions", self.base_url))
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

        let body: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let content = body
            .choices
            .first()
            .map(|c| c.message.content.clone())
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
        let mut stream_request = request.clone();
        stream_request.stream = true;
        let request_body = self.build_request_body(&stream_request, None).await?;

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
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
                                if let Some(chunk) = parse_openai_stream_data(&data) {
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
                                if let Some(chunk) = parse_openai_stream_data(&data) {
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
            "messages": prepared_messages.iter().map(|m| {
                let role = match m.role {
                    ChatRole::System => "system",
                    ChatRole::Developer => "developer",
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                    ChatRole::Tool => "tool",
                };
                serde_json::json!({
                    "role": role,
                    "content": openai_chat_message_content(&m.content)
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
        if request.stream {
            body["stream"] = serde_json::json!(true);
        }
        if let Some(frequency_penalty) = request.sampling.frequency_penalty {
            body["frequency_penalty"] = serde_json::json!(frequency_penalty);
        }
        if let Some(presence_penalty) = request.sampling.presence_penalty {
            body["presence_penalty"] = serde_json::json!(presence_penalty);
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

fn openai_chat_message_content(content: &[ContentPart]) -> serde_json::Value {
    if content
        .iter()
        .all(|part| matches!(part, ContentPart::Text { .. }))
    {
        let text = content
            .iter()
            .map(|part| match part {
                ContentPart::Text { text } => text.as_str(),
                _ => "",
            })
            .collect::<Vec<_>>()
            .join("\n");
        return serde_json::Value::String(text);
    }

    serde_json::Value::Array(
        content
            .iter()
            .map(|part| match part {
                ContentPart::Text { text } => serde_json::json!({
                    "type": "text",
                    "text": text
                }),
                ContentPart::ImageRef { image_url } => serde_json::json!({
                    "type": "image_url",
                    "image_url": { "url": image_url.url }
                }),
                ContentPart::FileRef { file } => serde_json::json!({
                    "type": "file",
                    "file": {
                        "file_id": file.file_id,
                        "file_data": file.file_data,
                        "filename": file.filename
                    }
                }),
                ContentPart::ToolResult {
                    tool_call_id,
                    content,
                } => serde_json::json!({
                    "type": "tool_result",
                    "tool_call_id": tool_call_id,
                    "content": content
                }),
            })
            .collect(),
    )
}

fn parse_openai_stream_data(data: &str) -> Option<Result<StreamChunk, String>> {
    if data == "[DONE]" {
        return Some(Ok(StreamChunk {
            delta: String::new(),
            finish_reason: Some("stop".to_string()),
        }));
    }

    let chunk = serde_json::from_str::<OpenAIStreamChunk>(data).ok()?;
    let choice = chunk.choices.first()?;
    Some(Ok(StreamChunk {
        delta: choice.delta.content.clone().unwrap_or_default(),
        finish_reason: choice.finish_reason.clone(),
    }))
}

#[cfg(test)]
mod tests {
    use super::{openai_chat_message_content, parse_openai_stream_data, OpenAIChatProvider};
    use crate::api::provider::{ContentPart, ImageUrl};
    use serde_json::json;

    #[test]
    fn retry_missing_file_detects_expected_errors() {
        assert!(OpenAIChatProvider::should_retry_missing_file(
            "API error: file not found"
        ));
        assert!(OpenAIChatProvider::should_retry_missing_file(
            "API error: invalid file reference"
        ));
        assert!(!OpenAIChatProvider::should_retry_missing_file(
            "API error: context length exceeded"
        ));
    }

    #[test]
    fn pure_text_messages_use_string_content_for_compatible_chat_apis() {
        let content = openai_chat_message_content(&[
            ContentPart::Text {
                text: "hello".to_string(),
            },
            ContentPart::Text {
                text: "world".to_string(),
            },
        ]);

        assert_eq!(content, json!("hello\nworld"));
    }

    #[test]
    fn multimodal_messages_keep_content_parts() {
        let content = openai_chat_message_content(&[
            ContentPart::Text {
                text: "look".to_string(),
            },
            ContentPart::ImageRef {
                image_url: ImageUrl {
                    url: "data:image/png;base64,abc".to_string(),
                },
            },
        ]);

        assert!(content.is_array());
    }

    #[test]
    fn parses_openai_stream_delta() {
        let chunk = parse_openai_stream_data(
            r#"{"choices":[{"delta":{"content":"hello"},"finish_reason":null}]}"#,
        )
        .expect("chunk")
        .expect("ok");

        assert_eq!(chunk.delta, "hello");
        assert_eq!(chunk.finish_reason, None);
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

// OpenAI Models API response types
#[derive(Debug, Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
}

#[derive(Debug, Deserialize)]
struct OpenAIModel {
    id: String,
    owned_by: String,
}
