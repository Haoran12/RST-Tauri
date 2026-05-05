//! Common types and enums used across Agent mode

use serde::{Deserialize, Serialize};

/// Time anchor for story timeline
/// Must be programmatically comparable within the same World
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeAnchor {
    /// Calendar system identifier
    pub calendar_id: String,
    /// Sortable time ordinal within the World
    pub ordinal: i64,
    /// Time precision level
    pub precision: TimePrecision,
    /// Human-readable display text (for LLM/UI, not for sorting)
    pub display_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimePrecision {
    Exact,
    Day,
    Period,
    Era,
}

/// Session kind for AgentSession
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentSessionKind {
    /// Current mainline session
    Mainline,
    /// Past timeline session (period_anchor < mainline_time_anchor)
    Retrospective,
    /// Future preview session (period_anchor > mainline_time_anchor, default non-canon)
    FuturePreview,
}

/// Canon status for AgentSession (whole session level)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionCanonStatus {
    /// No hard conflict yet, candidate details can be promoted after validation
    CanonCandidate,
    /// Pre-conflict can be promoted, conflict turn and after are non-canon
    PartiallyCanon,
    /// Whole session is non-canon
    NonCanon,
}

/// Canon status for SessionTurn (chat message level)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionTurnCanonStatus {
    CanonCandidate,
    CanonPromoted,
    ConflictWarned,
    NonCanon,
}

/// Canon status for WorldTurns (runtime turn level)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeTurnCanonStatus {
    /// Current mainline canonical turn; must have state_commit_records
    Canon,
    /// Past timeline candidate promoted to canonical; must have state_commit_records
    ProvisionalPromoted,
    /// Past timeline candidate runtime turn; can link Trace/Logs/provisional truth, but no canonical commit
    ProvisionalOnly,
    /// Non-canon runtime turn; cannot commit canonical
    NonCanon,
    /// Future preview runtime turn; cannot commit canonical
    FuturePreview,
}

/// Conflict policy decision for past timeline sessions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictPolicyDecision {
    /// Conflict turn and after are non-canon
    NonCanonAfterConflict,
    /// Whole session becomes non-canon
    WholeSessionNonCanon,
}

/// Player mode for AgentSession
///
/// This is a session-level permission boundary, not a UI helper field:
/// - Character: player_character_id is required, must reference a valid character in the World
/// - Director: player_character_id must be empty; director input can only go through
///   SceneNarration / DirectorHint / MetaCommand, cannot directly write any NPC's IntentPlan or L3 inner state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PlayerMode {
    /// Player roleplays as a specific character in the World
    #[default]
    Character,
    /// Player acts as a world-external director, not directly roleplaying any character
    Director,
}

/// Fact confidence level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactConfidence {
    Asserted,
    High,
    Medium,
    Low,
    Inferred,
}

/// Fact source tracking
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactSource {
    AuthorDefined,
    Imported,
    Derived { from_knowledge_ids: Vec<String> },
    LlmGenerated { scene_turn_id: String },
    UserConfirmed,
}

/// Schema version for data structures
pub const SCHEMA_VERSION: &str = "0.1";

/// Generate a unique ID with prefix
pub fn generate_id(prefix: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let random = rand_u32();
    format!("{}_{:x}_{:x}", prefix, timestamp, random)
}

/// Simple random u32 for ID generation
fn rand_u32() -> u32 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;

    let mut hasher = DefaultHasher::new();
    SystemTime::now().hash(&mut hasher);
    hasher.finish() as u32
}
