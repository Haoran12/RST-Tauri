//! Conflict resolution for past timeline sessions
//!
//! Implements the conflict handling UX from docs/11_agent_runtime.md §6.2.
//!
//! Key principles:
//! - Conflicts do NOT interrupt gameplay
//! - User chooses: NonCanonAfterConflict or WholeSessionNonCanon
//! - OutcomePlanner still generates playable narrative
//! - StateCommitter decides what to write based on canon status

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Conflict severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictSeverity {
    /// Soft conflict - warning only, can still be canon
    Soft,
    /// Hard conflict - requires user decision
    Hard,
}

/// Conflict report for a past timeline session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictReport {
    /// Unique conflict ID
    pub conflict_id: String,
    /// Session ID where conflict occurred
    pub session_id: String,
    /// Turn ID where conflict was detected
    pub session_turn_id: String,
    /// Scene turn ID (if applicable)
    pub scene_turn_id: Option<String>,
    /// Severity of the conflict
    pub severity: ConflictSeverity,
    /// Constraint IDs that were violated
    pub source_constraint_ids: Vec<String>,
    /// Provisional truth IDs affected by this conflict
    pub affected_provisional_ids: Vec<String>,
    /// User's policy decision (if resolved)
    pub policy_decision: Option<ConflictPolicyDecision>,
    /// Human-readable summary
    pub summary: ConflictSummary,
    /// When the conflict was detected
    pub created_at: DateTime<Utc>,
    /// When the conflict was resolved (if resolved)
    pub resolved_at: Option<DateTime<Utc>>,
}

/// User's decision on how to handle the conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictPolicyDecision {
    /// Conflict point and after become non-canon
    /// Prior validated provisional truth can still be promoted
    NonCanonAfterConflict,
    /// Entire session becomes non-canon
    /// No provisional truth can be promoted
    WholeSessionNonCanon,
}

/// Human-readable conflict summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictSummary {
    /// Brief description of what conflicted
    pub title: String,
    /// Detailed explanation of the conflict
    pub description: String,
    /// What would happen with NonCanonAfterConflict
    pub non_canon_after_impact: String,
    /// What would happen with WholeSessionNonCanon
    pub whole_session_impact: String,
    /// Affected story elements
    pub affected_elements: Vec<String>,
}

/// Conflict resolution request (sent to frontend)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolutionRequest {
    /// Request ID for tracking
    pub request_id: String,
    /// Session ID
    pub session_id: String,
    /// Conflicts to resolve
    pub conflicts: Vec<ConflictReport>,
    /// Default recommendation
    pub recommendation: ConflictPolicyDecision,
    /// Reason for recommendation
    pub recommendation_reason: String,
    /// When the request was created
    pub created_at: DateTime<Utc>,
}

/// Conflict resolution response (from frontend/user)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolutionResponse {
    /// Request ID being responded to
    pub request_id: String,
    /// User's decision
    pub decision: ConflictPolicyDecision,
    /// Optional user note
    pub user_note: Option<String>,
    /// When the response was received
    pub responded_at: DateTime<Utc>,
}

/// Conflict manager for tracking and resolving conflicts
#[derive(Debug, Clone, Default)]
pub struct ConflictManager {
    /// Pending conflicts awaiting resolution
    pending_conflicts: HashMap<String, ConflictReport>,
    /// Resolved conflicts
    resolved_conflicts: HashMap<String, ConflictReport>,
    /// Session conflict status
    session_status: HashMap<String, SessionConflictStatus>,
}

/// Conflict status for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConflictStatus {
    /// Session ID
    pub session_id: String,
    /// Whether the session has any hard conflicts
    pub has_hard_conflicts: bool,
    /// Overall policy (if decided)
    pub overall_policy: Option<ConflictPolicyDecision>,
    /// Turn ID where first conflict occurred
    pub first_conflict_turn_id: Option<String>,
    /// Number of conflicts
    pub conflict_count: usize,
}

impl ConflictManager {
    /// Create a new conflict manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Detect and record a conflict
    pub fn record_conflict(&mut self, report: ConflictReport) -> Result<(), String> {
        let session_id = report.session_id.clone();
        let is_hard = report.severity == ConflictSeverity::Hard;

        // Update session status
        let status =
            self.session_status
                .entry(session_id.clone())
                .or_insert(SessionConflictStatus {
                    session_id: session_id.clone(),
                    has_hard_conflicts: false,
                    overall_policy: None,
                    first_conflict_turn_id: None,
                    conflict_count: 0,
                });

        if is_hard {
            status.has_hard_conflicts = true;
            if status.first_conflict_turn_id.is_none() {
                status.first_conflict_turn_id = Some(report.session_turn_id.clone());
            }
        }
        status.conflict_count += 1;

        // Store the conflict
        if report.policy_decision.is_some() {
            self.resolved_conflicts
                .insert(report.conflict_id.clone(), report);
        } else {
            self.pending_conflicts
                .insert(report.conflict_id.clone(), report);
        }

        Ok(())
    }

    /// Apply a user's resolution decision
    pub fn apply_resolution(
        &mut self,
        response: &ConflictResolutionResponse,
    ) -> Result<(), String> {
        // Find all pending conflicts for this request
        let conflicts_to_resolve: Vec<String> = self
            .pending_conflicts
            .values()
            .filter(|c| c.session_id == response.request_id.split(':').next().unwrap_or(""))
            .map(|c| c.conflict_id.clone())
            .collect();

        let now = Utc::now();
        for conflict_id in conflicts_to_resolve {
            if let Some(mut conflict) = self.pending_conflicts.remove(&conflict_id) {
                conflict.policy_decision = Some(response.decision);
                conflict.resolved_at = Some(now);
                self.resolved_conflicts.insert(conflict_id, conflict);
            }
        }

        // Update session status
        if let Some(session_id) = response.request_id.split(':').next() {
            if let Some(status) = self.session_status.get_mut(session_id) {
                status.overall_policy = Some(response.decision);
            }
        }

        Ok(())
    }

    /// Get pending conflicts for a session
    pub fn get_pending_conflicts(&self, session_id: &str) -> Vec<&ConflictReport> {
        self.pending_conflicts
            .values()
            .filter(|c| c.session_id == session_id)
            .collect()
    }

    /// Get all conflicts for a session
    pub fn get_session_conflicts(&self, session_id: &str) -> Vec<&ConflictReport> {
        let mut conflicts: Vec<&ConflictReport> = Vec::new();
        conflicts.extend(
            self.pending_conflicts
                .values()
                .filter(|c| c.session_id == session_id),
        );
        conflicts.extend(
            self.resolved_conflicts
                .values()
                .filter(|c| c.session_id == session_id),
        );
        conflicts.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        conflicts
    }

    /// Get session conflict status
    pub fn get_session_status(&self, session_id: &str) -> Option<&SessionConflictStatus> {
        self.session_status.get(session_id)
    }

    /// Check if a session has unresolved hard conflicts
    pub fn has_unresolved_conflicts(&self, session_id: &str) -> bool {
        self.pending_conflicts
            .values()
            .any(|c| c.session_id == session_id && c.severity == ConflictSeverity::Hard)
    }

    /// Create a resolution request for the frontend
    pub fn create_resolution_request(&self, session_id: &str) -> Option<ConflictResolutionRequest> {
        let pending: Vec<_> = self
            .get_pending_conflicts(session_id)
            .into_iter()
            .cloned()
            .collect();

        if pending.is_empty() {
            return None;
        }

        // Determine recommendation based on conflict severity and count
        let hard_count = pending
            .iter()
            .filter(|c| c.severity == ConflictSeverity::Hard)
            .count();
        let recommendation = if hard_count > 1 {
            ConflictPolicyDecision::WholeSessionNonCanon
        } else {
            ConflictPolicyDecision::NonCanonAfterConflict
        };

        let reason = match recommendation {
            ConflictPolicyDecision::NonCanonAfterConflict => {
                "Single conflict detected. Marking conflict point and after as non-canon allows earlier content to remain canonical.".to_string()
            }
            ConflictPolicyDecision::WholeSessionNonCanon => {
                "Multiple conflicts detected. Recommend marking entire session as non-canon for consistency.".to_string()
            }
        };

        Some(ConflictResolutionRequest {
            request_id: format!("{}:conflict_request", session_id),
            session_id: session_id.to_string(),
            conflicts: pending,
            recommendation,
            recommendation_reason: reason,
            created_at: Utc::now(),
        })
    }

    /// Determine canon status for a turn based on conflict policy
    pub fn determine_turn_canon_status(&self, session_id: &str, turn_id: &str) -> TurnCanonStatus {
        let status = match self.session_status.get(session_id) {
            Some(s) => s,
            None => return TurnCanonStatus::CanonCandidate,
        };

        match status.overall_policy {
            Some(ConflictPolicyDecision::WholeSessionNonCanon) => TurnCanonStatus::NonCanon,
            Some(ConflictPolicyDecision::NonCanonAfterConflict) => {
                // Check if this turn is after the first conflict
                if let Some(ref conflict_turn_id) = status.first_conflict_turn_id {
                    if turn_id >= conflict_turn_id.as_str() {
                        TurnCanonStatus::ConflictWarned
                    } else {
                        TurnCanonStatus::CanonCandidate
                    }
                } else {
                    TurnCanonStatus::CanonCandidate
                }
            }
            None => {
                if status.has_hard_conflicts {
                    TurnCanonStatus::ConflictWarned
                } else {
                    TurnCanonStatus::CanonCandidate
                }
            }
        }
    }
}

/// Turn canon status (for session turns)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnCanonStatus {
    /// Turn is a candidate for canon
    CanonCandidate,
    /// Turn has been promoted to canon
    CanonPromoted,
    /// Turn has a conflict warning
    ConflictWarned,
    /// Turn is non-canon
    NonCanon,
}

impl ConflictReport {
    /// Create a new conflict report
    pub fn new(
        session_id: &str,
        session_turn_id: &str,
        severity: ConflictSeverity,
        source_constraint_ids: Vec<String>,
        summary: ConflictSummary,
    ) -> Self {
        Self {
            conflict_id: generate_conflict_id(),
            session_id: session_id.to_string(),
            session_turn_id: session_turn_id.to_string(),
            scene_turn_id: None,
            severity,
            source_constraint_ids,
            affected_provisional_ids: Vec::new(),
            policy_decision: None,
            summary,
            created_at: Utc::now(),
            resolved_at: None,
        }
    }

    /// Add affected provisional truth IDs
    pub fn with_affected_provisional_ids(mut self, ids: Vec<String>) -> Self {
        self.affected_provisional_ids = ids;
        self
    }

    /// Add scene turn ID
    pub fn with_scene_turn_id(mut self, id: &str) -> Self {
        self.scene_turn_id = Some(id.to_string());
        self
    }
}

impl ConflictSummary {
    /// Create a new conflict summary
    pub fn new(
        title: &str,
        description: &str,
        non_canon_after_impact: &str,
        whole_session_impact: &str,
    ) -> Self {
        Self {
            title: title.to_string(),
            description: description.to_string(),
            non_canon_after_impact: non_canon_after_impact.to_string(),
            whole_session_impact: whole_session_impact.to_string(),
            affected_elements: Vec::new(),
        }
    }

    /// Add affected elements
    pub fn with_affected_elements(mut self, elements: Vec<String>) -> Self {
        self.affected_elements = elements;
        self
    }
}

fn generate_conflict_id() -> String {
    format!("conflict_{}", uuid::Uuid::new_v4())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_conflict() {
        let mut manager = ConflictManager::new();
        let report = ConflictReport::new(
            "session_1",
            "turn_1",
            ConflictSeverity::Hard,
            vec!["constraint_1".to_string()],
            ConflictSummary::new(
                "Test conflict",
                "A test conflict occurred",
                "After becomes non-canon",
                "Entire session non-canon",
            ),
        );

        assert!(manager.record_conflict(report).is_ok());
        assert!(manager.has_unresolved_conflicts("session_1"));
    }

    #[test]
    fn applies_resolution() {
        let mut manager = ConflictManager::new();
        let report = ConflictReport::new(
            "session_1",
            "turn_1",
            ConflictSeverity::Hard,
            vec!["constraint_1".to_string()],
            ConflictSummary::new(
                "Test conflict",
                "A test conflict occurred",
                "After becomes non-canon",
                "Entire session non-canon",
            ),
        );

        manager.record_conflict(report).unwrap();

        let response = ConflictResolutionResponse {
            request_id: "session_1:conflict_request".to_string(),
            decision: ConflictPolicyDecision::NonCanonAfterConflict,
            user_note: None,
            responded_at: Utc::now(),
        };

        assert!(manager.apply_resolution(&response).is_ok());
        assert!(!manager.has_unresolved_conflicts("session_1"));

        let status = manager.get_session_status("session_1").unwrap();
        assert_eq!(
            status.overall_policy,
            Some(ConflictPolicyDecision::NonCanonAfterConflict)
        );
    }

    #[test]
    fn determines_turn_canon_status() {
        let mut manager = ConflictManager::new();
        let report = ConflictReport::new(
            "session_1",
            "turn_2",
            ConflictSeverity::Hard,
            vec!["constraint_1".to_string()],
            ConflictSummary::new(
                "Test conflict",
                "A test conflict occurred",
                "After becomes non-canon",
                "Entire session non-canon",
            ),
        );

        manager.record_conflict(report).unwrap();

        let response = ConflictResolutionResponse {
            request_id: "session_1:conflict_request".to_string(),
            decision: ConflictPolicyDecision::NonCanonAfterConflict,
            user_note: None,
            responded_at: Utc::now(),
        };

        manager.apply_resolution(&response).unwrap();

        // Turn before conflict should be canon candidate
        assert_eq!(
            manager.determine_turn_canon_status("session_1", "turn_1"),
            TurnCanonStatus::CanonCandidate
        );

        // Turn at/after conflict should be conflict warned
        assert_eq!(
            manager.determine_turn_canon_status("session_1", "turn_2"),
            TurnCanonStatus::ConflictWarned
        );
    }
}
