//! Simulation module
//!
//! Core simulation components:
//! - SceneInitializer: Candidate scene bootstrap from structured seed
//! - SceneExtractor: Recent free-text parsing into scene/user deltas
//! - AttributeResolver: Effective attribute derivation
//! - EmbodimentResolver: Layer 2 embodiment state
//! - SceneFilter: Layer 2 filtered scene view
//! - InputAssembly: Cognitive pass input assembly
//! - PhysicsResolver: Physical interaction calculations
//! - CombatMathResolver: Mana combat resolution
//! - EffectValidator: Skill effect validation
//! - OutcomePlanner: Candidate outcome/state update orchestration
//! - HistoricalTruthResolver: Truth guidance for retrospective sessions
//! - TemporalConsistencyValidator: Temporal consistency validation
//! - ReactionWindow: Reaction window management
//! - JsonRepair: Deterministic repair for LLM structured output
//! - ProvisionalTruthManager: Candidate facts from past timeline sessions
//! - CanonStatusManager: Canon status determination and promotion

pub mod attribute_resolver;
pub mod canon_status_manager;
pub mod combat_math_resolver;
pub mod effect_validator;
pub mod embodiment_resolver;
pub mod historical_truth_resolver;
pub mod input_assembly;
pub mod json_repair;
pub mod outcome_planner;
pub mod physics_resolver;
pub mod provisional_truth_manager;
pub mod reaction_window;
pub mod scene_extractor;
pub mod scene_filter;
pub mod scene_initializer;
pub mod temporal_consistency_validator;

pub use attribute_resolver::AttributeResolver;
pub use canon_status_manager::{
    BlockedCandidate, CanonStatusManager, PromotableCandidate, PromotionEvaluationResult,
    PromotionResult,
};
pub use combat_math_resolver::CombatMathResolver;
pub use effect_validator::EffectValidator;
pub use embodiment_resolver::EmbodimentResolver;
pub use historical_truth_resolver::HistoricalTruthResolver;
pub use input_assembly::InputAssembly;
pub use json_repair::{
    repair_and_deserialize, repair_and_parse, JsonRepair, JsonRepairKind, JsonRepairResult,
};
pub use outcome_planner::OutcomePlanner;
pub use physics_resolver::PhysicsResolver;
pub use provisional_truth_manager::{
    DetailSlotFillRequest, DetailSlotFillResult, ProvisionalTruthManager, SlotConflict,
    SlotConflictKind, SlotValidationResult,
};
pub use reaction_window::ReactionWindowManager;
pub use scene_extractor::SceneExtractor;
pub use scene_filter::SceneFilter;
pub use scene_initializer::SceneInitializer;
pub use temporal_consistency_validator::TemporalConsistencyValidator;
