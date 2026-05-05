//! Effect validator
//!
//! Validates skill effects and state updates against contracts.

use crate::agent::models::{
    ActivationCondition, ActivationTime, CharacterRecord, CharacterStateDelta, SceneEntity,
    SceneModel, Skill, SkillKind, StateDomain, StateUpdatePlan, TargetCount, TargetKind,
};

/// Effect validator - validates skill effects and state updates
pub struct EffectValidator;

impl EffectValidator {
    /// Validate state update plan against skills and constraints
    pub fn validate_state_update(
        plan: &StateUpdatePlan,
        skills: &[Skill],
        characters: &[CharacterRecord],
        scene: &SceneModel,
    ) -> Result<ValidatedPlan, String> {
        let mut valid_deltas = Vec::new();
        let mut blocked_effects = Vec::new();
        let soft_effects = Vec::new();

        // Validate each character state delta
        for delta in &plan.character_state_deltas {
            match Self::validate_character_delta(delta, skills, characters, scene) {
                Ok(()) => valid_deltas.push(delta.clone()),
                Err(reason) => {
                    blocked_effects.push(BlockedEffect {
                        source_id: delta.character_id.clone(),
                        target_id: Some(delta.character_id.clone()),
                        attempted_state_domain: "character_state".to_string(),
                        reason_code: reason,
                        fallback_soft_effect: None,
                    });
                }
            }
        }

        // Validate scene delta
        if let Some(scene_delta) = &plan.scene_delta {
            if scene_delta.scene_id != scene.scene_id {
                blocked_effects.push(BlockedEffect {
                    source_id: scene_delta.scene_id.clone(),
                    target_id: Some(scene.scene_id.clone()),
                    attempted_state_domain: "scene".to_string(),
                    reason_code: "scene_delta_scene_id_mismatch".to_string(),
                    fallback_soft_effect: None,
                });
            }

            let scene_entity_ids: std::collections::HashSet<&str> = scene
                .entities
                .iter()
                .map(|entity| entity.entity_id.as_str())
                .collect();
            let scene_signal_ids: std::collections::HashSet<&str> = scene
                .observable_signals
                .visual_signals
                .iter()
                .map(|signal| signal.signal_id.as_str())
                .chain(
                    scene
                        .observable_signals
                        .audio_signals
                        .iter()
                        .map(|signal| signal.signal_id.as_str()),
                )
                .chain(
                    scene
                        .observable_signals
                        .mana_signals
                        .iter()
                        .map(|signal| signal.signal_id.as_str()),
                )
                .collect();
            let private_fact_ids: std::collections::HashSet<&str> = scene
                .private_state
                .hidden_facts
                .iter()
                .map(|fact| fact.fact_id.as_str())
                .collect();

            for entity_delta in &scene_delta.entity_deltas {
                if !scene_entity_ids.contains(entity_delta.entity_id.as_str()) {
                    blocked_effects.push(BlockedEffect {
                        source_id: entity_delta.entity_id.clone(),
                        target_id: Some(entity_delta.entity_id.clone()),
                        attempted_state_domain: "scene.entity".to_string(),
                        reason_code: "unknown_scene_entity".to_string(),
                        fallback_soft_effect: None,
                    });
                }
                if entity_delta.delta_kind.trim().is_empty() || !entity_delta.payload.is_object() {
                    blocked_effects.push(BlockedEffect {
                        source_id: entity_delta.entity_id.clone(),
                        target_id: Some(entity_delta.entity_id.clone()),
                        attempted_state_domain: "scene.entity".to_string(),
                        reason_code: "invalid_entity_delta_payload".to_string(),
                        fallback_soft_effect: None,
                    });
                }
            }

            for signal_delta in &scene_delta.observable_signal_deltas {
                if !scene_signal_ids.contains(signal_delta.signal_id.as_str()) {
                    blocked_effects.push(BlockedEffect {
                        source_id: signal_delta.signal_id.clone(),
                        target_id: Some(signal_delta.signal_id.clone()),
                        attempted_state_domain: "scene.signal".to_string(),
                        reason_code: "unknown_observable_signal".to_string(),
                        fallback_soft_effect: None,
                    });
                }
                if signal_delta.delta_kind.trim().is_empty() || !signal_delta.payload.is_object() {
                    blocked_effects.push(BlockedEffect {
                        source_id: signal_delta.signal_id.clone(),
                        target_id: Some(signal_delta.signal_id.clone()),
                        attempted_state_domain: "scene.signal".to_string(),
                        reason_code: "invalid_signal_delta_payload".to_string(),
                        fallback_soft_effect: None,
                    });
                }
            }

            for private_delta in &scene_delta.private_state_deltas {
                if !private_fact_ids.contains(private_delta.private_fact_id.as_str()) {
                    blocked_effects.push(BlockedEffect {
                        source_id: private_delta.private_fact_id.clone(),
                        target_id: Some(private_delta.private_fact_id.clone()),
                        attempted_state_domain: "scene.private_state".to_string(),
                        reason_code: "unknown_private_fact".to_string(),
                        fallback_soft_effect: None,
                    });
                }
            }

            if scene_delta
                .physical_delta
                .as_ref()
                .is_some_and(|delta| !delta.field_patches.is_object())
                || scene_delta
                    .mana_field_delta
                    .as_ref()
                    .is_some_and(|delta| !delta.field_patches.is_object())
            {
                blocked_effects.push(BlockedEffect {
                    source_id: scene_delta.scene_id.clone(),
                    target_id: Some(scene_delta.scene_id.clone()),
                    attempted_state_domain: "scene".to_string(),
                    reason_code: "scene_field_patch_must_be_object".to_string(),
                    fallback_soft_effect: None,
                });
            }
        }

        Ok(ValidatedPlan {
            valid_character_deltas: valid_deltas,
            blocked_effects,
            soft_effects,
        })
    }

    /// Preview whether a reaction/passive skill is currently legal before it is
    /// exposed as a reaction option.
    pub fn preview_skill_use(
        skill: &Skill,
        actor: &CharacterRecord,
        characters: &[CharacterRecord],
        target_ids: &[String],
        scene: &SceneModel,
    ) -> Result<(), String> {
        let preview_target = match skill.effect_contract.target_kind {
            TargetKind::SelfTarget => actor,
            _ => target_ids
                .first()
                .and_then(|target_id| {
                    characters
                        .iter()
                        .find(|character| character.character_id == *target_id)
                })
                .unwrap_or(actor),
        };

        validate_skill_constraints(skill, actor, preview_target, target_ids, scene)
    }

    /// Validate a single character state delta
    fn validate_character_delta(
        delta: &CharacterStateDelta,
        skills: &[Skill],
        characters: &[CharacterRecord],
        scene: &SceneModel,
    ) -> Result<(), String> {
        let target_character = characters
            .iter()
            .find(|character| character.character_id == delta.character_id)
            .ok_or_else(|| "unknown_character".to_string())?;
        if !scene
            .entities
            .iter()
            .any(|entity| entity.entity_id == delta.character_id)
        {
            return Err("character_not_present_in_scene".to_string());
        }
        if !delta.temporary_state_delta.is_object() {
            return Err("temporary_state_delta_must_be_object".to_string());
        }

        let actor_id = delta
            .temporary_state_delta
            .get("actor_id")
            .and_then(|value| value.as_str())
            .unwrap_or(delta.character_id.as_str());
        let actor = characters
            .iter()
            .find(|character| character.character_id == actor_id)
            .ok_or_else(|| "unknown_actor".to_string())?;
        if !scene
            .entities
            .iter()
            .any(|entity| entity.entity_id == actor.character_id)
        {
            return Err("actor_not_present_in_scene".to_string());
        }

        if let Some(skill_id) = delta
            .temporary_state_delta
            .get("skill_id")
            .and_then(|value| value.as_str())
        {
            let skill = skills
                .iter()
                .find(|skill| skill.skill_id == skill_id && skill.belongs_to_character(actor_id))
                .or_else(|| skills.iter().find(|skill| skill.skill_id == skill_id))
                .ok_or_else(|| "unknown_skill_id".to_string())?;
            let target_ids = extract_target_ids(delta);
            validate_skill_constraints(skill, actor, target_character, &target_ids, scene)?;
        }

        let forbidden_keys = [
            "base_attributes",
            "baseline_body_profile",
            "mind_model_card_knowledge_id",
            "schema_version",
            "created_at",
        ];
        for key in forbidden_keys {
            if delta.temporary_state_delta.get(key).is_some() {
                return Err(format!("immutable_character_field_in_delta:{}", key));
            }
        }

        validate_optional_ratio(&delta.temporary_state_delta, "fatigue")?;
        validate_optional_ratio(&delta.temporary_state_delta, "pain_load")?;
        validate_optional_non_negative(&delta.temporary_state_delta, "mana_reserve_current")?;
        if let Some(environment) = delta.temporary_state_delta.get("environmental_exposure") {
            if !environment.is_object() {
                return Err("environmental_exposure_delta_must_be_object".to_string());
            }
            validate_optional_ratio(environment, "cold_strain")?;
            validate_optional_ratio(environment, "heat_strain")?;
            validate_optional_ratio(environment, "respiration_strain")?;
        }
        if let Some(expression) = delta.temporary_state_delta.get("mana_expression") {
            if !expression.is_object() {
                return Err("mana_expression_delta_must_be_object".to_string());
            }
            validate_optional_non_negative(expression, "display_ratio")?;
            validate_optional_non_negative(expression, "pressure_ratio")?;
        }
        if delta
            .outward_body_signals
            .iter()
            .any(|signal| signal.trim().is_empty())
        {
            return Err("empty_outward_body_signal".to_string());
        }
        if delta.outward_body_signals.len() > 16 {
            return Err("too_many_outward_body_signals".to_string());
        }

        Ok(())
    }

    /// Check if an effect is allowed by skill contract
    pub fn is_effect_allowed(
        effect: &EffectRequest,
        skill: &Skill,
        _character: &CharacterRecord,
    ) -> bool {
        let contract = &skill.effect_contract;

        // Check target kind
        if !contract.allowed_target_kinds.contains(&effect.target_kind) {
            return false;
        }

        // Check state domain
        if !contract
            .allowed_state_domains
            .iter()
            .any(|domain| domain.eq_ignore_ascii_case(state_domain_to_str(effect.state_domain)))
        {
            return false;
        }

        // Check intensity tier
        if effect.intensity_tier > contract.max_intensity_tier {
            return false;
        }

        // Check specific permissions
        match effect.effect_kind {
            EffectKind::Injury if !contract.allows_injury => return false,
            EffectKind::PositionChange if !contract.allows_position_change => return false,
            EffectKind::KnowledgeReveal if !contract.allows_knowledge_reveal => return false,
            _ => {}
        }

        true
    }
}

fn validate_skill_constraints(
    skill: &Skill,
    actor: &CharacterRecord,
    target_character: &CharacterRecord,
    target_ids: &[String],
    scene: &SceneModel,
) -> Result<(), String> {
    validate_skill_reaction_contract(skill, actor, target_character)?;
    validate_skill_cost(skill, actor)?;
    validate_skill_requirements(skill, actor)?;
    validate_skill_targets(skill, actor, target_character, target_ids, scene)?;
    validate_activation_conditions(skill, actor, target_ids, scene)?;
    Ok(())
}

fn validate_skill_reaction_contract(
    skill: &Skill,
    actor: &CharacterRecord,
    target_character: &CharacterRecord,
) -> Result<(), String> {
    let actor_id = actor.character_id.as_str();
    let target_id = target_character.character_id.as_str();

    let is_reaction = skill.skill_kind == SkillKind::Reaction
        || skill.activation.activation_time == ActivationTime::Reaction;
    let is_passive = matches!(skill.skill_kind, SkillKind::Passive | SkillKind::Stance);

    if is_reaction {
        let has_interrupt_tag = skill
            .metadata
            .tags
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case("interrupt"));
        let has_interrupt_ready = actor
            .temporary_state
            .active_conditions
            .iter()
            .any(|condition| {
                condition
                    .condition_kind
                    .eq_ignore_ascii_case("interrupt_ready")
            });

        if has_interrupt_tag && !has_interrupt_ready {
            return Err("interrupt_not_ready".to_string());
        }

        if actor_id == target_id && has_interrupt_tag {
            return Err("interrupt_cannot_target_self".to_string());
        }
    }

    if is_passive {
        let has_passive_field = actor
            .temporary_state
            .active_conditions
            .iter()
            .any(|condition| {
                condition
                    .condition_kind
                    .eq_ignore_ascii_case("passive_field")
            });
        if !has_passive_field {
            return Err("passive_field_not_active".to_string());
        }
    }

    Ok(())
}

fn validate_skill_cost(skill: &Skill, character: &CharacterRecord) -> Result<(), String> {
    if character
        .temporary_state
        .cooldowns
        .iter()
        .any(|cooldown| cooldown.ability_id == skill.skill_id && cooldown.remaining_turns > 0)
    {
        return Err("skill_on_cooldown".to_string());
    }

    if let Some(mana_delta) = skill.requirements.cost.mana_reserve_delta {
        if mana_delta < 0.0 {
            let current = character
                .temporary_state
                .mana_reserve_current
                .unwrap_or(0.0);
            if current + mana_delta < 0.0 {
                return Err("insufficient_mana_reserve".to_string());
            }
        }
    }
    if let Some(fatigue_delta) = skill.requirements.cost.fatigue_delta {
        if character.temporary_state.fatigue + fatigue_delta > 1.0 {
            return Err("fatigue_cost_exceeds_limit".to_string());
        }
    }
    if let Some(material) = skill.requirements.cost.material_refs.first() {
        return Err(format!("unverified_material_ref:{}", material));
    }
    if let Some(condition) = skill
        .requirements
        .cost
        .required_conditions
        .iter()
        .find(|condition| !has_condition(character, condition))
    {
        return Err(format!("missing_required_condition:{}", condition));
    }
    Ok(())
}

fn validate_skill_requirements(skill: &Skill, character: &CharacterRecord) -> Result<(), String> {
    for (attribute, minimum) in &skill.requirements.minimum_attributes {
        let actual = base_attribute_value(character, attribute)
            .ok_or_else(|| format!("unknown_required_attribute:{}", attribute))?;
        if actual < *minimum {
            return Err(format!("minimum_attribute_not_met:{}", attribute));
        }
    }

    if let Some(condition) = skill
        .requirements
        .prohibited_conditions
        .iter()
        .find(|condition| has_condition(character, condition))
    {
        return Err(format!("prohibited_condition_present:{}", condition));
    }

    if let Some(material) = skill.requirements.material_components.first() {
        return Err(format!("unverified_material_component:{}", material));
    }

    Ok(())
}

fn validate_skill_targets(
    skill: &Skill,
    actor: &CharacterRecord,
    target_character: &CharacterRecord,
    target_ids: &[String],
    scene: &SceneModel,
) -> Result<(), String> {
    let concrete_targets = if target_ids.is_empty() {
        vec![target_character.character_id.clone()]
    } else {
        target_ids.to_vec()
    };

    match skill.effect_contract.target_count {
        TargetCount::Single if concrete_targets.len() != 1 => {
            return Err("target_count_requires_single".to_string());
        }
        TargetCount::Multi(max) if concrete_targets.len() as u32 > max => {
            return Err("target_count_exceeded".to_string());
        }
        _ => {}
    }

    if let Some(range_m) = skill.effect_contract.range_m {
        validate_targets_in_range(actor, &concrete_targets, scene, range_m)?;
    }

    Ok(())
}

fn validate_activation_conditions(
    skill: &Skill,
    actor: &CharacterRecord,
    target_ids: &[String],
    scene: &SceneModel,
) -> Result<(), String> {
    for condition in &skill.activation.trigger_conditions {
        match condition {
            ActivationCondition::ManaReserveAtLeast(minimum) => {
                let current = actor
                    .temporary_state
                    .mana_reserve_current
                    .unwrap_or(actor.base_attributes.mana_power);
                if current < *minimum {
                    return Err("activation_mana_reserve_too_low".to_string());
                }
            }
            ActivationCondition::FatigueBelow(maximum) => {
                if actor.temporary_state.fatigue >= *maximum {
                    return Err("activation_fatigue_too_high".to_string());
                }
            }
            ActivationCondition::HasStatus(status) => {
                if !has_condition(actor, status) {
                    return Err(format!("activation_missing_status:{}", status));
                }
            }
            ActivationCondition::LacksStatus(status) => {
                if has_condition(actor, status) {
                    return Err(format!("activation_blocked_by_status:{}", status));
                }
            }
            ActivationCondition::TargetInRange(range_m) => {
                if target_ids.is_empty() {
                    return Err("activation_missing_target_for_range".to_string());
                }
                validate_targets_in_range(actor, target_ids, scene, *range_m)?;
            }
            ActivationCondition::TargetInLineOfSight => {
                if target_ids.is_empty() {
                    return Err("activation_missing_target_for_line_of_sight".to_string());
                }
                validate_targets_in_line_of_sight(actor, target_ids, scene)?;
            }
            ActivationCondition::InCombat
            | ActivationCondition::OutOfCombat
            | ActivationCondition::EnvironmentCondition(_)
            | ActivationCondition::Custom(_) => {}
        }
    }
    Ok(())
}

fn validate_targets_in_range(
    actor: &CharacterRecord,
    target_ids: &[String],
    scene: &SceneModel,
    range_m: f64,
) -> Result<(), String> {
    if !range_m.is_finite() || range_m < 0.0 {
        return Err("invalid_skill_range".to_string());
    }
    let actor_entity = find_scene_entity(scene, &actor.character_id)
        .ok_or_else(|| "actor_not_present_in_scene".to_string())?;
    for target_id in target_ids {
        let target_entity = find_scene_entity(scene, target_id)
            .ok_or_else(|| format!("unknown_target_entity:{}", target_id))?;
        if distance_m(actor_entity, target_entity) > range_m {
            return Err(format!("target_out_of_range:{}", target_id));
        }
    }
    Ok(())
}

fn validate_targets_in_line_of_sight(
    actor: &CharacterRecord,
    target_ids: &[String],
    scene: &SceneModel,
) -> Result<(), String> {
    let actor_entity = find_scene_entity(scene, &actor.character_id)
        .ok_or_else(|| "actor_not_present_in_scene".to_string())?;
    let visibility_range_m = scene.physical_conditions.airborne.visibility_range_m;
    if !visibility_range_m.is_finite() || visibility_range_m <= 0.0 {
        return Err("line_of_sight_blocked_by_visibility".to_string());
    }

    for target_id in target_ids {
        let target_entity = find_scene_entity(scene, target_id)
            .ok_or_else(|| format!("unknown_target_entity:{}", target_id))?;
        if distance_m(actor_entity, target_entity) > visibility_range_m {
            return Err(format!("line_of_sight_blocked:{}", target_id));
        }
    }
    Ok(())
}

fn extract_target_ids(delta: &CharacterStateDelta) -> Vec<String> {
    let mut ids = Vec::new();
    for key in ["target_ids", "target_refs", "applies_to"] {
        if let Some(values) = delta
            .temporary_state_delta
            .get(key)
            .and_then(|value| value.as_array())
        {
            ids.extend(
                values
                    .iter()
                    .filter_map(|value| value.as_str())
                    .map(str::to_string),
            );
        }
    }
    if let Some(target_id) = delta
        .temporary_state_delta
        .get("target_id")
        .and_then(|value| value.as_str())
    {
        ids.push(target_id.to_string());
    }
    ids.sort();
    ids.dedup();
    ids
}

fn has_condition(character: &CharacterRecord, condition: &str) -> bool {
    character
        .temporary_state
        .active_conditions
        .iter()
        .any(|state| {
            state.condition_id == condition || state.condition_kind.eq_ignore_ascii_case(condition)
        })
}

fn base_attribute_value(character: &CharacterRecord, attribute: &str) -> Option<f64> {
    match attribute {
        value if value.eq_ignore_ascii_case("physical") => Some(character.base_attributes.physical),
        value if value.eq_ignore_ascii_case("agility") => Some(character.base_attributes.agility),
        value if value.eq_ignore_ascii_case("endurance") => {
            Some(character.base_attributes.endurance)
        }
        value if value.eq_ignore_ascii_case("insight") => Some(character.base_attributes.insight),
        value if value.eq_ignore_ascii_case("mana_power") => {
            Some(character.base_attributes.mana_power)
        }
        value if value.eq_ignore_ascii_case("soul_strength") => {
            Some(character.base_attributes.soul_strength)
        }
        _ => None,
    }
}

fn find_scene_entity<'a>(scene: &'a SceneModel, entity_id: &str) -> Option<&'a SceneEntity> {
    scene
        .entities
        .iter()
        .find(|entity| entity.entity_id == entity_id)
}

fn distance_m(a: &SceneEntity, b: &SceneEntity) -> f64 {
    let dz = a.position.z.unwrap_or(0.0) - b.position.z.unwrap_or(0.0);
    ((a.position.x - b.position.x).powi(2) + (a.position.y - b.position.y).powi(2) + dz.powi(2))
        .sqrt()
}

fn validate_optional_ratio(value: &serde_json::Value, key: &str) -> Result<(), String> {
    if let Some(number) = value.get(key) {
        let number = number
            .as_f64()
            .ok_or_else(|| format!("{}_must_be_number", key))?;
        if !number.is_finite() || !(0.0..=1.0).contains(&number) {
            return Err(format!("{}_out_of_range", key));
        }
    }
    Ok(())
}

fn validate_optional_non_negative(value: &serde_json::Value, key: &str) -> Result<(), String> {
    if let Some(number) = value.get(key) {
        let number = number
            .as_f64()
            .ok_or_else(|| format!("{}_must_be_number", key))?;
        if !number.is_finite() || number < 0.0 {
            return Err(format!("{}_out_of_range", key));
        }
    }
    Ok(())
}

fn state_domain_to_str(domain: StateDomain) -> &'static str {
    match domain {
        StateDomain::Body => "body",
        StateDomain::Resource => "resource",
        StateDomain::Position => "position",
        StateDomain::Perception => "perception",
        StateDomain::Mind => "mind",
        StateDomain::Soul => "soul",
        StateDomain::Scene => "scene",
        StateDomain::KnowledgeReveal => "knowledge_reveal",
    }
}

/// Validated plan result
#[derive(Debug, Clone)]
pub struct ValidatedPlan {
    pub valid_character_deltas: Vec<CharacterStateDelta>,
    pub blocked_effects: Vec<BlockedEffect>,
    pub soft_effects: Vec<SoftEffect>,
}

/// Blocked effect
#[derive(Debug, Clone)]
pub struct BlockedEffect {
    pub source_id: String,
    pub target_id: Option<String>,
    pub attempted_state_domain: String,
    pub reason_code: String,
    pub fallback_soft_effect: Option<SoftEffect>,
}

/// Soft effect (narrative but not state change)
#[derive(Debug, Clone)]
pub struct SoftEffect {
    pub source_id: String,
    pub target_id: Option<String>,
    pub effect_kind: String,
    pub description: String,
}

/// Effect request for validation
#[derive(Debug, Clone)]
pub struct EffectRequest {
    pub target_kind: crate::agent::models::TargetKind,
    pub state_domain: crate::agent::models::StateDomain,
    pub effect_kind: EffectKind,
    pub intensity_tier: crate::agent::models::EffectIntensityTier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectKind {
    Injury,
    PositionChange,
    KnowledgeReveal,
    ResourceChange,
    ConditionApply,
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::models::*;
    use chrono::Utc;
    use serde_json::json;

    fn character(id: &str) -> CharacterRecord {
        CharacterRecord {
            character_id: id.to_string(),
            base_attributes: BaseAttributes {
                physical: 100.0,
                agility: 100.0,
                endurance: 100.0,
                insight: 100.0,
                mana_power: 200.0,
                soul_strength: 100.0,
            },
            baseline_body_profile: BaselineBodyProfile {
                species: "human".to_string(),
                comfort_temperature_range: (18.0, 26.0),
                mana_sense_baseline: ManaSenseBaseline {
                    acuity: 0.2,
                    overload_threshold: 1000.0,
                    attribute_bias: None,
                },
                mana_attribute_affinity: Vec::new(),
                size_class: SizeClass::Humanoid,
            },
            mana_expression_tendency: ManaExpressionTendency::Neutral,
            mana_expression_tendency_factor_override: None,
            mind_model_card_knowledge_id: format!("mind-{id}"),
            temporary_state: TemporaryCharacterState::new(),
            schema_version: "0.1".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn scene(visibility_range_m: f64) -> SceneModel {
        SceneModel {
            scene_id: "scene-1".to_string(),
            scene_turn_id: "turn-1".to_string(),
            time_context: TimeContext {
                time_anchor: TimeAnchor {
                    calendar_id: "default".to_string(),
                    ordinal: 1,
                    precision: TimePrecision::Exact,
                    display_text: "now".to_string(),
                },
                season: "spring".to_string(),
                day_phase: DayPhase::Day,
                weather_trend: "stable".to_string(),
            },
            spatial_layout: SpatialLayout {
                layout_type: "room".to_string(),
                dimensions: None,
                obstacles: Vec::new(),
                entrances: Vec::new(),
                zones: Vec::new(),
            },
            lighting: LightingState {
                ambient_level: 1.0,
                light_sources: Vec::new(),
                shadow_areas: Vec::new(),
                backlight: None,
            },
            acoustics: AcousticsState {
                ambient_noise_level: 0.0,
                echo_characteristics: "dry".to_string(),
                sound_sources: Vec::new(),
            },
            olfactory_field: OlfactoryField {
                dominant_scents: Vec::new(),
                airflow: AirflowState {
                    direction: "still".to_string(),
                    speed: 0.0,
                    turbulence: 0.0,
                },
            },
            scene_mood: SceneMood::Neutral,
            physical_conditions: PhysicalConditions {
                temperature: Temperature {
                    ambient_celsius: 22.0,
                    felt_celsius: 22.0,
                    modifiers: Vec::new(),
                },
                surface_state: SurfaceState {
                    slipperiness: 0.0,
                    wetness: 0.0,
                    debris: Vec::new(),
                    notes: String::new(),
                },
                airborne: AirborneEffects {
                    fog_density: 0.0,
                    dust_density: 0.0,
                    smoke_density: 0.0,
                    visibility_range_m,
                    mana_haze: None,
                },
                precipitation: None,
                wind: WindState {
                    direction_deg: 0.0,
                    speed_ms: 0.0,
                    gust: false,
                },
            },
            mana_field: ManaField {
                ambient_density: 0.0,
                ambient_attribute: ManaAttribute::Void,
                mana_sources: Vec::new(),
                character_presences: Vec::new(),
                flow: ManaFlow {
                    direction: "still".to_string(),
                    intensity: 0.0,
                    turbulence: 0.0,
                },
                interferences: Vec::new(),
            },
            entities: vec![
                SceneEntity {
                    entity_id: "actor".to_string(),
                    entity_kind: SceneEntityKind::Character,
                    position: Position {
                        x: 0.0,
                        y: 0.0,
                        z: None,
                    },
                    posture: "standing".to_string(),
                    display_name: "Actor".to_string(),
                    observable_facets: Vec::new(),
                },
                SceneEntity {
                    entity_id: "target".to_string(),
                    entity_kind: SceneEntityKind::Character,
                    position: Position {
                        x: 12.0,
                        y: 0.0,
                        z: None,
                    },
                    posture: "standing".to_string(),
                    display_name: "Target".to_string(),
                    observable_facets: Vec::new(),
                },
            ],
            observable_signals: ObservableSignals {
                visual_signals: Vec::new(),
                audio_signals: Vec::new(),
                mana_signals: Vec::new(),
            },
            private_state: ScenePrivateState {
                hidden_facts: Vec::new(),
                reveal_triggers: Vec::new(),
                source_constraint_ids: Vec::new(),
            },
            event_stream: Vec::new(),
            uncertainty_notes: Vec::new(),
        }
    }

    fn skill() -> Skill {
        Skill {
            skill_id: "dash".to_string(),
            name: "Dash".to_string(),
            description: String::new(),
            skill_kind: SkillKind::Active,
            activation: SkillActivation {
                activation_time: ActivationTime::Instant,
                trigger_conditions: Vec::new(),
                cooldown: Some(2),
                uses_per_scene: None,
                uses_per_day: None,
            },
            effect_contract: SkillEffectContract {
                primary_effects: Vec::new(),
                secondary_effects: Vec::new(),
                target_kind: TargetKind::Character,
                target_count: TargetCount::Single,
                range_m: Some(20.0),
                area_of_effect: None,
                duration_turns: None,
                attribute_modifier: None,
                mana_attribute: None,
                allowed_target_kinds: vec![TargetKind::Character],
                allowed_state_domains: vec!["body".to_string()],
                max_intensity_tier: EffectIntensityTier::Moderate,
                allows_injury: true,
                allows_position_change: false,
                allows_knowledge_reveal: false,
            },
            requirements: SkillRequirements {
                minimum_attributes: Vec::new(),
                required_skills: Vec::new(),
                required_knowledge: Vec::new(),
                prohibited_conditions: Vec::new(),
                material_components: Vec::new(),
                cost: CostProfile {
                    mana_reserve_delta: Some(-10.0),
                    fatigue_delta: None,
                    cooldown_turns: Some(2),
                    material_refs: Vec::new(),
                    required_conditions: Vec::new(),
                },
            },
            metadata: SkillMetadata {
                tags: Vec::new(),
                source: None,
                learning_difficulty: LearningDifficulty::Common,
                rarity: SkillRarity::Common,
            },
            schema_version: "0.1".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn plan_with_delta(payload: serde_json::Value) -> StateUpdatePlan {
        StateUpdatePlan {
            scene_delta: None,
            character_state_deltas: vec![CharacterStateDelta {
                character_id: "target".to_string(),
                temporary_state_delta: payload,
                outward_body_signals: Vec::new(),
            }],
            subjective_update_refs: Vec::new(),
            new_memory_entries: Vec::new(),
            soft_effects: Vec::new(),
            blocked_effects: Vec::new(),
            validation_warnings: Vec::new(),
            consistency_notes: Vec::new(),
        }
    }

    fn blocked_reason(plan: StateUpdatePlan, skill: Skill, actor: CharacterRecord) -> String {
        let result = EffectValidator::validate_state_update(
            &plan,
            &[skill],
            &[actor, character("target")],
            &scene(100.0),
        )
        .expect("validation result");
        result.blocked_effects[0].reason_code.clone()
    }

    #[test]
    fn blocks_skill_when_actor_cooldown_is_active() {
        let mut actor = character("actor");
        actor.temporary_state.mana_reserve_current = Some(100.0);
        actor.temporary_state.cooldowns.push(CooldownState {
            ability_id: "dash".to_string(),
            remaining_turns: 1,
        });

        let reason = blocked_reason(
            plan_with_delta(json!({
                "actor_id": "actor",
                "skill_id": "dash",
                "target_ids": ["target"]
            })),
            skill(),
            actor,
        );

        assert_eq!(reason, "skill_on_cooldown");
    }

    #[test]
    fn blocks_skill_when_material_component_cannot_be_verified() {
        let mut actor = character("actor");
        actor.temporary_state.mana_reserve_current = Some(100.0);
        let mut skill = skill();
        skill
            .requirements
            .material_components
            .push("jade-token".to_string());

        let reason = blocked_reason(
            plan_with_delta(json!({
                "actor_id": "actor",
                "skill_id": "dash",
                "target_ids": ["target"]
            })),
            skill,
            actor,
        );

        assert_eq!(reason, "unverified_material_component:jade-token");
    }

    #[test]
    fn blocks_skill_targets_beyond_contract_range() {
        let mut actor = character("actor");
        actor.temporary_state.mana_reserve_current = Some(100.0);
        let mut skill = skill();
        skill.effect_contract.range_m = Some(5.0);

        let reason = blocked_reason(
            plan_with_delta(json!({
                "actor_id": "actor",
                "skill_id": "dash",
                "target_ids": ["target"]
            })),
            skill,
            actor,
        );

        assert_eq!(reason, "target_out_of_range:target");
    }

    #[test]
    fn blocks_line_of_sight_when_visibility_is_too_low() {
        let mut actor = character("actor");
        actor.temporary_state.mana_reserve_current = Some(100.0);
        let mut skill = skill();
        skill
            .activation
            .trigger_conditions
            .push(ActivationCondition::TargetInLineOfSight);

        let result = EffectValidator::validate_state_update(
            &plan_with_delta(json!({
                "actor_id": "actor",
                "skill_id": "dash",
                "target_ids": ["target"]
            })),
            &[skill],
            &[actor, character("target")],
            &scene(5.0),
        )
        .expect("validation result");

        assert_eq!(
            result.blocked_effects[0].reason_code,
            "line_of_sight_blocked:target"
        );
    }

    #[test]
    fn blocks_interrupt_reaction_when_actor_is_not_interrupt_ready() {
        let mut actor = character("actor");
        actor.temporary_state.mana_reserve_current = Some(100.0);
        let mut skill = skill();
        skill.skill_kind = SkillKind::Reaction;
        skill.activation.activation_time = ActivationTime::Reaction;
        skill.metadata.tags.push("interrupt".to_string());

        let reason = blocked_reason(
            plan_with_delta(json!({
                "actor_id": "actor",
                "skill_id": "dash",
                "target_ids": ["target"]
            })),
            skill,
            actor,
        );

        assert_eq!(reason, "interrupt_not_ready");
    }

    #[test]
    fn allows_interrupt_reaction_when_actor_is_interrupt_ready() {
        let mut actor = character("actor");
        actor.temporary_state.mana_reserve_current = Some(100.0);
        actor
            .temporary_state
            .active_conditions
            .push(ConditionState {
                condition_id: "c1".to_string(),
                domain: StateDomain::Body,
                condition_kind: "interrupt_ready".to_string(),
                intensity: 1.0,
                source_id: None,
            });
        let mut skill = skill();
        skill.skill_kind = SkillKind::Reaction;
        skill.activation.activation_time = ActivationTime::Reaction;
        skill.metadata.tags.push("interrupt".to_string());

        let result = EffectValidator::validate_state_update(
            &plan_with_delta(json!({
                "actor_id": "actor",
                "skill_id": "dash",
                "target_ids": ["target"]
            })),
            &[skill],
            &[actor, character("target")],
            &scene(100.0),
        )
        .expect("validation result");

        assert!(result.blocked_effects.is_empty());
        assert_eq!(result.valid_character_deltas.len(), 1);
    }

    #[test]
    fn blocks_passive_reaction_when_passive_field_is_not_active() {
        let mut actor = character("actor");
        actor.temporary_state.mana_reserve_current = Some(100.0);
        let mut skill = skill();
        skill.skill_kind = SkillKind::Passive;
        skill.effect_contract.allowed_state_domains = vec!["scene".to_string()];

        let reason = blocked_reason(
            plan_with_delta(json!({
                "actor_id": "actor",
                "skill_id": "dash",
                "target_ids": ["target"]
            })),
            skill,
            actor,
        );

        assert_eq!(reason, "passive_field_not_active");
    }
}
