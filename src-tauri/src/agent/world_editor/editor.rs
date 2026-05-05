//! World editor
//!
//! Structured CRUD for world building.
//!
//! Constraints:
//! - Only operates when world is paused (no active turns/LLM calls)
//! - All changes go through validator before commit
//! - Changes are recorded in editor commit journal

use chrono::Utc;
use sqlx::SqlitePool;

use crate::agent::models::*;

/// World editor - structured CRUD for world building
pub struct WorldEditor {
    pool: SqlitePool,
    world_id: String,
    revision: u64,
    is_paused: bool,
}

impl WorldEditor {
    pub fn new(pool: SqlitePool, world_id: String) -> Self {
        Self {
            pool,
            world_id,
            revision: 0,
            is_paused: true,
        }
    }

    /// Check if world is paused (editable)
    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    /// Set paused state
    pub fn set_paused(&mut self, paused: bool) {
        self.is_paused = paused;
    }

    /// Ensure world is editable
    fn ensure_editable(&self) -> Result<(), String> {
        if !self.is_paused {
            return Err("World is not paused - editor operations not allowed".to_string());
        }
        Ok(())
    }

    /// Create a location node
    pub async fn create_location(&mut self, location: LocationNode) -> Result<String, String> {
        self.ensure_editable()?;

        let location_json = serde_json::to_string(&location)
            .map_err(|e| format!("Failed to serialize location: {}", e))?;

        sqlx::query(
            r#"
            INSERT INTO location_nodes (
                location_id, name, polity_id, parent_id, canonical_level,
                type_label, tags, status, metadata, schema_version,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&location.location_id)
        .bind(&location.name)
        .bind(&location.polity_id)
        .bind(&location.parent_id)
        .bind(location_level_to_str(&location.canonical_level))
        .bind(&location.type_label)
        .bind(serde_json::to_string(&location.tags).unwrap_or_else(|_| "[]".to_string()))
        .bind(location_status_to_str(&location.status))
        .bind(serde_json::to_string(&location.metadata).unwrap_or_else(|_| "{}".to_string()))
        .bind(&location.schema_version)
        .bind(location.created_at.to_rfc3339())
        .bind(location.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create location: {}", e))?;

        // Create aliases
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
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to create location alias: {}", e))?;
        }

        self.revision += 1;
        Ok(location.location_id.clone())
    }

    /// Update a location node
    pub async fn update_location(&mut self, location: &LocationNode) -> Result<(), String> {
        self.ensure_editable()?;

        let now = Utc::now();
        sqlx::query(
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
        .bind(serde_json::to_string(&location.tags).unwrap_or_else(|_| "[]".to_string()))
        .bind(location_status_to_str(&location.status))
        .bind(serde_json::to_string(&location.metadata).unwrap_or_else(|_| "{}".to_string()))
        .bind(&location.schema_version)
        .bind(now.to_rfc3339())
        .bind(&location.location_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update location: {}", e))?;

        self.revision += 1;
        Ok(())
    }

    /// Delete a location node
    pub async fn delete_location(&mut self, location_id: &str) -> Result<(), String> {
        self.ensure_editable()?;

        // Check for children
        let child_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM location_nodes WHERE parent_id = ?")
                .bind(location_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| format!("Failed to check children: {}", e))?;

        if child_count.0 > 0 {
            return Err("Cannot delete location with children".to_string());
        }

        // Delete aliases first
        sqlx::query("DELETE FROM location_aliases WHERE location_id = ?")
            .bind(location_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to delete location aliases: {}", e))?;

        // Delete edges
        sqlx::query("DELETE FROM location_edges WHERE from_location_id = ? OR to_location_id = ?")
            .bind(location_id)
            .bind(location_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to delete location edges: {}", e))?;

        // Delete spatial relations
        sqlx::query("DELETE FROM location_spatial_relations WHERE source_location_id = ? OR target_location_id = ?")
            .bind(location_id)
            .bind(location_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to delete spatial relations: {}", e))?;

        // Delete the node
        sqlx::query("DELETE FROM location_nodes WHERE location_id = ?")
            .bind(location_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to delete location: {}", e))?;

        self.revision += 1;
        Ok(())
    }

    /// Create a location edge
    pub async fn create_edge(&mut self, edge: LocationEdge) -> Result<String, String> {
        self.ensure_editable()?;

        sqlx::query(
            r#"
            INSERT INTO location_edges (
                edge_id, from_location_id, to_location_id, relation, bidirectional,
                distance_km, travel_time, terrain_cost, safety_cost,
                seasonal_modifiers, allowed_modes, confidence, source, schema_version,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&edge.edge_id)
        .bind(&edge.from_location_id)
        .bind(&edge.to_location_id)
        .bind(edge_relation_to_str(&edge.relation))
        .bind(edge.bidirectional as i32)
        .bind(
            edge.distance_km
                .as_ref()
                .map(|d| serde_json::to_string(d).unwrap()),
        )
        .bind(
            edge.travel_time
                .as_ref()
                .map(|t| serde_json::to_string(t).unwrap()),
        )
        .bind(edge.terrain_cost)
        .bind(edge.safety_cost)
        .bind(serde_json::to_string(&edge.seasonal_modifiers).unwrap_or_else(|_| "[]".to_string()))
        .bind(serde_json::to_string(&edge.allowed_modes).unwrap_or_else(|_| "[]".to_string()))
        .bind(fact_confidence_to_str(&edge.confidence))
        .bind(serde_json::to_string(&edge.source).unwrap_or_else(|_| "{}".to_string()))
        .bind(&edge.schema_version)
        .bind(edge.created_at.to_rfc3339())
        .bind(edge.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create edge: {}", e))?;

        self.revision += 1;
        Ok(edge.edge_id.clone())
    }

    /// Create a knowledge entry
    pub async fn create_knowledge(&mut self, entry: KnowledgeEntry) -> Result<String, String> {
        self.ensure_editable()?;

        let content_json = serde_json::to_string(&entry.content)
            .map_err(|e| format!("Failed to serialize content: {}", e))?;
        let apparent_json = entry
            .apparent_content
            .as_ref()
            .map(|c| serde_json::to_string(c).unwrap());
        let policy_json = serde_json::to_string(&entry.access_policy)
            .map_err(|e| format!("Failed to serialize access_policy: {}", e))?;
        let awareness_json = serde_json::to_string(&entry.subject_awareness)
            .map_err(|e| format!("Failed to serialize subject_awareness: {}", e))?;
        let metadata_json = serde_json::to_string(&entry.metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

        sqlx::query(
            r#"
            INSERT INTO knowledge_entries (
                knowledge_id, kind, subject_type, subject_id, facet_type,
                content, apparent_content, access_policy, subject_awareness, metadata,
                valid_from, valid_until, source_session_id, source_scene_turn_id,
                derived_from_event_id, schema_version, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&entry.knowledge_id)
        .bind(knowledge_kind_to_str(&entry.kind))
        .bind(subject_type_to_str(&entry.subject))
        .bind(subject_id(&entry.subject))
        .bind(facet_type(&entry.subject))
        .bind(&content_json)
        .bind(&apparent_json)
        .bind(&policy_json)
        .bind(&awareness_json)
        .bind(&metadata_json)
        .bind(
            entry
                .valid_from
                .as_ref()
                .map(|t| serde_json::to_string(t).unwrap()),
        )
        .bind(
            entry
                .valid_until
                .as_ref()
                .map(|t| serde_json::to_string(t).unwrap()),
        )
        .bind(&entry.source_session_id)
        .bind(&entry.source_scene_turn_id)
        .bind(&entry.derived_from_event_id)
        .bind(&entry.schema_version)
        .bind(entry.created_at.to_rfc3339())
        .bind(entry.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create knowledge: {}", e))?;

        // Update access indexes
        self.update_knowledge_access_indexes(&entry).await?;

        self.revision += 1;
        Ok(entry.knowledge_id.clone())
    }

    /// Update knowledge access indexes
    async fn update_knowledge_access_indexes(&self, entry: &KnowledgeEntry) -> Result<(), String> {
        // Update known_by index
        for character_id in &entry.access_policy.known_by {
            sqlx::query(
                "INSERT OR IGNORE INTO knowledge_access_known_by (knowledge_id, character_id) VALUES (?, ?)",
            )
            .bind(&entry.knowledge_id)
            .bind(character_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to update known_by index: {}", e))?;
        }

        // Update scopes index
        for scope in &entry.access_policy.scope {
            let (scope_type, scope_value) = scope_to_parts(scope);
            sqlx::query(
                "INSERT OR IGNORE INTO knowledge_access_scopes (knowledge_id, scope_type, scope_value) VALUES (?, ?, ?)",
            )
            .bind(&entry.knowledge_id)
            .bind(scope_type)
            .bind(scope_value)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to update scopes index: {}", e))?;
        }

        Ok(())
    }

    /// Update a knowledge entry
    pub async fn update_knowledge(&mut self, entry: &KnowledgeEntry) -> Result<(), String> {
        self.ensure_editable()?;

        let now = Utc::now();
        let content_json = serde_json::to_string(&entry.content)
            .map_err(|e| format!("Failed to serialize content: {}", e))?;
        let apparent_json = entry
            .apparent_content
            .as_ref()
            .map(|c| serde_json::to_string(c).unwrap());
        let policy_json = serde_json::to_string(&entry.access_policy)
            .map_err(|e| format!("Failed to serialize access_policy: {}", e))?;
        let awareness_json = serde_json::to_string(&entry.subject_awareness)
            .map_err(|e| format!("Failed to serialize subject_awareness: {}", e))?;
        let metadata_json = serde_json::to_string(&entry.metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

        sqlx::query(
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
        .bind(subject_type_to_str(&entry.subject))
        .bind(subject_id(&entry.subject))
        .bind(facet_type(&entry.subject))
        .bind(&content_json)
        .bind(&apparent_json)
        .bind(&policy_json)
        .bind(&awareness_json)
        .bind(&metadata_json)
        .bind(
            entry
                .valid_from
                .as_ref()
                .map(|t| serde_json::to_string(t).unwrap()),
        )
        .bind(
            entry
                .valid_until
                .as_ref()
                .map(|t| serde_json::to_string(t).unwrap()),
        )
        .bind(&entry.source_session_id)
        .bind(&entry.source_scene_turn_id)
        .bind(&entry.derived_from_event_id)
        .bind(&entry.schema_version)
        .bind(now.to_rfc3339())
        .bind(&entry.knowledge_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update knowledge: {}", e))?;

        // Rebuild access indexes
        sqlx::query("DELETE FROM knowledge_access_known_by WHERE knowledge_id = ?")
            .bind(&entry.knowledge_id)
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query("DELETE FROM knowledge_access_scopes WHERE knowledge_id = ?")
            .bind(&entry.knowledge_id)
            .execute(&self.pool)
            .await
            .ok();
        self.update_knowledge_access_indexes(entry).await?;

        self.revision += 1;
        Ok(())
    }

    /// Delete a knowledge entry
    pub async fn delete_knowledge(&mut self, knowledge_id: &str) -> Result<(), String> {
        self.ensure_editable()?;

        // Delete access indexes
        sqlx::query("DELETE FROM knowledge_access_known_by WHERE knowledge_id = ?")
            .bind(knowledge_id)
            .execute(&self.pool)
            .await
            .ok();

        sqlx::query("DELETE FROM knowledge_access_scopes WHERE knowledge_id = ?")
            .bind(knowledge_id)
            .execute(&self.pool)
            .await
            .ok();

        // Delete the entry
        sqlx::query("DELETE FROM knowledge_entries WHERE knowledge_id = ?")
            .bind(knowledge_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to delete knowledge: {}", e))?;

        self.revision += 1;
        Ok(())
    }

    /// Create a character record
    pub async fn create_character(&mut self, character: CharacterRecord) -> Result<String, String> {
        self.ensure_editable()?;

        let attrs_json = serde_json::to_string(&character.base_attributes)
            .map_err(|e| format!("Failed to serialize base_attributes: {}", e))?;
        let body_json = serde_json::to_string(&character.baseline_body_profile)
            .map_err(|e| format!("Failed to serialize baseline_body_profile: {}", e))?;
        let temp_json = serde_json::to_string(&character.temporary_state)
            .map_err(|e| format!("Failed to serialize temporary_state: {}", e))?;

        sqlx::query(
            r#"
            INSERT INTO character_records (
                character_id, base_attributes, baseline_body_profile,
                mana_expression_tendency, mana_expression_tendency_factor_override,
                mind_model_card_knowledge_id, temporary_state, schema_version,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&character.character_id)
        .bind(&attrs_json)
        .bind(&body_json)
        .bind(mana_tendency_to_str(&character.mana_expression_tendency))
        .bind(character.mana_expression_tendency_factor_override)
        .bind(&character.mind_model_card_knowledge_id)
        .bind(&temp_json)
        .bind(&character.schema_version)
        .bind(character.created_at.to_rfc3339())
        .bind(character.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create character: {}", e))?;

        self.revision += 1;
        Ok(character.character_id.clone())
    }

    /// Update a character record
    pub async fn update_character(&mut self, character: &CharacterRecord) -> Result<(), String> {
        self.ensure_editable()?;

        let now = Utc::now();
        let attrs_json = serde_json::to_string(&character.base_attributes)
            .map_err(|e| format!("Failed to serialize base_attributes: {}", e))?;
        let body_json = serde_json::to_string(&character.baseline_body_profile)
            .map_err(|e| format!("Failed to serialize baseline_body_profile: {}", e))?;
        let temp_json = serde_json::to_string(&character.temporary_state)
            .map_err(|e| format!("Failed to serialize temporary_state: {}", e))?;

        sqlx::query(
            r#"
            UPDATE character_records SET
                base_attributes = ?, baseline_body_profile = ?,
                mana_expression_tendency = ?, mana_expression_tendency_factor_override = ?,
                mind_model_card_knowledge_id = ?, temporary_state = ?,
                schema_version = ?, updated_at = ?
            WHERE character_id = ?
            "#,
        )
        .bind(&attrs_json)
        .bind(&body_json)
        .bind(mana_tendency_to_str(&character.mana_expression_tendency))
        .bind(character.mana_expression_tendency_factor_override)
        .bind(&character.mind_model_card_knowledge_id)
        .bind(&temp_json)
        .bind(&character.schema_version)
        .bind(now.to_rfc3339())
        .bind(&character.character_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update character: {}", e))?;

        self.revision += 1;
        Ok(())
    }

    /// Delete a character record
    pub async fn delete_character(&mut self, character_id: &str) -> Result<(), String> {
        self.ensure_editable()?;

        sqlx::query("DELETE FROM character_records WHERE character_id = ?")
            .bind(character_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to delete character: {}", e))?;

        self.revision += 1;
        Ok(())
    }

    /// Get current revision
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

// Helper functions for enum conversions

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

fn location_status_to_str(status: &LocationStatus) -> &'static str {
    match status {
        LocationStatus::Active => "active",
        LocationStatus::Deprecated => "deprecated",
        LocationStatus::PendingConfirmation => "pending_confirmation",
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

fn subject_type_to_str(subject: &KnowledgeSubject) -> &'static str {
    match subject {
        KnowledgeSubject::World => "world",
        KnowledgeSubject::Region(_) => "region",
        KnowledgeSubject::Faction(_) => "faction",
        KnowledgeSubject::Character { .. } => "character",
        KnowledgeSubject::Event { .. } => "event",
    }
}

fn subject_id(subject: &KnowledgeSubject) -> Option<String> {
    match subject {
        KnowledgeSubject::World => None,
        KnowledgeSubject::Region(id) => Some(id.clone()),
        KnowledgeSubject::Faction(id) => Some(id.clone()),
        KnowledgeSubject::Character { id, .. } => Some(id.clone()),
        KnowledgeSubject::Event { event_id } => Some(event_id.clone()),
    }
}

fn facet_type(subject: &KnowledgeSubject) -> Option<String> {
    match subject {
        KnowledgeSubject::Character { facet, .. } => Some(facet_type_to_str(facet)),
        _ => None,
    }
}

fn facet_type_to_str(facet: &CharacterFacetType) -> String {
    match facet {
        CharacterFacetType::Appearance => "appearance".to_string(),
        CharacterFacetType::Identity => "identity".to_string(),
        CharacterFacetType::TrueName => "true_name".to_string(),
        CharacterFacetType::Species => "species".to_string(),
        CharacterFacetType::Bloodline => "bloodline".to_string(),
        CharacterFacetType::CultivationRealm => "cultivation_realm".to_string(),
        CharacterFacetType::KnownAbility => "known_ability".to_string(),
        CharacterFacetType::HiddenAbility => "hidden_ability".to_string(),
        CharacterFacetType::Personality => "personality".to_string(),
        CharacterFacetType::Background => "background".to_string(),
        CharacterFacetType::Motivation => "motivation".to_string(),
        CharacterFacetType::Trauma => "trauma".to_string(),
        CharacterFacetType::MindModelCard => "mind_model_card".to_string(),
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

fn mana_tendency_to_str(tendency: &ManaExpressionTendency) -> &'static str {
    match tendency {
        ManaExpressionTendency::Inward => "inward",
        ManaExpressionTendency::Neutral => "neutral",
        ManaExpressionTendency::Expressive => "expressive",
    }
}

fn scope_to_parts(scope: &AccessScope) -> (String, String) {
    match scope {
        AccessScope::Public => ("public".to_string(), "".to_string()),
        AccessScope::GodOnly => ("god_only".to_string(), "".to_string()),
        AccessScope::Region(id) => ("region".to_string(), id.clone()),
        AccessScope::Faction(id) => ("faction".to_string(), id.clone()),
        AccessScope::Realm(level) => ("realm".to_string(), level.clone()),
        AccessScope::Role(role) => ("role".to_string(), role.clone()),
        AccessScope::Bloodline(blood) => ("bloodline".to_string(), blood.clone()),
    }
}
