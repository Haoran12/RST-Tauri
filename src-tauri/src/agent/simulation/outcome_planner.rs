//! Outcome planner
//!
//! Structured Agent LLM execution for candidate outcome/state update planning.

use std::collections::HashSet;

use uuid::Uuid;

use crate::agent::llm_support::default_agent_api_config;
use crate::agent::models::{OutcomePlannerInput, OutcomePlannerOutput};
use crate::agent::prompting::{
    AgentLlmNode, PromptBuildOptions, PromptBuilder, PromptInputSection, PromptPriority,
    PromptRequestOptions,
};
use crate::commands::chat_commands::create_provider;
use crate::storage::st_resources::ApiConfig;

pub struct OutcomePlanner {
    api_config: ApiConfig,
    prompt_builder: PromptBuilder,
}

impl OutcomePlanner {
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
        input: OutcomePlannerInput,
    ) -> Result<OutcomePlannerOutput, OutcomePlannerError> {
        self.validate_input(&input)?;

        let schema = outcome_planner_output_schema();
        let bundle = self
            .prompt_builder
            .build_bundle(
                AgentLlmNode::OutcomePlanner,
                &input,
                PromptBuildOptions {
                    task_instructions: vec![
                        "Resolve the current turn once, including eligible reactions.".to_string(),
                        "Separate hard state updates, soft effects, and blocked effects."
                            .to_string(),
                        "Only narratable_facts may feed the later SurfaceRealizer.".to_string(),
                    ],
                    output_schema_json: Some(schema.clone()),
                    input_sections: outcome_planner_input_sections(),
                    ..Default::default()
                },
            )
            .map_err(OutcomePlannerError::PromptBuild)?;

        let request = self
            .prompt_builder
            .build_chat_request(
                &bundle,
                PromptRequestOptions::new(Uuid::new_v4().to_string(), self.api_config.id.clone()),
            )
            .map_err(OutcomePlannerError::PromptBuild)?;

        let provider =
            create_provider(&self.api_config, None).map_err(OutcomePlannerError::LlmError)?;
        let raw_output = provider
            .chat_structured(request, schema)
            .await
            .map_err(OutcomePlannerError::LlmError)?;

        let output: OutcomePlannerOutput = serde_json::from_value(raw_output).map_err(|e| {
            OutcomePlannerError::SchemaError(format!(
                "Failed to decode structured OutcomePlannerOutput: {e}"
            ))
        })?;

        self.validate_output(&output, &input)?;
        Ok(output)
    }

    fn validate_input(&self, input: &OutcomePlannerInput) -> Result<(), OutcomePlannerError> {
        if input.scene_turn_id.trim().is_empty() {
            return Err(OutcomePlannerError::InputValidation(
                "scene_turn_id must not be empty".to_string(),
            ));
        }
        if input.scene_model.scene_turn_id != input.scene_turn_id {
            return Err(OutcomePlannerError::InputValidation(
                "scene_model.scene_turn_id must match input.scene_turn_id".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_output(
        &self,
        output: &OutcomePlannerOutput,
        input: &OutcomePlannerInput,
    ) -> Result<(), OutcomePlannerError> {
        let mut fact_ids = HashSet::new();
        for fact in &output.outcome_plan.narratable_facts {
            if fact.fact_id.trim().is_empty() {
                return Err(OutcomePlannerError::OutputValidation(
                    "narratable_facts.fact_id must not be empty".to_string(),
                ));
            }
            if !fact_ids.insert(fact.fact_id.as_str()) {
                return Err(OutcomePlannerError::OutputValidation(format!(
                    "duplicate narratable fact id '{}'",
                    fact.fact_id
                )));
            }
        }

        let allowed_actor_ids: HashSet<&str> = input
            .character_outputs
            .iter()
            .map(|item| item.intent_plan.character_id.as_str())
            .chain(
                input
                    .user_roleplay_intents
                    .iter()
                    .map(|item| item.character_id.as_str()),
            )
            .chain(
                input
                    .minor_actor_slots
                    .iter()
                    .map(|item| item.character_id.as_str()),
            )
            .collect();
        for action in &output.outcome_plan.outward_actions {
            if !allowed_actor_ids.is_empty()
                && !allowed_actor_ids.contains(action.actor_id.as_str())
            {
                return Err(OutcomePlannerError::OutputValidation(format!(
                    "outward action actor_id '{}' is not present in planner input",
                    action.actor_id
                )));
            }
        }

        if let Some(scene_delta) = &output.state_update_plan.scene_delta {
            if scene_delta.scene_id != input.scene_model.scene_id {
                return Err(OutcomePlannerError::OutputValidation(
                    "state_update_plan.scene_delta.scene_id must target input.scene_model.scene_id"
                        .to_string(),
                ));
            }
        }

        Ok(())
    }
}

impl Default for OutcomePlanner {
    fn default() -> Self {
        Self::new(default_agent_api_config())
    }
}

fn outcome_planner_input_sections() -> Vec<PromptInputSection> {
    vec![
        PromptInputSection {
            pointer: "/scene_model".to_string(),
            label: "scene_model".to_string(),
            priority: PromptPriority::P0Required,
        },
        PromptInputSection {
            pointer: "/character_outputs".to_string(),
            label: "character_outputs".to_string(),
            priority: PromptPriority::P0Required,
        },
        PromptInputSection {
            pointer: "/user_roleplay_intents".to_string(),
            label: "user_roleplay_intents".to_string(),
            priority: PromptPriority::P1DecisionCritical,
        },
        PromptInputSection {
            pointer: "/reaction_windows".to_string(),
            label: "reaction_windows".to_string(),
            priority: PromptPriority::P1DecisionCritical,
        },
        PromptInputSection {
            pointer: "/reaction_intents".to_string(),
            label: "reaction_intents".to_string(),
            priority: PromptPriority::P1DecisionCritical,
        },
        PromptInputSection {
            pointer: "/skills".to_string(),
            label: "skills".to_string(),
            priority: PromptPriority::P2Contextual,
        },
        PromptInputSection {
            pointer: "/relevant_knowledge".to_string(),
            label: "relevant_knowledge".to_string(),
            priority: PromptPriority::P2Contextual,
        },
        PromptInputSection {
            pointer: "/truth_guidance".to_string(),
            label: "truth_guidance".to_string(),
            priority: PromptPriority::P2Contextual,
        },
        PromptInputSection {
            pointer: "/minor_actor_slots".to_string(),
            label: "minor_actor_slots".to_string(),
            priority: PromptPriority::P3OptionalFlavor,
        },
    ]
}

fn outcome_planner_output_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "outcome_plan",
            "state_update_plan",
            "knowledge_reveal_events",
            "conflict_reports"
        ],
        "properties": {
            "outcome_plan": { "type": "object" },
            "state_update_plan": { "type": "object" },
            "knowledge_reveal_events": { "type": "array", "items": { "type": "object" } },
            "conflict_reports": { "type": "array", "items": { "type": "object" } }
        }
    })
}

#[derive(Debug, Clone)]
pub enum OutcomePlannerError {
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
        AgentSessionContext, DayPhase, OutcomePlan, PrimaryEmotion, SoftEffect, TimeAnchor,
        TimeContext, TimePrecision,
    };

    #[test]
    fn rejects_outward_action_from_unknown_actor() {
        let planner = OutcomePlanner::default();
        let input = sample_input();
        let mut output = sample_output(&input);
        output.outcome_plan.outward_actions[0].actor_id = "unknown".to_string();

        let err = planner
            .validate_output(&output, &input)
            .expect_err("unknown actor should fail");
        assert!(
            matches!(err, OutcomePlannerError::OutputValidation(message) if message.contains("actor_id"))
        );
    }

    fn sample_input() -> OutcomePlannerInput {
        let anchor = TimeAnchor {
            calendar_id: "default".to_string(),
            ordinal: 1,
            precision: TimePrecision::Exact,
            display_text: "night".to_string(),
        };
        OutcomePlannerInput {
            scene_turn_id: "turn_1".to_string(),
            session_context: AgentSessionContext {
                session_id: "session_1".to_string(),
                session_kind: "mainline".to_string(),
                period_anchor: anchor.clone(),
                mainline_time_anchor: anchor.clone(),
                player_character_id: Some("char_1".to_string()),
                canon_status: "canon_candidate".to_string(),
            },
            truth_guidance: None,
            scene_model: crate::agent::models::SceneModel {
                scene_id: "scene_1".to_string(),
                scene_turn_id: "turn_1".to_string(),
                time_context: TimeContext {
                    time_anchor: anchor,
                    season: "winter".to_string(),
                    day_phase: DayPhase::Night,
                    weather_trend: "snow".to_string(),
                },
                spatial_layout: crate::agent::models::SpatialLayout {
                    layout_type: "hall".to_string(),
                    dimensions: None,
                    obstacles: vec![],
                    entrances: vec![],
                    zones: vec![],
                },
                lighting: crate::agent::models::LightingState {
                    ambient_level: 0.5,
                    light_sources: vec![],
                    shadow_areas: vec![],
                    backlight: None,
                },
                acoustics: crate::agent::models::AcousticsState {
                    ambient_noise_level: 0.0,
                    echo_characteristics: "stone".to_string(),
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
                scene_mood: crate::agent::models::SceneMood::Tense,
                physical_conditions: crate::agent::models::PhysicalConditions {
                    temperature: crate::agent::models::Temperature {
                        ambient_celsius: 5.0,
                        felt_celsius: 2.0,
                        modifiers: vec![],
                    },
                    surface_state: crate::agent::models::SurfaceState {
                        slipperiness: 0.1,
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
            character_records: vec![],
            relevant_knowledge: vec![],
            skills: vec![],
            character_outputs: vec![crate::agent::models::CharacterCognitivePassOutput {
                perception_delta: crate::agent::models::PerceptionDelta {
                    new_observations: vec![],
                    updated_perceptions: vec![],
                    missed_observations: vec![],
                },
                belief_update: crate::agent::models::BeliefUpdate {
                    stable_beliefs_reinforced: vec![],
                    stable_beliefs_weakened: vec![],
                    new_hypotheses: vec![],
                    revised_models_of_others: vec![],
                    contradictions_and_tension: vec![],
                    emotional_shift: crate::agent::models::EmotionalShiftDelta {
                        primary_emotion: PrimaryEmotion::Neutral,
                        intensity_change: 0.0,
                        secondary_changes: vec![],
                    },
                    decision_relevant_beliefs: vec![],
                },
                intent_plan: crate::agent::models::IntentPlan {
                    character_id: "char_1".to_string(),
                    intent_kind: "advance".to_string(),
                    target_refs: vec![],
                    intended_actions: vec![],
                    priority: "high".to_string(),
                    commitment: "firm".to_string(),
                    rationale: "close distance".to_string(),
                },
                body_reaction_delta: None,
            }],
            user_roleplay_intents: vec![],
            minor_actor_slots: vec![],
            reaction_windows: vec![],
            reaction_intents: vec![],
            director_hint: None,
            provisional_truth_candidates: vec![],
        }
    }

    fn sample_output(input: &OutcomePlannerInput) -> OutcomePlannerOutput {
        OutcomePlannerOutput {
            outcome_plan: OutcomePlan {
                outward_actions: vec![crate::agent::models::OutwardAction {
                    action_id: "action_1".to_string(),
                    actor_id: input.character_outputs[0].intent_plan.character_id.clone(),
                    action_kind: "advance".to_string(),
                    target_refs: vec![],
                    narratable_fact_refs: vec!["fact_1".to_string()],
                    status: "executed".to_string(),
                }],
                resulting_state_changes: serde_json::json!({}),
                narratable_facts: vec![crate::agent::models::NarratableFact {
                    fact_id: "fact_1".to_string(),
                    fact_kind: "movement".to_string(),
                    subject_refs: vec!["char_1".to_string()],
                    source_refs: vec!["action_1".to_string()],
                    allowed_claim: "Ran moved forward.".to_string(),
                    narration_scope: crate::agent::models::NarrationScope::ObjectiveCamera,
                }],
                soft_effects: vec![SoftEffect {
                    source_id: "action_1".to_string(),
                    target_id: None,
                    effect_kind: "pressure".to_string(),
                    description: Utc::now().to_rfc3339(),
                }],
                blocked_effects: vec![],
            },
            state_update_plan: crate::agent::models::StateUpdatePlan {
                scene_delta: Some(crate::agent::models::SceneDelta {
                    scene_id: input.scene_model.scene_id.clone(),
                    entity_deltas: vec![],
                    physical_delta: None,
                    mana_field_delta: None,
                    observable_signal_deltas: vec![],
                    private_state_deltas: vec![],
                    event_appends: vec![],
                }),
                character_state_deltas: vec![],
                subjective_update_refs: vec![],
                new_memory_entries: vec![],
                soft_effects: vec![],
                blocked_effects: vec![],
                validation_warnings: vec![],
                consistency_notes: vec![],
            },
            knowledge_reveal_events: vec![],
            conflict_reports: vec![],
        }
    }
}
