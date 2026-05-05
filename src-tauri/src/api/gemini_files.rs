//! Shared Gemini Files API upload/cache helpers.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::{Arc, OnceLock};

use base64::Engine;
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::api::provider::{ChatMessage, ContentPart, FileRef};
use crate::storage::attachment_upload_cache::{
    delete_remote_handle, find_remote_handle, upsert_remote_handle,
};

type FileCacheMap = HashMap<String, String>;

fn file_cache() -> &'static Arc<RwLock<FileCacheMap>> {
    static CACHE: OnceLock<Arc<RwLock<FileCacheMap>>> = OnceLock::new();
    CACHE.get_or_init(|| Arc::new(RwLock::new(HashMap::new())))
}

pub async fn invalidate_gemini_file_cache(
    data_dir: Option<&Path>,
    base_url: &str,
    api_key: &str,
    api_config_id: &str,
    file: &FileRef,
) -> Result<(), String> {
    let cache_key = gemini_file_cache_key(base_url, api_key, file);
    file_cache().write().await.remove(&cache_key);

    if let (Some(data_dir), Some(attachment_id)) = (data_dir, file.attachment_id.as_deref()) {
        delete_remote_handle(
            data_dir,
            attachment_id,
            api_config_id,
            "gemini",
            base_url,
            api_key,
            "file_uri",
        )?;
    }

    Ok(())
}

pub async fn prepare_request_messages_with_file_cache(
    client: &Client,
    data_dir: Option<&Path>,
    base_url: &str,
    api_key: &str,
    api_config_id: &str,
    messages: &[ChatMessage],
) -> Result<Vec<ChatMessage>, String> {
    let mut prepared = Vec::with_capacity(messages.len());

    for message in messages {
        let mut content = Vec::with_capacity(message.content.len());
        for part in &message.content {
            match part {
                ContentPart::FileRef { file } if should_upload_as_gemini_file(file) => {
                    let cache_key = gemini_file_cache_key(base_url, api_key, file);
                    let file_uri = get_or_upload_gemini_file(
                        client,
                        data_dir,
                        base_url,
                        api_key,
                        api_config_id,
                        &cache_key,
                        file,
                    )
                    .await?;
                    content.push(ContentPart::FileRef {
                        file: FileRef {
                            attachment_id: file.attachment_id.clone(),
                            file_id: None,
                            file_uri: Some(file_uri),
                            file_data: None,
                            filename: file.filename.clone(),
                            mime_type: file.mime_type.clone(),
                        },
                    });
                }
                _ => content.push(part.clone()),
            }
        }

        prepared.push(ChatMessage {
            role: message.role.clone(),
            content,
            name: message.name.clone(),
        });
    }

    Ok(prepared)
}

fn should_upload_as_gemini_file(file: &FileRef) -> bool {
    file.file_uri.is_none()
        && file.file_data.is_some()
        && matches!(file.mime_type.as_deref(), Some("application/pdf"))
}

fn gemini_file_cache_key(base_url: &str, api_key: &str, file: &FileRef) -> String {
    let mut hasher = DefaultHasher::new();
    base_url.hash(&mut hasher);
    api_key.hash(&mut hasher);
    file.filename.hash(&mut hasher);
    file.mime_type.hash(&mut hasher);
    file.file_data.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

async fn get_or_upload_gemini_file(
    client: &Client,
    data_dir: Option<&Path>,
    base_url: &str,
    api_key: &str,
    api_config_id: &str,
    cache_key: &str,
    file: &FileRef,
) -> Result<String, String> {
    if let (Some(data_dir), Some(attachment_id)) = (data_dir, file.attachment_id.as_deref()) {
        if let Some(existing) = find_remote_handle(
            data_dir,
            attachment_id,
            api_config_id,
            "gemini",
            base_url,
            api_key,
            "file_uri",
        )? {
            file_cache()
                .write()
                .await
                .insert(cache_key.to_string(), existing.clone());
            return Ok(existing);
        }
    }

    if let Some(existing) = file_cache().read().await.get(cache_key).cloned() {
        return Ok(existing);
    }

    let file_data = file
        .file_data
        .as_deref()
        .ok_or_else(|| "Missing file_data for Gemini file upload".to_string())?;
    let filename = file
        .filename
        .clone()
        .unwrap_or_else(|| "attachment.pdf".to_string());
    let mime_type = file
        .mime_type
        .clone()
        .unwrap_or_else(|| "application/pdf".to_string());

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(file_data)
        .map_err(|e| format!("Failed to decode file_data for Gemini upload: {}", e))?;

    let upload_url = format!("{}/files", base_url.trim_end_matches('/'));
    let start_response = client
        .post(&upload_url)
        .query(&[("key", api_key)])
        .header("X-Goog-Upload-Protocol", "resumable")
        .header("X-Goog-Upload-Command", "start")
        .header(
            "X-Goog-Upload-Header-Content-Length",
            bytes.len().to_string(),
        )
        .header("X-Goog-Upload-Header-Content-Type", &mime_type)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "file": {
                "display_name": filename,
                "mime_type": mime_type
            }
        }))
        .send()
        .await
        .map_err(|e| format!("Gemini file upload start failed: {}", e))?;

    if !start_response.status().is_success() {
        let error_text = start_response.text().await.unwrap_or_default();
        return Err(format!(
            "Gemini file upload start API error: {}",
            error_text
        ));
    }

    let upload_session_url = start_response
        .headers()
        .get("x-goog-upload-url")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| "Gemini file upload missing x-goog-upload-url".to_string())?
        .to_string();

    let finalize_response = client
        .post(upload_session_url)
        .header("X-Goog-Upload-Command", "upload, finalize")
        .header("X-Goog-Upload-Offset", "0")
        .header("Content-Length", bytes.len().to_string())
        .header("Content-Type", "application/octet-stream")
        .body(bytes)
        .send()
        .await
        .map_err(|e| format!("Gemini file upload finalize failed: {}", e))?;

    if !finalize_response.status().is_success() {
        let error_text = finalize_response.text().await.unwrap_or_default();
        return Err(format!(
            "Gemini file upload finalize API error: {}",
            error_text
        ));
    }

    let body: GeminiFileUploadResponse = finalize_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Gemini file upload response: {}", e))?;

    let file_uri = body
        .file
        .uri
        .ok_or_else(|| "Gemini file upload response missing file.uri".to_string())?;

    file_cache()
        .write()
        .await
        .insert(cache_key.to_string(), file_uri.clone());

    if let (Some(data_dir), Some(attachment_id)) = (data_dir, file.attachment_id.as_deref()) {
        upsert_remote_handle(
            data_dir,
            attachment_id,
            api_config_id,
            "gemini",
            base_url,
            api_key,
            "file_uri",
            &file_uri,
        )?;
    }

    Ok(file_uri)
}

#[derive(Debug, Deserialize)]
struct GeminiFileUploadResponse {
    file: GeminiUploadedFile,
}

#[derive(Debug, Deserialize)]
struct GeminiUploadedFile {
    uri: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{gemini_file_cache_key, should_upload_as_gemini_file};
    use crate::api::provider::FileRef;

    fn pdf_file_ref() -> FileRef {
        FileRef {
            attachment_id: Some("att-1".to_string()),
            file_id: None,
            file_uri: None,
            file_data: Some("ZmFrZS1wZGY=".to_string()),
            filename: Some("test.pdf".to_string()),
            mime_type: Some("application/pdf".to_string()),
        }
    }

    #[test]
    fn uploads_only_inline_pdf_without_existing_file_uri() {
        assert!(should_upload_as_gemini_file(&pdf_file_ref()));

        let with_file_uri = FileRef {
            file_uri: Some("files/abc".to_string()),
            ..pdf_file_ref()
        };
        assert!(!should_upload_as_gemini_file(&with_file_uri));
    }

    #[test]
    fn cache_key_changes_for_different_accounts() {
        let file = pdf_file_ref();
        let key_a = gemini_file_cache_key(
            "https://generativelanguage.googleapis.com/upload/v1beta",
            "key-a",
            &file,
        );
        let key_b = gemini_file_cache_key(
            "https://generativelanguage.googleapis.com/upload/v1beta",
            "key-b",
            &file,
        );

        assert_ne!(key_a, key_b);
    }
}
