//! LLM API contracts loader and connection-level cache.

use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;

use crate::storage::st_resources::ApiConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmApiContractsSnapshot {
    pub llm_api_contracts_snapshot_id: String,
    pub schema_version: String,
    pub contracts_hash: String,
    pub root: Value,
}

impl LlmApiContractsSnapshot {
    pub fn protocol_contract(&self, protocol_kind: &str) -> Option<&Value> {
        self.root
            .get("contracts")
            .and_then(|c| c.get(protocol_kind))
    }

    pub fn multimodal_policy(&self) -> Option<&Value> {
        self.root
            .get("adapter_defaults")
            .and_then(|v| v.get("multimodal_attachment_policy"))
    }

    pub fn protocol_kind_for_provider(&self, provider_kind: &str) -> Option<&'static str> {
        protocol_kind_for_provider(provider_kind)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ProviderContractCacheKey {
    pub api_config_id: String,
    pub provider_kind: String,
    pub protocol_kind: String,
    pub model: String,
    pub base_url: String,
    pub provider_variant: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompiledProviderContractView {
    pub key: ProviderContractCacheKey,
    pub contract: Value,
    pub input_capabilities: Option<Value>,
    pub multimodal_policy: Option<Value>,
}

#[derive(Debug, Default)]
pub struct ProviderContractCache {
    inner: RwLock<HashMap<ProviderContractCacheKey, Arc<CompiledProviderContractView>>>,
}

impl ProviderContractCache {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    pub async fn get_or_insert(
        &self,
        snapshot: &LlmApiContractsSnapshot,
        api_config: &ApiConfig,
    ) -> Result<Arc<CompiledProviderContractView>, String> {
        let key = ProviderContractCacheKey::from_api_config(api_config);

        if let Some(existing) = self.inner.read().await.get(&key).cloned() {
            return Ok(existing);
        }

        let contract = snapshot
            .protocol_contract(&key.protocol_kind)
            .cloned()
            .ok_or_else(|| format!("No contract found for protocol {}", key.protocol_kind))?;

        let compiled = Arc::new(CompiledProviderContractView {
            key: key.clone(),
            input_capabilities: contract.get("input_capabilities").cloned(),
            multimodal_policy: snapshot.multimodal_policy().cloned(),
            contract,
        });

        self.inner.write().await.insert(key, compiled.clone());
        Ok(compiled)
    }

    pub async fn invalidate_api_config(&self, api_config_id: &str) {
        self.inner
            .write()
            .await
            .retain(|key, _| key.api_config_id != api_config_id);
    }

    pub async fn clear(&self) {
        self.inner.write().await.clear();
    }
}

impl ProviderContractCacheKey {
    pub fn from_api_config(api_config: &ApiConfig) -> Self {
        let protocol_kind = protocol_kind_for_provider(&api_config.provider)
            .unwrap_or(api_config.provider.as_str())
            .to_string();

        let provider_variant = api_config
            .settings
            .get("provider_variant")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Self {
            api_config_id: api_config.id.clone(),
            provider_kind: api_config.provider.clone(),
            protocol_kind,
            model: api_config.model.clone(),
            base_url: api_config.base_url.clone().unwrap_or_default(),
            provider_variant,
        }
    }
}

pub fn load_llm_api_contracts_snapshot(path: &Path) -> Result<LlmApiContractsSnapshot, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read llm_api_contracts.json: {}", e))?;
    load_llm_api_contracts_snapshot_from_str(&text)
}

pub fn load_llm_api_contracts_snapshot_from_str(
    text: &str,
) -> Result<LlmApiContractsSnapshot, String> {
    let root: Value = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse llm_api_contracts.json: {}", e))?;

    let schema_version = root
        .get("schema_version")
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
        .to_string();

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);

    Ok(LlmApiContractsSnapshot {
        llm_api_contracts_snapshot_id: format!(
            "llm_contracts:{}",
            root.get("researched_at")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
        ),
        schema_version,
        contracts_hash: format!("{:x}", hasher.finish()),
        root,
    })
}

pub fn connection_supports_attachments(
    compiled: &CompiledProviderContractView,
) -> (bool, bool, bool) {
    let caps = match compiled.input_capabilities.as_ref() {
        Some(v) => v,
        None => return (true, false, false),
    };

    let text = caps
        .get("text")
        .and_then(|v| v.get("supported"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let image = capability_supported(caps.get("image"));
    let pdf = capability_supported(caps.get("pdf"));

    (text, image, pdf)
}

fn capability_supported(value: Option<&Value>) -> bool {
    match value.and_then(|v| v.get("supported")) {
        Some(Value::Bool(v)) => *v,
        Some(Value::String(s)) if s == "provider_declared_only" => false,
        _ => false,
    }
}

pub fn protocol_kind_for_provider(provider_kind: &str) -> Option<&'static str> {
    match provider_kind {
        "openai_responses" => Some("openai_responses"),
        "openai_chat" => Some("openai_chat_completions"),
        "anthropic" => Some("anthropic_messages"),
        "gemini" => Some("gemini_generate_content"),
        "deepseek" => Some("deepseek_chat"),
        "claude_code" => Some("claude_code_interface"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_snapshot() -> LlmApiContractsSnapshot {
        LlmApiContractsSnapshot {
            llm_api_contracts_snapshot_id: "test".to_string(),
            schema_version: "1".to_string(),
            contracts_hash: "abc".to_string(),
            root: json!({
                "contracts": {
                    "openai_chat_completions": {
                        "input_capabilities": {
                            "text": { "supported": true },
                            "image": { "supported": true },
                            "pdf": { "supported": false }
                        }
                    },
                    "claude_code_interface": {
                        "input_capabilities": {
                            "text": { "supported": true },
                            "image": { "supported": "provider_declared_only" },
                            "pdf": { "supported": "provider_declared_only" }
                        }
                    }
                },
                "adapter_defaults": {
                    "multimodal_attachment_policy": {
                        "source_of_truth": "local_file"
                    }
                }
            }),
        }
    }

    fn sample_api_config(id: &str, provider: &str) -> ApiConfig {
        ApiConfig {
            id: id.to_string(),
            name: "cfg".to_string(),
            provider: provider.to_string(),
            model: "model-a".to_string(),
            base_url: Some("https://example.test".to_string()),
            api_key: None,
            enabled: true,
            settings: serde_json::Map::new(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn provider_kind_maps_to_protocol_kind() {
        assert_eq!(
            protocol_kind_for_provider("openai_chat"),
            Some("openai_chat_completions")
        );
        assert_eq!(
            protocol_kind_for_provider("claude_code"),
            Some("claude_code_interface")
        );
        assert_eq!(protocol_kind_for_provider("unknown"), None);
    }

    #[test]
    fn provider_declared_only_is_not_treated_as_supported() {
        let snapshot = sample_snapshot();
        let contract = snapshot
            .protocol_contract("claude_code_interface")
            .expect("contract")
            .clone();
        let compiled = CompiledProviderContractView {
            key: ProviderContractCacheKey::from_api_config(&sample_api_config("1", "claude_code")),
            input_capabilities: contract.get("input_capabilities").cloned(),
            multimodal_policy: snapshot.multimodal_policy().cloned(),
            contract,
        };

        assert_eq!(
            connection_supports_attachments(&compiled),
            (true, false, false)
        );
    }

    #[tokio::test]
    async fn cache_invalidation_removes_entries_for_api_config() {
        let snapshot = sample_snapshot();
        let cache = ProviderContractCache::new();
        let config = sample_api_config("cfg-1", "openai_chat");

        let first = cache
            .get_or_insert(&snapshot, &config)
            .await
            .expect("first contract");
        let second = cache
            .get_or_insert(&snapshot, &config)
            .await
            .expect("cached contract");

        assert!(Arc::ptr_eq(&first, &second));

        cache.invalidate_api_config("cfg-1").await;

        let third = cache
            .get_or_insert(&snapshot, &config)
            .await
            .expect("reloaded contract");
        assert!(!Arc::ptr_eq(&first, &third));
    }
}
