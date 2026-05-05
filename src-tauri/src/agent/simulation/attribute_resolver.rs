//! Attribute resolver
//!
//! Derives effective attributes and tier/delta from base attributes.
//!
//! Performance optimization: Supports caching via TurnScopedCache to avoid
//! redundant calculations within a turn.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::agent::cache::{DerivedAttributeKey, TurnScopedCache};
use crate::agent::models::{
    AttributeDelta, AttributeKind, AttributeTier, BaseAttributes, EffectiveAttributeProfile,
    InjurySeverity, ManaExpressionMode, ManaExpressionProfile, ManaExpressionState,
    ManaExpressionTendency, ManaPresenceRadiusTier, TemporaryCharacterState,
};

/// Attribute resolver - derives effective attributes
pub struct AttributeResolver {
    /// Optional cache for performance optimization
    cache: Option<Arc<TurnScopedCache>>,
}

impl AttributeResolver {
    /// Create a new resolver without caching
    pub fn new() -> Self {
        Self { cache: None }
    }

    /// Create a new resolver with caching enabled
    pub fn with_cache(cache: Arc<TurnScopedCache>) -> Self {
        Self { cache: Some(cache) }
    }

    /// Compute a hash for base attributes
    fn hash_base_attributes(base: &BaseAttributes) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        base.physical.to_bits().hash(&mut hasher);
        base.agility.to_bits().hash(&mut hasher);
        base.endurance.to_bits().hash(&mut hasher);
        base.insight.to_bits().hash(&mut hasher);
        base.mana_power.to_bits().hash(&mut hasher);
        base.soul_strength.to_bits().hash(&mut hasher);
        hasher.finish()
    }

    /// Compute a hash for temporary state
    fn hash_temp_state(temp_state: &TemporaryCharacterState) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        temp_state.fatigue.to_bits().hash(&mut hasher);
        temp_state.pain_load.to_bits().hash(&mut hasher);
        temp_state.injuries.len().hash(&mut hasher);
        hasher.finish()
    }

    /// Derive effective attributes from base + temporary state
    pub fn derive_effective_attributes(
        &self,
        character_id: &str,
        base: &BaseAttributes,
        temp_state: &TemporaryCharacterState,
        scene_modifiers: &[AttributeModifier],
    ) -> EffectiveAttributeProfile {
        // Check cache
        if let Some(ref cache) = self.cache {
            let key = DerivedAttributeKey {
                character_id: character_id.to_string(),
                base_hash: Self::hash_base_attributes(base),
                temp_state_hash: Self::hash_temp_state(temp_state),
                modifiers_hash: 0, // Simplified for now
            };
            if let Some(cached) = cache.get_attributes(&key) {
                return (*cached).clone();
            }
        }

        // Compute
        let result = self.derive_effective_attributes_uncached(base, temp_state, scene_modifiers);

        // Cache result
        if let Some(ref cache) = self.cache {
            let key = DerivedAttributeKey {
                character_id: character_id.to_string(),
                base_hash: Self::hash_base_attributes(base),
                temp_state_hash: Self::hash_temp_state(temp_state),
                modifiers_hash: 0,
            };
            cache.insert_attributes(key, Arc::new(result.clone()));
        }

        result
    }

    /// Internal implementation without caching
    fn derive_effective_attributes_uncached(
        &self,
        base: &BaseAttributes,
        temp_state: &TemporaryCharacterState,
        scene_modifiers: &[AttributeModifier],
    ) -> EffectiveAttributeProfile {
        let mut values = HashMap::new();

        // Base values
        values.insert(AttributeKind::Physical, base.physical);
        values.insert(AttributeKind::Agility, base.agility);
        values.insert(AttributeKind::Endurance, base.endurance);
        values.insert(AttributeKind::Insight, base.insight);
        values.insert(AttributeKind::ManaPower, base.mana_power);
        values.insert(AttributeKind::SoulStrength, base.soul_strength);

        // Apply injury modifiers
        for injury in &temp_state.injuries {
            let penalty = injury_severity_to_penalty(&injury.severity);
            // Apply penalty based on body region
            match injury.body_region.as_str() {
                "head" | "torso" => {
                    // Affects all attributes slightly
                    for value in values.values_mut() {
                        *value = (*value * (1.0 - penalty * 0.3)).max(0.0);
                    }
                }
                "arm" | "hand" => {
                    // Affects physical and agility
                    if let Some(v) = values.get_mut(&AttributeKind::Physical) {
                        *v = (*v * (1.0 - penalty)).max(0.0);
                    }
                    if let Some(v) = values.get_mut(&AttributeKind::Agility) {
                        *v = (*v * (1.0 - penalty)).max(0.0);
                    }
                }
                "leg" | "foot" => {
                    // Affects agility and endurance
                    if let Some(v) = values.get_mut(&AttributeKind::Agility) {
                        *v = (*v * (1.0 - penalty)).max(0.0);
                    }
                    if let Some(v) = values.get_mut(&AttributeKind::Endurance) {
                        *v = (*v * (1.0 - penalty)).max(0.0);
                    }
                }
                _ => {
                    // Default: affects physical attributes
                    if let Some(v) = values.get_mut(&AttributeKind::Physical) {
                        *v = (*v * (1.0 - penalty)).max(0.0);
                    }
                }
            }
        }

        // Apply fatigue
        if temp_state.fatigue > 0.0 {
            let fatigue_penalty = (temp_state.fatigue * 0.1).min(0.5); // Max 50% penalty
            for value in values.values_mut() {
                *value = (*value * (1.0 - fatigue_penalty)).max(0.0);
            }
        }

        // Apply pain
        if temp_state.pain_load > 0.0 {
            let pain_penalty = (temp_state.pain_load * 0.05).min(0.3); // Max 30% penalty
            if let Some(v) = values.get_mut(&AttributeKind::Physical) {
                *v = (*v * (1.0 - pain_penalty)).max(0.0);
            }
            if let Some(v) = values.get_mut(&AttributeKind::Agility) {
                *v = (*v * (1.0 - pain_penalty)).max(0.0);
            }
            // Insight also affected by pain (distraction)
            if let Some(v) = values.get_mut(&AttributeKind::Insight) {
                *v = (*v * (1.0 - pain_penalty * 0.5)).max(0.0);
            }
        }

        // Apply scene modifiers
        for modifier in scene_modifiers {
            if let Some(value) = values.get_mut(&modifier.kind) {
                *value = (*value + modifier.flat_delta).max(0.0);
                *value = (*value * (1.0 + modifier.ratio_modifier)).max(0.0);
            }
        }

        // Ensure minimum 0.0
        for value in values.values_mut() {
            *value = value.max(0.0);
        }

        // Derive tiers
        let mut tiers = HashMap::new();
        for (kind, value) in &values {
            tiers.insert(*kind, AttributeTier::from_value(*value));
        }

        // Generate descriptors
        let descriptors = Self::generate_descriptors(&values, &tiers);

        EffectiveAttributeProfile {
            character_id: String::new(), // Set by caller
            values,
            tiers,
            descriptors,
        }
    }

    /// Generate LLM-readable descriptors for attributes
    fn generate_descriptors(
        values: &HashMap<AttributeKind, f64>,
        tiers: &HashMap<AttributeKind, AttributeTier>,
    ) -> HashMap<AttributeKind, Vec<String>> {
        let mut descriptors = HashMap::new();

        for (kind, tier) in tiers {
            let mut desc = Vec::new();

            // Tier-based descriptor
            desc.push(format!("{:?} tier", tier));

            // Value-based nuance
            if let Some(value) = values.get(kind) {
                if *value < 100.0 {
                    desc.push("凡人水平".to_string());
                } else if *value < 500.0 {
                    desc.push("初窥门径".to_string());
                } else if *value < 1000.0 {
                    desc.push("小有所成".to_string());
                } else if *value < 1800.0 {
                    desc.push("登堂入室".to_string());
                } else if *value < 2600.0 {
                    desc.push("炉火纯青".to_string());
                } else if *value < 5600.0 {
                    desc.push("登峰造极".to_string());
                } else {
                    desc.push("超凡入圣".to_string());
                }
            }

            // Kind-specific descriptors
            match kind {
                AttributeKind::Physical => {
                    desc.push("肉身力量".to_string());
                }
                AttributeKind::Agility => {
                    desc.push("身法敏捷".to_string());
                }
                AttributeKind::Endurance => {
                    desc.push("体魄耐力".to_string());
                }
                AttributeKind::Insight => {
                    desc.push("洞察感知".to_string());
                }
                AttributeKind::ManaPower => {
                    desc.push("灵力修为".to_string());
                }
                AttributeKind::SoulStrength => {
                    desc.push("神魂根基".to_string());
                }
            }

            descriptors.insert(*kind, desc);
        }

        descriptors
    }

    /// Derive mana expression profile
    pub fn derive_mana_expression(
        tendency: ManaExpressionTendency,
        tendency_factor_override: Option<f64>,
        state: &ManaExpressionState,
        world_tendency_factors: &TendencyFactors,
        world_mode_factors: &ModeFactors,
    ) -> ManaExpressionProfile {
        // Get tendency factor
        let tendency_factor = tendency_factor_override.unwrap_or_else(|| match tendency {
            ManaExpressionTendency::Inward => world_tendency_factors.inward,
            ManaExpressionTendency::Neutral => world_tendency_factors.neutral,
            ManaExpressionTendency::Expressive => world_tendency_factors.expressive,
        });

        // Get mode factor
        let mode_factor = match state.mode {
            ManaExpressionMode::Sealed => world_mode_factors.sealed,
            ManaExpressionMode::Suppressed => world_mode_factors.suppressed,
            ManaExpressionMode::Natural => world_mode_factors.natural,
            ManaExpressionMode::Released => world_mode_factors.released,
            ManaExpressionMode::Dominating => world_mode_factors.dominating,
        };

        // Calculate display ratio
        let display_ratio = (1.0 + tendency_factor + mode_factor).clamp(0.0, 2.0);

        // Calculate pressure ratio (can differ from display for dominating)
        let pressure_ratio = match state.mode {
            ManaExpressionMode::Dominating => display_ratio * 1.2, // Extra pressure
            _ => display_ratio,
        };

        ManaExpressionProfile {
            character_id: String::new(),
            baseline_tendency: tendency,
            mode: state.mode,
            intentionality: state.intentionality,
            tendency_factor,
            mode_factor,
            display_ratio,
            pressure_ratio,
            radius_tier: state.radius_tier.clone(),
            overstated_signal: false, // TODO: Calculate based on sustainability
        }
    }

    /// Calculate attribute delta for perception
    pub fn calculate_delta(observer_value: f64, target_value: f64) -> AttributeDelta {
        let delta = target_value - observer_value;

        if delta < -2000.0 {
            AttributeDelta::Crushed
        } else if delta < -1000.0 {
            AttributeDelta::FarBelow
        } else if delta < -300.0 {
            AttributeDelta::NotablyBelow
        } else if delta < -150.0 {
            AttributeDelta::SlightlyBelow
        } else if delta < 150.0 {
            AttributeDelta::Indistinguishable
        } else if delta < 300.0 {
            AttributeDelta::SlightlyAbove
        } else if delta < 1000.0 {
            AttributeDelta::NotablyAbove
        } else if delta < 2000.0 {
            AttributeDelta::FarAbove
        } else {
            AttributeDelta::Overwhelming
        }
    }

    /// Generate delta descriptors for LLM
    pub fn generate_delta_descriptor(delta: &AttributeDelta) -> Vec<String> {
        match delta {
            AttributeDelta::Crushed => vec![
                "蝼蚁差距".to_string(),
                "无法测度".to_string(),
                "天壤之别".to_string(),
            ],
            AttributeDelta::FarBelow => vec![
                "远不及".to_string(),
                "基本无力应对".to_string(),
                "实力悬殊".to_string(),
            ],
            AttributeDelta::NotablyBelow => vec![
                "显著弱于".to_string(),
                "明显差距".to_string(),
                "难以抗衡".to_string(),
            ],
            AttributeDelta::SlightlyBelow => vec![
                "略逊一筹".to_string(),
                "稍显不足".to_string(),
                "略有差距".to_string(),
            ],
            AttributeDelta::Indistinguishable => vec![
                "旗鼓相当".to_string(),
                "难分高下".to_string(),
                "伯仲之间".to_string(),
            ],
            AttributeDelta::SlightlyAbove => vec![
                "略胜一筹".to_string(),
                "稍占上风".to_string(),
                "略强于".to_string(),
            ],
            AttributeDelta::NotablyAbove => vec![
                "显著强于".to_string(),
                "明显优势".to_string(),
                "实力高出".to_string(),
            ],
            AttributeDelta::FarAbove => vec![
                "远胜".to_string(),
                "碾压之势".to_string(),
                "实力悬殊".to_string(),
            ],
            AttributeDelta::Overwhelming => vec![
                "压顶之势".to_string(),
                "无法测度".to_string(),
                "天人之别".to_string(),
            ],
        }
    }
}

/// Convert injury severity to penalty ratio
fn injury_severity_to_penalty(severity: &InjurySeverity) -> f64 {
    match severity {
        InjurySeverity::Bruise => 0.02,
        InjurySeverity::Light => 0.05,
        InjurySeverity::Moderate => 0.15,
        InjurySeverity::Severe => 0.30,
        InjurySeverity::Critical => 0.50,
    }
}

/// Attribute modifier from scene/skills
#[derive(Debug, Clone)]
pub struct AttributeModifier {
    pub kind: AttributeKind,
    pub flat_delta: f64,
    pub ratio_modifier: f64,
    pub source_id: String,
}

/// World tendency factors
#[derive(Debug, Clone)]
pub struct TendencyFactors {
    pub inward: f64,
    pub neutral: f64,
    pub expressive: f64,
}

impl Default for TendencyFactors {
    fn default() -> Self {
        Self {
            inward: -0.5,
            neutral: -0.2,
            expressive: 0.1,
        }
    }
}

/// World mode factors
#[derive(Debug, Clone)]
pub struct ModeFactors {
    pub sealed: f64,
    pub suppressed: f64,
    pub natural: f64,
    pub released: f64,
    pub dominating: f64,
}

impl Default for ModeFactors {
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
