//! Surface realizer
//!
//! Structured Agent LLM execution for final narrative rendering.

use std::collections::HashSet;

use uuid::Uuid;

use crate::agent::llm_support::default_agent_api_config;
use crate::agent::models::{SurfaceRealizerInput, SurfaceRealizerOutput};
use crate::agent::prompting::{
    AgentLlmNode, PromptBuildOptions, PromptBuilder, PromptInputSection, PromptPriority,
    PromptRequestOptions,
};
use crate::commands::chat_commands::create_provider;
use crate::storage::st_resources::ApiConfig;

pub struct SurfaceRealizer {
    api_config: ApiConfig,
    prompt_builder: PromptBuilder,
}

impl SurfaceRealizer {
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
        input: SurfaceRealizerInput,
    ) -> Result<SurfaceRealizerOutput, SurfaceRealizerError> {
        self.validate_input(&input)?;

        let schema = surface_realizer_output_schema();
        let bundle = self
            .prompt_builder
            .build_bundle(
                AgentLlmNode::SurfaceRealizer,
                &input,
                PromptBuildOptions {
                    task_instructions: vec![
                        "Return final user-facing narrative text plus fact ids only.".to_string(),
                        "Ground every concrete claim in outcome_plan.narratable_facts.".to_string(),
                        "Narrate blocked effects only as failed or prevented attempts.".to_string(),
                    ],
                    output_schema_json: Some(schema.clone()),
                    input_sections: surface_realizer_input_sections(),
                    ..Default::default()
                },
            )
            .map_err(SurfaceRealizerError::PromptBuild)?;

        let request = self
            .prompt_builder
            .build_chat_request(
                &bundle,
                PromptRequestOptions::new(Uuid::new_v4().to_string(), self.api_config.id.clone()),
            )
            .map_err(SurfaceRealizerError::PromptBuild)?;

        let provider =
            create_provider(&self.api_config, None).map_err(SurfaceRealizerError::LlmError)?;
        let raw_output = provider
            .chat_structured(request, schema)
            .await
            .map_err(SurfaceRealizerError::LlmError)?;

        let output: SurfaceRealizerOutput = serde_json::from_value(raw_output).map_err(|e| {
            SurfaceRealizerError::SchemaError(format!(
                "Failed to decode structured SurfaceRealizerOutput: {e}"
            ))
        })?;

        self.validate_output(&output, &input)?;
        Ok(output)
    }

    fn validate_input(&self, input: &SurfaceRealizerInput) -> Result<(), SurfaceRealizerError> {
        if input.scene_turn_id.trim().is_empty() {
            return Err(SurfaceRealizerError::InputValidation(
                "scene_turn_id must not be empty".to_string(),
            ));
        }
        if input.scene_view.scene_turn_id != input.scene_turn_id {
            return Err(SurfaceRealizerError::InputValidation(
                "scene_view.scene_turn_id must match input.scene_turn_id".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_output(
        &self,
        output: &SurfaceRealizerOutput,
        input: &SurfaceRealizerInput,
    ) -> Result<(), SurfaceRealizerError> {
        if output.narrative_text.trim().is_empty() {
            return Err(SurfaceRealizerError::OutputValidation(
                "narrative_text must not be empty".to_string(),
            ));
        }

        let allowed_fact_ids: HashSet<&str> = input
            .outcome_plan
            .narratable_facts
            .iter()
            .map(|fact| fact.fact_id.as_str())
            .collect();
        for fact_id in &output.used_fact_ids {
            if !allowed_fact_ids.contains(fact_id.as_str()) {
                return Err(SurfaceRealizerError::OutputValidation(format!(
                    "used_fact_ids contains '{}' outside outcome_plan.narratable_facts",
                    fact_id
                )));
            }
        }

        Ok(())
    }
}

impl Default for SurfaceRealizer {
    fn default() -> Self {
        Self::new(default_agent_api_config())
    }
}

fn surface_realizer_input_sections() -> Vec<PromptInputSection> {
    vec![
        PromptInputSection {
            pointer: "/narration_scope".to_string(),
            label: "narration_scope".to_string(),
            priority: PromptPriority::P0Required,
        },
        PromptInputSection {
            pointer: "/scene_view".to_string(),
            label: "scene_view".to_string(),
            priority: PromptPriority::P1DecisionCritical,
        },
        PromptInputSection {
            pointer: "/character_views".to_string(),
            label: "character_views".to_string(),
            priority: PromptPriority::P2Contextual,
        },
        PromptInputSection {
            pointer: "/outcome_plan".to_string(),
            label: "outcome_plan".to_string(),
            priority: PromptPriority::P0Required,
        },
        PromptInputSection {
            pointer: "/style".to_string(),
            label: "style".to_string(),
            priority: PromptPriority::P3OptionalFlavor,
        },
    ]
}

fn surface_realizer_output_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["narrative_text", "used_fact_ids"],
        "properties": {
            "narrative_text": { "type": "string" },
            "used_fact_ids": { "type": "array", "items": { "type": "string" } }
        }
    })
}

#[derive(Debug, Clone)]
pub enum SurfaceRealizerError {
    PromptBuild(String),
    InputValidation(String),
    OutputValidation(String),
    LlmError(String),
    SchemaError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::models::{
        Atmosphere, DetailLevel, NarrationScope, NarrativeCharacterView, NarrativeEntityView,
        NarrativeEventView, Pacing, PointOfView, SceneNarrativeView, StyleConstraints,
        StyleRegister,
    };

    #[test]
    fn rejects_fact_ids_outside_narratable_scope() {
        let realizer = SurfaceRealizer::default();
        let input = sample_input();
        let output = SurfaceRealizerOutput {
            narrative_text: "A step forward.".to_string(),
            used_fact_ids: vec!["unknown".to_string()],
        };

        let err = realizer
            .validate_output(&output, &input)
            .expect_err("unknown fact ids should fail");
        assert!(
            matches!(err, SurfaceRealizerError::OutputValidation(message) if message.contains("used_fact_ids"))
        );
    }

    fn sample_input() -> SurfaceRealizerInput {
        SurfaceRealizerInput {
            scene_turn_id: "turn_1".to_string(),
            narration_scope: NarrationScope::ObjectiveCamera,
            scene_view: SceneNarrativeView {
                scene_id: "scene_1".to_string(),
                scene_turn_id: "turn_1".to_string(),
                narration_scope: NarrationScope::ObjectiveCamera,
                visible_entities: vec![NarrativeEntityView {
                    entity_id: "char_1".to_string(),
                    display_name: "Ran".to_string(),
                    observable_facts: vec!["standing".to_string()],
                    outward_state: vec!["steady".to_string()],
                }],
                visible_environment: serde_json::json!({}),
                visible_events: vec![NarrativeEventView {
                    event_id: "event_1".to_string(),
                    event_kind: "movement".to_string(),
                    narratable_fact_refs: vec!["fact_1".to_string()],
                }],
                allowed_private_refs: vec![],
            },
            character_views: vec![NarrativeCharacterView {
                character_id: "char_1".to_string(),
                display_name: "Ran".to_string(),
                outward_actions: vec!["steps forward".to_string()],
                outward_reactions: vec![],
                allowed_inner_summary: None,
            }],
            outcome_plan: crate::agent::models::OutcomePlan {
                outward_actions: vec![],
                resulting_state_changes: serde_json::json!({}),
                narratable_facts: vec![crate::agent::models::NarratableFact {
                    fact_id: "fact_1".to_string(),
                    fact_kind: "movement".to_string(),
                    subject_refs: vec!["char_1".to_string()],
                    source_refs: vec!["event_1".to_string()],
                    allowed_claim: "Ran stepped forward.".to_string(),
                    narration_scope: NarrationScope::ObjectiveCamera,
                }],
                soft_effects: vec![],
                blocked_effects: vec![],
            },
            style: StyleConstraints {
                register: StyleRegister::Formal,
                detail_level: DetailLevel::Moderate,
                atmosphere: Atmosphere::Tense,
                pacing: Pacing::Measured,
                pov: PointOfView::Objective,
                explicit_guidelines: vec![],
                reference_excerpts: vec![],
            },
        }
    }
}
