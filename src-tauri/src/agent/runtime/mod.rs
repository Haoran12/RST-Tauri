//! Agent runtime module
//!
//! Main loop, state management, turn processing, budget monitoring, config snapshots.

pub mod budget_monitor;
pub mod config_snapshot;
pub mod conflict;
pub mod parallel_cognitive;
pub mod runtime;
pub mod state_committer;
pub mod trace;
pub mod turn_state;

pub use budget_monitor::{BudgetConfig, BudgetMonitor, BudgetReport, BudgetTraceEntry};
pub use config_snapshot::{
    AttributeTierConfig, CognitiveSchedulingConfig, CombatResolutionConfig, InputTokenBudget,
    LogRetentionConfig, ManaExpressionConfig, RequestBudgetConfig, RuntimeConfigSnapshot,
    SnapshotManager, WorldRulesSnapshot,
};
pub use conflict::{
    ConflictManager, ConflictPolicyDecision, ConflictReport, ConflictResolutionRequest,
    ConflictResolutionResponse, ConflictSeverity, ConflictSummary, SessionConflictStatus,
    TurnCanonStatus,
};
pub use parallel_cognitive::{CognitivePassResult, ParallelCognitiveExecutor};
pub use runtime::{AgentRuntime, TurnResult};
pub use state_committer::StateCommitter;
pub use trace::{
    StepName, StepStatus, StepTrace, TraceKind, TraceRecorder, TraceSummary, TurnTrace,
};
pub use turn_state::TurnWorkingState;
