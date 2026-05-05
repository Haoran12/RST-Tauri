//! Scene models - Layer 1 Truth Store
//!
//! SceneModel, ScenePrivateState, PhysicalConditions, ManaField

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Scene model - objective scene state (Layer 1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneModel {
    pub scene_id: String,
    pub scene_turn_id: String,
    pub time_context: TimeContext,
    pub spatial_layout: SpatialLayout,
    pub lighting: LightingState,
    pub acoustics: AcousticsState,
    pub olfactory_field: OlfactoryField,
    pub scene_mood: SceneMood,
    pub physical_conditions: PhysicalConditions,
    pub mana_field: ManaField,
    pub entities: Vec<SceneEntity>,
    pub observable_signals: ObservableSignals,
    pub private_state: ScenePrivateState,
    pub event_stream: Vec<SceneEvent>,
    pub uncertainty_notes: Vec<String>,
}

/// Time and weather context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeContext {
    pub time_anchor: super::common::TimeAnchor,
    pub season: String,
    pub day_phase: DayPhase,
    pub weather_trend: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DayPhase {
    Dawn,
    Day,
    Dusk,
    Night,
    DeepNight,
}

/// Spatial layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialLayout {
    pub layout_type: String,
    pub dimensions: Option<Dimensions>,
    pub obstacles: Vec<Obstacle>,
    pub entrances: Vec<Entrance>,
    pub zones: Vec<Zone>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimensions {
    pub length_m: f64,
    pub width_m: f64,
    pub height_m: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Obstacle {
    pub obstacle_id: String,
    pub obstacle_type: String,
    pub position: Position,
    pub passable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entrance {
    pub entrance_id: String,
    pub entrance_type: String,
    pub position: Position,
    pub direction: String,
    pub leads_to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    pub zone_id: String,
    pub zone_type: String,
    pub label: String,
    pub bounds: ZoneBounds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneBounds {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: Option<f64>,
}

/// Lighting state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightingState {
    pub ambient_level: f64,
    pub light_sources: Vec<LightSource>,
    pub shadow_areas: Vec<ShadowArea>,
    pub backlight: Option<BacklightState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightSource {
    pub source_id: String,
    pub source_type: String,
    pub position: Position,
    pub intensity: f64,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowArea {
    pub area_id: String,
    pub bounds: ZoneBounds,
    pub density: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacklightState {
    pub direction: String,
    pub intensity: f64,
}

/// Acoustics state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcousticsState {
    pub ambient_noise_level: f64,
    pub echo_characteristics: String,
    pub sound_sources: Vec<SoundSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundSource {
    pub source_id: String,
    pub source_type: String,
    pub position: Position,
    pub volume: f64,
    pub description: String,
}

/// Olfactory field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OlfactoryField {
    pub dominant_scents: Vec<ScentSource>,
    pub airflow: AirflowState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScentSource {
    pub source_id: String,
    pub scent_type: String,
    pub position: Position,
    pub intensity: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirflowState {
    pub direction: String,
    pub speed: f64,
    pub turbulence: f64,
}

/// Scene mood (atmosphere)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneMood {
    Neutral,
    Tense,
    Solemn,
    Celebratory,
    Hostile,
    Intimate,
    Eerie,
    Melancholic,
    Hopeful,
    Chaotic,
}

/// Physical conditions (Layer 1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalConditions {
    pub temperature: Temperature,
    pub surface_state: SurfaceState,
    pub airborne: AirborneEffects,
    pub precipitation: Option<Precipitation>,
    pub wind: WindState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Temperature {
    /// Ambient temperature in Celsius
    pub ambient_celsius: f64,
    /// Felt temperature (with modifiers applied)
    pub felt_celsius: f64,
    pub modifiers: Vec<TemperatureModifier>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureModifier {
    pub source_id: String,
    pub delta_celsius: f64,
    pub radius_m: f64,
    pub kind: TemperatureModifierKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemperatureModifierKind {
    PhysicalHeat,
    PhysicalCold,
    ManaHeat,
    ManaCold,
    SpellBarrier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceState {
    pub slipperiness: f64,
    pub wetness: f64,
    pub debris: Vec<String>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirborneEffects {
    pub fog_density: f64,
    pub dust_density: f64,
    pub smoke_density: f64,
    pub visibility_range_m: f64,
    pub mana_haze: Option<ManaHaze>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManaHaze {
    pub density: f64,
    pub attribute: ManaAttribute,
    pub interference_level: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManaAttribute {
    Metal,
    Wood,
    Water,
    Fire,
    Earth,
    Wind,
    Lightning,
    Ice,
    Light,
    Dark,
    Void,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Precipitation {
    pub kind: PrecipitationKind,
    pub intensity: f64,
    pub mana_attribute: Option<ManaAttribute>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrecipitationKind {
    Rain,
    Snow,
    Hail,
    Sandstorm,
    SpiritRain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindState {
    /// Direction in degrees (0 = North)
    pub direction_deg: f64,
    /// Speed in m/s
    pub speed_ms: f64,
    pub gust: bool,
}

/// Mana field (Layer 1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManaField {
    pub ambient_density: f64,
    pub ambient_attribute: ManaAttribute,
    pub mana_sources: Vec<ManaSource>,
    pub character_presences: Vec<CharacterManaPresence>,
    pub flow: ManaFlow,
    pub interferences: Vec<ManaInterference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManaSource {
    pub source_id: String,
    pub source_type: ManaSourceType,
    pub position: Position,
    pub intensity: f64,
    pub attribute: Option<ManaAttribute>,
    pub radius_m: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManaSourceType {
    SpiritVein,
    FormationCore,
    BarrierNode,
    SpiritWell,
    CultivatorAura,
    ArtifactAura,
    SpiritBeastAura,
    FormationTrace,
    SpellResidue,
    Breakthrough,
    Tribulation,
    Sacrifice,
    Corruption,
    Seal,
    VoidRift,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterManaPresence {
    pub character_id: String,
    pub source_type: ManaSourceType,
    pub expression_mode: ManaExpressionMode,
    pub radius_tier: ManaPresenceRadiusTier,
    pub pressure_delta: AttributeDelta,
    pub attribute: Option<ManaAttribute>,
    pub descriptors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManaExpressionMode {
    Sealed,
    Suppressed,
    Natural,
    Released,
    Dominating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManaPresenceRadiusTier {
    SelfOnly,
    Touch,
    Close,
    Room,
    Area,
    Scene,
}

/// Attribute delta for perception
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttributeDelta {
    Indistinguishable,
    SlightlyBelow,
    NotablyBelow,
    FarBelow,
    Crushed,
    SlightlyAbove,
    NotablyAbove,
    FarAbove,
    Overwhelming,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManaFlow {
    pub direction: String,
    pub intensity: f64,
    pub turbulence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManaInterference {
    pub interference_id: String,
    pub kind: ManaInterferenceKind,
    pub source_id: String,
    pub intensity: f64,
    pub radius_m: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManaInterferenceKind {
    Shield,
    Disruption,
    Disguise,
    Amplification,
    Redirection,
}

/// Scene entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEntity {
    pub entity_id: String,
    pub entity_kind: SceneEntityKind,
    pub position: Position,
    pub posture: String,
    pub display_name: String,
    pub observable_facets: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneEntityKind {
    Character,
    Prop,
    TerrainFeature,
    BackgroundActor,
}

/// Observable signals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservableSignals {
    pub visual_signals: Vec<VisualSignal>,
    pub audio_signals: Vec<AudioSignal>,
    pub mana_signals: Vec<ManaSignal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualSignal {
    pub signal_id: String,
    pub source_entity_id: Option<String>,
    pub signal_type: String,
    pub description: String,
    pub intensity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSignal {
    pub signal_id: String,
    pub source_entity_id: Option<String>,
    pub signal_type: String,
    pub description: String,
    pub volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManaSignal {
    pub signal_id: String,
    pub source_entity_id: Option<String>,
    pub signal_type: String,
    pub description: String,
    pub intensity: f64,
    pub attribute: Option<ManaAttribute>,
}

/// Scene private state (Layer 1, not publicly observable)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenePrivateState {
    pub hidden_facts: Vec<ScenePrivateFact>,
    pub reveal_triggers: Vec<SceneRevealTrigger>,
    pub source_constraint_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenePrivateFact {
    pub fact_id: String,
    pub source_knowledge_id: Option<String>,
    pub applies_to: Vec<String>,
    pub fact_kind: ScenePrivateFactKind,
    pub structured_payload: serde_json::Value,
    pub summary_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScenePrivateFactKind {
    HiddenPresence,
    Trap,
    Disguise,
    SealedArea,
    ContinuitySecret,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneRevealTrigger {
    pub trigger_id: String,
    pub private_fact_id: String,
    pub condition_refs: Vec<String>,
    pub reveal_target: RevealTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RevealTarget {
    PublicSceneFact,
    KnowledgeEntry(String),
    CharacterKnownBy(Vec<String>),
}

/// Scene event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEvent {
    pub event_id: String,
    pub event_kind: String,
    pub involved_entity_ids: Vec<String>,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Scene snapshot for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneSnapshot {
    pub snapshot_id: String,
    pub scene_id: String,
    pub scene_turn_id: String,
    pub scene_model: SceneModel,
    pub created_at: DateTime<Utc>,
}

// ===== Environment Tier Types =====

/// Wind impact tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindImpactTier {
    Calm,
    Breeze,
    Moderate,
    Strong,
    Gale,
    Storm,
    Hurricane,
}

/// Temperature feel tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemperatureFeelTier {
    Sweltering,
    Hot,
    Warm,
    Comfortable,
    Cool,
    Cold,
    SevereCold,
    Lethal,
}

/// Surface impact tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceImpactTier {
    Stable,
    Slippery,
    Treacherous,
}

/// Visibility tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisibilityTier {
    Clear,
    Hazy,
    Limited,
    Blind,
}

/// Respiration impact tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RespirationImpactTier {
    Free,
    Irritating,
    Choking,
    Suffocating,
}

/// Precipitation intensity tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrecipitationIntensityTier {
    None,
    Light,
    Moderate,
    Heavy,
    Torrential,
}

/// Surface visual state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceVisualState {
    Dry,
    Damp,
    Wet,
    Puddled,
    Snowy,
    Icy,
    Bloody,
    Cluttered,
}

/// Ambient mana density tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AmbientManaDensityTier {
    Barren,
    Sparse,
    Normal,
    Rich,
    Dense,
    Saturated,
}

// ===== Layer 2 Types =====

/// Filtered scene view (Layer 2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilteredSceneView {
    pub character_id: String,
    pub scene_turn_id: String,
    pub observable_entities: Vec<ObservableEntity>,
    pub perceived_attributes: Vec<PerceivedAttributeProfile>,
    pub audible_signals: Vec<AudioSignal>,
    pub olfactory_signals: Vec<OlfactorySignal>,
    pub tactile_signals: Vec<TactileSignal>,
    pub mana_signals: Vec<ManaSignal>,
    pub mana_environment: ManaEnvironmentSense,
    pub weather_perception: WeatherPerception,
    pub spatial_context: SpatialContext,
}

/// Observable entity (Layer 2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservableEntity {
    pub entity_id: String,
    pub perception_score: f64,
    pub clarity: f64,
    pub observable_facets: Vec<String>,
    pub notes: String,
}

/// Perceived attribute profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceivedAttributeProfile {
    pub source_id: String,
    pub attribute_kind: AttributeKind,
    pub tier_assessment: Option<super::character::AttributeTier>,
    pub delta: Option<AttributeDelta>,
    pub confidence: f64,
    pub evidence: Vec<AttributeEvidenceKind>,
    pub descriptors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttributeKind {
    Physical,
    Agility,
    Endurance,
    Insight,
    ManaPower,
    SoulStrength,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttributeEvidenceKind {
    Appearance,
    Movement,
    SustainedAction,
    InjuryResponse,
    CombatExchange,
    TacticalRead,
    ManaSignal,
    SoulPressure,
    SkillEffect,
}

/// Olfactory signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OlfactorySignal {
    pub signal_id: String,
    pub source_entity_id: Option<String>,
    pub signal_type: String,
    pub description: String,
    pub intensity: f64,
}

/// Tactile signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TactileSignal {
    pub signal_id: String,
    pub source_entity_id: Option<String>,
    pub signal_type: String,
    pub description: String,
    pub intensity: f64,
}

/// Mana environment sense
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManaEnvironmentSense {
    pub density_tier: AmbientManaDensityTier,
    pub dominant_attribute: Option<ManaAttribute>,
    pub character_presences: Vec<ManaPresenceSense>,
    pub interferences: Vec<String>,
    pub overload_risk: bool,
    pub descriptors: Vec<String>,
}

/// Mana presence sense
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManaPresenceSense {
    pub source_id: String,
    pub expression_assessment: Option<ManaExpressionMode>,
    pub radius_tier: ManaPresenceRadiusTier,
    pub pressure_delta: AttributeDelta,
    pub cognitive_effect_hints: Vec<String>,
}

/// Weather perception
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherPerception {
    pub wind_tier: WindImpactTier,
    pub temperature_tier: TemperatureFeelTier,
    pub visibility_tier: VisibilityTier,
    pub respiration_tier: RespirationImpactTier,
    pub surface_visual: Vec<SurfaceVisualState>,
    pub surface_tier: SurfaceImpactTier,
    pub precipitation: Option<PrecipitationDescriptor>,
    pub effect_hints: Vec<String>,
}

/// Precipitation descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecipitationDescriptor {
    pub kind: PrecipitationKind,
    pub intensity_tier: PrecipitationIntensityTier,
    pub mana_attribute: Option<ManaAttribute>,
}

/// Spatial context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialContext {
    pub layout_type: String,
    pub visible_zones: Vec<Zone>,
    pub visible_obstacles: Vec<Obstacle>,
    pub visible_entrances: Vec<Entrance>,
}

// ===== Embodiment State (Layer 2) =====

/// Embodiment state (Layer 2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbodimentState {
    pub character_id: String,
    pub scene_turn_id: String,
    pub sensory_capabilities: SensoryCapabilities,
    pub body_constraints: BodyConstraints,
    pub salience_modifiers: SalienceModifiers,
    pub reasoning_modifiers: ReasoningModifiers,
    pub action_feasibility: ActionFeasibility,
}

/// Sensory capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensoryCapabilities {
    pub vision: SensoryCapability,
    pub hearing: SensoryCapability,
    pub smell: SensoryCapability,
    pub touch: SensoryCapability,
    pub proprioception: SensoryCapability,
    pub mana: SensoryCapability,
}

/// Sensory capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensoryCapability {
    pub availability: f64,
    pub acuity: f64,
    pub stability: f64,
    pub notes: String,
}

impl Default for SensoryCapability {
    fn default() -> Self {
        Self {
            availability: 1.0,
            acuity: 1.0,
            stability: 1.0,
            notes: String::new(),
        }
    }
}

/// Body constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyConstraints {
    pub mobility: f64,
    pub balance: f64,
    pub fine_control: f64,
    pub pain_load: f64,
    pub fatigue_load: f64,
    pub cognitive_clarity: f64,
    pub environmental_strain: EnvironmentalStrain,
}

/// Environmental strain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentalStrain {
    pub wind_tier: WindImpactTier,
    pub temperature_tier: TemperatureFeelTier,
    pub surface_tier: SurfaceImpactTier,
    pub respiration_tier: RespirationImpactTier,
    pub movement_penalty: f64,
    pub balance_penalty: f64,
    pub exposure_cold_delta: f64,
    pub exposure_heat_delta: f64,
    pub exposure_respiration_delta: f64,
    pub disrupted_actions: Vec<String>,
}

/// Salience modifiers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalienceModifiers {
    pub attention_biases: Vec<String>,
    pub aversion_triggers: Vec<String>,
    pub overload_risk: f64,
}

/// Reasoning modifiers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningModifiers {
    pub pain_bias: f64,
    pub threat_bias: f64,
    pub overload_bias: f64,
    pub notes: Vec<String>,
}

/// Action feasibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionFeasibility {
    pub physical_execution: f64,
    pub social_patience: f64,
    pub fine_control: f64,
    pub sustained_attention: f64,
    pub blocked_action_kinds: Vec<String>,
}

// ===== Effective Attribute Profile =====

/// Effective attribute profile (runtime derived)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectiveAttributeProfile {
    pub character_id: String,
    pub values: std::collections::HashMap<AttributeKind, f64>,
    pub tiers: std::collections::HashMap<AttributeKind, super::character::AttributeTier>,
    pub descriptors: std::collections::HashMap<AttributeKind, Vec<String>>,
}

/// Mana expression profile (runtime derived)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManaExpressionProfile {
    pub character_id: String,
    pub baseline_tendency: super::character::ManaExpressionTendency,
    pub mode: ManaExpressionMode,
    pub intentionality: super::character::ManaExpressionIntentionality,
    pub tendency_factor: f64,
    pub mode_factor: f64,
    pub display_ratio: f64,
    pub pressure_ratio: f64,
    pub radius_tier: ManaPresenceRadiusTier,
    pub overstated_signal: bool,
}

// ===== Scene Delta Types =====

/// Scene delta for updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDelta {
    pub scene_id: String,
    pub entity_deltas: Vec<SceneEntityDelta>,
    pub physical_delta: Option<PhysicalConditionsDelta>,
    pub mana_field_delta: Option<ManaFieldDelta>,
    pub observable_signal_deltas: Vec<ObservableSignalDelta>,
    pub private_state_deltas: Vec<ScenePrivateStateDelta>,
    pub event_appends: Vec<SceneEventDraft>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEntityDelta {
    pub entity_id: String,
    pub delta_kind: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalConditionsDelta {
    pub field_patches: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManaFieldDelta {
    pub field_patches: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservableSignalDelta {
    pub signal_id: String,
    pub delta_kind: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenePrivateStateDelta {
    pub private_fact_id: String,
    pub delta_kind: String,
    pub payload: serde_json::Value,
    pub source_constraint_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEventDraft {
    pub event_kind: String,
    pub involved_entity_ids: Vec<String>,
    pub payload: serde_json::Value,
}

/// Scene update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneUpdate {
    pub scene_turn_id: String,
    pub scene_delta: SceneDelta,
    pub update_reason: Vec<String>,
}

// ===== Scene Initialization Types =====

/// Scene initialization draft - output of SceneInitializer LLM node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneInitializationDraft {
    pub scene_turn_id: String,
    pub scene_model: SceneModel,
    pub assumptions: Vec<SceneAssumption>,
    pub blocked_additions: Vec<BlockedSceneAddition>,
    pub ambiguity_report: Vec<String>,
    pub validation_hints: Vec<String>,
}

/// Scene assumption - tracks what assumptions were made during initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneAssumption {
    pub field_path: String,
    pub source: SceneAssumptionSource,
    pub confidence: AssumptionConfidence,
    pub risk: AssumptionRisk,
    pub rationale: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneAssumptionSource {
    UserSeed,
    PublicWorldContext,
    LocationContext,
    ParticipantContext,
    ContinuityContext,
    PrivateSceneConstraint,
    ProgramDefault,
    LlmInferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssumptionConfidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssumptionRisk {
    Low,
    Medium,
    High,
}

/// Blocked scene addition - what couldn't be added and why
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedSceneAddition {
    pub attempted_domain: SceneDetailDomain,
    pub reason_code: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneDetailDomain {
    SpatialLayout,
    Lighting,
    Acoustics,
    OlfactoryField,
    PhysicalConditions,
    ManaField,
    SceneMood,
    BackgroundEntities,
    ObservableSignals,
}

// ===== Surface Realizer Types =====

/// Surface realizer input - input for the narrative rendering LLM node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceRealizerInput {
    pub scene_turn_id: String,
    /// Narrative disclosure boundary
    pub narration_scope: NarrationScope,
    /// Scene view for narrative
    pub scene_view: SceneNarrativeView,
    /// Character views for narrative
    pub character_views: Vec<NarrativeCharacterView>,
    /// Outcome plan from OutcomePlanner
    pub outcome_plan: super::subjective::OutcomePlan,
    /// Style constraints
    pub style: super::subjective::StyleConstraints,
}

/// Scene narrative view - scene model filtered for narrative
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneNarrativeView {
    pub scene_id: String,
    pub scene_turn_id: String,
    pub narration_scope: NarrationScope,
    pub visible_entities: Vec<NarrativeEntityView>,
    pub visible_environment: serde_json::Value,
    pub visible_events: Vec<NarrativeEventView>,
    pub allowed_private_refs: Vec<String>,
}

/// Narrative entity view - entity filtered for narrative
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeEntityView {
    pub entity_id: String,
    pub display_name: String,
    pub observable_facts: Vec<String>,
    pub outward_state: Vec<String>,
}

/// Narrative character view - character projection for narrative
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeCharacterView {
    pub character_id: String,
    pub display_name: String,
    pub outward_actions: Vec<String>,
    pub outward_reactions: Vec<String>,
    pub allowed_inner_summary: Option<String>,
}

/// Narrative event view - event filtered for narrative
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeEventView {
    pub event_id: String,
    pub event_kind: String,
    pub narratable_fact_refs: Vec<String>,
}

/// Narration scope - determines what can be narrated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NarrationScope {
    /// Only what the focused character can observe/infer
    CharacterFocused { character_id: String },
    /// Only outward facts, no inner thoughts
    ObjectiveCamera,
    /// Author/orchestrator view (still excludes GodOnly by default)
    DirectorView,
}

// ===== Scene Seed Types =====

/// Scene seed - input for SceneInitializer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneSeed {
    pub scene_id: String,
    pub transition_reason: SceneTransitionReason,
    pub time_seed: TimeContextSeed,
    pub location_anchor: LocationAnchor,
    pub required_participant_ids: Vec<String>,
    pub requested_mood: Option<SceneMood>,
    pub required_entities: Vec<SceneEntitySeed>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneTransitionReason {
    InitialScene,
    LocationChange,
    TimeSkip,
    RollbackRebuild,
}

/// Time context seed for scene initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeContextSeed {
    pub season: Option<String>,
    pub day_phase: Option<String>,
    pub absolute_time_hint: Option<String>,
    pub elapsed_from_previous: Option<String>,
    pub weather_trend: Option<String>,
}

/// Location anchor for scene initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationAnchor {
    pub location_id: Option<String>,
    pub fallback_region_id: Option<String>,
    pub location_type: String,
    pub known_exits: Vec<String>,
}

/// Scene entity seed for initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEntitySeed {
    pub entity_id: Option<String>,
    pub entity_kind: String,
    pub display_label: Option<String>,
    pub persistence: EntityPersistence,
    pub required: bool,
    pub position_hint: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityPersistence {
    Persistent,
    Transient,
    NonPersistent,
}

// ===== Scene / Agent LLM Node Input Types =====

/// Agent session context used by scene/runtime LLM nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionContext {
    pub session_id: String,
    pub session_kind: String,
    pub period_anchor: super::common::TimeAnchor,
    pub mainline_time_anchor: super::common::TimeAnchor,
    pub player_character_id: Option<String>,
    pub canon_status: String,
}

/// Public world context for SceneInitializer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicWorldContext {
    pub world_summary: String,
    pub public_rules: Vec<String>,
    pub ambient_defaults: serde_json::Value,
}

/// Scene participant seed for initialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneParticipantSeed {
    pub character_id: String,
    pub public_appearance_summary: String,
    pub entry_state: ParticipantEntryState,
    pub position_hint: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticipantEntryState {
    AlreadyPresent,
    Entering,
    ArrivingWithGroup,
    OffstageExpected,
}

/// Continuity context between scenes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneContinuityContext {
    pub previous_scene_summary: String,
    pub carried_entities: Vec<SceneEntitySeed>,
    pub unresolved_visible_events: Vec<String>,
}

/// Private scene constraint available only to scene-domain God-read nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenePrivateConstraint {
    pub constraint_id: String,
    pub source_knowledge_id: Option<String>,
    pub scope: PrivateConstraintScope,
    pub applies_to: Vec<String>,
    pub constraint_kind: String,
    pub constraint_summary: String,
    pub allowed_uses: Vec<PrivateConstraintUse>,
    pub reveal_conditions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivateConstraintScope {
    SceneBound,
    LocationBound,
    ParticipantBound,
    ContinuityBound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivateConstraintUse {
    InitializeHiddenState,
    PreserveContinuity,
    ValidateUserDelta,
    RevealIfTriggered,
}

/// Scene generation policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneGenerationPolicy {
    pub detail_level: super::subjective::DetailLevel,
    pub allowed_detail_domains: Vec<SceneDetailDomain>,
    pub allow_transient_background_entities: bool,
    pub max_generated_background_entities: u32,
    pub forbid_new_named_entities: bool,
    pub require_user_confirmation_above: AssumptionRisk,
}

/// Input for SceneInitializer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneInitializerInput {
    pub scene_turn_id: String,
    pub world_id: String,
    pub session_context: AgentSessionContext,
    pub seed: SceneSeed,
    pub public_world_context: PublicWorldContext,
    pub location_context: serde_json::Value,
    pub participant_context: Vec<SceneParticipantSeed>,
    pub continuity_context: Option<SceneContinuityContext>,
    pub private_scene_constraints: Vec<ScenePrivateConstraint>,
    pub truth_guidance: Option<super::knowledge::TruthGuidance>,
    pub world_constraints: serde_json::Value,
    pub generation_policy: SceneGenerationPolicy,
}

/// Input for SceneStateExtractor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneStateExtractorInput {
    pub scene_turn_id: String,
    pub session_context: AgentSessionContext,
    pub recent_free_text: String,
    pub current_scene: SceneModel,
    pub private_scene_constraints: Vec<ScenePrivateConstraint>,
    pub truth_guidance: Option<super::knowledge::TruthGuidance>,
    pub world_constraints: serde_json::Value,
}

/// Scene-domain reaction pass input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionPassInput {
    pub character_id: String,
    pub scene_turn_id: String,
    pub filtered_scene_view: FilteredSceneView,
    pub embodiment_state: EmbodimentState,
    pub accessible_knowledge: super::knowledge::AccessibleKnowledge,
    pub prior_subjective_state: super::subjective::CharacterSubjectiveState,
    pub reaction_window: super::subjective::ReactionWindow,
    pub available_reaction_options: Vec<super::subjective::ReactionOption>,
}
