//! Temporal consistency validator
//!
//! Validates that retrospective session outputs don't violate TruthGuidance.
//! Ensures temporal consistency with established canon.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::agent::models::{
    ConflictPolicyDecision, ConflictReport, ConflictSeverity, KnowledgeEntry, KnowledgeKind,
    TemporalCanonStatus, TimeAnchor, TruthGuidance,
};

use super::historical_truth_resolver::{
    Conflict, ConflictCheckResult, ConflictSeverity as ResolverConflictSeverity, ProposedChange,
};

/// Temporal consistency validator - validates retrospective outputs
pub struct TemporalConsistencyValidator {
    pool: SqlitePool,
}

impl TemporalConsistencyValidator {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Validate a proposed state update against truth guidance
    pub fn validate_against_guidance(
        &self,
        guidance: &TruthGuidance,
        proposed_updates: &[ProposedStateUpdate],
    ) -> TemporalValidationResult {
        let mut violations = Vec::new();
        let mut warnings = Vec::new();
        let mut blocked_updates = Vec::new();
        let mut allowed_updates = Vec::new();

        for update in proposed_updates {
            let validation = self.validate_single_update(guidance, update);

            if validation.has_hard_violation {
                violations.extend(validation.violations);
                blocked_updates.push(update.update_id.clone());
            } else {
                if !validation.warnings.is_empty() {
                    warnings.extend(validation.warnings.clone());
                }
                allowed_updates.push(update.update_id.clone());
            }
        }

        // Determine overall canon status
        let canon_status = if violations.is_empty() {
            if warnings.is_empty() {
                TemporalCanonStatus::Canon
            } else {
                TemporalCanonStatus::ProvisionalPromoted
            }
        } else {
            TemporalCanonStatus::NonCanon
        };

        TemporalValidationResult {
            is_valid: violations.is_empty(),
            canon_status,
            violations,
            warnings,
            blocked_updates,
            allowed_updates,
        }
    }

    /// Validate a single proposed update
    fn validate_single_update(
        &self,
        guidance: &TruthGuidance,
        update: &ProposedStateUpdate,
    ) -> SingleUpdateValidation {
        let mut violations = Vec::new();
        let mut warnings = Vec::new();

        // Check against hard constraints
        for constraint in &guidance.hard_constraints {
            if update
                .affects_events
                .contains(&constraint.source_knowledge_id)
            {
                violations.push(TemporalViolation {
                    violation_kind: TemporalViolationKind::HardConstraintViolation,
                    message: "提议修改受约束事件".to_string(),
                    update_id: update.update_id.clone(),
                    severity: ViolationSeverity::Hard,
                });
            }
        }

        // Check future knowledge warnings
        for warning in &guidance.future_knowledge_warnings {
            if update.uses_future_knowledge {
                violations.push(TemporalViolation {
                    violation_kind: TemporalViolationKind::FutureKnowledgeViolation,
                    message: warning.clone(),
                    update_id: update.update_id.clone(),
                    severity: ViolationSeverity::Hard,
                });
            }
        }

        // Check temporal ordering
        if let Some(ref event_time) = update.event_time {
            if event_time.ordinal > guidance.period_anchor.ordinal + 1000 {
                warnings.push("事件时间超出会话时间范围".to_string());
            }
        }

        SingleUpdateValidation {
            update_id: update.update_id.clone(),
            has_hard_violation: violations
                .iter()
                .any(|v| v.severity == ViolationSeverity::Hard),
            violations,
            warnings,
        }
    }

    /// Generate conflict report for a session
    pub async fn generate_conflict_report(
        &self,
        session_id: &str,
        session_turn_id: &str,
        guidance: &TruthGuidance,
        proposed_updates: &[ProposedStateUpdate],
    ) -> Result<ConflictReport, String> {
        let validation = self.validate_against_guidance(guidance, proposed_updates);

        Ok(ConflictReport {
            conflict_id: format!("conflict_{}", session_turn_id),
            session_id: session_id.to_string(),
            session_turn_id: session_turn_id.to_string(),
            scene_turn_id: None,
            severity: if validation.has_hard_conflict() {
                ConflictSeverity::Hard
            } else {
                ConflictSeverity::Soft
            },
            source_constraint_ids: validation
                .violations
                .iter()
                .map(|v| v.update_id.clone())
                .collect(),
            affected_provisional_ids: validation.blocked_updates.clone(),
            policy_decision: None,
            summary: serde_json::json!({
                "violations": validation.violations.iter().map(|v| &v.message).collect::<Vec<_>>(),
                "warnings": validation.warnings,
            }),
            created_at: Utc::now(),
            resolved_at: None,
        })
    }

    /// Check if a provisional truth can be promoted to canon
    pub async fn check_canon_promotion(
        &self,
        provisional_id: &str,
        guidance: &TruthGuidance,
    ) -> Result<CanonPromotionResult, String> {
        // Get the provisional truth
        let row = sqlx::query_as::<_, ProvisionalTruthRow>(
            r#"
            SELECT provisional_id, session_id, content, created_at
            FROM provisional_session_truths
            WHERE provisional_id = ?
            "#,
        )
        .bind(provisional_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to query provisional truth: {}", e))?;

        let row = match row {
            Some(r) => r,
            None => return Err(format!("Provisional truth {} not found", provisional_id)),
        };

        // Parse the content
        let content: serde_json::Value = serde_json::from_str(&row.content)
            .map_err(|e| format!("Failed to parse provisional truth: {}", e))?;

        // Create a proposed update from the content
        let update = ProposedStateUpdate::from_json(&content);

        // Validate against guidance
        let validation = self.validate_against_guidance(guidance, &[update]);

        Ok(CanonPromotionResult {
            provisional_id: provisional_id.to_string(),
            can_promote: validation.is_valid,
            violations: validation.violations,
            warnings: validation.warnings,
        })
    }
}

/// Proposed state update for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedStateUpdate {
    pub update_id: String,
    pub description: String,
    pub update_kind: UpdateKind,
    pub affects_events: Vec<String>,
    pub affected_characters: Vec<String>,
    pub event_time: Option<TimeAnchor>,
    pub changes_fate_for: Vec<String>,
    pub uses_future_knowledge: bool,
}

impl ProposedStateUpdate {
    /// Create from JSON
    pub fn from_json(value: &serde_json::Value) -> Self {
        Self {
            update_id: value
                .get("update_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            description: value
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            update_kind: UpdateKind::from_str(
                value
                    .get("update_kind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("AddDetail"),
            ),
            affects_events: value
                .get("affects_events")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            affected_characters: value
                .get("affected_characters")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            event_time: None, // Would need proper parsing
            changes_fate_for: value
                .get("changes_fate_for")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            uses_future_knowledge: value
                .get("uses_future_knowledge")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateKind {
    AddDetail,
    Reveal,
    Modify,
    AddEvent,
    RemoveEvent,
}

impl UpdateKind {
    pub fn from_str(s: &str) -> Self {
        match s {
            "AddDetail" => UpdateKind::AddDetail,
            "Reveal" => UpdateKind::Reveal,
            "Modify" => UpdateKind::Modify,
            "AddEvent" => UpdateKind::AddEvent,
            "RemoveEvent" => UpdateKind::RemoveEvent,
            _ => UpdateKind::AddDetail,
        }
    }
}

/// Temporal validation result
#[derive(Debug, Clone)]
pub struct TemporalValidationResult {
    pub is_valid: bool,
    pub canon_status: TemporalCanonStatus,
    pub violations: Vec<TemporalViolation>,
    pub warnings: Vec<String>,
    pub blocked_updates: Vec<String>,
    pub allowed_updates: Vec<String>,
}

impl TemporalValidationResult {
    pub fn has_hard_conflict(&self) -> bool {
        !self.violations.is_empty()
    }
}

/// Single update validation
#[derive(Debug, Clone)]
pub struct SingleUpdateValidation {
    pub update_id: String,
    pub has_hard_violation: bool,
    pub violations: Vec<TemporalViolation>,
    pub warnings: Vec<String>,
}

/// Temporal violation
#[derive(Debug, Clone)]
pub struct TemporalViolation {
    pub violation_kind: TemporalViolationKind,
    pub message: String,
    pub update_id: String,
    pub severity: ViolationSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalViolationKind {
    HardConstraintViolation,
    FutureKnowledgeViolation,
    CharacterFateViolation,
    TemporalParadox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationSeverity {
    Hard,
    Soft,
}

/// Canon promotion result
#[derive(Debug, Clone)]
pub struct CanonPromotionResult {
    pub provisional_id: String,
    pub can_promote: bool,
    pub violations: Vec<TemporalViolation>,
    pub warnings: Vec<String>,
}

// Database row types
#[derive(sqlx::FromRow)]
struct ProvisionalTruthRow {
    provisional_id: String,
    session_id: String,
    content: String,
    created_at: String,
}
