//! Configuration registry
//!
//! Manages runtime configuration snapshots

use serde_json::Value;

/// Runtime configuration snapshot
pub struct RuntimeConfigSnapshot {
    pub runtime_config_snapshot_id: String,
    pub schema_version: String,
    pub config_hash: String,
}

/// World rules snapshot
pub struct WorldRulesSnapshot {
    pub world_rules_snapshot_id: String,
    pub runtime_config_snapshot_id: String,
    pub world_id: String,
    pub schema_version: String,
    pub config_hash: String,
}

/// LLM API contracts snapshot
pub struct LlmApiContractsSnapshot {
    pub llm_api_contracts_snapshot_id: String,
    pub schema_version: String,
    pub contracts_hash: String,
    pub provider_contracts: Value,
    pub multimodal_policy: Value,
}
