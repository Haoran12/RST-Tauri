//! Character models - Layer 1 Truth Store
//!
//! CharacterRecord, BaseAttributes, TemporaryCharacterState, ManaExpressionState

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::common::*;
use super::scene::{ManaAttribute, ManaExpressionMode, ManaPresenceRadiusTier};

/// Character record - basic character data (Layer 1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterRecord {
    pub character_id: String,
    /// Six base attributes (f64 for calculation, UI shows integer)
    pub base_attributes: BaseAttributes,
    /// Baseline body profile (species, senses, mana sense)
    pub baseline_body_profile: BaselineBodyProfile,
    /// Long-term mana expression tendency
    pub mana_expression_tendency: ManaExpressionTendency,
    /// Optional character-specific tendency factor override
    pub mana_expression_tendency_factor_override: Option<f64>,
    /// Reference to MindModelCard knowledge entry
    pub mind_model_card_knowledge_id: String,
    /// Current temporary state (Layer 1)
    pub temporary_state: TemporaryCharacterState,
    pub schema_version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Six base attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseAttributes {
    /// Physical - body mass, burst, carry, grapple
    pub physical: f64,
    /// Agility - coordination, dodge, fine motor
    pub agility: f64,
    /// Endurance - sustained action, pain resistance
    pub endurance: f64,
    /// Insight - perception, tactical reading
    pub insight: f64,
    /// Mana power - spiritual/magical strength
    pub mana_power: f64,
    /// Soul strength - mental/spiritual resilience
    pub soul_strength: f64,
}

impl BaseAttributes {
    pub fn mundane_default() -> Self {
        Self {
            physical: 100.0,
            agility: 100.0,
            endurance: 100.0,
            insight: 100.0,
            mana_power: 0.0,
            soul_strength: 100.0,
        }
    }
}

/// Baseline body profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineBodyProfile {
    pub species: String,
    /// Comfort temperature range in Celsius
    pub comfort_temperature_range: (f64, f64),
    pub mana_sense_baseline: ManaSenseBaseline,
    pub mana_attribute_affinity: Vec<ManaAttribute>,
    pub size_class: SizeClass,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManaSenseBaseline {
    /// Acuity 0.0-1.0 (0.0 for mundane)
    pub acuity: f64,
    /// Overload threshold
    pub overload_threshold: f64,
    /// Innate attribute sensitivity
    pub attribute_bias: Option<ManaAttribute>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SizeClass {
    Tiny,
    Small,
    Humanoid,
    Large,
    Huge,
    Kaiju,
}

/// Long-term mana expression tendency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManaExpressionTendency {
    /// Inward - naturally concealing
    Inward,
    /// Neutral - natural state
    Neutral,
    /// Expressive - naturally revealing
    Expressive,
}

/// Temporary character state (Layer 1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporaryCharacterState {
    pub injuries: Vec<InjuryState>,
    /// Fatigue 0.0-1.0
    pub fatigue: f64,
    /// Pain load 0.0-1.0
    pub pain_load: f64,
    /// Current mana reserve
    pub mana_reserve_current: Option<f64>,
    /// Current mana expression state
    pub mana_expression: ManaExpressionState,
    /// Active suppressions
    pub mana_suppression: Vec<ManaSuppressionState>,
    /// Environmental exposure (cross-turn)
    pub environmental_exposure: EnvironmentalExposureState,
    /// Active conditions (poison, stun, etc.)
    pub active_conditions: Vec<ConditionState>,
    /// Ability cooldowns
    pub cooldowns: Vec<CooldownState>,
    /// Transient signals (trembling, flushed, etc.)
    pub transient_signals: Vec<String>,
    pub schema_version: String,
}

impl TemporaryCharacterState {
    pub fn new() -> Self {
        Self {
            injuries: Vec::new(),
            fatigue: 0.0,
            pain_load: 0.0,
            mana_reserve_current: None,
            mana_expression: ManaExpressionState::natural(),
            mana_suppression: Vec::new(),
            environmental_exposure: EnvironmentalExposureState::new(),
            active_conditions: Vec::new(),
            cooldowns: Vec::new(),
            transient_signals: Vec::new(),
            schema_version: SCHEMA_VERSION.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjuryState {
    pub injury_id: String,
    pub body_region: String,
    pub severity: InjurySeverity,
    pub effect_tags: Vec<String>,
    pub source_event_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InjurySeverity {
    Bruise,
    Light,
    Moderate,
    Severe,
    Critical,
}

/// Mana expression state (runtime)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManaExpressionState {
    pub mode: ManaExpressionMode,
    /// Display ratio = clamp(1 + tendency_factor + mode_factor, 0, 2)
    pub display_ratio: f64,
    /// Pressure ratio for environment
    pub pressure_ratio: f64,
    pub radius_tier: ManaPresenceRadiusTier,
    pub intentionality: ManaExpressionIntentionality,
    pub source_id: Option<String>,
    pub expires_at_turn: Option<String>,
}

impl ManaExpressionState {
    pub fn natural() -> Self {
        Self {
            mode: ManaExpressionMode::Natural,
            display_ratio: 1.0,
            pressure_ratio: 1.0,
            radius_tier: ManaPresenceRadiusTier::Close,
            intentionality: ManaExpressionIntentionality::Intentional,
            source_id: None,
            expires_at_turn: None,
        }
    }

    pub fn sealed() -> Self {
        Self {
            mode: ManaExpressionMode::Sealed,
            display_ratio: 0.3,
            pressure_ratio: 0.3,
            radius_tier: ManaPresenceRadiusTier::SelfOnly,
            intentionality: ManaExpressionIntentionality::Intentional,
            source_id: None,
            expires_at_turn: None,
        }
    }

    pub fn dominating() -> Self {
        Self {
            mode: ManaExpressionMode::Dominating,
            display_ratio: 1.4,
            pressure_ratio: 1.4,
            radius_tier: ManaPresenceRadiusTier::Scene,
            intentionality: ManaExpressionIntentionality::Intentional,
            source_id: None,
            expires_at_turn: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManaExpressionIntentionality {
    Intentional,
    Unintentional,
    Forced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManaSuppressionState {
    pub source_id: String,
    pub multiplier: f64,
    pub expires_at_turn: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentalExposureState {
    pub cold_strain: f64,
    pub heat_strain: f64,
    pub respiration_strain: f64,
    pub last_updated_turn: Option<String>,
}

impl EnvironmentalExposureState {
    pub fn new() -> Self {
        Self {
            cold_strain: 0.0,
            heat_strain: 0.0,
            respiration_strain: 0.0,
            last_updated_turn: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionState {
    pub condition_id: String,
    pub domain: StateDomain,
    pub condition_kind: String,
    pub intensity: f64,
    pub source_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateDomain {
    Body,
    Resource,
    Position,
    Perception,
    Mind,
    Soul,
    Scene,
    KnowledgeReveal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CooldownState {
    pub ability_id: String,
    pub remaining_turns: u32,
}

/// Attribute tier for display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttributeTier {
    Mundane,
    Awakened,
    Adept,
    Master,
    Ascendant,
    Transcendent,
}

impl AttributeTier {
    /// Determine tier from attribute value
    pub fn from_value(value: f64) -> Self {
        if value < 200.0 {
            AttributeTier::Mundane
        } else if value < 1000.0 {
            AttributeTier::Awakened
        } else if value < 1800.0 {
            AttributeTier::Adept
        } else if value < 2600.0 {
            AttributeTier::Master
        } else if value < 5600.0 {
            AttributeTier::Ascendant
        } else {
            AttributeTier::Transcendent
        }
    }
}

/// Cost profile for skills
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostProfile {
    pub mana_reserve_delta: Option<f64>,
    pub fatigue_delta: Option<f64>,
    pub cooldown_turns: Option<u32>,
    pub material_refs: Vec<String>,
    pub required_conditions: Vec<String>,
}

impl Default for CostProfile {
    fn default() -> Self {
        Self {
            mana_reserve_delta: None,
            fatigue_delta: None,
            cooldown_turns: None,
            material_refs: Vec::new(),
            required_conditions: Vec::new(),
        }
    }
}

/// Effect intensity tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EffectIntensityTier {
    Minor,
    Moderate,
    Major,
    Severe,
    Overwhelming,
}

/// Target kind for skills
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetKind {
    SelfTarget,
    Character,
    Location,
    Area,
    Object,
    Knowledge,
}
