//! Budget monitor for Agent runtime
//!
//! Tracks LLM call budgets, token usage, and cognitive pass scheduling.
//! Records budget decisions to Agent Trace.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::config_snapshot::RuntimeConfigSnapshot;
use crate::agent::models::generate_id;

/// Budget configuration for Agent runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    /// Maximum primary cognitive passes per turn
    pub max_primary_cognitive_passes: usize,
    /// Maximum reaction passes per window
    pub max_reaction_passes_per_window: usize,
    /// Maximum reaction depth
    pub max_reaction_depth: u8,
    /// Soft token limit for prompts (16K default)
    pub soft_token_limit: u32,
    /// Hard token limit for prompts (32K default)
    pub hard_token_limit: u32,
    /// Maximum total LLM calls per turn
    pub max_llm_calls_per_turn: usize,
    /// Enable tiering when active characters >= this threshold
    pub tiering_start_active_characters: usize,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            max_primary_cognitive_passes: 3,
            max_reaction_passes_per_window: 3,
            max_reaction_depth: 1,
            soft_token_limit: 16384,
            hard_token_limit: 32768,
            max_llm_calls_per_turn: 20,
            tiering_start_active_characters: 4,
        }
    }
}

impl BudgetConfig {
    /// Create BudgetConfig from a RuntimeConfigSnapshot
    pub fn from_snapshot(snapshot: &RuntimeConfigSnapshot) -> Self {
        Self {
            max_primary_cognitive_passes: snapshot
                .request_budget
                .cognitive_scheduling
                .max_primary_cognitive_passes,
            max_reaction_passes_per_window: snapshot.request_budget.max_reaction_passes_per_window,
            max_reaction_depth: snapshot.request_budget.max_reaction_depth,
            soft_token_limit: snapshot.request_budget.input_tokens.soft_tokens,
            hard_token_limit: snapshot.request_budget.input_tokens.max_context_tokens,
            max_llm_calls_per_turn: snapshot.request_budget.max_llm_calls_per_turn,
            tiering_start_active_characters: snapshot
                .request_budget
                .cognitive_scheduling
                .tiering_start_active_characters,
        }
    }
}

/// Budget usage tracker for a single turn
#[derive(Debug, Clone, Default)]
pub struct BudgetUsage {
    /// Number of cognitive passes executed
    pub cognitive_passes: usize,
    /// Number of reaction passes executed
    pub reaction_passes: usize,
    /// Total LLM calls made
    pub llm_calls: usize,
    /// Total input tokens used
    pub total_input_tokens: u32,
    /// Total output tokens used
    pub total_output_tokens: u32,
    /// Characters deferred due to budget
    pub budget_deferred: Vec<String>,
    /// Characters that used template intent
    pub template_intent_used: Vec<String>,
    /// Characters that used minor actor slot
    pub minor_actor_slot_used: Vec<String>,
    /// Budget warnings generated
    pub warnings: Vec<BudgetWarning>,
}

/// Budget warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetWarning {
    pub warning_id: String,
    pub warning_kind: BudgetWarningKind,
    pub message: String,
    pub affected_character_ids: Vec<String>,
    pub suggested_action: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BudgetWarningKind {
    CognitivePassLimitReached,
    ReactionPassLimitReached,
    TokenSoftLimitExceeded,
    TokenHardLimitApproaching,
    LlmCallLimitApproaching,
    CharacterDeferredToTemplate,
    CharacterDeferredToMinorSlot,
}

/// Budget monitor for tracking usage and enforcing limits
#[derive(Debug, Clone)]
pub struct BudgetMonitor {
    config: BudgetConfig,
    usage: BudgetUsage,
    /// Per-character token usage
    character_tokens: HashMap<String, u32>,
    /// Per-node call counts
    node_call_counts: HashMap<String, usize>,
}

impl BudgetMonitor {
    /// Create a new budget monitor with the given configuration
    pub fn new(config: BudgetConfig) -> Self {
        Self {
            config,
            usage: BudgetUsage::default(),
            character_tokens: HashMap::new(),
            node_call_counts: HashMap::new(),
        }
    }

    /// Update configuration from a runtime snapshot
    ///
    /// This should be called at the start of each turn after the snapshot is captured.
    pub fn update_from_snapshot(&mut self, snapshot: &RuntimeConfigSnapshot) {
        self.config = BudgetConfig::from_snapshot(snapshot);
    }

    /// Get current budget usage
    pub fn usage(&self) -> &BudgetUsage {
        &self.usage
    }

    /// Get budget configuration
    pub fn config(&self) -> &BudgetConfig {
        &self.config
    }

    /// Check if a cognitive pass can be executed
    pub fn can_execute_cognitive_pass(&self) -> bool {
        self.usage.cognitive_passes < self.config.max_primary_cognitive_passes
    }

    /// Check if a reaction pass can be executed
    pub fn can_execute_reaction_pass(&self, window_passes: usize) -> bool {
        window_passes < self.config.max_reaction_passes_per_window
            && self.usage.reaction_passes < self.config.max_primary_cognitive_passes * 2
    }

    /// Check if an LLM call can be made
    pub fn can_make_llm_call(&self) -> bool {
        self.usage.llm_calls < self.config.max_llm_calls_per_turn
    }

    /// Check if token budget allows the estimated tokens
    pub fn can_use_tokens(&self, estimated_tokens: u32) -> bool {
        self.usage.total_input_tokens + estimated_tokens <= self.config.hard_token_limit
    }

    /// Check if soft token limit is exceeded
    pub fn is_soft_limit_exceeded(&self) -> bool {
        self.usage.total_input_tokens > self.config.soft_token_limit
    }

    /// Record a cognitive pass execution
    pub fn record_cognitive_pass(
        &mut self,
        character_id: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) {
        self.usage.cognitive_passes += 1;
        self.usage.llm_calls += 1;
        self.usage.total_input_tokens += input_tokens;
        self.usage.total_output_tokens += output_tokens;
        *self
            .character_tokens
            .entry(character_id.to_string())
            .or_insert(0) += input_tokens + output_tokens;

        // Check for warnings
        if self.usage.cognitive_passes == self.config.max_primary_cognitive_passes {
            self.usage.warnings.push(BudgetWarning {
                warning_id: generate_id("warn"),
                warning_kind: BudgetWarningKind::CognitivePassLimitReached,
                message: format!(
                    "Cognitive pass limit ({}) reached",
                    self.config.max_primary_cognitive_passes
                ),
                affected_character_ids: Vec::new(),
                suggested_action: Some(
                    "Additional characters will use template intents or minor actor slots"
                        .to_string(),
                ),
            });
        }
    }

    /// Record a reaction pass execution
    pub fn record_reaction_pass(
        &mut self,
        character_id: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) {
        self.usage.reaction_passes += 1;
        self.usage.llm_calls += 1;
        self.usage.total_input_tokens += input_tokens;
        self.usage.total_output_tokens += output_tokens;
        *self
            .character_tokens
            .entry(character_id.to_string())
            .or_insert(0) += input_tokens + output_tokens;
    }

    /// Record an LLM call (non-cognitive pass)
    pub fn record_llm_call(&mut self, node_name: &str, input_tokens: u32, output_tokens: u32) {
        self.usage.llm_calls += 1;
        self.usage.total_input_tokens += input_tokens;
        self.usage.total_output_tokens += output_tokens;
        *self
            .node_call_counts
            .entry(node_name.to_string())
            .or_insert(0) += 1;

        // Check for approaching limits
        if self.usage.llm_calls == self.config.max_llm_calls_per_turn - 3 {
            self.usage.warnings.push(BudgetWarning {
                warning_id: generate_id("warn"),
                warning_kind: BudgetWarningKind::LlmCallLimitApproaching,
                message: format!(
                    "Approaching LLM call limit ({}/{})",
                    self.usage.llm_calls, self.config.max_llm_calls_per_turn
                ),
                affected_character_ids: Vec::new(),
                suggested_action: Some("Consider simplifying remaining operations".to_string()),
            });
        }

        // Check token limits
        if self.usage.total_input_tokens > self.config.soft_token_limit {
            self.usage.warnings.push(BudgetWarning {
                warning_id: generate_id("warn"),
                warning_kind: BudgetWarningKind::TokenSoftLimitExceeded,
                message: format!(
                    "Token soft limit ({}) exceeded: {} tokens used",
                    self.config.soft_token_limit, self.usage.total_input_tokens
                ),
                affected_character_ids: Vec::new(),
                suggested_action: Some("Prompt pruning may occur for subsequent calls".to_string()),
            });
        }
    }

    /// Record a character deferred to template intent
    pub fn record_template_intent(&mut self, character_id: &str) {
        self.usage
            .template_intent_used
            .push(character_id.to_string());
        self.usage.budget_deferred.push(character_id.to_string());

        self.usage.warnings.push(BudgetWarning {
            warning_id: generate_id("warn"),
            warning_kind: BudgetWarningKind::CharacterDeferredToTemplate,
            message: format!(
                "Character {} using template intent due to budget",
                character_id
            ),
            affected_character_ids: vec![character_id.to_string()],
            suggested_action: None,
        });
    }

    /// Record a character deferred to minor actor slot
    pub fn record_minor_actor_slot(&mut self, character_id: &str) {
        self.usage
            .minor_actor_slot_used
            .push(character_id.to_string());
        self.usage.budget_deferred.push(character_id.to_string());

        self.usage.warnings.push(BudgetWarning {
            warning_id: generate_id("warn"),
            warning_kind: BudgetWarningKind::CharacterDeferredToMinorSlot,
            message: format!(
                "Character {} using minor actor slot due to budget",
                character_id
            ),
            affected_character_ids: vec![character_id.to_string()],
            suggested_action: None,
        });
    }

    /// Generate a budget report for trace
    pub fn generate_report(&self) -> BudgetReport {
        BudgetReport {
            config: self.config.clone(),
            cognitive_passes: self.usage.cognitive_passes,
            reaction_passes: self.usage.reaction_passes,
            total_llm_calls: self.usage.llm_calls,
            total_input_tokens: self.usage.total_input_tokens,
            total_output_tokens: self.usage.total_output_tokens,
            budget_deferred: self.usage.budget_deferred.clone(),
            template_intent_used: self.usage.template_intent_used.clone(),
            minor_actor_slot_used: self.usage.minor_actor_slot_used.clone(),
            warnings: self.usage.warnings.clone(),
            character_token_breakdown: self.character_tokens.clone(),
            node_call_breakdown: self.node_call_counts.clone(),
        }
    }

    /// Reset for a new turn
    pub fn reset(&mut self) {
        self.usage = BudgetUsage::default();
        self.character_tokens.clear();
        self.node_call_counts.clear();
    }

    /// Sync from a budget report (used after parallel execution)
    pub fn sync_from_report(&mut self, report: &BudgetReport) {
        self.usage.cognitive_passes = report.cognitive_passes;
        self.usage.reaction_passes = report.reaction_passes;
        self.usage.llm_calls = report.total_llm_calls;
        self.usage.total_input_tokens = report.total_input_tokens;
        self.usage.total_output_tokens = report.total_output_tokens;
        self.usage.budget_deferred = report.budget_deferred.clone();
        self.usage.template_intent_used = report.template_intent_used.clone();
        self.usage.minor_actor_slot_used = report.minor_actor_slot_used.clone();
        self.usage.warnings = report.warnings.clone();
        self.character_tokens = report.character_token_breakdown.clone();
        self.node_call_counts = report.node_call_breakdown.clone();
    }
}

impl Default for BudgetMonitor {
    fn default() -> Self {
        Self::new(BudgetConfig::default())
    }
}

/// Budget report for trace recording
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetReport {
    pub config: BudgetConfig,
    pub cognitive_passes: usize,
    pub reaction_passes: usize,
    pub total_llm_calls: usize,
    pub total_input_tokens: u32,
    pub total_output_tokens: u32,
    pub budget_deferred: Vec<String>,
    pub template_intent_used: Vec<String>,
    pub minor_actor_slot_used: Vec<String>,
    pub warnings: Vec<BudgetWarning>,
    pub character_token_breakdown: HashMap<String, u32>,
    pub node_call_breakdown: HashMap<String, usize>,
}

/// Trace recorder for budget events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetTraceEntry {
    pub entry_id: String,
    pub scene_turn_id: String,
    pub step_name: String,
    pub budget_stage: String,
    pub report: BudgetReport,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl BudgetTraceEntry {
    /// Create a new budget trace entry
    pub fn new(scene_turn_id: &str, step_name: &str, report: BudgetReport) -> Self {
        let budget_stage = if report.total_input_tokens <= report.config.soft_token_limit / 2 {
            "within_8k".to_string()
        } else if report.total_input_tokens <= report.config.soft_token_limit {
            "within_16k".to_string()
        } else if report.total_input_tokens <= report.config.hard_token_limit {
            "max_context_fit".to_string()
        } else {
            "blocked_unfit".to_string()
        };

        Self {
            entry_id: generate_id("budget_trace"),
            scene_turn_id: scene_turn_id.to_string(),
            step_name: step_name.to_string(),
            budget_stage,
            report,
            created_at: chrono::Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_cognitive_pass_budget() {
        let mut monitor = BudgetMonitor::default();

        assert!(monitor.can_execute_cognitive_pass());
        monitor.record_cognitive_pass("char_1", 1000, 500);
        assert_eq!(monitor.usage().cognitive_passes, 1);
        assert_eq!(monitor.usage().total_input_tokens, 1000);

        // Use up the limit
        monitor.record_cognitive_pass("char_2", 1000, 500);
        monitor.record_cognitive_pass("char_3", 1000, 500);

        assert!(!monitor.can_execute_cognitive_pass());
        assert!(monitor
            .usage()
            .warnings
            .iter()
            .any(|w| { w.warning_kind == BudgetWarningKind::CognitivePassLimitReached }));
    }

    #[test]
    fn tracks_token_budget() {
        let config = BudgetConfig {
            soft_token_limit: 500,
            hard_token_limit: 1000,
            ..Default::default()
        };
        let mut monitor = BudgetMonitor::new(config);

        assert!(monitor.can_use_tokens(300));
        monitor.record_llm_call("test_node", 600, 200);

        // 600 > 500, so soft limit should be exceeded
        assert!(monitor.is_soft_limit_exceeded());
        assert!(monitor
            .usage()
            .warnings
            .iter()
            .any(|w| { w.warning_kind == BudgetWarningKind::TokenSoftLimitExceeded }));

        // 600 + 500 = 1100 > 1000, so cannot use 500 more tokens
        assert!(!monitor.can_use_tokens(500));
    }

    #[test]
    fn generates_trace_report() {
        let mut monitor = BudgetMonitor::default();

        monitor.record_cognitive_pass("char_1", 1000, 500);
        monitor.record_template_intent("char_2");

        let report = monitor.generate_report();

        assert_eq!(report.cognitive_passes, 1);
        assert_eq!(report.template_intent_used.len(), 1);
        assert!(report.character_token_breakdown.contains_key("char_1"));
    }
}
