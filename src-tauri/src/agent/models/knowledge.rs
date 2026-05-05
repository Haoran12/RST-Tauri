//! Knowledge models - Layer 1 Truth Store
//!
//! KnowledgeEntry, AccessPolicy, SubjectAwareness, TruthGuidance, KnowledgeRevealEvent

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::common::*;
use super::scene::ManaAttribute; // Used in ManaHaze and KnowledgeEntry content

/// Knowledge entry - unified knowledge model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub knowledge_id: String,
    pub kind: KnowledgeKind,
    pub subject: KnowledgeSubject,
    /// Objective truth (structured)
    pub content: serde_json::Value,
    /// Apparent content (for observers when hidden/disguised)
    pub apparent_content: Option<serde_json::Value>,
    pub access_policy: AccessPolicy,
    pub subject_awareness: SubjectAwareness,
    pub metadata: KnowledgeMetadata,
    pub valid_from: Option<TimeAnchor>,
    pub valid_until: Option<TimeAnchor>,
    pub source_session_id: Option<String>,
    pub source_scene_turn_id: Option<String>,
    pub derived_from_event_id: Option<String>,
    pub schema_version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnowledgeKind {
    /// World-level facts (cosmic rules, cultivation system)
    WorldFact,
    /// Location/region facts (geography, customs, climate, bans)
    RegionFact,
    /// Faction facts (sect rules, techniques)
    FactionFact,
    /// Character facet (appearance, identity, abilities, etc.)
    CharacterFacet,
    /// Historical event constraints (for past timeline guidance)
    HistoricalEvent,
    /// Memory (witnessed or heard)
    Memory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KnowledgeSubject {
    World,
    Region(String),
    Faction(String),
    Character {
        id: String,
        facet: CharacterFacetType,
    },
    Event {
        event_id: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CharacterFacetType {
    Appearance,
    Identity,
    TrueName,
    Species,
    Bloodline,
    CultivationRealm,
    KnownAbility,
    HiddenAbility,
    Personality,
    Background,
    Motivation,
    Trauma,
    MindModelCard,
}

/// Access policy - three predicates (OR relationship)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPolicy {
    /// Name-list access (character IDs who can access)
    pub known_by: Vec<String>,
    /// Scope-based access
    pub scope: Vec<AccessScope>,
    /// Condition-based access (runtime evaluation)
    pub conditions: Vec<AccessCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessScope {
    /// All inhabitants can access
    Public,
    /// Only orchestrator can access (hard deny for all characters)
    GodOnly,
    /// Characters in this region
    Region(String),
    /// Members of this faction
    Faction(String),
    /// Cultivation at this level or above
    Realm(String),
    /// Characters with this role
    Role(String),
    /// Characters with this bloodline
    Bloodline(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessCondition {
    /// In same scene and can observe
    InSameSceneObservable,
    /// Social access level threshold
    SocialAccessAtLeast { target: String, threshold: f64 },
    /// Has specific skill
    HasSkill(String),
    /// Cultivation at least level
    CultivationAtLeast(String),
    /// Custom predicate (structured DSL)
    CustomPredicate(AccessExpression),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessExpression {
    All(Vec<AccessExpression>),
    Any(Vec<AccessExpression>),
    Not(Box<AccessExpression>),
    HasTag { subject_id: String, tag: String },
    NumericAtLeast { path: String, value: f64 },
    BooleanFlag { path: String, expected: bool },
}

/// Subject awareness - for CharacterFacet knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubjectAwareness {
    /// Subject knows about this facet (content is accessible)
    Aware,
    /// Subject doesn't know the truth, but has a self-belief
    Unaware { self_belief: serde_json::Value },
}

/// Knowledge metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeMetadata {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub valid_from: Option<TimeAnchor>,
    pub valid_until: Option<TimeAnchor>,
    pub source_session_id: Option<String>,
    pub source_scene_turn_id: Option<String>,
    pub derived_from_event_id: Option<String>,
    /// Memory-specific fields
    pub emotional_weight: Option<f64>,
    pub last_accessed_at: Option<DateTime<Utc>>,
    pub source: Option<MemorySource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemorySource {
    Witnessed,
    ToldBy,
    Inferred,
}

/// Truth guidance for past timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruthGuidance {
    pub session_id: String,
    pub period_anchor: TimeAnchor,
    pub related_event_ids: Vec<String>,
    pub hard_constraints: Vec<TruthConstraint>,
    pub soft_context: Vec<String>,
    pub open_detail_slots: Vec<OpenDetailSlot>,
    pub future_knowledge_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruthConstraint {
    pub constraint_id: String,
    pub source_knowledge_id: String,
    pub constraint_kind: TruthConstraintKind,
    pub applies_to_refs: Vec<String>,
    pub structured_payload: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TruthConstraintKind {
    RequiredOutcome,
    ForbiddenOutcome,
    KnownAfterEffect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenDetailSlot {
    pub slot_id: String,
    pub source_event_id: String,
    pub detail_kind: DetailKind,
    pub promotion_policy: PromotionPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetailKind {
    Motive,
    Dialogue,
    Witness,
    Route,
    LocalCause,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromotionPolicy {
    PromoteIfConsistent,
    TraceOnly,
}

/// Knowledge reveal event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeRevealEvent {
    pub event_id: String,
    pub knowledge_id: String,
    pub newly_known_by: Vec<String>,
    pub trigger: RevealTrigger,
    pub scope_change: Option<AccessScopeChange>,
    pub scene_turn_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RevealTrigger {
    Witnessed,
    Told { by_character_id: String },
    Inferred { from_knowledge_ids: Vec<String> },
    Awakened,
    Scripted { event_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessScopeChange {
    RemoveGodOnly,
    ReplaceScopes(Vec<AccessScope>),
}

/// Accessible knowledge entry (Layer 2 view)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibleEntry {
    pub knowledge_id: String,
    pub kind: KnowledgeKind,
    pub subject: KnowledgeSubject,
    /// Content after access control filtering
    pub accessible_content: serde_json::Value,
    pub source_hint: AccessSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessSource {
    InKnownBy,
    ScopeMatch(String),
    ConditionMet(String),
    SelfFacetAware,
    SelfFacetBelief,
    ApparentFromObservation,
}

/// Accessible knowledge (Layer 2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibleKnowledge {
    pub character_id: String,
    pub scene_turn_id: String,
    pub entries: Vec<AccessibleEntry>,
}

// ===== Historical Event Content Schema =====

/// Historical event content - for KnowledgeEntry with kind = HistoricalEvent
///
/// Represents canonical event constraints for past timeline guidance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalEventContent {
    /// Human-readable summary
    pub summary_text: String,
    /// Unique event identifier
    pub event_id: String,
    /// Time window when the event occurred
    pub time_window: EventTimeWindow,
    /// Participants and their roles
    pub participants: Vec<EventParticipant>,
    /// Outcomes that must happen
    pub required_outcomes: Vec<EventOutcome>,
    /// Outcomes that cannot happen
    pub forbidden_outcomes: Vec<EventOutcome>,
    /// Facts that become true after this event
    pub known_after_effects: Vec<AfterEffect>,
    /// Detail slots that can be filled in past timeline
    pub open_detail_slots: Vec<OpenDetailSlotRef>,
    /// Extension fields
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

/// Event time window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTimeWindow {
    pub start: EventTimePoint,
    pub end: Option<EventTimePoint>,
}

/// Point in event time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTimePoint {
    pub calendar_id: String,
    pub ordinal: i64,
    pub precision: super::common::TimePrecision,
    pub display_text: String,
}

/// Event participant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventParticipant {
    pub character_id: String,
    pub role: String,
    pub optional: bool,
}

/// Event outcome constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventOutcome {
    pub outcome_id: String,
    pub domain: OutcomeDomain,
    pub subject_id: String,
    pub target_id: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutcomeDomain {
    Relationship,
    ItemState,
    CharacterLifeState,
    LocationState,
    KnowledgeState,
    EventNegation,
}

/// After effect of an event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfterEffect {
    pub fact_ref: String,
    pub valid_from_ordinal: i64,
}

/// Reference to an open detail slot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenDetailSlotRef {
    pub slot_id: String,
    pub detail_kind: DetailKind,
    pub promotion_policy: PromotionPolicy,
}

// ===== Character Facet Content Schemas =====

/// Character appearance content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterAppearanceContent {
    pub summary_text: String,
    pub height: Option<String>,
    pub build: Option<String>,
    pub hair: Option<HairDescription>,
    pub distinctive_marks: Vec<String>,
    pub clothing: Option<ClothingDescription>,
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HairDescription {
    pub color: String,
    pub style: String,
    pub length: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClothingDescription {
    pub upper: Option<String>,
    pub lower: Option<String>,
    pub accessories: Vec<String>,
}

/// Character cultivation realm content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CultivationRealmContent {
    pub summary_text: String,
    pub realm: String,
    pub stage: String,
    pub progress_within_stage: Option<f64>,
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

/// Character ability content (known or hidden)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterAbilityContent {
    pub summary_text: String,
    pub ability_id: String,
    pub category: String,
    pub trigger_condition: Option<String>,
    pub power_level: Option<String>,
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

/// Character mind model card content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MindModelCardContent {
    pub summary_text: String,
    pub attention_biases: Vec<String>,
    pub risk_tolerance: RiskTolerance,
    pub default_social_strategy: String,
    pub value_priorities: Vec<String>,
    pub cognitive_patterns: Vec<String>,
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskTolerance {
    VeryLow,
    Low,
    Moderate,
    High,
    VeryHigh,
}

// ===== Region Fact Content Schema =====

/// Region fact content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionFactContent {
    pub summary_text: String,
    pub fact_type: String,
    pub applies_to_location_id: String,
    pub inheritance: Option<FactInheritance>,
    pub confidence: FactConfidence,
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

/// Fact inheritance rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactInheritance {
    pub inheritable: bool,
    pub applies_to_descendants: bool,
    pub max_depth: Option<u32>,
    pub blocked_location_ids: Vec<String>,
    pub override_policy: InheritanceOverridePolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InheritanceOverridePolicy {
    ChildOverridesParent,
    ParentOverridesChild,
    Merge,
}

// ===== Memory Content Schema =====

/// Memory content - for KnowledgeEntry with kind = Memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryContent {
    pub summary_text: String,
    pub event_type: String,
    pub actor: Option<String>,
    pub target: Option<String>,
    pub location: Option<String>,
    pub timestamp: Option<String>,
    pub key_observations: Vec<String>,
    pub emotional_weight: Option<f64>,
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

// ===== Faction Fact Content Schema =====

/// Faction fact content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionFactContent {
    pub summary_text: String,
    pub rule_id: Option<String>,
    pub applies_to: Option<FactionAppliesTo>,
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionAppliesTo {
    pub role: Option<String>,
    pub rank: Option<String>,
    pub member_ids: Vec<String>,
}
