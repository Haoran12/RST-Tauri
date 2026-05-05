//! Skill models
//!
//! Skill, SkillEffectContract, SkillActivation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::character::{CostProfile, EffectIntensityTier, TargetKind};
use super::common::*;
use super::knowledge::{
    CharacterAbilityContent, CharacterFacetType, KnowledgeEntry, KnowledgeSubject,
};
use super::scene::ManaAttribute;

/// Skill definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub skill_id: String,
    pub name: String,
    pub description: String,
    pub skill_kind: SkillKind,
    pub activation: SkillActivation,
    pub effect_contract: SkillEffectContract,
    pub requirements: SkillRequirements,
    pub metadata: SkillMetadata,
    pub schema_version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillKind {
    /// Active skill - requires activation
    Active,
    /// Passive skill - always active
    Passive,
    /// Reaction skill - triggered by events
    Reaction,
    /// Stance - modifies other skills
    Stance,
    /// Ritual - extended activation
    Ritual,
}

/// Skill activation conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillActivation {
    pub activation_time: ActivationTime,
    pub trigger_conditions: Vec<ActivationCondition>,
    pub cooldown: Option<u32>,
    pub uses_per_scene: Option<u32>,
    pub uses_per_day: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivationTime {
    Instant,
    FullTurn,
    MultiTurn(u32),
    Reaction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivationCondition {
    ManaReserveAtLeast(f64),
    FatigueBelow(f64),
    InCombat,
    OutOfCombat,
    HasStatus(String),
    LacksStatus(String),
    TargetInRange(f64),
    TargetInLineOfSight,
    EnvironmentCondition(String),
    Custom(String),
}

/// Skill effect contract - declares what the skill does
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEffectContract {
    pub primary_effects: Vec<SkillEffect>,
    pub secondary_effects: Vec<SkillEffect>,
    pub target_kind: TargetKind,
    pub target_count: TargetCount,
    pub range_m: Option<f64>,
    pub area_of_effect: Option<AreaOfEffect>,
    pub duration_turns: Option<u32>,
    pub attribute_modifier: Option<AttributeModifier>,
    pub mana_attribute: Option<ManaAttribute>,
    // Validation fields
    pub allowed_target_kinds: Vec<TargetKind>,
    pub allowed_state_domains: Vec<String>,
    pub max_intensity_tier: EffectIntensityTier,
    pub allows_injury: bool,
    pub allows_position_change: bool,
    pub allows_knowledge_reveal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEffect {
    pub effect_id: String,
    pub effect_kind: SkillEffectKind,
    pub intensity_tier: EffectIntensityTier,
    pub target_domain: String,
    pub description: String,
    pub numeric_value: Option<f64>,
    pub applies_to: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillEffectKind {
    Damage,
    Healing,
    Buff,
    Debuff,
    StatusApply,
    StatusRemove,
    Movement,
    Summon,
    TerrainChange,
    ManaFieldChange,
    KnowledgeReveal,
    Social,
    Utility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetCount {
    Single,
    Multi(u32),
    Area,
    AllValid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AreaOfEffect {
    pub shape: AreaShape,
    pub radius_m: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AreaShape {
    Circle,
    Cone,
    Line,
    Cube,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeModifier {
    pub attribute: String,
    pub modifier_kind: AttributeModifierKind,
    pub value: f64,
    pub duration_turns: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttributeModifierKind {
    Add,
    Multiply,
    Set,
    TempAdd,
    TempMultiply,
}

/// Skill requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRequirements {
    pub minimum_attributes: Vec<(String, f64)>,
    pub required_skills: Vec<String>,
    pub required_knowledge: Vec<String>,
    pub prohibited_conditions: Vec<String>,
    pub material_components: Vec<String>,
    pub cost: CostProfile,
}

/// Skill metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub tags: Vec<String>,
    pub source: Option<String>,
    pub learning_difficulty: LearningDifficulty,
    pub rarity: SkillRarity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LearningDifficulty {
    Common,
    Uncommon,
    Rare,
    Legendary,
    Unique,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillRarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

/// Skill instance - a character's learned skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInstance {
    pub instance_id: String,
    pub character_id: String,
    pub skill_id: String,
    pub proficiency: f64,
    pub customizations: Vec<SkillCustomization>,
    pub acquired_at: TimeAnchor,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCustomization {
    pub customization_id: String,
    pub kind: SkillCustomizationKind,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillCustomizationKind {
    VariantSelection,
    Enhancement,
    FlavorText,
    Renamed,
}

impl Skill {
    /// Build a runtime-usable skill from a CharacterFacet ability entry.
    ///
    /// Ability facets can either carry a full skill definition under
    /// `content.extensions.skill{,_definition,runtime_skill}` or expose the
    /// skill fragment directly on `content.extensions`.
    pub fn from_character_ability_entry(entry: &KnowledgeEntry) -> Result<Option<Self>, String> {
        let (character_id, facet_type) = match &entry.subject {
            KnowledgeSubject::Character { id, facet } => (id.as_str(), *facet),
            _ => return Ok(None),
        };

        if !matches!(
            facet_type,
            CharacterFacetType::KnownAbility | CharacterFacetType::HiddenAbility
        ) {
            return Ok(None);
        }

        let ability_content: CharacterAbilityContent =
            serde_json::from_value(entry.content.clone()).map_err(|error| {
                format!(
                    "Failed to parse character ability content for {}: {}",
                    entry.knowledge_id, error
                )
            })?;

        let skill_value = ability_content
            .extensions
            .get("skill")
            .or_else(|| ability_content.extensions.get("skill_definition"))
            .or_else(|| ability_content.extensions.get("runtime_skill"))
            .cloned()
            .or_else(|| {
                has_skill_fragment_fields_in_map(&ability_content.extensions)
                    .then(|| Value::Object(ability_content.extensions.clone()))
            })
            .or_else(|| has_skill_fragment_fields(&entry.content).then(|| entry.content.clone()));

        let Some(skill_value) = skill_value else {
            return Ok(None);
        };

        if let Ok(mut skill) = serde_json::from_value::<Skill>(skill_value.clone()) {
            attach_owner_tag(&mut skill.metadata, character_id);
            if skill.skill_id.trim().is_empty() {
                skill.skill_id = ability_content.ability_id.clone();
            }
            if skill.name.trim().is_empty() {
                skill.name = ability_content.summary_text.clone();
            }
            if skill.description.trim().is_empty() {
                skill.description = ability_content.summary_text.clone();
            }
            return Ok(Some(skill));
        }

        let skill_object = skill_value.as_object().ok_or_else(|| {
            format!(
                "Ability skill fragment for {} must be an object",
                entry.knowledge_id
            )
        })?;

        let skill_kind = skill_object
            .get("skill_kind")
            .cloned()
            .map(serde_json::from_value::<SkillKind>)
            .transpose()
            .map_err(|error| format!("Invalid skill_kind for {}: {}", entry.knowledge_id, error))?
            .unwrap_or_else(|| default_skill_kind(ability_content.trigger_condition.as_deref()));

        let activation = skill_object
            .get("activation")
            .cloned()
            .map(serde_json::from_value::<SkillActivation>)
            .transpose()
            .map_err(|error| format!("Invalid activation for {}: {}", entry.knowledge_id, error))?
            .unwrap_or_else(|| {
                default_skill_activation(ability_content.trigger_condition.as_deref())
            });

        let effect_contract_value = skill_object
            .get("effect_contract")
            .cloned()
            .unwrap_or_else(|| Value::Object(skill_object.clone()));
        let effect_contract: SkillEffectContract = serde_json::from_value(effect_contract_value)
            .map_err(|error| {
                format!(
                    "Missing or invalid effect_contract for ability {}: {}",
                    entry.knowledge_id, error
                )
            })?;

        let requirements = skill_object
            .get("requirements")
            .cloned()
            .map(serde_json::from_value::<SkillRequirements>)
            .transpose()
            .map_err(|error| format!("Invalid requirements for {}: {}", entry.knowledge_id, error))?
            .unwrap_or_default();

        let mut metadata = skill_object
            .get("metadata")
            .cloned()
            .map(serde_json::from_value::<SkillMetadata>)
            .transpose()
            .map_err(|error| format!("Invalid metadata for {}: {}", entry.knowledge_id, error))?
            .unwrap_or_else(|| SkillMetadata {
                tags: Vec::new(),
                source: Some(entry.knowledge_id.clone()),
                learning_difficulty: LearningDifficulty::Common,
                rarity: SkillRarity::Common,
            });
        attach_owner_tag(&mut metadata, character_id);

        Ok(Some(Skill {
            skill_id: skill_object
                .get("skill_id")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(ability_content.ability_id.as_str())
                .to_string(),
            name: skill_object
                .get("name")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(ability_content.summary_text.as_str())
                .to_string(),
            description: skill_object
                .get("description")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(ability_content.summary_text.as_str())
                .to_string(),
            skill_kind,
            activation,
            effect_contract,
            requirements,
            metadata,
            schema_version: skill_object
                .get("schema_version")
                .and_then(Value::as_str)
                .unwrap_or(entry.schema_version.as_str())
                .to_string(),
            created_at: entry.created_at.to_owned(),
            updated_at: entry.updated_at.to_owned(),
        }))
    }

    pub fn owner_character_id(&self) -> Option<&str> {
        self.metadata
            .tags
            .iter()
            .find_map(|tag| tag.strip_prefix("owner:"))
            .or_else(|| self.skill_id.split_once(':').map(|(owner, _)| owner))
    }

    pub fn belongs_to_character(&self, character_id: &str) -> bool {
        self.owner_character_id() == Some(character_id)
    }
}

impl Default for SkillRequirements {
    fn default() -> Self {
        Self {
            minimum_attributes: Vec::new(),
            required_skills: Vec::new(),
            required_knowledge: Vec::new(),
            prohibited_conditions: Vec::new(),
            material_components: Vec::new(),
            cost: CostProfile::default(),
        }
    }
}

fn has_skill_fragment_fields(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(has_skill_fragment_fields_in_map)
}

fn has_skill_fragment_fields_in_map(map: &serde_json::Map<String, Value>) -> bool {
    map.contains_key("effect_contract")
        || map.contains_key("skill_kind")
        || map.contains_key("activation")
        || map.contains_key("requirements")
        || map.contains_key("metadata")
}

fn attach_owner_tag(metadata: &mut SkillMetadata, character_id: &str) {
    let owner_tag = format!("owner:{character_id}");
    if !metadata.tags.iter().any(|tag| tag == &owner_tag) {
        metadata.tags.push(owner_tag);
    }
}

fn default_skill_kind(trigger_condition: Option<&str>) -> SkillKind {
    if trigger_condition.is_some_and(|condition| condition.to_ascii_lowercase().contains("passive"))
    {
        SkillKind::Passive
    } else if trigger_condition.is_some_and(|condition| {
        let lower = condition.to_ascii_lowercase();
        lower.contains("interrupt") || lower.contains("reaction")
    }) {
        SkillKind::Reaction
    } else {
        SkillKind::Active
    }
}

fn default_skill_activation(trigger_condition: Option<&str>) -> SkillActivation {
    SkillActivation {
        activation_time: match default_skill_kind(trigger_condition) {
            SkillKind::Reaction => ActivationTime::Reaction,
            _ => ActivationTime::Instant,
        },
        trigger_conditions: Vec::new(),
        cooldown: None,
        uses_per_scene: None,
        uses_per_day: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::models::{
        AccessPolicy, CharacterAbilityContent, KnowledgeKind, KnowledgeMetadata, SubjectAwareness,
    };

    fn ability_entry(content: serde_json::Value) -> KnowledgeEntry {
        KnowledgeEntry {
            knowledge_id: "know-1".to_string(),
            kind: KnowledgeKind::CharacterFacet,
            subject: KnowledgeSubject::Character {
                id: "char-1".to_string(),
                facet: CharacterFacetType::KnownAbility,
            },
            content,
            apparent_content: None,
            access_policy: AccessPolicy {
                known_by: vec!["char-1".to_string()],
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
        }
    }

    #[test]
    fn builds_skill_from_nested_extension_fragment() {
        let mut extensions = serde_json::Map::new();
        extensions.insert(
            "skill".to_string(),
            serde_json::json!({
                "skill_kind": "Reaction",
                "activation": {
                    "activation_time": "Reaction",
                    "trigger_conditions": [],
                    "cooldown": 1,
                    "uses_per_scene": null,
                    "uses_per_day": null
                },
                "effect_contract": {
                    "primary_effects": [],
                    "secondary_effects": [],
                    "target_kind": "Character",
                    "target_count": "Single",
                    "range_m": 8.0,
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
            }),
        );
        let content = serde_json::to_value(CharacterAbilityContent {
            summary_text: "Void Counter".to_string(),
            ability_id: "void_counter".to_string(),
            category: "combat".to_string(),
            trigger_condition: Some("reaction".to_string()),
            power_level: None,
            extensions,
        })
        .expect("ability content json");

        let skill = Skill::from_character_ability_entry(&ability_entry(content))
            .expect("skill parse")
            .expect("skill exists");

        assert_eq!(skill.skill_id, "void_counter");
        assert_eq!(skill.skill_kind, SkillKind::Reaction);
        assert!(skill.belongs_to_character("char-1"));
    }

    #[test]
    fn builds_skill_from_flat_extension_fragment() {
        let content = serde_json::json!({
            "summary_text": "Protective Ward",
            "ability_id": "protective_ward",
            "category": "field",
            "trigger_condition": "passive_field",
            "power_level": null,
            "extensions": {
                "skill_kind": "Passive",
                "effect_contract": {
                    "primary_effects": [],
                    "secondary_effects": [],
                    "target_kind": "Area",
                    "target_count": "Area",
                    "range_m": 12.0,
                    "area_of_effect": null,
                    "duration_turns": null,
                    "attribute_modifier": null,
                    "mana_attribute": null,
                    "allowed_target_kinds": ["Area", "Character"],
                    "allowed_state_domains": ["scene", "body"],
                    "max_intensity_tier": "Moderate",
                    "allows_injury": false,
                    "allows_position_change": false,
                    "allows_knowledge_reveal": false
                }
            }
        });

        let skill = Skill::from_character_ability_entry(&ability_entry(content))
            .expect("skill parse")
            .expect("skill exists");

        assert_eq!(skill.skill_id, "protective_ward");
        assert_eq!(skill.skill_kind, SkillKind::Passive);
        assert!(skill.belongs_to_character("char-1"));
    }
}
