//! Subjective state models - Layer 3
//!
//! CharacterSubjectiveState, BeliefState, EmotionState, RelationModel, CurrentGoals

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::common::*;

/// Character subjective state (Layer 3)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSubjectiveState {
    pub character_id: String,
    pub scene_turn_id: String,
    pub session_id: Option<String>,
    pub story_time_anchor: Option<TimeAnchor>,
    pub canon_status: SubjectiveCanonStatus,
    pub belief_state: BeliefState,
    pub emotion_state: EmotionState,
    pub relation_models: Vec<RelationModel>,
    pub current_goals: CurrentGoals,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubjectiveCanonStatus {
    Canon,
    NonCanon,
}

impl CharacterSubjectiveState {
    pub fn new(character_id: String, scene_turn_id: String) -> Self {
        Self {
            character_id,
            scene_turn_id,
            session_id: None,
            story_time_anchor: None,
            canon_status: SubjectiveCanonStatus::Canon,
            belief_state: BeliefState::new(),
            emotion_state: EmotionState::new(),
            relation_models: Vec::new(),
            current_goals: CurrentGoals::new(),
            created_at: Utc::now(),
        }
    }
}

/// Belief state - propositions about world/events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefState {
    pub beliefs: Vec<BeliefEntry>,
}

impl BeliefState {
    pub fn new() -> Self {
        Self {
            beliefs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefEntry {
    pub belief_id: String,
    pub proposition: String,
    pub confidence: f64,
    pub source: BeliefSource,
    pub stability: BeliefStability,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BeliefSource {
    DirectObservation,
    Inference,
    ToldBy { character_id: String },
    Assumption,
    PriorBelief,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeliefStability {
    Tentative,
    Working,
    Stable,
    Core,
}

/// Emotion state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionState {
    pub primary_emotion: PrimaryEmotion,
    pub intensity: f64,
    pub secondary_emotions: Vec<SecondaryEmotion>,
    pub mood_trend: MoodTrend,
}

impl EmotionState {
    pub fn new() -> Self {
        Self {
            primary_emotion: PrimaryEmotion::Neutral,
            intensity: 0.5,
            secondary_emotions: Vec::new(),
            mood_trend: MoodTrend::Stable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimaryEmotion {
    Neutral,
    Joy,
    Sadness,
    Anger,
    Fear,
    Disgust,
    Surprise,
    Anticipation,
    Trust,
    Contempt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecondaryEmotion {
    pub emotion: PrimaryEmotion,
    pub intensity: f64,
    pub trigger: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoodTrend {
    Improving,
    Stable,
    Declining,
    Volatile,
}

/// Relation model - subjective impression of another character
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationModel {
    pub target_character_id: String,
    pub trust: f64,
    pub perceived_intent: PerceivedIntent,
    pub emotional_valence: f64,
    pub relationship_type: RelationshipType,
    pub key_impressions: Vec<String>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PerceivedIntent {
    Unknown,
    Friendly,
    Neutral,
    Suspicious,
    Hostile,
    Deceptive,
    Protective,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipType {
    Stranger,
    Acquaintance,
    Friend,
    CloseFriend,
    Ally,
    Rival,
    Enemy,
    Family,
    Superior,
    Subordinate,
    Mentor,
    Student,
}

/// Current goals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentGoals {
    pub short_term: Vec<Goal>,
    pub medium_term: Vec<Goal>,
    pub hidden: Vec<Goal>,
}

impl CurrentGoals {
    pub fn new() -> Self {
        Self {
            short_term: Vec::new(),
            medium_term: Vec::new(),
            hidden: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub goal_id: String,
    pub description: String,
    pub priority: GoalPriority,
    pub status: GoalStatus,
    pub progress: f64,
    pub deadline: Option<TimeAnchor>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalPriority {
    Low,
    Normal,
    High,
    Urgent,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalStatus {
    Active,
    Paused,
    Completed,
    Abandoned,
    Blocked,
}

/// Subjective state snapshot for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectiveSnapshot {
    pub snapshot_id: String,
    pub character_id: String,
    pub scene_turn_id: String,
    pub session_id: Option<String>,
    pub story_time_anchor: Option<TimeAnchor>,
    pub canon_status: SubjectiveCanonStatus,
    pub belief_state: BeliefState,
    pub emotion_state: EmotionState,
    pub relation_models: Vec<RelationModel>,
    pub current_goals: CurrentGoals,
    pub created_at: DateTime<Utc>,
}

// ===== Cognitive Pass I/O Types =====

/// Character cognitive pass input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterCognitivePassInput {
    pub character_id: String,
    pub scene_turn_id: String,
    pub filtered_scene_view: super::scene::FilteredSceneView,
    pub embodiment_state: super::scene::EmbodimentState,
    pub accessible_knowledge: super::knowledge::AccessibleKnowledge,
    pub prior_subjective_state: CharacterSubjectiveState,
    pub recent_event_delta: Vec<ObservableEventDelta>,
}

/// Observable event delta
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObservableEventDelta {
    pub event_id: String,
    pub scene_turn_id: String,
    pub event_kind: String,
    pub involved_observable_entities: Vec<String>,
    pub observable_effects: serde_json::Value,
    pub sensory_descriptors: Vec<String>,
    pub source_hint: Option<super::knowledge::AccessSource>,
}

/// Character cognitive pass output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterCognitivePassOutput {
    pub perception_delta: PerceptionDelta,
    pub belief_update: BeliefUpdate,
    pub intent_plan: IntentPlan,
    pub body_reaction_delta: Option<BodyReactionDelta>,
}

/// Perception delta
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionDelta {
    pub new_observations: Vec<String>,
    pub updated_perceptions: Vec<String>,
    pub missed_observations: Vec<String>,
}

/// Belief update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefUpdate {
    pub stable_beliefs_reinforced: Vec<BeliefShiftEntry>,
    pub stable_beliefs_weakened: Vec<BeliefShiftEntry>,
    pub new_hypotheses: Vec<NewHypothesis>,
    pub revised_models_of_others: Vec<RevisedRelationModel>,
    pub contradictions_and_tension: Vec<ContradictionResolution>,
    pub emotional_shift: EmotionalShiftDelta,
    pub decision_relevant_beliefs: Vec<String>,
}

/// Belief shift entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefShiftEntry {
    pub proposition: String,
    pub confidence_shift: ConfidenceShift,
}

/// Confidence shift (discrete levels)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceShift {
    StrongDecrease,
    Decrease,
    Unchanged,
    Increase,
    StrongIncrease,
    Flip,
}

/// New hypothesis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewHypothesis {
    pub proposition: String,
    pub status: String,
    pub evidence_refs: Vec<String>,
}

/// Revised relation model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevisedRelationModel {
    pub target_character_id: String,
    pub trust_shift: ConfidenceShift,
    pub intent_assessment_change: Option<String>,
    pub new_impressions: Vec<String>,
}

/// Contradiction resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContradictionResolution {
    pub contradiction_id: String,
    pub conflicting_beliefs: Vec<String>,
    pub resolution_strategy: String,
    pub resolution_notes: String,
}

/// Emotional shift delta
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalShiftDelta {
    pub primary_emotion: PrimaryEmotion,
    pub intensity_change: f64,
    pub secondary_changes: Vec<SecondaryEmotion>,
}

impl From<EmotionalShiftDelta> for EmotionState {
    fn from(delta: EmotionalShiftDelta) -> Self {
        EmotionState {
            primary_emotion: delta.primary_emotion,
            intensity: 0.5 + delta.intensity_change,
            secondary_emotions: delta.secondary_changes,
            mood_trend: MoodTrend::Stable,
        }
    }
}

/// Intent plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentPlan {
    pub character_id: String,
    pub intent_kind: String,
    pub target_refs: Vec<String>,
    pub intended_actions: Vec<CharacterAction>,
    pub priority: String,
    pub commitment: String,
    pub rationale: String,
}

/// Character action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterAction {
    pub action_id: String,
    pub action_kind: String,
    pub target_refs: Vec<String>,
    pub spoken_text: Option<String>,
    pub skill_id: Option<String>,
    pub requested_mana_expression: Option<super::scene::ManaExpressionMode>,
    pub declared_effect_refs: Vec<String>,
    pub outward_description: String,
}

/// Body reaction delta
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyReactionDelta {
    pub character_id: String,
    pub reaction_kind: String,
    pub intensity: String,
    pub outward_signal: String,
    pub possible_state_effect: Option<String>,
}

// ===== Player Input Types =====

/// Player subjective input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSubjectiveInput {
    pub emotion_declared: Option<EmotionalShiftDelta>,
    pub belief_directives: Vec<PlayerBeliefDirective>,
    pub goal_directives: Vec<PlayerGoalDirective>,
    pub inner_monologue: Vec<String>,
}

/// Player belief directive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerBeliefDirective {
    pub proposition_ref: String,
    pub source: PlayerBeliefSource,
    pub confidence_shift: ConfidenceShift,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerBeliefSource {
    ExistingAccessibleFact,
    ExistingSubjectiveBelief,
    NewHypothesis,
    DirectorInstructionRef,
}

/// Player goal directive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerGoalDirective {
    pub goal_ref: String,
    pub operation: String,
}

// ===== Scene State Extractor Types =====

/// User input delta
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInputDelta {
    pub turn_id: String,
    pub raw_text: String,
    pub authority_class: UserInputAuthorityClass,
    pub authority_notes: Vec<UserInputAuthorityNote>,
    pub kind: UserInputKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserInputAuthorityClass {
    PlayerCharacterIntent,
    PlayerSubjectiveState,
    SceneCandidate,
    DirectorBias,
    SessionControl,
    AmbiguousOrBlocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInputAuthorityNote {
    pub note_kind: String,
    pub field_path: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserInputKind {
    SceneNarration {
        scene_delta: super::scene::SceneDelta,
    },
    CharacterRoleplay {
        character_id: String,
        intent_plan: IntentPlan,
        spoken_dialogue: Option<String>,
        actions: Vec<CharacterAction>,
        subjective_input: Option<PlayerSubjectiveInput>,
    },
    MetaCommand {
        command: MetaCommandKind,
    },
    DirectorHint {
        outcome_bias: Option<OutcomeBias>,
        style_override: Option<StyleConstraints>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetaCommandKind {
    SkipTime,
    ChangeScene,
    Reset,
    Pause,
}

/// Outcome bias
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeBias {
    pub preferred_tone: Option<String>,
    pub outcome_pressure: Option<OutcomePressure>,
    pub protected_entities: Vec<String>,
    pub forbidden_outcomes: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutcomePressure {
    PreserveStatusQuo,
    EscalateConflict,
    DeescalateConflict,
    FavorPlayerIntent,
    FavorSimulationStrictness,
}

/// Minor actor slot for budget-deferred characters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinorActorSlot {
    pub character_id: String,
    pub tier: String,
    pub scene_role: String,
    pub current_posture: String,
    pub observable_constraints: Vec<String>,
    pub allowed_action_kinds: Vec<String>,
    pub default_behavior: String,
    pub relevant_relationship_refs: Vec<String>,
    pub salience_reason: Option<String>,
}

/// Style constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleConstraints {
    pub register: StyleRegister,
    pub detail_level: DetailLevel,
    pub atmosphere: Atmosphere,
    pub pacing: Pacing,
    pub pov: PointOfView,
    pub explicit_guidelines: Vec<String>,
    pub reference_excerpts: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StyleRegister {
    Ancient,
    Modern,
    Casual,
    Formal,
    Poetic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetailLevel {
    Sparse,
    Moderate,
    Rich,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Atmosphere {
    Tense,
    Serene,
    Ominous,
    Melancholic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pacing {
    Fast,
    Measured,
    Slow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PointOfView {
    Omniscient,
    CharacterFocused(String),
    Objective,
}

// ===== Outcome Planner Types =====

/// Outcome planner output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomePlannerOutput {
    pub outcome_plan: OutcomePlan,
    pub state_update_plan: StateUpdatePlan,
    pub knowledge_reveal_events: Vec<super::knowledge::KnowledgeRevealEvent>,
    pub conflict_reports: Vec<super::session::ConflictReport>,
}

/// Input for OutcomePlanner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomePlannerInput {
    pub scene_turn_id: String,
    pub session_context: super::scene::AgentSessionContext,
    pub truth_guidance: Option<super::knowledge::TruthGuidance>,
    pub scene_model: super::scene::SceneModel,
    pub character_records: Vec<super::character::CharacterRecord>,
    pub relevant_knowledge: Vec<super::knowledge::KnowledgeEntry>,
    pub skills: Vec<super::skill::Skill>,
    pub character_outputs: Vec<CharacterCognitivePassOutput>,
    pub user_roleplay_intents: Vec<IntentPlan>,
    pub minor_actor_slots: Vec<MinorActorSlot>,
    pub reaction_windows: Vec<ReactionWindow>,
    pub reaction_intents: Vec<ReactionIntent>,
    pub director_hint: Option<OutcomeBias>,
    pub provisional_truth_candidates: Vec<super::session::ProvisionalTruthCandidate>,
}

/// Outcome plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomePlan {
    pub outward_actions: Vec<OutwardAction>,
    pub resulting_state_changes: serde_json::Value,
    pub narratable_facts: Vec<NarratableFact>,
    pub soft_effects: Vec<SoftEffect>,
    pub blocked_effects: Vec<BlockedEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutwardAction {
    pub action_id: String,
    pub actor_id: String,
    pub action_kind: String,
    pub target_refs: Vec<String>,
    pub narratable_fact_refs: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarratableFact {
    pub fact_id: String,
    pub fact_kind: String,
    pub subject_refs: Vec<String>,
    pub source_refs: Vec<String>,
    pub allowed_claim: String,
    pub narration_scope: super::scene::NarrationScope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftEffect {
    pub source_id: String,
    pub target_id: Option<String>,
    pub effect_kind: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedEffect {
    pub source_id: String,
    pub target_id: Option<String>,
    pub attempted_state_domain: String,
    pub reason_code: String,
    pub fallback_soft_effect: Option<SoftEffect>,
}

/// State update plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateUpdatePlan {
    pub scene_delta: Option<super::scene::SceneDelta>,
    pub character_state_deltas: Vec<CharacterStateDelta>,
    pub subjective_update_refs: Vec<String>,
    pub new_memory_entries: Vec<super::knowledge::KnowledgeEntry>,
    pub soft_effects: Vec<SoftEffect>,
    pub blocked_effects: Vec<BlockedEffect>,
    pub validation_warnings: Vec<String>,
    pub consistency_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterStateDelta {
    pub character_id: String,
    pub temporary_state_delta: serde_json::Value,
    pub outward_body_signals: Vec<String>,
}

// ===== Surface Realizer Types =====

/// Surface realizer output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceRealizerOutput {
    pub narrative_text: String,
    pub used_fact_ids: Vec<String>,
}

// ===== Scene State Extractor Output =====

/// Scene state extractor output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneStateExtractorOutput {
    pub scene_update: Option<super::scene::SceneUpdate>,
    pub user_input_delta: UserInputDelta,
    pub provisional_truth_candidates: Vec<super::session::ProvisionalTruthCandidate>,
    pub conflict_warnings: Vec<super::session::ConflictWarning>,
    pub ambiguity_report: Vec<String>,
}

// ===== Scene Delta Types =====

// Note: SceneUpdate and SceneDelta are defined in scene.rs

// ===== Reaction Window Types =====

/// Reaction window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionWindow {
    pub window_id: String,
    pub scene_turn_id: String,
    pub source_event_id: String,
    pub source_action_id: String,
    pub threat_source_id: String,
    pub primary_targets: Vec<String>,
    pub observable_threat: ObservableEventDelta,
    pub eligible_reactors: Vec<ReactionEligibility>,
    pub max_reaction_depth: u8,
    pub no_reaction_to_reaction: bool,
    pub one_reaction_per_character: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionEligibility {
    pub character_id: String,
    pub reason: ReactionEligibilityReason,
    pub available_reaction_options: Vec<ReactionOption>,
    pub sensory_basis: Vec<super::knowledge::AccessSource>,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReactionEligibilityReason {
    Target,
    AllyGuard,
    AreaProtector,
    PassiveField,
    InterruptSkill,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionOption {
    pub option_id: String,
    pub skill_id: Option<String>,
    pub reaction_kind: ReactionKind,
    pub target_scope: Vec<String>,
    pub cost_preview: super::character::CostProfile,
    pub legality_basis: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReactionKind {
    Dodge,
    Block,
    Counter,
    ProtectAlly,
    Interrupt,
    PassiveMitigation,
}

/// Reaction intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionIntent {
    pub window_id: String,
    pub character_id: String,
    pub chosen_option_id: String,
    pub target_ids: Vec<String>,
    pub intent_rationale: String,
}
