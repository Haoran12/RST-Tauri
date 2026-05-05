//! Scene extractor
//!
//! Structured Agent LLM execution for parsing recent free text into scene/user deltas.

use uuid::Uuid;

use crate::agent::llm_support::default_agent_api_config;
use crate::agent::models::{
    SceneStateExtractorInput, SceneStateExtractorOutput, UserInputAuthorityClass, UserInputKind,
};
use crate::agent::prompting::{
    AgentLlmNode, PromptBuildOptions, PromptBuilder, PromptInputSection, PromptPriority,
    PromptRequestOptions,
};
use crate::commands::chat_commands::create_provider;
use crate::storage::st_resources::ApiConfig;

pub struct SceneExtractor {
    api_config: ApiConfig,
    prompt_builder: PromptBuilder,
}

impl SceneExtractor {
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
        input: SceneStateExtractorInput,
    ) -> Result<SceneStateExtractorOutput, SceneExtractorError> {
        self.validate_input(&input)?;

        let schema = scene_extractor_output_schema();
        let bundle = self
            .prompt_builder
            .build_bundle(
                AgentLlmNode::SceneStateExtractor,
                &input,
                PromptBuildOptions {
                    task_instructions: vec![
                        "Preserve the raw recent free text verbatim in user_input_delta.raw_text."
                            .to_string(),
                        "Downgrade ambiguous or over-authoritative claims into warnings or blocked classes."
                            .to_string(),
                        "Emit only candidate scene updates and user intent/state deltas.".to_string(),
                    ],
                    output_schema_json: Some(schema.clone()),
                    input_sections: scene_extractor_input_sections(),
                    ..Default::default()
                },
            )
            .map_err(SceneExtractorError::PromptBuild)?;

        let request = self
            .prompt_builder
            .build_chat_request(
                &bundle,
                PromptRequestOptions::new(Uuid::new_v4().to_string(), self.api_config.id.clone()),
            )
            .map_err(SceneExtractorError::PromptBuild)?;

        let provider =
            create_provider(&self.api_config, None).map_err(SceneExtractorError::LlmError)?;
        let raw_output = provider
            .chat_structured(request, schema)
            .await
            .map_err(SceneExtractorError::LlmError)?;

        let output: SceneStateExtractorOutput =
            serde_json::from_value(raw_output).map_err(|e| {
                SceneExtractorError::SchemaError(format!(
                    "Failed to decode structured SceneStateExtractorOutput: {e}"
                ))
            })?;

        self.validate_output(&output, &input)?;
        Ok(output)
    }

    fn validate_input(&self, input: &SceneStateExtractorInput) -> Result<(), SceneExtractorError> {
        if input.scene_turn_id.trim().is_empty() {
            return Err(SceneExtractorError::InputValidation(
                "scene_turn_id must not be empty".to_string(),
            ));
        }
        if input.recent_free_text.trim().is_empty() {
            return Err(SceneExtractorError::InputValidation(
                "recent_free_text must not be empty".to_string(),
            ));
        }
        if input.current_scene.scene_turn_id != input.scene_turn_id {
            return Err(SceneExtractorError::InputValidation(
                "current_scene.scene_turn_id must match input.scene_turn_id".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_output(
        &self,
        output: &SceneStateExtractorOutput,
        input: &SceneStateExtractorInput,
    ) -> Result<(), SceneExtractorError> {
        if output.user_input_delta.turn_id != input.scene_turn_id {
            return Err(SceneExtractorError::OutputValidation(
                "user_input_delta.turn_id must match input.scene_turn_id".to_string(),
            ));
        }

        if output.user_input_delta.raw_text != input.recent_free_text {
            return Err(SceneExtractorError::OutputValidation(
                "user_input_delta.raw_text must preserve recent_free_text verbatim".to_string(),
            ));
        }

        if let Some(scene_update) = &output.scene_update {
            if scene_update.scene_turn_id != input.scene_turn_id {
                return Err(SceneExtractorError::OutputValidation(
                    "scene_update.scene_turn_id must match input.scene_turn_id".to_string(),
                ));
            }
            if scene_update.scene_delta.scene_id != input.current_scene.scene_id {
                return Err(SceneExtractorError::OutputValidation(
                    "scene_update.scene_delta.scene_id must target current_scene.scene_id"
                        .to_string(),
                ));
            }
        }

        match &output.user_input_delta.kind {
            UserInputKind::MetaCommand { .. } => {
                if output.user_input_delta.authority_class
                    != UserInputAuthorityClass::SessionControl
                {
                    return Err(SceneExtractorError::OutputValidation(
                        "MetaCommand requires SessionControl authority_class".to_string(),
                    ));
                }
            }
            UserInputKind::DirectorHint { .. } => {
                if output.user_input_delta.authority_class != UserInputAuthorityClass::DirectorBias
                {
                    return Err(SceneExtractorError::OutputValidation(
                        "DirectorHint requires DirectorBias authority_class".to_string(),
                    ));
                }
            }
            UserInputKind::SceneNarration { .. } => {
                if output.user_input_delta.authority_class
                    == UserInputAuthorityClass::SessionControl
                {
                    return Err(SceneExtractorError::OutputValidation(
                        "SceneNarration cannot be classified as SessionControl".to_string(),
                    ));
                }
            }
            UserInputKind::CharacterRoleplay { character_id, .. } => {
                if let Some(player_character_id) = &input.session_context.player_character_id {
                    if character_id != player_character_id {
                        return Err(SceneExtractorError::OutputValidation(format!(
                            "CharacterRoleplay character_id '{}' must match player_character_id '{}'",
                            character_id, player_character_id
                        )));
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for SceneExtractor {
    fn default() -> Self {
        Self::new(default_agent_api_config())
    }
}

fn scene_extractor_input_sections() -> Vec<PromptInputSection> {
    vec![
        PromptInputSection {
            pointer: "/recent_free_text".to_string(),
            label: "recent_free_text".to_string(),
            priority: PromptPriority::P0Required,
        },
        PromptInputSection {
            pointer: "/current_scene".to_string(),
            label: "current_scene".to_string(),
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
            pointer: "/world_constraints".to_string(),
            label: "world_constraints".to_string(),
            priority: PromptPriority::P2Contextual,
        },
    ]
}

fn scene_extractor_output_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "scene_update",
            "user_input_delta",
            "provisional_truth_candidates",
            "conflict_warnings",
            "ambiguity_report"
        ],
        "properties": {
            "scene_update": {
                "anyOf": [
                    { "type": "null" },
                    { "type": "object" }
                ]
            },
            "user_input_delta": {
                "type": "object",
                "additionalProperties": false,
                "required": ["turn_id", "raw_text", "authority_class", "authority_notes", "kind"],
                "properties": {
                    "turn_id": { "type": "string" },
                    "raw_text": { "type": "string" },
                    "authority_class": { "type": "string" },
                    "authority_notes": { "type": "array", "items": { "type": "object" } },
                    "kind": { "type": "object" }
                }
            },
            "provisional_truth_candidates": { "type": "array", "items": { "type": "object" } },
            "conflict_warnings": { "type": "array", "items": { "type": "object" } },
            "ambiguity_report": { "type": "array", "items": { "type": "string" } }
        }
    })
}

#[derive(Debug, Clone)]
pub enum SceneExtractorError {
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
        AgentSessionContext, DayPhase, SceneModel, TimeAnchor, TimeContext, TimePrecision,
        UserInputAuthorityNote,
    };

    #[test]
    fn rejects_raw_text_mismatch() {
        let extractor = SceneExtractor::default();
        let input = sample_input();
        let mut output = sample_output(&input);
        output.user_input_delta.raw_text = "changed".to_string();

        let err = extractor
            .validate_output(&output, &input)
            .expect_err("raw_text mismatch should fail");
        assert!(
            matches!(err, SceneExtractorError::OutputValidation(message) if message.contains("raw_text"))
        );
    }

    fn sample_input() -> SceneStateExtractorInput {
        let anchor = TimeAnchor {
            calendar_id: "default".to_string(),
            ordinal: 1,
            precision: TimePrecision::Exact,
            display_text: "noon".to_string(),
        };
        SceneStateExtractorInput {
            scene_turn_id: "turn_1".to_string(),
            session_context: AgentSessionContext {
                session_id: "session_1".to_string(),
                session_kind: "mainline".to_string(),
                period_anchor: anchor.clone(),
                mainline_time_anchor: anchor.clone(),
                player_character_id: Some("char_1".to_string()),
                canon_status: "canon_candidate".to_string(),
            },
            recent_free_text: "I draw my sword.".to_string(),
            current_scene: SceneModel {
                scene_id: "scene_1".to_string(),
                scene_turn_id: "turn_1".to_string(),
                time_context: TimeContext {
                    time_anchor: anchor,
                    season: "spring".to_string(),
                    day_phase: DayPhase::Day,
                    weather_trend: "clear".to_string(),
                },
                spatial_layout: crate::agent::models::SpatialLayout {
                    layout_type: "yard".to_string(),
                    dimensions: None,
                    obstacles: vec![],
                    entrances: vec![],
                    zones: vec![],
                },
                lighting: crate::agent::models::LightingState {
                    ambient_level: 1.0,
                    light_sources: vec![],
                    shadow_areas: vec![],
                    backlight: None,
                },
                acoustics: crate::agent::models::AcousticsState {
                    ambient_noise_level: 0.0,
                    echo_characteristics: "open".to_string(),
                    sound_sources: vec![],
                },
                olfactory_field: crate::agent::models::OlfactoryField {
                    dominant_scents: vec![],
                    airflow: crate::agent::models::AirflowState {
                        direction: "still".to_string(),
                        speed: 0.0,
                        turbulence: 0.0,
                    },
                },
                scene_mood: crate::agent::models::SceneMood::Neutral,
                physical_conditions: crate::agent::models::PhysicalConditions {
                    temperature: crate::agent::models::Temperature {
                        ambient_celsius: 20.0,
                        felt_celsius: 20.0,
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
                        visibility_range_m: 20.0,
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
                entities: vec![],
                observable_signals: crate::agent::models::ObservableSignals {
                    visual_signals: vec![],
                    audio_signals: vec![],
                    mana_signals: vec![],
                },
                private_state: crate::agent::models::ScenePrivateState {
                    hidden_facts: vec![],
                    reveal_triggers: vec![],
                    source_constraint_ids: vec![],
                },
                event_stream: vec![],
                uncertainty_notes: vec![],
            },
            private_scene_constraints: vec![],
            truth_guidance: None,
            world_constraints: serde_json::json!({}),
        }
    }

    fn sample_output(input: &SceneStateExtractorInput) -> SceneStateExtractorOutput {
        SceneStateExtractorOutput {
            scene_update: None,
            user_input_delta: crate::agent::models::UserInputDelta {
                turn_id: input.scene_turn_id.clone(),
                raw_text: input.recent_free_text.clone(),
                authority_class: UserInputAuthorityClass::PlayerCharacterIntent,
                authority_notes: vec![UserInputAuthorityNote {
                    note_kind: "parsed".to_string(),
                    field_path: None,
                    reason: Utc::now().to_rfc3339(),
                }],
                kind: UserInputKind::CharacterRoleplay {
                    character_id: "char_1".to_string(),
                    intent_plan: crate::agent::models::IntentPlan {
                        character_id: "char_1".to_string(),
                        intent_kind: "draw_weapon".to_string(),
                        target_refs: vec![],
                        intended_actions: vec![],
                        priority: "high".to_string(),
                        commitment: "firm".to_string(),
                        rationale: "prepare".to_string(),
                    },
                    spoken_dialogue: None,
                    actions: vec![],
                    subjective_input: None,
                },
            },
            provisional_truth_candidates: vec![],
            conflict_warnings: vec![],
            ambiguity_report: vec![],
        }
    }
}
