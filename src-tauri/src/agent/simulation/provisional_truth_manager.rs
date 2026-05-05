//! Provisional truth manager
//!
//! Manages candidate facts from retrospective sessions:
//! - Creation of provisional truth candidates
//! - Validation against truth guidance constraints
//! - Promotion to canonical knowledge entries
//!
//! See docs/16_agent_timeline_and_canon.md for the full specification.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::agent::models::*;

/// Provisional truth manager
///
/// Handles the lifecycle of candidate facts from past timeline sessions.
pub struct ProvisionalTruthManager {
    pool: SqlitePool,
}

impl ProvisionalTruthManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new provisional truth candidate
    pub async fn create_candidate(
        &self,
        session_id: &str,
        session_turn_id: &str,
        scene_turn_id: Option<&str>,
        story_time_anchor: &TimeAnchor,
        derived_from_event_id: Option<&str>,
        detail_slot_id: Option<&str>,
        candidate_kind: ProvisionalCandidateKind,
        candidate_payload: serde_json::Value,
    ) -> Result<ProvisionalSessionTruth, String> {
        let now = Utc::now();
        let provisional_id = generate_id("prov");

        let candidate = ProvisionalSessionTruth {
            provisional_id: provisional_id.clone(),
            session_id: session_id.to_string(),
            source_session_turn_id: session_turn_id.to_string(),
            source_scene_turn_id: scene_turn_id.map(|s| s.to_string()),
            story_time_anchor: story_time_anchor.clone(),
            derived_from_event_id: derived_from_event_id.map(|s| s.to_string()),
            candidate_kind,
            candidate_payload,
            promotion_status: PromotionStatus::Pending,
            promoted_knowledge_id: None,
            promoted_scene_turn_id: None,
            created_at: now,
            updated_at: now,
        };

        // Persist to database
        self.insert_candidate(&candidate).await?;

        Ok(candidate)
    }

    /// Insert candidate into database
    async fn insert_candidate(&self, candidate: &ProvisionalSessionTruth) -> Result<(), String> {
        let story_time_json = serde_json::to_string(&candidate.story_time_anchor)
            .map_err(|e| format!("Failed to serialize story_time_anchor: {}", e))?;
        let payload_json = serde_json::to_string(&candidate.candidate_payload)
            .map_err(|e| format!("Failed to serialize candidate_payload: {}", e))?;

        sqlx::query(
            r#"
            INSERT INTO provisional_session_truth (
                provisional_id, session_id, source_session_turn_id, source_scene_turn_id,
                story_time_anchor, derived_from_event_id, candidate_kind, candidate_payload,
                promotion_status, promoted_knowledge_id, promoted_scene_turn_id,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL, ?, ?)
            "#,
        )
        .bind(&candidate.provisional_id)
        .bind(&candidate.session_id)
        .bind(&candidate.source_session_turn_id)
        .bind(&candidate.source_scene_turn_id)
        .bind(&story_time_json)
        .bind(&candidate.derived_from_event_id)
        .bind(candidate_kind_to_str(&candidate.candidate_kind))
        .bind(&payload_json)
        .bind(promotion_status_to_str(&candidate.promotion_status))
        .bind(candidate.created_at.to_rfc3339())
        .bind(candidate.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to insert provisional candidate: {}", e))?;

        Ok(())
    }

    /// Get pending candidates for a session
    pub async fn get_pending_candidates(
        &self,
        session_id: &str,
    ) -> Result<Vec<ProvisionalSessionTruth>, String> {
        let rows = sqlx::query_as::<_, ProvisionalRow>(
            r#"
            SELECT provisional_id, session_id, source_session_turn_id, source_scene_turn_id,
                   story_time_anchor, derived_from_event_id, candidate_kind, candidate_payload,
                   promotion_status, promoted_knowledge_id, promoted_scene_turn_id,
                   created_at, updated_at
            FROM provisional_session_truth
            WHERE session_id = ? AND promotion_status = 'pending'
            ORDER BY created_at ASC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to query pending candidates: {}", e))?;

        rows.into_iter().map(|r| self.row_to_candidate(r)).collect()
    }

    /// Get all candidates for a session
    pub async fn get_session_candidates(
        &self,
        session_id: &str,
    ) -> Result<Vec<ProvisionalSessionTruth>, String> {
        let rows = sqlx::query_as::<_, ProvisionalRow>(
            r#"
            SELECT provisional_id, session_id, source_session_turn_id, source_scene_turn_id,
                   story_time_anchor, derived_from_event_id, candidate_kind, candidate_payload,
                   promotion_status, promoted_knowledge_id, promoted_scene_turn_id,
                   created_at, updated_at
            FROM provisional_session_truth
            WHERE session_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to query session candidates: {}", e))?;

        rows.into_iter().map(|r| self.row_to_candidate(r)).collect()
    }

    /// Promote a candidate to canonical knowledge
    pub async fn promote_candidate(
        &self,
        provisional_id: &str,
        knowledge_entry: &KnowledgeEntry,
        scene_turn_id: &str,
    ) -> Result<(), String> {
        let now = Utc::now();

        // Begin transaction
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to begin transaction: {}", e))?;

        // Insert the knowledge entry
        let content_json = serde_json::to_string(&knowledge_entry.content)
            .map_err(|e| format!("Failed to serialize content: {}", e))?;
        let apparent_content_json = knowledge_entry
            .apparent_content
            .as_ref()
            .map(|ac| serde_json::to_string(ac))
            .transpose()
            .map_err(|e| format!("Failed to serialize apparent_content: {}", e))?;
        let access_policy_json = serde_json::to_string(&knowledge_entry.access_policy)
            .map_err(|e| format!("Failed to serialize access_policy: {}", e))?;
        let subject_awareness_json = serde_json::to_string(&knowledge_entry.subject_awareness)
            .map_err(|e| format!("Failed to serialize subject_awareness: {}", e))?;
        let metadata_json = serde_json::to_string(&knowledge_entry.metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
        let valid_from_json = knowledge_entry
            .valid_from
            .as_ref()
            .map(|vf| serde_json::to_string(vf))
            .transpose()
            .map_err(|e| format!("Failed to serialize valid_from: {}", e))?;
        let valid_until_json = knowledge_entry
            .valid_until
            .as_ref()
            .map(|vu| serde_json::to_string(vu))
            .transpose()
            .map_err(|e| format!("Failed to serialize valid_until: {}", e))?;

        let (subject_type, subject_id, facet_type) = match &knowledge_entry.subject {
            KnowledgeSubject::World => ("world", None, None),
            KnowledgeSubject::Region(id) => ("region", Some(id.as_str()), None),
            KnowledgeSubject::Faction(id) => ("faction", Some(id.as_str()), None),
            KnowledgeSubject::Character { id, facet } => (
                "character",
                Some(id.as_str()),
                Some(facet_type_to_str(facet)),
            ),
            KnowledgeSubject::Event { event_id } => ("event", Some(event_id.as_str()), None),
        };

        sqlx::query(
            r#"
            INSERT INTO knowledge_entries (
                knowledge_id, kind, subject_type, subject_id, facet_type,
                content, apparent_content, access_policy, subject_awareness,
                metadata, valid_from, valid_until, source_session_id,
                source_scene_turn_id, derived_from_event_id, schema_version,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, '0.1', ?, ?)
            "#,
        )
        .bind(&knowledge_entry.knowledge_id)
        .bind(kind_to_str(&knowledge_entry.kind))
        .bind(subject_type)
        .bind(subject_id)
        .bind(facet_type)
        .bind(&content_json)
        .bind(&apparent_content_json)
        .bind(&access_policy_json)
        .bind(&subject_awareness_json)
        .bind(&metadata_json)
        .bind(&valid_from_json)
        .bind(&valid_until_json)
        .bind(&knowledge_entry.source_session_id)
        .bind(&knowledge_entry.source_scene_turn_id)
        .bind(&knowledge_entry.derived_from_event_id)
        .bind(knowledge_entry.created_at.to_rfc3339())
        .bind(knowledge_entry.updated_at.to_rfc3339())
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to insert knowledge entry: {}", e))?;

        // Update the provisional candidate status
        sqlx::query(
            r#"
            UPDATE provisional_session_truth
            SET promotion_status = 'promoted',
                promoted_knowledge_id = ?,
                promoted_scene_turn_id = ?,
                updated_at = ?
            WHERE provisional_id = ?
            "#,
        )
        .bind(&knowledge_entry.knowledge_id)
        .bind(scene_turn_id)
        .bind(now.to_rfc3339())
        .bind(provisional_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to update provisional status: {}", e))?;

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;

        Ok(())
    }

    /// Mark a candidate as blocked due to conflict
    pub async fn block_candidate(&self, provisional_id: &str, reason: &str) -> Result<(), String> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE provisional_session_truth
            SET promotion_status = 'blocked_conflict',
                candidate_payload = json_set(candidate_payload, '$._block_reason', ?),
                updated_at = ?
            WHERE provisional_id = ?
            "#,
        )
        .bind(reason)
        .bind(now.to_rfc3339())
        .bind(provisional_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to block candidate: {}", e))?;

        Ok(())
    }

    /// Mark a candidate as non-canon
    pub async fn mark_non_canon(&self, provisional_id: &str) -> Result<(), String> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE provisional_session_truth
            SET promotion_status = 'noncanon',
                updated_at = ?
            WHERE provisional_id = ?
            "#,
        )
        .bind(now.to_rfc3339())
        .bind(provisional_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to mark candidate as non-canon: {}", e))?;

        Ok(())
    }

    /// Get candidates derived from a specific event
    pub async fn get_event_candidates(
        &self,
        event_id: &str,
    ) -> Result<Vec<ProvisionalSessionTruth>, String> {
        let rows = sqlx::query_as::<_, ProvisionalRow>(
            r#"
            SELECT provisional_id, session_id, source_session_turn_id, source_scene_turn_id,
                   story_time_anchor, derived_from_event_id, candidate_kind, candidate_payload,
                   promotion_status, promoted_knowledge_id, promoted_scene_turn_id,
                   created_at, updated_at
            FROM provisional_session_truth
            WHERE derived_from_event_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(event_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to query event candidates: {}", e))?;

        rows.into_iter().map(|r| self.row_to_candidate(r)).collect()
    }

    /// Check if an event has any promoted candidates
    pub async fn has_promoted_candidates(&self, event_id: &str) -> Result<bool, String> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM provisional_session_truth
            WHERE derived_from_event_id = ? AND promotion_status = 'promoted'
            "#,
        )
        .bind(event_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to count promoted candidates: {}", e))?;

        Ok(count.0 > 0)
    }

    /// Convert database row to candidate struct
    fn row_to_candidate(&self, row: ProvisionalRow) -> Result<ProvisionalSessionTruth, String> {
        let story_time_anchor: TimeAnchor = serde_json::from_str(&row.story_time_anchor)
            .map_err(|e| format!("Failed to parse story_time_anchor: {}", e))?;
        let candidate_payload: serde_json::Value = serde_json::from_str(&row.candidate_payload)
            .map_err(|e| format!("Failed to parse candidate_payload: {}", e))?;

        Ok(ProvisionalSessionTruth {
            provisional_id: row.provisional_id,
            session_id: row.session_id,
            source_session_turn_id: row.source_session_turn_id,
            source_scene_turn_id: row.source_scene_turn_id,
            story_time_anchor,
            derived_from_event_id: row.derived_from_event_id,
            candidate_kind: str_to_candidate_kind(&row.candidate_kind),
            candidate_payload,
            promotion_status: str_to_promotion_status(&row.promotion_status),
            promoted_knowledge_id: row.promoted_knowledge_id,
            promoted_scene_turn_id: row.promoted_scene_turn_id,
            created_at: parse_datetime(&row.created_at)?,
            updated_at: parse_datetime(&row.updated_at)?,
        })
    }
}

// =============================================================================
// Detail Slot Filling
// =============================================================================

/// Detail slot fill request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailSlotFillRequest {
    pub session_id: String,
    pub session_turn_id: String,
    pub scene_turn_id: Option<String>,
    pub event_id: String,
    pub slot_id: String,
    pub detail_kind: DetailKind,
    pub fill_content: serde_json::Value,
}

/// Detail slot fill result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailSlotFillResult {
    pub provisional_id: String,
    pub slot_id: String,
    pub event_id: String,
    pub validation_result: SlotValidationResult,
    pub can_promote: bool,
}

/// Validation result for slot fill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotValidationResult {
    pub is_valid: bool,
    pub conflicts: Vec<SlotConflict>,
    pub warnings: Vec<String>,
}

/// Conflict detected in slot fill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotConflict {
    pub conflict_kind: SlotConflictKind,
    pub constraint_id: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotConflictKind {
    HardConstraint,
    TimelineConflict,
    CharacterConflict,
    LocationConflict,
}

impl ProvisionalTruthManager {
    /// Fill a detail slot with content
    ///
    /// This is the main entry point for past timeline detail completion.
    /// It creates a provisional candidate and validates it against constraints.
    pub async fn fill_detail_slot(
        &self,
        request: DetailSlotFillRequest,
        story_time: &TimeAnchor,
    ) -> Result<DetailSlotFillResult, String> {
        // Create the provisional candidate
        let candidate = self
            .create_candidate(
                &request.session_id,
                &request.session_turn_id,
                request.scene_turn_id.as_deref(),
                story_time,
                Some(&request.event_id),
                Some(&request.slot_id),
                ProvisionalCandidateKind::EventDetail,
                request.fill_content.clone(),
            )
            .await?;

        // Validate against constraints (simplified - full implementation would
        // use TemporalConsistencyValidator)
        let validation_result = self.validate_slot_fill(&request).await?;

        // Determine if promotion is possible
        let can_promote = validation_result.is_valid
            && !validation_result.conflicts.iter().any(|c| {
                matches!(
                    c.conflict_kind,
                    SlotConflictKind::HardConstraint | SlotConflictKind::TimelineConflict
                )
            });

        Ok(DetailSlotFillResult {
            provisional_id: candidate.provisional_id,
            slot_id: request.slot_id,
            event_id: request.event_id,
            validation_result,
            can_promote,
        })
    }

    /// Validate a slot fill against constraints
    async fn validate_slot_fill(
        &self,
        request: &DetailSlotFillRequest,
    ) -> Result<SlotValidationResult, String> {
        // Get the historical event to check constraints
        let event_row: Option<EventConstraintRow> = sqlx::query_as(
            r#"
            SELECT knowledge_id, content
            FROM knowledge_entries
            WHERE knowledge_id = ?
            "#,
        )
        .bind(&request.event_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to query event: {}", e))?;

        let mut conflicts = Vec::new();
        let mut warnings = Vec::new();

        if let Some(_row) = event_row {
            // Parse the event content and check constraints
            // For now, simplified validation - full implementation would
            // check against required_outcomes, forbidden_outcomes, etc.
            warnings.push("候选细节待审核".to_string());
        } else {
            warnings.push(format!("事件 {} 不存在", request.event_id));
        }

        Ok(SlotValidationResult {
            is_valid: conflicts.is_empty(),
            conflicts,
            warnings,
        })
    }

    /// Batch promote multiple candidates
    pub async fn batch_promote(
        &self,
        provisional_ids: &[String],
        scene_turn_id: &str,
    ) -> Result<Vec<String>, String> {
        let mut promoted = Vec::new();

        for provisional_id in provisional_ids {
            // Get the candidate
            let row: Option<ProvisionalRow> = sqlx::query_as(
                r#"
                SELECT provisional_id, session_id, source_session_turn_id, source_scene_turn_id,
                       story_time_anchor, derived_from_event_id, candidate_kind, candidate_payload,
                       promotion_status, promoted_knowledge_id, promoted_scene_turn_id,
                       created_at, updated_at
                FROM provisional_session_truth
                WHERE provisional_id = ?
                "#,
            )
            .bind(provisional_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| format!("Failed to query candidate: {}", e))?;

            if let Some(row) = row {
                let candidate = self.row_to_candidate(row)?;

                // Only promote pending candidates
                if candidate.promotion_status != PromotionStatus::Pending {
                    continue;
                }

                // Create knowledge entry from candidate
                let knowledge_entry = self.candidate_to_knowledge(&candidate)?;

                // Promote
                self.promote_candidate(provisional_id, &knowledge_entry, scene_turn_id)
                    .await?;
                promoted.push(provisional_id.clone());
            }
        }

        Ok(promoted)
    }

    /// Convert a provisional candidate to a knowledge entry
    fn candidate_to_knowledge(
        &self,
        candidate: &ProvisionalSessionTruth,
    ) -> Result<KnowledgeEntry, String> {
        let now = Utc::now();
        let knowledge_id = generate_id("know");

        // Determine subject from the payload
        let subject = match &candidate.candidate_payload.get("subject") {
            Some(serde_json::Value::String(s)) if s == "world" => KnowledgeSubject::World,
            Some(serde_json::Value::Object(obj)) => {
                if let Some(serde_json::Value::String(kind)) = obj.get("kind") {
                    match kind.as_str() {
                        "region" => obj
                            .get("id")
                            .and_then(|v| v.as_str())
                            .map(|id| KnowledgeSubject::Region(id.to_string()))
                            .unwrap_or(KnowledgeSubject::World),
                        "faction" => obj
                            .get("id")
                            .and_then(|v| v.as_str())
                            .map(|id| KnowledgeSubject::Faction(id.to_string()))
                            .unwrap_or(KnowledgeSubject::World),
                        "character" => {
                            let id = obj.get("id").and_then(|v| v.as_str()).unwrap_or("");
                            let facet = obj
                                .get("facet")
                                .and_then(|v| v.as_str())
                                .map(|s| str_to_facet_type(s))
                                .unwrap_or(CharacterFacetType::Background);
                            KnowledgeSubject::Character {
                                id: id.to_string(),
                                facet,
                            }
                        }
                        _ => KnowledgeSubject::World,
                    }
                } else {
                    KnowledgeSubject::World
                }
            }
            _ => KnowledgeSubject::World,
        };

        Ok(KnowledgeEntry {
            knowledge_id,
            kind: KnowledgeKind::Memory,
            subject,
            content: candidate.candidate_payload.clone(),
            apparent_content: None,
            access_policy: AccessPolicy {
                known_by: Vec::new(),
                scope: vec![AccessScope::Public],
                conditions: Vec::new(),
            },
            subject_awareness: SubjectAwareness::Aware,
            metadata: KnowledgeMetadata {
                created_at: now,
                updated_at: now,
                valid_from: Some(candidate.story_time_anchor.clone()),
                valid_until: None,
                source_session_id: Some(candidate.session_id.clone()),
                source_scene_turn_id: candidate.source_scene_turn_id.clone(),
                derived_from_event_id: candidate.derived_from_event_id.clone(),
                emotional_weight: None,
                last_accessed_at: None,
                source: None,
            },
            valid_from: Some(candidate.story_time_anchor.clone()),
            valid_until: None,
            source_session_id: Some(candidate.session_id.clone()),
            source_scene_turn_id: candidate.source_scene_turn_id.clone(),
            derived_from_event_id: candidate.derived_from_event_id.clone(),
            schema_version: SCHEMA_VERSION.to_string(),
            created_at: now,
            updated_at: now,
        })
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

fn candidate_kind_to_str(kind: &ProvisionalCandidateKind) -> &'static str {
    match kind {
        ProvisionalCandidateKind::KnowledgeEntry => "knowledge_entry",
        ProvisionalCandidateKind::EventDetail => "event_detail",
        ProvisionalCandidateKind::RelationDetail => "relation_detail",
        ProvisionalCandidateKind::LocationDetail => "location_detail",
    }
}

fn str_to_candidate_kind(s: &str) -> ProvisionalCandidateKind {
    match s {
        "knowledge_entry" => ProvisionalCandidateKind::KnowledgeEntry,
        "event_detail" => ProvisionalCandidateKind::EventDetail,
        "relation_detail" => ProvisionalCandidateKind::RelationDetail,
        "location_detail" => ProvisionalCandidateKind::LocationDetail,
        _ => ProvisionalCandidateKind::EventDetail,
    }
}

fn promotion_status_to_str(status: &PromotionStatus) -> &'static str {
    match status {
        PromotionStatus::Pending => "pending",
        PromotionStatus::Promoted => "promoted",
        PromotionStatus::BlockedConflict => "blocked_conflict",
        PromotionStatus::NonCanon => "noncanon",
        PromotionStatus::TraceOnly => "trace_only",
    }
}

fn str_to_promotion_status(s: &str) -> PromotionStatus {
    match s {
        "pending" => PromotionStatus::Pending,
        "promoted" => PromotionStatus::Promoted,
        "blocked_conflict" => PromotionStatus::BlockedConflict,
        "noncanon" => PromotionStatus::NonCanon,
        "trace_only" => PromotionStatus::TraceOnly,
        _ => PromotionStatus::Pending,
    }
}

fn kind_to_str(kind: &KnowledgeKind) -> &'static str {
    match kind {
        KnowledgeKind::WorldFact => "world_fact",
        KnowledgeKind::RegionFact => "region_fact",
        KnowledgeKind::FactionFact => "faction_fact",
        KnowledgeKind::CharacterFacet => "character_facet",
        KnowledgeKind::HistoricalEvent => "historical_event",
        KnowledgeKind::Memory => "memory",
    }
}

fn facet_type_to_str(facet: &CharacterFacetType) -> &'static str {
    match facet {
        CharacterFacetType::Appearance => "appearance",
        CharacterFacetType::Identity => "identity",
        CharacterFacetType::TrueName => "true_name",
        CharacterFacetType::Species => "species",
        CharacterFacetType::Bloodline => "bloodline",
        CharacterFacetType::CultivationRealm => "cultivation_realm",
        CharacterFacetType::KnownAbility => "known_ability",
        CharacterFacetType::HiddenAbility => "hidden_ability",
        CharacterFacetType::Personality => "personality",
        CharacterFacetType::Background => "background",
        CharacterFacetType::Motivation => "motivation",
        CharacterFacetType::Trauma => "trauma",
        CharacterFacetType::MindModelCard => "mind_model_card",
    }
}

fn str_to_facet_type(s: &str) -> CharacterFacetType {
    match s {
        "appearance" => CharacterFacetType::Appearance,
        "identity" => CharacterFacetType::Identity,
        "true_name" => CharacterFacetType::TrueName,
        "species" => CharacterFacetType::Species,
        "bloodline" => CharacterFacetType::Bloodline,
        "cultivation_realm" => CharacterFacetType::CultivationRealm,
        "known_ability" => CharacterFacetType::KnownAbility,
        "hidden_ability" => CharacterFacetType::HiddenAbility,
        "personality" => CharacterFacetType::Personality,
        "background" => CharacterFacetType::Background,
        "motivation" => CharacterFacetType::Motivation,
        "trauma" => CharacterFacetType::Trauma,
        "mind_model_card" => CharacterFacetType::MindModelCard,
        _ => CharacterFacetType::Background,
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
struct ProvisionalRow {
    provisional_id: String,
    session_id: String,
    source_session_turn_id: String,
    source_scene_turn_id: Option<String>,
    story_time_anchor: String,
    derived_from_event_id: Option<String>,
    candidate_kind: String,
    candidate_payload: String,
    promotion_status: String,
    promoted_knowledge_id: Option<String>,
    promoted_scene_turn_id: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(sqlx::FromRow)]
struct EventConstraintRow {
    knowledge_id: String,
    content: String,
}
