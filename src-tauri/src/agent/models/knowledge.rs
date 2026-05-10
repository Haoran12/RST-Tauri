//! Knowledge models - Layer 1 Truth Store
//!
//! KnowledgeEntry, AccessPolicy, SubjectAwareness, TruthGuidance, KnowledgeRevealEvent

use chrono::{DateTime, Utc};
use serde::{de, Deserialize, Deserializer, Serialize};

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
    #[serde(default)]
    pub known_by: Vec<String>,
    /// Scope-based access
    #[serde(default)]
    pub scope: Vec<AccessScope>,
    /// Condition-based access (runtime evaluation)
    #[serde(default)]
    pub conditions: Vec<AccessCondition>,
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Clone, Serialize)]
pub enum SubjectAwareness {
    /// Subject knows about this facet (content is accessible)
    Aware,
    /// Subject doesn't know the truth, but has a self-belief
    Unaware { self_belief: serde_json::Value },
}

/// Knowledge metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeMetadata {
    #[serde(default = "default_utc_now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "default_utc_now")]
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub valid_from: Option<TimeAnchor>,
    #[serde(default)]
    pub valid_until: Option<TimeAnchor>,
    #[serde(default)]
    pub source_session_id: Option<String>,
    #[serde(default)]
    pub source_scene_turn_id: Option<String>,
    #[serde(default)]
    pub derived_from_event_id: Option<String>,
    /// Memory-specific fields
    #[serde(default)]
    pub emotional_weight: Option<f64>,
    #[serde(default)]
    pub last_accessed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub source: Option<MemorySource>,
}

impl<'de> Deserialize<'de> for AccessScope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer).map_err(de::Error::custom)?;
        parse_access_scope_value(value).map_err(de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for AccessCondition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer).map_err(de::Error::custom)?;
        parse_access_condition_value(value).map_err(de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for SubjectAwareness {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer).map_err(de::Error::custom)?;
        parse_subject_awareness_value(value).map_err(de::Error::custom)
    }
}

fn default_utc_now() -> DateTime<Utc> {
    Utc::now()
}

fn parse_access_scope_value(value: serde_json::Value) -> Result<AccessScope, String> {
    match value {
        serde_json::Value::String(kind) => match kind.as_str() {
            "Public" => Ok(AccessScope::Public),
            "GodOnly" => Ok(AccessScope::GodOnly),
            other => Err(format!(
                "unknown variant `{}`, expected one of `Public`, `GodOnly`, `Region`, `Faction`, `Realm`, `Role`, `Bloodline`",
                other
            )),
        },
        serde_json::Value::Object(mut map) => {
            if let Some(kind) = map
                .remove("type")
                .and_then(|value| value.as_str().map(|s| s.to_string()))
            {
                return parse_access_scope_kind(&kind, map.remove("value"));
            }

            if map.len() == 1 {
                let (kind, payload) = map.into_iter().next().unwrap();
                return parse_access_scope_kind(&kind, Some(payload));
            }

            Err("expected access scope string or single-variant object".to_string())
        }
        other => Err(format!(
            "expected access scope string or object, got {}",
            other
        )),
    }
}

fn parse_access_scope_kind(
    kind: &str,
    payload: Option<serde_json::Value>,
) -> Result<AccessScope, String> {
    let payload_text = match payload {
        Some(serde_json::Value::String(value)) => value,
        Some(serde_json::Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    };

    match kind {
        "Public" => Ok(AccessScope::Public),
        "GodOnly" => Ok(AccessScope::GodOnly),
        "Region" => Ok(AccessScope::Region(payload_text)),
        "Faction" => Ok(AccessScope::Faction(payload_text)),
        "Realm" => Ok(AccessScope::Realm(payload_text)),
        "Role" => Ok(AccessScope::Role(payload_text)),
        "Bloodline" => Ok(AccessScope::Bloodline(payload_text)),
        other => Err(format!(
            "unknown variant `{}`, expected one of `Public`, `GodOnly`, `Region`, `Faction`, `Realm`, `Role`, `Bloodline`",
            other
        )),
    }
}

fn parse_access_condition_value(value: serde_json::Value) -> Result<AccessCondition, String> {
    match value {
        serde_json::Value::String(kind) => parse_access_condition_kind(&kind, None),
        serde_json::Value::Object(mut map) => {
            if let Some(kind) = map
                .remove("kind")
                .and_then(|value| value.as_str().map(|s| s.to_string()))
            {
                let payload = map.remove("payload").or_else(|| {
                    if map.is_empty() {
                        None
                    } else {
                        Some(serde_json::Value::Object(map))
                    }
                });
                return parse_access_condition_kind(&kind, payload);
            }

            if map.len() == 1 {
                let (kind, payload) = map.into_iter().next().unwrap();
                return parse_access_condition_kind(&kind, Some(payload));
            }

            Err("expected access condition string or single-variant object".to_string())
        }
        other => Err(format!(
            "expected access condition string or object, got {}",
            other
        )),
    }
}

fn parse_access_condition_kind(
    kind: &str,
    payload: Option<serde_json::Value>,
) -> Result<AccessCondition, String> {
    match kind {
        "InSameSceneObservable" => Ok(AccessCondition::InSameSceneObservable),
        "HasSkill" => Ok(AccessCondition::HasSkill(extract_payload_string(payload))),
        "CultivationAtLeast" => Ok(AccessCondition::CultivationAtLeast(
            extract_payload_string(payload),
        )),
        "SocialAccessAtLeast" => {
            let payload = payload.unwrap_or_else(|| serde_json::json!({}));
            let object = payload.as_object().cloned().unwrap_or_default();
            Ok(AccessCondition::SocialAccessAtLeast {
                target: object
                    .get("target")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                threshold: object
                    .get("threshold")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or_default(),
            })
        }
        "CustomPredicate" => {
            let payload = payload.ok_or_else(|| {
                "CustomPredicate requires a structured payload".to_string()
            })?;
            let expression = serde_json::from_value(payload)
                .map_err(|e| format!("invalid CustomPredicate payload: {}", e))?;
            Ok(AccessCondition::CustomPredicate(expression))
        }
        other => Err(format!(
            "unknown variant `{}`, expected one of `InSameSceneObservable`, `SocialAccessAtLeast`, `HasSkill`, `CultivationAtLeast`, `CustomPredicate`",
            other
        )),
    }
}

fn extract_payload_string(payload: Option<serde_json::Value>) -> String {
    match payload {
        Some(serde_json::Value::String(value)) => value,
        Some(serde_json::Value::Object(mut object)) => object
            .remove("value")
            .and_then(|value| value.as_str().map(|s| s.to_string()))
            .unwrap_or_default(),
        Some(serde_json::Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

fn parse_subject_awareness_value(value: serde_json::Value) -> Result<SubjectAwareness, String> {
    match value {
        serde_json::Value::String(kind) => match kind.as_str() {
            "Aware" => Ok(SubjectAwareness::Aware),
            "Unaware" => Ok(SubjectAwareness::Unaware {
                self_belief: serde_json::Value::Null,
            }),
            other => Err(format!(
                "unknown variant `{}`, expected one of `Aware`, `Unaware`",
                other
            )),
        },
        serde_json::Value::Object(mut map) => {
            if let Some(kind) = map
                .remove("kind")
                .and_then(|value| value.as_str().map(|s| s.to_string()))
            {
                return match kind.as_str() {
                    "Aware" => Ok(SubjectAwareness::Aware),
                    "Unaware" => Ok(SubjectAwareness::Unaware {
                        self_belief: map.remove("self_belief").unwrap_or(serde_json::Value::Null),
                    }),
                    other => Err(format!(
                        "unknown variant `{}`, expected one of `Aware`, `Unaware`",
                        other
                    )),
                };
            }

            if map.contains_key("Aware") {
                return Ok(SubjectAwareness::Aware);
            }

            if let Some(payload) = map.remove("Unaware") {
                let self_belief = match payload {
                    serde_json::Value::Object(mut object) => object
                        .remove("self_belief")
                        .unwrap_or(serde_json::Value::Null),
                    other => other,
                };
                return Ok(SubjectAwareness::Unaware { self_belief });
            }

            Err("expected subject awareness string or variant object".to_string())
        }
        other => Err(format!(
            "expected subject awareness string or object, got {}",
            other
        )),
    }
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

#[cfg(test)]
mod tests {
    use super::{AccessCondition, AccessPolicy, AccessScope, KnowledgeMetadata, SubjectAwareness};
    use serde_json::json;

    #[test]
    fn access_policy_accepts_world_editor_scope_shape() {
        let policy: AccessPolicy = serde_json::from_value(json!({
            "known_by": ["character_a"],
            "scope": [
                { "type": "Public" },
                { "type": "Region", "value": "region_001" },
                { "Faction": "faction_001" }
            ],
            "conditions": []
        }))
        .expect("policy should deserialize");

        assert_eq!(policy.known_by, vec!["character_a"]);
        assert!(matches!(policy.scope[0], AccessScope::Public));
        assert!(matches!(policy.scope[1], AccessScope::Region(ref value) if value == "region_001"));
        assert!(
            matches!(policy.scope[2], AccessScope::Faction(ref value) if value == "faction_001")
        );
    }

    #[test]
    fn subject_awareness_accepts_editor_shape() {
        let awareness: SubjectAwareness = serde_json::from_value(json!({
            "kind": "Unaware",
            "self_belief": {
                "summary_text": "误以为自己毫无灵力"
            }
        }))
        .expect("subject awareness should deserialize");

        assert!(matches!(
            awareness,
            SubjectAwareness::Unaware { ref self_belief }
                if self_belief.get("summary_text").and_then(|value| value.as_str()) == Some("误以为自己毫无灵力")
        ));
    }

    #[test]
    fn knowledge_metadata_defaults_missing_optional_fields() {
        let metadata: KnowledgeMetadata = serde_json::from_value(json!({
            "created_at": "2026-05-10T12:00:00Z",
            "updated_at": "2026-05-10T12:30:00Z"
        }))
        .expect("metadata should deserialize");

        assert_eq!(metadata.source_session_id, None);
        assert_eq!(metadata.derived_from_event_id, None);
        assert_eq!(metadata.emotional_weight, None);
        assert_eq!(metadata.last_accessed_at, None);
    }

    #[test]
    fn access_condition_accepts_editor_shape() {
        let condition: AccessCondition = serde_json::from_value(json!({
            "kind": "SocialAccessAtLeast",
            "payload": {
                "target": "character_b",
                "threshold": 0.75
            }
        }))
        .expect("condition should deserialize");

        assert!(matches!(
            condition,
            AccessCondition::SocialAccessAtLeast { ref target, threshold }
                if target == "character_b" && (threshold - 0.75).abs() < f64::EPSILON
        ));
    }
}
