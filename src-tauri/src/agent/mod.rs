//! Agent Mode - Advanced Role-Play System
//!
//! This module implements the Agent mode architecture with three-layer semantic isolation:
//! - Layer 1 (Truth Store): Objective truth, accessible only by orchestrator and outcome planner/validator
//! - Layer 2 (Per-Character Access): Per-character accessible view, rebuilt each turn, no persistence
//! - Layer 3 (Subjective State): Subjective mind, updated after each cognitive pass
//!
//! Key invariants:
//! - Free text only appears in: user input, SceneStateExtractor input, SurfaceRealizer output
//! - Cross-layer direct read/write is prohibited
//! - God-read ≠ commit permission
//! - KnowledgeAccessResolver never calls LLM

pub mod cache;
pub mod cognitive;
pub mod director_hint;
pub mod input_preparser;
pub mod knowledge;
pub(crate) mod llm_support;
pub mod location;
pub mod models;
pub mod presentation;
pub mod prompting;
pub mod runtime;
pub mod simulation;
pub mod storage;
pub mod validation;
pub mod world_editor;

pub use director_hint::{DirectorHint, DirectorHintCollection, OutcomeBias, StyleOverride};
pub use input_preparser::{
    InputAuthorityClass, InputPreparser, PreparsedUserInput, UserInputDelta,
};
pub use models::*;
pub use prompting::*;
pub use runtime::AgentRuntime;
