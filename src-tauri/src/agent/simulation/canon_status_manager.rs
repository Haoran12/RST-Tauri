//! Canon status manager
//!
//! Manages canon status for sessions and turns:
//! - Determines canon eligibility for retrospective sessions
//! - Handles promotion of provisional truths to canon
//! - Updates session and turn canon status
//!
//! See docs/16_agent_timeline_and_canon.md for the full specification.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::agent::models::common::generate_id;
use crate::agent::models::{
    AgentSession, AgentSessionKind, ConflictPolicyDecision, ConflictReport, ConflictSeverity,
    PromotionStatus, ProvisionalSessionTruth, RuntimeTurnCanonStatus, SessionCanonStatus,
    TimeAnchor, TruthConstraint, TruthConstraintKind, TruthGuidance,
};

use super::provisional_truth_manager::ProvisionalTruthManager;
use super::temporal_consistency_validator::{
    ProposedStateUpdate, TemporalConsistencyValidator, TemporalViolation,
};

/// Canon status manager
///
/// Handles canon status determination and promotion for retrospective sessions.
pub struct CanonStatusManager {
    pool: SqlitePool,
}

impl CanonStatusManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Determine canon eligibility for a session
    ///
    /// Returns the appropriate canon status based on:
    /// - Session kind (Mainline/Retrospective/FuturePreview)
    /// - Existing conflict reports
    /// - Provisional truth promotion status
    pub async fn determine_session_canon_status(
        &self,
        session: &AgentSession,
    ) -> Result<SessionCanonStatus, String> {
        match session.session_kind {
            AgentSessionKind::Mainline => {
                // Mainline sessions are canon candidates by default
                Ok(session.canon_status)
            }
            AgentSessionKind::Retrospective => {
                // Check for existing conflicts
                let has_conflicts = self.has_session_conflicts(&session.session_id).await?;
                let has_promoted = self
                    .has_promoted_provisional_truths(&session.session_id)
                    .await?;

                if has_conflicts {
                    // Check conflict policy
                    match &session.conflict_policy {
                        Some(ConflictPolicyDecision::WholeSessionNonCanon) => {
                            Ok(SessionCanonStatus::NonCanon)
                        }
                        Some(ConflictPolicyDecision::NonCanonAfterConflict) => {
                            if has_promoted {
                                Ok(SessionCanonStatus::PartiallyCanon)
                            } else {
                                Ok(SessionCanonStatus::NonCanon)
                            }
                        }
                        None => {
                            // Default: conflict makes it non-canon
                            Ok(SessionCanonStatus::NonCanon)
                        }
                    }
                } else {
                    // No conflicts, check if any provisional truths are pending
                    let pending_count = self
                        .count_pending_provisional_truths(&session.session_id)
                        .await?;
                    if pending_count > 0 {
                        Ok(SessionCanonStatus::CanonCandidate)
                    } else if has_promoted {
                        Ok(SessionCanonStatus::CanonCandidate)
                    } else {
                        Ok(SessionCanonStatus::CanonCandidate)
                    }
                }
            }
            AgentSessionKind::FuturePreview => {
                // Future preview is always non-canon
                Ok(SessionCanonStatus::NonCanon)
            }
        }
    }

    /// Check if a session has any conflict reports
    async fn has_session_conflicts(&self, session_id: &str) -> Result<bool, String> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM conflict_reports
            WHERE session_id = ? AND severity = 'hard'
            "#,
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to check session conflicts: {}", e))?;

        Ok(count.0 > 0)
    }

    /// Check if a session has any promoted provisional truths
    async fn has_promoted_provisional_truths(&self, session_id: &str) -> Result<bool, String> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM provisional_session_truth
            WHERE session_id = ? AND promotion_status = 'promoted'
            "#,
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to check promoted truths: {}", e))?;

        Ok(count.0 > 0)
    }

    /// Count pending provisional truths for a session
    async fn count_pending_provisional_truths(&self, session_id: &str) -> Result<i64, String> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM provisional_session_truth
            WHERE session_id = ? AND promotion_status = 'pending'
            "#,
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to count pending truths: {}", e))?;

        Ok(count.0)
    }

    /// Evaluate provisional truths for canon promotion
    ///
    /// This is the main entry point for the promotion workflow.
    /// It validates all pending provisional truths against truth guidance
    /// and determines which can be promoted.
    pub async fn evaluate_for_promotion(
        &self,
        session: &AgentSession,
        guidance: &TruthGuidance,
    ) -> Result<PromotionEvaluationResult, String> {
        // Get all pending provisional truths
        let manager = ProvisionalTruthManager::new(self.pool.clone());
        let pending = manager.get_pending_candidates(&session.session_id).await?;

        let mut promotable = Vec::new();
        let mut blocked = Vec::new();
        let mut warnings = Vec::new();

        let validator = TemporalConsistencyValidator::new(self.pool.clone());

        for candidate in pending {
            // Create proposed update from candidate
            let update = self.candidate_to_update(&candidate);

            // Validate against guidance
            let validation = validator.validate_against_guidance(guidance, &[update]);

            if validation.is_valid {
                promotable.push(PromotableCandidate {
                    provisional_id: candidate.provisional_id.clone(),
                    candidate_kind: candidate.candidate_kind,
                    confidence: 1.0, // Would be computed from validation
                    warnings: validation.warnings,
                });
            } else {
                blocked.push(BlockedCandidate {
                    provisional_id: candidate.provisional_id.clone(),
                    violations: validation
                        .violations
                        .iter()
                        .map(|v| v.message.clone())
                        .collect(),
                });
            }
        }

        // Check for soft warnings
        if !guidance.soft_context.is_empty() {
            warnings.extend(guidance.soft_context.clone());
        }

        Ok(PromotionEvaluationResult {
            session_id: session.session_id.clone(),
            promotable_count: promotable.len(),
            blocked_count: blocked.len(),
            promotable,
            blocked,
            warnings,
            evaluated_at: Utc::now(),
        })
    }

    /// Convert a provisional candidate to a proposed update
    fn candidate_to_update(&self, candidate: &ProvisionalSessionTruth) -> ProposedStateUpdate {
        ProposedStateUpdate {
            update_id: candidate.provisional_id.clone(),
            description: format!("Provisional candidate {:?}", candidate.candidate_kind),
            update_kind: super::temporal_consistency_validator::UpdateKind::AddDetail,
            affects_events: candidate
                .derived_from_event_id
                .as_ref()
                .map(|id| vec![id.clone()])
                .unwrap_or_default(),
            affected_characters: Vec::new(),
            event_time: Some(candidate.story_time_anchor.clone()),
            changes_fate_for: Vec::new(),
            uses_future_knowledge: false,
        }
    }

    /// Promote eligible provisional truths to canon
    ///
    /// This performs the actual promotion:
    /// 1. Validates each candidate against current guidance
    /// 2. Creates canonical knowledge entries
    /// 3. Updates provisional status
    /// 4. Updates session canon status
    pub async fn promote_eligible_truths(
        &self,
        session: &AgentSession,
        guidance: &TruthGuidance,
        scene_turn_id: &str,
    ) -> Result<PromotionResult, String> {
        // Evaluate which truths can be promoted
        let evaluation = self.evaluate_for_promotion(session, guidance).await?;

        let mut promoted_ids = Vec::new();
        let mut failed_ids = Vec::new();

        let manager = ProvisionalTruthManager::new(self.pool.clone());

        // Promote each eligible candidate
        for promotable in &evaluation.promotable {
            match manager
                .batch_promote(&[promotable.provisional_id.clone()], scene_turn_id)
                .await
            {
                Ok(ids) => {
                    promoted_ids.extend(ids);
                }
                Err(e) => {
                    tracing::warn!("Failed to promote {}: {}", promotable.provisional_id, e);
                    failed_ids.push(promotable.provisional_id.clone());
                }
            }
        }

        // Update session canon status
        let new_status = self.determine_session_canon_status(session).await?;
        self.update_session_canon_status(&session.session_id, new_status)
            .await?;

        Ok(PromotionResult {
            session_id: session.session_id.clone(),
            promoted_count: promoted_ids.len(),
            failed_count: failed_ids.len(),
            promoted_ids,
            failed_ids,
            new_session_status: new_status,
            promoted_at: Utc::now(),
        })
    }

    /// Update session canon status in database
    async fn update_session_canon_status(
        &self,
        session_id: &str,
        new_status: SessionCanonStatus,
    ) -> Result<(), String> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE agent_sessions
            SET canon_status = ?, updated_at = ?
            WHERE session_id = ?
            "#,
        )
        .bind(session_canon_status_to_str(&new_status))
        .bind(now.to_rfc3339())
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update session canon status: {}", e))?;

        Ok(())
    }

    /// Record a conflict and update session status
    pub async fn record_conflict(
        &self,
        session_id: &str,
        session_turn_id: &str,
        scene_turn_id: Option<&str>,
        violations: &[TemporalViolation],
        policy: ConflictPolicyDecision,
    ) -> Result<ConflictReport, String> {
        let now = Utc::now();
        let conflict_id = generate_id("conflict");

        let severity = if violations
            .iter()
            .any(|v| v.severity == super::temporal_consistency_validator::ViolationSeverity::Hard)
        {
            ConflictSeverity::Hard
        } else {
            ConflictSeverity::Soft
        };

        let report = ConflictReport {
            conflict_id: conflict_id.clone(),
            session_id: session_id.to_string(),
            session_turn_id: session_turn_id.to_string(),
            scene_turn_id: scene_turn_id.map(|s| s.to_string()),
            severity,
            source_constraint_ids: violations.iter().map(|v| v.update_id.clone()).collect(),
            affected_provisional_ids: Vec::new(),
            policy_decision: Some(policy),
            summary: serde_json::json!({
                "violations": violations.iter().map(|v| &v.message).collect::<Vec<_>>(),
                "policy": format!("{:?}", policy),
            }),
            created_at: now,
            resolved_at: None,
        };

        // Insert conflict report
        let summary_json = serde_json::to_string(&report.summary)
            .map_err(|e| format!("Failed to serialize summary: {}", e))?;

        sqlx::query(
            r#"
            INSERT INTO conflict_reports (
                conflict_id, session_id, session_turn_id, scene_turn_id,
                severity, source_constraint_ids, affected_provisional_ids,
                policy_decision, summary, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&report.conflict_id)
        .bind(&report.session_id)
        .bind(&report.session_turn_id)
        .bind(&report.scene_turn_id)
        .bind(conflict_severity_to_str(&report.severity))
        .bind(&serde_json::to_string(&report.source_constraint_ids).unwrap())
        .bind(&serde_json::to_string(&report.affected_provisional_ids).unwrap())
        .bind(conflict_policy_to_str(&policy))
        .bind(&summary_json)
        .bind(report.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to insert conflict report: {}", e))?;

        // Update session conflict policy if not set
        sqlx::query(
            r#"
            UPDATE agent_sessions
            SET conflict_policy = COALESCE(conflict_policy, ?),
                canon_status = CASE
                    WHEN ? = 'whole_session_noncanon' THEN 'noncanon'
                    ELSE canon_status
                END,
                updated_at = ?
            WHERE session_id = ?
            "#,
        )
        .bind(conflict_policy_to_str(&policy))
        .bind(conflict_policy_to_str(&policy))
        .bind(now.to_rfc3339())
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update session: {}", e))?;

        Ok(report)
    }

    /// Get all conflict reports for a session
    pub async fn get_session_conflicts(
        &self,
        session_id: &str,
    ) -> Result<Vec<ConflictReport>, String> {
        let rows = sqlx::query_as::<_, ConflictReportRow>(
            r#"
            SELECT conflict_id, session_id, session_turn_id, scene_turn_id,
                   severity, source_constraint_ids, affected_provisional_ids,
                   policy_decision, summary, created_at, resolved_at
            FROM conflict_reports
            WHERE session_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to get conflict reports: {}", e))?;

        rows.into_iter()
            .map(|r| self.row_to_conflict_report(r))
            .collect()
    }

    /// Convert database row to conflict report
    fn row_to_conflict_report(&self, row: ConflictReportRow) -> Result<ConflictReport, String> {
        Ok(ConflictReport {
            conflict_id: row.conflict_id,
            session_id: row.session_id,
            session_turn_id: row.session_turn_id,
            scene_turn_id: row.scene_turn_id,
            severity: str_to_conflict_severity(&row.severity),
            source_constraint_ids: serde_json::from_str(&row.source_constraint_ids)
                .unwrap_or_default(),
            affected_provisional_ids: serde_json::from_str(&row.affected_provisional_ids)
                .unwrap_or_default(),
            policy_decision: row
                .policy_decision
                .as_ref()
                .map(|s| str_to_conflict_policy(s))
                .transpose()?,
            summary: serde_json::from_str(&row.summary).unwrap_or(serde_json::Value::Null),
            created_at: parse_datetime(&row.created_at)?,
            resolved_at: row
                .resolved_at
                .as_ref()
                .map(|s| parse_datetime(s))
                .transpose()?,
        })
    }
}

// =============================================================================
// Result Types
// =============================================================================

/// Result of evaluating provisional truths for promotion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionEvaluationResult {
    pub session_id: String,
    pub promotable_count: usize,
    pub blocked_count: usize,
    pub promotable: Vec<PromotableCandidate>,
    pub blocked: Vec<BlockedCandidate>,
    pub warnings: Vec<String>,
    pub evaluated_at: DateTime<Utc>,
}

/// A candidate that can be promoted to canon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotableCandidate {
    pub provisional_id: String,
    pub candidate_kind: crate::agent::models::session::ProvisionalCandidateKind,
    pub confidence: f64,
    pub warnings: Vec<String>,
}

/// A candidate that is blocked from promotion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedCandidate {
    pub provisional_id: String,
    pub violations: Vec<String>,
}

/// Result of promoting provisional truths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionResult {
    pub session_id: String,
    pub promoted_count: usize,
    pub failed_count: usize,
    pub promoted_ids: Vec<String>,
    pub failed_ids: Vec<String>,
    pub new_session_status: SessionCanonStatus,
    pub promoted_at: DateTime<Utc>,
}

// =============================================================================
// Helper Functions
// =============================================================================

fn session_canon_status_to_str(status: &SessionCanonStatus) -> &'static str {
    match status {
        SessionCanonStatus::CanonCandidate => "canon_candidate",
        SessionCanonStatus::PartiallyCanon => "partially_canon",
        SessionCanonStatus::NonCanon => "noncanon",
    }
}

fn conflict_severity_to_str(severity: &ConflictSeverity) -> &'static str {
    match severity {
        ConflictSeverity::Soft => "soft",
        ConflictSeverity::Hard => "hard",
    }
}

fn str_to_conflict_severity(s: &str) -> ConflictSeverity {
    match s {
        "hard" => ConflictSeverity::Hard,
        _ => ConflictSeverity::Soft,
    }
}

fn conflict_policy_to_str(policy: &ConflictPolicyDecision) -> &'static str {
    match policy {
        ConflictPolicyDecision::NonCanonAfterConflict => "noncanon_after_conflict",
        ConflictPolicyDecision::WholeSessionNonCanon => "whole_session_noncanon",
    }
}

fn str_to_conflict_policy(s: &str) -> Result<ConflictPolicyDecision, String> {
    match s {
        "noncanon_after_conflict" => Ok(ConflictPolicyDecision::NonCanonAfterConflict),
        "whole_session_noncanon" => Ok(ConflictPolicyDecision::WholeSessionNonCanon),
        _ => Err(format!("Unknown conflict policy: {}", s)),
    }
}

fn parse_datetime(s: &str) -> Result<DateTime<Utc>, String> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| format!("Failed to parse datetime: {}", e))
}

// =============================================================================
// Database Row Types
// =============================================================================

#[derive(sqlx::FromRow)]
struct ConflictReportRow {
    conflict_id: String,
    session_id: String,
    session_turn_id: String,
    scene_turn_id: Option<String>,
    severity: String,
    source_constraint_ids: String,
    affected_provisional_ids: String,
    policy_decision: Option<String>,
    summary: String,
    created_at: String,
    resolved_at: Option<String>,
}
