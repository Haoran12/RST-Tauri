//! Knowledge access protocol
//!
//! SQLite index pre-filtering + KnowledgeAccessResolver final filtering.
//! Builds AccessibleKnowledge for Layer 2.
//!
//! Performance optimization: Uses TurnScopedCache to cache query results
//! within a turn, avoiding redundant database queries.

use chrono::{DateTime, Utc};
use serde_json;
use sqlx::FromRow;
use sqlx::SqlitePool;
use std::sync::Arc;

use crate::agent::cache::{KnowledgeAccessKey, TurnScopedCache};
use crate::agent::models::{AccessibleEntry, AccessibleKnowledge, KnowledgeEntry, TimeAnchor};

use super::access_resolver::{
    AccessSceneContext, CharacterScopeMembership, KnowledgeAccessResolver,
};
use super::store::{columns_to_subject, str_to_kind};

/// Knowledge access protocol - builds Layer 2 AccessibleKnowledge
pub struct KnowledgeAccessProtocol {
    pool: SqlitePool,
    /// Optional cache for performance optimization
    cache: Option<Arc<TurnScopedCache>>,
}

impl KnowledgeAccessProtocol {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool, cache: None }
    }

    /// Create with cache enabled
    pub fn with_cache(pool: SqlitePool, cache: Arc<TurnScopedCache>) -> Self {
        Self {
            pool,
            cache: Some(cache),
        }
    }

    /// Build accessible knowledge for a character at a specific time
    pub async fn build_accessible_knowledge(
        &self,
        character_id: &str,
        scene_turn_id: &str,
        time_anchor: &TimeAnchor,
        scene_context: Option<&AccessSceneContext>,
    ) -> Result<AccessibleKnowledge, String> {
        // Check cache first
        if let Some(ref cache) = self.cache {
            let key = KnowledgeAccessKey::new(character_id, scene_turn_id, time_anchor);
            if let Some(cached) = cache.get_knowledge(&key) {
                return Ok((*cached).clone());
            }
        }

        // Compute if not cached
        let result = self
            .build_accessible_knowledge_uncached(
                character_id,
                scene_turn_id,
                time_anchor,
                scene_context,
            )
            .await?;

        // Cache the result
        if let Some(ref cache) = self.cache {
            let key = KnowledgeAccessKey::new(character_id, scene_turn_id, time_anchor);
            cache.insert_knowledge(key, Arc::new(result.clone()));
        }

        Ok(result)
    }

    /// Internal implementation without caching
    async fn build_accessible_knowledge_uncached(
        &self,
        character_id: &str,
        scene_turn_id: &str,
        time_anchor: &TimeAnchor,
        scene_context: Option<&AccessSceneContext>,
    ) -> Result<AccessibleKnowledge, String> {
        // Step 1: Pre-filter candidates using SQLite indexes
        let candidates = self.query_candidates(character_id, time_anchor).await?;

        // Step 2: Get character scope memberships
        let memberships = self.get_character_scopes(character_id).await?;

        // Step 3: Final filtering by KnowledgeAccessResolver, including DB-backed conditions
        let mut entries = Vec::new();
        for entry in &candidates {
            if KnowledgeAccessResolver::can_access_async(
                entry,
                character_id,
                &memberships,
                scene_context,
                &self.pool,
            )
            .await?
            {
                let (content, source) =
                    KnowledgeAccessResolver::resolve_content(entry, character_id);
                entries.push(AccessibleEntry {
                    knowledge_id: entry.knowledge_id.clone(),
                    kind: entry.kind,
                    subject: entry.subject.clone(),
                    accessible_content: content,
                    source_hint: source,
                });
            }
        }

        Ok(AccessibleKnowledge {
            character_id: character_id.to_string(),
            scene_turn_id: scene_turn_id.to_string(),
            entries,
        })
    }

    /// Query candidate knowledge entries using indexes
    async fn query_candidates(
        &self,
        character_id: &str,
        time_anchor: &TimeAnchor,
    ) -> Result<Vec<KnowledgeEntry>, String> {
        // Step 1: Query knowledge_access_known_by for direct known_by matches
        let known_by_rows = sqlx::query_as::<_, KnowledgeIdRow>(
            r#"
            SELECT knowledge_id FROM knowledge_access_known_by
            WHERE character_id = ?
            "#,
        )
        .bind(character_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to query known_by index: {}", e))?;

        // Step 2: Query knowledge_access_scopes matching character's scopes
        let scope_rows = sqlx::query_as::<_, KnowledgeIdRow>(
            r#"
            SELECT DISTINCT kas.knowledge_id
            FROM knowledge_access_scopes kas
            INNER JOIN character_scope_memberships csm
                ON kas.scope_type = csm.scope_type
                AND (kas.scope_value = csm.scope_value OR kas.scope_value = '')
            WHERE csm.character_id = ?
              AND kas.scope_type != 'god_only'
            "#,
        )
        .bind(character_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to query scopes index: {}", e))?;

        // Step 3: Query public scope knowledge
        let public_rows = sqlx::query_as::<_, KnowledgeIdRow>(
            r#"
            SELECT knowledge_id FROM knowledge_access_scopes
            WHERE scope_type = 'public'
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to query public scope: {}", e))?;

        // Step 4: Include condition-only entries. These have no derived index row
        // unless they also define known_by/scope, so they must be prefiltered
        // from the base table and finalized by KnowledgeAccessResolver.
        let condition_rows = sqlx::query_as::<_, KnowledgeIdRow>(
            r#"
            SELECT knowledge_id
            FROM knowledge_entries
            WHERE json_array_length(json_extract(access_policy, '$.conditions')) > 0
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to query conditional knowledge: {}", e))?;

        // Step 5: Combine all candidate IDs
        let mut candidate_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        for row in known_by_rows {
            candidate_ids.insert(row.knowledge_id);
        }
        for row in scope_rows {
            candidate_ids.insert(row.knowledge_id);
        }
        for row in public_rows {
            candidate_ids.insert(row.knowledge_id);
        }
        for row in condition_rows {
            candidate_ids.insert(row.knowledge_id);
        }

        // Step 6: Query full knowledge entries for candidates, filtered by time
        let candidates = self
            .query_knowledge_entries_by_ids(&candidate_ids, time_anchor)
            .await?;

        Ok(candidates)
    }

    /// Get character scope memberships
    async fn get_character_scopes(
        &self,
        character_id: &str,
    ) -> Result<Vec<CharacterScopeMembership>, String> {
        let rows = sqlx::query_as::<_, ScopeMembershipRow>(
            r#"
            SELECT character_id, scope_type, scope_value
            FROM character_scope_memberships
            WHERE character_id = ?
            "#,
        )
        .bind(character_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to query character scopes: {}", e))?;

        Ok(rows
            .into_iter()
            .map(|row| CharacterScopeMembership {
                character_id: row.character_id,
                scope_type: row.scope_type,
                scope_value: row.scope_value,
            })
            .collect())
    }

    /// Query full knowledge entries by IDs with time filter
    async fn query_knowledge_entries_by_ids(
        &self,
        ids: &std::collections::HashSet<String>,
        time_anchor: &TimeAnchor,
    ) -> Result<Vec<KnowledgeEntry>, String> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        // Build IN clause
        let id_list: Vec<String> = ids.iter().cloned().collect();
        let placeholders: Vec<&str> = id_list.iter().map(|_| "?").collect();
        let in_clause = placeholders.join(",");

        let query = format!(
            r#"
            SELECT knowledge_id, kind, subject_type, subject_id, facet_type,
                   content, apparent_content, access_policy, subject_awareness, metadata,
                   valid_from, valid_until, source_session_id, source_scene_turn_id, derived_from_event_id,
                   schema_version, created_at, updated_at
            FROM knowledge_entries
            WHERE knowledge_id IN ({})
              AND (
                valid_from IS NULL OR (
                    json_extract(valid_from, '$.calendar_id') = ?
                    AND CAST(json_extract(valid_from, '$.ordinal') AS INTEGER) <= ?
                )
              )
              AND (
                valid_until IS NULL OR (
                    json_extract(valid_until, '$.calendar_id') = ?
                    AND CAST(json_extract(valid_until, '$.ordinal') AS INTEGER) >= ?
                )
              )
            ORDER BY created_at ASC
            "#,
            in_clause
        );

        // Build query with bindings
        let mut query_builder = sqlx::query_as::<_, KnowledgeRow>(&query);
        for id in &id_list {
            query_builder = query_builder.bind(id);
        }
        query_builder = query_builder.bind(&time_anchor.calendar_id);
        query_builder = query_builder.bind(time_anchor.ordinal);
        query_builder = query_builder.bind(&time_anchor.calendar_id);
        query_builder = query_builder.bind(time_anchor.ordinal);

        let rows = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("Failed to query knowledge entries: {}", e))?;

        self.rows_to_entries(rows)
    }

    /// Build accessible knowledge for a past timeline (TimeAnchor query)
    pub async fn build_accessible_knowledge_at_time(
        &self,
        character_id: &str,
        scene_turn_id: &str,
        time_anchor: &TimeAnchor,
    ) -> Result<AccessibleKnowledge, String> {
        // For past timeline, use the same logic but with temporal_state_records
        // to reconstruct character's scope memberships at that time
        let memberships = self
            .get_character_scopes_at_time(character_id, time_anchor)
            .await?;
        let candidates = self.query_candidates(character_id, time_anchor).await?;

        let mut entries = Vec::new();
        for entry in &candidates {
            if KnowledgeAccessResolver::can_access_async(
                entry,
                character_id,
                &memberships,
                None,
                &self.pool,
            )
            .await?
            {
                let (content, source) =
                    KnowledgeAccessResolver::resolve_content(entry, character_id);
                entries.push(AccessibleEntry {
                    knowledge_id: entry.knowledge_id.clone(),
                    kind: entry.kind,
                    subject: entry.subject.clone(),
                    accessible_content: content,
                    source_hint: source,
                });
            }
        }

        Ok(AccessibleKnowledge {
            character_id: character_id.to_string(),
            scene_turn_id: scene_turn_id.to_string(),
            entries,
        })
    }

    /// Get character scope memberships at a specific time (for past timeline)
    async fn get_character_scopes_at_time(
        &self,
        character_id: &str,
        time_anchor: &TimeAnchor,
    ) -> Result<Vec<CharacterScopeMembership>, String> {
        let _ = time_anchor;

        // Query temporal_state_records for scope memberships at that time
        // This is a simplified version - full implementation would need to
        // reconstruct from temporal_state_records with state_kind = 'scope_membership'
        let rows = sqlx::query_as::<_, ScopeMembershipRow>(
            r#"
            SELECT character_id, scope_type, scope_value
            FROM character_scope_memberships
            WHERE character_id = ?
            "#,
        )
        .bind(character_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to query character scopes at time: {}", e))?;

        Ok(rows
            .into_iter()
            .map(|row| CharacterScopeMembership {
                character_id: row.character_id,
                scope_type: row.scope_type,
                scope_value: row.scope_value,
            })
            .collect())
    }

    /// Convert database rows to KnowledgeEntry objects
    fn rows_to_entries(&self, rows: Vec<KnowledgeRow>) -> Result<Vec<KnowledgeEntry>, String> {
        rows.into_iter().map(|row| self.row_to_entry(row)).collect()
    }

    /// Convert a single database row to KnowledgeEntry
    fn row_to_entry(&self, row: KnowledgeRow) -> Result<KnowledgeEntry, String> {
        let kind = str_to_kind(&row.kind)?;
        let subject = columns_to_subject(&row.subject_type, row.subject_id, row.facet_type)?;
        let content: serde_json::Value = serde_json::from_str(&row.content)
            .map_err(|e| format!("Failed to parse content: {}", e))?;
        let apparent_content = row
            .apparent_content
            .map(|v| serde_json::from_str(&v))
            .transpose()
            .map_err(|e| format!("Failed to parse apparent_content: {}", e))?;
        let access_policy: crate::agent::models::AccessPolicy =
            serde_json::from_str(&row.access_policy)
                .map_err(|e| format!("Failed to parse access_policy: {}", e))?;
        let subject_awareness: crate::agent::models::SubjectAwareness =
            serde_json::from_str(&row.subject_awareness)
                .map_err(|e| format!("Failed to parse subject_awareness: {}", e))?;
        let metadata: crate::agent::models::KnowledgeMetadata = serde_json::from_str(&row.metadata)
            .map_err(|e| format!("Failed to parse metadata: {}", e))?;
        let valid_from = row
            .valid_from
            .map(|v| serde_json::from_str(&v))
            .transpose()
            .map_err(|e| format!("Failed to parse valid_from: {}", e))?;
        let valid_until = row
            .valid_until
            .map(|v| serde_json::from_str(&v))
            .transpose()
            .map_err(|e| format!("Failed to parse valid_until: {}", e))?;
        let created_at: DateTime<Utc> = row
            .created_at
            .parse()
            .map_err(|e| format!("Failed to parse created_at: {}", e))?;
        let updated_at: DateTime<Utc> = row
            .updated_at
            .parse()
            .map_err(|e| format!("Failed to parse updated_at: {}", e))?;

        Ok(KnowledgeEntry {
            knowledge_id: row.knowledge_id,
            kind,
            subject,
            content,
            apparent_content,
            access_policy,
            subject_awareness,
            metadata,
            valid_from,
            valid_until,
            source_session_id: row.source_session_id,
            source_scene_turn_id: row.source_scene_turn_id,
            derived_from_event_id: row.derived_from_event_id,
            schema_version: row.schema_version,
            created_at,
            updated_at,
        })
    }
}

// ===== Database row types =====

#[derive(FromRow)]
struct KnowledgeIdRow {
    knowledge_id: String,
}

#[derive(FromRow)]
struct ScopeMembershipRow {
    character_id: String,
    scope_type: String,
    scope_value: String,
}

#[derive(FromRow)]
struct KnowledgeRow {
    knowledge_id: String,
    kind: String,
    subject_type: String,
    subject_id: Option<String>,
    facet_type: Option<String>,
    content: String,
    apparent_content: Option<String>,
    access_policy: String,
    subject_awareness: String,
    metadata: String,
    valid_from: Option<String>,
    valid_until: Option<String>,
    source_session_id: Option<String>,
    source_scene_turn_id: Option<String>,
    derived_from_event_id: Option<String>,
    schema_version: String,
    created_at: String,
    updated_at: String,
}
