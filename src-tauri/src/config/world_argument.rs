//! World argument configuration for Agent worlds.
//!
//! This file defines the on-disk YAML schema for world-specific runtime rules.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const WORLD_ARGUMENT_FILE_NAME: &str = "world_argument.yaml";
pub const LEGACY_WORLD_ARGUMENT_FILE_NAME: &str = "world_base.yaml";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorldArgumentConfig {
    pub schema_version: u32,
    pub world: WorldMetadataConfig,
    pub calendar: CalendarConfig,
    pub attribute_rules: AttributeRulesConfig,
    pub mana_rules: ManaRulesConfig,
    pub combat_rules: CombatRulesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorldMetadataConfig {
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CalendarConfig {
    pub default_calendar_id: String,
    pub eras: Vec<CalendarEraConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CalendarEraConfig {
    pub era_id: String,
    pub display_name: String,
    pub start_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AttributeRulesConfig {
    pub tier_thresholds: AttributeTierThresholds,
    pub delta_thresholds: AttributeDeltaThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AttributeTierThresholds {
    pub mundane: (f64, Option<f64>),
    pub awakened: (f64, Option<f64>),
    pub adept: (f64, Option<f64>),
    pub master: (f64, Option<f64>),
    pub ascendant: (f64, Option<f64>),
    pub transcendent: (f64, Option<f64>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AttributeDeltaThresholds {
    pub indistinguishable_abs_lt: f64,
    pub slight_abs_lt: f64,
    pub notable_abs_lt: f64,
    pub far_abs_lt: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ManaRulesConfig {
    pub display_ratio_clamp: (f64, f64),
    pub tendency_factors: ManaTendencyFactorsConfig,
    pub mode_factors: ManaModeFactorsConfig,
    pub expression_modes: ManaExpressionModesConfig,
    pub concealment_suspected_gap: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ManaTendencyFactorsConfig {
    pub inward: f64,
    pub neutral: f64,
    pub expressive: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ManaModeFactorsConfig {
    pub sealed: f64,
    pub suppressed: f64,
    pub natural: f64,
    pub released: f64,
    pub dominating: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ManaExpressionModesConfig {
    pub sealed: ManaExpressionModeConfig,
    pub suppressed: ManaExpressionModeConfig,
    pub natural: ManaExpressionModeConfig,
    pub released: ManaExpressionModeConfig,
    pub dominating: ManaExpressionModeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ManaExpressionModeConfig {
    pub radius: String,
    pub pressure_multiplier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CombatRulesConfig {
    pub delta_thresholds: CombatDeltaThresholdsConfig,
    pub min_effectiveness: f64,
    pub soul_tier_factors: SoulTierFactorsConfig,
    pub soul_damage_floor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CombatDeltaThresholdsConfig {
    pub indistinguishable_abs_lt: f64,
    pub slight_abs_lt: f64,
    pub marked_abs_lt: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoulTierFactorsConfig {
    pub mundane: f64,
    pub awakened: f64,
    pub adept: f64,
    pub master: f64,
    pub ascendant: f64,
    pub transcendent: f64,
}

impl Default for WorldArgumentConfig {
    fn default() -> Self {
        Self {
            schema_version: 1,
            world: WorldMetadataConfig::default(),
            calendar: CalendarConfig::default(),
            attribute_rules: AttributeRulesConfig::default(),
            mana_rules: ManaRulesConfig::default(),
            combat_rules: CombatRulesConfig::default(),
        }
    }
}

impl Default for WorldMetadataConfig {
    fn default() -> Self {
        Self {
            display_name: "Unnamed World".to_string(),
        }
    }
}

impl Default for CalendarConfig {
    fn default() -> Self {
        Self {
            default_calendar_id: String::new(),
            eras: Vec::new(),
        }
    }
}

impl Default for CalendarEraConfig {
    fn default() -> Self {
        Self {
            era_id: String::new(),
            display_name: String::new(),
            start_label: String::new(),
        }
    }
}

impl Default for AttributeRulesConfig {
    fn default() -> Self {
        Self {
            tier_thresholds: AttributeTierThresholds::default(),
            delta_thresholds: AttributeDeltaThresholds::default(),
        }
    }
}

impl Default for AttributeTierThresholds {
    fn default() -> Self {
        Self {
            mundane: (0.0, Some(200.0)),
            awakened: (200.0, Some(1000.0)),
            adept: (1000.0, Some(1800.0)),
            master: (1800.0, Some(2600.0)),
            ascendant: (2600.0, Some(5600.0)),
            transcendent: (5600.0, None),
        }
    }
}

impl Default for AttributeDeltaThresholds {
    fn default() -> Self {
        Self {
            indistinguishable_abs_lt: 150.0,
            slight_abs_lt: 300.0,
            notable_abs_lt: 1000.0,
            far_abs_lt: 2000.0,
        }
    }
}

impl Default for ManaRulesConfig {
    fn default() -> Self {
        Self {
            display_ratio_clamp: (0.0, 2.0),
            tendency_factors: ManaTendencyFactorsConfig::default(),
            mode_factors: ManaModeFactorsConfig::default(),
            expression_modes: ManaExpressionModesConfig::default(),
            concealment_suspected_gap: 200.0,
        }
    }
}

impl Default for ManaTendencyFactorsConfig {
    fn default() -> Self {
        Self {
            inward: -0.5,
            neutral: -0.2,
            expressive: 0.1,
        }
    }
}

impl Default for ManaModeFactorsConfig {
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

impl Default for ManaExpressionModesConfig {
    fn default() -> Self {
        Self {
            sealed: ManaExpressionModeConfig {
                radius: "self_only".to_string(),
                pressure_multiplier: 0.0,
            },
            suppressed: ManaExpressionModeConfig {
                radius: "close".to_string(),
                pressure_multiplier: 0.5,
            },
            natural: ManaExpressionModeConfig {
                radius: "room".to_string(),
                pressure_multiplier: 1.0,
            },
            released: ManaExpressionModeConfig {
                radius: "area".to_string(),
                pressure_multiplier: 1.15,
            },
            dominating: ManaExpressionModeConfig {
                radius: "scene".to_string(),
                pressure_multiplier: 1.3,
            },
        }
    }
}

impl Default for ManaExpressionModeConfig {
    fn default() -> Self {
        Self {
            radius: "room".to_string(),
            pressure_multiplier: 1.0,
        }
    }
}

impl Default for CombatRulesConfig {
    fn default() -> Self {
        Self {
            delta_thresholds: CombatDeltaThresholdsConfig::default(),
            min_effectiveness: 0.1,
            soul_tier_factors: SoulTierFactorsConfig::default(),
            soul_damage_floor: 0.2,
        }
    }
}

impl Default for CombatDeltaThresholdsConfig {
    fn default() -> Self {
        Self {
            indistinguishable_abs_lt: 150.0,
            slight_abs_lt: 300.0,
            marked_abs_lt: 1000.0,
        }
    }
}

impl Default for SoulTierFactorsConfig {
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

impl WorldArgumentConfig {
    pub fn with_world_name(world_name: &str) -> Self {
        let mut config = Self::default();
        config.world.display_name = world_name.trim().to_string();
        config
    }
}

pub fn default_world_argument_yaml(world_name: &str) -> Result<String, String> {
    serde_yaml::to_string(&WorldArgumentConfig::with_world_name(world_name))
        .map_err(|e| format!("Failed to serialize default world_argument.yaml: {}", e))
}

pub fn validate_world_argument_config(config: &WorldArgumentConfig) -> Result<(), String> {
    if config.schema_version != 1 {
        return Err(format!(
            "Unsupported world argument schema_version: {}",
            config.schema_version
        ));
    }

    validate_range("attribute_rules.tier_thresholds.mundane", config.attribute_rules.tier_thresholds.mundane)?;
    validate_range("attribute_rules.tier_thresholds.awakened", config.attribute_rules.tier_thresholds.awakened)?;
    validate_range("attribute_rules.tier_thresholds.adept", config.attribute_rules.tier_thresholds.adept)?;
    validate_range("attribute_rules.tier_thresholds.master", config.attribute_rules.tier_thresholds.master)?;
    validate_range("attribute_rules.tier_thresholds.ascendant", config.attribute_rules.tier_thresholds.ascendant)?;
    validate_range("attribute_rules.tier_thresholds.transcendent", config.attribute_rules.tier_thresholds.transcendent)?;

    let tier_ranges = [
        config.attribute_rules.tier_thresholds.mundane,
        config.attribute_rules.tier_thresholds.awakened,
        config.attribute_rules.tier_thresholds.adept,
        config.attribute_rules.tier_thresholds.master,
        config.attribute_rules.tier_thresholds.ascendant,
        config.attribute_rules.tier_thresholds.transcendent,
    ];
    for pair in tier_ranges.windows(2) {
        let left = pair[0];
        let right = pair[1];
        let left_max = left.1.ok_or_else(|| {
            "Only the final attribute tier may have an open upper bound".to_string()
        })?;
        if (left_max - right.0).abs() > f64::EPSILON {
            return Err("attribute tier thresholds must be contiguous".to_string());
        }
    }

    validate_ascending(
        "attribute_rules.delta_thresholds",
        &[
            config.attribute_rules.delta_thresholds.indistinguishable_abs_lt,
            config.attribute_rules.delta_thresholds.slight_abs_lt,
            config.attribute_rules.delta_thresholds.notable_abs_lt,
            config.attribute_rules.delta_thresholds.far_abs_lt,
        ],
    )?;

    if !config.mana_rules.display_ratio_clamp.0.is_finite()
        || !config.mana_rules.display_ratio_clamp.1.is_finite()
        || config.mana_rules.display_ratio_clamp.0 < 0.0
        || config.mana_rules.display_ratio_clamp.0 > config.mana_rules.display_ratio_clamp.1
    {
        return Err("mana_rules.display_ratio_clamp must be a finite non-negative ascending pair".to_string());
    }

    for (field, value) in [
        ("mana_rules.tendency_factors.inward", config.mana_rules.tendency_factors.inward),
        ("mana_rules.tendency_factors.neutral", config.mana_rules.tendency_factors.neutral),
        ("mana_rules.tendency_factors.expressive", config.mana_rules.tendency_factors.expressive),
        ("mana_rules.mode_factors.sealed", config.mana_rules.mode_factors.sealed),
        ("mana_rules.mode_factors.suppressed", config.mana_rules.mode_factors.suppressed),
        ("mana_rules.mode_factors.natural", config.mana_rules.mode_factors.natural),
        ("mana_rules.mode_factors.released", config.mana_rules.mode_factors.released),
        ("mana_rules.mode_factors.dominating", config.mana_rules.mode_factors.dominating),
        (
            "mana_rules.concealment_suspected_gap",
            config.mana_rules.concealment_suspected_gap,
        ),
        ("combat_rules.min_effectiveness", config.combat_rules.min_effectiveness),
        ("combat_rules.soul_damage_floor", config.combat_rules.soul_damage_floor),
    ] {
        if !value.is_finite() {
            return Err(format!("{field} must be finite"));
        }
    }

    for (field, mode) in [
        ("mana_rules.expression_modes.sealed", &config.mana_rules.expression_modes.sealed),
        (
            "mana_rules.expression_modes.suppressed",
            &config.mana_rules.expression_modes.suppressed,
        ),
        ("mana_rules.expression_modes.natural", &config.mana_rules.expression_modes.natural),
        ("mana_rules.expression_modes.released", &config.mana_rules.expression_modes.released),
        (
            "mana_rules.expression_modes.dominating",
            &config.mana_rules.expression_modes.dominating,
        ),
    ] {
        if mode.radius.trim().is_empty() {
            return Err(format!("{field}.radius must not be empty"));
        }
        if !mode.pressure_multiplier.is_finite() || mode.pressure_multiplier < 0.0 {
            return Err(format!("{field}.pressure_multiplier must be finite and >= 0"));
        }
    }

    validate_ascending(
        "combat_rules.delta_thresholds",
        &[
            config.combat_rules.delta_thresholds.indistinguishable_abs_lt,
            config.combat_rules.delta_thresholds.slight_abs_lt,
            config.combat_rules.delta_thresholds.marked_abs_lt,
        ],
    )?;

    for (field, value) in [
        (
            "combat_rules.soul_tier_factors.mundane",
            config.combat_rules.soul_tier_factors.mundane,
        ),
        (
            "combat_rules.soul_tier_factors.awakened",
            config.combat_rules.soul_tier_factors.awakened,
        ),
        (
            "combat_rules.soul_tier_factors.adept",
            config.combat_rules.soul_tier_factors.adept,
        ),
        (
            "combat_rules.soul_tier_factors.master",
            config.combat_rules.soul_tier_factors.master,
        ),
        (
            "combat_rules.soul_tier_factors.ascendant",
            config.combat_rules.soul_tier_factors.ascendant,
        ),
        (
            "combat_rules.soul_tier_factors.transcendent",
            config.combat_rules.soul_tier_factors.transcendent,
        ),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("{field} must be finite and > 0"));
        }
    }

    Ok(())
}

pub fn parse_world_argument_yaml(yaml_text: &str) -> Result<WorldArgumentConfig, String> {
    let config: WorldArgumentConfig = serde_yaml::from_str(yaml_text)
        .map_err(|e| format!("Failed to parse world_argument.yaml: {}", e))?;
    validate_world_argument_config(&config)?;
    Ok(config)
}

pub fn load_world_argument_config(path: &Path) -> Result<WorldArgumentConfig, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    parse_world_argument_yaml(&text)
}

pub fn ensure_world_argument_file(
    world_dir: &Path,
    world_name: &str,
) -> Result<PathBuf, String> {
    let path = world_dir.join(WORLD_ARGUMENT_FILE_NAME);
    if path.exists() {
        return Ok(path);
    }

    let legacy_path = world_dir.join(LEGACY_WORLD_ARGUMENT_FILE_NAME);
    if legacy_path.exists() {
        std::fs::rename(&legacy_path, &path).map_err(|e| {
            format!(
                "Failed to migrate {} to {}: {}",
                legacy_path.display(),
                path.display(),
                e
            )
        })?;
        return Ok(path);
    }

    let yaml = default_world_argument_yaml(world_name)?;
    std::fs::write(&path, yaml)
        .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
    Ok(path)
}

pub fn load_world_argument_from_dir(world_dir: &Path) -> Result<(WorldArgumentConfig, PathBuf), String> {
    let path = world_dir.join(WORLD_ARGUMENT_FILE_NAME);
    if path.exists() {
        let config = load_world_argument_config(&path)?;
        return Ok((config, path));
    }

    let legacy_path = world_dir.join(LEGACY_WORLD_ARGUMENT_FILE_NAME);
    if legacy_path.exists() {
        let config = load_world_argument_config(&legacy_path)?;
        let yaml = serde_yaml::to_string(&config)
            .map_err(|e| format!("Failed to serialize migrated world arguments: {}", e))?;
        std::fs::write(&path, yaml)
            .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
        std::fs::remove_file(&legacy_path).map_err(|e| {
            format!(
                "Failed to remove legacy {} after migration: {}",
                legacy_path.display(),
                e
            )
        })?;
        return Ok((config, path));
    }

    Err(format!(
        "Missing {} in {}",
        WORLD_ARGUMENT_FILE_NAME,
        world_dir.display()
    ))
}

fn validate_range(field: &str, range: (f64, Option<f64>)) -> Result<(), String> {
    if !range.0.is_finite() {
        return Err(format!("{field}[0] must be finite"));
    }
    if let Some(max) = range.1 {
        if !max.is_finite() {
            return Err(format!("{field}[1] must be finite or null"));
        }
        if range.0 >= max {
            return Err(format!("{field} lower bound must be < upper bound"));
        }
    }
    Ok(())
}

fn validate_ascending(field: &str, values: &[f64]) -> Result<(), String> {
    let mut prev = None::<f64>;
    for value in values {
        if !value.is_finite() || *value <= 0.0 {
            return Err(format!("{field} values must be finite and > 0"));
        }
        if let Some(prev_value) = prev {
            if *value <= prev_value {
                return Err(format!("{field} values must be strictly ascending"));
            }
        }
        prev = Some(*value);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        default_world_argument_yaml, ensure_world_argument_file, load_world_argument_from_dir,
        parse_world_argument_yaml, LEGACY_WORLD_ARGUMENT_FILE_NAME, WORLD_ARGUMENT_FILE_NAME,
    };

    #[test]
    fn parses_default_yaml() {
        let yaml = default_world_argument_yaml("Test World").expect("default yaml");
        let config = parse_world_argument_yaml(&yaml).expect("parse yaml");
        assert_eq!(config.world.display_name, "Test World");
        assert_eq!(config.attribute_rules.delta_thresholds.slight_abs_lt, 300.0);
    }

    #[test]
    fn migrates_legacy_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let legacy_path = dir.path().join(LEGACY_WORLD_ARGUMENT_FILE_NAME);
        std::fs::write(
            &legacy_path,
            default_world_argument_yaml("Legacy World").expect("legacy yaml"),
        )
        .expect("write legacy file");

        let (config, path) = load_world_argument_from_dir(dir.path()).expect("load migrated file");
        assert_eq!(config.world.display_name, "Legacy World");
        assert_eq!(path.file_name().and_then(|s| s.to_str()), Some(WORLD_ARGUMENT_FILE_NAME));
        assert!(!legacy_path.exists());
        assert!(path.exists());
    }

    #[test]
    fn creates_default_file_when_missing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = ensure_world_argument_file(dir.path(), "New World").expect("ensure file");
        let text = std::fs::read_to_string(path).expect("read file");
        assert!(text.contains("display_name: New World"));
    }
}
