//! Character cognitive pass
//!
//! Structured Agent LLM execution for single-character subjective reasoning.

use uuid::Uuid;

use crate::agent::llm_support::default_agent_api_config;
use crate::agent::models::{CharacterCognitivePassInput, CharacterCognitivePassOutput};
use crate::agent::prompting::{
    AgentLlmNode, PromptBuildOptions, PromptBuilder, PromptInputSection, PromptPriority,
    PromptRequestOptions,
};
use crate::agent::validation::validator::ValidationSeverity;
use crate::agent::validation::Validator;
use crate::commands::chat_commands::create_provider;
use crate::storage::st_resources::ApiConfig;

/// Character cognitive pass - LLM-based cognitive processing.
pub struct CharacterCognitivePass {
    api_config: ApiConfig,
    prompt_builder: PromptBuilder,
}

impl CharacterCognitivePass {
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

    /// Execute cognitive pass.
    pub async fn execute(
        &self,
        input: CharacterCognitivePassInput,
    ) -> Result<CharacterCognitivePassOutput, CognitivePassError> {
        self.validate_input(&input)?;

        let schema = cognitive_output_schema();
        let bundle = self
            .prompt_builder
            .build_bundle(
                AgentLlmNode::CharacterCognitivePass,
                &input,
                PromptBuildOptions {
                    task_instructions: vec![
                        "Stay inside the character's subjective perspective.".to_string(),
                        "Prefer conservative uncertainty over invented certainty.".to_string(),
                        "Return one actionable intent plan for this turn.".to_string(),
                    ],
                    output_schema_json: Some(schema.clone()),
                    input_sections: cognitive_input_sections(),
                    ..Default::default()
                },
            )
            .map_err(CognitivePassError::PromptBuild)?;

        let request = self
            .prompt_builder
            .build_chat_request(
                &bundle,
                PromptRequestOptions::new(Uuid::new_v4().to_string(), self.api_config.id.clone()),
            )
            .map_err(CognitivePassError::PromptBuild)?;

        let provider =
            create_provider(&self.api_config, None).map_err(CognitivePassError::LlmError)?;
        let raw_output = provider
            .chat_structured(request, schema)
            .await
            .map_err(CognitivePassError::LlmError)?;

        let output: CharacterCognitivePassOutput =
            serde_json::from_value(raw_output).map_err(|e| {
                CognitivePassError::SchemaError(format!("Failed to decode structured output: {e}"))
            })?;

        self.validate_output(&output, &input)?;
        Ok(output)
    }

    fn validate_input(
        &self,
        input: &CharacterCognitivePassInput,
    ) -> Result<(), CognitivePassError> {
        if input.character_id.trim().is_empty() {
            return Err(CognitivePassError::InputValidation(
                "character_id must not be empty".to_string(),
            ));
        }

        if input.scene_turn_id.trim().is_empty() {
            return Err(CognitivePassError::InputValidation(
                "scene_turn_id must not be empty".to_string(),
            ));
        }

        if input.filtered_scene_view.character_id != input.character_id {
            return Err(CognitivePassError::InputValidation(
                "filtered_scene_view.character_id does not match character_id".to_string(),
            ));
        }

        if input.embodiment_state.character_id != input.character_id {
            return Err(CognitivePassError::InputValidation(
                "embodiment_state.character_id does not match character_id".to_string(),
            ));
        }

        if input.accessible_knowledge.character_id != input.character_id {
            return Err(CognitivePassError::InputValidation(
                "accessible_knowledge.character_id does not match character_id".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_output(
        &self,
        output: &CharacterCognitivePassOutput,
        input: &CharacterCognitivePassInput,
    ) -> Result<(), CognitivePassError> {
        let issues = Validator::validate_cognitive(output, input);

        let hard_issues: Vec<String> = issues
            .into_iter()
            .filter(|issue| {
                matches!(
                    issue.severity,
                    ValidationSeverity::Error | ValidationSeverity::Critical
                )
            })
            .map(|issue| match issue.field_path {
                Some(path) => format!("{} at {}: {}", issue.rule, path, issue.description),
                None => format!("{}: {}", issue.rule, issue.description),
            })
            .collect();

        if hard_issues.is_empty() {
            Ok(())
        } else {
            Err(CognitivePassError::OutputValidation(hard_issues.join("; ")))
        }
    }
}

impl Default for CharacterCognitivePass {
    fn default() -> Self {
        Self::new(default_agent_api_config())
    }
}

fn cognitive_input_sections() -> Vec<PromptInputSection> {
    vec![
        PromptInputSection {
            pointer: "/filtered_scene_view/spatial_context".to_string(),
            label: "spatial_context".to_string(),
            priority: PromptPriority::P2Contextual,
        },
        PromptInputSection {
            pointer: "/accessible_knowledge/entries".to_string(),
            label: "accessible_knowledge_entries".to_string(),
            priority: PromptPriority::P1DecisionCritical,
        },
        PromptInputSection {
            pointer: "/accessible_knowledge/summary".to_string(),
            label: "accessible_knowledge_summary".to_string(),
            priority: PromptPriority::P2Contextual,
        },
        PromptInputSection {
            pointer: "/prior_subjective_state/relation_models".to_string(),
            label: "prior_relation_models".to_string(),
            priority: PromptPriority::P2Contextual,
        },
        PromptInputSection {
            pointer: "/prior_subjective_state/current_goals/hidden".to_string(),
            label: "hidden_goals".to_string(),
            priority: PromptPriority::P3OptionalFlavor,
        },
    ]
}

fn cognitive_output_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["perception_delta", "belief_update", "intent_plan", "body_reaction_delta"],
        "properties": {
            "perception_delta": {
                "type": "object",
                "additionalProperties": false,
                "required": ["new_observations", "updated_perceptions", "missed_observations"],
                "properties": {
                    "new_observations": {"type": "array", "items": {"type": "string"}},
                    "updated_perceptions": {"type": "array", "items": {"type": "string"}},
                    "missed_observations": {"type": "array", "items": {"type": "string"}}
                }
            },
            "belief_update": {
                "type": "object",
                "additionalProperties": false,
                "required": [
                    "stable_beliefs_reinforced",
                    "stable_beliefs_weakened",
                    "new_hypotheses",
                    "revised_models_of_others",
                    "contradictions_and_tension",
                    "emotional_shift",
                    "decision_relevant_beliefs"
                ],
                "properties": {
                    "stable_beliefs_reinforced": {"type": "array", "items": belief_shift_entry_schema()},
                    "stable_beliefs_weakened": {"type": "array", "items": belief_shift_entry_schema()},
                    "new_hypotheses": {"type": "array", "items": new_hypothesis_schema()},
                    "revised_models_of_others": {"type": "array", "items": revised_relation_model_schema()},
                    "contradictions_and_tension": {"type": "array", "items": contradiction_resolution_schema()},
                    "emotional_shift": emotional_shift_schema(),
                    "decision_relevant_beliefs": {"type": "array", "items": {"type": "string"}}
                }
            },
            "intent_plan": intent_plan_schema(),
            "body_reaction_delta": {
                "anyOf": [
                    {"type": "null"},
                    body_reaction_delta_schema()
                ]
            }
        }
    })
}

fn belief_shift_entry_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["proposition", "confidence_shift"],
        "properties": {
            "proposition": {"type": "string"},
            "confidence_shift": confidence_shift_schema()
        }
    })
}

fn new_hypothesis_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["proposition", "status", "evidence_refs"],
        "properties": {
            "proposition": {"type": "string"},
            "status": {"type": "string"},
            "evidence_refs": {"type": "array", "items": {"type": "string"}}
        }
    })
}

fn revised_relation_model_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["target_character_id", "trust_shift", "intent_assessment_change", "new_impressions"],
        "properties": {
            "target_character_id": {"type": "string"},
            "trust_shift": confidence_shift_schema(),
            "intent_assessment_change": {
                "anyOf": [
                    {"type": "null"},
                    {"type": "string"}
                ]
            },
            "new_impressions": {"type": "array", "items": {"type": "string"}}
        }
    })
}

fn contradiction_resolution_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["contradiction_id", "conflicting_beliefs", "resolution_strategy", "resolution_notes"],
        "properties": {
            "contradiction_id": {"type": "string"},
            "conflicting_beliefs": {"type": "array", "items": {"type": "string"}},
            "resolution_strategy": {"type": "string"},
            "resolution_notes": {"type": "string"}
        }
    })
}

fn emotional_shift_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["primary_emotion", "intensity_change", "secondary_changes"],
        "properties": {
            "primary_emotion": primary_emotion_schema(),
            "intensity_change": {"type": "number"},
            "secondary_changes": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["emotion", "intensity", "trigger"],
                    "properties": {
                        "emotion": primary_emotion_schema(),
                        "intensity": {"type": "number"},
                        "trigger": {
                            "anyOf": [
                                {"type": "null"},
                                {"type": "string"}
                            ]
                        }
                    }
                }
            }
        }
    })
}

fn intent_plan_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "character_id",
            "intent_kind",
            "target_refs",
            "intended_actions",
            "priority",
            "commitment",
            "rationale"
        ],
        "properties": {
            "character_id": {"type": "string"},
            "intent_kind": {"type": "string"},
            "target_refs": {"type": "array", "items": {"type": "string"}},
            "intended_actions": {"type": "array", "items": character_action_schema()},
            "priority": {"type": "string"},
            "commitment": {"type": "string"},
            "rationale": {"type": "string"}
        }
    })
}

fn character_action_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "action_id",
            "action_kind",
            "target_refs",
            "spoken_text",
            "skill_id",
            "requested_mana_expression",
            "declared_effect_refs",
            "outward_description"
        ],
        "properties": {
            "action_id": {"type": "string"},
            "action_kind": {"type": "string"},
            "target_refs": {"type": "array", "items": {"type": "string"}},
            "spoken_text": {
                "anyOf": [
                    {"type": "null"},
                    {"type": "string"}
                ]
            },
            "skill_id": {
                "anyOf": [
                    {"type": "null"},
                    {"type": "string"}
                ]
            },
            "requested_mana_expression": {
                "anyOf": [
                    {"type": "null"},
                    mana_expression_mode_schema()
                ]
            },
            "declared_effect_refs": {"type": "array", "items": {"type": "string"}},
            "outward_description": {"type": "string"}
        }
    })
}

fn body_reaction_delta_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["character_id", "reaction_kind", "intensity", "outward_signal", "possible_state_effect"],
        "properties": {
            "character_id": {"type": "string"},
            "reaction_kind": {"type": "string"},
            "intensity": {"type": "string"},
            "outward_signal": {"type": "string"},
            "possible_state_effect": {
                "anyOf": [
                    {"type": "null"},
                    {"type": "string"}
                ]
            }
        }
    })
}

fn confidence_shift_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "string",
        "enum": [
            "StrongDecrease",
            "Decrease",
            "Unchanged",
            "Increase",
            "StrongIncrease",
            "Flip"
        ]
    })
}

fn primary_emotion_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "string",
        "enum": [
            "Neutral",
            "Joy",
            "Sadness",
            "Anger",
            "Fear",
            "Disgust",
            "Surprise",
            "Anticipation",
            "Trust",
            "Contempt"
        ]
    })
}

fn mana_expression_mode_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "string",
        "enum": [
            "Sealed",
            "Suppressed",
            "Natural",
            "Released",
            "Dominating"
        ]
    })
}

/// Cognitive pass error.
#[derive(Debug, Clone)]
pub enum CognitivePassError {
    PromptBuild(String),
    InputValidation(String),
    OutputValidation(String),
    LlmError(String),
    SchemaError(String),
}
