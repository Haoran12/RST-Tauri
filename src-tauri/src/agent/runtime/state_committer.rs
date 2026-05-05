//! State committer
//!
//! Single write point for all state changes in Agent runtime.
//! Implements the StateCommitter pattern from docs/11_agent_runtime.md §6.

use chrono::Utc;
use serde_json;
use sqlx::SqlitePool;

use super::turn_state::TurnWorkingState;
use crate::agent::models::*;

/// State committer - single write point for state changes
///
/// This is the ONLY place where Layer 1, Layer 3, KnowledgeRevealEvents,
/// and Trace are written to the database. All writes happen in a single
/// SQLite transaction to ensure consistency.
pub struct StateCommitter {
    pool: SqlitePool,
}

/// Commit result
#[derive(Debug, Clone)]
pub struct CommitResult {
    pub commit_id: String,
    pub scene_turn_id: String,
    pub canon_status: RuntimeTurnCanonStatus,
}

/// Turn trace for logging
#[derive(Debug, Clone)]
pub struct TurnTrace {
    pub trace_id: String,
    pub scene_turn_id: String,
    pub session_id: Option<String>,
    pub story_time_anchor: Option<TimeAnchor>,
    pub runtime_turn_status: RuntimeTurnCanonStatus,
    pub summary: serde_json::Value,
}

impl StateCommitter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Commit state changes from an outcome plan
    ///
    /// This is the main entry point for state commitment. It performs
    /// all writes in a single transaction:
    /// 1. Update SceneModel (Layer 1)
    /// 2. Process KnowledgeRevealEvents
    /// 3. Add Memory knowledge entries
    /// 4. Apply StateUpdatePlan
    /// 5. Update WorldMainlineCursor (if mainline)
    /// 6. Write subjective snapshots (Layer 3)
    /// 7. Write turn traces
    pub async fn commit(
        &self,
        scene_turn_id: &str,
        session: &AgentSession,
        outcome: &OutcomePlannerOutput,
        narrative: &SurfaceRealizerOutput,
        working_state: &TurnWorkingState,
    ) -> Result<CommitResult, String> {
        // Determine canon status based on session and timeline
        let canon_status = self.determine_canon_status(session);

        // Generate commit ID
        let commit_id = format!("commit_{}", uuid::Uuid::new_v4());
        let now = Utc::now();

        // Begin transaction
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to begin transaction: {}", e))?;

        // Step 1: Create world_turn record
        self.create_world_turn(
            &mut tx,
            scene_turn_id,
            session,
            &working_state.raw_user_message,
            &narrative.narrative_text,
            canon_status,
            now,
        )
        .await?;

        // Step 2: Snapshot SceneModel (Layer 1) - only if canon
        if canon_status == RuntimeTurnCanonStatus::Canon {
            self.write_scene_snapshot(&mut tx, &working_state.scene, now)
                .await?;
        }

        // Step 3: Process KnowledgeRevealEvents - only if canon
        if canon_status == RuntimeTurnCanonStatus::Canon {
            for event in &outcome.knowledge_reveal_events {
                self.process_reveal_event(&mut tx, event, scene_turn_id, now)
                    .await?;
            }
        }

        // Step 4: Add Memory knowledge entries - only if canon
        if canon_status == RuntimeTurnCanonStatus::Canon {
            for entry in &outcome.state_update_plan.new_memory_entries {
                self.add_memory_entry(
                    &mut tx,
                    entry,
                    session.session_id.clone(),
                    scene_turn_id,
                    now,
                )
                .await?;
            }
        }

        // Step 5: Apply StateUpdatePlan - only if canon
        if canon_status == RuntimeTurnCanonStatus::Canon {
            self.apply_state_update(&mut tx, &outcome.state_update_plan, scene_turn_id, now)
                .await?;
        }

        // Step 6: Update WorldMainlineCursor (if mainline session)
        if session.session_kind == AgentSessionKind::Mainline
            && canon_status == RuntimeTurnCanonStatus::Canon
        {
            self.update_mainline_cursor(
                &mut tx,
                &session.world_id,
                scene_turn_id,
                &session.period_anchor,
                now,
            )
            .await?;
        }

        // Step 7: Write subjective snapshots (Layer 3)
        for character_id in &outcome.state_update_plan.subjective_update_refs {
            // Get prior subjective state from working state
            if let Some(subjective) = working_state.subjective_states.get(character_id) {
                self.write_subjective_snapshot(
                    &mut tx,
                    character_id,
                    scene_turn_id,
                    &session.session_id,
                    subjective,
                    canon_status,
                    now,
                )
                .await?;
            }
        }

        // Step 8: Write state_commit_record only for canonical commits.
        if matches!(
            canon_status,
            RuntimeTurnCanonStatus::Canon | RuntimeTurnCanonStatus::ProvisionalPromoted
        ) {
            self.write_commit_record(
                &mut tx,
                &commit_id,
                scene_turn_id,
                &outcome,
                canon_status,
                now,
            )
            .await?;
        }

        // Commit transaction
        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;

        Ok(CommitResult {
            commit_id,
            scene_turn_id: scene_turn_id.to_string(),
            canon_status,
        })
    }

    /// Determine canon status based on session kind and status
    fn determine_canon_status(&self, session: &AgentSession) -> RuntimeTurnCanonStatus {
        match session.session_kind {
            AgentSessionKind::Mainline => match session.canon_status {
                SessionCanonStatus::CanonCandidate => RuntimeTurnCanonStatus::Canon,
                SessionCanonStatus::PartiallyCanon => RuntimeTurnCanonStatus::Canon,
                SessionCanonStatus::NonCanon => RuntimeTurnCanonStatus::NonCanon,
            },
            AgentSessionKind::Retrospective => RuntimeTurnCanonStatus::ProvisionalOnly,
            AgentSessionKind::FuturePreview => RuntimeTurnCanonStatus::FuturePreview,
        }
    }

    /// Create world_turn record
    async fn create_world_turn(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        scene_turn_id: &str,
        session: &AgentSession,
        user_message: &serde_json::Value,
        narrative_text: &str,
        canon_status: RuntimeTurnCanonStatus,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), String> {
        let story_time_json = serde_json::to_string(&session.period_anchor)
            .map_err(|e| format!("Failed to serialize story_time_anchor: {}", e))?;
        let user_message_json = serde_json::to_string(user_message)
            .map_err(|e| format!("Failed to serialize user_message: {}", e))?;

        let parent_turn_id: Option<String> = if session.session_kind == AgentSessionKind::Mainline {
            sqlx::query_scalar(
                r#"
                SELECT mainline_head_turn_id
                FROM world_mainline_cursor
                WHERE world_id = ? AND timeline_id = 'main'
                "#,
            )
            .bind(&session.world_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|e| format!("Failed to load parent turn: {}", e))?
            .flatten()
        } else {
            None
        };

        sqlx::query(
            r#"
            INSERT INTO world_turns (
                scene_turn_id, parent_turn_id, session_id, timeline_id,
                story_time_anchor, user_message, rendered_output,
                runtime_turn_status, status, created_at
            ) VALUES (?, ?, ?, 'main', ?, ?, ?, ?, 'active', ?)
            "#,
        )
        .bind(scene_turn_id)
        .bind(&parent_turn_id)
        .bind(&session.session_id)
        .bind(&story_time_json)
        .bind(&user_message_json)
        .bind(narrative_text)
        .bind(runtime_turn_status_to_str(canon_status))
        .bind(now.to_rfc3339())
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("Failed to create world_turn: {}", e))?;

        Ok(())
    }

    /// Write canonical scene snapshot
    async fn write_scene_snapshot(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        scene: &SceneModel,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), String> {
        // Create scene snapshot for this turn
        let snapshot_id = format!("snap_{}", uuid::Uuid::new_v4());
        let scene_model_json = serde_json::to_string(scene)
            .map_err(|e| format!("Failed to serialize scene model: {}", e))?;

        sqlx::query(
            r#"
            INSERT INTO scene_snapshots (
                snapshot_id, scene_id, scene_turn_id, scene_model, created_at
            ) VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&snapshot_id)
        .bind(&scene.scene_id)
        .bind(&scene.scene_turn_id)
        .bind(&scene_model_json)
        .bind(now.to_rfc3339())
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("Failed to create scene snapshot: {}", e))?;

        Ok(())
    }

    /// Process knowledge reveal events
    async fn process_reveal_event(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        event: &KnowledgeRevealEvent,
        scene_turn_id: &str,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), String> {
        // Insert reveal event record
        let newly_known_by_json = serde_json::to_string(&event.newly_known_by)
            .map_err(|e| format!("Failed to serialize newly_known_by: {}", e))?;
        let trigger_json = serde_json::to_string(&event.trigger)
            .map_err(|e| format!("Failed to serialize trigger: {}", e))?;
        let scope_change_json = event
            .scope_change
            .as_ref()
            .map(|sc| serde_json::to_string(sc))
            .transpose()
            .map_err(|e| format!("Failed to serialize scope_change: {}", e))?;

        sqlx::query(
            r#"
            INSERT INTO knowledge_reveal_events (
                event_id, knowledge_id, newly_known_by, trigger,
                scope_change, scene_turn_id, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&event.event_id)
        .bind(&event.knowledge_id)
        .bind(&newly_known_by_json)
        .bind(&trigger_json)
        .bind(&scope_change_json)
        .bind(scene_turn_id)
        .bind(now.to_rfc3339())
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("Failed to insert knowledge_reveal_event: {}", e))?;

        let access_policy_json: String = sqlx::query_scalar(
            "SELECT access_policy FROM knowledge_entries WHERE knowledge_id = ?",
        )
        .bind(&event.knowledge_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| format!("Failed to load knowledge access_policy: {}", e))?;
        let access_policy: AccessPolicy = serde_json::from_str(&access_policy_json)
            .map_err(|e| format!("Failed to parse knowledge access_policy: {}", e))?;
        let updated_policy = apply_reveal_to_access_policy(access_policy, event)?;
        let updated_policy_json = serde_json::to_string(&updated_policy)
            .map_err(|e| format!("Failed to serialize updated access_policy: {}", e))?;

        sqlx::query(
            "UPDATE knowledge_entries SET access_policy = ?, updated_at = ? WHERE knowledge_id = ?",
        )
        .bind(&updated_policy_json)
        .bind(now.to_rfc3339())
        .bind(&event.knowledge_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("Failed to update knowledge access_policy: {}", e))?;

        sqlx::query("DELETE FROM knowledge_access_known_by WHERE knowledge_id = ?")
            .bind(&event.knowledge_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to reset knowledge_access_known_by: {}", e))?;
        for character_id in &updated_policy.known_by {
            sqlx::query(
                r#"
                INSERT INTO knowledge_access_known_by (knowledge_id, character_id)
                VALUES (?, ?)
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(&event.knowledge_id)
            .bind(character_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to update knowledge_access_known_by: {}", e))?;
        }

        sqlx::query("DELETE FROM knowledge_access_scopes WHERE knowledge_id = ?")
            .bind(&event.knowledge_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to reset knowledge_access_scopes: {}", e))?;
        for scope in &updated_policy.scope {
            let (scope_type, scope_value) = access_scope_index_pair(scope);
            sqlx::query(
                r#"
                INSERT INTO knowledge_access_scopes (knowledge_id, scope_type, scope_value)
                VALUES (?, ?, ?)
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(&event.knowledge_id)
            .bind(scope_type)
            .bind(scope_value)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to update knowledge_access_scopes: {}", e))?;
        }

        Ok(())
    }

    /// Add memory entries
    async fn add_memory_entry(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entry: &KnowledgeEntry,
        session_id: String,
        scene_turn_id: &str,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), String> {
        let content_json = serde_json::to_string(&entry.content)
            .map_err(|e| format!("Failed to serialize content: {}", e))?;
        let apparent_content_json = entry
            .apparent_content
            .as_ref()
            .map(|ac| serde_json::to_string(ac))
            .transpose()
            .map_err(|e| format!("Failed to serialize apparent_content: {}", e))?;
        let access_policy_json = serde_json::to_string(&entry.access_policy)
            .map_err(|e| format!("Failed to serialize access_policy: {}", e))?;
        let subject_awareness_json = serde_json::to_string(&entry.subject_awareness)
            .map_err(|e| format!("Failed to serialize subject_awareness: {}", e))?;
        let metadata_json = serde_json::to_string(&entry.metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
        let valid_from_json = entry
            .valid_from
            .as_ref()
            .map(|vf| serde_json::to_string(vf))
            .transpose()
            .map_err(|e| format!("Failed to serialize valid_from: {}", e))?;
        let valid_until_json = entry
            .valid_until
            .as_ref()
            .map(|vu| serde_json::to_string(vu))
            .transpose()
            .map_err(|e| format!("Failed to serialize valid_until: {}", e))?;

        sqlx::query(
            r#"
            INSERT INTO knowledge_entries (
                knowledge_id, kind, subject_type, subject_id, facet_type,
                content, apparent_content, access_policy, subject_awareness,
                metadata, valid_from, valid_until, source_session_id,
                source_scene_turn_id, derived_from_event_id, schema_version,
                created_at, updated_at
            ) VALUES (?, 'memory', ?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, '0.1', ?, ?)
            "#,
        )
        .bind(&entry.knowledge_id)
        .bind(subject_type_to_str(&entry.subject))
        .bind(subject_id(&entry.subject))
        .bind(&content_json)
        .bind(&apparent_content_json)
        .bind(&access_policy_json)
        .bind(&subject_awareness_json)
        .bind(&metadata_json)
        .bind(&valid_from_json)
        .bind(&valid_until_json)
        .bind(&session_id)
        .bind(scene_turn_id)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("Failed to insert memory entry: {}", e))?;

        Ok(())
    }

    /// Apply state update plan
    async fn apply_state_update(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        plan: &StateUpdatePlan,
        scene_turn_id: &str,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), String> {
        // Apply character state deltas
        for delta in &plan.character_state_deltas {
            let temp_state_json = serde_json::to_string(&delta.temporary_state_delta)
                .map_err(|e| format!("Failed to serialize temporary_state_delta: {}", e))?;

            // Update character_records.temporary_state
            sqlx::query(
                r#"
                UPDATE character_records
                SET temporary_state = ?, updated_at = ?
                WHERE character_id = ?
                "#,
            )
            .bind(&temp_state_json)
            .bind(now.to_rfc3339())
            .bind(&delta.character_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to update character temporary_state: {}", e))?;

            // Create temporal_state_record for history
            let state_record_id = format!("tsr_{}", uuid::Uuid::new_v4());
            sqlx::query(
                r#"
                INSERT INTO temporal_state_records (
                    state_record_id, subject_type, subject_id, state_kind,
                    valid_from, valid_until, payload, source_scene_turn_id,
                    source_session_id, canon_status, schema_version,
                    created_at, updated_at
                ) VALUES (?, 'character', ?, 'temporary_state', ?, NULL, ?, ?, NULL, 'canon', '0.1', ?, ?)
                "#,
            )
            .bind(&state_record_id)
            .bind(&delta.character_id)
            .bind(now.to_rfc3339()) // valid_from = now
            .bind(&temp_state_json)
            .bind(scene_turn_id)
            .bind(now.to_rfc3339())
            .bind(now.to_rfc3339())
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to create temporal_state_record: {}", e))?;
        }

        Ok(())
    }

    /// Update world mainline cursor
    async fn update_mainline_cursor(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        world_id: &str,
        new_turn_id: &str,
        new_time_anchor: &TimeAnchor,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), String> {
        let time_anchor_json = serde_json::to_string(new_time_anchor)
            .map_err(|e| format!("Failed to serialize time_anchor: {}", e))?;

        sqlx::query(
            r#"
            UPDATE world_mainline_cursor
            SET mainline_head_turn_id = ?,
                mainline_time_anchor = ?,
                updated_at = ?
            WHERE world_id = ? AND timeline_id = 'main'
            "#,
        )
        .bind(new_turn_id)
        .bind(&time_anchor_json)
        .bind(now.to_rfc3339())
        .bind(world_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("Failed to update mainline cursor: {}", e))?;

        Ok(())
    }

    /// Write subjective snapshot
    async fn write_subjective_snapshot(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        character_id: &str,
        scene_turn_id: &str,
        session_id: &str,
        subjective: &CharacterSubjectiveState,
        canon_status: RuntimeTurnCanonStatus,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), String> {
        let snapshot_id = format!("subj_{}", uuid::Uuid::new_v4());
        let story_time_json = subjective
            .story_time_anchor
            .as_ref()
            .map(|ta| serde_json::to_string(ta))
            .transpose()
            .map_err(|e| format!("Failed to serialize story_time_anchor: {}", e))?;
        let belief_state_json = serde_json::to_string(&subjective.belief_state)
            .map_err(|e| format!("Failed to serialize belief_state: {}", e))?;
        let emotion_state_json = serde_json::to_string(&subjective.emotion_state)
            .map_err(|e| format!("Failed to serialize emotion_state: {}", e))?;
        let relation_models_json = serde_json::to_string(&subjective.relation_models)
            .map_err(|e| format!("Failed to serialize relation_models: {}", e))?;
        let current_goals_json = serde_json::to_string(&subjective.current_goals)
            .map_err(|e| format!("Failed to serialize current_goals: {}", e))?;

        sqlx::query(
            r#"
            INSERT INTO character_subjective_snapshots (
                snapshot_id, character_id, scene_turn_id, session_id,
                story_time_anchor, canon_status, belief_state, emotion_state,
                relation_models, current_goals, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&snapshot_id)
        .bind(character_id)
        .bind(scene_turn_id)
        .bind(session_id)
        .bind(&story_time_json)
        .bind(subjective_canon_status_to_str(canon_status))
        .bind(&belief_state_json)
        .bind(&emotion_state_json)
        .bind(&relation_models_json)
        .bind(&current_goals_json)
        .bind(now.to_rfc3339())
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("Failed to write subjective snapshot: {}", e))?;

        Ok(())
    }

    /// Write state commit record
    async fn write_commit_record(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        commit_id: &str,
        scene_turn_id: &str,
        outcome: &OutcomePlannerOutput,
        canon_status: RuntimeTurnCanonStatus,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), String> {
        // Collect changed IDs from outcome plan
        let changed_scene_ids: Vec<String> = outcome
            .state_update_plan
            .scene_delta
            .as_ref()
            .map(|sd| vec![sd.scene_id.clone()])
            .unwrap_or_default();
        let changed_knowledge_ids: Vec<String> = outcome
            .state_update_plan
            .new_memory_entries
            .iter()
            .map(|e| e.knowledge_id.clone())
            .collect();
        let changed_character_ids: Vec<String> = outcome
            .state_update_plan
            .character_state_deltas
            .iter()
            .map(|d| d.character_id.clone())
            .collect();
        let changed_subjective_ids: Vec<String> =
            outcome.state_update_plan.subjective_update_refs.clone();

        // Create rollback patch (simplified - just store the commit info)
        let rollback_patch = serde_json::json!({
            "commit_id": commit_id,
            "canon_status": runtime_turn_status_to_str(canon_status),
            "timestamp": now.to_rfc3339(),
        });

        sqlx::query(
            r#"
            INSERT INTO state_commit_records (
                commit_id, scene_turn_id, changed_scene_snapshot_ids,
                changed_location_ids, changed_knowledge_ids, changed_character_ids,
                changed_subjective_snapshot_ids, trace_ids, rollback_patch,
                created_at
            ) VALUES (?, ?, ?, '[]', ?, ?, ?, '[]', ?, ?)
            "#,
        )
        .bind(commit_id)
        .bind(scene_turn_id)
        .bind(&serde_json::to_string(&changed_scene_ids).unwrap())
        .bind(&serde_json::to_string(&changed_knowledge_ids).unwrap())
        .bind(&serde_json::to_string(&changed_character_ids).unwrap())
        .bind(&serde_json::to_string(&changed_subjective_ids).unwrap())
        .bind(&serde_json::to_string(&rollback_patch).unwrap())
        .bind(now.to_rfc3339())
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("Failed to write commit record: {}", e))?;

        Ok(())
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

fn runtime_turn_status_to_str(status: RuntimeTurnCanonStatus) -> &'static str {
    match status {
        RuntimeTurnCanonStatus::Canon => "canon",
        RuntimeTurnCanonStatus::ProvisionalPromoted => "provisional_promoted",
        RuntimeTurnCanonStatus::ProvisionalOnly => "provisional_only",
        RuntimeTurnCanonStatus::NonCanon => "noncanon",
        RuntimeTurnCanonStatus::FuturePreview => "future_preview",
    }
}

fn subjective_canon_status_to_str(status: RuntimeTurnCanonStatus) -> &'static str {
    match status {
        RuntimeTurnCanonStatus::Canon => "canon",
        _ => "non_canon",
    }
}

fn subject_type_to_str(subject: &KnowledgeSubject) -> &'static str {
    match subject {
        KnowledgeSubject::World => "world",
        KnowledgeSubject::Region(_) => "region",
        KnowledgeSubject::Faction(_) => "faction",
        KnowledgeSubject::Character { .. } => "character",
        KnowledgeSubject::Event { .. } => "event",
    }
}

fn subject_id(subject: &KnowledgeSubject) -> Option<&str> {
    match subject {
        KnowledgeSubject::World => None,
        KnowledgeSubject::Region(id) => Some(id.as_str()),
        KnowledgeSubject::Faction(id) => Some(id.as_str()),
        KnowledgeSubject::Character { id, .. } => Some(id.as_str()),
        KnowledgeSubject::Event { event_id } => Some(event_id.as_str()),
    }
}

fn apply_reveal_to_access_policy(
    mut policy: AccessPolicy,
    event: &KnowledgeRevealEvent,
) -> Result<AccessPolicy, String> {
    let had_god_only = policy
        .scope
        .iter()
        .any(|scope| matches!(scope, AccessScope::GodOnly));

    if had_god_only && !matches!(event.scope_change, Some(AccessScopeChange::RemoveGodOnly)) {
        return Err(
            "GodOnly knowledge must be revealed with scope_change=RemoveGodOnly before adding known_by"
                .to_string(),
        );
    }

    if let Some(scope_change) = &event.scope_change {
        match scope_change {
            AccessScopeChange::RemoveGodOnly => {
                policy
                    .scope
                    .retain(|scope| !matches!(scope, AccessScope::GodOnly));
            }
            AccessScopeChange::ReplaceScopes(scopes) => {
                policy.scope = scopes.clone();
            }
        }
    }

    for character_id in &event.newly_known_by {
        if !policy.known_by.iter().any(|known| known == character_id) {
            policy.known_by.push(character_id.clone());
        }
    }

    Ok(policy)
}

fn access_scope_index_pair(scope: &AccessScope) -> (&'static str, String) {
    match scope {
        AccessScope::Public => ("public", String::new()),
        AccessScope::GodOnly => ("god_only", String::new()),
        AccessScope::Region(value) => ("region", value.clone()),
        AccessScope::Faction(value) => ("faction", value.clone()),
        AccessScope::Realm(value) => ("realm", value.clone()),
        AccessScope::Role(value) => ("role", value.clone()),
        AccessScope::Bloodline(value) => ("bloodline", value.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reveal_event(scope_change: Option<AccessScopeChange>) -> KnowledgeRevealEvent {
        KnowledgeRevealEvent {
            event_id: "reveal-1".to_string(),
            knowledge_id: "k-god".to_string(),
            newly_known_by: vec!["char-a".to_string()],
            trigger: RevealTrigger::Witnessed,
            scope_change,
            scene_turn_id: "turn-1".to_string(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn god_only_reveal_requires_remove_god_only_scope_change() {
        let policy = AccessPolicy {
            known_by: Vec::new(),
            scope: vec![AccessScope::GodOnly],
            conditions: Vec::new(),
        };

        let result = apply_reveal_to_access_policy(policy, &reveal_event(None));

        assert!(result.is_err());
    }

    #[test]
    fn god_only_reveal_removes_scope_before_adding_known_by() {
        let policy = AccessPolicy {
            known_by: Vec::new(),
            scope: vec![AccessScope::GodOnly],
            conditions: Vec::new(),
        };

        let updated = apply_reveal_to_access_policy(
            policy,
            &reveal_event(Some(AccessScopeChange::RemoveGodOnly)),
        )
        .expect("updated policy");

        assert!(updated.scope.is_empty());
        assert_eq!(updated.known_by, vec!["char-a".to_string()]);
    }

    #[tokio::test]
    async fn non_canon_turns_do_not_need_state_commit_records() {
        let session = AgentSession {
            session_id: "s1".to_string(),
            world_id: "w1".to_string(),
            title: "side".to_string(),
            session_kind: AgentSessionKind::FuturePreview,
            period_anchor: TimeAnchor {
                calendar_id: "default".to_string(),
                ordinal: 10,
                precision: TimePrecision::Day,
                display_text: "future".to_string(),
            },
            player_mode: PlayerMode::Director,
            player_character_id: None,
            canon_status: SessionCanonStatus::CanonCandidate,
            conflict_policy: None,
            status: SessionStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let committer = StateCommitter::new(SqlitePool::connect_lazy("sqlite::memory:").unwrap());

        assert_eq!(
            committer.determine_canon_status(&session),
            RuntimeTurnCanonStatus::FuturePreview
        );
    }
}
