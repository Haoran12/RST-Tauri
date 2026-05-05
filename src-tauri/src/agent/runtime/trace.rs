//! Agent Trace recording
//!
//! Implements the trace recording points from docs/11_agent_runtime.md §6.3.
//!
//! Agent Trace is written to world.sqlite and records:
//! - Turn-level trace (turn_traces)
//! - Step-level trace (agent_step_traces)
//!
//! Key principles:
//! - Trace is for debugging and replay, NOT for business logic
//! - Trace records program decisions and LLM node outputs
//! - Trace IDs link to LLM call logs and event logs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::agent::models::generate_id;

/// Turn trace - top-level trace for a single turn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnTrace {
    /// Unique trace ID
    pub trace_id: String,
    /// Scene turn ID this trace belongs to
    pub scene_turn_id: String,
    /// Session ID (if applicable)
    pub session_id: Option<String>,
    /// Story time anchor
    pub story_time_anchor: Option<String>,
    /// Runtime turn status
    pub runtime_turn_status: String,
    /// Kind of trace
    pub trace_kind: TraceKind,
    /// Character ID (for character-specific traces)
    pub character_id: Option<String>,
    /// Runtime config snapshot ID used
    pub runtime_config_snapshot_id: String,
    /// World rules snapshot ID used
    pub world_rules_snapshot_id: Option<String>,
    /// Summary of key outputs
    pub summary: TraceSummary,
    /// Linked LLM request IDs
    pub linked_request_ids: Vec<String>,
    /// Linked event IDs
    pub linked_event_ids: Vec<String>,
    /// When this trace was created
    pub created_at: DateTime<Utc>,
}

/// Step trace - individual step within a turn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepTrace {
    /// Unique step trace ID
    pub step_trace_id: String,
    /// Parent turn trace ID
    pub trace_id: String,
    /// Scene turn ID
    pub scene_turn_id: String,
    /// Character ID (for character-specific steps)
    pub character_id: Option<String>,
    /// Step name
    pub step_name: StepName,
    /// Step status
    pub step_status: StepStatus,
    /// Input summary (JSON)
    pub input_summary: Option<String>,
    /// Output summary (JSON)
    pub output_summary: Option<String>,
    /// Decision details (JSON)
    pub decision_json: Option<String>,
    /// Linked LLM request ID
    pub linked_request_id: Option<String>,
    /// Linked error event ID
    pub error_event_id: Option<String>,
    /// When this step was recorded
    pub created_at: DateTime<Utc>,
}

/// Trace kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceKind {
    /// Turn-level trace
    Turn,
    /// Character-specific trace
    Character,
    /// Presentation trace
    Presentation,
    /// Rollback trace
    Rollback,
}

/// Step name for trace recording
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepName {
    /// Active set calculation
    ActiveSet,
    /// Dirty flags calculation
    DirtyFlags,
    /// Scene filter
    SceneFilter,
    /// Embodiment resolver
    EmbodimentResolver,
    /// Knowledge access
    KnowledgeAccess,
    /// Input assembly
    InputAssembly,
    /// Cognitive pass
    CognitivePass,
    /// Validation
    Validation,
    /// Outcome planning
    OutcomePlanning,
    /// Effect validation
    EffectValidation,
    /// Temporal validation
    TemporalValidation,
    /// Surface realization
    SurfaceRealization,
    /// Narrative fact check
    NarrativeFactCheck,
    /// State commit
    StateCommit,
    /// Reaction window
    ReactionWindow,
    /// Reaction pass
    ReactionPass,
    /// Scene initializer
    SceneInitializer,
    /// Scene state extractor
    SceneStateExtractor,
    /// Attribute resolver
    AttributeResolver,
    /// Mechanical evolution
    MechanicalEvolution,
}

/// Step status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    /// Step started
    Started,
    /// Step was skipped
    Skipped,
    /// Step succeeded
    Succeeded,
    /// Step failed
    Failed,
    /// Fallback was used
    FallbackUsed,
}

/// Trace summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSummary {
    /// Brief description
    pub description: String,
    /// Key outputs indexed by step name
    pub key_outputs: HashMap<String, String>,
    /// Characters processed
    pub characters_processed: Vec<String>,
    /// Characters deferred
    pub characters_deferred: Vec<String>,
    /// Warnings encountered
    pub warnings: Vec<String>,
    /// Errors encountered
    pub errors: Vec<String>,
}

impl Default for TraceSummary {
    fn default() -> Self {
        Self {
            description: String::new(),
            key_outputs: HashMap::new(),
            characters_processed: Vec::new(),
            characters_deferred: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }
}

/// Trace recorder for collecting trace entries during a turn
#[derive(Debug, Clone, Default)]
pub struct TraceRecorder {
    /// Current turn trace
    turn_trace: Option<TurnTrace>,
    /// Step traces for this turn
    step_traces: Vec<StepTrace>,
    /// Linked request IDs
    request_ids: Vec<String>,
    /// Linked event IDs
    event_ids: Vec<String>,
}

impl TraceRecorder {
    /// Create a new trace recorder
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new turn trace
    pub fn start_turn(
        &mut self,
        scene_turn_id: &str,
        session_id: Option<&str>,
        story_time_anchor: Option<&str>,
        runtime_turn_status: &str,
        runtime_config_snapshot_id: &str,
        world_rules_snapshot_id: Option<&str>,
    ) -> String {
        let trace_id = generate_id("trace");

        self.turn_trace = Some(TurnTrace {
            trace_id: trace_id.clone(),
            scene_turn_id: scene_turn_id.to_string(),
            session_id: session_id.map(|s| s.to_string()),
            story_time_anchor: story_time_anchor.map(|s| s.to_string()),
            runtime_turn_status: runtime_turn_status.to_string(),
            trace_kind: TraceKind::Turn,
            character_id: None,
            runtime_config_snapshot_id: runtime_config_snapshot_id.to_string(),
            world_rules_snapshot_id: world_rules_snapshot_id.map(|s| s.to_string()),
            summary: TraceSummary::default(),
            linked_request_ids: Vec::new(),
            linked_event_ids: Vec::new(),
            created_at: Utc::now(),
        });

        self.step_traces.clear();
        self.request_ids.clear();
        self.event_ids.clear();

        trace_id
    }

    /// Record a step
    pub fn record_step(
        &mut self,
        step_name: StepName,
        step_status: StepStatus,
        character_id: Option<&str>,
        input_summary: Option<String>,
        output_summary: Option<String>,
        decision_json: Option<String>,
    ) -> String {
        let step_trace_id = generate_id("step_trace");
        let trace_id = self
            .turn_trace
            .as_ref()
            .map(|t| t.trace_id.clone())
            .unwrap_or_default();
        let scene_turn_id = self
            .turn_trace
            .as_ref()
            .map(|t| t.scene_turn_id.clone())
            .unwrap_or_default();

        self.step_traces.push(StepTrace {
            step_trace_id: step_trace_id.clone(),
            trace_id,
            scene_turn_id,
            character_id: character_id.map(|s| s.to_string()),
            step_name,
            step_status,
            input_summary,
            output_summary,
            decision_json,
            linked_request_id: None,
            error_event_id: None,
            created_at: Utc::now(),
        });

        step_trace_id
    }

    /// Record a step with linked request ID
    pub fn record_step_with_request(
        &mut self,
        step_name: StepName,
        step_status: StepStatus,
        character_id: Option<&str>,
        request_id: &str,
        input_summary: Option<String>,
        output_summary: Option<String>,
    ) -> String {
        let step_trace_id = self.record_step(
            step_name,
            step_status,
            character_id,
            input_summary,
            output_summary,
            None,
        );

        // Link request ID
        self.request_ids.push(request_id.to_string());
        if let Some(step) = self
            .step_traces
            .iter_mut()
            .rev()
            .find(|s| s.step_trace_id == step_trace_id)
        {
            step.linked_request_id = Some(request_id.to_string());
        }

        step_trace_id
    }

    /// Record a step with error
    pub fn record_step_with_error(
        &mut self,
        step_name: StepName,
        step_status: StepStatus,
        character_id: Option<&str>,
        error_event_id: &str,
        error_summary: String,
    ) -> String {
        let step_trace_id = self.record_step(
            step_name,
            step_status,
            character_id,
            None,
            None,
            Some(serde_json::json!({ "error": error_summary }).to_string()),
        );

        // Link error event ID
        self.event_ids.push(error_event_id.to_string());
        if let Some(step) = self
            .step_traces
            .iter_mut()
            .rev()
            .find(|s| s.step_trace_id == step_trace_id)
        {
            step.error_event_id = Some(error_event_id.to_string());
        }

        step_trace_id
    }

    /// Add a key output to the summary
    pub fn add_key_output(&mut self, step_name: &str, output: String) {
        if let Some(ref mut trace) = self.turn_trace {
            trace
                .summary
                .key_outputs
                .insert(step_name.to_string(), output);
        }
    }

    /// Add a processed character
    pub fn add_processed_character(&mut self, character_id: &str) {
        if let Some(ref mut trace) = self.turn_trace {
            trace
                .summary
                .characters_processed
                .push(character_id.to_string());
        }
    }

    /// Add a deferred character
    pub fn add_deferred_character(&mut self, character_id: &str) {
        if let Some(ref mut trace) = self.turn_trace {
            trace
                .summary
                .characters_deferred
                .push(character_id.to_string());
        }
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: String) {
        if let Some(ref mut trace) = self.turn_trace {
            trace.summary.warnings.push(warning);
        }
    }

    /// Add an error
    pub fn add_error(&mut self, error: String) {
        if let Some(ref mut trace) = self.turn_trace {
            trace.summary.errors.push(error);
        }
    }

    /// Update the scene turn ID after scene initialization.
    pub fn set_scene_turn_id(&mut self, scene_turn_id: &str) {
        if let Some(ref mut trace) = self.turn_trace {
            trace.scene_turn_id = scene_turn_id.to_string();
        }
        for step in &mut self.step_traces {
            step.scene_turn_id = scene_turn_id.to_string();
        }
    }

    /// Set the trace description
    pub fn set_description(&mut self, description: &str) {
        if let Some(ref mut trace) = self.turn_trace {
            trace.summary.description = description.to_string();
        }
    }

    /// Finalize the turn trace
    pub fn finalize(&mut self) -> Option<(TurnTrace, Vec<StepTrace>)> {
        if let Some(ref mut trace) = self.turn_trace {
            trace.linked_request_ids = self.request_ids.clone();
            trace.linked_event_ids = self.event_ids.clone();
        }

        let turn = self.turn_trace.take();
        let steps = self.step_traces.clone();

        turn.map(|t| (t, steps))
    }

    /// Get the current trace ID
    pub fn current_trace_id(&self) -> Option<&str> {
        self.turn_trace.as_ref().map(|t| t.trace_id.as_str())
    }

    /// Create a character-specific trace
    pub fn create_character_trace(&mut self, character_id: &str) -> Option<String> {
        let turn_trace = self.turn_trace.as_ref()?;
        let trace_id = generate_id("trace_char");

        // Note: We don't add this to step_traces as it's a separate turn-level trace
        // In production, this would be stored separately

        Some(trace_id)
    }
}

impl StepName {
    /// Convert to string for serialization
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ActiveSet => "active_set",
            Self::DirtyFlags => "dirty_flags",
            Self::SceneFilter => "scene_filter",
            Self::EmbodimentResolver => "embodiment_resolver",
            Self::KnowledgeAccess => "knowledge_access",
            Self::InputAssembly => "input_assembly",
            Self::CognitivePass => "cognitive_pass",
            Self::Validation => "validation",
            Self::OutcomePlanning => "outcome_planning",
            Self::EffectValidation => "effect_validation",
            Self::TemporalValidation => "temporal_validation",
            Self::SurfaceRealization => "surface_realization",
            Self::NarrativeFactCheck => "narrative_fact_check",
            Self::StateCommit => "state_commit",
            Self::ReactionWindow => "reaction_window",
            Self::ReactionPass => "reaction_pass",
            Self::SceneInitializer => "scene_initializer",
            Self::SceneStateExtractor => "scene_state_extractor",
            Self::AttributeResolver => "attribute_resolver",
            Self::MechanicalEvolution => "mechanical_evolution",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_turn_trace() {
        let mut recorder = TraceRecorder::new();
        let trace_id = recorder.start_turn(
            "turn_1",
            Some("session_1"),
            Some("time_anchor_1"),
            "canon",
            "runtime_snapshot_1",
            Some("world_snapshot_1"),
        );

        assert!(!trace_id.is_empty());
        assert!(recorder.current_trace_id().is_some());
    }

    #[test]
    fn records_step_trace() {
        let mut recorder = TraceRecorder::new();
        recorder.start_turn("turn_1", None, None, "canon", "runtime_snapshot_1", None);

        let step_id = recorder.record_step(
            StepName::ActiveSet,
            StepStatus::Succeeded,
            None,
            Some(serde_json::json!({ "count": 3 }).to_string()),
            Some(serde_json::json!({ "active": ["char_1", "char_2"] }).to_string()),
            None,
        );

        assert!(!step_id.is_empty());
        assert_eq!(recorder.step_traces.len(), 1);
    }

    #[test]
    fn finalizes_trace() {
        let mut recorder = TraceRecorder::new();
        recorder.start_turn(
            "turn_1",
            Some("session_1"),
            None,
            "canon",
            "runtime_snapshot_1",
            None,
        );

        recorder.add_processed_character("char_1");
        recorder.add_warning("Budget limit reached".to_string());

        let result = recorder.finalize();
        assert!(result.is_some());

        let (turn, _steps) = result.unwrap();
        assert_eq!(turn.summary.characters_processed.len(), 1);
        assert_eq!(turn.summary.warnings.len(), 1);
    }
}
