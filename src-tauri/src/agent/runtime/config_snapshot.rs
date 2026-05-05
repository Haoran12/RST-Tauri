//! Configuration snapshots for Agent runtime
//!
//! Provides fixed configuration snapshots that are captured at the start of each turn
//! and used consistently throughout the turn. This ensures that configuration changes
//! mid-turn don't affect ongoing operations.
//!
//! See docs/11_agent_runtime.md §2 for the fixed snapshot mechanism.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Runtime configuration snapshot
///
/// Captured at the start of each turn from global `app_runtime.yaml`.
/// Contains budget limits, retention policies, and other runtime settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfigSnapshot {
    /// Unique identifier for this snapshot
    pub snapshot_id: String,
    /// Schema version for future migrations
    pub schema_version: u32,
    /// Hash of the configuration content
    pub config_hash: String,
    /// Source paths that contributed to this snapshot
    pub source_paths: Vec<String>,
    /// Budget configuration
    pub request_budget: RequestBudgetConfig,
    /// Log retention configuration
    pub log_retention: LogRetentionConfig,
    /// When this snapshot was created
    pub created_at: DateTime<Utc>,
}

/// World rules snapshot
///
/// Captured at the start of each turn from World's `world_base.yaml`.
/// Contains world-specific rules, attribute tiers, and thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldRulesSnapshot {
    /// Unique identifier for this snapshot
    pub snapshot_id: String,
    /// Parent runtime config snapshot
    pub runtime_config_snapshot_id: String,
    /// World this snapshot belongs to
    pub world_id: String,
    /// Schema version for future migrations
    pub schema_version: u32,
    /// Hash of the configuration content
    pub config_hash: String,
    /// Source paths that contributed to this snapshot
    pub source_paths: Vec<String>,
    /// Attribute tier boundaries
    pub attribute_tiers: AttributeTierConfig,
    /// Mana expression settings
    pub mana_expression: ManaExpressionConfig,
    /// Combat resolution settings
    pub combat_resolution: CombatResolutionConfig,
    /// When this snapshot was created
    pub created_at: DateTime<Utc>,
}

/// Request budget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBudgetConfig {
    /// Input token budgets
    pub input_tokens: InputTokenBudget,
    /// Cognitive pass scheduling
    pub cognitive_scheduling: CognitiveSchedulingConfig,
    /// Maximum LLM calls per turn
    pub max_llm_calls_per_turn: usize,
    /// Maximum reaction passes per window
    pub max_reaction_passes_per_window: usize,
    /// Maximum reaction depth
    pub max_reaction_depth: u8,
}

/// Input token budget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputTokenBudget {
    /// Critical attention band (default 8K)
    pub critical_attention_tokens: u32,
    /// Soft limit (default 16K)
    pub soft_tokens: u32,
    /// Maximum context (default 32K)
    pub max_context_tokens: u32,
    /// Reserved output tokens
    pub reserved_output_tokens: u32,
}

/// Cognitive pass scheduling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveSchedulingConfig {
    /// Maximum primary cognitive passes per turn
    pub max_primary_cognitive_passes: usize,
    /// Threshold for enabling tiering
    pub tiering_start_active_characters: usize,
}

/// Log retention configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRetentionConfig {
    /// Maximum log size in bytes (default 1GB)
    pub max_size_bytes: u64,
    /// Days before warning about inactive worlds
    pub inactive_world_warning_days: u32,
}

/// Attribute tier configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeTierConfig {
    /// Tier boundaries for attributes
    pub boundaries: Vec<AttributeTierBoundary>,
}

/// Attribute tier boundary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeTierBoundary {
    /// Tier name
    pub tier: String,
    /// Minimum value for this tier
    pub min_value: f64,
    /// Maximum value for this tier
    pub max_value: f64,
}

/// Mana expression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManaExpressionConfig {
    /// Default tendency for new characters
    pub default_tendency: String,
    /// Tendency factor range
    pub tendency_factor_range: (f64, f64),
}

/// Combat resolution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatResolutionConfig {
    /// Minimum effectiveness ratio
    pub min_effectiveness: f64,
    /// Soul factor range
    pub soul_factor_range: (f64, f64),
}

impl Default for RequestBudgetConfig {
    fn default() -> Self {
        Self {
            input_tokens: InputTokenBudget::default(),
            cognitive_scheduling: CognitiveSchedulingConfig::default(),
            max_llm_calls_per_turn: 20,
            max_reaction_passes_per_window: 3,
            max_reaction_depth: 1,
        }
    }
}

impl Default for InputTokenBudget {
    fn default() -> Self {
        Self {
            critical_attention_tokens: 8192,
            soft_tokens: 16384,
            max_context_tokens: 32768,
            reserved_output_tokens: 4096,
        }
    }
}

impl Default for CognitiveSchedulingConfig {
    fn default() -> Self {
        Self {
            max_primary_cognitive_passes: 3,
            tiering_start_active_characters: 4,
        }
    }
}

impl Default for LogRetentionConfig {
    fn default() -> Self {
        Self {
            max_size_bytes: 1024 * 1024 * 1024, // 1GB
            inactive_world_warning_days: 30,
        }
    }
}

impl Default for AttributeTierConfig {
    fn default() -> Self {
        Self {
            boundaries: vec![
                AttributeTierBoundary {
                    tier: "Mundane".to_string(),
                    min_value: 0.0,
                    max_value: 100.0,
                },
                AttributeTierBoundary {
                    tier: "Awakened".to_string(),
                    min_value: 100.0,
                    max_value: 300.0,
                },
                AttributeTierBoundary {
                    tier: "Adept".to_string(),
                    min_value: 300.0,
                    max_value: 600.0,
                },
                AttributeTierBoundary {
                    tier: "Master".to_string(),
                    min_value: 600.0,
                    max_value: 1000.0,
                },
                AttributeTierBoundary {
                    tier: "Ascendant".to_string(),
                    min_value: 1000.0,
                    max_value: 2000.0,
                },
                AttributeTierBoundary {
                    tier: "Transcendent".to_string(),
                    min_value: 2000.0,
                    max_value: f64::MAX,
                },
            ],
        }
    }
}

impl Default for ManaExpressionConfig {
    fn default() -> Self {
        Self {
            default_tendency: "Neutral".to_string(),
            tendency_factor_range: (-0.5, 0.5),
        }
    }
}

impl Default for CombatResolutionConfig {
    fn default() -> Self {
        Self {
            min_effectiveness: 0.1,
            soul_factor_range: (0.5, 1.5),
        }
    }
}

impl RuntimeConfigSnapshot {
    /// Create a new runtime config snapshot with default values
    pub fn new(source_paths: Vec<String>) -> Self {
        let config = RequestBudgetConfig::default();
        let config_hash = Self::compute_hash(&config);

        Self {
            snapshot_id: generate_snapshot_id("rcfg"),
            schema_version: 1,
            config_hash,
            source_paths,
            request_budget: config,
            log_retention: LogRetentionConfig::default(),
            created_at: Utc::now(),
        }
    }

    /// Create a snapshot with custom configuration
    pub fn with_config(
        source_paths: Vec<String>,
        request_budget: RequestBudgetConfig,
        log_retention: LogRetentionConfig,
    ) -> Self {
        let config_hash = Self::compute_hash(&request_budget);

        Self {
            snapshot_id: generate_snapshot_id("rcfg"),
            schema_version: 1,
            config_hash,
            source_paths,
            request_budget,
            log_retention,
            created_at: Utc::now(),
        }
    }

    fn compute_hash(config: &RequestBudgetConfig) -> String {
        let json = serde_json::to_string(config).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        format!("{:x}", hasher.finalize())[..16].to_string()
    }

    /// Get the effective max context tokens
    pub fn effective_max_context(&self, provider_context_window: Option<u32>) -> u32 {
        let base_max = self.request_budget.input_tokens.max_context_tokens;
        if let Some(provider_max) = provider_context_window {
            base_max.min(provider_max - self.request_budget.input_tokens.reserved_output_tokens)
        } else {
            base_max
        }
    }
}

impl WorldRulesSnapshot {
    /// Create a new world rules snapshot with default values
    pub fn new(
        world_id: String,
        runtime_config_snapshot_id: String,
        source_paths: Vec<String>,
    ) -> Self {
        let config_hash = Self::compute_hash();

        Self {
            snapshot_id: generate_snapshot_id("wrules"),
            runtime_config_snapshot_id,
            world_id,
            schema_version: 1,
            config_hash,
            source_paths,
            attribute_tiers: AttributeTierConfig::default(),
            mana_expression: ManaExpressionConfig::default(),
            combat_resolution: CombatResolutionConfig::default(),
            created_at: Utc::now(),
        }
    }

    fn compute_hash() -> String {
        // For now, use a simple hash based on default config
        let mut hasher = Sha256::new();
        hasher.update(b"default_world_rules");
        format!("{:x}", hasher.finalize())[..16].to_string()
    }
}

/// Generate a unique snapshot ID
fn generate_snapshot_id(prefix: &str) -> String {
    format!("{}_{}", prefix, uuid::Uuid::new_v4())
}

/// Snapshot manager for tracking active snapshots
#[derive(Debug, Clone, Default)]
pub struct SnapshotManager {
    /// Current runtime config snapshot
    current_runtime_snapshot: Option<RuntimeConfigSnapshot>,
    /// Current world rules snapshot (per world)
    current_world_snapshots: std::collections::HashMap<String, WorldRulesSnapshot>,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Capture a new runtime config snapshot
    pub fn capture_runtime_snapshot(&mut self, source_paths: Vec<String>) -> RuntimeConfigSnapshot {
        let snapshot = RuntimeConfigSnapshot::new(source_paths);
        self.current_runtime_snapshot = Some(snapshot.clone());
        snapshot
    }

    /// Capture a new runtime config snapshot with custom config
    pub fn capture_runtime_snapshot_with_config(
        &mut self,
        source_paths: Vec<String>,
        request_budget: RequestBudgetConfig,
        log_retention: LogRetentionConfig,
    ) -> RuntimeConfigSnapshot {
        let snapshot =
            RuntimeConfigSnapshot::with_config(source_paths, request_budget, log_retention);
        self.current_runtime_snapshot = Some(snapshot.clone());
        snapshot
    }

    /// Capture a new world rules snapshot
    pub fn capture_world_snapshot(
        &mut self,
        world_id: String,
        source_paths: Vec<String>,
    ) -> Result<WorldRulesSnapshot, String> {
        let runtime_snapshot_id = self
            .current_runtime_snapshot
            .as_ref()
            .map(|s| s.snapshot_id.clone())
            .ok_or_else(|| "No runtime config snapshot captured".to_string())?;

        let snapshot = WorldRulesSnapshot::new(world_id.clone(), runtime_snapshot_id, source_paths);
        self.current_world_snapshots
            .insert(world_id, snapshot.clone());
        Ok(snapshot)
    }

    /// Get the current runtime config snapshot
    pub fn current_runtime(&self) -> Option<&RuntimeConfigSnapshot> {
        self.current_runtime_snapshot.as_ref()
    }

    /// Get the current world rules snapshot for a world
    pub fn current_world(&self, world_id: &str) -> Option<&WorldRulesSnapshot> {
        self.current_world_snapshots.get(world_id)
    }

    /// Get runtime snapshot ID
    pub fn runtime_snapshot_id(&self) -> Option<&str> {
        self.current_runtime_snapshot
            .as_ref()
            .map(|s| s.snapshot_id.as_str())
    }

    /// Get world snapshot ID
    pub fn world_snapshot_id(&self, world_id: &str) -> Option<&str> {
        self.current_world_snapshots
            .get(world_id)
            .map(|s| s.snapshot_id.as_str())
    }

    /// Clear all snapshots (used when resetting)
    pub fn clear(&mut self) {
        self.current_runtime_snapshot = None;
        self.current_world_snapshots.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_runtime_snapshot() {
        let snapshot = RuntimeConfigSnapshot::new(vec!["app_runtime.yaml".to_string()]);

        assert!(!snapshot.snapshot_id.is_empty());
        assert!(!snapshot.config_hash.is_empty());
        assert_eq!(snapshot.request_budget.max_llm_calls_per_turn, 20);
    }

    #[test]
    fn creates_world_snapshot() {
        let runtime = RuntimeConfigSnapshot::new(vec!["app_runtime.yaml".to_string()]);
        let world = WorldRulesSnapshot::new(
            "world_1".to_string(),
            runtime.snapshot_id.clone(),
            vec!["world_base.yaml".to_string()],
        );

        assert!(!world.snapshot_id.is_empty());
        assert_eq!(world.world_id, "world_1");
        assert_eq!(world.runtime_config_snapshot_id, runtime.snapshot_id);
    }

    #[test]
    fn manages_snapshots() {
        let mut manager = SnapshotManager::new();

        let _runtime = manager.capture_runtime_snapshot(vec!["app_runtime.yaml".to_string()]);
        assert!(manager.current_runtime().is_some());

        let world = manager
            .capture_world_snapshot("world_1".to_string(), vec!["world_base.yaml".to_string()]);
        assert!(world.is_ok());
        assert!(manager.current_world("world_1").is_some());
    }

    #[test]
    fn computes_effective_max_context() {
        let snapshot = RuntimeConfigSnapshot::new(vec![]);

        // Without provider limit, use config max
        assert_eq!(snapshot.effective_max_context(None), 32768);

        // With provider limit, use minimum
        assert_eq!(snapshot.effective_max_context(Some(40000)), 32768);
        assert_eq!(snapshot.effective_max_context(Some(20000)), 15904); // 20000 - 4096 reserved
    }
}
