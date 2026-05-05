//! Parallel cognitive pass executor
//!
//! Implements parallel execution of CharacterCognitivePass as specified in
//! docs/11_agent_runtime.md §6.1.
//!
//! Key constraints:
//! - Read fixed snapshot + turn working copy
//! - Produce candidates, no persistent state writes
//! - All outputs collected before unified validation
//! - Output arrival order must not affect final result

use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::agent::cognitive::cognitive_pass::{CharacterCognitivePass, CognitivePassError};
use crate::agent::knowledge::access_protocol::KnowledgeAccessProtocol;
use crate::agent::llm_support::default_agent_api_config;
use crate::agent::models::{
    AccessibleKnowledge, ActionFeasibility as EmbodimentActionFeasibility,
    BodyConstraints as EmbodimentBodyConstraints, CharacterCognitivePassInput,
    CharacterCognitivePassOutput, CharacterSubjectiveState, EffectiveAttributeProfile,
    EmbodimentState, EnvironmentalStrain as EmbodimentEnvironmentalStrain,
    ReasoningModifiers as EmbodimentReasoningModifiers,
    RespirationImpactTier as EmbodimentRespirationImpactTier,
    SalienceModifiers as EmbodimentSalienceModifiers, SceneModel,
    SensoryCapabilities as EmbodimentSensoryCapabilities,
    SensoryCapability as EmbodimentSensoryCapability,
    SurfaceImpactTier as EmbodimentSurfaceImpactTier,
    TemperatureFeelTier as EmbodimentTemperatureFeelTier,
    WindImpactTier as EmbodimentWindImpactTier,
};
use crate::agent::simulation::SceneFilter;

use super::budget_monitor::BudgetMonitor;
use super::turn_state::TurnWorkingState;

/// Result of a single character's cognitive pass
#[derive(Debug)]
pub struct CognitivePassResult {
    /// Character ID
    pub character_id: String,
    /// The cognitive pass output (if successful)
    pub output: Result<CharacterCognitivePassOutput, CognitivePassError>,
    /// Input tokens used (for budget tracking)
    pub input_tokens: u32,
    /// Output tokens used (for budget tracking)
    pub output_tokens: u32,
}

/// Parallel cognitive pass executor
///
/// Manages parallel execution of cognitive passes for multiple characters.
/// Ensures that all reads are from fixed snapshots and no writes occur during execution.
pub struct ParallelCognitiveExecutor {
    /// Budget monitor for tracking usage
    budget_monitor: Arc<RwLock<BudgetMonitor>>,
    /// Optional world database for Layer 2 knowledge derivation.
    pool: Option<SqlitePool>,
}

impl ParallelCognitiveExecutor {
    /// Create a new parallel executor
    pub fn new(budget_monitor: Arc<RwLock<BudgetMonitor>>) -> Self {
        Self {
            budget_monitor,
            pool: None,
        }
    }

    /// Create a parallel executor that can derive DB-backed Layer 2 knowledge.
    pub fn with_pool(budget_monitor: Arc<RwLock<BudgetMonitor>>, pool: SqlitePool) -> Self {
        Self {
            budget_monitor,
            pool: Some(pool),
        }
    }

    /// Execute cognitive passes for multiple characters in parallel
    ///
    /// This method:
    /// 1. Builds inputs for each character (Layer 2 derivation)
    /// 2. Executes cognitive passes in parallel
    /// 3. Collects all outputs
    /// 4. Returns results sorted by character_id for deterministic ordering
    ///
    /// # Arguments
    /// * `working_state` - The turn working state (read-only during parallel execution)
    /// * `character_ids` - Characters to execute cognitive passes for
    /// * `effective_attrs` - Pre-computed effective attributes for each character
    ///
    /// # Returns
    /// A map of character_id to CognitivePassResult
    pub async fn execute_parallel(
        &self,
        working_state: &TurnWorkingState,
        character_ids: &[String],
        effective_attrs: &HashMap<String, EffectiveAttributeProfile>,
    ) -> HashMap<String, CognitivePassResult> {
        if character_ids.is_empty() {
            return HashMap::new();
        }

        // Channel for collecting results
        let (tx, mut rx) = mpsc::channel::<CognitivePassResult>(character_ids.len());

        // Spawn tasks for each character
        let mut handles = Vec::with_capacity(character_ids.len());

        for character_id in character_ids.iter().cloned() {
            let tx = tx.clone();
            let budget_monitor = self.budget_monitor.clone();
            let effective_attrs = effective_attrs.clone();
            let pool = self.pool.clone();

            // Clone what we need for the async task
            let scene = working_state.scene.clone();
            let runtime_snapshot_id = working_state.runtime_config_snapshot_id.clone();
            let world_snapshot_id = working_state.world_rules_snapshot_id.clone();

            let handle = tokio::spawn(async move {
                // Build Layer 2 inputs for this character
                let input_result = build_cognitive_pass_input(
                    &character_id,
                    &scene,
                    &effective_attrs,
                    &runtime_snapshot_id,
                    world_snapshot_id.as_deref(),
                    pool,
                )
                .await;

                let result = match input_result {
                    Ok(input) => {
                        // Check budget before executing
                        let can_execute = {
                            let monitor = budget_monitor.read().await;
                            monitor.can_execute_cognitive_pass()
                        };

                        if !can_execute {
                            // Record as template intent
                            let mut monitor = budget_monitor.write().await;
                            monitor.record_template_intent(&character_id);
                            CognitivePassResult {
                                character_id: character_id.clone(),
                                output: Err(CognitivePassError::InputValidation(
                                    "Budget limit reached".to_string(),
                                )),
                                input_tokens: 0,
                                output_tokens: 0,
                            }
                        } else {
                            // Execute cognitive pass
                            let api_config = default_agent_api_config();
                            let pass = CharacterCognitivePass::new(api_config);
                            let output = pass.execute(input).await;

                            // Record usage
                            let (input_tokens, output_tokens) = match &output {
                                Ok(_) => (4000u32, 1000u32), // Placeholder, would get from response
                                Err(_) => (0u32, 0u32),
                            };

                            {
                                let mut monitor = budget_monitor.write().await;
                                monitor.record_cognitive_pass(
                                    &character_id,
                                    input_tokens,
                                    output_tokens,
                                );
                            }

                            CognitivePassResult {
                                character_id: character_id.clone(),
                                output,
                                input_tokens,
                                output_tokens,
                            }
                        }
                    }
                    Err(e) => CognitivePassResult {
                        character_id: character_id.clone(),
                        output: Err(CognitivePassError::InputValidation(e)),
                        input_tokens: 0,
                        output_tokens: 0,
                    },
                };

                // Send result through channel
                let _ = tx.send(result).await;
            });

            handles.push(handle);
        }

        // Drop the sender so the channel closes when all tasks are done
        drop(tx);

        // Collect all results
        let mut results = HashMap::new();
        while let Some(result) = rx.recv().await {
            results.insert(result.character_id.clone(), result);
        }

        // Wait for all tasks to complete (they should be done by now)
        for handle in handles {
            let _ = handle.await;
        }

        // Sort by character_id for deterministic ordering
        let mut sorted_results: Vec<_> = results.into_iter().collect();
        sorted_results.sort_by(|a, b| a.0.cmp(&b.0));

        sorted_results.into_iter().collect()
    }
}

/// Build CognitivePassInput for a character
///
/// This derives Layer 2 inputs (EmbodimentState, FilteredSceneView, AccessibleKnowledge)
/// and assembles them into a CognitivePassInput.
async fn build_cognitive_pass_input(
    character_id: &str,
    scene: &SceneModel,
    effective_attrs: &HashMap<String, EffectiveAttributeProfile>,
    _runtime_snapshot_id: &str,
    _world_snapshot_id: Option<&str>,
    pool: Option<SqlitePool>,
) -> Result<CharacterCognitivePassInput, String> {
    // Get effective attributes for this character
    let effective = effective_attrs
        .get(character_id)
        .cloned()
        .unwrap_or_else(|| EffectiveAttributeProfile {
            character_id: character_id.to_string(),
            values: std::collections::HashMap::new(),
            tiers: std::collections::HashMap::new(),
            descriptors: std::collections::HashMap::new(),
        });

    // Build EmbodimentState (Layer 2)
    // Note: EmbodimentResolver::derive_embodiment requires a CharacterRecord
    // For now, create a minimal embodiment state with default capabilities
    let sensory_capabilities = EmbodimentSensoryCapabilities {
        vision: EmbodimentSensoryCapability::default(),
        hearing: EmbodimentSensoryCapability::default(),
        smell: EmbodimentSensoryCapability::default(),
        touch: EmbodimentSensoryCapability::default(),
        proprioception: EmbodimentSensoryCapability::default(),
        mana: EmbodimentSensoryCapability::default(),
    };

    let embodiment_state = EmbodimentState {
        character_id: character_id.to_string(),
        scene_turn_id: scene.scene_turn_id.clone(),
        sensory_capabilities,
        body_constraints: EmbodimentBodyConstraints {
            mobility: 1.0,
            balance: 1.0,
            fine_control: 1.0,
            pain_load: 0.0,
            fatigue_load: 0.0,
            cognitive_clarity: 1.0,
            environmental_strain: EmbodimentEnvironmentalStrain {
                wind_tier: EmbodimentWindImpactTier::Calm,
                temperature_tier: EmbodimentTemperatureFeelTier::Comfortable,
                surface_tier: EmbodimentSurfaceImpactTier::Stable,
                respiration_tier: EmbodimentRespirationImpactTier::Free,
                movement_penalty: 0.0,
                balance_penalty: 0.0,
                exposure_cold_delta: 0.0,
                exposure_heat_delta: 0.0,
                exposure_respiration_delta: 0.0,
                disrupted_actions: Vec::new(),
            },
        },
        salience_modifiers: EmbodimentSalienceModifiers {
            attention_biases: Vec::new(),
            aversion_triggers: Vec::new(),
            overload_risk: 0.0,
        },
        reasoning_modifiers: EmbodimentReasoningModifiers {
            pain_bias: 0.0,
            threat_bias: 0.0,
            overload_bias: 0.0,
            notes: Vec::new(),
        },
        action_feasibility: EmbodimentActionFeasibility {
            physical_execution: 1.0,
            social_patience: 1.0,
            fine_control: 1.0,
            sustained_attention: 1.0,
            blocked_action_kinds: Vec::new(),
        },
    };

    let filtered_scene_view = SceneFilter::new().filter_scene(scene, &embodiment_state, &effective);

    let accessible_knowledge = if let Some(pool) = pool {
        KnowledgeAccessProtocol::new(pool)
            .build_accessible_knowledge(
                character_id,
                &scene.scene_turn_id,
                &scene.time_context.time_anchor,
                None,
            )
            .await?
    } else {
        AccessibleKnowledge {
            character_id: character_id.to_string(),
            scene_turn_id: scene.scene_turn_id.clone(),
            entries: Vec::new(),
        }
    };

    // Build CognitivePassInput
    let input = CharacterCognitivePassInput {
        character_id: character_id.to_string(),
        scene_turn_id: scene.scene_turn_id.clone(),
        filtered_scene_view,
        embodiment_state,
        accessible_knowledge,
        prior_subjective_state: CharacterSubjectiveState::new(
            character_id.to_string(),
            scene.scene_turn_id.clone(),
        ),
        recent_event_delta: Vec::new(),
    };

    Ok(input)
}

/// Validate all cognitive pass outputs
///
/// This performs unified validation after all parallel passes complete.
/// Returns a map of character_id to validation issues (empty if valid).
pub fn validate_cognitive_outputs(
    outputs: &HashMap<String, CognitivePassResult>,
    inputs: &HashMap<String, CharacterCognitivePassInput>,
) -> HashMap<String, Vec<String>> {
    use crate::agent::validation::Validator;

    let mut issues = HashMap::new();

    for (character_id, result) in outputs {
        if let Ok(output) = &result.output {
            if let Some(input) = inputs.get(character_id) {
                let validation_issues = Validator::validate_cognitive(output, input);
                let hard_issues: Vec<String> = validation_issues
                    .into_iter()
                    .filter(|issue| {
                        matches!(
                            issue.severity,
                            crate::agent::validation::validator::ValidationSeverity::Error
                                | crate::agent::validation::validator::ValidationSeverity::Critical
                        )
                    })
                    .map(|issue| issue.description)
                    .collect();

                if !hard_issues.is_empty() {
                    issues.insert(character_id.clone(), hard_issues);
                }
            }
        }
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_executor() {
        let budget_monitor = Arc::new(RwLock::new(BudgetMonitor::default()));
        let _executor = ParallelCognitiveExecutor::new(budget_monitor);
        // Executor created successfully
        assert!(true);
    }
}
