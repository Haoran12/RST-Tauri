//! World editor committer
//!
//! Paused-only single transaction commit for world editor.
//!
//! Constraints:
//! - Only allowed when world is paused (no active turns/LLM calls)
//! - All changes validated before commit
//! - Changes recorded in world_editor_commits journal
//! - Rollback patch generated for each commit

use chrono::{DateTime, Utc};
use serde_json;
use sqlx::{Row, Sqlite, SqlitePool, Transaction};

use crate::agent::models::*;

use super::validator::{ValidationSeverity, WorldEditorValidator};

/// World editor committer - paused-only commit
pub struct WorldEditorCommitter {
    pool: SqlitePool,
}

impl WorldEditorCommitter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Commit world editor changes
    /// Only allowed when world is paused (no active turns)
    pub async fn commit(
        &self,
        world_id: &str,
        base_revision: u64,
        changes: WorldEditorChanges,
    ) -> Result<EditorCommitResult, String> {
        if !self.is_world_paused(world_id).await? {
            return Err("World is not paused - cannot commit editor changes".to_string());
        }

        let current_revision = self.get_current_revision(world_id).await?;
        if current_revision != base_revision {
            return Err(format!(
                "Revision mismatch: expected {}, got {}",
                base_revision, current_revision
            ));
        }

        let validation_summary = self.validate_changes(&changes)?;
        let rollback_patch = self.generate_rollback_patch(world_id, &changes).await?;
        let resulting_revision = base_revision + 1;
        let commit_id = generate_id("editor_commit");

        let mut changed_location_ids = Vec::new();
        let mut changed_knowledge_ids = Vec::new();
        let mut changed_character_ids = Vec::new();
        let mut changed_relationship_ids = Vec::new();

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to begin transaction: {}", e))?;

        for location in &changes.location_creates {
            Self::insert_location(&mut tx, location).await?;
            changed_location_ids.push(location.location_id.clone());
        }

        for location in &changes.location_updates {
            Self::update_location(&mut tx, location).await?;
            changed_location_ids.push(location.location_id.clone());
        }

        for location_id in &changes.location_deletes {
            Self::delete_location(&mut tx, location_id).await?;
            changed_location_ids.push(location_id.clone());
        }

        for edge in &changes.edge_creates {
            Self::insert_edge(&mut tx, edge).await?;
            changed_relationship_ids.push(edge.edge_id.clone());
        }

        for edge_id in &changes.edge_deletes {
            Self::delete_edge(&mut tx, edge_id).await?;
            changed_relationship_ids.push(edge_id.clone());
        }

        for entry in &changes.knowledge_creates {
            Self::insert_knowledge(&mut tx, entry).await?;
            changed_knowledge_ids.push(entry.knowledge_id.clone());
        }

        for entry in &changes.knowledge_updates {
            Self::update_knowledge(&mut tx, entry).await?;
            changed_knowledge_ids.push(entry.knowledge_id.clone());
        }

        for knowledge_id in &changes.knowledge_deletes {
            Self::delete_knowledge(&mut tx, knowledge_id).await?;
            changed_knowledge_ids.push(knowledge_id.clone());
        }

        for character in &changes.character_creates {
            Self::insert_character(&mut tx, character).await?;
            changed_character_ids.push(character.character_id.clone());
        }

        for character in &changes.character_updates {
            Self::update_character(&mut tx, character).await?;
            changed_character_ids.push(character.character_id.clone());
        }

        for character_id in &changes.character_deletes {
            Self::delete_character(&mut tx, character_id).await?;
            changed_character_ids.push(character_id.clone());
        }

        Self::write_commit_journal(
            &mut tx,
            &commit_id,
            world_id,
            base_revision,
            resulting_revision,
            &changed_location_ids,
            &changed_knowledge_ids,
            &changed_character_ids,
            &changed_relationship_ids,
            &rollback_patch,
            &validation_summary,
        )
        .await?;

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;

        Ok(EditorCommitResult {
            commit_id,
            resulting_revision,
            changed_location_ids,
            changed_knowledge_ids,
            changed_character_ids,
        })
    }

    fn validate_changes(&self, changes: &WorldEditorChanges) -> Result<serde_json::Value, String> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        for location in changes
            .location_creates
            .iter()
            .chain(changes.location_updates.iter())
        {
            Self::collect_validation(
                &mut errors,
                &mut warnings,
                WorldEditorValidator::validate_location(location)?,
            );
        }

        for edge in &changes.edge_creates {
            Self::collect_validation(
                &mut errors,
                &mut warnings,
                WorldEditorValidator::validate_edge(edge)?,
            );
        }

        for entry in changes
            .knowledge_creates
            .iter()
            .chain(changes.knowledge_updates.iter())
        {
            Self::collect_validation(
                &mut errors,
                &mut warnings,
                WorldEditorValidator::validate_knowledge(entry)?,
            );
        }

        for character in changes
            .character_creates
            .iter()
            .chain(changes.character_updates.iter())
        {
            Self::collect_validation(
                &mut errors,
                &mut warnings,
                WorldEditorValidator::validate_character(character)?,
            );
        }

        if !errors.is_empty() {
            return Err(format!(
                "World editor validation failed: {}",
                errors.join("; ")
            ));
        }

        Ok(serde_json::json!({
            "error_count": errors.len(),
            "warning_count": warnings.len(),
            "warnings": warnings,
        }))
    }

    fn collect_validation(
        errors: &mut Vec<String>,
        warnings: &mut Vec<String>,
        issues: Vec<super::validator::ValidationIssue>,
    ) {
        for issue in issues {
            let message = format!("{}: {}", issue.field_path, issue.message);
            match issue.severity {
                ValidationSeverity::Error => errors.push(message),
                ValidationSeverity::Warning => warnings.push(message),
            }
        }
    }

    /// Check if world is paused (no active turns or LLM calls)
    async fn is_world_paused(&self, world_id: &str) -> Result<bool, String> {
        let active_turns: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM world_turns wt
            JOIN agent_sessions s ON wt.session_id = s.session_id
            WHERE s.world_id = ? AND wt.status = 'active'
            "#,
        )
        .bind(world_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to check active turns: {}", e))?;

        if active_turns.0 > 0 {
            return Ok(false);
        }

        let pending_calls: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM llm_call_logs
            WHERE world_id = ? AND status = 'started'
            "#,
        )
        .bind(world_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to check pending LLM calls: {}", e))?;

        Ok(pending_calls.0 == 0)
    }

    /// Get current editor revision for a world
    async fn get_current_revision(&self, world_id: &str) -> Result<u64, String> {
        let row: Option<(Option<i64>,)> = sqlx::query_as(
            "SELECT MAX(resulting_editor_revision) FROM world_editor_commits WHERE world_id = ?",
        )
        .bind(world_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to get current revision: {}", e))?;

        Ok(row.and_then(|r| r.0).unwrap_or(0) as u64)
    }

    /// Generate rollback patch for changes.
    ///
    /// Created records are stored as JSON null, so rollback deletes them.
    /// Updated/deleted records store their previous full image.
    async fn generate_rollback_patch(
        &self,
        _world_id: &str,
        changes: &WorldEditorChanges,
    ) -> Result<serde_json::Value, String> {
        let mut locations = serde_json::Map::new();
        let mut edges = serde_json::Map::new();
        let mut knowledge = serde_json::Map::new();
        let mut characters = serde_json::Map::new();

        for location in &changes.location_creates {
            locations.insert(location.location_id.clone(), serde_json::Value::Null);
        }
        for location in &changes.location_updates {
            let current = self
                .get_location(&location.location_id)
                .await?
                .ok_or_else(|| {
                    format!("Location not found for update: {}", location.location_id)
                })?;
            locations.insert(
                location.location_id.clone(),
                serde_json::to_value(current).map_err(|e| e.to_string())?,
            );
        }
        for location_id in &changes.location_deletes {
            let current = self
                .get_location(location_id)
                .await?
                .ok_or_else(|| format!("Location not found for delete: {}", location_id))?;
            locations.insert(
                location_id.clone(),
                serde_json::to_value(current).map_err(|e| e.to_string())?,
            );
        }

        for edge in &changes.edge_creates {
            edges.insert(edge.edge_id.clone(), serde_json::Value::Null);
        }
        for edge_id in &changes.edge_deletes {
            let current = self
                .get_edge(edge_id)
                .await?
                .ok_or_else(|| format!("Location edge not found for delete: {}", edge_id))?;
            edges.insert(
                edge_id.clone(),
                serde_json::to_value(current).map_err(|e| e.to_string())?,
            );
        }

        for entry in &changes.knowledge_creates {
            knowledge.insert(entry.knowledge_id.clone(), serde_json::Value::Null);
        }
        for entry in &changes.knowledge_updates {
            let current = self
                .get_knowledge(&entry.knowledge_id)
                .await?
                .ok_or_else(|| format!("Knowledge not found for update: {}", entry.knowledge_id))?;
            knowledge.insert(
                entry.knowledge_id.clone(),
                serde_json::to_value(current).map_err(|e| e.to_string())?,
            );
        }
        for knowledge_id in &changes.knowledge_deletes {
            let current = self
                .get_knowledge(knowledge_id)
                .await?
                .ok_or_else(|| format!("Knowledge not found for delete: {}", knowledge_id))?;
            knowledge.insert(
                knowledge_id.clone(),
                serde_json::to_value(current).map_err(|e| e.to_string())?,
            );
        }

        for character in &changes.character_creates {
            characters.insert(character.character_id.clone(), serde_json::Value::Null);
        }
        for character in &changes.character_updates {
            let current = self
                .get_character(&character.character_id)
                .await?
                .ok_or_else(|| {
                    format!("Character not found for update: {}", character.character_id)
                })?;
            characters.insert(
                character.character_id.clone(),
                serde_json::to_value(current).map_err(|e| e.to_string())?,
            );
        }
        for character_id in &changes.character_deletes {
            let current = self
                .get_character(character_id)
                .await?
                .ok_or_else(|| format!("Character not found for delete: {}", character_id))?;
            characters.insert(
                character_id.clone(),
                serde_json::to_value(current).map_err(|e| e.to_string())?,
            );
        }

        Ok(serde_json::json!({
            "locations": locations,
            "edges": edges,
            "knowledge": knowledge,
            "characters": characters,
        }))
    }

    async fn get_location(&self, location_id: &str) -> Result<Option<LocationNode>, String> {
        let row = sqlx::query(
            r#"
            SELECT location_id, name, polity_id, parent_id, canonical_level,
                   type_label, tags, status, metadata, schema_version, created_at, updated_at
            FROM location_nodes
            WHERE location_id = ?
            "#,
        )
        .bind(location_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to get location: {}", e))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let alias_rows = sqlx::query(
            "SELECT alias, locale, normalized_alias FROM location_aliases WHERE location_id = ?",
        )
        .bind(location_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to get location aliases: {}", e))?;

        Ok(Some(LocationNode {
            location_id: row.try_get("location_id").map_err(row_err)?,
            name: row.try_get("name").map_err(row_err)?,
            aliases: alias_rows
                .into_iter()
                .map(|alias_row| {
                    Ok(LocationAlias {
                        alias: alias_row.try_get("alias").map_err(row_err)?,
                        locale: alias_row.try_get("locale").map_err(row_err)?,
                        normalized_alias: alias_row.try_get("normalized_alias").map_err(row_err)?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
            polity_id: row.try_get("polity_id").map_err(row_err)?,
            parent_id: row.try_get("parent_id").map_err(row_err)?,
            canonical_level: str_to_location_level(
                row.try_get::<String, _>("canonical_level")
                    .map_err(row_err)?
                    .as_str(),
            )?,
            type_label: row.try_get("type_label").map_err(row_err)?,
            tags: parse_json_field(
                row.try_get::<String, _>("tags").map_err(row_err)?.as_str(),
                "location.tags",
            )?,
            status: str_to_location_status(
                row.try_get::<String, _>("status")
                    .map_err(row_err)?
                    .as_str(),
            )?,
            metadata: parse_json_field(
                row.try_get::<String, _>("metadata")
                    .map_err(row_err)?
                    .as_str(),
                "location.metadata",
            )?,
            schema_version: row.try_get("schema_version").map_err(row_err)?,
            created_at: parse_rfc3339(
                row.try_get::<String, _>("created_at")
                    .map_err(row_err)?
                    .as_str(),
            )?,
            updated_at: parse_rfc3339(
                row.try_get::<String, _>("updated_at")
                    .map_err(row_err)?
                    .as_str(),
            )?,
        }))
    }

    async fn get_edge(&self, edge_id: &str) -> Result<Option<LocationEdge>, String> {
        let row = sqlx::query(
            r#"
            SELECT edge_id, from_location_id, to_location_id, relation, bidirectional,
                   distance_km, travel_time, terrain_cost, safety_cost, seasonal_modifiers,
                   allowed_modes, confidence, source, schema_version, created_at, updated_at
            FROM location_edges
            WHERE edge_id = ?
            "#,
        )
        .bind(edge_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to get location edge: {}", e))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let distance_json: Option<String> = row.try_get("distance_km").map_err(row_err)?;
        let travel_json: Option<String> = row.try_get("travel_time").map_err(row_err)?;

        Ok(Some(LocationEdge {
            edge_id: row.try_get("edge_id").map_err(row_err)?,
            from_location_id: row.try_get("from_location_id").map_err(row_err)?,
            to_location_id: row.try_get("to_location_id").map_err(row_err)?,
            relation: str_to_edge_relation(
                row.try_get::<String, _>("relation")
                    .map_err(row_err)?
                    .as_str(),
            )?,
            bidirectional: row.try_get::<i64, _>("bidirectional").map_err(row_err)? != 0,
            distance_km: parse_optional_json_field(distance_json, "edge.distance_km")?,
            travel_time: parse_optional_json_field(travel_json, "edge.travel_time")?,
            terrain_cost: row.try_get("terrain_cost").map_err(row_err)?,
            safety_cost: row.try_get("safety_cost").map_err(row_err)?,
            seasonal_modifiers: parse_json_field(
                row.try_get::<String, _>("seasonal_modifiers")
                    .map_err(row_err)?
                    .as_str(),
                "edge.seasonal_modifiers",
            )?,
            allowed_modes: parse_json_field(
                row.try_get::<String, _>("allowed_modes")
                    .map_err(row_err)?
                    .as_str(),
                "edge.allowed_modes",
            )?,
            confidence: str_to_fact_confidence(
                row.try_get::<String, _>("confidence")
                    .map_err(row_err)?
                    .as_str(),
            )?,
            source: parse_json_field(
                row.try_get::<String, _>("source")
                    .map_err(row_err)?
                    .as_str(),
                "edge.source",
            )?,
            schema_version: row.try_get("schema_version").map_err(row_err)?,
            created_at: parse_rfc3339(
                row.try_get::<String, _>("created_at")
                    .map_err(row_err)?
                    .as_str(),
            )?,
            updated_at: parse_rfc3339(
                row.try_get::<String, _>("updated_at")
                    .map_err(row_err)?
                    .as_str(),
            )?,
        }))
    }

    async fn get_knowledge(&self, knowledge_id: &str) -> Result<Option<KnowledgeEntry>, String> {
        let row = sqlx::query(
            r#"
            SELECT knowledge_id, kind, subject_type, subject_id, facet_type, content,
                   apparent_content, access_policy, subject_awareness, metadata,
                   valid_from, valid_until, source_session_id, source_scene_turn_id,
                   derived_from_event_id, schema_version, created_at, updated_at
            FROM knowledge_entries
            WHERE knowledge_id = ?
            "#,
        )
        .bind(knowledge_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to get knowledge: {}", e))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let apparent_content: Option<String> = row.try_get("apparent_content").map_err(row_err)?;
        let valid_from: Option<String> = row.try_get("valid_from").map_err(row_err)?;
        let valid_until: Option<String> = row.try_get("valid_until").map_err(row_err)?;

        Ok(Some(KnowledgeEntry {
            knowledge_id: row.try_get("knowledge_id").map_err(row_err)?,
            kind: str_to_knowledge_kind(
                row.try_get::<String, _>("kind").map_err(row_err)?.as_str(),
            )?,
            subject: columns_to_subject(
                row.try_get::<String, _>("subject_type")
                    .map_err(row_err)?
                    .as_str(),
                row.try_get("subject_id").map_err(row_err)?,
                row.try_get("facet_type").map_err(row_err)?,
            )?,
            content: parse_json_field(
                row.try_get::<String, _>("content")
                    .map_err(row_err)?
                    .as_str(),
                "knowledge.content",
            )?,
            apparent_content: parse_optional_json_field(
                apparent_content,
                "knowledge.apparent_content",
            )?,
            access_policy: parse_json_field(
                row.try_get::<String, _>("access_policy")
                    .map_err(row_err)?
                    .as_str(),
                "knowledge.access_policy",
            )?,
            subject_awareness: parse_json_field(
                row.try_get::<String, _>("subject_awareness")
                    .map_err(row_err)?
                    .as_str(),
                "knowledge.subject_awareness",
            )?,
            metadata: parse_json_field(
                row.try_get::<String, _>("metadata")
                    .map_err(row_err)?
                    .as_str(),
                "knowledge.metadata",
            )?,
            valid_from: parse_optional_json_field(valid_from, "knowledge.valid_from")?,
            valid_until: parse_optional_json_field(valid_until, "knowledge.valid_until")?,
            source_session_id: row.try_get("source_session_id").map_err(row_err)?,
            source_scene_turn_id: row.try_get("source_scene_turn_id").map_err(row_err)?,
            derived_from_event_id: row.try_get("derived_from_event_id").map_err(row_err)?,
            schema_version: row.try_get("schema_version").map_err(row_err)?,
            created_at: parse_rfc3339(
                row.try_get::<String, _>("created_at")
                    .map_err(row_err)?
                    .as_str(),
            )?,
            updated_at: parse_rfc3339(
                row.try_get::<String, _>("updated_at")
                    .map_err(row_err)?
                    .as_str(),
            )?,
        }))
    }

    async fn get_character(&self, character_id: &str) -> Result<Option<CharacterRecord>, String> {
        let row = sqlx::query(
            r#"
            SELECT character_id, base_attributes, baseline_body_profile,
                   mana_expression_tendency, mana_expression_tendency_factor_override,
                   mind_model_card_knowledge_id, temporary_state, schema_version,
                   created_at, updated_at
            FROM character_records
            WHERE character_id = ?
            "#,
        )
        .bind(character_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to get character: {}", e))?;

        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(CharacterRecord {
            character_id: row.try_get("character_id").map_err(row_err)?,
            base_attributes: parse_json_field(
                row.try_get::<String, _>("base_attributes")
                    .map_err(row_err)?
                    .as_str(),
                "character.base_attributes",
            )?,
            baseline_body_profile: parse_json_field(
                row.try_get::<String, _>("baseline_body_profile")
                    .map_err(row_err)?
                    .as_str(),
                "character.baseline_body_profile",
            )?,
            mana_expression_tendency: str_to_mana_tendency(
                row.try_get::<String, _>("mana_expression_tendency")
                    .map_err(row_err)?
                    .as_str(),
            )?,
            mana_expression_tendency_factor_override: row
                .try_get("mana_expression_tendency_factor_override")
                .map_err(row_err)?,
            mind_model_card_knowledge_id: row
                .try_get("mind_model_card_knowledge_id")
                .map_err(row_err)?,
            temporary_state: parse_json_field(
                row.try_get::<String, _>("temporary_state")
                    .map_err(row_err)?
                    .as_str(),
                "character.temporary_state",
            )?,
            schema_version: row.try_get("schema_version").map_err(row_err)?,
            created_at: parse_rfc3339(
                row.try_get::<String, _>("created_at")
                    .map_err(row_err)?
                    .as_str(),
            )?,
            updated_at: parse_rfc3339(
                row.try_get::<String, _>("updated_at")
                    .map_err(row_err)?
                    .as_str(),
            )?,
        }))
    }

    async fn write_commit_journal(
        tx: &mut Transaction<'_, Sqlite>,
        commit_id: &str,
        world_id: &str,
        base_revision: u64,
        resulting_revision: u64,
        changed_location_ids: &[String],
        changed_knowledge_ids: &[String],
        changed_character_ids: &[String],
        changed_relationship_ids: &[String],
        rollback_patch: &serde_json::Value,
        validation_summary: &serde_json::Value,
    ) -> Result<(), String> {
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO world_editor_commits (
                editor_commit_id, world_id, base_editor_revision, resulting_editor_revision,
                changed_location_ids, changed_knowledge_ids, changed_character_ids,
                changed_relationship_ids, changed_temporal_state_ids, changed_config_keys,
                rollback_patch, validation_summary, author_note, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(commit_id)
        .bind(world_id)
        .bind(base_revision as i64)
        .bind(resulting_revision as i64)
        .bind(serde_json::to_string(changed_location_ids).unwrap_or_else(|_| "[]".to_string()))
        .bind(serde_json::to_string(changed_knowledge_ids).unwrap_or_else(|_| "[]".to_string()))
        .bind(serde_json::to_string(changed_character_ids).unwrap_or_else(|_| "[]".to_string()))
        .bind(serde_json::to_string(changed_relationship_ids).unwrap_or_else(|_| "[]".to_string()))
        .bind("[]")
        .bind("[]")
        .bind(serde_json::to_string(rollback_patch).unwrap_or_else(|_| "{}".to_string()))
        .bind(serde_json::to_string(validation_summary).unwrap_or_else(|_| "{}".to_string()))
        .bind(None::<String>)
        .bind(now.to_rfc3339())
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("Failed to write commit journal: {}", e))?;

        Ok(())
    }

    /// Rollback a commit
    pub async fn rollback(&self, commit_id: &str) -> Result<(), String> {
        let row: Option<(String, String)> = sqlx::query_as(
            r#"
            SELECT world_id, rollback_patch
            FROM world_editor_commits
            WHERE editor_commit_id = ?
            "#,
        )
        .bind(commit_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to get commit: {}", e))?;

        let (world_id, rollback_patch_json) = match row {
            Some(r) => r,
            None => return Err("Commit not found".to_string()),
        };

        if !self.is_world_paused(&world_id).await? {
            return Err("World is not paused - cannot rollback".to_string());
        }

        let rollback_patch: serde_json::Value = serde_json::from_str(&rollback_patch_json)
            .map_err(|e| format!("Failed to parse rollback patch: {}", e))?;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to begin rollback transaction: {}", e))?;

        Self::apply_rollback_section(&mut tx, &rollback_patch, "locations", |tx, id, value| {
            Box::pin(async move {
                if value.is_null() {
                    Self::delete_location(tx, id).await
                } else {
                    let location: LocationNode = serde_json::from_value(value.clone())
                        .map_err(|e| format!("Invalid rollback location {}: {}", id, e))?;
                    Self::save_location(tx, &location).await
                }
            })
        })
        .await?;

        Self::apply_rollback_section(&mut tx, &rollback_patch, "edges", |tx, id, value| {
            Box::pin(async move {
                if value.is_null() {
                    Self::delete_edge(tx, id).await
                } else {
                    let edge: LocationEdge = serde_json::from_value(value.clone())
                        .map_err(|e| format!("Invalid rollback edge {}: {}", id, e))?;
                    Self::save_edge(tx, &edge).await
                }
            })
        })
        .await?;

        Self::apply_rollback_section(&mut tx, &rollback_patch, "knowledge", |tx, id, value| {
            Box::pin(async move {
                if value.is_null() {
                    Self::delete_knowledge(tx, id).await
                } else {
                    let entry: KnowledgeEntry = serde_json::from_value(value.clone())
                        .map_err(|e| format!("Invalid rollback knowledge {}: {}", id, e))?;
                    Self::save_knowledge(tx, &entry).await
                }
            })
        })
        .await?;

        Self::apply_rollback_section(&mut tx, &rollback_patch, "characters", |tx, id, value| {
            Box::pin(async move {
                if value.is_null() {
                    Self::delete_character(tx, id).await
                } else {
                    let character: CharacterRecord = serde_json::from_value(value.clone())
                        .map_err(|e| format!("Invalid rollback character {}: {}", id, e))?;
                    Self::save_character(tx, &character).await
                }
            })
        })
        .await?;

        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE world_editor_commits
            SET validation_summary = ?
            WHERE editor_commit_id = ?
            "#,
        )
        .bind(serde_json::json!({"rolled_back_at": now.to_rfc3339()}).to_string())
        .bind(commit_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to mark commit as rolled back: {}", e))?;

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit rollback transaction: {}", e))?;

        Ok(())
    }

    async fn apply_rollback_section<F>(
        tx: &mut Transaction<'_, Sqlite>,
        patch: &serde_json::Value,
        section: &str,
        mut apply: F,
    ) -> Result<(), String>
    where
        F: for<'a> FnMut(
            &'a mut Transaction<'_, Sqlite>,
            &'a str,
            &'a serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<(), String>> + 'a>,
        >,
    {
        if let Some(entries) = patch.get(section).and_then(|value| value.as_object()) {
            for (id, value) in entries {
                apply(tx, id, value).await?;
            }
        }

        Ok(())
    }

    /// Get commit history for a world
    pub async fn get_commit_history(
        &self,
        world_id: &str,
        limit: u32,
    ) -> Result<Vec<EditorCommitInfo>, String> {
        let rows: Vec<(String, i64, i64, String, String)> = sqlx::query_as(
            r#"
            SELECT editor_commit_id, base_editor_revision, resulting_editor_revision,
                   changed_location_ids, created_at
            FROM world_editor_commits
            WHERE world_id = ?
            ORDER BY resulting_editor_revision DESC
            LIMIT ?
            "#,
        )
        .bind(world_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to get commit history: {}", e))?;

        Ok(rows
            .into_iter()
            .map(|row| EditorCommitInfo {
                commit_id: row.0,
                base_revision: row.1 as u64,
                resulting_revision: row.2 as u64,
                changed_location_ids: serde_json::from_str(&row.3).unwrap_or_default(),
                created_at: row.4,
            })
            .collect())
    }

    async fn insert_location(
        tx: &mut Transaction<'_, Sqlite>,
        location: &LocationNode,
    ) -> Result<(), String> {
        Self::write_location(tx, location, false).await
    }

    async fn update_location(
        tx: &mut Transaction<'_, Sqlite>,
        location: &LocationNode,
    ) -> Result<(), String> {
        let result = Self::write_location_update(tx, location).await?;
        if result == 0 {
            return Err(format!(
                "Location not found for update: {}",
                location.location_id
            ));
        }
        Ok(())
    }

    async fn save_location(
        tx: &mut Transaction<'_, Sqlite>,
        location: &LocationNode,
    ) -> Result<(), String> {
        Self::write_location(tx, location, true).await
    }

    async fn write_location(
        tx: &mut Transaction<'_, Sqlite>,
        location: &LocationNode,
        replace: bool,
    ) -> Result<(), String> {
        let sql = if replace {
            r#"
            INSERT OR REPLACE INTO location_nodes (
                location_id, name, polity_id, parent_id, canonical_level,
                type_label, tags, status, metadata, schema_version,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        } else {
            r#"
            INSERT INTO location_nodes (
                location_id, name, polity_id, parent_id, canonical_level,
                type_label, tags, status, metadata, schema_version,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        };

        sqlx::query(sql)
            .bind(&location.location_id)
            .bind(&location.name)
            .bind(&location.polity_id)
            .bind(&location.parent_id)
            .bind(location_level_to_str(&location.canonical_level))
            .bind(&location.type_label)
            .bind(serde_json::to_string(&location.tags).map_err(|e| e.to_string())?)
            .bind(location_status_to_str(&location.status))
            .bind(serde_json::to_string(&location.metadata).map_err(|e| e.to_string())?)
            .bind(&location.schema_version)
            .bind(location.created_at.to_rfc3339())
            .bind(location.updated_at.to_rfc3339())
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to write location: {}", e))?;

        Self::replace_location_aliases(tx, location).await
    }

    async fn write_location_update(
        tx: &mut Transaction<'_, Sqlite>,
        location: &LocationNode,
    ) -> Result<u64, String> {
        let result = sqlx::query(
            r#"
            UPDATE location_nodes SET
                name = ?, polity_id = ?, parent_id = ?, canonical_level = ?,
                type_label = ?, tags = ?, status = ?, metadata = ?,
                schema_version = ?, updated_at = ?
            WHERE location_id = ?
            "#,
        )
        .bind(&location.name)
        .bind(&location.polity_id)
        .bind(&location.parent_id)
        .bind(location_level_to_str(&location.canonical_level))
        .bind(&location.type_label)
        .bind(serde_json::to_string(&location.tags).map_err(|e| e.to_string())?)
        .bind(location_status_to_str(&location.status))
        .bind(serde_json::to_string(&location.metadata).map_err(|e| e.to_string())?)
        .bind(&location.schema_version)
        .bind(Utc::now().to_rfc3339())
        .bind(&location.location_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("Failed to update location: {}", e))?;

        Self::replace_location_aliases(tx, location).await?;
        Ok(result.rows_affected())
    }

    async fn replace_location_aliases(
        tx: &mut Transaction<'_, Sqlite>,
        location: &LocationNode,
    ) -> Result<(), String> {
        sqlx::query("DELETE FROM location_aliases WHERE location_id = ?")
            .bind(&location.location_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to clear location aliases: {}", e))?;

        for alias in &location.aliases {
            sqlx::query(
                r#"
                INSERT INTO location_aliases (alias, location_id, locale, normalized_alias)
                VALUES (?, ?, ?, ?)
                "#,
            )
            .bind(&alias.alias)
            .bind(&location.location_id)
            .bind(&alias.locale)
            .bind(&alias.normalized_alias)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to write location alias: {}", e))?;
        }

        Ok(())
    }

    async fn delete_location(
        tx: &mut Transaction<'_, Sqlite>,
        location_id: &str,
    ) -> Result<(), String> {
        sqlx::query("DELETE FROM location_aliases WHERE location_id = ?")
            .bind(location_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to delete location aliases: {}", e))?;
        sqlx::query("DELETE FROM location_edges WHERE from_location_id = ? OR to_location_id = ?")
            .bind(location_id)
            .bind(location_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to delete location edges: {}", e))?;
        sqlx::query("DELETE FROM location_spatial_relations WHERE source_location_id = ? OR target_location_id = ?")
            .bind(location_id)
            .bind(location_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to delete spatial relations: {}", e))?;
        sqlx::query("DELETE FROM location_nodes WHERE location_id = ?")
            .bind(location_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to delete location: {}", e))?;
        Ok(())
    }

    async fn insert_edge(
        tx: &mut Transaction<'_, Sqlite>,
        edge: &LocationEdge,
    ) -> Result<(), String> {
        Self::write_edge(tx, edge, false).await
    }

    async fn save_edge(
        tx: &mut Transaction<'_, Sqlite>,
        edge: &LocationEdge,
    ) -> Result<(), String> {
        Self::write_edge(tx, edge, true).await
    }

    async fn write_edge(
        tx: &mut Transaction<'_, Sqlite>,
        edge: &LocationEdge,
        replace: bool,
    ) -> Result<(), String> {
        let sql = if replace {
            r#"
            INSERT OR REPLACE INTO location_edges (
                edge_id, from_location_id, to_location_id, relation, bidirectional,
                distance_km, travel_time, terrain_cost, safety_cost,
                seasonal_modifiers, allowed_modes, confidence, source, schema_version,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        } else {
            r#"
            INSERT INTO location_edges (
                edge_id, from_location_id, to_location_id, relation, bidirectional,
                distance_km, travel_time, terrain_cost, safety_cost,
                seasonal_modifiers, allowed_modes, confidence, source, schema_version,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        };

        sqlx::query(sql)
            .bind(&edge.edge_id)
            .bind(&edge.from_location_id)
            .bind(&edge.to_location_id)
            .bind(edge_relation_to_str(&edge.relation))
            .bind(edge.bidirectional as i32)
            .bind(
                edge.distance_km
                    .as_ref()
                    .map(|d| serde_json::to_string(d).unwrap_or_default()),
            )
            .bind(
                edge.travel_time
                    .as_ref()
                    .map(|t| serde_json::to_string(t).unwrap_or_default()),
            )
            .bind(edge.terrain_cost)
            .bind(edge.safety_cost)
            .bind(serde_json::to_string(&edge.seasonal_modifiers).map_err(|e| e.to_string())?)
            .bind(serde_json::to_string(&edge.allowed_modes).map_err(|e| e.to_string())?)
            .bind(fact_confidence_to_str(&edge.confidence))
            .bind(serde_json::to_string(&edge.source).map_err(|e| e.to_string())?)
            .bind(&edge.schema_version)
            .bind(edge.created_at.to_rfc3339())
            .bind(edge.updated_at.to_rfc3339())
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to write location edge: {}", e))?;

        Ok(())
    }

    async fn delete_edge(tx: &mut Transaction<'_, Sqlite>, edge_id: &str) -> Result<(), String> {
        sqlx::query("DELETE FROM location_edges WHERE edge_id = ?")
            .bind(edge_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to delete location edge: {}", e))?;
        Ok(())
    }

    async fn insert_knowledge(
        tx: &mut Transaction<'_, Sqlite>,
        entry: &KnowledgeEntry,
    ) -> Result<(), String> {
        Self::write_knowledge(tx, entry, false).await
    }

    async fn update_knowledge(
        tx: &mut Transaction<'_, Sqlite>,
        entry: &KnowledgeEntry,
    ) -> Result<(), String> {
        let result = Self::write_knowledge_update(tx, entry).await?;
        if result == 0 {
            return Err(format!(
                "Knowledge not found for update: {}",
                entry.knowledge_id
            ));
        }
        Ok(())
    }

    async fn save_knowledge(
        tx: &mut Transaction<'_, Sqlite>,
        entry: &KnowledgeEntry,
    ) -> Result<(), String> {
        Self::write_knowledge(tx, entry, true).await
    }

    async fn write_knowledge(
        tx: &mut Transaction<'_, Sqlite>,
        entry: &KnowledgeEntry,
        replace: bool,
    ) -> Result<(), String> {
        let sql = if replace {
            r#"
            INSERT OR REPLACE INTO knowledge_entries (
                knowledge_id, kind, subject_type, subject_id, facet_type,
                content, apparent_content, access_policy, subject_awareness, metadata,
                valid_from, valid_until, source_session_id, source_scene_turn_id,
                derived_from_event_id, schema_version, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        } else {
            r#"
            INSERT INTO knowledge_entries (
                knowledge_id, kind, subject_type, subject_id, facet_type,
                content, apparent_content, access_policy, subject_awareness, metadata,
                valid_from, valid_until, source_session_id, source_scene_turn_id,
                derived_from_event_id, schema_version, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        };
        let (subject_type, subject_id, facet_type) = subject_to_columns(&entry.subject);

        sqlx::query(sql)
            .bind(&entry.knowledge_id)
            .bind(knowledge_kind_to_str(&entry.kind))
            .bind(subject_type)
            .bind(subject_id)
            .bind(facet_type)
            .bind(serde_json::to_string(&entry.content).map_err(|e| e.to_string())?)
            .bind(
                entry
                    .apparent_content
                    .as_ref()
                    .map(|c| serde_json::to_string(c).unwrap_or_default()),
            )
            .bind(serde_json::to_string(&entry.access_policy).map_err(|e| e.to_string())?)
            .bind(serde_json::to_string(&entry.subject_awareness).map_err(|e| e.to_string())?)
            .bind(serde_json::to_string(&entry.metadata).map_err(|e| e.to_string())?)
            .bind(
                entry
                    .valid_from
                    .as_ref()
                    .map(|t| serde_json::to_string(t).unwrap_or_default()),
            )
            .bind(
                entry
                    .valid_until
                    .as_ref()
                    .map(|t| serde_json::to_string(t).unwrap_or_default()),
            )
            .bind(&entry.source_session_id)
            .bind(&entry.source_scene_turn_id)
            .bind(&entry.derived_from_event_id)
            .bind(&entry.schema_version)
            .bind(entry.created_at.to_rfc3339())
            .bind(entry.updated_at.to_rfc3339())
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to write knowledge: {}", e))?;

        Self::replace_knowledge_access_indexes(tx, entry).await
    }

    async fn write_knowledge_update(
        tx: &mut Transaction<'_, Sqlite>,
        entry: &KnowledgeEntry,
    ) -> Result<u64, String> {
        let (subject_type, subject_id, facet_type) = subject_to_columns(&entry.subject);
        let result = sqlx::query(
            r#"
            UPDATE knowledge_entries SET
                kind = ?, subject_type = ?, subject_id = ?, facet_type = ?,
                content = ?, apparent_content = ?, access_policy = ?,
                subject_awareness = ?, metadata = ?, valid_from = ?, valid_until = ?,
                source_session_id = ?, source_scene_turn_id = ?, derived_from_event_id = ?,
                schema_version = ?, updated_at = ?
            WHERE knowledge_id = ?
            "#,
        )
        .bind(knowledge_kind_to_str(&entry.kind))
        .bind(subject_type)
        .bind(subject_id)
        .bind(facet_type)
        .bind(serde_json::to_string(&entry.content).map_err(|e| e.to_string())?)
        .bind(
            entry
                .apparent_content
                .as_ref()
                .map(|c| serde_json::to_string(c).unwrap_or_default()),
        )
        .bind(serde_json::to_string(&entry.access_policy).map_err(|e| e.to_string())?)
        .bind(serde_json::to_string(&entry.subject_awareness).map_err(|e| e.to_string())?)
        .bind(serde_json::to_string(&entry.metadata).map_err(|e| e.to_string())?)
        .bind(
            entry
                .valid_from
                .as_ref()
                .map(|t| serde_json::to_string(t).unwrap_or_default()),
        )
        .bind(
            entry
                .valid_until
                .as_ref()
                .map(|t| serde_json::to_string(t).unwrap_or_default()),
        )
        .bind(&entry.source_session_id)
        .bind(&entry.source_scene_turn_id)
        .bind(&entry.derived_from_event_id)
        .bind(&entry.schema_version)
        .bind(Utc::now().to_rfc3339())
        .bind(&entry.knowledge_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("Failed to update knowledge: {}", e))?;

        Self::replace_knowledge_access_indexes(tx, entry).await?;
        Ok(result.rows_affected())
    }

    async fn replace_knowledge_access_indexes(
        tx: &mut Transaction<'_, Sqlite>,
        entry: &KnowledgeEntry,
    ) -> Result<(), String> {
        sqlx::query("DELETE FROM knowledge_access_known_by WHERE knowledge_id = ?")
            .bind(&entry.knowledge_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to clear known_by index: {}", e))?;
        sqlx::query("DELETE FROM knowledge_access_scopes WHERE knowledge_id = ?")
            .bind(&entry.knowledge_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to clear scopes index: {}", e))?;

        for character_id in &entry.access_policy.known_by {
            sqlx::query(
                "INSERT OR IGNORE INTO knowledge_access_known_by (knowledge_id, character_id) VALUES (?, ?)",
            )
            .bind(&entry.knowledge_id)
            .bind(character_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to write known_by index: {}", e))?;
        }

        for scope in &entry.access_policy.scope {
            let (scope_type, scope_value) = scope_to_parts(scope);
            sqlx::query(
                "INSERT OR IGNORE INTO knowledge_access_scopes (knowledge_id, scope_type, scope_value) VALUES (?, ?, ?)",
            )
            .bind(&entry.knowledge_id)
            .bind(scope_type)
            .bind(scope_value)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to write scope index: {}", e))?;
        }

        Ok(())
    }

    async fn delete_knowledge(
        tx: &mut Transaction<'_, Sqlite>,
        knowledge_id: &str,
    ) -> Result<(), String> {
        sqlx::query("DELETE FROM knowledge_access_known_by WHERE knowledge_id = ?")
            .bind(knowledge_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to delete known_by indexes: {}", e))?;
        sqlx::query("DELETE FROM knowledge_access_scopes WHERE knowledge_id = ?")
            .bind(knowledge_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to delete scope indexes: {}", e))?;
        sqlx::query("DELETE FROM knowledge_entries WHERE knowledge_id = ?")
            .bind(knowledge_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to delete knowledge: {}", e))?;
        Ok(())
    }

    async fn insert_character(
        tx: &mut Transaction<'_, Sqlite>,
        character: &CharacterRecord,
    ) -> Result<(), String> {
        Self::write_character(tx, character, false).await
    }

    async fn update_character(
        tx: &mut Transaction<'_, Sqlite>,
        character: &CharacterRecord,
    ) -> Result<(), String> {
        let result = Self::write_character_update(tx, character).await?;
        if result == 0 {
            return Err(format!(
                "Character not found for update: {}",
                character.character_id
            ));
        }
        Ok(())
    }

    async fn save_character(
        tx: &mut Transaction<'_, Sqlite>,
        character: &CharacterRecord,
    ) -> Result<(), String> {
        Self::write_character(tx, character, true).await
    }

    async fn write_character(
        tx: &mut Transaction<'_, Sqlite>,
        character: &CharacterRecord,
        replace: bool,
    ) -> Result<(), String> {
        let sql = if replace {
            r#"
            INSERT OR REPLACE INTO character_records (
                character_id, base_attributes, baseline_body_profile,
                mana_expression_tendency, mana_expression_tendency_factor_override,
                mind_model_card_knowledge_id, temporary_state, schema_version,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        } else {
            r#"
            INSERT INTO character_records (
                character_id, base_attributes, baseline_body_profile,
                mana_expression_tendency, mana_expression_tendency_factor_override,
                mind_model_card_knowledge_id, temporary_state, schema_version,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        };

        sqlx::query(sql)
            .bind(&character.character_id)
            .bind(serde_json::to_string(&character.base_attributes).map_err(|e| e.to_string())?)
            .bind(
                serde_json::to_string(&character.baseline_body_profile)
                    .map_err(|e| e.to_string())?,
            )
            .bind(mana_tendency_to_str(&character.mana_expression_tendency))
            .bind(character.mana_expression_tendency_factor_override)
            .bind(&character.mind_model_card_knowledge_id)
            .bind(serde_json::to_string(&character.temporary_state).map_err(|e| e.to_string())?)
            .bind(&character.schema_version)
            .bind(character.created_at.to_rfc3339())
            .bind(character.updated_at.to_rfc3339())
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to write character: {}", e))?;

        Ok(())
    }

    async fn write_character_update(
        tx: &mut Transaction<'_, Sqlite>,
        character: &CharacterRecord,
    ) -> Result<u64, String> {
        let result = sqlx::query(
            r#"
            UPDATE character_records SET
                base_attributes = ?, baseline_body_profile = ?,
                mana_expression_tendency = ?, mana_expression_tendency_factor_override = ?,
                mind_model_card_knowledge_id = ?, temporary_state = ?,
                schema_version = ?, updated_at = ?
            WHERE character_id = ?
            "#,
        )
        .bind(serde_json::to_string(&character.base_attributes).map_err(|e| e.to_string())?)
        .bind(serde_json::to_string(&character.baseline_body_profile).map_err(|e| e.to_string())?)
        .bind(mana_tendency_to_str(&character.mana_expression_tendency))
        .bind(character.mana_expression_tendency_factor_override)
        .bind(&character.mind_model_card_knowledge_id)
        .bind(serde_json::to_string(&character.temporary_state).map_err(|e| e.to_string())?)
        .bind(&character.schema_version)
        .bind(Utc::now().to_rfc3339())
        .bind(&character.character_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("Failed to update character: {}", e))?;

        Ok(result.rows_affected())
    }

    async fn delete_character(
        tx: &mut Transaction<'_, Sqlite>,
        character_id: &str,
    ) -> Result<(), String> {
        sqlx::query("DELETE FROM character_records WHERE character_id = ?")
            .bind(character_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to delete character: {}", e))?;
        Ok(())
    }
}

/// World editor changes
#[derive(Debug, Clone, Default)]
pub struct WorldEditorChanges {
    pub location_creates: Vec<LocationNode>,
    pub location_updates: Vec<LocationNode>,
    pub location_deletes: Vec<String>,
    pub edge_creates: Vec<LocationEdge>,
    pub edge_deletes: Vec<String>,
    pub knowledge_creates: Vec<KnowledgeEntry>,
    pub knowledge_updates: Vec<KnowledgeEntry>,
    pub knowledge_deletes: Vec<String>,
    pub character_creates: Vec<CharacterRecord>,
    pub character_updates: Vec<CharacterRecord>,
    pub character_deletes: Vec<String>,
}

/// Editor commit result
#[derive(Debug, Clone)]
pub struct EditorCommitResult {
    pub commit_id: String,
    pub resulting_revision: u64,
    pub changed_location_ids: Vec<String>,
    pub changed_knowledge_ids: Vec<String>,
    pub changed_character_ids: Vec<String>,
}

/// Editor commit info (for history listing)
#[derive(Debug, Clone)]
pub struct EditorCommitInfo {
    pub commit_id: String,
    pub base_revision: u64,
    pub resulting_revision: u64,
    pub changed_location_ids: Vec<String>,
    pub created_at: String,
}

fn row_err(error: sqlx::Error) -> String {
    format!("Failed to read database row: {}", error)
}

fn parse_json_field<T: serde::de::DeserializeOwned>(value: &str, field: &str) -> Result<T, String> {
    serde_json::from_str(value).map_err(|e| format!("Invalid {} JSON: {}", field, e))
}

fn parse_optional_json_field<T: serde::de::DeserializeOwned>(
    value: Option<String>,
    field: &str,
) -> Result<Option<T>, String> {
    value
        .map(|json| parse_json_field(json.as_str(), field))
        .transpose()
}

fn parse_rfc3339(value: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| format!("Invalid timestamp {}: {}", value, e))
}

fn location_level_to_str(level: &LocationLevel) -> &'static str {
    match level {
        LocationLevel::WorldRoot => "world_root",
        LocationLevel::Realm => "realm",
        LocationLevel::Continent => "continent",
        LocationLevel::NaturalRegion => "natural_region",
        LocationLevel::Polity => "polity",
        LocationLevel::MajorRegion => "major_region",
        LocationLevel::LocalRegion => "local_region",
        LocationLevel::Settlement => "settlement",
        LocationLevel::DistrictOrSite => "district_or_site",
        LocationLevel::RoomOrSubsite => "room_or_subsite",
    }
}

fn str_to_location_level(value: &str) -> Result<LocationLevel, String> {
    match value {
        "world_root" => Ok(LocationLevel::WorldRoot),
        "realm" => Ok(LocationLevel::Realm),
        "continent" => Ok(LocationLevel::Continent),
        "natural_region" => Ok(LocationLevel::NaturalRegion),
        "polity" => Ok(LocationLevel::Polity),
        "major_region" => Ok(LocationLevel::MajorRegion),
        "local_region" => Ok(LocationLevel::LocalRegion),
        "settlement" => Ok(LocationLevel::Settlement),
        "district_or_site" => Ok(LocationLevel::DistrictOrSite),
        "room_or_subsite" => Ok(LocationLevel::RoomOrSubsite),
        _ => Err(format!("Invalid location level: {}", value)),
    }
}

fn location_status_to_str(status: &LocationStatus) -> &'static str {
    match status {
        LocationStatus::Active => "active",
        LocationStatus::Deprecated => "deprecated",
        LocationStatus::PendingConfirmation => "pending_confirmation",
    }
}

fn str_to_location_status(value: &str) -> Result<LocationStatus, String> {
    match value {
        "active" => Ok(LocationStatus::Active),
        "deprecated" => Ok(LocationStatus::Deprecated),
        "pending_confirmation" => Ok(LocationStatus::PendingConfirmation),
        _ => Err(format!("Invalid location status: {}", value)),
    }
}

fn edge_relation_to_str(relation: &LocationEdgeRelation) -> &'static str {
    match relation {
        LocationEdgeRelation::Adjacent => "adjacent",
        LocationEdgeRelation::Road => "road",
        LocationEdgeRelation::RiverRoute => "river_route",
        LocationEdgeRelation::SeaRoute => "sea_route",
        LocationEdgeRelation::MountainPass => "mountain_pass",
        LocationEdgeRelation::ForestTrail => "forest_trail",
        LocationEdgeRelation::BorderCrossing => "border_crossing",
        LocationEdgeRelation::TeleportGate => "teleport_gate",
        LocationEdgeRelation::ContainsShortcut => "contains_shortcut",
    }
}

fn str_to_edge_relation(value: &str) -> Result<LocationEdgeRelation, String> {
    match value {
        "adjacent" => Ok(LocationEdgeRelation::Adjacent),
        "road" => Ok(LocationEdgeRelation::Road),
        "river_route" => Ok(LocationEdgeRelation::RiverRoute),
        "sea_route" => Ok(LocationEdgeRelation::SeaRoute),
        "mountain_pass" => Ok(LocationEdgeRelation::MountainPass),
        "forest_trail" => Ok(LocationEdgeRelation::ForestTrail),
        "border_crossing" => Ok(LocationEdgeRelation::BorderCrossing),
        "teleport_gate" => Ok(LocationEdgeRelation::TeleportGate),
        "contains_shortcut" => Ok(LocationEdgeRelation::ContainsShortcut),
        _ => Err(format!("Invalid edge relation: {}", value)),
    }
}

fn knowledge_kind_to_str(kind: &KnowledgeKind) -> &'static str {
    match kind {
        KnowledgeKind::WorldFact => "world_fact",
        KnowledgeKind::RegionFact => "region_fact",
        KnowledgeKind::FactionFact => "faction_fact",
        KnowledgeKind::CharacterFacet => "character_facet",
        KnowledgeKind::HistoricalEvent => "historical_event",
        KnowledgeKind::Memory => "memory",
    }
}

fn str_to_knowledge_kind(value: &str) -> Result<KnowledgeKind, String> {
    match value {
        "world_fact" => Ok(KnowledgeKind::WorldFact),
        "region_fact" => Ok(KnowledgeKind::RegionFact),
        "faction_fact" => Ok(KnowledgeKind::FactionFact),
        "character_facet" => Ok(KnowledgeKind::CharacterFacet),
        "historical_event" => Ok(KnowledgeKind::HistoricalEvent),
        "memory" => Ok(KnowledgeKind::Memory),
        _ => Err(format!("Invalid knowledge kind: {}", value)),
    }
}

fn subject_to_columns(subject: &KnowledgeSubject) -> (String, Option<String>, Option<String>) {
    match subject {
        KnowledgeSubject::World => ("world".to_string(), None, None),
        KnowledgeSubject::Region(id) => ("region".to_string(), Some(id.clone()), None),
        KnowledgeSubject::Faction(id) => ("faction".to_string(), Some(id.clone()), None),
        KnowledgeSubject::Character { id, facet } => (
            "character".to_string(),
            Some(id.clone()),
            Some(facet_type_to_str(facet)),
        ),
        KnowledgeSubject::Event { event_id } => ("event".to_string(), Some(event_id.clone()), None),
    }
}

fn columns_to_subject(
    subject_type: &str,
    subject_id: Option<String>,
    facet_type: Option<String>,
) -> Result<KnowledgeSubject, String> {
    match subject_type {
        "world" => Ok(KnowledgeSubject::World),
        "region" => Ok(KnowledgeSubject::Region(subject_id.unwrap_or_default())),
        "faction" => Ok(KnowledgeSubject::Faction(subject_id.unwrap_or_default())),
        "character" => Ok(KnowledgeSubject::Character {
            id: subject_id.unwrap_or_default(),
            facet: facet_type
                .map(|value| str_to_facet_type(&value))
                .transpose()?
                .unwrap_or(CharacterFacetType::Identity),
        }),
        "event" => Ok(KnowledgeSubject::Event {
            event_id: subject_id.unwrap_or_default(),
        }),
        _ => Err(format!("Invalid subject type: {}", subject_type)),
    }
}

fn facet_type_to_str(facet: &CharacterFacetType) -> String {
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
    .to_string()
}

fn str_to_facet_type(value: &str) -> Result<CharacterFacetType, String> {
    match value {
        "appearance" => Ok(CharacterFacetType::Appearance),
        "identity" => Ok(CharacterFacetType::Identity),
        "true_name" => Ok(CharacterFacetType::TrueName),
        "species" => Ok(CharacterFacetType::Species),
        "bloodline" => Ok(CharacterFacetType::Bloodline),
        "cultivation_realm" => Ok(CharacterFacetType::CultivationRealm),
        "known_ability" => Ok(CharacterFacetType::KnownAbility),
        "hidden_ability" => Ok(CharacterFacetType::HiddenAbility),
        "personality" => Ok(CharacterFacetType::Personality),
        "background" => Ok(CharacterFacetType::Background),
        "motivation" => Ok(CharacterFacetType::Motivation),
        "trauma" => Ok(CharacterFacetType::Trauma),
        "mind_model_card" => Ok(CharacterFacetType::MindModelCard),
        _ => Err(format!("Invalid facet type: {}", value)),
    }
}

fn scope_to_parts(scope: &AccessScope) -> (String, String) {
    match scope {
        AccessScope::Public => ("public".to_string(), "".to_string()),
        AccessScope::GodOnly => ("god_only".to_string(), "".to_string()),
        AccessScope::Region(id) => ("region".to_string(), id.clone()),
        AccessScope::Faction(id) => ("faction".to_string(), id.clone()),
        AccessScope::Realm(id) => ("realm".to_string(), id.clone()),
        AccessScope::Role(id) => ("role".to_string(), id.clone()),
        AccessScope::Bloodline(id) => ("bloodline".to_string(), id.clone()),
    }
}

fn fact_confidence_to_str(confidence: &FactConfidence) -> &'static str {
    match confidence {
        FactConfidence::Asserted => "asserted",
        FactConfidence::High => "high",
        FactConfidence::Medium => "medium",
        FactConfidence::Low => "low",
        FactConfidence::Inferred => "inferred",
    }
}

fn str_to_fact_confidence(value: &str) -> Result<FactConfidence, String> {
    match value {
        "asserted" => Ok(FactConfidence::Asserted),
        "high" => Ok(FactConfidence::High),
        "medium" => Ok(FactConfidence::Medium),
        "low" => Ok(FactConfidence::Low),
        "inferred" => Ok(FactConfidence::Inferred),
        _ => Err(format!("Invalid fact confidence: {}", value)),
    }
}

fn mana_tendency_to_str(tendency: &ManaExpressionTendency) -> &'static str {
    match tendency {
        ManaExpressionTendency::Inward => "inward",
        ManaExpressionTendency::Neutral => "neutral",
        ManaExpressionTendency::Expressive => "expressive",
    }
}

fn str_to_mana_tendency(value: &str) -> Result<ManaExpressionTendency, String> {
    match value {
        "inward" | "Inward" => Ok(ManaExpressionTendency::Inward),
        "neutral" | "Neutral" => Ok(ManaExpressionTendency::Neutral),
        "expressive" | "Expressive" => Ok(ManaExpressionTendency::Expressive),
        _ => Err(format!("Invalid mana tendency: {}", value)),
    }
}
