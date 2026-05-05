//! Combat math resolver
//!
//! Mana Combat Resolution - programmatic combat calculations.
//! Uses effective_mana_power (not displayed) for actual combat.

use crate::agent::models::{
    AttributeKind, AttributeTier, CharacterRecord, EffectIntensityTier, EffectiveAttributeProfile,
    InjurySeverity, ManaAttribute, Skill, SkillEffectContract, SkillEffectKind,
    TemporaryCharacterState,
};

/// Combat math resolver - handles mana combat resolution
pub struct CombatMathResolver;

impl CombatMathResolver {
    /// Calculate combat power for a character
    ///
    /// Formula: combat_power = effective_mana_power × max(0.1, 1 + Σ_modifiers) × soul_factor
    pub fn calculate_combat_power(
        character: &CharacterRecord,
        effective_attrs: &EffectiveAttributeProfile,
        combat_context: &CombatContext,
    ) -> CombatPowerResult {
        // Get effective mana power (not displayed)
        let effective_mana_power = effective_attrs
            .values
            .get(&AttributeKind::ManaPower)
            .copied()
            .unwrap_or(0.0);

        // Calculate modifiers (additive)
        let modifiers = Self::calculate_modifiers(character, effective_attrs, combat_context);
        let modifier_sum: f64 = modifiers.iter().map(|m| m.value).sum();

        // Calculate soul factor (separate multiplicative region)
        let soul_factor = Self::calculate_soul_factor(character, effective_attrs);

        // Apply formula
        let additive_coefficient = (1.0 + modifier_sum).max(0.1);
        let combat_power = effective_mana_power * additive_coefficient * soul_factor;

        CombatPowerResult {
            character_id: character.character_id.clone(),
            effective_mana_power,
            modifiers,
            additive_coefficient,
            soul_factor,
            combat_power,
        }
    }

    /// Calculate all modifiers (additive region)
    fn calculate_modifiers(
        character: &CharacterRecord,
        effective_attrs: &EffectiveAttributeProfile,
        context: &CombatContext,
    ) -> Vec<CombatModifier> {
        let mut modifiers = Vec::new();

        // Skill modifiers
        for skill_mod in &context.skill_modifiers {
            modifiers.push(skill_mod.clone());
        }

        // Injury modifiers
        for injury in &character.temporary_state.injuries {
            let penalty = match injury.severity {
                InjurySeverity::Bruise => -0.02,
                InjurySeverity::Light => -0.05,
                InjurySeverity::Moderate => -0.10,
                InjurySeverity::Severe => -0.20,
                InjurySeverity::Critical => -0.35,
            };
            modifiers.push(CombatModifier {
                source: format!("伤势: {:?}", injury.severity),
                value: penalty,
                kind: ModifierKind::Injury,
            });
        }

        // Fatigue modifier
        if character.temporary_state.fatigue > 0.3 {
            let fatigue_penalty = -0.25 * (character.temporary_state.fatigue - 0.3) / 0.7;
            modifiers.push(CombatModifier {
                source: "疲惫".to_string(),
                value: fatigue_penalty,
                kind: ModifierKind::Fatigue,
            });
        }

        // Pain modifier
        if character.temporary_state.pain_load > 0.5 {
            let pain_penalty = -0.15 * (character.temporary_state.pain_load - 0.5) / 0.5;
            modifiers.push(CombatModifier {
                source: "疼痛".to_string(),
                value: pain_penalty,
                kind: ModifierKind::Pain,
            });
        }

        // Emotional state modifiers
        for emotion_mod in &context.emotional_modifiers {
            modifiers.push(emotion_mod.clone());
        }

        // Environmental modifiers
        for env_mod in &context.environmental_modifiers {
            modifiers.push(env_mod.clone());
        }

        // Attribute affinity modifier (if in favorable environment)
        if let Some(env_attribute) = context.environment_mana_attribute {
            let affinity = &character.baseline_body_profile.mana_attribute_affinity;
            if affinity.contains(&env_attribute) {
                modifiers.push(CombatModifier {
                    source: "属性亲和".to_string(),
                    value: 0.10,
                    kind: ModifierKind::AttributeAffinity,
                });
            }
            // Check for counter-attribute
            if let Some(counter) = Self::get_counter_attribute(&env_attribute) {
                if affinity.contains(&counter) {
                    modifiers.push(CombatModifier {
                        source: "属性克制".to_string(),
                        value: -0.15,
                        kind: ModifierKind::AttributeCounter,
                    });
                }
            }
        }

        modifiers
    }

    /// Get counter attribute for a given attribute
    fn get_counter_attribute(attr: &ManaAttribute) -> Option<ManaAttribute> {
        use ManaAttribute::*;
        Some(match attr {
            Fire => Water,
            Water => Fire,
            Wood => Metal,
            Metal => Wood,
            Earth => Wood, // Wood overcomes Earth
            Light => Dark,
            Dark => Light,
            Void => Void,                          // Self-counter
            Wind | Lightning | Ice => return None, // No specific counter
        })
    }

    /// Calculate soul factor (separate multiplicative region)
    fn calculate_soul_factor(
        character: &CharacterRecord,
        effective_attrs: &EffectiveAttributeProfile,
    ) -> f64 {
        let soul_strength = effective_attrs
            .values
            .get(&AttributeKind::SoulStrength)
            .copied()
            .unwrap_or(100.0);

        // Check for soul damage conditions
        let soul_damage: f64 = character
            .temporary_state
            .active_conditions
            .iter()
            .filter(|c| c.domain == crate::agent::models::StateDomain::Soul)
            .map(|c| c.intensity * 0.3)
            .sum();

        // Base soul factor from soul strength tier
        let base_factor = match AttributeTier::from_value(soul_strength) {
            AttributeTier::Mundane => 0.8,
            AttributeTier::Awakened => 0.9,
            AttributeTier::Adept => 1.0,
            AttributeTier::Master => 1.05,
            AttributeTier::Ascendant => 1.1,
            AttributeTier::Transcendent => 1.15,
        };

        // Apply soul damage
        (base_factor - soul_damage).max(0.2)
    }

    /// Resolve combat between actor and target
    pub fn resolve_combat(
        actor_power: &CombatPowerResult,
        target_power: &CombatPowerResult,
        thresholds: &CombatDeltaThresholds,
    ) -> ManaCombatResolution {
        let combat_delta = actor_power.combat_power - target_power.combat_power;

        let outcome_tier = if combat_delta >= thresholds.crushing {
            CombatOutcomeTier::Crushing
        } else if combat_delta >= thresholds.marked_edge {
            CombatOutcomeTier::MarkedEdge
        } else if combat_delta >= thresholds.slight_edge {
            CombatOutcomeTier::SlightEdge
        } else if combat_delta >= -thresholds.slight_edge {
            CombatOutcomeTier::Indistinguishable
        } else if combat_delta >= -thresholds.marked_edge {
            CombatOutcomeTier::SlightDisadvantage
        } else if combat_delta >= -thresholds.crushing {
            CombatOutcomeTier::MarkedDisadvantage
        } else {
            CombatOutcomeTier::CrushingDisadvantage
        };

        // Generate disrupting factors description
        let disrupting_factors = Self::generate_disrupting_factors(actor_power, target_power);

        ManaCombatResolution {
            actor_id: actor_power.character_id.clone(),
            target_id: target_power.character_id.clone(),
            actor_combat_power: actor_power.combat_power,
            target_combat_power: target_power.combat_power,
            combat_delta,
            outcome_tier,
            disrupting_factors,
        }
    }

    /// Generate human-readable disrupting factors
    fn generate_disrupting_factors(
        actor: &CombatPowerResult,
        target: &CombatPowerResult,
    ) -> Vec<String> {
        let mut factors = Vec::new();

        // Actor modifiers
        for modifier in &actor.modifiers {
            if modifier.value.abs() > 0.05 {
                let sign = if modifier.value > 0.0 { "+" } else { "" };
                factors.push(format!(
                    "攻方{}: {}{:.0}%",
                    modifier.source,
                    sign,
                    modifier.value * 100.0
                ));
            }
        }

        // Target modifiers
        for modifier in &target.modifiers {
            if modifier.value.abs() > 0.05 {
                let sign = if modifier.value > 0.0 { "+" } else { "" };
                factors.push(format!(
                    "守方{}: {}{:.0}%",
                    modifier.source,
                    sign,
                    modifier.value * 100.0
                ));
            }
        }

        // Soul factor differences
        if (actor.soul_factor - target.soul_factor).abs() > 0.1 {
            factors.push(format!(
                "灵魂系数: 攻方{:.2} vs 守方{:.2}",
                actor.soul_factor, target.soul_factor
            ));
        }

        factors
    }

    /// Validate skill effect against combat constraints
    pub fn validate_skill_effect(
        skill: &Skill,
        effect_kind: SkillEffectKind,
        intensity: EffectIntensityTier,
        actor_power: &CombatPowerResult,
    ) -> SkillEffectValidation {
        let contract = &skill.effect_contract;

        let mut violations = Vec::new();
        let mut warnings = Vec::new();

        // Check intensity tier
        if intensity > contract.max_intensity_tier {
            violations.push(format!(
                "效果强度 {:?} 超过技能上限 {:?}",
                intensity, contract.max_intensity_tier
            ));
        }

        // Check if skill allows this effect kind
        let effect_allowed = match effect_kind {
            SkillEffectKind::Damage => contract.allows_injury,
            SkillEffectKind::Movement => contract.allows_position_change,
            SkillEffectKind::KnowledgeReveal => contract.allows_knowledge_reveal,
            _ => true,
        };

        if !effect_allowed {
            violations.push(format!("技能契约不允许 {:?} 效果", effect_kind));
        }

        // Check mana reserve
        if let Some(mana_cost) = skill.requirements.cost.mana_reserve_delta {
            if actor_power.effective_mana_power + mana_cost < 0.0 {
                violations.push("灵力不足以支付技能消耗".to_string());
            }
        }

        // Generate warnings for significant modifiers
        for modifier in &actor_power.modifiers {
            if modifier.value.abs() > 0.2 {
                warnings.push(format!(
                    "显著修正: {} ({}{:.0}%)",
                    modifier.source,
                    if modifier.value > 0.0 { "+" } else { "" },
                    modifier.value * 100.0
                ));
            }
        }

        SkillEffectValidation {
            skill_id: skill.skill_id.clone(),
            effect_kind,
            intensity,
            is_valid: violations.is_empty(),
            violations,
            warnings,
        }
    }
}

/// Combat context for modifier calculation
#[derive(Debug, Clone, Default)]
pub struct CombatContext {
    pub skill_modifiers: Vec<CombatModifier>,
    pub emotional_modifiers: Vec<CombatModifier>,
    pub environmental_modifiers: Vec<CombatModifier>,
    pub environment_mana_attribute: Option<ManaAttribute>,
    pub ambient_density_tier: AmbientManaDensityTier,
}

#[derive(Debug, Clone, Default)]
pub enum AmbientManaDensityTier {
    #[default]
    Normal,
    Barren,
    Sparse,
    Rich,
    Dense,
    Saturated,
}

/// Combat modifier
#[derive(Debug, Clone)]
pub struct CombatModifier {
    pub source: String,
    pub value: f64,
    pub kind: ModifierKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifierKind {
    Skill,
    Injury,
    Fatigue,
    Pain,
    Emotional,
    Environmental,
    AttributeAffinity,
    AttributeCounter,
    Soul,
}

/// Combat power calculation result
#[derive(Debug, Clone)]
pub struct CombatPowerResult {
    pub character_id: String,
    pub effective_mana_power: f64,
    pub modifiers: Vec<CombatModifier>,
    pub additive_coefficient: f64,
    pub soul_factor: f64,
    pub combat_power: f64,
}

/// Mana combat resolution result
#[derive(Debug, Clone)]
pub struct ManaCombatResolution {
    pub actor_id: String,
    pub target_id: String,
    pub actor_combat_power: f64,
    pub target_combat_power: f64,
    pub combat_delta: f64,
    pub outcome_tier: CombatOutcomeTier,
    pub disrupting_factors: Vec<String>,
}

/// Combat outcome tier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatOutcomeTier {
    /// |Δ| < 150 - even match
    Indistinguishable,
    /// Δ ∈ [150, 300) - slight advantage
    SlightEdge,
    /// Δ ∈ [300, 1000) - marked advantage
    MarkedEdge,
    /// Δ ≥ 1000 - crushing victory
    Crushing,
    /// Negative counterparts
    SlightDisadvantage,
    MarkedDisadvantage,
    CrushingDisadvantage,
}

impl CombatOutcomeTier {
    /// Check if actor wins
    pub fn is_actor_advantage(&self) -> bool {
        matches!(
            self,
            CombatOutcomeTier::SlightEdge
                | CombatOutcomeTier::MarkedEdge
                | CombatOutcomeTier::Crushing
        )
    }

    /// Check if target wins
    pub fn is_target_advantage(&self) -> bool {
        matches!(
            self,
            CombatOutcomeTier::SlightDisadvantage
                | CombatOutcomeTier::MarkedDisadvantage
                | CombatOutcomeTier::CrushingDisadvantage
        )
    }
}

/// Combat delta thresholds (configurable per world)
#[derive(Debug, Clone)]
pub struct CombatDeltaThresholds {
    pub slight_edge: f64,
    pub marked_edge: f64,
    pub crushing: f64,
}

impl Default for CombatDeltaThresholds {
    fn default() -> Self {
        Self {
            slight_edge: 150.0,
            marked_edge: 300.0,
            crushing: 1000.0,
        }
    }
}

/// Skill effect validation result
#[derive(Debug, Clone)]
pub struct SkillEffectValidation {
    pub skill_id: String,
    pub effect_kind: SkillEffectKind,
    pub intensity: EffectIntensityTier,
    pub is_valid: bool,
    pub violations: Vec<String>,
    pub warnings: Vec<String>,
}
