//! Shared OpenAI Files API upload/cache helpers.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::{Arc, OnceLock};

use base64::Engine;
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::api::provider::{ContentPart, FileRef};
use crate::storage::attachment_upload_cache::{
    delete_remote_handle, find_remote_handle, upsert_remote_handle,
};

type FileCacheMap = HashMap<String, String>;

fn file_cache() -> &'static Arc<RwLock<FileCacheMap>> {
    static CACHE: OnceLock<Arc<RwLock<FileCacheMap>>> = OnceLock::new();
    CACHE.get_or_init(|| Arc::new(RwLock::new(HashMap::new())))
}

pub async fn invalidate_openai_file_cache(
    data_dir: Option<&Path>,
    base_url: &str,
    api_key: &str,
    api_config_id: &str,
    file: &FileRef,
) -> Result<(), String> {
    let cache_key = openai_file_cache_key(base_url, api_key, file);
    file_cache().write().await.remove(&cache_key);

    if let (Some(data_dir), Some(attachment_id)) = (data_dir, file.attachment_id.as_deref()) {
        delete_remote_handle(
            data_dir,
            attachment_id,
            api_config_id,
            "openai",
            base_url,
            api_key,
            "file_id",
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
    messages: &[crate::api::provider::ChatMessage],
) -> Result<Vec<crate::api::provider::ChatMessage>, String> {
    let mut prepared = Vec::with_capacity(messages.len());

    for message in messages {
        let mut content = Vec::with_capacity(message.content.len());
        for part in &message.content {
            match part {
                ContentPart::FileRef { file } if should_upload_as_openai_file(file) => {
                    let cache_key = openai_file_cache_key(base_url, api_key, file);
                    let file_id = get_or_upload_openai_file(
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
                            file_id: Some(file_id),
                            file_uri: None,
                            file_data: None,
                            filename: file.filename.clone(),
                            mime_type: file.mime_type.clone(),
                        },
                    });
                }
                _ => content.push(part.clone()),
            }
        }

        prepared.push(crate::api::provider::ChatMessage {
            role: message.role.clone(),
            content,
            name: message.name.clone(),
        });
    }

    Ok(prepared)
}

fn should_upload_as_openai_file(file: &FileRef) -> bool {
    file.file_id.is_none()
        && file.file_data.is_some()
        && matches!(file.mime_type.as_deref(), Some("application/pdf"))
}

fn openai_file_cache_key(base_url: &str, api_key: &str, file: &FileRef) -> String {
    let mut hasher = DefaultHasher::new();
    base_url.hash(&mut hasher);
    api_key.hash(&mut hasher);
    file.filename.hash(&mut hasher);
    file.mime_type.hash(&mut hasher);
    file.file_data.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

async fn get_or_upload_openai_file(
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
            "openai",
            base_url,
            api_key,
            "file_id",
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
        .ok_or_else(|| "Missing file_data for OpenAI file upload".to_string())?;
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
        .map_err(|e| format!("Failed to decode file_data for OpenAI upload: {}", e))?;

    let part = Part::bytes(bytes)
        .file_name(filename.clone())
        .mime_str(&mime_type)
        .map_err(|e| format!("Failed to build OpenAI file part: {}", e))?;
    let form = Form::new().part("file", part).text("purpose", "user_data");

    let response = client
        .post(format!("{}/files", base_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("OpenAI file upload failed: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("OpenAI file upload API error: {}", error_text));
    }

    let body: OpenAIFileUploadResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse OpenAI file upload response: {}", e))?;

    file_cache()
        .write()
        .await
        .insert(cache_key.to_string(), body.id.clone());

    if let (Some(data_dir), Some(attachment_id)) = (data_dir, file.attachment_id.as_deref()) {
        upsert_remote_handle(
            data_dir,
            attachment_id,
            api_config_id,
            "openai",
            base_url,
            api_key,
            "file_id",
            &body.id,
        )?;
    }

    Ok(body.id)
}

#[derive(Debug, Deserialize)]
struct OpenAIFileUploadResponse {
    id: String,
}

#[cfg(test)]
mod tests {
    use super::{openai_file_cache_key, should_upload_as_openai_file};
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
    fn uploads_only_inline_pdf_without_existing_file_id() {
        assert!(should_upload_as_openai_file(&pdf_file_ref()));

        let with_file_id = FileRef {
            file_id: Some("file_123".to_string()),
            ..pdf_file_ref()
        };
        assert!(!should_upload_as_openai_file(&with_file_id));

        let image = FileRef {
            mime_type: Some("image/png".to_string()),
            ..pdf_file_ref()
        };
        assert!(!should_upload_as_openai_file(&image));
    }

    #[test]
    fn cache_key_changes_for_different_accounts() {
        let file = pdf_file_ref();
        let key_a = openai_file_cache_key("https://api.openai.com/v1", "sk-account-a", &file);
        let key_b = openai_file_cache_key("https://api.openai.com/v1", "sk-account-b", &file);

        assert_ne!(key_a, key_b);
    }

    #[test]
    fn cache_key_does_not_inline_raw_file_data() {
        let file = pdf_file_ref();
        let key = openai_file_cache_key("https://api.openai.com/v1", "sk-account-a", &file);

        assert!(!key.contains("ZmFrZS1wZGY="));
        assert_eq!(key.len(), 16);
    }
}
