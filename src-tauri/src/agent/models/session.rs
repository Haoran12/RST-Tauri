//! Session and timeline models
//!
//! WorldMainlineCursor, AgentSession, SessionTurn, WorldTurn
//! TemporalStateRecord, ObjectiveRelationship, WorldStateAt

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::common::*;

/// World mainline cursor - tracks current mainline frontier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldMainlineCursor {
    pub world_id: String,
    /// Timeline ID (first version fixed to "main")
    pub timeline_id: String,
    /// Current mainline head turn ID
    pub mainline_head_turn_id: Option<String>,
    /// Current mainline time anchor
    pub mainline_time_anchor: TimeAnchor,
    pub updated_at: DateTime<Utc>,
}

impl WorldMainlineCursor {
    pub fn new(world_id: String, initial_time: TimeAnchor) -> Self {
        Self {
            world_id,
            timeline_id: "main".to_string(),
            mainline_head_turn_id: None,
            mainline_time_anchor: initial_time,
            updated_at: Utc::now(),
        }
    }
}

/// Agent session - a chat/play session within a World
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub session_id: String,
    pub world_id: String,
    pub title: String,
    pub session_kind: AgentSessionKind,
    /// Time anchor for this session's period
    pub period_anchor: TimeAnchor,
    /// Player mode: Character (roleplay as a character) or Director (world-external director)
    pub player_mode: PlayerMode,
    /// Player-controlled character ID (required for Character mode, must be None for Director mode)
    pub player_character_id: Option<String>,
    pub canon_status: SessionCanonStatus,
    pub conflict_policy: Option<ConflictPolicyDecision>,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Archived,
    Deleted,
}

impl AgentSession {
    pub fn new(
        world_id: String,
        title: String,
        session_kind: AgentSessionKind,
        period_anchor: TimeAnchor,
    ) -> Self {
        let now = Utc::now();
        Self {
            session_id: generate_id("session"),
            world_id,
            title,
            session_kind,
            period_anchor,
            player_mode: PlayerMode::default(),
            player_character_id: None,
            canon_status: SessionCanonStatus::CanonCandidate,
            conflict_policy: None,
            status: SessionStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new session with explicit player mode
    pub fn new_with_mode(
        world_id: String,
        title: String,
        session_kind: AgentSessionKind,
        period_anchor: TimeAnchor,
        player_mode: PlayerMode,
        player_character_id: Option<String>,
    ) -> Result<Self, String> {
        // Validate player_mode and player_character_id consistency
        match player_mode {
            PlayerMode::Character => {
                if player_character_id.is_none() {
                    return Err("Character mode requires player_character_id".to_string());
                }
            }
            PlayerMode::Director => {
                if player_character_id.is_some() {
                    return Err("Director mode must not have player_character_id".to_string());
                }
            }
        }

        let now = Utc::now();
        Ok(Self {
            session_id: generate_id("session"),
            world_id,
            title,
            session_kind,
            period_anchor,
            player_mode,
            player_character_id,
            canon_status: SessionCanonStatus::CanonCandidate,
            conflict_policy: None,
            status: SessionStatus::Active,
            created_at: now,
            updated_at: now,
        })
    }

    /// Validate the session's player_mode and player_character_id consistency
    pub fn validate(&self) -> Result<(), String> {
        match self.player_mode {
            PlayerMode::Character => {
                if self.player_character_id.is_none() {
                    return Err("Character mode requires player_character_id".to_string());
                }
            }
            PlayerMode::Director => {
                if self.player_character_id.is_some() {
                    return Err("Director mode must not have player_character_id".to_string());
                }
            }
        }
        Ok(())
    }

    /// Check if the session is in Character mode
    pub fn is_character_mode(&self) -> bool {
        self.player_mode == PlayerMode::Character
    }

    /// Check if the session is in Director mode
    pub fn is_director_mode(&self) -> bool {
        self.player_mode == PlayerMode::Director
    }
}

/// Session turn - message within a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTurn {
    pub session_turn_id: String,
    pub session_id: String,
    /// Reference to world turn (if canonical)
    pub scene_turn_id: Option<String>,
    /// Local index within session
    pub local_index: u32,
    pub role: TurnRole,
    /// Message content (JSON)
    pub message_json: serde_json::Value,
    pub canon_status: SessionTurnCanonStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnRole {
    User,
    Assistant,
    System,
}

/// World turn - Agent runtime turn journal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldTurn {
    pub scene_turn_id: String,
    pub parent_turn_id: Option<String>,
    pub session_id: Option<String>,
    pub timeline_id: String,
    /// Story time anchor (not wall clock)
    pub story_time_anchor: TimeAnchor,
    /// User input (JSON)
    pub user_message: serde_json::Value,
    /// Rendered output (narrative text)
    pub rendered_output: Option<String>,
    pub runtime_turn_status: RuntimeTurnCanonStatus,
    pub status: WorldTurnStatus,
    pub created_at: DateTime<Utc>,
    pub rolled_back_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldTurnStatus {
    Active,
    RolledBack,
}

impl WorldTurn {
    pub fn new(
        session_id: Option<String>,
        story_time_anchor: TimeAnchor,
        user_message: serde_json::Value,
    ) -> Self {
        Self {
            scene_turn_id: generate_id("turn"),
            parent_turn_id: None,
            session_id,
            timeline_id: "main".to_string(),
            story_time_anchor,
            user_message,
            rendered_output: None,
            runtime_turn_status: RuntimeTurnCanonStatus::Canon,
            status: WorldTurnStatus::Active,
            created_at: Utc::now(),
            rolled_back_at: None,
        }
    }
}

/// State commit record - tracks canonical state changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateCommitRecord {
    pub commit_id: String,
    pub scene_turn_id: String,
    pub changed_scene_snapshot_ids: Vec<String>,
    pub changed_location_ids: Vec<String>,
    pub changed_knowledge_ids: Vec<String>,
    pub changed_character_ids: Vec<String>,
    pub changed_subjective_snapshot_ids: Vec<String>,
    pub trace_ids: Vec<String>,
    /// Rollback patch (reverse delta or before-image)
    pub rollback_patch: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub rolled_back_at: Option<DateTime<Utc>>,
    pub rollback_reason: Option<String>,
}

/// Provisional session truth - candidate facts from past timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionalSessionTruth {
    pub provisional_id: String,
    pub session_id: String,
    pub source_session_turn_id: String,
    pub source_scene_turn_id: Option<String>,
    pub story_time_anchor: TimeAnchor,
    pub derived_from_event_id: Option<String>,
    pub candidate_kind: ProvisionalCandidateKind,
    pub candidate_payload: serde_json::Value,
    pub promotion_status: PromotionStatus,
    pub promoted_knowledge_id: Option<String>,
    pub promoted_scene_turn_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProvisionalCandidateKind {
    KnowledgeEntry,
    EventDetail,
    RelationDetail,
    LocationDetail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromotionStatus {
    Pending,
    Promoted,
    BlockedConflict,
    NonCanon,
    TraceOnly,
}

/// Conflict report - hard conflict in past timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictReport {
    pub conflict_id: String,
    pub session_id: String,
    pub session_turn_id: String,
    pub scene_turn_id: Option<String>,
    pub severity: ConflictSeverity,
    pub source_constraint_ids: Vec<String>,
    pub affected_provisional_ids: Vec<String>,
    pub policy_decision: Option<ConflictPolicyDecision>,
    pub summary: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictSeverity {
    Soft,
    Hard,
}

/// Provisional truth candidate - for scene state extractor output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionalTruthCandidate {
    pub provisional_id: String,
    pub source_session_id: String,
    pub source_session_turn_id: String,
    pub source_scene_turn_id: Option<String>,
    pub source_kind: String,
    pub payload: serde_json::Value,
    pub confidence: f64,
    pub constraints: Vec<String>,
}

/// Conflict warning - potential conflict detected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictWarning {
    pub warning_id: String,
    pub severity: ConflictSeverity,
    pub description: String,
    pub affected_entities: Vec<String>,
    pub suggested_resolution: Option<String>,
}

// ===== Temporal State Record =====

/// Temporal state record - time-varying Layer 1 state
///
/// Used to reconstruct world state at any point in the timeline.
/// Character position, temporary state, location status, item state,
/// objective relationships, etc. must all be recorded here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalStateRecord {
    pub state_record_id: String,
    pub subject_type: TemporalStateSubjectType,
    pub subject_id: String,
    pub state_kind: TemporalStateKind,
    /// When this state became valid
    pub valid_from: TimeAnchor,
    /// When this state became invalid (None = still valid)
    pub valid_until: Option<TimeAnchor>,
    /// Structured state payload
    pub payload: serde_json::Value,
    pub source_scene_turn_id: Option<String>,
    pub source_session_id: Option<String>,
    pub canon_status: TemporalCanonStatus,
    pub schema_version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemporalStateSubjectType {
    Character,
    Location,
    Scene,
    Object,
    Relationship,
    Resource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemporalStateKind {
    /// Character position in scene
    Position,
    /// Character temporary state (injuries, fatigue, etc.)
    TemporaryState,
    /// Location status (active, damaged, sealed, etc.)
    LocationStatus,
    /// Item/object state
    ItemState,
    /// Objective relationship between characters
    ObjectiveRelation,
    /// Authorization/permission
    Authorization,
    /// Resource state (mana, currency, etc.)
    ResourceState,
    /// Scene-specific state
    SceneState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemporalCanonStatus {
    /// Canonical truth
    Canon,
    /// Promoted from provisional
    ProvisionalPromoted,
    /// Non-canon (what-if, future preview)
    NonCanon,
}

impl TemporalStateRecord {
    pub fn new(
        subject_type: TemporalStateSubjectType,
        subject_id: String,
        state_kind: TemporalStateKind,
        valid_from: TimeAnchor,
        payload: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        Self {
            state_record_id: generate_id("tsr"),
            subject_type,
            subject_id,
            state_kind,
            valid_from,
            valid_until: None,
            payload,
            source_scene_turn_id: None,
            source_session_id: None,
            canon_status: TemporalCanonStatus::Canon,
            schema_version: SCHEMA_VERSION.to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if this record is valid at the given time anchor
    pub fn is_valid_at(&self, time: &TimeAnchor) -> bool {
        // Must be valid from before or at the query time
        if self.valid_from.ordinal > time.ordinal {
            return false;
        }
        // If valid_until is set, must be after the query time
        if let Some(valid_until) = &self.valid_until {
            if valid_until.ordinal <= time.ordinal {
                return false;
            }
        }
        true
    }
}

// ===== Objective Relationship =====

/// Objective relationship - L1 materialized cache for current mainline
///
/// Used for AccessCondition::SocialAccessAtLeast and other high-frequency
/// program judgments. Authority comes from TemporalStateRecord or
/// KnowledgeEntry, not from L3 relation_models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveRelationship {
    pub relation_id: String,
    pub subject_character_id: String,
    pub target_character_id: String,
    pub relation_kind: ObjectiveRelationKind,
    /// Access level for SocialAccessAtLeast conditions
    pub access_level: f64,
    /// Authorization tags (private_room_access, archive_access, command_rank, etc.)
    pub authorization_tags: Vec<String>,
    pub valid_from: TimeAnchor,
    pub valid_until: Option<TimeAnchor>,
    pub source_knowledge_id: Option<String>,
    pub source_scene_turn_id: Option<String>,
    pub schema_version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectiveRelationKind {
    /// Allied relationship
    Ally,
    /// Family relationship
    Family,
    /// Faction rank/position
    FactionRank,
    /// Employer relationship
    Employer,
    /// Oath/sworn relationship
    Oath,
    /// Access grant
    AccessGrant,
    /// Hostile relationship
    Hostility,
    /// Rivalry
    Rival,
    /// Master/disciple
    MasterDisciple,
    /// Trade partner
    TradePartner,
    /// Custom relationship
    Custom,
}

impl ObjectiveRelationship {
    pub fn new(
        subject_character_id: String,
        target_character_id: String,
        relation_kind: ObjectiveRelationKind,
        valid_from: TimeAnchor,
    ) -> Self {
        let now = Utc::now();
        Self {
            relation_id: generate_id("orel"),
            subject_character_id,
            target_character_id,
            relation_kind,
            access_level: 0.0,
            authorization_tags: Vec::new(),
            valid_from,
            valid_until: None,
            source_knowledge_id: None,
            source_scene_turn_id: None,
            schema_version: SCHEMA_VERSION.to_string(),
            created_at: now,
            updated_at: now,
        }
    }
}

// ===== World State At =====

/// World state at a specific time anchor
///
/// Reconstructs the world state for past timeline queries.
/// Uses TemporalStateRecord and KnowledgeEntry valid_from/valid_until.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldStateAt {
    pub world_id: String,
    pub time_anchor: TimeAnchor,
    /// Character positions at this time
    pub character_positions: Vec<CharacterPositionState>,
    /// Character temporary states at this time
    pub character_temporary_states: Vec<CharacterTemporaryStateAt>,
    /// Location states at this time
    pub location_states: Vec<LocationStateAt>,
    /// Objective relationships at this time
    pub objective_relationships: Vec<ObjectiveRelationshipAt>,
    /// Valid knowledge entries at this time
    pub valid_knowledge_ids: Vec<String>,
    pub reconstructed_at: DateTime<Utc>,
}

/// Character position state at a specific time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterPositionState {
    pub character_id: String,
    pub location_id: String,
    pub position_details: Option<serde_json::Value>,
}

/// Character temporary state at a specific time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterTemporaryStateAt {
    pub character_id: String,
    pub injuries: Vec<super::character::InjuryState>,
    pub fatigue: f64,
    pub pain_load: f64,
    pub mana_reserve: Option<f64>,
    pub active_conditions: Vec<String>,
}

/// Location state at a specific time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationStateAt {
    pub location_id: String,
    pub status: String,
    pub additional_state: Option<serde_json::Value>,
}

/// Objective relationship at a specific time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveRelationshipAt {
    pub subject_character_id: String,
    pub target_character_id: String,
    pub relation_kind: ObjectiveRelationKind,
    pub access_level: f64,
    pub authorization_tags: Vec<String>,
}
