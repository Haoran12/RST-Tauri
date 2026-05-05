//! Persistent upload-cache entries for ST chat attachments.

use std::fs;
use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::storage::paths::safe_join;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentUploadCacheEntry {
    pub attachment_id: String,
    pub api_config_id: String,
    pub provider_kind: String,
    pub base_url: String,
    pub account_fingerprint: String,
    pub remote_kind: String,
    pub remote_handle: String,
    pub created_at: String,
    pub updated_at: String,
}

pub fn find_remote_handle(
    data_dir: &Path,
    attachment_id: &str,
    api_config_id: &str,
    provider_kind: &str,
    base_url: &str,
    api_key: &str,
    remote_kind: &str,
) -> Result<Option<String>, String> {
    let entries = load_upload_cache_entries(data_dir, attachment_id)?;
    let account_fingerprint = account_fingerprint(api_key);
    Ok(entries
        .into_iter()
        .find(|entry| {
            entry.api_config_id == api_config_id
                && entry.provider_kind == provider_kind
                && entry.base_url == base_url
                && entry.account_fingerprint == account_fingerprint
                && entry.remote_kind == remote_kind
        })
        .map(|entry| entry.remote_handle))
}

pub fn upsert_remote_handle(
    data_dir: &Path,
    attachment_id: &str,
    api_config_id: &str,
    provider_kind: &str,
    base_url: &str,
    api_key: &str,
    remote_kind: &str,
    remote_handle: &str,
) -> Result<(), String> {
    let mut entries = load_upload_cache_entries(data_dir, attachment_id)?;
    let account_fingerprint = account_fingerprint(api_key);
    let now = Utc::now().to_rfc3339();

    if let Some(entry) = entries.iter_mut().find(|entry| {
        entry.api_config_id == api_config_id
            && entry.provider_kind == provider_kind
            && entry.base_url == base_url
            && entry.account_fingerprint == account_fingerprint
            && entry.remote_kind == remote_kind
    }) {
        entry.remote_handle = remote_handle.to_string();
        entry.updated_at = now;
    } else {
        entries.push(AttachmentUploadCacheEntry {
            attachment_id: attachment_id.to_string(),
            api_config_id: api_config_id.to_string(),
            provider_kind: provider_kind.to_string(),
            base_url: base_url.to_string(),
            account_fingerprint,
            remote_kind: remote_kind.to_string(),
            remote_handle: remote_handle.to_string(),
            created_at: now.clone(),
            updated_at: now,
        });
    }

    write_upload_cache_entries(data_dir, attachment_id, &entries)
}

pub fn delete_remote_handle(
    data_dir: &Path,
    attachment_id: &str,
    api_config_id: &str,
    provider_kind: &str,
    base_url: &str,
    api_key: &str,
    remote_kind: &str,
) -> Result<(), String> {
    let mut entries = load_upload_cache_entries(data_dir, attachment_id)?;
    let account_fingerprint = account_fingerprint(api_key);
    let original_len = entries.len();
    entries.retain(|entry| {
        !(entry.api_config_id == api_config_id
            && entry.provider_kind == provider_kind
            && entry.base_url == base_url
            && entry.account_fingerprint == account_fingerprint
            && entry.remote_kind == remote_kind)
    });

    if entries.len() == original_len {
        return Ok(());
    }

    write_upload_cache_entries(data_dir, attachment_id, &entries)
}

pub fn list_remote_handles(
    data_dir: &Path,
    attachment_id: &str,
) -> Result<Vec<AttachmentUploadCacheEntry>, String> {
    load_upload_cache_entries(data_dir, attachment_id)
}

pub fn clear_remote_handles(
    data_dir: &Path,
    attachment_id: &str,
    api_config_id: Option<&str>,
) -> Result<usize, String> {
    let mut entries = load_upload_cache_entries(data_dir, attachment_id)?;
    let original_len = entries.len();

    match api_config_id {
        Some(target_api_config_id) => {
            entries.retain(|entry| entry.api_config_id != target_api_config_id);
        }
        None => entries.clear(),
    }

    let removed = original_len.saturating_sub(entries.len());
    if removed == 0 {
        return Ok(0);
    }

    write_upload_cache_entries(data_dir, attachment_id, &entries)?;
    Ok(removed)
}

fn load_upload_cache_entries(
    data_dir: &Path,
    attachment_id: &str,
) -> Result<Vec<AttachmentUploadCacheEntry>, String> {
    let path = safe_join(
        data_dir,
        &format!("chat_attachments/{}/upload_cache.json", attachment_id),
    )?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let text = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read upload cache {}: {}", attachment_id, e))?;
    serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse upload cache {}: {}", attachment_id, e))
}

fn write_upload_cache_entries(
    data_dir: &Path,
    attachment_id: &str,
    entries: &[AttachmentUploadCacheEntry],
) -> Result<(), String> {
    let path = safe_join(
        data_dir,
        &format!("chat_attachments/{}/upload_cache.json", attachment_id),
    )?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create upload cache directory: {}", e))?;
    }

    let text = serde_json::to_string_pretty(entries)
        .map_err(|e| format!("Failed to serialize upload cache {}: {}", attachment_id, e))?;
    fs::write(&path, text)
        .map_err(|e| format!("Failed to write upload cache {}: {}", attachment_id, e))
}

fn account_fingerprint(api_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::{
        clear_remote_handles, delete_remote_handle, find_remote_handle, list_remote_handles,
        upsert_remote_handle,
    };

    #[test]
    fn upload_cache_roundtrip_respects_connection_scope() {
        let temp_dir = tempfile::tempdir().unwrap();
        let data_dir = temp_dir.path();

        upsert_remote_handle(
            data_dir,
            "att-1",
            "api-a",
            "openai_responses",
            "https://api.openai.com/v1",
            "sk-a",
            "file_id",
            "file_123",
        )
        .unwrap();

        let found = find_remote_handle(
            data_dir,
            "att-1",
            "api-a",
            "openai_responses",
            "https://api.openai.com/v1",
            "sk-a",
            "file_id",
        )
        .unwrap();
        assert_eq!(found.as_deref(), Some("file_123"));

        let wrong_account = find_remote_handle(
            data_dir,
            "att-1",
            "api-a",
            "openai_responses",
            "https://api.openai.com/v1",
            "sk-b",
            "file_id",
        )
        .unwrap();
        assert!(wrong_account.is_none());
    }

    #[test]
    fn delete_remote_handle_removes_only_matching_connection() {
        let temp_dir = tempfile::tempdir().unwrap();
        let data_dir = temp_dir.path();

        upsert_remote_handle(
            data_dir,
            "att-1",
            "api-a",
            "openai_responses",
            "https://api.openai.com/v1",
            "sk-a",
            "file_id",
            "file_123",
        )
        .unwrap();
        upsert_remote_handle(
            data_dir,
            "att-1",
            "api-a",
            "openai_responses",
            "https://api.openai.com/v1",
            "sk-b",
            "file_id",
            "file_456",
        )
        .unwrap();

        delete_remote_handle(
            data_dir,
            "att-1",
            "api-a",
            "openai_responses",
            "https://api.openai.com/v1",
            "sk-a",
            "file_id",
        )
        .unwrap();

        let deleted = find_remote_handle(
            data_dir,
            "att-1",
            "api-a",
            "openai_responses",
            "https://api.openai.com/v1",
            "sk-a",
            "file_id",
        )
        .unwrap();
        assert!(deleted.is_none());

        let untouched = find_remote_handle(
            data_dir,
            "att-1",
            "api-a",
            "openai_responses",
            "https://api.openai.com/v1",
            "sk-b",
            "file_id",
        )
        .unwrap();
        assert_eq!(untouched.as_deref(), Some("file_456"));
    }

    #[test]
    fn clear_remote_handles_can_target_api_config_or_all() {
        let temp_dir = tempfile::tempdir().unwrap();
        let data_dir = temp_dir.path();

        upsert_remote_handle(
            data_dir,
            "att-1",
            "api-a",
            "openai_responses",
            "https://api.openai.com/v1",
            "sk-a",
            "file_id",
            "file_123",
        )
        .unwrap();
        upsert_remote_handle(
            data_dir,
            "att-1",
            "api-b",
            "gemini",
            "https://generativelanguage.googleapis.com/upload/v1beta",
            "sk-b",
            "file_uri",
            "files/abc",
        )
        .unwrap();

        let removed_current = clear_remote_handles(data_dir, "att-1", Some("api-a")).unwrap();
        assert_eq!(removed_current, 1);
        assert_eq!(list_remote_handles(data_dir, "att-1").unwrap().len(), 1);

        let removed_rest = clear_remote_handles(data_dir, "att-1", None).unwrap();
        assert_eq!(removed_rest, 1);
        assert!(list_remote_handles(data_dir, "att-1").unwrap().is_empty());
    }
}
