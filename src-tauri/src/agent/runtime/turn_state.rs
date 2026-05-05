//! Turn working state
//!
//! In-memory working copy for a single turn.
//! Contains fixed configuration snapshots captured at turn start.

use std::collections::HashMap;

use crate::agent::models::*;

/// Turn working state - in-memory working copy for a single turn
///
/// This structure holds all the state for a single turn, including:
/// - Fixed configuration snapshots (captured at turn start)
/// - Scene model (may be modified during turn)
/// - Character states and intents
/// - Reaction windows and provisional truths
pub struct TurnWorkingState {
    /// Runtime config snapshot ID (fixed for this turn)
    pub runtime_config_snapshot_id: String,
    /// World rules snapshot ID (fixed for this turn)
    pub world_rules_snapshot_id: Option<String>,
    /// Current scene model (may be modified during turn)
    pub scene: SceneModel,
    /// Raw user message for persistence and trace replay.
    pub raw_user_message: serde_json::Value,
    /// User input delta
    pub user_input_delta: Option<UserInputDelta>,
    /// Director hint from user
    pub director_hint: Option<OutcomeBias>,
    /// Event delta this turn
    pub event_delta: Vec<ObservableEventDelta>,
    /// Character temporary states
    pub character_states: HashMap<String, TemporaryCharacterState>,
    /// Character subjective states (Layer 3)
    pub subjective_states: HashMap<String, CharacterSubjectiveState>,
    /// Accessible knowledge per character (Layer 2)
    pub accessible_knowledge: HashMap<String, AccessibleKnowledge>,
    /// Reaction windows opened this turn
    pub pending_reactions: Vec<ReactionWindow>,
    /// Provisional truth candidates (for past timeline)
    pub provisional_truths: Vec<ProvisionalTruthCandidate>,
    /// Character intent plans (from CognitivePass or user roleplay)
    pub character_intents: HashMap<String, IntentPlan>,
    /// Conflict warnings
    pub conflict_warnings: Vec<ConflictWarning>,
}

impl TurnWorkingState {
    /// Create a new working state from a scene model with config snapshots
    pub fn new_with_snapshots(
        scene: SceneModel,
        runtime_config_snapshot_id: String,
        world_rules_snapshot_id: Option<String>,
    ) -> Self {
        Self {
            runtime_config_snapshot_id,
            world_rules_snapshot_id,
            scene,
            raw_user_message: serde_json::Value::Null,
            user_input_delta: None,
            director_hint: None,
            event_delta: Vec::new(),
            character_states: HashMap::new(),
            subjective_states: HashMap::new(),
            accessible_knowledge: HashMap::new(),
            pending_reactions: Vec::new(),
            provisional_truths: Vec::new(),
            character_intents: HashMap::new(),
            conflict_warnings: Vec::new(),
        }
    }

    /// Create a new working state from a scene model (legacy, without snapshots)
    pub fn new(scene: SceneModel) -> Self {
        Self::new_with_snapshots(scene, "unknown_runtime_snapshot".to_string(), None)
    }

    /// Get the scene ID
    pub fn scene_id(&self) -> &str {
        &self.scene.scene_id
    }

    /// Get the scene turn ID
    pub fn scene_turn_id(&self) -> &str {
        &self.scene.scene_turn_id
    }

    /// Apply a scene delta
    pub fn apply_scene_delta(&mut self, delta: &SceneDelta) -> Result<(), String> {
        // Apply entity deltas
        for entity_delta in &delta.entity_deltas {
            match entity_delta.delta_kind.as_str() {
                "update" => {
                    if let Some(entity) = self
                        .scene
                        .entities
                        .iter_mut()
                        .find(|e| e.entity_id == entity_delta.entity_id)
                    {
                        // Apply patch to entity
                        if let Ok(updates) = serde_json::from_value::<
                            HashMap<String, serde_json::Value>,
                        >(entity_delta.payload.clone())
                        {
                            if let Some(posture) = updates.get("posture").and_then(|v| v.as_str()) {
                                entity.posture = posture.to_string();
                            }
                        }
                    }
                }
                "add" => {
                    // Add new entity
                    if let Ok(entity) =
                        serde_json::from_value::<SceneEntity>(entity_delta.payload.clone())
                    {
                        self.scene.entities.push(entity);
                    }
                }
                "remove" => {
                    self.scene
                        .entities
                        .retain(|e| e.entity_id != entity_delta.entity_id);
                }
                _ => {}
            }
        }

        // Apply physical condition deltas
        if let Some(physical_delta) = &delta.physical_delta {
            // Apply patches to physical conditions
            let _ = physical_delta; // TODO: Apply actual patches
        }

        // Append events
        for event_draft in &delta.event_appends {
            let event = SceneEvent {
                event_id: format!("evt_{}", uuid::Uuid::new_v4()),
                event_kind: event_draft.event_kind.clone(),
                involved_entity_ids: event_draft.involved_entity_ids.clone(),
                payload: event_draft.payload.clone(),
                created_at: chrono::Utc::now(),
            };
            self.scene.event_stream.push(event);
        }

        Ok(())
    }

    /// Set character intent
    pub fn set_character_intent(&mut self, character_id: String, intent: IntentPlan) {
        self.character_intents.insert(character_id, intent);
    }

    /// Get character intent
    pub fn get_character_intent(&self, character_id: &str) -> Option<&IntentPlan> {
        self.character_intents.get(character_id)
    }

    /// Add a reaction window
    pub fn add_reaction_window(&mut self, window: ReactionWindow) {
        self.pending_reactions.push(window);
    }

    /// Add provisional truth candidate
    pub fn add_provisional_candidate(&mut self, candidate: ProvisionalTruthCandidate) {
        self.provisional_truths.push(candidate);
    }

    /// Add conflict warning
    pub fn add_conflict_warning(&mut self, warning: ConflictWarning) {
        self.conflict_warnings.push(warning);
    }
}
