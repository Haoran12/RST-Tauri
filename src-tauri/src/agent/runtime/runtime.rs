//! Agent runtime
//!
//! Main loop for Agent mode.
//!
//! Implements the fixed snapshot mechanism from docs/11_agent_runtime.md §2:
//! - RuntimeConfigSnapshot captured from app_runtime.yaml at turn start
//! - WorldRulesSnapshot captured from world_argument.yaml at turn start
//! - All operations during the turn use the fixed snapshots

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::budget_monitor::{BudgetConfig, BudgetMonitor, BudgetTraceEntry};
use super::config_snapshot::{RuntimeConfigSnapshot, SnapshotManager, WorldRulesSnapshot};
use super::state_committer::StateCommitter;
use super::trace::{StepName, StepStatus, TraceRecorder};
use super::turn_state::TurnWorkingState;
use crate::agent::knowledge::KnowledgeStore;
use crate::agent::models::*;
use crate::agent::simulation::ReactionWindowManager;
use crate::agent::storage::AgentStore;
use crate::config::world_argument::{load_world_argument_from_dir, WORLD_ARGUMENT_FILE_NAME};
use sqlx::Row;

// =============================================================================
// Public Types
// =============================================================================

/// Agent runtime - main loop
pub struct AgentRuntime {
    store: Arc<RwLock<AgentStore>>,
    reaction_window_manager: RefCell<ReactionWindowManager>,
    budget_monitor: RefCell<BudgetMonitor>,
    snapshot_manager: RefCell<SnapshotManager>,
    trace_recorder: RefCell<TraceRecorder>,
}

/// Turn result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TurnResult {
    pub scene_turn_id: String,
    pub narrative_text: String,
    pub canon_status: RuntimeTurnCanonStatus,
    /// Runtime config snapshot ID used for this turn
    pub runtime_config_snapshot_id: String,
    /// World rules snapshot ID used for this turn
    pub world_rules_snapshot_id: Option<String>,
}

/// Commit result
#[derive(Debug, Clone)]
pub struct CommitResult {
    pub scene_turn_id: String,
    pub canon_status: RuntimeTurnCanonStatus,
}

/// Character tier for cognitive pass scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CharacterTier {
    /// Background character - no cognitive pass
    TierC,
    /// Secondary NPC - simplified cognitive pass
    TierB,
    /// Main character/important NPC - full cognitive pass
    TierA,
}

/// Active set calculation result
#[derive(Debug, Clone)]
pub struct ActiveSetResult {
    /// Characters that need cognitive pass
    pub active_characters: Vec<String>,
    /// Dirty flags for each character
    pub dirty_flags: HashMap<String, DirtyFlags>,
    /// Tier for each character
    pub character_tiers: HashMap<String, CharacterTier>,
    /// Primary cognitive pass candidates (limited by budget)
    pub primary_candidates: Vec<String>,
    /// Characters deferred due to budget
    pub budget_deferred: Vec<String>,
    /// Reason for scheduling decisions
    pub scheduling_notes: Vec<String>,
}

/// Dirty flags for cognitive pass triggering
///
/// Tracks conditions that require a character to undergo cognitive pass.
/// Hard conditions (program-determinable) trigger the pass;
/// Soft conditions only provide hints to the LLM.
#[derive(Debug, Clone, Default)]
pub struct DirtyFlags {
    // === Hard conditions (trigger cognitive pass) ===
    /// Character was directly addressed/named in dialogue
    pub directly_addressed: bool,
    /// Character is under immediate threat/attack
    pub under_threat: bool,
    /// Reaction window is open for this character
    pub reaction_window_open: bool,
    /// Scene observable state changed significantly
    pub scene_changed: bool,
    /// Character's body state changed significantly
    pub body_changed: bool,
    /// Character received new accessible knowledge this turn
    pub knowledge_revealed: bool,

    // === Soft conditions (prompt hints only) ===
    /// Received new salient signal (not a trigger, just hint)
    pub received_new_salient_signal: bool,
    /// Prior belief was contradicted
    pub belief_invalidated: bool,
    /// Relationship model changed
    pub relation_changed: bool,
    /// Prior intent became impossible/invalid
    pub intent_invalidated: bool,

    // === Computed state ===
    /// Whether this character needs cognitive pass
    pub needs_cognitive_pass: bool,
    /// Reason for needing (or skipping) cognitive pass
    pub reason: String,
}

// =============================================================================
// Implementations
// =============================================================================

impl AgentRuntime {
    /// Create a new agent runtime
    pub fn new(store: Arc<RwLock<AgentStore>>) -> Self {
        Self {
            store,
            reaction_window_manager: RefCell::new(ReactionWindowManager::new()),
            budget_monitor: RefCell::new(BudgetMonitor::default()),
            snapshot_manager: RefCell::new(SnapshotManager::new()),
            trace_recorder: RefCell::new(TraceRecorder::new()),
        }
    }

    /// Create a new agent runtime with custom budget config
    pub fn with_budget_config(store: Arc<RwLock<AgentStore>>, config: BudgetConfig) -> Self {
        Self {
            store,
            reaction_window_manager: RefCell::new(ReactionWindowManager::new()),
            budget_monitor: RefCell::new(BudgetMonitor::new(config)),
            snapshot_manager: RefCell::new(SnapshotManager::new()),
            trace_recorder: RefCell::new(TraceRecorder::new()),
        }
    }

    /// Get the budget monitor
    pub fn budget_monitor(&self) -> &RefCell<BudgetMonitor> {
        &self.budget_monitor
    }

    /// Get the snapshot manager
    pub fn snapshot_manager(&self) -> &RefCell<SnapshotManager> {
        &self.snapshot_manager
    }

    /// Get the trace recorder
    pub fn trace_recorder(&self) -> &RefCell<TraceRecorder> {
        &self.trace_recorder
    }

    /// Capture configuration snapshots for a new turn
    ///
    /// This must be called at the start of each turn to ensure consistent
    /// configuration throughout the turn. The snapshots are fixed and will
    /// not change even if the user modifies configuration mid-turn.
    async fn capture_turn_snapshots(
        &self,
        world_id: &str,
    ) -> Result<(RuntimeConfigSnapshot, Option<WorldRulesSnapshot>), String> {
        let runtime_snapshot = self
            .snapshot_manager
            .borrow_mut()
            .capture_runtime_snapshot(vec!["app_runtime.yaml".to_string()]);

        let world_dir = self.resolve_world_directory().await?;
        let (world_argument, source_path) = load_world_argument_from_dir(&world_dir)?;
        let source_name = source_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(WORLD_ARGUMENT_FILE_NAME)
            .to_string();

        let world_snapshot = self
            .snapshot_manager
            .borrow_mut()
            .capture_world_snapshot(world_id.to_string(), vec![source_name], &world_argument)
            .map_err(|e| format!("Failed to capture world snapshot: {}", e))?;

        Ok((runtime_snapshot, Some(world_snapshot)))
    }

    async fn resolve_world_directory(&self) -> Result<PathBuf, String> {
        let pool = {
            let store = self.store.read().await;
            store.pool().clone()
        };

        let rows = sqlx::query("PRAGMA database_list")
            .fetch_all(&pool)
            .await
            .map_err(|e| format!("Failed to inspect SQLite database path: {}", e))?;

        let file_path = rows
            .into_iter()
            .find_map(|row| {
                let name: Result<String, _> = row.try_get("name");
                let file: Result<String, _> = row.try_get("file");
                match (name, file) {
                    (Ok(db_name), Ok(path)) if db_name == "main" && !path.trim().is_empty() => {
                        Some(path)
                    }
                    _ => None,
                }
            })
            .ok_or_else(|| "Could not resolve world SQLite file path".to_string())?;

        PathBuf::from(file_path)
            .parent()
            .map(|path| path.to_path_buf())
            .ok_or_else(|| "World SQLite file has no parent directory".to_string())
    }

    /// Process a user turn
    pub async fn process_turn(
        &mut self,
        session_id: &str,
        user_message: serde_json::Value,
    ) -> Result<TurnResult, String> {
        // Reset budget monitor for new turn
        self.budget_monitor.borrow_mut().reset();

        // Step 0: Load session and world state
        let session = self.load_session(session_id).await?;
        let world_cursor = self.load_world_cursor(&session.world_id).await?;

        // Capture configuration snapshots BEFORE any other operations
        // This ensures consistent configuration throughout the turn
        let (runtime_snapshot, world_snapshot) =
            self.capture_turn_snapshots(&session.world_id).await?;

        // Start turn trace
        let trace_id = self.trace_recorder.borrow_mut().start_turn(
            "pending_scene_turn", // Will be updated later
            Some(session_id),
            Some(&serde_json::to_string(&session.period_anchor).unwrap_or_default()),
            "canon",
            &runtime_snapshot.snapshot_id,
            world_snapshot.as_ref().map(|w| w.snapshot_id.as_str()),
        );

        tracing::info!(
            "Turn snapshots captured: runtime={}, world={}, trace={}",
            runtime_snapshot.snapshot_id,
            world_snapshot
                .as_ref()
                .map(|w| w.snapshot_id.as_str())
                .unwrap_or("none"),
            trace_id
        );

        // Update budget monitor from snapshot config
        self.budget_monitor
            .borrow_mut()
            .update_from_snapshot(&runtime_snapshot);

        // Determine timeline kind
        let _timeline_kind = self.determine_timeline_kind(&session, &world_cursor);

        // Step 1: Collect user input
        // (user_message already provided)
        self.trace_recorder.borrow_mut().add_key_output(
            "user_input",
            serde_json::to_string(&user_message).unwrap_or_default(),
        );

        // Step 1a: Check if scene initialization needed
        let scene = self.get_or_initialize_scene(&session).await?;
        self.trace_recorder
            .borrow_mut()
            .set_scene_turn_id(&scene.scene_turn_id);

        // Step 2: SceneStateExtractor
        self.trace_recorder.borrow_mut().record_step(
            StepName::SceneStateExtractor,
            StepStatus::Started,
            None,
            Some(serde_json::json!({ "scene_turn_id": scene.scene_turn_id }).to_string()),
            None,
            None,
        );

        let extractor_output = self
            .run_scene_state_extractor(&scene, &user_message, &session)
            .await?;

        // Record LLM call for SceneStateExtractor
        // (In production, actual token counts would come from the LLM response)
        self.budget_monitor.borrow_mut().record_llm_call(
            "SceneStateExtractor",
            2000, // placeholder
            500,  // placeholder
        );

        self.trace_recorder.borrow_mut().record_step(
            StepName::SceneStateExtractor,
            StepStatus::Succeeded,
            None,
            None,
            Some(
                serde_json::json!({
                    "user_input_delta_kind": format!("{:?}", extractor_output.user_input_delta.kind)
                })
                .to_string(),
            ),
            None,
        );

        // Step 3: Apply UserInputDelta to working state
        let mut working_state = TurnWorkingState::new_with_snapshots(
            scene.clone(),
            runtime_snapshot.snapshot_id.clone(),
            world_snapshot.as_ref().map(|w| w.snapshot_id.clone()),
        );
        working_state.raw_user_message = user_message.clone();
        self.apply_user_input_delta(&mut working_state, &extractor_output.user_input_delta)?;
        if let Some(scene_update) = &extractor_output.scene_update {
            working_state.apply_scene_delta(&scene_update.scene_delta)?;
        }
        working_state.provisional_truths = extractor_output.provisional_truth_candidates.clone();
        working_state.conflict_warnings = extractor_output.conflict_warnings.clone();

        let characters = {
            let store = self.store.read().await;
            store.list_characters().await?
        };
        self.load_character_runtime_state(&mut working_state, &characters);

        // Step 4: Update body/resources/state (mechanical evolution)
        self.trace_recorder.borrow_mut().record_step(
            StepName::MechanicalEvolution,
            StepStatus::Started,
            None,
            None,
            None,
            None,
        );
        self.apply_mechanical_evolution(&mut working_state)?;
        self.trace_recorder.borrow_mut().record_step(
            StepName::MechanicalEvolution,
            StepStatus::Succeeded,
            None,
            None,
            None,
            None,
        );

        // Step 4a: AttributeResolver
        let effective_attrs =
            self.resolve_attributes(&working_state, &characters, world_snapshot.as_ref())?;

        // Step 5: Generate event delta
        working_state.event_delta = self.generate_event_delta(&working_state)?;

        // Step 6: Calculate active set + dirty flags
        // Get max primary passes from budget config
        let max_primary_passes = self
            .budget_monitor
            .borrow()
            .config()
            .max_primary_cognitive_passes;

        let active_set = self.calculate_active_set(
            &working_state,
            &characters,
            session.player_character_id.as_deref(),
            max_primary_passes,
        )?;

        // Record active set trace
        self.trace_recorder.borrow_mut().record_step(
            StepName::ActiveSet,
            StepStatus::Succeeded,
            None,
            None,
            Some(
                serde_json::json!({
                    "active_characters": active_set.active_characters.len(),
                    "primary_candidates": active_set.primary_candidates.len(),
                    "budget_deferred": active_set.budget_deferred.len(),
                })
                .to_string(),
            ),
            None,
        );

        // Record budget deferred characters
        for character_id in &active_set.budget_deferred {
            self.budget_monitor
                .borrow_mut()
                .record_template_intent(character_id);
            self.trace_recorder
                .borrow_mut()
                .add_deferred_character(character_id);
        }

        // Step 7-12: Per active & dirty character
        let cognitive_outputs = self
            .run_cognitive_passes(
                &working_state,
                &active_set.primary_candidates,
                &active_set.dirty_flags,
                &effective_attrs,
            )
            .await?;

        // Record processed characters
        for character_id in cognitive_outputs.keys() {
            self.trace_recorder
                .borrow_mut()
                .add_processed_character(character_id);
        }

        let skills = self.load_runtime_skills(&characters).await?;

        // Step 12a-12d: Reaction window processing
        self.trace_recorder.borrow_mut().record_step(
            StepName::ReactionWindow,
            StepStatus::Started,
            None,
            None,
            None,
            None,
        );
        let reaction_intents = self
            .process_reaction_windows(
                &mut working_state,
                &cognitive_outputs,
                &characters,
                &effective_attrs,
                &skills,
            )
            .await?;
        self.trace_recorder.borrow_mut().record_step(
            StepName::ReactionWindow,
            StepStatus::Succeeded,
            None,
            None,
            Some(
                serde_json::json!({
                    "reaction_intents": reaction_intents.len()
                })
                .to_string(),
            ),
            None,
        );

        // Step 13: OutcomePlanner
        self.trace_recorder.borrow_mut().record_step(
            StepName::OutcomePlanning,
            StepStatus::Started,
            None,
            None,
            None,
            None,
        );
        let outcome = self
            .run_outcome_planner(
                &working_state,
                &cognitive_outputs,
                &reaction_intents,
                &skills,
                &session,
            )
            .await?;

        // Record LLM call for OutcomePlanner
        self.budget_monitor.borrow_mut().record_llm_call(
            "OutcomePlanner",
            3000, // placeholder
            800,  // placeholder
        );

        self.trace_recorder.borrow_mut().record_step(
            StepName::OutcomePlanning,
            StepStatus::Succeeded,
            None,
            None,
            Some(
                serde_json::json!({
                    "outward_actions": outcome.outcome_plan.outward_actions.len(),
                    "knowledge_reveal_events": outcome.knowledge_reveal_events.len(),
                })
                .to_string(),
            ),
            None,
        );

        // Step 14: SurfaceRealizer
        self.trace_recorder.borrow_mut().record_step(
            StepName::SurfaceRealization,
            StepStatus::Started,
            None,
            None,
            None,
            None,
        );
        let narrative = self.run_surface_realizer(&working_state, &outcome).await?;

        // Record LLM call for SurfaceRealizer
        self.budget_monitor.borrow_mut().record_llm_call(
            "SurfaceRealizer",
            2500, // placeholder
            1000, // placeholder
        );

        self.trace_recorder.borrow_mut().record_step(
            StepName::SurfaceRealization,
            StepStatus::Succeeded,
            None,
            None,
            Some(
                serde_json::json!({
                    "narrative_length": narrative.narrative_text.len(),
                    "used_fact_ids": narrative.used_fact_ids.len(),
                })
                .to_string(),
            ),
            None,
        );

        // Step 15: NarrativeFactCheck
        self.trace_recorder.borrow_mut().record_step(
            StepName::NarrativeFactCheck,
            StepStatus::Started,
            None,
            None,
            None,
            None,
        );
        self.validate_narrative(&narrative, &outcome)?;
        self.trace_recorder.borrow_mut().record_step(
            StepName::NarrativeFactCheck,
            StepStatus::Succeeded,
            None,
            None,
            None,
            None,
        );

        // Generate budget trace entry before commit
        let budget_report = self.budget_monitor.borrow_mut().generate_report();
        let budget_trace = BudgetTraceEntry::new(
            &working_state.scene.scene_turn_id,
            "turn_complete",
            budget_report,
        );

        // Log budget summary
        tracing::info!(
            "Turn budget summary: {} cognitive passes, {} LLM calls, {} input tokens, {} warnings",
            budget_trace.report.cognitive_passes,
            budget_trace.report.total_llm_calls,
            budget_trace.report.total_input_tokens,
            budget_trace.report.warnings.len()
        );

        // Set trace description
        self.trace_recorder.borrow_mut().set_description(&format!(
            "Turn completed with {} cognitive passes, {} LLM calls",
            budget_trace.report.cognitive_passes, budget_trace.report.total_llm_calls
        ));

        // Add warnings to trace
        for warning in &budget_trace.report.warnings {
            self.trace_recorder
                .borrow_mut()
                .add_warning(warning.message.clone());
        }

        // Step 16: StateCommitter
        self.trace_recorder.borrow_mut().record_step(
            StepName::StateCommit,
            StepStatus::Started,
            None,
            None,
            None,
            None,
        );
        let commit_result = self
            .commit_state(&working_state, &outcome, &narrative, &session)
            .await?;
        self.trace_recorder.borrow_mut().record_step(
            StepName::StateCommit,
            StepStatus::Succeeded,
            None,
            None,
            Some(
                serde_json::json!({
                    "scene_turn_id": commit_result.scene_turn_id,
                    "canon_status": format!("{:?}", commit_result.canon_status),
                })
                .to_string(),
            ),
            None,
        );

        // Finalize trace
        if let Some((turn_trace, step_traces)) = self.trace_recorder.borrow_mut().finalize() {
            tracing::info!("Turn trace finalized: {} steps recorded", step_traces.len());
            self.persist_config_snapshots(&runtime_snapshot, world_snapshot.as_ref())
                .await?;
            self.persist_turn_trace(&turn_trace, &step_traces).await?;
            self.attach_trace_to_commit(&commit_result.scene_turn_id, &turn_trace.trace_id)
                .await?;
        }

        Ok(TurnResult {
            scene_turn_id: commit_result.scene_turn_id,
            narrative_text: narrative.narrative_text,
            canon_status: commit_result.canon_status,
            runtime_config_snapshot_id: runtime_snapshot.snapshot_id,
            world_rules_snapshot_id: world_snapshot.map(|w| w.snapshot_id),
        })
    }

    /// Generate a budget trace for the current turn
    pub fn generate_budget_trace(&self, scene_turn_id: &str, step_name: &str) -> BudgetTraceEntry {
        let report = self.budget_monitor.borrow_mut().generate_report();
        BudgetTraceEntry::new(scene_turn_id, step_name, report)
    }

    /// Load session from storage
    async fn load_session(&self, session_id: &str) -> Result<AgentSession, String> {
        let store = self.store.read().await;
        store
            .get_session(session_id)
            .await?
            .ok_or_else(|| format!("Agent session '{}' not found", session_id))
    }

    /// Load world mainline cursor
    async fn load_world_cursor(&self, world_id: &str) -> Result<WorldMainlineCursor, String> {
        let store = self.store.read().await;
        if store.world_id() != world_id {
            return Err(format!(
                "Agent store is bound to world '{}', requested '{}'",
                store.world_id(),
                world_id
            ));
        }
        store.get_mainline_cursor().await
    }

    /// Determine timeline kind (mainline/retrospective/future_preview)
    fn determine_timeline_kind(
        &self,
        session: &AgentSession,
        cursor: &WorldMainlineCursor,
    ) -> AgentSessionKind {
        if session.period_anchor.ordinal < cursor.mainline_time_anchor.ordinal {
            AgentSessionKind::Retrospective
        } else if session.period_anchor.ordinal > cursor.mainline_time_anchor.ordinal {
            AgentSessionKind::FuturePreview
        } else {
            AgentSessionKind::Mainline
        }
    }

    /// Get or initialize scene
    async fn get_or_initialize_scene(&self, session: &AgentSession) -> Result<SceneModel, String> {
        if let Some(mut scene) = self.load_latest_scene_snapshot(&session.session_id).await? {
            scene.scene_turn_id = format!("turn_{}", uuid::Uuid::new_v4());
            scene.time_context.time_anchor = session.period_anchor.clone();
            return Ok(scene);
        }

        let mut scene = default_scene_model(session);
        let characters = {
            let store = self.store.read().await;
            store.list_characters().await?
        };
        populate_scene_entities_from_characters(
            &mut scene,
            &characters,
            session.player_character_id.as_deref(),
        );
        Ok(scene)
    }

    async fn load_latest_scene_snapshot(
        &self,
        session_id: &str,
    ) -> Result<Option<SceneModel>, String> {
        let pool = {
            let store = self.store.read().await;
            store.pool().clone()
        };

        let scene_json: Option<String> = sqlx::query_scalar(
            r#"
            SELECT ss.scene_model
            FROM scene_snapshots ss
            JOIN world_turns wt ON wt.scene_turn_id = ss.scene_turn_id
            WHERE wt.session_id = ? AND wt.status = 'active'
            ORDER BY ss.created_at DESC
            LIMIT 1
            "#,
        )
        .bind(session_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| format!("Failed to load latest scene snapshot: {}", e))?;

        match scene_json {
            Some(json) => serde_json::from_str::<SceneModel>(&json)
                .map(Some)
                .map_err(|e| format!("Failed to parse latest scene snapshot: {}", e)),
            None => Ok(None),
        }
    }

    /// Run SceneStateExtractor
    async fn run_scene_state_extractor(
        &self,
        scene: &SceneModel,
        user_message: &serde_json::Value,
        session: &AgentSession,
    ) -> Result<SceneStateExtractorOutput, String> {
        let recent_free_text = user_message_to_text(user_message)?;
        Ok(deterministic_user_input_delta(
            scene,
            session,
            recent_free_text,
        ))
    }

    /// Apply UserInputDelta to working state
    fn apply_user_input_delta(
        &self,
        working_state: &mut TurnWorkingState,
        delta: &UserInputDelta,
    ) -> Result<(), String> {
        working_state.user_input_delta = Some(delta.clone());

        // Apply scene delta if present
        match &delta.kind {
            UserInputKind::SceneNarration { scene_delta } => {
                working_state.apply_scene_delta(scene_delta)?;
            }
            UserInputKind::CharacterRoleplay {
                character_id,
                intent_plan,
                ..
            } => {
                working_state
                    .character_intents
                    .insert(character_id.clone(), intent_plan.clone());
            }
            UserInputKind::DirectorHint { outcome_bias, .. } => {
                working_state.director_hint = outcome_bias.clone();
            }
            _ => {}
        }

        Ok(())
    }

    /// Apply mechanical evolution
    fn apply_mechanical_evolution(
        &self,
        working_state: &mut TurnWorkingState,
    ) -> Result<(), String> {
        for state in working_state.character_states.values_mut() {
            for cooldown in &mut state.cooldowns {
                if cooldown.remaining_turns > 0 {
                    cooldown.remaining_turns -= 1;
                }
            }
            state
                .cooldowns
                .retain(|cooldown| cooldown.remaining_turns > 0);
            state.fatigue = (state.fatigue * 0.98).clamp(0.0, 1.0);
            state.pain_load = (state.pain_load * 0.99).clamp(0.0, 1.0);
        }
        Ok(())
    }

    fn load_character_runtime_state(
        &self,
        working_state: &mut TurnWorkingState,
        characters: &[CharacterRecord],
    ) {
        for character in characters {
            working_state.character_states.insert(
                character.character_id.clone(),
                character.temporary_state.clone(),
            );
        }
    }

    /// Resolve attributes for all characters
    fn resolve_attributes(
        &self,
        working_state: &TurnWorkingState,
        characters: &[CharacterRecord],
        world_snapshot: Option<&WorldRulesSnapshot>,
    ) -> Result<HashMap<String, EffectiveAttributeProfile>, String> {
        let mut profiles = HashMap::new();
        for character in characters {
            let temp_state = working_state
                .character_states
                .get(&character.character_id)
                .unwrap_or(&character.temporary_state);
            let injury_penalty = temp_state.injuries.len() as f64 * 5.0;
            let fatigue_factor = 1.0 - (temp_state.fatigue * 0.25).clamp(0.0, 0.25);
            let pain_factor = 1.0 - (temp_state.pain_load * 0.15).clamp(0.0, 0.15);

            let mut values = HashMap::new();
            values.insert(
                AttributeKind::Physical,
                ((character.base_attributes.physical - injury_penalty) * fatigue_factor).max(0.0),
            );
            values.insert(
                AttributeKind::Agility,
                ((character.base_attributes.agility - injury_penalty) * fatigue_factor).max(0.0),
            );
            values.insert(
                AttributeKind::Endurance,
                (character.base_attributes.endurance * pain_factor).max(0.0),
            );
            values.insert(
                AttributeKind::Insight,
                (character.base_attributes.insight * pain_factor).max(0.0),
            );
            values.insert(
                AttributeKind::ManaPower,
                character.base_attributes.mana_power.max(0.0),
            );
            values.insert(
                AttributeKind::SoulStrength,
                character.base_attributes.soul_strength.max(0.0),
            );

            let tiers = values
                .iter()
                .map(|(kind, value)| {
                    let tier = world_snapshot
                        .map(|snapshot| snapshot.attribute_tier_for_value(*value))
                        .unwrap_or_else(|| AttributeTier::from_value(*value));
                    (*kind, tier)
                })
                .collect();
            let descriptors = values
                .iter()
                .map(|(kind, value)| {
                    let mut labels = Vec::new();
                    if temp_state.fatigue > 0.5 {
                        labels.push("fatigued".to_string());
                    }
                    if temp_state.pain_load > 0.4 {
                        labels.push("in pain".to_string());
                    }
                    if *value <= 0.0 {
                        labels.push("blocked".to_string());
                    }
                    (*kind, labels)
                })
                .collect();

            profiles.insert(
                character.character_id.clone(),
                EffectiveAttributeProfile {
                    character_id: character.character_id.clone(),
                    values,
                    tiers,
                    descriptors,
                },
            );
        }

        Ok(profiles)
    }

    /// Generate event delta
    fn generate_event_delta(
        &self,
        working_state: &TurnWorkingState,
    ) -> Result<Vec<ObservableEventDelta>, String> {
        let mut events: Vec<ObservableEventDelta> = working_state
            .scene
            .event_stream
            .iter()
            .map(|event| ObservableEventDelta {
                event_id: event.event_id.clone(),
                scene_turn_id: working_state.scene.scene_turn_id.clone(),
                event_kind: event.event_kind.clone(),
                involved_observable_entities: event.involved_entity_ids.clone(),
                observable_effects: event.payload.clone(),
                sensory_descriptors: Vec::new(),
                source_hint: None,
            })
            .collect();

        if let Some(delta) = &working_state.user_input_delta {
            match &delta.kind {
                UserInputKind::CharacterRoleplay {
                    character_id,
                    intent_plan,
                    actions,
                    ..
                } => {
                    events.push(ObservableEventDelta {
                        event_id: format!("evt_{}", uuid::Uuid::new_v4()),
                        scene_turn_id: working_state.scene.scene_turn_id.clone(),
                        event_kind: intent_plan.intent_kind.clone(),
                        involved_observable_entities: std::iter::once(character_id.clone())
                            .chain(intent_plan.target_refs.iter().cloned())
                            .collect(),
                        observable_effects: serde_json::json!({
                            "raw_text": delta.raw_text,
                            "actions": actions,
                        }),
                        sensory_descriptors: vec![delta.raw_text.clone()],
                        source_hint: None,
                    });
                }
                UserInputKind::DirectorHint { .. } => {
                    events.push(ObservableEventDelta {
                        event_id: format!("evt_{}", uuid::Uuid::new_v4()),
                        scene_turn_id: working_state.scene.scene_turn_id.clone(),
                        event_kind: "director_hint".to_string(),
                        involved_observable_entities: Vec::new(),
                        observable_effects: serde_json::json!({ "raw_text": delta.raw_text }),
                        sensory_descriptors: Vec::new(),
                        source_hint: None,
                    });
                }
                UserInputKind::SceneNarration { .. } => {
                    events.push(ObservableEventDelta {
                        event_id: format!("evt_{}", uuid::Uuid::new_v4()),
                        scene_turn_id: working_state.scene.scene_turn_id.clone(),
                        event_kind: "scene_narration".to_string(),
                        involved_observable_entities: working_state
                            .scene
                            .entities
                            .iter()
                            .map(|entity| entity.entity_id.clone())
                            .collect(),
                        observable_effects: serde_json::json!({ "raw_text": delta.raw_text }),
                        sensory_descriptors: vec![delta.raw_text.clone()],
                        source_hint: None,
                    });
                }
                UserInputKind::MetaCommand { .. } => {}
            }
        }

        Ok(events)
    }

    /// Calculate active set and dirty flags
    ///
    /// This determines which characters need cognitive passes and schedules them
    /// based on priority rules and budget constraints.
    pub fn calculate_active_set(
        &self,
        working_state: &TurnWorkingState,
        characters: &[CharacterRecord],
        player_character_id: Option<&str>,
        max_primary_passes: usize,
    ) -> Result<ActiveSetResult, String> {
        let mut active_characters = Vec::new();
        let mut dirty_flags = HashMap::new();
        let mut character_tiers = HashMap::new();
        let mut notes = Vec::new();

        // Step 1: Compute dirty flags for each character
        for character in characters {
            let flags = DirtyFlags::compute(
                &character.character_id,
                &working_state.scene,
                &working_state.event_delta,
                &working_state.user_input_delta,
                &HashSet::new(), // TODO: Track prior knowledge
                &HashSet::new(), // TODO: Track new knowledge
                player_character_id,
            );

            character_tiers.insert(
                character.character_id.clone(),
                CharacterTier::from_character(
                    character,
                    player_character_id == Some(&character.character_id),
                ),
            );

            if flags.needs_cognitive_pass {
                active_characters.push(character.character_id.clone());
            }

            dirty_flags.insert(character.character_id.clone(), flags);
        }

        // Step 2: Sort by priority
        let mut sorted_active: Vec<String> = active_characters.clone();
        sorted_active.sort_by(|a, b| {
            // Sort by tier (A > B > C), then by trigger priority
            let tier_a = character_tiers.get(a).unwrap_or(&CharacterTier::TierC);
            let tier_b = character_tiers.get(b).unwrap_or(&CharacterTier::TierC);

            match tier_a.cmp(tier_b).reverse() {
                std::cmp::Ordering::Equal => {
                    // Within same tier, prioritize by trigger type
                    let flags_a = dirty_flags.get(a).unwrap();
                    let flags_b = dirty_flags.get(b).unwrap();

                    // under_threat > directly_addressed > reaction_window > others
                    let priority_a = trigger_priority(flags_a);
                    let priority_b = trigger_priority(flags_b);
                    priority_a.cmp(&priority_b).reverse()
                }
                other => other,
            }
        });

        // Step 3: Apply budget constraints
        let primary_candidates: Vec<String> = sorted_active
            .iter()
            .take(max_primary_passes)
            .cloned()
            .collect();

        let budget_deferred: Vec<String> = sorted_active
            .iter()
            .skip(max_primary_passes)
            .cloned()
            .collect();

        if !budget_deferred.is_empty() {
            notes.push(format!(
                "Budget deferred {} characters: {}",
                budget_deferred.len(),
                budget_deferred.join(", ")
            ));
        }

        Ok(ActiveSetResult {
            active_characters: sorted_active,
            dirty_flags,
            character_tiers,
            primary_candidates,
            budget_deferred,
            scheduling_notes: notes,
        })
    }

    /// Run cognitive passes for active characters (parallel execution)
    ///
    /// Uses ParallelCognitiveExecutor to run multiple cognitive passes concurrently.
    /// All passes read from the fixed snapshot and working state, producing candidates
    /// without any persistent writes.
    async fn run_cognitive_passes(
        &self,
        working_state: &TurnWorkingState,
        active_characters: &[String],
        _dirty_flags: &HashMap<String, DirtyFlags>,
        effective_attrs: &HashMap<String, EffectiveAttributeProfile>,
    ) -> Result<HashMap<String, CharacterCognitivePassOutput>, String> {
        use super::parallel_cognitive::ParallelCognitiveExecutor;

        if active_characters.is_empty() {
            return Ok(HashMap::new());
        }

        // Create parallel executor with budget monitor
        let budget_monitor = Arc::new(RwLock::new(self.budget_monitor.borrow().clone()));
        let pool = {
            let store = self.store.read().await;
            store.pool().clone()
        };
        let executor = ParallelCognitiveExecutor::with_pool(budget_monitor.clone(), pool);

        // Execute cognitive passes in parallel
        tracing::info!(
            "Starting parallel cognitive passes for {} characters",
            active_characters.len()
        );

        let results = executor
            .execute_parallel(working_state, active_characters, effective_attrs)
            .await;

        // Update budget monitor from the parallel execution
        {
            let parallel_monitor = budget_monitor.read().await;
            let report = parallel_monitor.generate_report();
            // Sync back to our budget monitor
            self.budget_monitor.borrow_mut().sync_from_report(&report);
        }

        // Collect successful outputs
        let mut outputs = HashMap::new();
        for (character_id, result) in &results {
            match &result.output {
                Ok(output) => {
                    outputs.insert(character_id.clone(), output.clone());
                    tracing::info!(
                        "Cognitive pass completed for character {} ({} input tokens, {} output tokens)",
                        character_id,
                        result.input_tokens,
                        result.output_tokens
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "Cognitive pass failed for character {}: {:?}",
                        character_id,
                        e
                    );
                    outputs.insert(
                        character_id.clone(),
                        deterministic_cognitive_output(character_id, working_state),
                    );
                }
            }
        }

        Ok(outputs)
    }

    /// Process reaction windows
    async fn process_reaction_windows(
        &self,
        working_state: &mut TurnWorkingState,
        cognitive_outputs: &HashMap<String, CharacterCognitivePassOutput>,
        characters: &[CharacterRecord],
        effective_attrs: &HashMap<String, EffectiveAttributeProfile>,
        skills: &[Skill],
    ) -> Result<Vec<ReactionIntent>, String> {
        let relationships = {
            let store = self.store.read().await;
            store.list_objective_relationships().await?
        };

        // Check if any cognitive outputs open reaction windows
        for (_character_id, output) in cognitive_outputs {
            // Check intent_plan for actions that might trigger reactions
            let intent = &output.intent_plan;
            working_state
                .character_intents
                .insert(intent.character_id.clone(), intent.clone());
            if intent.intent_kind.contains("attack")
                || intent.intent_kind.contains("threat")
                || intent.intent_kind.contains("interrupt")
            {
                // Open reaction window
                // Note: This is a placeholder - in production we would need to pass
                // the full list of characters and effective attributes
                let _window_id = self.reaction_window_manager.borrow_mut().open_window(
                    &working_state.scene.scene_turn_id,
                    &intent.intent_kind,
                    &intent.character_id, // source_action_id
                    &intent.character_id, // threat_source_id
                    &intent.target_refs,
                    &working_state.scene,
                    characters,
                    &relationships,
                    skills,
                    effective_attrs,
                )?;
                if let Some(window) = self
                    .reaction_window_manager
                    .borrow()
                    .get_window(&_window_id)
                    .cloned()
                {
                    working_state.pending_reactions.push(window);
                }
            }
        }

        // Collect reaction intents from eligible reactors
        let mut intents = Vec::new();
        for window in self
            .reaction_window_manager
            .borrow()
            .get_active_windows(&working_state.scene.scene_turn_id)
        {
            for eligibility in &window.eligible_reactors {
                if let Some(option) = eligibility.available_reaction_options.first() {
                    intents.push(ReactionIntent {
                        window_id: window.window_id.clone(),
                        character_id: eligibility.character_id.clone(),
                        chosen_option_id: option.option_id.clone(),
                        target_ids: option.target_scope.clone(),
                        intent_rationale: format!(
                            "Deterministic reaction selected from {:?}",
                            eligibility.reason
                        ),
                    });
                }
            }
        }

        Ok(intents)
    }

    async fn load_runtime_skills(
        &self,
        characters: &[CharacterRecord],
    ) -> Result<Vec<Skill>, String> {
        let pool = {
            let store = self.store.read().await;
            store.pool().clone()
        };
        let knowledge_store = KnowledgeStore::new(pool);
        let mut skills = Vec::new();

        for character in characters {
            for facet_type in [
                CharacterFacetType::KnownAbility,
                CharacterFacetType::HiddenAbility,
            ] {
                let entries = knowledge_store
                    .query_character_facets(&character.character_id, Some(facet_type))
                    .await?;
                for entry in entries {
                    if let Some(skill) = Skill::from_character_ability_entry(&entry)? {
                        skills.push(skill);
                    }
                }
            }
        }

        skills.extend(self.collect_condition_derived_skills(characters));
        let mut dedupe = HashSet::new();
        skills.retain(|skill| {
            let owner = skill.owner_character_id().unwrap_or_default();
            dedupe.insert(format!("{owner}::{}", skill.skill_id))
        });

        Ok(skills)
    }

    fn collect_condition_derived_skills(&self, characters: &[CharacterRecord]) -> Vec<Skill> {
        let mut skills = Vec::new();
        for character in characters {
            for condition in &character.temporary_state.active_conditions {
                if condition
                    .condition_kind
                    .to_ascii_lowercase()
                    .contains("passive_field")
                {
                    skills.push(Skill {
                        skill_id: format!("{}:passive_field", character.character_id),
                        name: "Passive Field".to_string(),
                        description: "Derived runtime passive field reaction.".to_string(),
                        skill_kind: SkillKind::Passive,
                        activation: SkillActivation {
                            activation_time: ActivationTime::Reaction,
                            trigger_conditions: Vec::new(),
                            cooldown: None,
                            uses_per_scene: None,
                            uses_per_day: None,
                        },
                        effect_contract: SkillEffectContract {
                            primary_effects: Vec::new(),
                            secondary_effects: Vec::new(),
                            target_kind: TargetKind::Area,
                            target_count: TargetCount::Area,
                            range_m: Some(10.0),
                            area_of_effect: None,
                            duration_turns: None,
                            attribute_modifier: None,
                            mana_attribute: None,
                            allowed_target_kinds: vec![TargetKind::Area, TargetKind::Character],
                            allowed_state_domains: vec!["scene".to_string(), "body".to_string()],
                            max_intensity_tier: EffectIntensityTier::Moderate,
                            allows_injury: false,
                            allows_position_change: false,
                            allows_knowledge_reveal: false,
                        },
                        requirements: SkillRequirements {
                            minimum_attributes: Vec::new(),
                            required_skills: Vec::new(),
                            required_knowledge: Vec::new(),
                            prohibited_conditions: Vec::new(),
                            material_components: Vec::new(),
                            cost: CostProfile::default(),
                        },
                        metadata: SkillMetadata {
                            tags: vec![
                                "runtime".to_string(),
                                "passive_field".to_string(),
                                format!("owner:{}", character.character_id),
                            ],
                            source: Some("runtime".to_string()),
                            learning_difficulty: LearningDifficulty::Common,
                            rarity: SkillRarity::Common,
                        },
                        schema_version: "0.1".to_string(),
                        created_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                    });
                }

                if condition
                    .condition_kind
                    .to_ascii_lowercase()
                    .contains("interrupt_ready")
                {
                    skills.push(Skill {
                        skill_id: format!("{}:interrupt", character.character_id),
                        name: "Interrupt".to_string(),
                        description: "Derived runtime interrupt reaction.".to_string(),
                        skill_kind: SkillKind::Reaction,
                        activation: SkillActivation {
                            activation_time: ActivationTime::Reaction,
                            trigger_conditions: vec![ActivationCondition::TargetInLineOfSight],
                            cooldown: Some(1),
                            uses_per_scene: None,
                            uses_per_day: None,
                        },
                        effect_contract: SkillEffectContract {
                            primary_effects: Vec::new(),
                            secondary_effects: Vec::new(),
                            target_kind: TargetKind::Character,
                            target_count: TargetCount::Single,
                            range_m: Some(12.0),
                            area_of_effect: None,
                            duration_turns: None,
                            attribute_modifier: None,
                            mana_attribute: None,
                            allowed_target_kinds: vec![TargetKind::Character],
                            allowed_state_domains: vec![
                                "body".to_string(),
                                "position".to_string(),
                                "scene".to_string(),
                            ],
                            max_intensity_tier: EffectIntensityTier::Moderate,
                            allows_injury: true,
                            allows_position_change: true,
                            allows_knowledge_reveal: false,
                        },
                        requirements: SkillRequirements {
                            minimum_attributes: Vec::new(),
                            required_skills: Vec::new(),
                            required_knowledge: Vec::new(),
                            prohibited_conditions: Vec::new(),
                            material_components: Vec::new(),
                            cost: CostProfile {
                                mana_reserve_delta: Some(-5.0),
                                fatigue_delta: Some(0.1),
                                cooldown_turns: Some(1),
                                material_refs: Vec::new(),
                                required_conditions: vec![condition.condition_kind.clone()],
                            },
                        },
                        metadata: SkillMetadata {
                            tags: vec![
                                "runtime".to_string(),
                                "interrupt".to_string(),
                                format!("owner:{}", character.character_id),
                            ],
                            source: Some("runtime".to_string()),
                            learning_difficulty: LearningDifficulty::Common,
                            rarity: SkillRarity::Common,
                        },
                        schema_version: "0.1".to_string(),
                        created_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                    });
                }
            }
        }

        skills
    }

    /// Run OutcomePlanner
    async fn run_outcome_planner(
        &self,
        working_state: &TurnWorkingState,
        cognitive_outputs: &HashMap<String, CharacterCognitivePassOutput>,
        reaction_intents: &[ReactionIntent],
        skills: &[Skill],
        session: &AgentSession,
    ) -> Result<OutcomePlannerOutput, String> {
        let user_roleplay_intents = match &working_state.user_input_delta {
            Some(UserInputDelta {
                kind: UserInputKind::CharacterRoleplay { intent_plan, .. },
                ..
            }) => vec![intent_plan.clone()],
            _ => Vec::new(),
        };

        let character_records = {
            let store = self.store.read().await;
            store.list_characters().await?
        };

        let input = OutcomePlannerInput {
            scene_turn_id: working_state.scene.scene_turn_id.clone(),
            session_context: build_session_context(session),
            truth_guidance: None,
            scene_model: working_state.scene.clone(),
            character_records,
            relevant_knowledge: Vec::new(),
            skills: skills.to_vec(),
            character_outputs: cognitive_outputs.values().cloned().collect(),
            user_roleplay_intents,
            minor_actor_slots: Vec::new(),
            reaction_windows: working_state.pending_reactions.clone(),
            reaction_intents: reaction_intents.to_vec(),
            director_hint: working_state.director_hint.clone(),
            provisional_truth_candidates: working_state.provisional_truths.clone(),
        };

        Ok(deterministic_outcome_plan(working_state, input))
    }

    /// Run SurfaceRealizer
    async fn run_surface_realizer(
        &self,
        working_state: &TurnWorkingState,
        outcome: &OutcomePlannerOutput,
    ) -> Result<SurfaceRealizerOutput, String> {
        let scene_view = SceneNarrativeView {
            scene_id: working_state.scene.scene_id.clone(),
            scene_turn_id: working_state.scene.scene_turn_id.clone(),
            narration_scope: NarrationScope::ObjectiveCamera,
            visible_entities: working_state
                .scene
                .entities
                .iter()
                .map(|entity| NarrativeEntityView {
                    entity_id: entity.entity_id.clone(),
                    display_name: entity.display_name.clone(),
                    observable_facts: entity.observable_facets.clone(),
                    outward_state: vec![entity.posture.clone()],
                })
                .collect(),
            visible_environment: serde_json::json!({
                "scene_mood": working_state.scene.scene_mood,
                "uncertainty_notes": working_state.scene.uncertainty_notes,
            }),
            visible_events: working_state
                .scene
                .event_stream
                .iter()
                .map(|event| NarrativeEventView {
                    event_id: event.event_id.clone(),
                    event_kind: event.event_kind.clone(),
                    narratable_fact_refs: outcome
                        .outcome_plan
                        .narratable_facts
                        .iter()
                        .filter(|fact| {
                            fact.source_refs
                                .iter()
                                .any(|source| source == &event.event_id)
                        })
                        .map(|fact| fact.fact_id.clone())
                        .collect(),
                })
                .collect(),
            allowed_private_refs: Vec::new(),
        };

        let character_views = outcome
            .outcome_plan
            .outward_actions
            .iter()
            .fold(
                std::collections::BTreeMap::<String, NarrativeCharacterView>::new(),
                |mut acc, action| {
                    let entry = acc.entry(action.actor_id.clone()).or_insert_with(|| {
                        NarrativeCharacterView {
                            character_id: action.actor_id.clone(),
                            display_name: action.actor_id.clone(),
                            outward_actions: Vec::new(),
                            outward_reactions: Vec::new(),
                            allowed_inner_summary: None,
                        }
                    });
                    entry.outward_actions.push(action.action_kind.clone());
                    acc
                },
            )
            .into_values()
            .collect();

        let input = SurfaceRealizerInput {
            scene_turn_id: working_state.scene.scene_turn_id.clone(),
            narration_scope: NarrationScope::ObjectiveCamera,
            scene_view,
            character_views,
            outcome_plan: outcome.outcome_plan.clone(),
            style: default_style_constraints(),
        };

        Ok(deterministic_surface_output(&input))
    }

    /// Validate narrative
    fn validate_narrative(
        &self,
        narrative: &SurfaceRealizerOutput,
        outcome: &OutcomePlannerOutput,
    ) -> Result<(), String> {
        let allowed_fact_ids: HashSet<&str> = outcome
            .outcome_plan
            .narratable_facts
            .iter()
            .map(|fact| fact.fact_id.as_str())
            .collect();

        for fact_id in &narrative.used_fact_ids {
            if !allowed_fact_ids.contains(fact_id.as_str()) {
                return Err(format!(
                    "NarrativeFactCheck failed: used_fact_id '{}' is outside outcome_plan.narratable_facts",
                    fact_id
                ));
            }
        }

        Ok(())
    }

    /// Commit state changes
    async fn commit_state(
        &self,
        working_state: &TurnWorkingState,
        outcome: &OutcomePlannerOutput,
        narrative: &SurfaceRealizerOutput,
        session: &AgentSession,
    ) -> Result<CommitResult, String> {
        let pool = {
            let store = self.store.read().await;
            store.pool().clone()
        };
        let committer = StateCommitter::new(pool);
        let result = committer
            .commit(
                &working_state.scene.scene_turn_id,
                session,
                outcome,
                narrative,
                working_state,
            )
            .await?;

        Ok(CommitResult {
            scene_turn_id: result.scene_turn_id,
            canon_status: result.canon_status,
        })
    }

    async fn persist_config_snapshots(
        &self,
        runtime_snapshot: &RuntimeConfigSnapshot,
        world_snapshot: Option<&WorldRulesSnapshot>,
    ) -> Result<(), String> {
        let pool = {
            let store = self.store.read().await;
            store.pool().clone()
        };

        sqlx::query(
            r#"
            INSERT OR IGNORE INTO config_snapshots (
                config_snapshot_id, snapshot_kind, scope, world_id, schema_version,
                config_hash, source_paths, compiled_summary, created_at
            ) VALUES (?, 'runtime_config', 'global', NULL, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&runtime_snapshot.snapshot_id)
        .bind(runtime_snapshot.schema_version as i64)
        .bind(&runtime_snapshot.config_hash)
        .bind(
            serde_json::to_string(&runtime_snapshot.source_paths)
                .unwrap_or_else(|_| "[]".to_string()),
        )
        .bind(serde_json::to_string(runtime_snapshot).unwrap_or_else(|_| "{}".to_string()))
        .bind(runtime_snapshot.created_at.to_rfc3339())
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to persist runtime config snapshot: {}", e))?;

        if let Some(world_snapshot) = world_snapshot {
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO config_snapshots (
                    config_snapshot_id, snapshot_kind, scope, world_id, schema_version,
                    config_hash, source_paths, compiled_summary, created_at
                ) VALUES (?, 'world_rules', 'world', ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&world_snapshot.snapshot_id)
            .bind(&world_snapshot.world_id)
            .bind(world_snapshot.schema_version as i64)
            .bind(&world_snapshot.config_hash)
            .bind(
                serde_json::to_string(&world_snapshot.source_paths)
                    .unwrap_or_else(|_| "[]".to_string()),
            )
            .bind(serde_json::to_string(world_snapshot).unwrap_or_else(|_| "{}".to_string()))
            .bind(world_snapshot.created_at.to_rfc3339())
            .execute(&pool)
            .await
            .map_err(|e| format!("Failed to persist world rules snapshot: {}", e))?;
        }

        Ok(())
    }

    async fn persist_turn_trace(
        &self,
        turn_trace: &super::trace::TurnTrace,
        step_traces: &[super::trace::StepTrace],
    ) -> Result<(), String> {
        let pool = {
            let store = self.store.read().await;
            store.pool().clone()
        };

        sqlx::query(
            r#"
            INSERT INTO turn_traces (
                trace_id, scene_turn_id, session_id, story_time_anchor,
                runtime_turn_status, trace_kind, character_id,
                runtime_config_snapshot_id, world_rules_snapshot_id, summary,
                linked_request_ids, linked_event_ids, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&turn_trace.trace_id)
        .bind(&turn_trace.scene_turn_id)
        .bind(&turn_trace.session_id)
        .bind(&turn_trace.story_time_anchor)
        .bind(&turn_trace.runtime_turn_status)
        .bind(trace_kind_to_str(turn_trace.trace_kind))
        .bind(&turn_trace.character_id)
        .bind(&turn_trace.runtime_config_snapshot_id)
        .bind(&turn_trace.world_rules_snapshot_id)
        .bind(serde_json::to_string(&turn_trace.summary).unwrap_or_else(|_| "{}".to_string()))
        .bind(
            serde_json::to_string(&turn_trace.linked_request_ids)
                .unwrap_or_else(|_| "[]".to_string()),
        )
        .bind(
            serde_json::to_string(&turn_trace.linked_event_ids)
                .unwrap_or_else(|_| "[]".to_string()),
        )
        .bind(turn_trace.created_at.to_rfc3339())
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to persist turn trace: {}", e))?;

        for step in step_traces {
            sqlx::query(
                r#"
                INSERT INTO agent_step_traces (
                    step_trace_id, trace_id, scene_turn_id, character_id, step_name,
                    step_status, input_summary, output_summary, decision_json,
                    linked_request_id, error_event_id, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&step.step_trace_id)
            .bind(&step.trace_id)
            .bind(&step.scene_turn_id)
            .bind(&step.character_id)
            .bind(step.step_name.as_str())
            .bind(step_status_to_str(step.step_status))
            .bind(&step.input_summary)
            .bind(&step.output_summary)
            .bind(&step.decision_json)
            .bind(&step.linked_request_id)
            .bind(&step.error_event_id)
            .bind(step.created_at.to_rfc3339())
            .execute(&pool)
            .await
            .map_err(|e| format!("Failed to persist step trace: {}", e))?;
        }

        Ok(())
    }

    async fn attach_trace_to_commit(
        &self,
        scene_turn_id: &str,
        trace_id: &str,
    ) -> Result<(), String> {
        let pool = {
            let store = self.store.read().await;
            store.pool().clone()
        };

        sqlx::query("UPDATE state_commit_records SET trace_ids = ? WHERE scene_turn_id = ?")
            .bind(serde_json::to_string(&vec![trace_id]).unwrap_or_else(|_| "[]".to_string()))
            .bind(scene_turn_id)
            .execute(&pool)
            .await
            .map_err(|e| format!("Failed to attach trace to commit record: {}", e))?;

        Ok(())
    }
}

impl CharacterTier {
    /// Determine tier from character attributes and role
    pub fn from_character(character: &CharacterRecord, is_player: bool) -> Self {
        // Player character is always Tier A
        if is_player {
            return CharacterTier::TierA;
        }

        // Determine tier based on attribute tier
        let max_attribute = character
            .base_attributes
            .physical
            .max(character.base_attributes.mana_power)
            .max(character.base_attributes.soul_strength);

        let attr_tier = AttributeTier::from_value(max_attribute);

        match attr_tier {
            AttributeTier::Transcendent | AttributeTier::Ascendant | AttributeTier::Master => {
                CharacterTier::TierA
            }
            AttributeTier::Adept => CharacterTier::TierB,
            _ => CharacterTier::TierC,
        }
    }
}

impl DirtyFlags {
    /// Create dirty flags from scene state and events
    pub fn compute(
        character_id: &str,
        scene: &SceneModel,
        event_delta: &[ObservableEventDelta],
        user_input_delta: &Option<UserInputDelta>,
        prior_knowledge_ids: &HashSet<String>,
        new_knowledge_ids: &HashSet<String>,
        player_character_id: Option<&str>,
    ) -> Self {
        let mut flags = Self::default();

        // Skip player-controlled character (their actions come from user input)
        if player_character_id == Some(character_id) {
            flags.needs_cognitive_pass = false;
            flags.reason = "Player-controlled character".to_string();
            return flags;
        }

        // Check directly_addressed
        flags.directly_addressed = Self::check_directly_addressed(character_id, user_input_delta);

        // Check under_threat
        flags.under_threat = Self::check_under_threat(character_id, event_delta);

        // Check scene_changed
        flags.scene_changed = Self::check_scene_changed(character_id, scene, event_delta);

        // Check body_changed
        flags.body_changed = Self::check_body_changed(character_id, event_delta);

        // Check knowledge_revealed
        flags.knowledge_revealed =
            Self::check_knowledge_revealed(character_id, prior_knowledge_ids, new_knowledge_ids);

        // Compute needs_cognitive_pass
        flags.needs_cognitive_pass = flags.should_trigger_cognitive_pass();
        flags.reason = flags.compute_reason();

        flags
    }

    /// Check if cognitive pass should be triggered (hard conditions)
    pub fn should_trigger_cognitive_pass(&self) -> bool {
        self.directly_addressed
            || self.under_threat
            || self.reaction_window_open
            || self.scene_changed
            || self.body_changed
            || self.knowledge_revealed
    }

    /// Check if character was directly addressed in user input
    fn check_directly_addressed(
        character_id: &str,
        user_input_delta: &Option<UserInputDelta>,
    ) -> bool {
        match user_input_delta {
            Some(delta) => {
                // Check if character is mentioned in roleplay or dialogue
                match &delta.kind {
                    UserInputKind::CharacterRoleplay {
                        character_id: cid,
                        intent_plan,
                        ..
                    } => {
                        // Check if this is the character roleplaying
                        cid == character_id
                            // Or if this character is a target of the intent
                            || intent_plan.target_refs.iter().any(|t| t == character_id)
                    }
                    UserInputKind::SceneNarration { scene_delta } => {
                        // Check if character appears in entity deltas
                        scene_delta
                            .entity_deltas
                            .iter()
                            .any(|ed| ed.entity_id == character_id)
                    }
                    _ => false,
                }
            }
            None => false,
        }
    }

    /// Check if character is under threat from events
    fn check_under_threat(character_id: &str, event_delta: &[ObservableEventDelta]) -> bool {
        event_delta.iter().any(|event| {
            // Check if event involves this character as target of threat
            event
                .involved_observable_entities
                .contains(&character_id.to_string())
                && (event.event_kind.contains("threat")
                    || event.event_kind.contains("attack")
                    || event.event_kind.contains("danger"))
        })
    }

    /// Check if scene observable state changed for this character
    fn check_scene_changed(
        character_id: &str,
        scene: &SceneModel,
        event_delta: &[ObservableEventDelta],
    ) -> bool {
        // Check if events affect entities near this character
        let character_position = scene
            .entities
            .iter()
            .find(|e| e.entity_id == character_id)
            .map(|e| &e.position);

        match character_position {
            Some(pos) => {
                // Check if any event involves entities within perception range
                event_delta.iter().any(|event| {
                    event.involved_observable_entities.iter().any(|entity_id| {
                        // Check distance to involved entities
                        scene
                            .entities
                            .iter()
                            .find(|e| &e.entity_id == entity_id)
                            .map(|e| Self::within_perception_range(pos, &e.position))
                            .unwrap_or(false)
                    })
                })
            }
            None => false,
        }
    }

    /// Check if body state changed for this character
    fn check_body_changed(character_id: &str, event_delta: &[ObservableEventDelta]) -> bool {
        event_delta.iter().any(|event| {
            event
                .involved_observable_entities
                .contains(&character_id.to_string())
                && (event.event_kind.contains("injury")
                    || event.event_kind.contains("fatigue")
                    || event.event_kind.contains("condition")
                    || event.event_kind.contains("state_change"))
        })
    }

    /// Check if new knowledge was revealed to this character
    fn check_knowledge_revealed(
        character_id: &str,
        prior: &HashSet<String>,
        new: &HashSet<String>,
    ) -> bool {
        // Check if any new knowledge entries are accessible to this character
        // TODO: Actually check access policy
        let _ = character_id;
        new.iter().any(|id| !prior.contains(id))
    }

    /// Check if two positions are within perception range
    fn within_perception_range(a: &Position, b: &Position) -> bool {
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        let dz = match (a.z, b.z) {
            (Some(az), Some(bz)) => az - bz,
            _ => 0.0,
        };
        // Default perception range ~10 meters
        (dx * dx + dy * dy + dz * dz) < 100.0
    }

    /// Compute reason string for cognitive pass decision
    fn compute_reason(&self) -> String {
        if self.needs_cognitive_pass {
            let triggers: Vec<&str> = [
                ("directly_addressed", self.directly_addressed),
                ("under_threat", self.under_threat),
                ("reaction_window_open", self.reaction_window_open),
                ("scene_changed", self.scene_changed),
                ("body_changed", self.body_changed),
                ("knowledge_revealed", self.knowledge_revealed),
            ]
            .iter()
            .filter(|(_, v)| *v)
            .map(|(k, _)| *k)
            .collect();
            format!("Triggered by: {}", triggers.join(", "))
        } else {
            "No hard trigger conditions met".to_string()
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Compute trigger priority for sorting
fn trigger_priority(flags: &DirtyFlags) -> u8 {
    if flags.under_threat {
        4
    } else if flags.directly_addressed {
        3
    } else if flags.reaction_window_open {
        2
    } else {
        1
    }
}

fn build_session_context(session: &AgentSession) -> AgentSessionContext {
    AgentSessionContext {
        session_id: session.session_id.clone(),
        session_kind: format!("{:?}", session.session_kind),
        period_anchor: session.period_anchor.clone(),
        mainline_time_anchor: session.period_anchor.clone(),
        player_character_id: session.player_character_id.clone(),
        canon_status: format!("{:?}", session.canon_status),
    }
}

fn default_style_constraints() -> StyleConstraints {
    StyleConstraints {
        register: StyleRegister::Formal,
        detail_level: DetailLevel::Moderate,
        atmosphere: Atmosphere::Tense,
        pacing: Pacing::Measured,
        pov: PointOfView::Objective,
        explicit_guidelines: Vec::new(),
        reference_excerpts: Vec::new(),
    }
}

fn default_scene_model(session: &AgentSession) -> SceneModel {
    let scene_turn_id = format!("turn_{}", uuid::Uuid::new_v4());
    let scene_id = format!("scene_{}", session.session_id);
    let mut entities = Vec::new();
    if let Some(character_id) = &session.player_character_id {
        entities.push(SceneEntity {
            entity_id: character_id.clone(),
            entity_kind: SceneEntityKind::Character,
            position: Position {
                x: 0.0,
                y: 0.0,
                z: None,
            },
            posture: "present".to_string(),
            display_name: character_id.clone(),
            observable_facets: Vec::new(),
        });
    }

    SceneModel {
        scene_id,
        scene_turn_id,
        time_context: TimeContext {
            time_anchor: session.period_anchor.clone(),
            season: "unknown".to_string(),
            day_phase: DayPhase::Day,
            weather_trend: "stable".to_string(),
        },
        spatial_layout: SpatialLayout {
            layout_type: "unspecified".to_string(),
            dimensions: None,
            obstacles: Vec::new(),
            entrances: Vec::new(),
            zones: Vec::new(),
        },
        lighting: LightingState {
            ambient_level: 0.6,
            light_sources: Vec::new(),
            shadow_areas: Vec::new(),
            backlight: None,
        },
        acoustics: AcousticsState {
            ambient_noise_level: 0.2,
            echo_characteristics: "neutral".to_string(),
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
                ambient_celsius: 20.0,
                felt_celsius: 20.0,
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
                visibility_range_m: 100.0,
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
        entities,
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
        uncertainty_notes: vec![
            "Scene initialized by deterministic runtime fallback; SceneInitializer LLM profile is not connected yet.".to_string(),
        ],
    }
}

fn populate_scene_entities_from_characters(
    scene: &mut SceneModel,
    characters: &[CharacterRecord],
    player_character_id: Option<&str>,
) {
    let mut existing: HashSet<String> = scene
        .entities
        .iter()
        .map(|entity| entity.entity_id.clone())
        .collect();

    for (index, character) in characters.iter().enumerate() {
        if existing.contains(&character.character_id) {
            continue;
        }

        let is_player = player_character_id == Some(character.character_id.as_str());
        scene.entities.push(SceneEntity {
            entity_id: character.character_id.clone(),
            entity_kind: SceneEntityKind::Character,
            position: Position {
                x: index as f64 * 1.5,
                y: if is_player { 0.0 } else { 2.0 },
                z: None,
            },
            posture: "present".to_string(),
            display_name: character.character_id.clone(),
            observable_facets: default_character_observable_facets(character),
        });
        existing.insert(character.character_id.clone());
    }
}

fn default_character_observable_facets(character: &CharacterRecord) -> Vec<String> {
    let mut facets = vec![format!(
        "species: {}",
        character.baseline_body_profile.species
    )];
    if character.temporary_state.fatigue > 0.5 {
        facets.push("fatigued".to_string());
    }
    if character.temporary_state.pain_load > 0.4 {
        facets.push("in pain".to_string());
    }
    facets.extend(character.temporary_state.transient_signals.iter().cloned());
    facets
}

fn deterministic_user_input_delta(
    scene: &SceneModel,
    session: &AgentSession,
    raw_text: String,
) -> SceneStateExtractorOutput {
    let kind = match (&session.player_mode, session.player_character_id.as_ref()) {
        (PlayerMode::Character, Some(character_id)) => {
            let action_id = format!("act_{}", uuid::Uuid::new_v4());
            let intent_plan = IntentPlan {
                character_id: character_id.clone(),
                intent_kind: "player_input".to_string(),
                target_refs: Vec::new(),
                intended_actions: vec![CharacterAction {
                    action_id,
                    action_kind: "roleplay_input".to_string(),
                    target_refs: Vec::new(),
                    spoken_text: Some(raw_text.clone()),
                    skill_id: None,
                    requested_mana_expression: None,
                    declared_effect_refs: Vec::new(),
                    outward_description: raw_text.clone(),
                }],
                priority: "normal".to_string(),
                commitment: "declared_by_player".to_string(),
                rationale: "Parsed by deterministic runtime fallback.".to_string(),
            };
            UserInputKind::CharacterRoleplay {
                character_id: character_id.clone(),
                intent_plan,
                spoken_dialogue: Some(raw_text.clone()),
                actions: Vec::new(),
                subjective_input: None,
            }
        }
        _ => UserInputKind::DirectorHint {
            outcome_bias: Some(OutcomeBias {
                preferred_tone: None,
                outcome_pressure: None,
                protected_entities: Vec::new(),
                forbidden_outcomes: Vec::new(),
                notes: vec![raw_text.clone()],
            }),
            style_override: None,
        },
    };

    SceneStateExtractorOutput {
        scene_update: None,
        user_input_delta: UserInputDelta {
            turn_id: scene.scene_turn_id.clone(),
            raw_text,
            authority_class: match session.player_mode {
                PlayerMode::Character => UserInputAuthorityClass::PlayerCharacterIntent,
                PlayerMode::Director => UserInputAuthorityClass::DirectorBias,
            },
            authority_notes: Vec::new(),
            kind,
        },
        provisional_truth_candidates: Vec::new(),
        conflict_warnings: Vec::new(),
        ambiguity_report: Vec::new(),
    }
}

fn deterministic_cognitive_output(
    character_id: &str,
    working_state: &TurnWorkingState,
) -> CharacterCognitivePassOutput {
    let raw_text = working_state
        .user_input_delta
        .as_ref()
        .map(|delta| delta.raw_text.clone())
        .unwrap_or_else(|| "The scene changes.".to_string());
    let target_refs = working_state
        .event_delta
        .iter()
        .flat_map(|event| event.involved_observable_entities.iter().cloned())
        .filter(|entity_id| entity_id != character_id)
        .take(3)
        .collect::<Vec<_>>();
    let action = CharacterAction {
        action_id: format!("action_{}", uuid::Uuid::new_v4()),
        action_kind: "observe".to_string(),
        target_refs: target_refs.clone(),
        spoken_text: None,
        skill_id: None,
        requested_mana_expression: None,
        declared_effect_refs: Vec::new(),
        outward_description: format!("{character_id} observes the change and holds position."),
    };

    CharacterCognitivePassOutput {
        perception_delta: PerceptionDelta {
            new_observations: vec![raw_text],
            updated_perceptions: Vec::new(),
            missed_observations: Vec::new(),
        },
        belief_update: BeliefUpdate {
            stable_beliefs_reinforced: Vec::new(),
            stable_beliefs_weakened: Vec::new(),
            new_hypotheses: Vec::new(),
            revised_models_of_others: Vec::new(),
            contradictions_and_tension: Vec::new(),
            emotional_shift: EmotionalShiftDelta {
                primary_emotion: PrimaryEmotion::Anticipation,
                intensity_change: 0.05,
                secondary_changes: Vec::new(),
            },
            decision_relevant_beliefs: Vec::new(),
        },
        intent_plan: IntentPlan {
            character_id: character_id.to_string(),
            intent_kind: "observe".to_string(),
            target_refs,
            intended_actions: vec![action],
            priority: "normal".to_string(),
            commitment: "tentative".to_string(),
            rationale: "Deterministic fallback used because cognitive LLM output was unavailable."
                .to_string(),
        },
        body_reaction_delta: None,
    }
}

fn deterministic_outcome_plan(
    working_state: &TurnWorkingState,
    input: OutcomePlannerInput,
) -> OutcomePlannerOutput {
    let mut outward_actions = Vec::new();
    let mut narratable_facts = Vec::new();

    let mut intent_refs: Vec<&IntentPlan> = input.user_roleplay_intents.iter().collect();
    intent_refs.extend(
        input
            .character_outputs
            .iter()
            .map(|output| &output.intent_plan),
    );
    intent_refs.extend(working_state.character_intents.values());
    intent_refs.sort_by(|a, b| {
        a.character_id
            .cmp(&b.character_id)
            .then(a.intent_kind.cmp(&b.intent_kind))
    });
    intent_refs.dedup_by(|a, b| a.character_id == b.character_id && a.intent_kind == b.intent_kind);

    for intent in intent_refs {
        let fact_id = format!("fact_{}", uuid::Uuid::new_v4());
        let action_id = format!("out_{}", uuid::Uuid::new_v4());
        let claim = intent
            .intended_actions
            .first()
            .map(|action| action.outward_description.clone())
            .filter(|text| !text.trim().is_empty())
            .unwrap_or_else(|| intent.intent_kind.clone());

        outward_actions.push(OutwardAction {
            action_id,
            actor_id: intent.character_id.clone(),
            action_kind: intent.intent_kind.clone(),
            target_refs: intent.target_refs.clone(),
            narratable_fact_refs: vec![fact_id.clone()],
            status: "accepted_as_intent".to_string(),
        });
        narratable_facts.push(NarratableFact {
            fact_id,
            fact_kind: "player_intent".to_string(),
            subject_refs: vec![intent.character_id.clone()],
            source_refs: Vec::new(),
            allowed_claim: claim,
            narration_scope: NarrationScope::ObjectiveCamera,
        });
    }

    if narratable_facts.is_empty() {
        let fact_id = format!("fact_{}", uuid::Uuid::new_v4());
        let raw_text = working_state
            .user_input_delta
            .as_ref()
            .map(|delta| delta.raw_text.clone())
            .unwrap_or_else(|| "The turn advances without a concrete action.".to_string());
        narratable_facts.push(NarratableFact {
            fact_id,
            fact_kind: "scene_note".to_string(),
            subject_refs: Vec::new(),
            source_refs: Vec::new(),
            allowed_claim: raw_text,
            narration_scope: NarrationScope::ObjectiveCamera,
        });
    }

    OutcomePlannerOutput {
        outcome_plan: OutcomePlan {
            outward_actions,
            resulting_state_changes: serde_json::json!({}),
            narratable_facts,
            soft_effects: Vec::new(),
            blocked_effects: Vec::new(),
        },
        state_update_plan: StateUpdatePlan {
            scene_delta: None,
            character_state_deltas: Vec::new(),
            subjective_update_refs: Vec::new(),
            new_memory_entries: Vec::new(),
            soft_effects: Vec::new(),
            blocked_effects: Vec::new(),
            validation_warnings: vec![
                "Outcome planned by deterministic runtime fallback; OutcomePlanner LLM profile is not connected yet.".to_string(),
            ],
            consistency_notes: Vec::new(),
        },
        knowledge_reveal_events: Vec::new(),
        conflict_reports: Vec::new(),
    }
}

fn deterministic_surface_output(input: &SurfaceRealizerInput) -> SurfaceRealizerOutput {
    let narrative_text = input
        .outcome_plan
        .narratable_facts
        .iter()
        .map(|fact| fact.allowed_claim.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    SurfaceRealizerOutput {
        narrative_text: if narrative_text.is_empty() {
            "这一回合没有产生可叙述的外显变化。".to_string()
        } else {
            narrative_text
        },
        used_fact_ids: input
            .outcome_plan
            .narratable_facts
            .iter()
            .map(|fact| fact.fact_id.clone())
            .collect(),
    }
}

fn trace_kind_to_str(kind: super::trace::TraceKind) -> &'static str {
    match kind {
        super::trace::TraceKind::Turn => "turn",
        super::trace::TraceKind::Character => "character",
        super::trace::TraceKind::Presentation => "presentation",
        super::trace::TraceKind::Rollback => "rollback",
    }
}

fn step_status_to_str(status: StepStatus) -> &'static str {
    match status {
        StepStatus::Started => "started",
        StepStatus::Skipped => "skipped",
        StepStatus::Succeeded => "succeeded",
        StepStatus::Failed => "failed",
        StepStatus::FallbackUsed => "fallback_used",
    }
}

fn user_message_to_text(user_message: &serde_json::Value) -> Result<String, String> {
    if let Some(text) = user_message.as_str() {
        return Ok(text.to_string());
    }
    if let Some(text) = user_message.get("text").and_then(serde_json::Value::as_str) {
        return Ok(text.to_string());
    }

    serde_json::to_string(user_message)
        .map_err(|error| format!("Failed to serialize user_message as recent free text: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::knowledge::KnowledgeStore;
    use chrono::Utc;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn process_turn_persists_commit_and_trace() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite pool");
        let store = AgentStore::new(pool.clone(), "world_runtime_test".to_string())
            .await
            .expect("agent store");
        let session = AgentSession::new_with_mode(
            "world_runtime_test".to_string(),
            "Runtime smoke test".to_string(),
            AgentSessionKind::Mainline,
            TimeAnchor {
                calendar_id: "default".to_string(),
                ordinal: 1,
                precision: TimePrecision::Exact,
                display_text: "第一回合".to_string(),
            },
            PlayerMode::Director,
            None,
        )
        .expect("director session");
        store
            .create_session(&session)
            .await
            .expect("create session");

        let mut runtime = AgentRuntime::new(Arc::new(RwLock::new(store)));
        let result = runtime
            .process_turn(
                &session.session_id,
                serde_json::json!({ "text": "风吹过庭院" }),
            )
            .await
            .expect("process turn");

        assert!(result.narrative_text.contains("风吹过庭院"));

        let second_result = runtime
            .process_turn(
                &session.session_id,
                serde_json::json!({ "text": "灯影轻轻摇晃" }),
            )
            .await
            .expect("process second turn");
        assert!(second_result.narrative_text.contains("灯影轻轻摇晃"));

        let world_turn_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM world_turns")
            .fetch_one(&pool)
            .await
            .expect("world turn count");
        let commit_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM state_commit_records")
            .fetch_one(&pool)
            .await
            .expect("commit count");
        let turn_trace_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM turn_traces")
            .fetch_one(&pool)
            .await
            .expect("turn trace count");
        let step_trace_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_step_traces")
            .fetch_one(&pool)
            .await
            .expect("step trace count");
        let scene_snapshot_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM scene_snapshots")
            .fetch_one(&pool)
            .await
            .expect("scene snapshot count");
        let persisted_user_message: String =
            sqlx::query_scalar("SELECT user_message FROM world_turns WHERE scene_turn_id = ?")
                .bind(&result.scene_turn_id)
                .fetch_one(&pool)
                .await
                .expect("persisted user message");
        let trace_ids: String = sqlx::query_scalar(
            "SELECT trace_ids FROM state_commit_records WHERE scene_turn_id = ?",
        )
        .bind(&result.scene_turn_id)
        .fetch_one(&pool)
        .await
        .expect("commit trace ids");
        let second_parent_turn_id: Option<String> =
            sqlx::query_scalar("SELECT parent_turn_id FROM world_turns WHERE scene_turn_id = ?")
                .bind(&second_result.scene_turn_id)
                .fetch_one(&pool)
                .await
                .expect("second parent turn id");
        let first_scene_id: String =
            sqlx::query_scalar("SELECT scene_id FROM scene_snapshots WHERE scene_turn_id = ?")
                .bind(&result.scene_turn_id)
                .fetch_one(&pool)
                .await
                .expect("first scene id");
        let second_scene_id: String =
            sqlx::query_scalar("SELECT scene_id FROM scene_snapshots WHERE scene_turn_id = ?")
                .bind(&second_result.scene_turn_id)
                .fetch_one(&pool)
                .await
                .expect("second scene id");

        assert_eq!(world_turn_count, 2);
        assert_eq!(commit_count, 2);
        assert_eq!(turn_trace_count, 2);
        assert_eq!(scene_snapshot_count, 2);
        assert!(step_trace_count > 0);
        assert!(persisted_user_message.contains("风吹过庭院"));
        assert_ne!(trace_ids, "[]");
        assert_eq!(
            second_parent_turn_id.as_deref(),
            Some(result.scene_turn_id.as_str())
        );
        assert_eq!(first_scene_id, second_scene_id);
    }

    fn character(id: &str) -> CharacterRecord {
        CharacterRecord {
            character_id: id.to_string(),
            base_attributes: BaseAttributes {
                physical: 120.0,
                agility: 100.0,
                endurance: 100.0,
                insight: 100.0,
                mana_power: 180.0,
                soul_strength: 100.0,
            },
            baseline_body_profile: BaselineBodyProfile {
                species: "human".to_string(),
                comfort_temperature_range: (18.0, 26.0),
                mana_sense_baseline: ManaSenseBaseline {
                    acuity: 0.3,
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

    #[tokio::test]
    async fn load_runtime_skills_reads_character_ability_facets() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite pool");
        let store = AgentStore::new(pool.clone(), "world_runtime_skills".to_string())
            .await
            .expect("agent store");
        let runtime = AgentRuntime::new(Arc::new(RwLock::new(store)));

        let actor = character("actor");
        {
            let store = runtime.store.read().await;
            store.save_character(&actor).await.expect("save character");
        }

        let knowledge_store = KnowledgeStore::new(pool.clone());
        let ability_entry = KnowledgeEntry {
            knowledge_id: "know-skill-1".to_string(),
            kind: KnowledgeKind::CharacterFacet,
            subject: KnowledgeSubject::Character {
                id: "actor".to_string(),
                facet: CharacterFacetType::KnownAbility,
            },
            content: serde_json::json!({
                "summary_text": "Void Counter",
                "ability_id": "void_counter",
                "category": "combat",
                "trigger_condition": "reaction",
                "power_level": null,
                "extensions": {
                    "skill": {
                        "skill_kind": "Reaction",
                        "activation": {
                            "activation_time": "Reaction",
                            "trigger_conditions": ["TargetInLineOfSight"],
                            "cooldown": 1,
                            "uses_per_scene": null,
                            "uses_per_day": null
                        },
                        "effect_contract": {
                            "primary_effects": [],
                            "secondary_effects": [],
                            "target_kind": "Character",
                            "target_count": "Single",
                            "range_m": 10.0,
                            "area_of_effect": null,
                            "duration_turns": null,
                            "attribute_modifier": null,
                            "mana_attribute": null,
                            "allowed_target_kinds": ["Character"],
                            "allowed_state_domains": ["body"],
                            "max_intensity_tier": "Moderate",
                            "allows_injury": true,
                            "allows_position_change": false,
                            "allows_knowledge_reveal": false
                        }
                    }
                }
            }),
            apparent_content: None,
            access_policy: AccessPolicy {
                known_by: vec!["actor".to_string()],
                scope: Vec::new(),
                conditions: Vec::new(),
            },
            subject_awareness: SubjectAwareness::Aware,
            metadata: KnowledgeMetadata {
                created_at: Utc::now(),
                updated_at: Utc::now(),
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
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        knowledge_store
            .create(&ability_entry)
            .await
            .expect("create ability entry");

        let skills = runtime
            .load_runtime_skills(&[actor])
            .await
            .expect("load runtime skills");

        assert!(skills
            .iter()
            .any(|skill| skill.skill_id == "void_counter" && skill.belongs_to_character("actor")));
    }
}
