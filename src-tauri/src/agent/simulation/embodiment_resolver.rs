//! Embodiment resolver
//!
//! Derives Layer 2 EmbodimentState from Layer 1 data.
//!
//! Key invariant: EmbodimentState contains only derived/tier data,
//! never raw Layer 1 values.
//!
//! Performance optimization: Supports caching via TurnScopedCache.

use std::sync::Arc;

use crate::agent::cache::{SceneDerivedKey, TurnScopedCache};
use crate::agent::models::{
    ActionFeasibility, AirborneEffects, AttributeKind, BaselineBodyProfile, BodyConstraints,
    CharacterRecord, EffectiveAttributeProfile, EmbodimentState, EnvironmentalStrain, ManaField,
    ManaPresenceRadiusTier, PhysicalConditions, ReasoningModifiers, RespirationImpactTier,
    SalienceModifiers, SceneModel, SensoryCapabilities, SensoryCapability, SurfaceImpactTier,
    SurfaceState, Temperature, TemperatureFeelTier, WindImpactTier, WindState,
};

/// Embodiment resolver - derives Layer 2 embodiment state
pub struct EmbodimentResolver {
    /// Optional cache for performance optimization
    cache: Option<Arc<TurnScopedCache>>,
}

impl EmbodimentResolver {
    /// Create a new resolver without caching
    pub fn new() -> Self {
        Self { cache: None }
    }

    /// Create a new resolver with caching enabled
    pub fn with_cache(cache: Arc<TurnScopedCache>) -> Self {
        Self { cache: Some(cache) }
    }

    /// Derive embodiment state from character record and scene
    pub fn derive_embodiment(
        &self,
        character: &CharacterRecord,
        effective_attrs: &EffectiveAttributeProfile,
        scene: &SceneModel,
    ) -> EmbodimentState {
        // Check cache
        if let Some(ref cache) = self.cache {
            let key = SceneDerivedKey {
                character_id: character.character_id.clone(),
                scene_turn_id: scene.scene_turn_id.clone(),
                embodiment_hash: 0, // Simplified
            };
            if let Some(cached) = cache.get_embodiment(&key) {
                return (*cached).clone();
            }
        }

        // Compute
        let result = self.derive_embodiment_uncached(character, effective_attrs, scene);

        // Cache result
        if let Some(ref cache) = self.cache {
            let key = SceneDerivedKey {
                character_id: character.character_id.clone(),
                scene_turn_id: scene.scene_turn_id.clone(),
                embodiment_hash: 0,
            };
            cache.insert_embodiment(key, Arc::new(result.clone()));
        }

        result
    }

    /// Internal implementation without caching
    fn derive_embodiment_uncached(
        &self,
        character: &CharacterRecord,
        effective_attrs: &EffectiveAttributeProfile,
        scene: &SceneModel,
    ) -> EmbodimentState {
        // Derive sensory capabilities
        let sensory = Self::derive_sensory_capabilities(character, effective_attrs);

        // Derive body constraints
        let body = Self::derive_body_constraints(character, effective_attrs, scene);

        // Derive salience modifiers
        let salience = Self::derive_salience_modifiers(character, scene, effective_attrs);

        // Derive reasoning modifiers
        let reasoning = Self::derive_reasoning_modifiers(character, effective_attrs, scene);

        // Derive action feasibility
        let action = Self::derive_action_feasibility(&body, effective_attrs);

        EmbodimentState {
            character_id: character.character_id.clone(),
            scene_turn_id: scene.scene_turn_id.clone(),
            sensory_capabilities: sensory,
            body_constraints: body,
            salience_modifiers: salience,
            reasoning_modifiers: reasoning,
            action_feasibility: action,
        }
    }

    /// Derive sensory capabilities from character state
    fn derive_sensory_capabilities(
        character: &CharacterRecord,
        effective_attrs: &EffectiveAttributeProfile,
    ) -> SensoryCapabilities {
        let insight = effective_attrs
            .values
            .get(&AttributeKind::Insight)
            .copied()
            .unwrap_or(100.0);
        let insight_tier = insight / 500.0; // Normalize to ~0-10 range

        // Get mana acuity from temporary state
        let mana_acuity = character
            .temporary_state
            .mana_expression
            .radius_tier
            .clone();

        // Base sensory capabilities - use mana_sense_baseline for mana, defaults for others
        let base_mana = character.baseline_body_profile.mana_sense_baseline.acuity;

        // Apply temporary state modifiers
        let fatigue_modifier = 1.0 - (character.temporary_state.fatigue * 0.2).min(0.5);
        let pain_modifier = 1.0 - (character.temporary_state.pain_load * 0.1).min(0.3);

        SensoryCapabilities {
            vision: SensoryCapability {
                availability: fatigue_modifier * pain_modifier,
                acuity: (insight_tier * 0.1 + 0.5).min(1.0),
                stability: fatigue_modifier,
                notes: Self::generate_vision_notes(&character.temporary_state),
            },
            hearing: SensoryCapability {
                availability: fatigue_modifier,
                acuity: (insight_tier * 0.08 + 0.5).min(1.0),
                stability: fatigue_modifier,
                notes: String::new(),
            },
            smell: SensoryCapability {
                availability: fatigue_modifier,
                acuity: (insight_tier * 0.05 + 0.3).min(1.0),
                stability: fatigue_modifier,
                notes: String::new(),
            },
            touch: SensoryCapability {
                availability: pain_modifier,
                acuity: 0.8,
                stability: pain_modifier,
                notes: if character.temporary_state.pain_load > 0.5 {
                    "触觉因疼痛而迟钝".to_string()
                } else {
                    String::new()
                },
            },
            proprioception: SensoryCapability {
                availability: fatigue_modifier,
                acuity: 0.9,
                stability: fatigue_modifier,
                notes: String::new(),
            },
            mana: SensoryCapability {
                availability: base_mana,
                acuity: Self::mana_acuity_from_radius(&mana_acuity),
                stability: fatigue_modifier,
                notes: Self::generate_mana_notes(&mana_acuity),
            },
        }
    }

    /// Derive body constraints from character state and environment
    fn derive_body_constraints(
        character: &CharacterRecord,
        effective_attrs: &EffectiveAttributeProfile,
        scene: &SceneModel,
    ) -> BodyConstraints {
        // Calculate environmental strain
        let environmental = Self::translate_environment(
            &scene.physical_conditions,
            &character.baseline_body_profile,
        );

        // Base mobility from attributes
        let agility = effective_attrs
            .values
            .get(&AttributeKind::Agility)
            .copied()
            .unwrap_or(100.0);
        let endurance = effective_attrs
            .values
            .get(&AttributeKind::Endurance)
            .copied()
            .unwrap_or(100.0);

        // Apply injuries
        let injury_penalty: f64 = character
            .temporary_state
            .injuries
            .iter()
            .map(|i| match i.severity {
                crate::agent::models::InjurySeverity::Bruise => 0.02,
                crate::agent::models::InjurySeverity::Light => 0.05,
                crate::agent::models::InjurySeverity::Moderate => 0.15,
                crate::agent::models::InjurySeverity::Severe => 0.30,
                crate::agent::models::InjurySeverity::Critical => 0.50,
            })
            .sum::<f64>()
            .min(0.8);

        // Apply fatigue
        let fatigue_penalty = (character.temporary_state.fatigue * 0.3).min(0.6);

        // Apply pain
        let pain_penalty = (character.temporary_state.pain_load * 0.2).min(0.4);

        // Apply environmental penalties
        let env_mobility_penalty = environmental.movement_penalty;
        let env_balance_penalty = environmental.balance_penalty;

        // Calculate final values
        let mobility = (1.0 - injury_penalty - fatigue_penalty - env_mobility_penalty).max(0.1);
        let balance = (1.0 - pain_penalty - env_balance_penalty).max(0.1);
        let fine_control = (1.0 - pain_penalty * 0.5 - fatigue_penalty * 0.3).max(0.1);
        let cognitive_clarity = (1.0 - fatigue_penalty * 0.5 - pain_penalty * 0.3).max(0.1);

        BodyConstraints {
            mobility,
            balance,
            fine_control,
            pain_load: character.temporary_state.pain_load,
            fatigue_load: character.temporary_state.fatigue,
            cognitive_clarity,
            environmental_strain: environmental,
        }
    }

    /// Translate environment conditions to strain tiers
    fn translate_environment(
        physical: &PhysicalConditions,
        body_profile: &BaselineBodyProfile,
    ) -> EnvironmentalStrain {
        // Wind tier
        let wind_tier = Self::wind_speed_to_tier(physical.wind.speed_ms);

        // Temperature tier (relative to comfort range)
        let temp_tier = Self::temperature_to_tier(
            physical.temperature.felt_celsius,
            &body_profile.comfort_temperature_range,
        );

        // Surface tier
        let surface_tier = Self::surface_state_to_tier(&physical.surface_state);

        // Respiration tier
        let respiration_tier = Self::airborne_to_respiration_tier(&physical.airborne);

        // Calculate penalties
        let (movement_penalty, balance_penalty) =
            Self::calculate_movement_penalties(&wind_tier, &surface_tier, &physical.precipitation);

        // Calculate exposure deltas
        let (exposure_cold, exposure_heat, exposure_respiration) =
            Self::calculate_exposure_deltas(&temp_tier, &respiration_tier);

        // Generate disrupted actions
        let disrupted_actions =
            Self::generate_disrupted_actions(&wind_tier, &surface_tier, &respiration_tier);

        EnvironmentalStrain {
            wind_tier,
            temperature_tier: temp_tier,
            surface_tier,
            respiration_tier,
            movement_penalty,
            balance_penalty,
            exposure_cold_delta: exposure_cold,
            exposure_heat_delta: exposure_heat,
            exposure_respiration_delta: exposure_respiration,
            disrupted_actions,
        }
    }

    /// Convert wind speed to tier
    fn wind_speed_to_tier(speed_ms: f64) -> WindImpactTier {
        if speed_ms < 0.5 {
            WindImpactTier::Calm
        } else if speed_ms < 5.0 {
            WindImpactTier::Breeze
        } else if speed_ms < 10.0 {
            WindImpactTier::Moderate
        } else if speed_ms < 17.0 {
            WindImpactTier::Strong
        } else if speed_ms < 25.0 {
            WindImpactTier::Gale
        } else if speed_ms < 32.0 {
            WindImpactTier::Storm
        } else {
            WindImpactTier::Hurricane
        }
    }

    /// Convert temperature to tier relative to comfort range
    fn temperature_to_tier(felt_celsius: f64, comfort_range: &(f64, f64)) -> TemperatureFeelTier {
        let (min_comfort, max_comfort) = *comfort_range;

        if felt_celsius < min_comfort - 30.0 {
            TemperatureFeelTier::Lethal
        } else if felt_celsius < min_comfort - 15.0 {
            TemperatureFeelTier::SevereCold
        } else if felt_celsius < min_comfort - 5.0 {
            TemperatureFeelTier::Cold
        } else if felt_celsius < min_comfort {
            TemperatureFeelTier::Cool
        } else if felt_celsius <= max_comfort {
            TemperatureFeelTier::Comfortable
        } else if felt_celsius <= max_comfort + 5.0 {
            TemperatureFeelTier::Warm
        } else if felt_celsius <= max_comfort + 15.0 {
            TemperatureFeelTier::Hot
        } else {
            TemperatureFeelTier::Sweltering
        }
    }

    /// Convert surface state to tier
    fn surface_state_to_tier(surface: &SurfaceState) -> SurfaceImpactTier {
        if surface.slipperiness > 0.7 {
            SurfaceImpactTier::Treacherous
        } else if surface.slipperiness > 0.3 {
            SurfaceImpactTier::Slippery
        } else {
            SurfaceImpactTier::Stable
        }
    }

    /// Convert airborne effects to respiration tier
    fn airborne_to_respiration_tier(airborne: &AirborneEffects) -> RespirationImpactTier {
        let total_density = airborne.fog_density + airborne.dust_density + airborne.smoke_density;

        if total_density > 0.8 {
            RespirationImpactTier::Suffocating
        } else if total_density > 0.5 {
            RespirationImpactTier::Choking
        } else if total_density > 0.2 {
            RespirationImpactTier::Irritating
        } else {
            RespirationImpactTier::Free
        }
    }

    /// Calculate movement and balance penalties
    fn calculate_movement_penalties(
        wind_tier: &WindImpactTier,
        surface_tier: &SurfaceImpactTier,
        precipitation: &Option<crate::agent::models::Precipitation>,
    ) -> (f64, f64) {
        let wind_penalty = match wind_tier {
            WindImpactTier::Calm | WindImpactTier::Breeze => 0.0,
            WindImpactTier::Moderate => 0.05,
            WindImpactTier::Strong => 0.15,
            WindImpactTier::Gale => 0.30,
            WindImpactTier::Storm => 0.50,
            WindImpactTier::Hurricane => 0.70,
        };

        let surface_penalty = match surface_tier {
            SurfaceImpactTier::Stable => 0.0,
            SurfaceImpactTier::Slippery => 0.15,
            SurfaceImpactTier::Treacherous => 0.35,
        };

        let precip_penalty = precipitation
            .as_ref()
            .map(|p| {
                let base = match p.kind {
                    crate::agent::models::PrecipitationKind::Rain => 0.1,
                    crate::agent::models::PrecipitationKind::Snow => 0.15,
                    crate::agent::models::PrecipitationKind::Hail => 0.2,
                    crate::agent::models::PrecipitationKind::Sandstorm => 0.25,
                    crate::agent::models::PrecipitationKind::SpiritRain => 0.1,
                };
                p.intensity * base
            })
            .unwrap_or(0.0);

        let movement_penalty = (wind_penalty + surface_penalty + precip_penalty).min(0.8);
        let balance_penalty = (surface_penalty * 1.5 + wind_penalty * 0.5).min(0.8);

        (movement_penalty, balance_penalty)
    }

    /// Calculate exposure deltas for this turn
    fn calculate_exposure_deltas(
        temp_tier: &TemperatureFeelTier,
        respiration_tier: &RespirationImpactTier,
    ) -> (f64, f64, f64) {
        let exposure_cold = match temp_tier {
            TemperatureFeelTier::SevereCold => 0.3,
            TemperatureFeelTier::Lethal => 0.5,
            _ => 0.0,
        };

        let exposure_heat = match temp_tier {
            TemperatureFeelTier::Hot => 0.2,
            TemperatureFeelTier::Sweltering => 0.4,
            _ => 0.0,
        };

        let exposure_respiration = match respiration_tier {
            RespirationImpactTier::Choking => 0.3,
            RespirationImpactTier::Suffocating => 0.5,
            _ => 0.0,
        };

        (exposure_cold, exposure_heat, exposure_respiration)
    }

    /// Generate list of disrupted actions
    fn generate_disrupted_actions(
        wind_tier: &WindImpactTier,
        surface_tier: &SurfaceImpactTier,
        respiration_tier: &RespirationImpactTier,
    ) -> Vec<String> {
        let mut actions = Vec::new();

        if matches!(
            wind_tier,
            WindImpactTier::Gale | WindImpactTier::Storm | WindImpactTier::Hurricane
        ) {
            actions.push("远程瞄准命中-40%".to_string());
        }
        if matches!(wind_tier, WindImpactTier::Storm | WindImpactTier::Hurricane) {
            actions.push("无法稳定站立".to_string());
            actions.push("持续吟唱法术被打断".to_string());
        }
        if matches!(
            surface_tier,
            SurfaceImpactTier::Slippery | SurfaceImpactTier::Treacherous
        ) {
            actions.push("急停困难".to_string());
            actions.push("跑动失败概率增加".to_string());
        }
        if matches!(
            respiration_tier,
            RespirationImpactTier::Choking | RespirationImpactTier::Suffocating
        ) {
            actions.push("持续动作受影响".to_string());
        }

        actions
    }

    /// Derive salience modifiers from scene and character state
    fn derive_salience_modifiers(
        character: &CharacterRecord,
        scene: &SceneModel,
        effective_attrs: &EffectiveAttributeProfile,
    ) -> SalienceModifiers {
        let mut attention_biases = Vec::new();
        let mut aversion_triggers = Vec::new();
        let mut overload_risk: f64 = 0.0;

        // Check mana field for pressure sources
        for presence in &scene.mana_field.character_presences {
            if presence.character_id != character.character_id {
                // High pressure from others
                if matches!(
                    presence.expression_mode,
                    crate::agent::models::ManaExpressionMode::Dominating
                ) {
                    attention_biases.push(format!("被{}的气息牵引", presence.character_id));
                    overload_risk += 0.2;
                }
            }
        }

        // Check for pain-based aversion
        if character.temporary_state.pain_load > 0.5 {
            aversion_triggers.push("疼痛分散注意力".to_string());
        }

        // Check for fatigue-based overload
        if character.temporary_state.fatigue > 0.7 {
            overload_risk += 0.3;
        }

        SalienceModifiers {
            attention_biases,
            aversion_triggers,
            overload_risk: overload_risk.min(1.0_f64),
        }
    }

    /// Derive reasoning modifiers from character state
    fn derive_reasoning_modifiers(
        character: &CharacterRecord,
        effective_attrs: &EffectiveAttributeProfile,
        scene: &SceneModel,
    ) -> ReasoningModifiers {
        let mut notes = Vec::new();

        // Pain bias
        let pain_bias = (character.temporary_state.pain_load * 0.3).min(0.5);
        if pain_bias > 0.1 {
            notes.push(format!("疼痛影响判断（偏差{:.0}%）", pain_bias * 100.0));
        }

        // Threat bias from mana field
        let threat_bias = scene
            .mana_field
            .character_presences
            .iter()
            .filter(|p| p.character_id != character.character_id)
            .map(|p| match p.expression_mode {
                crate::agent::models::ManaExpressionMode::Dominating => 0.3,
                crate::agent::models::ManaExpressionMode::Released => 0.1,
                _ => 0.0,
            })
            .sum::<f64>()
            .min(0.5);

        if threat_bias > 0.1 {
            notes.push("感知到威胁气息".to_string());
        }

        // Overload bias
        let overload_bias = (character.temporary_state.fatigue * 0.2
            + character.temporary_state.pain_load * 0.1)
            .min(0.4);

        ReasoningModifiers {
            pain_bias,
            threat_bias,
            overload_bias,
            notes,
        }
    }

    /// Derive action feasibility from body constraints
    fn derive_action_feasibility(
        body: &BodyConstraints,
        effective_attrs: &EffectiveAttributeProfile,
    ) -> ActionFeasibility {
        // Social patience affected by pain and fatigue
        let social_patience = (1.0 - body.pain_load * 0.3 - body.fatigue_load * 0.2).max(0.1);

        ActionFeasibility {
            physical_execution: body.mobility,
            social_patience,
            fine_control: body.fine_control,
            sustained_attention: body.cognitive_clarity,
            blocked_action_kinds: body.environmental_strain.disrupted_actions.clone(),
        }
    }

    // ===== Helper methods =====

    fn generate_vision_notes(temp_state: &crate::agent::models::TemporaryCharacterState) -> String {
        if temp_state.fatigue > 0.7 {
            "视野因疲惫而模糊".to_string()
        } else if temp_state.pain_load > 0.5 {
            "疼痛分散视觉注意力".to_string()
        } else {
            String::new()
        }
    }

    fn mana_acuity_from_radius(radius: &ManaPresenceRadiusTier) -> f64 {
        match radius {
            ManaPresenceRadiusTier::SelfOnly => 0.3,
            ManaPresenceRadiusTier::Touch => 0.4,
            ManaPresenceRadiusTier::Close => 0.5,
            ManaPresenceRadiusTier::Room => 0.6,
            ManaPresenceRadiusTier::Area => 0.7,
            ManaPresenceRadiusTier::Scene => 0.8,
        }
    }

    fn generate_mana_notes(radius: &ManaPresenceRadiusTier) -> String {
        match radius {
            ManaPresenceRadiusTier::SelfOnly => "仅能感知自身灵力".to_string(),
            ManaPresenceRadiusTier::Touch => "近距离灵觉".to_string(),
            ManaPresenceRadiusTier::Close => "中等范围灵觉".to_string(),
            ManaPresenceRadiusTier::Room => "房间级灵觉".to_string(),
            ManaPresenceRadiusTier::Area => "区域级灵觉".to_string(),
            ManaPresenceRadiusTier::Scene => "场景级灵觉".to_string(),
        }
    }
}

impl Default for EmbodimentResolver {
    fn default() -> Self {
        Self::new()
    }
}
