//! Subjective state reducer
//!
//! Reduces prior L3 + CognitivePassOutput to new L3 snapshot.
//!
//! Key rules:
//! - ConfidenceShift, emotion changes, goal operations are mapped to L3 values by program
//! - Propositions about others go to relation_models, about world/events go to belief_state
//! - User roleplay skips CognitivePass, uses PlayerSubjectiveInput directly
//! - PlayerBeliefSource::NewHypothesis only writes to this character's L3

use crate::agent::models::{
    BeliefEntry, BeliefSource, BeliefStability, BeliefUpdate, CharacterCognitivePassOutput,
    CharacterSubjectiveState, ConfidenceShift, EmotionalShiftDelta, Goal, GoalPriority, GoalStatus,
    MoodTrend, ObservableEventDelta, PerceivedIntent, PlayerBeliefSource, PlayerSubjectiveInput,
    PrimaryEmotion, RelationModel,
};
use chrono::Utc;

/// Subjective state reducer - generates L3 snapshots
pub struct SubjectiveStateReducer;

impl SubjectiveStateReducer {
    /// Reduce prior state + cognitive output to new state
    pub fn reduce(
        character_id: &str,
        scene_turn_id: &str,
        prior_state: CharacterSubjectiveState,
        cognitive_output: Option<CharacterCognitivePassOutput>,
        player_input: Option<PlayerSubjectiveInput>,
        events: Vec<ObservableEventDelta>,
    ) -> CharacterSubjectiveState {
        let mut new_state = prior_state.clone();
        new_state.scene_turn_id = scene_turn_id.to_string();

        // Apply cognitive output if present
        if let Some(output) = cognitive_output {
            Self::apply_cognitive_output(&mut new_state, &output);
        }

        // Apply player input if present
        if let Some(input) = player_input {
            Self::apply_player_input(&mut new_state, &input, character_id);
        }

        // Process observable events
        Self::process_events(&mut new_state, &events);

        new_state
    }

    /// Apply cognitive pass output to state
    fn apply_cognitive_output(
        state: &mut CharacterSubjectiveState,
        output: &CharacterCognitivePassOutput,
    ) {
        // Apply belief updates
        Self::apply_belief_update(state, &output.belief_update);

        // Update emotion state from emotional shift
        Self::apply_emotional_shift(state, &output.belief_update.emotional_shift);

        // Note: Intent plan is not stored in L3, it's used by OutcomePlanner
        // Note: Body reaction delta is not stored in L3, it's used by OutcomePlanner
    }

    /// Apply belief update to state
    fn apply_belief_update(state: &mut CharacterSubjectiveState, update: &BeliefUpdate) {
        // Apply reinforced beliefs
        for entry in &update.stable_beliefs_reinforced {
            Self::apply_confidence_shift(state, &entry.proposition, entry.confidence_shift);
        }

        // Apply weakened beliefs
        for entry in &update.stable_beliefs_weakened {
            Self::apply_confidence_shift(state, &entry.proposition, entry.confidence_shift);
        }

        // Add new hypotheses
        for hypothesis in &update.new_hypotheses {
            // Avoid duplicate propositions
            if !state
                .belief_state
                .beliefs
                .iter()
                .any(|b| b.proposition == hypothesis.proposition)
            {
                state.belief_state.beliefs.push(BeliefEntry {
                    belief_id: crate::agent::models::generate_id("belief"),
                    proposition: hypothesis.proposition.clone(),
                    confidence: Self::hypothesis_status_to_confidence(&hypothesis.status),
                    source: BeliefSource::Inference,
                    stability: Self::hypothesis_status_to_stability(&hypothesis.status),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                });
            }
        }

        // Apply revised relation models
        for revised in &update.revised_models_of_others {
            Self::apply_relation_revision(state, revised);
        }

        // Handle contradictions and tension
        for contradiction in &update.contradictions_and_tension {
            // Log contradiction for trace, may affect stability
            Self::handle_contradiction(state, contradiction);
        }
    }

    /// Apply confidence shift to a belief
    fn apply_confidence_shift(
        state: &mut CharacterSubjectiveState,
        proposition: &str,
        shift: ConfidenceShift,
    ) {
        if let Some(belief) = state
            .belief_state
            .beliefs
            .iter_mut()
            .find(|b| b.proposition == proposition)
        {
            let delta = Self::confidence_shift_to_delta(shift);
            belief.confidence = (belief.confidence + delta).clamp(0.0, 1.0);
            belief.updated_at = Utc::now();
        }
    }

    /// Convert confidence shift to numeric delta
    fn confidence_shift_to_delta(shift: ConfidenceShift) -> f64 {
        match shift {
            ConfidenceShift::StrongDecrease => -0.3,
            ConfidenceShift::Decrease => -0.15,
            ConfidenceShift::Unchanged => 0.0,
            ConfidenceShift::Increase => 0.15,
            ConfidenceShift::StrongIncrease => 0.3,
            ConfidenceShift::Flip => 0.0, // Handled separately
        }
    }

    /// Convert hypothesis status to confidence value
    fn hypothesis_status_to_confidence(status: &str) -> f64 {
        match status {
            "tentative" => 0.3,
            "working" => 0.5,
            "strong" => 0.8,
            _ => 0.5,
        }
    }

    /// Convert hypothesis status to stability
    fn hypothesis_status_to_stability(status: &str) -> BeliefStability {
        match status {
            "tentative" => BeliefStability::Tentative,
            "working" => BeliefStability::Working,
            "strong" => BeliefStability::Stable,
            _ => BeliefStability::Tentative,
        }
    }

    /// Apply relation model revision
    fn apply_relation_revision(
        state: &mut CharacterSubjectiveState,
        revised: &crate::agent::models::RevisedRelationModel,
    ) {
        // Find or create relation model for target
        if let Some(relation) = state
            .relation_models
            .iter_mut()
            .find(|r| r.target_character_id == revised.target_character_id)
        {
            // Apply trust shift
            let trust_delta = Self::confidence_shift_to_delta(revised.trust_shift);
            relation.trust = (relation.trust + trust_delta).clamp(0.0, 1.0);

            // Apply intent assessment change
            if let Some(intent) = &revised.intent_assessment_change {
                relation.perceived_intent = Self::parse_perceived_intent(intent);
            }

            // Add new impressions
            for impression in &revised.new_impressions {
                if !relation.key_impressions.contains(impression) {
                    relation.key_impressions.push(impression.clone());
                }
            }
            relation.last_updated = Utc::now();
        } else {
            // Create new relation model
            state.relation_models.push(RelationModel {
                target_character_id: revised.target_character_id.clone(),
                trust: 0.5,
                perceived_intent: PerceivedIntent::Unknown,
                emotional_valence: 0.0,
                relationship_type: crate::agent::models::RelationshipType::Stranger,
                key_impressions: revised.new_impressions.clone(),
                last_updated: Utc::now(),
            });
        }
    }

    /// Parse perceived intent from string
    fn parse_perceived_intent(intent: &str) -> PerceivedIntent {
        match intent {
            "friendly" => PerceivedIntent::Friendly,
            "neutral" => PerceivedIntent::Neutral,
            "suspicious" => PerceivedIntent::Suspicious,
            "hostile" => PerceivedIntent::Hostile,
            "deceptive" => PerceivedIntent::Deceptive,
            "protective" => PerceivedIntent::Protective,
            _ => PerceivedIntent::Unknown,
        }
    }

    /// Handle contradiction in belief system
    fn handle_contradiction(
        state: &mut CharacterSubjectiveState,
        contradiction: &crate::agent::models::ContradictionResolution,
    ) {
        // Mark affected beliefs as less stable
        for belief in &mut state.belief_state.beliefs {
            if contradiction
                .conflicting_beliefs
                .contains(&belief.proposition)
            {
                belief.stability = BeliefStability::Tentative;
                belief.updated_at = Utc::now();
            }
        }
    }

    /// Apply emotional shift to state
    fn apply_emotional_shift(state: &mut CharacterSubjectiveState, shift: &EmotionalShiftDelta) {
        state.emotion_state.primary_emotion = shift.primary_emotion;
        state.emotion_state.intensity = (0.5 + shift.intensity_change).clamp(0.0, 1.0);
        state.emotion_state.secondary_emotions = shift.secondary_changes.clone();

        // Determine mood trend from intensity change
        state.emotion_state.mood_trend = if shift.intensity_change > 0.1 {
            MoodTrend::Improving
        } else if shift.intensity_change < -0.1 {
            MoodTrend::Declining
        } else {
            MoodTrend::Stable
        };
    }

    /// Apply player input to state
    fn apply_player_input(
        state: &mut CharacterSubjectiveState,
        input: &PlayerSubjectiveInput,
        _character_id: &str,
    ) {
        // Apply declared emotion
        if let Some(emotion) = &input.emotion_declared {
            Self::apply_emotional_shift(state, emotion);
        }

        // Apply belief directives
        for directive in &input.belief_directives {
            Self::apply_belief_directive(state, directive);
        }

        // Apply goal directives
        for goal_directive in &input.goal_directives {
            Self::apply_goal_directive(state, goal_directive);
        }

        // Inner monologue doesn't directly affect L3 structure
        // It's used by CognitivePass and narrative generation
    }

    /// Apply a single belief directive from player
    fn apply_belief_directive(
        state: &mut CharacterSubjectiveState,
        directive: &crate::agent::models::PlayerBeliefDirective,
    ) {
        match directive.source {
            PlayerBeliefSource::ExistingAccessibleFact => {
                // Reinforce or weaken belief based on accessible fact
                Self::apply_confidence_shift(
                    state,
                    &directive.proposition_ref,
                    directive.confidence_shift,
                );
            }
            PlayerBeliefSource::ExistingSubjectiveBelief => {
                // Update existing subjective belief
                Self::apply_confidence_shift(
                    state,
                    &directive.proposition_ref,
                    directive.confidence_shift,
                );
            }
            PlayerBeliefSource::NewHypothesis => {
                // Create new hypothesis (can be wrong belief)
                if !state
                    .belief_state
                    .beliefs
                    .iter()
                    .any(|b| b.proposition == directive.proposition_ref)
                {
                    state.belief_state.beliefs.push(BeliefEntry {
                        belief_id: crate::agent::models::generate_id("belief"),
                        proposition: directive.proposition_ref.clone(),
                        confidence: 0.5,
                        source: BeliefSource::Assumption,
                        stability: BeliefStability::Tentative,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    });
                }
            }
            PlayerBeliefSource::DirectorInstructionRef => {
                // Director instruction - treat as strong signal
                Self::apply_confidence_shift(
                    state,
                    &directive.proposition_ref,
                    directive.confidence_shift,
                );
            }
        }
    }

    /// Apply a single goal directive from player
    fn apply_goal_directive(
        state: &mut CharacterSubjectiveState,
        directive: &crate::agent::models::PlayerGoalDirective,
    ) {
        let operation = directive.operation.as_str();

        // Find goal in all goal lists
        let all_goals: Vec<&mut Vec<Goal>> = vec![
            &mut state.current_goals.short_term,
            &mut state.current_goals.medium_term,
            &mut state.current_goals.hidden,
        ];

        match operation {
            "abandon" | "weaken" => {
                // Remove or mark abandoned
                for goals in all_goals {
                    if let Some(goal) = goals.iter_mut().find(|g| g.goal_id == directive.goal_ref) {
                        goal.status = GoalStatus::Abandoned;
                    }
                }
            }
            "reinforce" => {
                // Increase priority
                for goals in all_goals {
                    if let Some(goal) = goals.iter_mut().find(|g| g.goal_id == directive.goal_ref) {
                        goal.priority = match goal.priority {
                            GoalPriority::Low => GoalPriority::Normal,
                            GoalPriority::Normal => GoalPriority::High,
                            GoalPriority::High => GoalPriority::Urgent,
                            GoalPriority::Urgent => GoalPriority::Critical,
                            GoalPriority::Critical => GoalPriority::Critical,
                        };
                    }
                }
            }
            "add" => {
                // Add new short-term goal
                state.current_goals.short_term.push(Goal {
                    goal_id: directive.goal_ref.clone(),
                    description: String::new(), // Would need more context
                    priority: GoalPriority::Normal,
                    status: GoalStatus::Active,
                    progress: 0.0,
                    deadline: None,
                    created_at: Utc::now(),
                });
            }
            _ => {}
        }
    }

    /// Process observable events to derive belief updates
    fn process_events(state: &mut CharacterSubjectiveState, events: &[ObservableEventDelta]) {
        for event in events {
            // Derive observations from event
            for descriptor in &event.sensory_descriptors {
                // Could create new beliefs or update existing ones
                // based on what the character observed
                let _ = descriptor; // Placeholder for actual processing
            }
        }
    }
}
