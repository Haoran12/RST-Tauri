//! Knowledge store - Layer 1 CRUD operations
//!
//! Manages KnowledgeEntry persistence and retrieval.
//! Does NOT perform access control - that's handled by KnowledgeAccessResolver.

use chrono::{DateTime, Utc};
use serde_json;
use sqlx::FromRow;
use sqlx::SqlitePool;

use crate::agent::models::{
    AccessPolicy, CharacterFacetType, KnowledgeEntry, KnowledgeKind, KnowledgeMetadata,
    KnowledgeSubject, SubjectAwareness, TimeAnchor,
};

/// Database row for knowledge entry
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

/// Knowledge store for Layer 1 CRUD
pub struct KnowledgeStore {
    pool: SqlitePool,
}

impl KnowledgeStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new knowledge entry
    pub async fn create(&self, entry: &KnowledgeEntry) -> Result<(), String> {
        let (subject_type, subject_id, facet_type) = subject_to_columns(&entry.subject);
        let content_json = serde_json::to_string(&entry.content)
            .map_err(|e| format!("Failed to serialize content: {}", e))?;
        let apparent_content_json = entry
            .apparent_content
            .as_ref()
            .map(|v| serde_json::to_string(v))
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
            .map(|v| serde_json::to_string(v))
            .transpose()
            .map_err(|e| format!("Failed to serialize valid_from: {}", e))?;
        let valid_until_json = entry
            .valid_until
            .as_ref()
            .map(|v| serde_json::to_string(v))
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
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&entry.knowledge_id)
        .bind(kind_to_str(&entry.kind))
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
        .bind(&entry.source_session_id)
        .bind(&entry.source_scene_turn_id)
        .bind(&entry.derived_from_event_id)
        .bind(&entry.schema_version)
        .bind(entry.created_at.to_rfc3339())
        .bind(entry.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create knowledge entry: {}", e))?;

        // Update derived indexes
        self.update_access_indexes(&entry.knowledge_id, &entry.access_policy)
            .await?;

        Ok(())
    }

    /// Get a knowledge entry by ID
    pub async fn get(&self, knowledge_id: &str) -> Result<Option<KnowledgeEntry>, String> {
        let row = sqlx::query_as::<_, KnowledgeRow>(
            r#"
            SELECT knowledge_id, kind, subject_type, subject_id, facet_type,
                   content, apparent_content, access_policy, subject_awareness, metadata,
                   valid_from, valid_until, source_session_id, source_scene_turn_id, derived_from_event_id,
                   schema_version, created_at, updated_at
            FROM knowledge_entries
            WHERE knowledge_id = ?
            "#,
        )
        .bind(knowledge_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to get knowledge entry: {}", e))?;

        match row {
            Some(row) => {
                let entry = self.row_to_entry(row)?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    /// Update a knowledge entry
    pub async fn update(&self, entry: &KnowledgeEntry) -> Result<(), String> {
        let (subject_type, subject_id, facet_type) = subject_to_columns(&entry.subject);
        let content_json = serde_json::to_string(&entry.content)
            .map_err(|e| format!("Failed to serialize content: {}", e))?;
        let apparent_content_json = entry
            .apparent_content
            .as_ref()
            .map(|v| serde_json::to_string(v))
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
            .map(|v| serde_json::to_string(v))
            .transpose()
            .map_err(|e| format!("Failed to serialize valid_from: {}", e))?;
        let valid_until_json = entry
            .valid_until
            .as_ref()
            .map(|v| serde_json::to_string(v))
            .transpose()
            .map_err(|e| format!("Failed to serialize valid_until: {}", e))?;

        sqlx::query(
            r#"
            UPDATE knowledge_entries SET
                kind = ?, subject_type = ?, subject_id = ?, facet_type = ?,
                content = ?, apparent_content = ?, access_policy = ?, subject_awareness = ?,
                metadata = ?, valid_from = ?, valid_until = ?, source_session_id = ?,
                source_scene_turn_id = ?, derived_from_event_id = ?, schema_version = ?,
                updated_at = ?
            WHERE knowledge_id = ?
            "#,
        )
        .bind(kind_to_str(&entry.kind))
        .bind(&subject_type)
        .bind(&subject_id)
        .bind(&facet_type)
        .bind(&content_json)
        .bind(&apparent_content_json)
        .bind(&access_policy_json)
        .bind(&subject_awareness_json)
        .bind(&metadata_json)
        .bind(&valid_from_json)
        .bind(&valid_until_json)
        .bind(&entry.source_session_id)
        .bind(&entry.source_scene_turn_id)
        .bind(&entry.derived_from_event_id)
        .bind(&entry.schema_version)
        .bind(entry.updated_at.to_rfc3339())
        .bind(&entry.knowledge_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update knowledge entry: {}", e))?;

        // Update derived indexes
        self.update_access_indexes(&entry.knowledge_id, &entry.access_policy)
            .await?;

        Ok(())
    }

    /// Delete a knowledge entry
    pub async fn delete(&self, knowledge_id: &str) -> Result<(), String> {
        // Delete derived indexes first
        sqlx::query("DELETE FROM knowledge_access_known_by WHERE knowledge_id = ?")
            .bind(knowledge_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to delete known_by index: {}", e))?;

        sqlx::query("DELETE FROM knowledge_access_scopes WHERE knowledge_id = ?")
            .bind(knowledge_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to delete scopes index: {}", e))?;

        // Delete the entry
        sqlx::query("DELETE FROM knowledge_entries WHERE knowledge_id = ?")
            .bind(knowledge_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to delete knowledge entry: {}", e))?;

        Ok(())
    }

    /// Query knowledge entries by kind
    pub async fn query_by_kind(&self, kind: KnowledgeKind) -> Result<Vec<KnowledgeEntry>, String> {
        let rows = sqlx::query_as::<_, KnowledgeRow>(
            r#"
            SELECT knowledge_id, kind, subject_type, subject_id, facet_type,
                   content, apparent_content, access_policy, subject_awareness, metadata,
                   valid_from, valid_until, source_session_id, source_scene_turn_id, derived_from_event_id,
                   schema_version, created_at, updated_at
            FROM knowledge_entries
            WHERE kind = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(kind_to_str(&kind))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to query by kind: {}", e))?;

        self.rows_to_entries(rows)
    }

    /// Query knowledge entries by subject
    pub async fn query_by_subject(
        &self,
        subject: &KnowledgeSubject,
    ) -> Result<Vec<KnowledgeEntry>, String> {
        let (subject_type, subject_id, facet_type) = subject_to_columns(subject);

        let rows = if facet_type.is_some() {
            sqlx::query_as::<_, KnowledgeRow>(
                r#"
                SELECT knowledge_id, kind, subject_type, subject_id, facet_type,
                       content, apparent_content, access_policy, subject_awareness, metadata,
                       valid_from, valid_until, source_session_id, source_scene_turn_id, derived_from_event_id,
                       schema_version, created_at, updated_at
                FROM knowledge_entries
                WHERE subject_type = ? AND subject_id = ? AND facet_type = ?
                ORDER BY created_at ASC
                "#,
            )
            .bind(&subject_type)
            .bind(&subject_id)
            .bind(&facet_type)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("Failed to query by subject: {}", e))?
        } else {
            sqlx::query_as::<_, KnowledgeRow>(
                r#"
                SELECT knowledge_id, kind, subject_type, subject_id, facet_type,
                       content, apparent_content, access_policy, subject_awareness, metadata,
                       valid_from, valid_until, source_session_id, source_scene_turn_id, derived_from_event_id,
                       schema_version, created_at, updated_at
                FROM knowledge_entries
                WHERE subject_type = ? AND subject_id = ?
                ORDER BY created_at ASC
                "#,
            )
            .bind(&subject_type)
            .bind(&subject_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("Failed to query by subject: {}", e))?
        };

        self.rows_to_entries(rows)
    }

    /// Query knowledge entries valid at a time anchor
    pub async fn query_valid_at(
        &self,
        time_anchor: &TimeAnchor,
    ) -> Result<Vec<KnowledgeEntry>, String> {
        let rows = sqlx::query_as::<_, KnowledgeRow>(
            r#"
            SELECT knowledge_id, kind, subject_type, subject_id, facet_type,
                   content, apparent_content, access_policy, subject_awareness, metadata,
                   valid_from, valid_until, source_session_id, source_scene_turn_id, derived_from_event_id,
                   schema_version, created_at, updated_at
            FROM knowledge_entries
            WHERE (
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
        )
        .bind(&time_anchor.calendar_id)
        .bind(time_anchor.ordinal)
        .bind(&time_anchor.calendar_id)
        .bind(time_anchor.ordinal)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to query valid at: {}", e))?;

        self.rows_to_entries(rows)
    }

    /// Query character facets
    pub async fn query_character_facets(
        &self,
        character_id: &str,
        facet_type: Option<CharacterFacetType>,
    ) -> Result<Vec<KnowledgeEntry>, String> {
        let rows = match facet_type {
            Some(ft) => {
                sqlx::query_as::<_, KnowledgeRow>(
                    r#"
                    SELECT knowledge_id, kind, subject_type, subject_id, facet_type,
                           content, apparent_content, access_policy, subject_awareness, metadata,
                           valid_from, valid_until, source_session_id, source_scene_turn_id, derived_from_event_id,
                           schema_version, created_at, updated_at
                    FROM knowledge_entries
                    WHERE subject_type = 'character' AND subject_id = ? AND facet_type = ?
                    ORDER BY created_at ASC
                    "#,
                )
                .bind(character_id)
                .bind(facet_type_to_str(&ft))
                .fetch_all(&self.pool)
                .await
                .map_err(|e| format!("Failed to query character facets: {}", e))?
            }
            None => {
                sqlx::query_as::<_, KnowledgeRow>(
                    r#"
                    SELECT knowledge_id, kind, subject_type, subject_id, facet_type,
                           content, apparent_content, access_policy, subject_awareness, metadata,
                           valid_from, valid_until, source_session_id, source_scene_turn_id, derived_from_event_id,
                           schema_version, created_at, updated_at
                    FROM knowledge_entries
                    WHERE subject_type = 'character' AND subject_id = ?
                    ORDER BY created_at ASC
                    "#,
                )
                .bind(character_id)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| format!("Failed to query character facets: {}", e))?
            }
        };

        self.rows_to_entries(rows)
    }

    // ===== Helper methods =====

    /// Convert database row to KnowledgeEntry
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
        let access_policy: AccessPolicy = serde_json::from_str(&row.access_policy)
            .map_err(|e| format!("Failed to parse access_policy: {}", e))?;
        let subject_awareness: SubjectAwareness = serde_json::from_str(&row.subject_awareness)
            .map_err(|e| format!("Failed to parse subject_awareness: {}", e))?;
        let metadata: KnowledgeMetadata = serde_json::from_str(&row.metadata)
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

    /// Convert multiple rows to entries
    fn rows_to_entries(&self, rows: Vec<KnowledgeRow>) -> Result<Vec<KnowledgeEntry>, String> {
        rows.into_iter().map(|row| self.row_to_entry(row)).collect()
    }

    /// Update access derived indexes
    async fn update_access_indexes(
        &self,
        knowledge_id: &str,
        access_policy: &AccessPolicy,
    ) -> Result<(), String> {
        // Delete existing indexes
        sqlx::query("DELETE FROM knowledge_access_known_by WHERE knowledge_id = ?")
            .bind(knowledge_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to clear known_by index: {}", e))?;

        sqlx::query("DELETE FROM knowledge_access_scopes WHERE knowledge_id = ?")
            .bind(knowledge_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to clear scopes index: {}", e))?;

        // Insert known_by index
        for character_id in &access_policy.known_by {
            sqlx::query(
                "INSERT INTO knowledge_access_known_by (knowledge_id, character_id) VALUES (?, ?)",
            )
            .bind(knowledge_id)
            .bind(character_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to insert known_by index: {}", e))?;
        }

        // Insert scopes index
        for scope in &access_policy.scope {
            let (scope_type, scope_value) = scope_to_columns(scope);
            sqlx::query(
                "INSERT INTO knowledge_access_scopes (knowledge_id, scope_type, scope_value) VALUES (?, ?, ?)"
            )
            .bind(knowledge_id)
            .bind(scope_type)
            .bind(scope_value)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to insert scopes index: {}", e))?;
        }

        Ok(())
    }
}

// ===== Helper functions for enum serialization =====

pub fn kind_to_str(kind: &KnowledgeKind) -> &'static str {
    match kind {
        KnowledgeKind::WorldFact => "world_fact",
        KnowledgeKind::RegionFact => "region_fact",
        KnowledgeKind::FactionFact => "faction_fact",
        KnowledgeKind::CharacterFacet => "character_facet",
        KnowledgeKind::HistoricalEvent => "historical_event",
        KnowledgeKind::Memory => "memory",
    }
}

pub fn str_to_kind(s: &str) -> Result<KnowledgeKind, String> {
    match s {
        "world_fact" => Ok(KnowledgeKind::WorldFact),
        "region_fact" => Ok(KnowledgeKind::RegionFact),
        "faction_fact" => Ok(KnowledgeKind::FactionFact),
        "character_facet" => Ok(KnowledgeKind::CharacterFacet),
        "historical_event" => Ok(KnowledgeKind::HistoricalEvent),
        "memory" => Ok(KnowledgeKind::Memory),
        _ => Err(format!("Invalid knowledge kind: {}", s)),
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

pub fn columns_to_subject(
    subject_type: &str,
    subject_id: Option<String>,
    facet_type: Option<String>,
) -> Result<KnowledgeSubject, String> {
    match subject_type {
        "world" => Ok(KnowledgeSubject::World),
        "region" => Ok(KnowledgeSubject::Region(subject_id.unwrap_or_default())),
        "faction" => Ok(KnowledgeSubject::Faction(subject_id.unwrap_or_default())),
        "character" => {
            let facet = facet_type
                .map(|s| str_to_facet_type(&s))
                .transpose()?
                .unwrap_or(CharacterFacetType::Identity);
            Ok(KnowledgeSubject::Character {
                id: subject_id.unwrap_or_default(),
                facet,
            })
        }
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

fn str_to_facet_type(s: &str) -> Result<CharacterFacetType, String> {
    match s {
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
        _ => Err(format!("Invalid facet type: {}", s)),
    }
}

fn scope_to_columns(scope: &crate::agent::models::AccessScope) -> (String, String) {
    match scope {
        crate::agent::models::AccessScope::Public => ("public".to_string(), "".to_string()),
        crate::agent::models::AccessScope::GodOnly => ("god_only".to_string(), "".to_string()),
        crate::agent::models::AccessScope::Region(id) => ("region".to_string(), id.clone()),
        crate::agent::models::AccessScope::Faction(id) => ("faction".to_string(), id.clone()),
        crate::agent::models::AccessScope::Realm(id) => ("realm".to_string(), id.clone()),
        crate::agent::models::AccessScope::Role(id) => ("role".to_string(), id.clone()),
        crate::agent::models::AccessScope::Bloodline(id) => ("bloodline".to_string(), id.clone()),
    }
}
