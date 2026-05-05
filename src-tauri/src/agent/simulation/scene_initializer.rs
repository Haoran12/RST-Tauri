//! Scene initializer
//!
//! Structured Agent LLM execution for bootstrapping a candidate scene draft.

use std::collections::HashSet;

use uuid::Uuid;

use crate::agent::llm_support::default_agent_api_config;
use crate::agent::models::{
    AssumptionConfidence, AssumptionRisk, BlockedSceneAddition, SceneAssumption, SceneDetailDomain,
    SceneInitializationDraft, SceneInitializerInput,
};
use crate::agent::prompting::{
    AgentLlmNode, PromptBuildOptions, PromptBuilder, PromptInputSection, PromptPriority,
    PromptRequestOptions,
};
use crate::commands::chat_commands::create_provider;
use crate::storage::st_resources::ApiConfig;

pub struct SceneInitializer {
    api_config: ApiConfig,
    prompt_builder: PromptBuilder,
}

impl SceneInitializer {
    pub fn new(api_config: ApiConfig) -> Self {
        Self {
            api_config,
            prompt_builder: PromptBuilder::default(),
        }
    }

    pub fn with_prompt_builder(api_config: ApiConfig, prompt_builder: PromptBuilder) -> Self {
        Self {
            api_config,
            prompt_builder,
        }
    }

    pub async fn execute(
        &self,
        input: SceneInitializerInput,
    ) -> Result<SceneInitializationDraft, SceneInitializerError> {
        self.validate_input(&input)?;

        let schema = scene_initializer_output_schema();
        let bundle = self
            .prompt_builder
            .build_bundle(
                AgentLlmNode::SceneInitializer,
                &input,
                PromptBuildOptions {
                    task_instructions: vec![
                        "Bootstrap a structurally complete candidate scene draft.".to_string(),
                        "Record every inferred addition inside assumptions.".to_string(),
                        "Use blocked_additions instead of inventing forbidden details.".to_string(),
                    ],
                    output_schema_json: Some(schema.clone()),
                    input_sections: scene_initializer_input_sections(),
                    ..Default::default()
                },
            )
            .map_err(SceneInitializerError::PromptBuild)?;

        let request = self
            .prompt_builder
            .build_chat_request(
                &bundle,
                PromptRequestOptions::new(Uuid::new_v4().to_string(), self.api_config.id.clone()),
            )
            .map_err(SceneInitializerError::PromptBuild)?;

        let provider =
            create_provider(&self.api_config, None).map_err(SceneInitializerError::LlmError)?;
        let raw_output = provider
            .chat_structured(request, schema)
            .await
            .map_err(SceneInitializerError::LlmError)?;

        let output: SceneInitializationDraft = serde_json::from_value(raw_output).map_err(|e| {
            SceneInitializerError::SchemaError(format!(
                "Failed to decode structured SceneInitializationDraft: {e}"
            ))
        })?;

        self.validate_output(&output, &input)?;
        Ok(output)
    }

    fn validate_input(&self, input: &SceneInitializerInput) -> Result<(), SceneInitializerError> {
        if input.scene_turn_id.trim().is_empty() {
            return Err(SceneInitializerError::InputValidation(
                "scene_turn_id must not be empty".to_string(),
            ));
        }
        if input.world_id.trim().is_empty() {
            return Err(SceneInitializerError::InputValidation(
                "world_id must not be empty".to_string(),
            ));
        }
        if input.seed.scene_id.trim().is_empty() {
            return Err(SceneInitializerError::InputValidation(
                "seed.scene_id must not be empty".to_string(),
            ));
        }
        if input.generation_policy.max_generated_background_entities == 0
            && input.generation_policy.allow_transient_background_entities
        {
            return Err(SceneInitializerError::InputValidation(
                "allow_transient_background_entities=true requires max_generated_background_entities > 0"
                    .to_string(),
            ));
        }

        Ok(())
    }

    fn validate_output(
        &self,
        output: &SceneInitializationDraft,
        input: &SceneInitializerInput,
    ) -> Result<(), SceneInitializerError> {
        if output.scene_turn_id != input.scene_turn_id {
            return Err(SceneInitializerError::OutputValidation(format!(
                "scene_turn_id mismatch: expected '{}', got '{}'",
                input.scene_turn_id, output.scene_turn_id
            )));
        }

        if output.scene_model.scene_turn_id != input.scene_turn_id {
            return Err(SceneInitializerError::OutputValidation(
                "scene_model.scene_turn_id must match input.scene_turn_id".to_string(),
            ));
        }

        if output.scene_model.scene_id != input.seed.scene_id {
            return Err(SceneInitializerError::OutputValidation(format!(
                "scene_model.scene_id mismatch: expected '{}', got '{}'",
                input.seed.scene_id, output.scene_model.scene_id
            )));
        }

        let allowed_constraint_ids: HashSet<&str> = input
            .private_scene_constraints
            .iter()
            .map(|constraint| constraint.constraint_id.as_str())
            .collect();
        for constraint_id in &output.scene_model.private_state.source_constraint_ids {
            if !allowed_constraint_ids.contains(constraint_id.as_str()) {
                return Err(SceneInitializerError::OutputValidation(format!(
                    "scene_model.private_state.source_constraint_ids contains unknown constraint '{}'",
                    constraint_id
                )));
            }
        }

        if input.generation_policy.forbid_new_named_entities {
            for entity in &output.scene_model.entities {
                if matches!(
                    entity.entity_kind,
                    crate::agent::models::SceneEntityKind::BackgroundActor
                ) && !entity.display_name.trim().is_empty()
                {
                    return Err(SceneInitializerError::OutputValidation(
                        "forbid_new_named_entities=true forbids named background actors"
                            .to_string(),
                    ));
                }
            }
        }

        let required_participants: HashSet<&str> = input
            .participant_context
            .iter()
            .map(|participant| participant.character_id.as_str())
            .collect();
        let present_entities: HashSet<&str> = output
            .scene_model
            .entities
            .iter()
            .map(|entity| entity.entity_id.as_str())
            .collect();
        for character_id in required_participants {
            if !present_entities.contains(character_id) {
                return Err(SceneInitializerError::OutputValidation(format!(
                    "required participant '{}' missing from scene_model.entities",
                    character_id
                )));
            }
        }

        for assumption in &output.assumptions {
            validate_assumption(assumption)?;
        }
        for addition in &output.blocked_additions {
            validate_blocked_addition(addition)?;
        }

        Ok(())
    }
}

impl Default for SceneInitializer {
    fn default() -> Self {
        Self::new(default_agent_api_config())
    }
}

fn validate_assumption(assumption: &SceneAssumption) -> Result<(), SceneInitializerError> {
    if assumption.field_path.trim().is_empty() {
        return Err(SceneInitializerError::OutputValidation(
            "assumption.field_path must not be empty".to_string(),
        ));
    }
    if assumption.rationale.trim().is_empty() {
        return Err(SceneInitializerError::OutputValidation(
            "assumption.rationale must not be empty".to_string(),
        ));
    }

    match assumption.confidence {
        AssumptionConfidence::Low | AssumptionConfidence::Medium | AssumptionConfidence::High => {}
    }
    match assumption.risk {
        AssumptionRisk::Low | AssumptionRisk::Medium | AssumptionRisk::High => {}
    }

    Ok(())
}

fn validate_blocked_addition(addition: &BlockedSceneAddition) -> Result<(), SceneInitializerError> {
    match addition.attempted_domain {
        SceneDetailDomain::SpatialLayout
        | SceneDetailDomain::Lighting
        | SceneDetailDomain::Acoustics
        | SceneDetailDomain::OlfactoryField
        | SceneDetailDomain::PhysicalConditions
        | SceneDetailDomain::ManaField
        | SceneDetailDomain::SceneMood
        | SceneDetailDomain::BackgroundEntities
        | SceneDetailDomain::ObservableSignals => {}
    }

    if addition.reason_code.trim().is_empty() || addition.description.trim().is_empty() {
        return Err(SceneInitializerError::OutputValidation(
            "blocked_additions entries require non-empty reason_code and description".to_string(),
        ));
    }

    Ok(())
}

fn scene_initializer_input_sections() -> Vec<PromptInputSection> {
    vec![
        PromptInputSection {
            pointer: "/seed".to_string(),
            label: "scene_seed".to_string(),
            priority: PromptPriority::P0Required,
        },
        PromptInputSection {
            pointer: "/public_world_context".to_string(),
            label: "public_world_context".to_string(),
            priority: PromptPriority::P1DecisionCritical,
        },
        PromptInputSection {
            pointer: "/location_context".to_string(),
            label: "location_context".to_string(),
            priority: PromptPriority::P1DecisionCritical,
        },
        PromptInputSection {
            pointer: "/participant_context".to_string(),
            label: "participant_context".to_string(),
            priority: PromptPriority::P1DecisionCritical,
        },
        PromptInputSection {
            pointer: "/private_scene_constraints".to_string(),
            label: "private_scene_constraints".to_string(),
            priority: PromptPriority::P1DecisionCritical,
        },
        PromptInputSection {
            pointer: "/truth_guidance".to_string(),
            label: "truth_guidance".to_string(),
            priority: PromptPriority::P2Contextual,
        },
        PromptInputSection {
            pointer: "/continuity_context".to_string(),
            label: "continuity_context".to_string(),
            priority: PromptPriority::P2Contextual,
        },
        PromptInputSection {
            pointer: "/world_constraints".to_string(),
            label: "world_constraints".to_string(),
            priority: PromptPriority::P2Contextual,
        },
    ]
}

fn scene_initializer_output_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "scene_turn_id",
            "scene_model",
            "assumptions",
            "blocked_additions",
            "ambiguity_report",
            "validation_hints"
        ],
        "properties": {
            "scene_turn_id": { "type": "string" },
            "scene_model": { "type": "object" },
            "assumptions": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["field_path", "source", "confidence", "risk", "rationale"],
                    "properties": {
                        "field_path": { "type": "string" },
                        "source": { "type": "string" },
                        "confidence": { "type": "string", "enum": ["Low", "Medium", "High"] },
                        "risk": { "type": "string", "enum": ["Low", "Medium", "High"] },
                        "rationale": { "type": "string" }
                    }
                }
            },
            "blocked_additions": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["attempted_domain", "reason_code", "description"],
                    "properties": {
                        "attempted_domain": { "type": "string" },
                        "reason_code": { "type": "string" },
                        "description": { "type": "string" }
                    }
                }
            },
            "ambiguity_report": { "type": "array", "items": { "type": "string" } },
            "validation_hints": { "type": "array", "items": { "type": "string" } }
        }
    })
}

#[derive(Debug, Clone)]
pub enum SceneInitializerError {
    PromptBuild(String),
    InputValidation(String),
    OutputValidation(String),
    LlmError(String),
    SchemaError(String),
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::agent::models::{
        AgentSessionContext, AssumptionRisk, LocationAnchor, ParticipantEntryState,
        PublicWorldContext, SceneEntitySeed, SceneGenerationPolicy, SceneMood,
        SceneParticipantSeed, SceneSeed, SceneTransitionReason, TimeAnchor, TimeContextSeed,
        TimePrecision,
    };

    #[test]
    fn rejects_unknown_private_constraint_ids() {
        let initializer = SceneInitializer::default();
        let input = sample_input();
        let mut output = sample_output(&input);
        output.scene_model.private_state.source_constraint_ids = vec!["unknown".to_string()];

        let err = initializer
            .validate_output(&output, &input)
            .expect_err("unknown constraint id should fail");
        assert!(
            matches!(err, SceneInitializerError::OutputValidation(message) if message.contains("unknown constraint"))
        );
    }

    fn sample_input() -> SceneInitializerInput {
        SceneInitializerInput {
            scene_turn_id: "turn_1".to_string(),
            world_id: "world_1".to_string(),
            session_context: AgentSessionContext {
                session_id: "session_1".to_string(),
                session_kind: "mainline".to_string(),
                period_anchor: TimeAnchor {
                    calendar_id: "default".to_string(),
                    ordinal: 1,
                    precision: TimePrecision::Exact,
                    display_text: "daybreak".to_string(),
                },
                mainline_time_anchor: TimeAnchor {
                    calendar_id: "default".to_string(),
                    ordinal: 1,
                    precision: TimePrecision::Exact,
                    display_text: "daybreak".to_string(),
                },
                player_character_id: Some("char_1".to_string()),
                canon_status: "canon_candidate".to_string(),
            },
            seed: SceneSeed {
                scene_id: "scene_1".to_string(),
                transition_reason: SceneTransitionReason::InitialScene,
                time_seed: TimeContextSeed {
                    season: Some("spring".to_string()),
                    day_phase: Some("Day".to_string()),
                    absolute_time_hint: None,
                    elapsed_from_previous: None,
                    weather_trend: Some("clear".to_string()),
                },
                location_anchor: LocationAnchor {
                    location_id: Some("loc_1".to_string()),
                    fallback_region_id: None,
                    location_type: "courtyard".to_string(),
                    known_exits: vec!["gate".to_string()],
                },
                required_participant_ids: vec!["char_1".to_string()],
                requested_mood: Some(SceneMood::Neutral),
                required_entities: vec![SceneEntitySeed {
                    entity_id: Some("char_1".to_string()),
                    entity_kind: "Character".to_string(),
                    display_label: Some("Ran".to_string()),
                    persistence: crate::agent::models::EntityPersistence::Persistent,
                    required: true,
                    position_hint: Some("center".to_string()),
                }],
            },
            public_world_context: PublicWorldContext {
                world_summary: "test world".to_string(),
                public_rules: vec!["rule".to_string()],
                ambient_defaults: serde_json::json!({}),
            },
            location_context: serde_json::json!({}),
            participant_context: vec![SceneParticipantSeed {
                character_id: "char_1".to_string(),
                public_appearance_summary: "appears calm".to_string(),
                entry_state: ParticipantEntryState::AlreadyPresent,
                position_hint: Some("center".to_string()),
            }],
            continuity_context: None,
            private_scene_constraints: vec![crate::agent::models::ScenePrivateConstraint {
                constraint_id: "constraint_1".to_string(),
                source_knowledge_id: None,
                scope: crate::agent::models::PrivateConstraintScope::SceneBound,
                applies_to: vec!["scene_1".to_string()],
                constraint_kind: "HiddenPresence".to_string(),
                constraint_summary: "hidden watcher".to_string(),
                allowed_uses: vec![
                    crate::agent::models::PrivateConstraintUse::InitializeHiddenState,
                ],
                reveal_conditions: vec![],
            }],
            truth_guidance: None,
            world_constraints: serde_json::json!({}),
            generation_policy: SceneGenerationPolicy {
                detail_level: crate::agent::models::DetailLevel::Moderate,
                allowed_detail_domains: vec![SceneDetailDomain::SpatialLayout],
                allow_transient_background_entities: false,
                max_generated_background_entities: 1,
                forbid_new_named_entities: true,
                require_user_confirmation_above: AssumptionRisk::High,
            },
        }
    }

    fn sample_output(input: &SceneInitializerInput) -> SceneInitializationDraft {
        SceneInitializationDraft {
            scene_turn_id: input.scene_turn_id.clone(),
            scene_model: crate::agent::models::SceneModel {
                scene_id: input.seed.scene_id.clone(),
                scene_turn_id: input.scene_turn_id.clone(),
                time_context: crate::agent::models::TimeContext {
                    time_anchor: input.session_context.period_anchor.clone(),
                    season: "spring".to_string(),
                    day_phase: crate::agent::models::DayPhase::Day,
                    weather_trend: "clear".to_string(),
                },
                spatial_layout: crate::agent::models::SpatialLayout {
                    layout_type: "courtyard".to_string(),
                    dimensions: None,
                    obstacles: vec![],
                    entrances: vec![],
                    zones: vec![],
                },
                lighting: crate::agent::models::LightingState {
                    ambient_level: 0.8,
                    light_sources: vec![],
                    shadow_areas: vec![],
                    backlight: None,
                },
                acoustics: crate::agent::models::AcousticsState {
                    ambient_noise_level: 0.1,
                    echo_characteristics: "open".to_string(),
                    sound_sources: vec![],
                },
                olfactory_field: crate::agent::models::OlfactoryField {
                    dominant_scents: vec![],
                    airflow: crate::agent::models::AirflowState {
                        direction: "north".to_string(),
                        speed: 0.0,
                        turbulence: 0.0,
                    },
                },
                scene_mood: crate::agent::models::SceneMood::Neutral,
                physical_conditions: crate::agent::models::PhysicalConditions {
                    temperature: crate::agent::models::Temperature {
                        ambient_celsius: 22.0,
                        felt_celsius: 22.0,
                        modifiers: vec![],
                    },
                    surface_state: crate::agent::models::SurfaceState {
                        slipperiness: 0.0,
                        wetness: 0.0,
                        debris: vec![],
                        notes: String::new(),
                    },
                    airborne: crate::agent::models::AirborneEffects {
                        fog_density: 0.0,
                        dust_density: 0.0,
                        smoke_density: 0.0,
                        visibility_range_m: 30.0,
                        mana_haze: None,
                    },
                    precipitation: None,
                    wind: crate::agent::models::WindState {
                        direction_deg: 0.0,
                        speed_ms: 0.0,
                        gust: false,
                    },
                },
                mana_field: crate::agent::models::ManaField {
                    ambient_density: 0.0,
                    ambient_attribute: crate::agent::models::ManaAttribute::Water,
                    mana_sources: vec![],
                    character_presences: vec![],
                    flow: crate::agent::models::ManaFlow {
                        direction: "still".to_string(),
                        intensity: 0.0,
                        turbulence: 0.0,
                    },
                    interferences: vec![],
                },
                entities: vec![crate::agent::models::SceneEntity {
                    entity_id: "char_1".to_string(),
                    entity_kind: crate::agent::models::SceneEntityKind::Character,
                    position: crate::agent::models::Position {
                        x: 0.0,
                        y: 0.0,
                        z: None,
                    },
                    posture: "standing".to_string(),
                    display_name: "Ran".to_string(),
                    observable_facets: vec![],
                }],
                observable_signals: crate::agent::models::ObservableSignals {
                    visual_signals: vec![],
                    audio_signals: vec![],
                    mana_signals: vec![],
                },
                private_state: crate::agent::models::ScenePrivateState {
                    hidden_facts: vec![],
                    reveal_triggers: vec![],
                    source_constraint_ids: vec!["constraint_1".to_string()],
                },
                event_stream: vec![],
                uncertainty_notes: vec![],
            },
            assumptions: vec![SceneAssumption {
                field_path: "/lighting".to_string(),
                source: crate::agent::models::SceneAssumptionSource::PublicWorldContext,
                confidence: AssumptionConfidence::Medium,
                risk: AssumptionRisk::Low,
                rationale: "ambient default".to_string(),
            }],
            blocked_additions: vec![],
            ambiguity_report: vec![],
            validation_hints: vec![Utc::now().to_rfc3339()],
        }
    }
}
