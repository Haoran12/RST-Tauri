//! Configuration snapshots for Agent runtime.
//!
//! Provides fixed configuration snapshots that are captured at the start of each turn
//! and used consistently throughout the turn. This ensures that configuration changes
//! mid-turn don't affect ongoing operations.

use crate::agent::models::{AttributeDelta, AttributeTier, ManaPresenceRadiusTier};
use crate::config::world_argument::WorldArgumentConfig;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Runtime configuration snapshot
///
/// Captured at the start of each turn from global `app_runtime.yaml`.
/// Contains budget limits, retention policies, and other runtime settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfigSnapshot {
    pub snapshot_id: String,
    pub schema_version: u32,
    pub config_hash: String,
    pub source_paths: Vec<String>,
    pub request_budget: RequestBudgetConfig,
    pub log_retention: LogRetentionConfig,
    pub created_at: DateTime<Utc>,
}

/// World rules snapshot
///
/// Captured at the start of each turn from World's `world_argument.yaml`.
/// Contains compiled world-specific rules and thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldRulesSnapshot {
    pub snapshot_id: String,
    pub runtime_config_snapshot_id: String,
    pub world_id: String,
    pub schema_version: u32,
    pub config_hash: String,
    pub source_paths: Vec<String>,
    pub world: WorldMetadataSnapshot,
    pub calendar: CalendarSnapshot,
    pub attribute_tiers: AttributeTierConfig,
    pub mana_expression: ManaExpressionConfig,
    pub combat_resolution: CombatResolutionConfig,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBudgetConfig {
    pub input_tokens: InputTokenBudget,
    pub cognitive_scheduling: CognitiveSchedulingConfig,
    pub max_llm_calls_per_turn: usize,
    pub max_reaction_passes_per_window: usize,
    pub max_reaction_depth: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputTokenBudget {
    pub critical_attention_tokens: u32,
    pub soft_tokens: u32,
    pub max_context_tokens: u32,
    pub reserved_output_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveSchedulingConfig {
    pub max_primary_cognitive_passes: usize,
    pub tiering_start_active_characters: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRetentionConfig {
    pub max_size_bytes: u64,
    pub inactive_world_warning_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldMetadataSnapshot {
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalendarSnapshot {
    pub default_calendar_id: String,
    pub eras: Vec<CalendarEraSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalendarEraSnapshot {
    pub era_id: String,
    pub display_name: String,
    pub start_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeTierConfig {
    pub boundaries: Vec<AttributeTierBoundary>,
    pub delta_thresholds: AttributeDeltaThresholdConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeTierBoundary {
    pub tier: String,
    pub min_value: f64,
    pub max_value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeDeltaThresholdConfig {
    pub indistinguishable_abs_lt: f64,
    pub slight_abs_lt: f64,
    pub notable_abs_lt: f64,
    pub far_abs_lt: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManaExpressionConfig {
    pub display_ratio_clamp: (f64, f64),
    pub tendency_factors: SnapshotTendencyFactors,
    pub mode_factors: SnapshotModeFactors,
    pub expression_modes: SnapshotExpressionModes,
    pub concealment_suspected_gap: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotTendencyFactors {
    pub inward: f64,
    pub neutral: f64,
    pub expressive: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotModeFactors {
    pub sealed: f64,
    pub suppressed: f64,
    pub natural: f64,
    pub released: f64,
    pub dominating: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotExpressionModes {
    pub sealed: SnapshotExpressionMode,
    pub suppressed: SnapshotExpressionMode,
    pub natural: SnapshotExpressionMode,
    pub released: SnapshotExpressionMode,
    pub dominating: SnapshotExpressionMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotExpressionMode {
    pub radius_tier: ManaPresenceRadiusTier,
    pub pressure_multiplier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatResolutionConfig {
    pub min_effectiveness: f64,
    pub soul_tier_factors: SoulTierFactorConfig,
    pub soul_damage_floor: f64,
    pub delta_thresholds: CombatDeltaThresholdConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulTierFactorConfig {
    pub mundane: f64,
    pub awakened: f64,
    pub adept: f64,
    pub master: f64,
    pub ascendant: f64,
    pub transcendent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatDeltaThresholdConfig {
    pub indistinguishable_abs_lt: f64,
    pub slight_abs_lt: f64,
    pub marked_abs_lt: f64,
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
            max_size_bytes: 1024 * 1024 * 1024,
            inactive_world_warning_days: 30,
        }
    }
}

impl Default for WorldMetadataSnapshot {
    fn default() -> Self {
        Self {
            display_name: "Unnamed World".to_string(),
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
                    max_value: Some(200.0),
                },
                AttributeTierBoundary {
                    tier: "Awakened".to_string(),
                    min_value: 200.0,
                    max_value: Some(1000.0),
                },
                AttributeTierBoundary {
                    tier: "Adept".to_string(),
                    min_value: 1000.0,
                    max_value: Some(1800.0),
                },
                AttributeTierBoundary {
                    tier: "Master".to_string(),
                    min_value: 1800.0,
                    max_value: Some(2600.0),
                },
                AttributeTierBoundary {
                    tier: "Ascendant".to_string(),
                    min_value: 2600.0,
                    max_value: Some(5600.0),
                },
                AttributeTierBoundary {
                    tier: "Transcendent".to_string(),
                    min_value: 5600.0,
                    max_value: None,
                },
            ],
            delta_thresholds: AttributeDeltaThresholdConfig::default(),
        }
    }
}

impl Default for AttributeDeltaThresholdConfig {
    fn default() -> Self {
        Self {
            indistinguishable_abs_lt: 150.0,
            slight_abs_lt: 300.0,
            notable_abs_lt: 1000.0,
            far_abs_lt: 2000.0,
        }
    }
}

impl Default for ManaExpressionConfig {
    fn default() -> Self {
        Self {
            display_ratio_clamp: (0.0, 2.0),
            tendency_factors: SnapshotTendencyFactors::default(),
            mode_factors: SnapshotModeFactors::default(),
            expression_modes: SnapshotExpressionModes::default(),
            concealment_suspected_gap: 200.0,
        }
    }
}

impl Default for SnapshotTendencyFactors {
    fn default() -> Self {
        Self {
            inward: -0.5,
            neutral: -0.2,
            expressive: 0.1,
        }
    }
}

impl Default for SnapshotModeFactors {
    fn default() -> Self {
        Self {
            sealed: -0.7,
            suppressed: -0.3,
            natural: 0.0,
            released: 0.2,
            dominating: 0.4,
        }
    }
}

impl Default for SnapshotExpressionModes {
    fn default() -> Self {
        Self {
            sealed: SnapshotExpressionMode {
                radius_tier: ManaPresenceRadiusTier::SelfOnly,
                pressure_multiplier: 0.0,
            },
            suppressed: SnapshotExpressionMode {
                radius_tier: ManaPresenceRadiusTier::Close,
                pressure_multiplier: 0.5,
            },
            natural: SnapshotExpressionMode {
                radius_tier: ManaPresenceRadiusTier::Room,
                pressure_multiplier: 1.0,
            },
            released: SnapshotExpressionMode {
                radius_tier: ManaPresenceRadiusTier::Area,
                pressure_multiplier: 1.15,
            },
            dominating: SnapshotExpressionMode {
                radius_tier: ManaPresenceRadiusTier::Scene,
                pressure_multiplier: 1.3,
            },
        }
    }
}

impl Default for CombatResolutionConfig {
    fn default() -> Self {
        Self {
            min_effectiveness: 0.1,
            soul_tier_factors: SoulTierFactorConfig::default(),
            soul_damage_floor: 0.2,
            delta_thresholds: CombatDeltaThresholdConfig::default(),
        }
    }
}

impl Default for SoulTierFactorConfig {
    fn default() -> Self {
        Self {
            mundane: 0.8,
            awakened: 0.9,
            adept: 1.0,
            master: 1.05,
            ascendant: 1.1,
            transcendent: 1.15,
        }
    }
}

impl Default for CombatDeltaThresholdConfig {
    fn default() -> Self {
        Self {
            indistinguishable_abs_lt: 150.0,
            slight_abs_lt: 300.0,
            marked_abs_lt: 1000.0,
        }
    }
}

impl RuntimeConfigSnapshot {
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
    pub fn from_world_argument(
        world_id: String,
        runtime_config_snapshot_id: String,
        source_paths: Vec<String>,
        config: &WorldArgumentConfig,
    ) -> Result<Self, String> {
        let world = WorldMetadataSnapshot {
            display_name: config.world.display_name.clone(),
        };
        let calendar = CalendarSnapshot {
            default_calendar_id: config.calendar.default_calendar_id.clone(),
            eras: config
                .calendar
                .eras
                .iter()
                .map(|era| CalendarEraSnapshot {
                    era_id: era.era_id.clone(),
                    display_name: era.display_name.clone(),
                    start_label: era.start_label.clone(),
                })
                .collect(),
        };
        let attribute_tiers = AttributeTierConfig {
            boundaries: vec![
                AttributeTierBoundary {
                    tier: "Mundane".to_string(),
                    min_value: config.attribute_rules.tier_thresholds.mundane.0,
                    max_value: config.attribute_rules.tier_thresholds.mundane.1,
                },
                AttributeTierBoundary {
                    tier: "Awakened".to_string(),
                    min_value: config.attribute_rules.tier_thresholds.awakened.0,
                    max_value: config.attribute_rules.tier_thresholds.awakened.1,
                },
                AttributeTierBoundary {
                    tier: "Adept".to_string(),
                    min_value: config.attribute_rules.tier_thresholds.adept.0,
                    max_value: config.attribute_rules.tier_thresholds.adept.1,
                },
                AttributeTierBoundary {
                    tier: "Master".to_string(),
                    min_value: config.attribute_rules.tier_thresholds.master.0,
                    max_value: config.attribute_rules.tier_thresholds.master.1,
                },
                AttributeTierBoundary {
                    tier: "Ascendant".to_string(),
                    min_value: config.attribute_rules.tier_thresholds.ascendant.0,
                    max_value: config.attribute_rules.tier_thresholds.ascendant.1,
                },
                AttributeTierBoundary {
                    tier: "Transcendent".to_string(),
                    min_value: config.attribute_rules.tier_thresholds.transcendent.0,
                    max_value: config.attribute_rules.tier_thresholds.transcendent.1,
                },
            ],
            delta_thresholds: AttributeDeltaThresholdConfig {
                indistinguishable_abs_lt: config
                    .attribute_rules
                    .delta_thresholds
                    .indistinguishable_abs_lt,
                slight_abs_lt: config.attribute_rules.delta_thresholds.slight_abs_lt,
                notable_abs_lt: config.attribute_rules.delta_thresholds.notable_abs_lt,
                far_abs_lt: config.attribute_rules.delta_thresholds.far_abs_lt,
            },
        };
        let mana_expression = ManaExpressionConfig {
            display_ratio_clamp: config.mana_rules.display_ratio_clamp,
            tendency_factors: SnapshotTendencyFactors {
                inward: config.mana_rules.tendency_factors.inward,
                neutral: config.mana_rules.tendency_factors.neutral,
                expressive: config.mana_rules.tendency_factors.expressive,
            },
            mode_factors: SnapshotModeFactors {
                sealed: config.mana_rules.mode_factors.sealed,
                suppressed: config.mana_rules.mode_factors.suppressed,
                natural: config.mana_rules.mode_factors.natural,
                released: config.mana_rules.mode_factors.released,
                dominating: config.mana_rules.mode_factors.dominating,
            },
            expression_modes: SnapshotExpressionModes {
                sealed: SnapshotExpressionMode {
                    radius_tier: parse_radius_tier(&config.mana_rules.expression_modes.sealed.radius)?,
                    pressure_multiplier: config
                        .mana_rules
                        .expression_modes
                        .sealed
                        .pressure_multiplier,
                },
                suppressed: SnapshotExpressionMode {
                    radius_tier: parse_radius_tier(
                        &config.mana_rules.expression_modes.suppressed.radius,
                    )?,
                    pressure_multiplier: config
                        .mana_rules
                        .expression_modes
                        .suppressed
                        .pressure_multiplier,
                },
                natural: SnapshotExpressionMode {
                    radius_tier: parse_radius_tier(&config.mana_rules.expression_modes.natural.radius)?,
                    pressure_multiplier: config
                        .mana_rules
                        .expression_modes
                        .natural
                        .pressure_multiplier,
                },
                released: SnapshotExpressionMode {
                    radius_tier: parse_radius_tier(&config.mana_rules.expression_modes.released.radius)?,
                    pressure_multiplier: config
                        .mana_rules
                        .expression_modes
                        .released
                        .pressure_multiplier,
                },
                dominating: SnapshotExpressionMode {
                    radius_tier: parse_radius_tier(
                        &config.mana_rules.expression_modes.dominating.radius,
                    )?,
                    pressure_multiplier: config
                        .mana_rules
                        .expression_modes
                        .dominating
                        .pressure_multiplier,
                },
            },
            concealment_suspected_gap: config.mana_rules.concealment_suspected_gap,
        };
        let combat_resolution = CombatResolutionConfig {
            min_effectiveness: config.combat_rules.min_effectiveness,
            soul_tier_factors: SoulTierFactorConfig {
                mundane: config.combat_rules.soul_tier_factors.mundane,
                awakened: config.combat_rules.soul_tier_factors.awakened,
                adept: config.combat_rules.soul_tier_factors.adept,
                master: config.combat_rules.soul_tier_factors.master,
                ascendant: config.combat_rules.soul_tier_factors.ascendant,
                transcendent: config.combat_rules.soul_tier_factors.transcendent,
            },
            soul_damage_floor: config.combat_rules.soul_damage_floor,
            delta_thresholds: CombatDeltaThresholdConfig {
                indistinguishable_abs_lt: config
                    .combat_rules
                    .delta_thresholds
                    .indistinguishable_abs_lt,
                slight_abs_lt: config.combat_rules.delta_thresholds.slight_abs_lt,
                marked_abs_lt: config.combat_rules.delta_thresholds.marked_abs_lt,
            },
        };
        let config_hash = Self::compute_hash(
            config.schema_version,
            &world,
            &calendar,
            &attribute_tiers,
            &mana_expression,
            &combat_resolution,
        )?;

        Ok(Self {
            snapshot_id: generate_snapshot_id("wrules"),
            runtime_config_snapshot_id,
            world_id,
            schema_version: config.schema_version,
            config_hash,
            source_paths,
            world,
            calendar,
            attribute_tiers,
            mana_expression,
            combat_resolution,
            created_at: Utc::now(),
        })
    }

    fn compute_hash(
        schema_version: u32,
        world: &WorldMetadataSnapshot,
        calendar: &CalendarSnapshot,
        attribute_tiers: &AttributeTierConfig,
        mana_expression: &ManaExpressionConfig,
        combat_resolution: &CombatResolutionConfig,
    ) -> Result<String, String> {
        let json = serde_json::json!({
            "schema_version": schema_version,
            "world": world,
            "calendar": calendar,
            "attribute_tiers": attribute_tiers,
            "mana_expression": mana_expression,
            "combat_resolution": combat_resolution,
        });
        let encoded =
            serde_json::to_vec(&json).map_err(|e| format!("Failed to hash world rules: {}", e))?;
        let mut hasher = Sha256::new();
        hasher.update(encoded);
        Ok(format!("{:x}", hasher.finalize())[..16].to_string())
    }

    pub fn attribute_tier_for_value(&self, value: f64) -> AttributeTier {
        if value < self.boundary_max(0) {
            AttributeTier::Mundane
        } else if value < self.boundary_max(1) {
            AttributeTier::Awakened
        } else if value < self.boundary_max(2) {
            AttributeTier::Adept
        } else if value < self.boundary_max(3) {
            AttributeTier::Master
        } else if value < self.boundary_max(4) {
            AttributeTier::Ascendant
        } else {
            AttributeTier::Transcendent
        }
    }

    pub fn attribute_delta_for_difference(&self, delta: f64) -> AttributeDelta {
        let abs = delta.abs();
        let thresholds = &self.attribute_tiers.delta_thresholds;
        let level = if abs < thresholds.indistinguishable_abs_lt {
            0
        } else if abs < thresholds.slight_abs_lt {
            1
        } else if abs < thresholds.notable_abs_lt {
            2
        } else if abs < thresholds.far_abs_lt {
            3
        } else {
            4
        };

        match (delta.is_sign_negative(), level) {
            (_, 0) => AttributeDelta::Indistinguishable,
            (true, 1) => AttributeDelta::SlightlyBelow,
            (true, 2) => AttributeDelta::NotablyBelow,
            (true, 3) => AttributeDelta::FarBelow,
            (true, _) => AttributeDelta::Crushed,
            (false, 1) => AttributeDelta::SlightlyAbove,
            (false, 2) => AttributeDelta::NotablyAbove,
            (false, 3) => AttributeDelta::FarAbove,
            (false, _) => AttributeDelta::Overwhelming,
        }
    }

    fn boundary_max(&self, index: usize) -> f64 {
        self.attribute_tiers
            .boundaries
            .get(index)
            .and_then(|boundary| boundary.max_value)
            .unwrap_or(f64::MAX)
    }
}

fn parse_radius_tier(raw: &str) -> Result<ManaPresenceRadiusTier, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "self_only" | "selfonly" => Ok(ManaPresenceRadiusTier::SelfOnly),
        "touch" => Ok(ManaPresenceRadiusTier::Touch),
        "close" => Ok(ManaPresenceRadiusTier::Close),
        "room" => Ok(ManaPresenceRadiusTier::Room),
        "area" => Ok(ManaPresenceRadiusTier::Area),
        "scene" => Ok(ManaPresenceRadiusTier::Scene),
        other => Err(format!("Unsupported mana expression radius: {}", other)),
    }
}

fn generate_snapshot_id(prefix: &str) -> String {
    format!("{}_{}", prefix, uuid::Uuid::new_v4())
}

#[derive(Debug, Clone, Default)]
pub struct SnapshotManager {
    current_runtime_snapshot: Option<RuntimeConfigSnapshot>,
    current_world_snapshots: std::collections::HashMap<String, WorldRulesSnapshot>,
}

impl SnapshotManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn capture_runtime_snapshot(&mut self, source_paths: Vec<String>) -> RuntimeConfigSnapshot {
        let snapshot = RuntimeConfigSnapshot::new(source_paths);
        self.current_runtime_snapshot = Some(snapshot.clone());
        snapshot
    }

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

    pub fn capture_world_snapshot(
        &mut self,
        world_id: String,
        source_paths: Vec<String>,
        config: &WorldArgumentConfig,
    ) -> Result<WorldRulesSnapshot, String> {
        let runtime_snapshot_id = self
            .current_runtime_snapshot
            .as_ref()
            .map(|s| s.snapshot_id.clone())
            .ok_or_else(|| "No runtime config snapshot captured".to_string())?;

        let snapshot = WorldRulesSnapshot::from_world_argument(
            world_id.clone(),
            runtime_snapshot_id,
            source_paths,
            config,
        )?;
        self.current_world_snapshots
            .insert(world_id, snapshot.clone());
        Ok(snapshot)
    }

    pub fn current_runtime(&self) -> Option<&RuntimeConfigSnapshot> {
        self.current_runtime_snapshot.as_ref()
    }

    pub fn current_world(&self, world_id: &str) -> Option<&WorldRulesSnapshot> {
        self.current_world_snapshots.get(world_id)
    }

    pub fn runtime_snapshot_id(&self) -> Option<&str> {
        self.current_runtime_snapshot
            .as_ref()
            .map(|s| s.snapshot_id.as_str())
    }

    pub fn world_snapshot_id(&self, world_id: &str) -> Option<&str> {
        self.current_world_snapshots
            .get(world_id)
            .map(|s| s.snapshot_id.as_str())
    }

    pub fn clear(&mut self) {
        self.current_runtime_snapshot = None;
        self.current_world_snapshots.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{RuntimeConfigSnapshot, SnapshotManager, WorldRulesSnapshot};
    use crate::config::world_argument::WorldArgumentConfig;

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
        let world = WorldRulesSnapshot::from_world_argument(
            "world_1".to_string(),
            runtime.snapshot_id.clone(),
            vec!["world_argument.yaml".to_string()],
            &WorldArgumentConfig::default(),
        )
        .expect("world snapshot");

        assert!(!world.snapshot_id.is_empty());
        assert_eq!(world.world_id, "world_1");
        assert_eq!(world.runtime_config_snapshot_id, runtime.snapshot_id);
        assert_eq!(world.world.display_name, "Unnamed World");
    }

    #[test]
    fn manages_snapshots() {
        let mut manager = SnapshotManager::new();

        let _runtime = manager.capture_runtime_snapshot(vec!["app_runtime.yaml".to_string()]);
        assert!(manager.current_runtime().is_some());

        let world = manager.capture_world_snapshot(
            "world_1".to_string(),
            vec!["world_argument.yaml".to_string()],
            &WorldArgumentConfig::default(),
        );
        assert!(world.is_ok());
        assert!(manager.current_world("world_1").is_some());
    }

    #[test]
    fn computes_effective_max_context() {
        let snapshot = RuntimeConfigSnapshot::new(vec![]);

        assert_eq!(snapshot.effective_max_context(None), 32768);
        assert_eq!(snapshot.effective_max_context(Some(40000)), 32768);
        assert_eq!(snapshot.effective_max_context(Some(20000)), 15904);
    }
}
