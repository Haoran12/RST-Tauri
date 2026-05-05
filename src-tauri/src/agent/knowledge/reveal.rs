//! Knowledge reveal handler
//!
//! Handles KnowledgeRevealEvent processing.

use chrono::{DateTime, Utc};
use serde_json;
use sqlx::FromRow;
use sqlx::SqlitePool;

use crate::agent::models::{
    AccessPolicy, AccessScope, AccessScopeChange, CharacterFacetType, KnowledgeEntry,
    KnowledgeKind, KnowledgeMetadata, KnowledgeRevealEvent, KnowledgeSubject, MemoryContent,
    MemorySource, SubjectAwareness,
};

/// Knowledge reveal handler - processes reveal events
pub struct KnowledgeRevealHandler {
    pool: SqlitePool,
}

impl KnowledgeRevealHandler {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Process a knowledge reveal event
    pub async fn process_reveal(&self, event: &KnowledgeRevealEvent) -> Result<(), String> {
        // Step 1: Get the original knowledge entry
        let original = self
            .get_knowledge_entry(&event.knowledge_id)
            .await?
            .ok_or_else(|| format!("Knowledge entry {} not found", event.knowledge_id))?;

        // Step 2: If GodOnly, verify scope_change removes it
        let has_god_only = original
            .access_policy
            .scope
            .iter()
            .any(|s| matches!(s, AccessScope::GodOnly));
        if has_god_only {
            match &event.scope_change {
                Some(AccessScopeChange::RemoveGodOnly) => {
                    // Valid - GodOnly will be removed
                }
                Some(AccessScopeChange::ReplaceScopes(new_scopes)) => {
                    // Valid if new scopes don't contain GodOnly
                    if new_scopes.iter().any(|s| matches!(s, AccessScope::GodOnly)) {
                        return Err(
                            "Cannot reveal GodOnly knowledge with new GodOnly scope".to_string()
                        );
                    }
                }
                None => {
                    return Err("GodOnly knowledge requires scope_change to reveal".to_string());
                }
            }
        }

        // Step 3: Update knowledge_entries.access_policy
        let new_policy = self.update_access_policy(&original.access_policy, event)?;
        self.update_knowledge_policy(&event.knowledge_id, &new_policy)
            .await?;

        // Step 4: Update knowledge_access_known_by index
        for character_id in &event.newly_known_by {
            sqlx::query(
                "INSERT OR IGNORE INTO knowledge_access_known_by (knowledge_id, character_id) VALUES (?, ?)"
            )
            .bind(&event.knowledge_id)
            .bind(character_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to update known_by index: {}", e))?;
        }

        // Step 5: Update knowledge_access_scopes index if scope changed
        if let Some(scope_change) = &event.scope_change {
            // Remove old scopes
            sqlx::query("DELETE FROM knowledge_access_scopes WHERE knowledge_id = ?")
                .bind(&event.knowledge_id)
                .execute(&self.pool)
                .await
                .map_err(|e| format!("Failed to clear scopes index: {}", e))?;

            // Insert new scopes
            for scope in &new_policy.scope {
                let (scope_type, scope_value) = scope_to_columns(scope);
                sqlx::query(
                    "INSERT INTO knowledge_access_scopes (knowledge_id, scope_type, scope_value) VALUES (?, ?, ?)"
                )
                .bind(&event.knowledge_id)
                .bind(scope_type)
                .bind(scope_value)
                .execute(&self.pool)
                .await
                .map_err(|e| format!("Failed to insert scopes index: {}", e))?;
            }
        }

        // Step 6: Create Memory knowledge entry for the reveal
        self.create_reveal_memory(event, &original).await?;

        // Step 7: Record the reveal event
        self.record_reveal_event(event).await?;

        Ok(())
    }

    /// Validate reveal event before processing
    pub async fn validate_reveal(&self, event: &KnowledgeRevealEvent) -> Result<(), String> {
        // Check that the knowledge entry exists
        let original = self
            .get_knowledge_entry(&event.knowledge_id)
            .await?
            .ok_or_else(|| format!("Knowledge entry {} not found", event.knowledge_id))?;

        // Check that GodOnly entries have scope_change
        let has_god_only = original
            .access_policy
            .scope
            .iter()
            .any(|s| matches!(s, AccessScope::GodOnly));
        if has_god_only && event.scope_change.is_none() {
            return Err("GodOnly knowledge requires scope_change to reveal".to_string());
        }

        // Check that newly_known_by is not empty
        if event.newly_known_by.is_empty() {
            return Err("newly_known_by must not be empty".to_string());
        }

        // Check that characters exist
        for character_id in &event.newly_known_by {
            let exists = self.character_exists(character_id).await?;
            if !exists {
                return Err(format!("Character {} not found", character_id));
            }
        }

        Ok(())
    }

    /// Get a knowledge entry by ID
    async fn get_knowledge_entry(
        &self,
        knowledge_id: &str,
    ) -> Result<Option<KnowledgeEntry>, String> {
        let row = sqlx::query_as::<_, KnowledgeRow>(
            r#"
            SELECT knowledge_id, kind, subject_type, subject_id, facet_type,
                   content, apparent_content, access_policy, subject_awareness, metadata,
                   valid_from, valid_until, source_session_id, source_scene_turn_id,
                   derived_from_event_id, schema_version, created_at, updated_at
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

    /// Update access policy based on reveal event
    fn update_access_policy(
        &self,
        original: &AccessPolicy,
        event: &KnowledgeRevealEvent,
    ) -> Result<AccessPolicy, String> {
        let mut new_policy = original.clone();

        // Apply scope change
        if let Some(scope_change) = &event.scope_change {
            match scope_change {
                AccessScopeChange::RemoveGodOnly => {
                    new_policy
                        .scope
                        .retain(|s| !matches!(s, AccessScope::GodOnly));
                }
                AccessScopeChange::ReplaceScopes(new_scopes) => {
                    new_policy.scope = new_scopes.clone();
                }
            }
        }

        // Add newly known characters
        for character_id in &event.newly_known_by {
            if !new_policy.known_by.contains(character_id) {
                new_policy.known_by.push(character_id.clone());
            }
        }

        Ok(new_policy)
    }

    /// Update knowledge entry's access policy in database
    async fn update_knowledge_policy(
        &self,
        knowledge_id: &str,
        policy: &AccessPolicy,
    ) -> Result<(), String> {
        let policy_json = serde_json::to_string(policy)
            .map_err(|e| format!("Failed to serialize access_policy: {}", e))?;

        sqlx::query(
            r#"
            UPDATE knowledge_entries
            SET access_policy = ?, updated_at = ?
            WHERE knowledge_id = ?
            "#,
        )
        .bind(&policy_json)
        .bind(Utc::now().to_rfc3339())
        .bind(knowledge_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update knowledge policy: {}", e))?;

        Ok(())
    }

    /// Create a Memory knowledge entry for the reveal
    async fn create_reveal_memory(
        &self,
        event: &KnowledgeRevealEvent,
        original: &KnowledgeEntry,
    ) -> Result<(), String> {
        let memory_id = format!("memory_{}_{}", event.knowledge_id, event.event_id);

        // Build memory content
        let trigger_desc = match &event.trigger {
            crate::agent::models::RevealTrigger::Witnessed => "亲眼见证".to_string(),
            crate::agent::models::RevealTrigger::Told { by_character_id } => {
                format!("由 {} 告知", by_character_id)
            }
            crate::agent::models::RevealTrigger::Inferred { from_knowledge_ids } => {
                format!("从 {} 推断", from_knowledge_ids.join(", "))
            }
            crate::agent::models::RevealTrigger::Awakened => "觉醒/回忆".to_string(),
            crate::agent::models::RevealTrigger::Scripted { event_id } => {
                format!("事件 {} 触发", event_id)
            }
        };

        let summary_text = format!(
            "获知知识 {}：{}（{}）",
            original.knowledge_id,
            original
                .content
                .get("summary_text")
                .and_then(|v| v.as_str())
                .unwrap_or("未知内容"),
            trigger_desc
        );

        let memory_content = MemoryContent {
            summary_text,
            event_type: "knowledge_reveal".to_string(),
            actor: None,
            target: Some(event.knowledge_id.clone()),
            location: None,
            timestamp: Some(Utc::now().to_rfc3339()),
            key_observations: event.newly_known_by.clone(),
            emotional_weight: None,
            extensions: serde_json::Map::new(),
        };

        let content_json = serde_json::to_string(&memory_content)
            .map_err(|e| format!("Failed to serialize memory content: {}", e))?;

        // Create memory entries for each newly known character
        for character_id in &event.newly_known_by {
            let memory_id_for_char = format!("{}_{}", memory_id, character_id);

            sqlx::query(
                r#"
                INSERT INTO knowledge_entries (
                    knowledge_id, kind, subject_type, subject_id, facet_type,
                    content, apparent_content, access_policy, subject_awareness, metadata,
                    valid_from, valid_until, source_session_id, source_scene_turn_id,
                    derived_from_event_id, schema_version, created_at, updated_at
                ) VALUES (?, 'memory', 'character', ?, NULL, ?, NULL, ?, ?, ?, NULL, NULL, ?, ?, ?, '0.1', ?, ?)
                "#,
            )
            .bind(&memory_id_for_char)
            .bind(character_id)
            .bind(&content_json)
            .bind(serde_json::to_string(&AccessPolicy {
                known_by: vec![character_id.clone()],
                scope: vec![AccessScope::Public],
                conditions: vec![],
            }).map_err(|e| format!("Failed to serialize access_policy: {}", e))?)
            .bind(serde_json::to_string(&SubjectAwareness::Aware)
                .map_err(|e| format!("Failed to serialize subject_awareness: {}", e))?)
            .bind(serde_json::to_string(&KnowledgeMetadata {
                created_at: Utc::now(),
                updated_at: Utc::now(),
                valid_from: None,
                valid_until: None,
                source_session_id: None,
                source_scene_turn_id: Some(event.scene_turn_id.clone()),
                derived_from_event_id: Some(event.event_id.clone()),
                emotional_weight: None,
                last_accessed_at: None,
                source: Some(MemorySource::Witnessed),
            }).map_err(|e| format!("Failed to serialize metadata: {}", e))?)
            .bind(&event.scene_turn_id)
            .bind(&event.event_id)
            .bind(Utc::now().to_rfc3339())
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to create memory entry: {}", e))?;
        }

        Ok(())
    }

    /// Record the reveal event in database
    async fn record_reveal_event(&self, event: &KnowledgeRevealEvent) -> Result<(), String> {
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
                event_id, knowledge_id, newly_known_by, trigger, scope_change,
                scene_turn_id, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&event.event_id)
        .bind(&event.knowledge_id)
        .bind(&newly_known_by_json)
        .bind(&trigger_json)
        .bind(&scope_change_json)
        .bind(&event.scene_turn_id)
        .bind(event.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to record reveal event: {}", e))?;

        Ok(())
    }

    /// Check if a character exists
    async fn character_exists(&self, character_id: &str) -> Result<bool, String> {
        let row = sqlx::query_as::<_, CountRow>(
            "SELECT COUNT(*) as count FROM character_records WHERE character_id = ?",
        )
        .bind(character_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to check character existence: {}", e))?;

        Ok(row.count > 0)
    }

    /// Convert database row to KnowledgeEntry
    fn row_to_entry(&self, row: KnowledgeRow) -> Result<KnowledgeEntry, String> {
        let kind = str_to_kind(&row.kind)?;
        let subject = str_to_subject(&row.subject_type, row.subject_id, row.facet_type)?;
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
}

// ===== Database row types =====

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

#[derive(FromRow)]
struct CountRow {
    count: i64,
}

// ===== Helper functions =====

fn str_to_kind(s: &str) -> Result<KnowledgeKind, String> {
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

fn str_to_subject(
    subject_type: &str,
    subject_id: Option<String>,
    facet_type: Option<String>,
) -> Result<KnowledgeSubject, String> {
    use crate::agent::models::CharacterFacetType;

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

fn str_to_facet_type(s: &str) -> Result<CharacterFacetType, String> {
    use crate::agent::models::CharacterFacetType;

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

fn scope_to_columns(scope: &AccessScope) -> (String, String) {
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
