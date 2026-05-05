//! Input assembly
//!
//! Assembles CognitivePassInput, ensuring no Layer 1 raw objects leak.
//!
//! Key invariant: InputAssembly guarantees that CognitivePassInput
//! contains ONLY Layer 2 derived data - no raw Layer 1 objects.

use serde_json;

use crate::agent::models::{
    AccessibleEntry, AccessibleKnowledge, CharacterCognitivePassInput, CharacterSubjectiveState,
    EmbodimentState, FilteredSceneView, KnowledgeKind, ObservableEventDelta,
};

/// Input assembly - builds CognitivePassInput from Layer 2 data
pub struct InputAssembly;

impl InputAssembly {
    /// Assemble cognitive pass input
    pub fn assemble(
        character_id: &str,
        scene_turn_id: &str,
        filtered_scene: FilteredSceneView,
        embodiment: EmbodimentState,
        accessible_knowledge: AccessibleKnowledge,
        prior_subjective: CharacterSubjectiveState,
        recent_events: Vec<ObservableEventDelta>,
    ) -> Result<CharacterCognitivePassInput, String> {
        // Validate that no Layer 1 raw objects are present
        Self::validate_no_layer1_leak(&filtered_scene, &accessible_knowledge)?;

        // Validate character_id consistency
        if filtered_scene.character_id != character_id {
            return Err(format!(
                "FilteredSceneView character_id mismatch: expected {}, got {}",
                character_id, filtered_scene.character_id
            ));
        }
        if embodiment.character_id != character_id {
            return Err(format!(
                "EmbodimentState character_id mismatch: expected {}, got {}",
                character_id, embodiment.character_id
            ));
        }
        if accessible_knowledge.character_id != character_id {
            return Err(format!(
                "AccessibleKnowledge character_id mismatch: expected {}, got {}",
                character_id, accessible_knowledge.character_id
            ));
        }
        if prior_subjective.character_id != character_id {
            return Err(format!(
                "PriorSubjectiveState character_id mismatch: expected {}, got {}",
                character_id, prior_subjective.character_id
            ));
        }

        Ok(CharacterCognitivePassInput {
            character_id: character_id.to_string(),
            scene_turn_id: scene_turn_id.to_string(),
            filtered_scene_view: filtered_scene,
            embodiment_state: embodiment,
            accessible_knowledge,
            prior_subjective_state: prior_subjective,
            recent_event_delta: recent_events,
        })
    }

    /// Validate that no Layer 1 raw objects are present
    fn validate_no_layer1_leak(
        filtered_scene: &FilteredSceneView,
        accessible_knowledge: &AccessibleKnowledge,
    ) -> Result<(), String> {
        // Check that filtered_scene doesn't contain raw Layer 1 objects
        // - No raw base_attributes values
        // - No raw mana_power values
        // - No raw physical_conditions values
        // All should be tier/delta/descriptors

        // Validate perceived_attributes don't contain raw values
        for profile in &filtered_scene.perceived_attributes {
            // PerceivedAttributeProfile should only have tier/delta/descriptors
            // Raw values should not be present
            if profile.tier_assessment.is_none()
                && profile.delta.is_none()
                && profile.confidence < 0.5
            {
                return Err(format!(
                    "PerceivedAttributeProfile for {} has no tier/delta and low confidence - possible Layer 1 leak",
                    profile.source_id
                ));
            }
        }

        // Validate mana_signals don't contain raw values
        for signal in &filtered_scene.mana_signals {
            // ManaSignal should only have intensity (tier-like) and attribute
            // Not raw mana_power values from characters
            if signal.intensity > 1.0 {
                // Intensity should be normalized 0-1 range
                return Err(format!(
                    "ManaSignal {} has intensity {} > 1.0 - possible Layer 1 raw value leak",
                    signal.signal_id, signal.intensity
                ));
            }
        }

        // Validate accessible_knowledge entries
        Self::validate_accessible_knowledge(accessible_knowledge)?;

        Ok(())
    }

    /// Validate accessible knowledge entries
    fn validate_accessible_knowledge(
        accessible_knowledge: &AccessibleKnowledge,
    ) -> Result<(), String> {
        for entry in &accessible_knowledge.entries {
            // Check that entry is AccessibleEntry, not raw KnowledgeEntry
            Self::validate_accessible_entry(entry)?;
        }
        Ok(())
    }

    /// Validate a single accessible entry
    fn validate_accessible_entry(entry: &AccessibleEntry) -> Result<(), String> {
        // AccessibleEntry should have:
        // - knowledge_id (reference, not full object)
        // - kind (enum, safe)
        // - subject (reference, safe)
        // - accessible_content (filtered content, not raw content)
        // - source_hint (how access was granted)

        // Check that accessible_content doesn't contain forbidden fields
        Self::validate_content_no_layer1_fields(&entry.accessible_content, &entry.kind)?;

        Ok(())
    }

    /// Validate content doesn't contain Layer 1 raw fields
    fn validate_content_no_layer1_fields(
        content: &serde_json::Value,
        kind: &KnowledgeKind,
    ) -> Result<(), String> {
        // Forbidden fields that indicate Layer 1 leak:
        let forbidden_fields = [
            "base_attributes",      // Raw attribute values
            "effective_mana_power", // Raw mana power
            "raw_value",            // Any raw numeric value
            "access_policy",        // Full access policy (should be filtered)
            "subject_awareness",    // Full awareness info (should be filtered)
            "valid_from",           // Time anchor (Layer 1 metadata)
            "valid_until",          // Time anchor (Layer 1 metadata)
            "source_session_id",    // Source info (Layer 1 metadata)
            "source_scene_turn_id", // Source info (Layer 1 metadata)
            "god_only",             // GodOnly flag should never appear
        ];

        if let serde_json::Value::Object(map) = content {
            for field in &forbidden_fields {
                if map.contains_key(*field) {
                    return Err(format!(
                        "AccessibleEntry content for {:?} contains forbidden field '{}' - Layer 1 leak detected",
                        kind, field
                    ));
                }
            }

            // Also check nested objects
            for (_key, value) in map {
                if let serde_json::Value::Object(nested) = value {
                    for field in &forbidden_fields {
                        if nested.contains_key(*field) {
                            return Err(format!(
                                "AccessibleEntry content for {:?} contains forbidden field '{}' in nested object - Layer 1 leak detected",
                                kind, field
                            ));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Create a summary of the input for logging/trace
    pub fn create_input_summary(input: &CharacterCognitivePassInput) -> InputSummary {
        InputSummary {
            character_id: input.character_id.clone(),
            scene_turn_id: input.scene_turn_id.clone(),
            observable_entity_count: input.filtered_scene_view.observable_entities.len(),
            accessible_knowledge_count: input.accessible_knowledge.entries.len(),
            recent_event_count: input.recent_event_delta.len(),
            has_prior_goals: !input
                .prior_subjective_state
                .current_goals
                .short_term
                .is_empty()
                || !input
                    .prior_subjective_state
                    .current_goals
                    .medium_term
                    .is_empty(),
            primary_emotion: input.prior_subjective_state.emotion_state.primary_emotion,
        }
    }
}

/// Summary of cognitive pass input for logging
#[derive(Debug, Clone)]
pub struct InputSummary {
    pub character_id: String,
    pub scene_turn_id: String,
    pub observable_entity_count: usize,
    pub accessible_knowledge_count: usize,
    pub recent_event_count: usize,
    pub has_prior_goals: bool,
    pub primary_emotion: crate::agent::models::PrimaryEmotion,
}
