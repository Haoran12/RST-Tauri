//! Historical truth resolver
//!
//! Generates TruthGuidance for retrospective sessions.
//! Provides constraints for past-line narratives.
//!
//! Key responsibilities:
//! - Extract open_detail_slots from HistoricalEventContent
//! - Generate hard constraints from required_outcomes/forbidden_outcomes
//! - Provide after-effects that become known after events

use sqlx::SqlitePool;

use crate::agent::models::common::generate_id;
use crate::agent::models::{
    HistoricalEventContent, OpenDetailSlot, TimeAnchor, TruthConstraint, TruthConstraintKind,
    TruthGuidance,
};

/// Historical truth resolver - generates guidance for retrospective sessions
pub struct HistoricalTruthResolver {
    pool: SqlitePool,
}

impl HistoricalTruthResolver {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Get the underlying pool for external use
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Generate truth guidance for a retrospective session
    pub async fn generate_truth_guidance(
        &self,
        session_id: &str,
        session_time_anchor: &TimeAnchor,
        mainline_time_anchor: &TimeAnchor,
        world_id: &str,
    ) -> Result<TruthGuidance, String> {
        // Determine session type
        let session_type = Self::determine_session_type(session_time_anchor, mainline_time_anchor);

        match session_type {
            SessionType::Mainline => {
                // Mainline session - no special constraints
                Ok(TruthGuidance {
                    session_id: session_id.to_string(),
                    period_anchor: session_time_anchor.clone(),
                    related_event_ids: Vec::new(),
                    hard_constraints: Vec::new(),
                    soft_context: Vec::new(),
                    open_detail_slots: Vec::new(),
                    future_knowledge_warnings: Vec::new(),
                })
            }
            SessionType::Retrospective => {
                // Past session - gather constraints
                self.generate_retrospective_guidance(
                    session_id,
                    session_time_anchor,
                    mainline_time_anchor,
                    world_id,
                )
                .await
            }
            SessionType::FuturePreview => {
                // Future session - warnings only
                self.generate_future_guidance(session_id, session_time_anchor)
                    .await
            }
        }
    }

    /// Determine session type based on time anchors
    fn determine_session_type(session: &TimeAnchor, mainline: &TimeAnchor) -> SessionType {
        match session.ordinal.cmp(&mainline.ordinal) {
            std::cmp::Ordering::Less => SessionType::Retrospective,
            std::cmp::Ordering::Greater => SessionType::FuturePreview,
            _ => SessionType::Mainline,
        }
    }

    /// Generate guidance for retrospective (past) sessions
    async fn generate_retrospective_guidance(
        &self,
        session_id: &str,
        session_time: &TimeAnchor,
        mainline_time: &TimeAnchor,
        world_id: &str,
    ) -> Result<TruthGuidance, String> {
        // Get all canonical events between session time and mainline time
        let related_event_ids = self
            .get_related_event_ids(world_id, session_time, mainline_time)
            .await?;

        // Get hard constraints from established facts
        let hard_constraints = self
            .get_hard_constraints(world_id, session_time, mainline_time)
            .await?;

        // Generate soft context
        let soft_context =
            Self::generate_soft_context(&related_event_ids, session_time, mainline_time);

        // Generate open detail slots
        let open_detail_slots = self.get_open_detail_slots(world_id, session_time).await?;

        // Generate future knowledge warnings
        let future_knowledge_warnings = Self::generate_future_warnings(session_time, mainline_time);

        Ok(TruthGuidance {
            session_id: session_id.to_string(),
            period_anchor: session_time.clone(),
            related_event_ids,
            hard_constraints,
            soft_context,
            open_detail_slots,
            future_knowledge_warnings,
        })
    }

    /// Generate guidance for future preview sessions
    async fn generate_future_guidance(
        &self,
        session_id: &str,
        session_time: &TimeAnchor,
    ) -> Result<TruthGuidance, String> {
        // Future sessions get warnings about knowledge that shouldn't exist yet
        Ok(TruthGuidance {
            session_id: session_id.to_string(),
            period_anchor: session_time.clone(),
            related_event_ids: Vec::new(),
            hard_constraints: Vec::new(),
            soft_context: vec![
                "未来预览会话不写入正史".to_string(),
                "所有状态变化仅在本会话有效".to_string(),
            ],
            open_detail_slots: Vec::new(),
            future_knowledge_warnings: vec![
                "角色无法知晓未来事件".to_string(),
                "未来线产生的知识不可用于主线".to_string(),
            ],
        })
    }

    /// Get related event IDs between session time and mainline time
    async fn get_related_event_ids(
        &self,
        world_id: &str,
        from_time: &TimeAnchor,
        to_time: &TimeAnchor,
    ) -> Result<Vec<String>, String> {
        let rows = sqlx::query_as::<_, EventIdRow>(
            r#"
            SELECT knowledge_id
            FROM knowledge_entries
            WHERE kind = 'historical_event'
            AND subject_type = 'world'
            AND subject_id = ?
            "#,
        )
        .bind(world_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to query related events: {}", e))?;

        Ok(rows.into_iter().map(|r| r.knowledge_id).collect())
    }

    /// Get hard constraints from established facts
    ///
    /// Extracts constraints from historical events:
    /// - Required outcomes (must happen)
    /// - Forbidden outcomes (cannot happen)
    /// - Known after effects (facts that become true after the event)
    async fn get_hard_constraints(
        &self,
        world_id: &str,
        from_time: &TimeAnchor,
        to_time: &TimeAnchor,
    ) -> Result<Vec<TruthConstraint>, String> {
        // Query historical events between from_time and to_time
        let rows = sqlx::query_as::<_, HistoricalEventRow>(
            r#"
            SELECT knowledge_id, content, valid_from
            FROM knowledge_entries
            WHERE kind = 'historical_event'
            AND (subject_type = 'world' OR subject_id = ?)
            ORDER BY created_at ASC
            "#,
        )
        .bind(world_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to query historical events for constraints: {}", e))?;

        let mut constraints = Vec::new();

        for row in rows {
            match serde_json::from_str::<HistoricalEventContent>(&row.content) {
                Ok(event_content) => {
                    let event_time = event_content.time_window.start.ordinal;

                    // Only include constraints for events between from_time and to_time
                    if event_time >= from_time.ordinal && event_time <= to_time.ordinal {
                        // Add required outcomes as constraints
                        for outcome in &event_content.required_outcomes {
                            constraints.push(TruthConstraint {
                                constraint_id: generate_constraint_id(
                                    &row.knowledge_id,
                                    &outcome.outcome_id,
                                ),
                                source_knowledge_id: row.knowledge_id.clone(),
                                constraint_kind: TruthConstraintKind::RequiredOutcome,
                                applies_to_refs: vec![outcome.subject_id.clone()],
                                structured_payload: serde_json::to_value(outcome)
                                    .unwrap_or(serde_json::Value::Null),
                            });
                        }

                        // Add forbidden outcomes as constraints
                        for outcome in &event_content.forbidden_outcomes {
                            constraints.push(TruthConstraint {
                                constraint_id: generate_constraint_id(
                                    &row.knowledge_id,
                                    &outcome.outcome_id,
                                ),
                                source_knowledge_id: row.knowledge_id.clone(),
                                constraint_kind: TruthConstraintKind::ForbiddenOutcome,
                                applies_to_refs: vec![outcome.subject_id.clone()],
                                structured_payload: serde_json::to_value(outcome)
                                    .unwrap_or(serde_json::Value::Null),
                            });
                        }

                        // Add known after effects as constraints
                        for after_effect in &event_content.known_after_effects {
                            constraints.push(TruthConstraint {
                                constraint_id: generate_constraint_id(
                                    &row.knowledge_id,
                                    &after_effect.fact_ref,
                                ),
                                source_knowledge_id: row.knowledge_id.clone(),
                                constraint_kind: TruthConstraintKind::KnownAfterEffect,
                                applies_to_refs: vec![after_effect.fact_ref.clone()],
                                structured_payload: serde_json::json!({
                                    "fact_ref": after_effect.fact_ref,
                                    "valid_from_ordinal": after_effect.valid_from_ordinal,
                                }),
                            });
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse historical event {}: {}",
                        row.knowledge_id,
                        e
                    );
                }
            }
        }

        Ok(constraints)
    }

    /// Generate soft context for guidance
    fn generate_soft_context(
        related_event_ids: &[String],
        session_time: &TimeAnchor,
        mainline_time: &TimeAnchor,
    ) -> Vec<String> {
        let mut context = Vec::new();

        context.push(format!(
            "过去线会话: {} → 主线 {}",
            session_time.display_text, mainline_time.display_text
        ));

        context.push("本会话产生的细节为临时设定，需审核后纳入正史".to_string());

        if !related_event_ids.is_empty() {
            context.push(format!(
                "涉及 {} 个已确立的历史事件",
                related_event_ids.len()
            ));
        }

        context
    }

    /// Get open detail slots for retrospective sessions
    ///
    /// Queries historical events and extracts their open_detail_slots.
    /// Only returns slots that haven't been filled yet.
    async fn get_open_detail_slots(
        &self,
        world_id: &str,
        session_time: &TimeAnchor,
    ) -> Result<Vec<OpenDetailSlot>, String> {
        // Query historical events that occurred before or at session time
        let rows = sqlx::query_as::<_, HistoricalEventRow>(
            r#"
            SELECT knowledge_id, content, valid_from
            FROM knowledge_entries
            WHERE kind = 'historical_event'
            AND (subject_type = 'world' OR subject_id = ?)
            ORDER BY created_at ASC
            "#,
        )
        .bind(world_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to query historical events: {}", e))?;

        let mut slots = Vec::new();

        for row in rows {
            // Parse the event content
            match serde_json::from_str::<HistoricalEventContent>(&row.content) {
                Ok(event_content) => {
                    // Check if event time is before or at session time
                    let event_time = event_content.time_window.start.ordinal;
                    if event_time <= session_time.ordinal {
                        // Extract open detail slots
                        for slot_ref in event_content.open_detail_slots {
                            // Check if this slot has already been filled
                            let filled = self
                                .is_slot_filled(&row.knowledge_id, &slot_ref.slot_id)
                                .await?;

                            if !filled {
                                slots.push(OpenDetailSlot {
                                    slot_id: slot_ref.slot_id,
                                    source_event_id: row.knowledge_id.clone(),
                                    detail_kind: slot_ref.detail_kind,
                                    promotion_policy: slot_ref.promotion_policy,
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    // Log warning but continue processing other events
                    tracing::warn!(
                        "Failed to parse historical event {}: {}",
                        row.knowledge_id,
                        e
                    );
                }
            }
        }

        Ok(slots)
    }

    /// Check if a detail slot has already been filled
    async fn is_slot_filled(&self, event_id: &str, slot_id: &str) -> Result<bool, String> {
        // Check if there's a promoted provisional truth for this slot
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM provisional_session_truth
            WHERE derived_from_event_id = ?
            AND candidate_payload->>'$.slot_id' = ?
            AND promotion_status = 'promoted'
            "#,
        )
        .bind(event_id)
        .bind(slot_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to check slot filled: {}", e))?;

        Ok(count.0 > 0)
    }

    /// Public method to get open detail slots for a session
    ///
    /// This is a convenience method for the Tauri command layer.
    pub async fn get_open_detail_slots_for_session(
        &self,
        world_id: &str,
        session_time: &TimeAnchor,
    ) -> Result<Vec<OpenDetailSlot>, String> {
        self.get_open_detail_slots(world_id, session_time).await
    }

    /// Generate future knowledge warnings
    fn generate_future_warnings(
        session_time: &TimeAnchor,
        mainline_time: &TimeAnchor,
    ) -> Vec<String> {
        let mut warnings = Vec::new();

        warnings.push(format!(
            "角色在 {} 无法知晓之后的事件",
            session_time.display_text
        ));

        if mainline_time.ordinal > session_time.ordinal {
            warnings.push(format!(
                "主线时间 {} 的事件不可被提前知晓",
                mainline_time.display_text
            ));
        }

        warnings
    }

    /// Check if a proposed change conflicts with truth guidance
    pub fn check_conflict(
        &self,
        guidance: &TruthGuidance,
        proposed_change: &ProposedChange,
    ) -> ConflictCheckResult {
        let mut conflicts = Vec::new();

        // Check against hard constraints
        for constraint in &guidance.hard_constraints {
            if proposed_change.affects_event_id == Some(constraint.source_knowledge_id.clone()) {
                conflicts.push(Conflict {
                    conflict_kind: ConflictKind::HardConstraint,
                    message: "提议与硬约束冲突".to_string(),
                    severity: ConflictSeverity::Hard,
                });
            }
        }

        // Check future knowledge warnings
        for warning in &guidance.future_knowledge_warnings {
            if proposed_change.uses_future_knowledge {
                conflicts.push(Conflict {
                    conflict_kind: ConflictKind::FutureKnowledge,
                    message: warning.clone(),
                    severity: ConflictSeverity::Hard,
                });
            }
        }

        ConflictCheckResult {
            has_hard_conflict: conflicts
                .iter()
                .any(|c| c.severity == ConflictSeverity::Hard),
            conflicts,
        }
    }
}

/// Session type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionType {
    Mainline,
    Retrospective,
    FuturePreview,
}

/// Proposed change to historical truth
#[derive(Debug, Clone)]
pub struct ProposedChange {
    pub change_id: String,
    pub description: String,
    pub affects_event_id: Option<String>,
    pub uses_future_knowledge: bool,
    pub change_kind: ChangeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    AddDetail,
    Reveal,
    Modify,
    AddEvent,
}

/// Conflict check result
#[derive(Debug, Clone)]
pub struct ConflictCheckResult {
    pub has_hard_conflict: bool,
    pub conflicts: Vec<Conflict>,
}

/// Conflict description
#[derive(Debug, Clone)]
pub struct Conflict {
    pub conflict_kind: ConflictKind,
    pub message: String,
    pub severity: ConflictSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKind {
    HardConstraint,
    FutureKnowledge,
    TemporalParadox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictSeverity {
    Hard,
    Soft,
}

// Database row types
#[derive(sqlx::FromRow)]
struct EventIdRow {
    knowledge_id: String,
}

#[derive(sqlx::FromRow)]
struct HistoricalEventRow {
    knowledge_id: String,
    content: String,
    valid_from: Option<String>,
}

/// Generate a constraint ID from event ID and outcome ID
fn generate_constraint_id(event_id: &str, outcome_id: &str) -> String {
    format!("constraint_{}_{}", event_id, outcome_id)
}
