//! World editor validator
//!
//! Validates world editor patches before commit.

use crate::agent::models::*;

/// World editor validator
pub struct WorldEditorValidator;

impl WorldEditorValidator {
    /// Validate a location node
    pub fn validate_location(location: &LocationNode) -> Result<Vec<ValidationIssue>, String> {
        let mut issues = Vec::new();

        if location.location_id.trim().is_empty() {
            issues.push(error("location_id", "location_id cannot be empty"));
        }
        if location.name.trim().is_empty() {
            issues.push(error("name", "location name cannot be empty"));
        }
        if location.type_label.trim().is_empty() {
            issues.push(error("type_label", "type_label cannot be empty"));
        }
        if location
            .parent_id
            .as_ref()
            .is_some_and(|parent_id| parent_id == &location.location_id)
        {
            issues.push(error("parent_id", "location cannot be its own parent"));
        }
        if location.schema_version.trim().is_empty() {
            issues.push(error("schema_version", "schema_version cannot be empty"));
        }
        if !location.metadata.is_object() {
            issues.push(warning(
                "metadata",
                "metadata should be a JSON object for forward-compatible editor patches",
            ));
        }
        for (index, alias) in location.aliases.iter().enumerate() {
            if alias.alias.trim().is_empty() {
                issues.push(error(
                    format!("aliases[{}].alias", index),
                    "alias cannot be empty",
                ));
            }
            if alias.normalized_alias.trim().is_empty() {
                issues.push(error(
                    format!("aliases[{}].normalized_alias", index),
                    "normalized_alias cannot be empty",
                ));
            }
        }

        Ok(issues)
    }

    /// Validate a location edge
    pub fn validate_edge(edge: &LocationEdge) -> Result<Vec<ValidationIssue>, String> {
        let mut issues = Vec::new();

        if edge.edge_id.trim().is_empty() {
            issues.push(error("edge_id", "edge_id cannot be empty"));
        }
        if edge.from_location_id.trim().is_empty() {
            issues.push(error(
                "from_location_id",
                "from_location_id cannot be empty",
            ));
        }
        if edge.to_location_id.trim().is_empty() {
            issues.push(error("to_location_id", "to_location_id cannot be empty"));
        }
        if edge.from_location_id == edge.to_location_id {
            issues.push(error(
                "to_location_id",
                "location edge cannot point to the same location",
            ));
        }
        if !edge.terrain_cost.is_finite() || edge.terrain_cost <= 0.0 {
            issues.push(error(
                "terrain_cost",
                "terrain_cost must be a positive finite number",
            ));
        }
        if !edge.safety_cost.is_finite() || edge.safety_cost <= 0.0 {
            issues.push(error(
                "safety_cost",
                "safety_cost must be a positive finite number",
            ));
        }
        if let Some(distance) = &edge.distance_km {
            validate_min_max(
                &mut issues,
                "distance_km",
                distance.min_km,
                distance.max_km,
                "distance",
            );
        }
        if let Some(travel_time) = &edge.travel_time {
            for (field, estimate) in [
                ("walking", &travel_time.walking),
                ("horse", &travel_time.horse),
                ("carriage", &travel_time.carriage),
                ("boat", &travel_time.boat),
                ("flying", &travel_time.flying),
                ("teleport", &travel_time.teleport),
            ] {
                if let Some(estimate) = estimate {
                    validate_min_max(
                        &mut issues,
                        format!("travel_time.{}", field),
                        estimate.min_hours,
                        estimate.max_hours,
                        "travel time",
                    );
                }
            }
        }
        for (index, modifier) in edge.seasonal_modifiers.iter().enumerate() {
            if !modifier.terrain_cost_modifier.is_finite() || modifier.terrain_cost_modifier < 0.0 {
                issues.push(error(
                    format!("seasonal_modifiers[{}].terrain_cost_modifier", index),
                    "terrain cost modifier must be a non-negative finite number",
                ));
            }
            if !modifier.safety_cost_modifier.is_finite() || modifier.safety_cost_modifier < 0.0 {
                issues.push(error(
                    format!("seasonal_modifiers[{}].safety_cost_modifier", index),
                    "safety cost modifier must be a non-negative finite number",
                ));
            }
        }

        Ok(issues)
    }

    /// Validate a knowledge entry
    pub fn validate_knowledge(entry: &KnowledgeEntry) -> Result<Vec<ValidationIssue>, String> {
        let mut issues = Vec::new();

        if entry.knowledge_id.trim().is_empty() {
            issues.push(error("knowledge_id", "knowledge_id cannot be empty"));
        }
        if entry.schema_version.trim().is_empty() {
            issues.push(error("schema_version", "schema_version cannot be empty"));
        }
        if entry.content.is_null() {
            issues.push(error("content", "content cannot be null"));
        }
        validate_access_policy(&mut issues, &entry.access_policy);
        match (&entry.kind, &entry.subject) {
            (KnowledgeKind::WorldFact, KnowledgeSubject::World)
            | (KnowledgeKind::RegionFact, KnowledgeSubject::Region(_))
            | (KnowledgeKind::FactionFact, KnowledgeSubject::Faction(_))
            | (KnowledgeKind::CharacterFacet, KnowledgeSubject::Character { .. })
            | (KnowledgeKind::HistoricalEvent, KnowledgeSubject::Event { .. }) => {}
            (KnowledgeKind::Memory, _) => {}
            _ => issues.push(warning(
                "subject",
                "knowledge kind and subject type look inconsistent",
            )),
        }
        if matches!(entry.subject, KnowledgeSubject::Character { .. })
            && matches!(entry.subject_awareness, SubjectAwareness::Unaware { .. })
            && entry.apparent_content.is_none()
        {
            issues.push(warning(
                "apparent_content",
                "unaware character facets should usually provide apparent_content or self_belief",
            ));
        }
        validate_knowledge_content_schema(&mut issues, entry);

        Ok(issues)
    }

    /// Validate a character record
    pub fn validate_character(character: &CharacterRecord) -> Result<Vec<ValidationIssue>, String> {
        let mut issues = Vec::new();

        if character.character_id.trim().is_empty() {
            issues.push(error("character_id", "character_id cannot be empty"));
        }
        if character.mind_model_card_knowledge_id.trim().is_empty() {
            issues.push(error(
                "mind_model_card_knowledge_id",
                "mind_model_card_knowledge_id cannot be empty",
            ));
        }
        if character.schema_version.trim().is_empty() {
            issues.push(error("schema_version", "schema_version cannot be empty"));
        }
        for (field, value) in [
            (
                "base_attributes.physical",
                character.base_attributes.physical,
            ),
            ("base_attributes.agility", character.base_attributes.agility),
            (
                "base_attributes.endurance",
                character.base_attributes.endurance,
            ),
            ("base_attributes.insight", character.base_attributes.insight),
            (
                "base_attributes.mana_power",
                character.base_attributes.mana_power,
            ),
            (
                "base_attributes.soul_strength",
                character.base_attributes.soul_strength,
            ),
        ] {
            if !value.is_finite() || value < 0.0 {
                issues.push(error(
                    field,
                    "base attribute must be a non-negative finite number",
                ));
            }
        }
        if let Some(value) = character.mana_expression_tendency_factor_override {
            if !value.is_finite() {
                issues.push(error(
                    "mana_expression_tendency_factor_override",
                    "override must be finite when present",
                ));
            }
        }
        let body = &character.baseline_body_profile;
        if body.species.trim().is_empty() {
            issues.push(error(
                "baseline_body_profile.species",
                "species cannot be empty",
            ));
        }
        if body.comfort_temperature_range.0 > body.comfort_temperature_range.1 {
            issues.push(error(
                "baseline_body_profile.comfort_temperature_range",
                "minimum comfort temperature cannot exceed maximum",
            ));
        }
        if !body.mana_sense_baseline.acuity.is_finite()
            || !(0.0..=1.0).contains(&body.mana_sense_baseline.acuity)
        {
            issues.push(error(
                "baseline_body_profile.mana_sense_baseline.acuity",
                "mana sense acuity must be in 0.0..=1.0",
            ));
        }
        if !body.mana_sense_baseline.overload_threshold.is_finite()
            || body.mana_sense_baseline.overload_threshold < 0.0
        {
            issues.push(error(
                "baseline_body_profile.mana_sense_baseline.overload_threshold",
                "overload_threshold must be a non-negative finite number",
            ));
        }
        let state = &character.temporary_state;
        validate_ratio(&mut issues, "temporary_state.fatigue", state.fatigue);
        validate_ratio(&mut issues, "temporary_state.pain_load", state.pain_load);
        if let Some(mana) = state.mana_reserve_current {
            if !mana.is_finite() || mana < 0.0 {
                issues.push(error(
                    "temporary_state.mana_reserve_current",
                    "mana_reserve_current must be a non-negative finite number",
                ));
            }
        }
        if !state.mana_expression.display_ratio.is_finite()
            || state.mana_expression.display_ratio < 0.0
        {
            issues.push(error(
                "temporary_state.mana_expression.display_ratio",
                "display_ratio must be a non-negative finite number",
            ));
        }
        if !state.mana_expression.pressure_ratio.is_finite()
            || state.mana_expression.pressure_ratio < 0.0
        {
            issues.push(error(
                "temporary_state.mana_expression.pressure_ratio",
                "pressure_ratio must be a non-negative finite number",
            ));
        }
        validate_ratio(
            &mut issues,
            "temporary_state.environmental_exposure.cold_strain",
            state.environmental_exposure.cold_strain,
        );
        validate_ratio(
            &mut issues,
            "temporary_state.environmental_exposure.heat_strain",
            state.environmental_exposure.heat_strain,
        );
        validate_ratio(
            &mut issues,
            "temporary_state.environmental_exposure.respiration_strain",
            state.environmental_exposure.respiration_strain,
        );
        for (index, condition) in state.active_conditions.iter().enumerate() {
            if condition.condition_id.trim().is_empty() {
                issues.push(error(
                    format!("temporary_state.active_conditions[{}].condition_id", index),
                    "condition_id cannot be empty",
                ));
            }
            if !condition.intensity.is_finite() || condition.intensity < 0.0 {
                issues.push(error(
                    format!("temporary_state.active_conditions[{}].intensity", index),
                    "condition intensity must be a non-negative finite number",
                ));
            }
        }
        for (index, suppression) in state.mana_suppression.iter().enumerate() {
            if suppression.source_id.trim().is_empty() {
                issues.push(error(
                    format!("temporary_state.mana_suppression[{}].source_id", index),
                    "source_id cannot be empty",
                ));
            }
            if !suppression.multiplier.is_finite() || suppression.multiplier < 0.0 {
                issues.push(error(
                    format!("temporary_state.mana_suppression[{}].multiplier", index),
                    "suppression multiplier must be a non-negative finite number",
                ));
            }
        }
        for (index, cooldown) in state.cooldowns.iter().enumerate() {
            if cooldown.ability_id.trim().is_empty() {
                issues.push(error(
                    format!("temporary_state.cooldowns[{}].ability_id", index),
                    "ability_id cannot be empty",
                ));
            }
        }

        Ok(issues)
    }
}

/// Validation issue
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub field_path: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationSeverity {
    Warning,
    Error,
}

fn error(field_path: impl Into<String>, message: impl Into<String>) -> ValidationIssue {
    ValidationIssue {
        severity: ValidationSeverity::Error,
        field_path: field_path.into(),
        message: message.into(),
    }
}

fn warning(field_path: impl Into<String>, message: impl Into<String>) -> ValidationIssue {
    ValidationIssue {
        severity: ValidationSeverity::Warning,
        field_path: field_path.into(),
        message: message.into(),
    }
}

fn validate_min_max(
    issues: &mut Vec<ValidationIssue>,
    field_path: impl Into<String>,
    min: f64,
    max: f64,
    label: &str,
) {
    let field_path = field_path.into();
    if !min.is_finite() || !max.is_finite() {
        issues.push(error(
            field_path,
            format!("{} bounds must be finite numbers", label),
        ));
    } else if min < 0.0 || max < 0.0 {
        issues.push(error(
            field_path,
            format!("{} bounds must be non-negative", label),
        ));
    } else if min > max {
        issues.push(error(
            field_path,
            format!("{} min cannot exceed max", label),
        ));
    }
}

fn validate_ratio(issues: &mut Vec<ValidationIssue>, field_path: &str, value: f64) {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        issues.push(error(field_path, "value must be in 0.0..=1.0"));
    }
}

fn validate_access_policy(issues: &mut Vec<ValidationIssue>, policy: &AccessPolicy) {
    let has_god_only = policy
        .scope
        .iter()
        .any(|scope| matches!(scope, AccessScope::GodOnly));
    if has_god_only && !policy.known_by.is_empty() {
        issues.push(error(
            "access_policy.known_by",
            "GodOnly knowledge cannot also grant known_by access",
        ));
    }
    if has_god_only && policy.scope.len() > 1 {
        issues.push(error(
            "access_policy.scope",
            "GodOnly must not be combined with other access scopes",
        ));
    }
    if has_god_only && !policy.conditions.is_empty() {
        issues.push(error(
            "access_policy.conditions",
            "GodOnly knowledge cannot also grant conditional access",
        ));
    }
    for (index, character_id) in policy.known_by.iter().enumerate() {
        if character_id.trim().is_empty() {
            issues.push(error(
                format!("access_policy.known_by[{}]", index),
                "known_by character_id cannot be empty",
            ));
        }
    }
    for (index, scope) in policy.scope.iter().enumerate() {
        match scope {
            AccessScope::Region(value)
            | AccessScope::Faction(value)
            | AccessScope::Realm(value)
            | AccessScope::Role(value)
            | AccessScope::Bloodline(value)
                if value.trim().is_empty() =>
            {
                issues.push(error(
                    format!("access_policy.scope[{}]", index),
                    "access scope value cannot be empty",
                ));
            }
            _ => {}
        }
    }
}

fn validate_knowledge_content_schema(issues: &mut Vec<ValidationIssue>, entry: &KnowledgeEntry) {
    match (&entry.kind, &entry.subject) {
        (
            KnowledgeKind::CharacterFacet,
            KnowledgeSubject::Character {
                facet: CharacterFacetType::MindModelCard,
                ..
            },
        ) => {
            if let Err(parse_error) =
                serde_json::from_value::<MindModelCardContent>(entry.content.clone())
            {
                issues.push(error(
                    "content",
                    format!("MindModelCard content schema invalid: {}", parse_error),
                ));
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::WorldEditorValidator;
    use crate::agent::models::knowledge::{
        AccessPolicy, AccessScope, CharacterFacetType, KnowledgeEntry, KnowledgeKind,
        KnowledgeMetadata, KnowledgeSubject, SubjectAwareness,
    };
    use chrono::Utc;
    use serde_json::json;

    fn build_mind_model_entry(content: serde_json::Value) -> KnowledgeEntry {
        let now = Utc::now();
        KnowledgeEntry {
            knowledge_id: "knowledge_mind_model_test".to_string(),
            kind: KnowledgeKind::CharacterFacet,
            subject: KnowledgeSubject::Character {
                id: "character_test".to_string(),
                facet: CharacterFacetType::MindModelCard,
            },
            content,
            apparent_content: None,
            access_policy: AccessPolicy {
                known_by: Vec::new(),
                scope: vec![AccessScope::Public],
                conditions: Vec::new(),
            },
            subject_awareness: SubjectAwareness::Aware,
            metadata: KnowledgeMetadata {
                created_at: now,
                updated_at: now,
                valid_from: None,
                valid_until: None,
                source_session_id: None,
                source_scene_turn_id: None,
                derived_from_event_id: None,
                emotional_weight: None,
                last_accessed_at: None,
                source: None,
            },
            valid_from: None,
            valid_until: None,
            source_session_id: None,
            source_scene_turn_id: None,
            derived_from_event_id: None,
            schema_version: "0.1".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn validate_knowledge_accepts_complete_mind_model_card_content() {
        let entry = build_mind_model_entry(json!({
            "summary_text": "谨慎分析后再行动",
            "attention_biases": ["威胁信号", "权力差距"],
            "risk_tolerance": "Moderate",
            "default_social_strategy": "先观察再交换信息",
            "value_priorities": ["生存", "情报"],
            "cognitive_patterns": ["先收集证据", "避免正面冲突"],
            "extensions": {}
        }));

        let issues = WorldEditorValidator::validate_knowledge(&entry).expect("validation");
        assert!(issues.is_empty());
    }

    #[test]
    fn validate_knowledge_rejects_sparse_mind_model_card_content() {
        let entry = build_mind_model_entry(json!({
            "summary_text": "只有摘要"
        }));

        let issues = WorldEditorValidator::validate_knowledge(&entry).expect("validation");
        assert!(issues.iter().any(|issue| {
            issue.field_path == "content"
                && issue.message.contains("MindModelCard content schema invalid")
        }));
    }
}
