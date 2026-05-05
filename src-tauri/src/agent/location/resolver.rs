//! Location resolver
//!
//! Resolves location names to LocationNode, handles aliases and ambiguity.

use chrono::{DateTime, Utc};
use serde_json;
use sqlx::FromRow;
use sqlx::SqlitePool;

use crate::agent::models::{
    LocationAlias, LocationAmbiguity, LocationLevel, LocationNode, LocationStatus,
};

/// Location resolver - resolves names to location IDs
pub struct LocationResolver {
    pool: SqlitePool,
}

impl LocationResolver {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Resolve a location name to a location node
    pub async fn resolve(&self, name: &str) -> Result<LocationResolveResult, String> {
        // Step 1: Normalize the name
        let normalized = Self::normalize_name(name);

        // Step 2: Query location_aliases table
        let candidates = self.query_by_alias(&normalized).await?;

        match candidates.len() {
            0 => Ok(LocationResolveResult::NotFound {
                raw_name: name.to_string(),
            }),
            1 => {
                let location_id = candidates[0].clone();
                let node = self.get_location(&location_id).await?;
                match node {
                    Some(node) => Ok(LocationResolveResult::Resolved {
                        node,
                        ancestors: self.get_ancestors(&location_id).await?,
                    }),
                    None => Ok(LocationResolveResult::NotFound {
                        raw_name: name.to_string(),
                    }),
                }
            }
            _ => Ok(LocationResolveResult::Ambiguous(LocationAmbiguity {
                raw_name: name.to_string(),
                candidate_location_ids: candidates,
                reason: "Multiple locations match this name".to_string(),
            })),
        }
    }

    /// Resolve by exact location ID
    pub async fn resolve_by_id(&self, location_id: &str) -> Result<Option<LocationNode>, String> {
        self.get_location(location_id).await
    }

    /// Normalize a location name for matching
    fn normalize_name(name: &str) -> String {
        // Convert to lowercase, trim whitespace, normalize unicode
        name.to_lowercase()
            .trim()
            .chars()
            .map(|c| if c.is_whitespace() { ' ' } else { c })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Query locations by alias
    async fn query_by_alias(&self, normalized_alias: &str) -> Result<Vec<String>, String> {
        let rows = sqlx::query_as::<_, LocationIdRow>(
            r#"
            SELECT location_id FROM location_aliases
            WHERE normalized_alias = ?
            "#,
        )
        .bind(normalized_alias)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to query location aliases: {}", e))?;

        Ok(rows.into_iter().map(|r| r.location_id).collect())
    }

    /// Get a location node by ID
    pub async fn get_location(&self, location_id: &str) -> Result<Option<LocationNode>, String> {
        let row = sqlx::query_as::<_, LocationRow>(
            r#"
            SELECT location_id, name, polity_id, parent_id, canonical_level,
                   type_label, tags, status, metadata, schema_version,
                   created_at, updated_at
            FROM location_nodes
            WHERE location_id = ?
            "#,
        )
        .bind(location_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to get location: {}", e))?;

        match row {
            Some(row) => {
                let node = self.row_to_node(row).await?;
                Ok(Some(node))
            }
            None => Ok(None),
        }
    }

    /// Get ancestor chain for a location (from immediate parent to root)
    pub async fn get_ancestors(&self, location_id: &str) -> Result<Vec<LocationNode>, String> {
        let mut ancestors = Vec::new();
        let mut current_id = location_id.to_string();
        let mut visited = std::collections::HashSet::new();

        // Prevent infinite loops
        visited.insert(location_id.to_string());

        loop {
            let node = self.get_location(&current_id).await?;
            match node {
                Some(node) => {
                    if let Some(parent_id) = &node.parent_id {
                        if visited.contains(parent_id) {
                            // Cycle detected, stop
                            break;
                        }
                        visited.insert(parent_id.clone());
                        current_id = parent_id.clone();
                        ancestors.push(node);
                    } else {
                        // Reached root
                        break;
                    }
                }
                None => break,
            }
        }

        Ok(ancestors)
    }

    /// Get children of a location
    pub async fn get_children(&self, location_id: &str) -> Result<Vec<LocationNode>, String> {
        let rows = sqlx::query_as::<_, LocationRow>(
            r#"
            SELECT location_id, name, polity_id, parent_id, canonical_level,
                   type_label, tags, status, metadata, schema_version,
                   created_at, updated_at
            FROM location_nodes
            WHERE parent_id = ?
            ORDER BY name ASC
            "#,
        )
        .bind(location_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to get children: {}", e))?;

        let mut children = Vec::new();
        for row in rows {
            let node = self.row_to_node(row).await?;
            children.push(node);
        }

        Ok(children)
    }

    /// Get aliases for a location
    async fn get_aliases(&self, location_id: &str) -> Result<Vec<LocationAlias>, String> {
        let rows = sqlx::query_as::<_, AliasRow>(
            r#"
            SELECT alias, locale, normalized_alias
            FROM location_aliases
            WHERE location_id = ?
            "#,
        )
        .bind(location_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to get aliases: {}", e))?;

        Ok(rows
            .into_iter()
            .map(|r| LocationAlias {
                alias: r.alias,
                locale: r.locale,
                normalized_alias: r.normalized_alias,
            })
            .collect())
    }

    /// Convert database row to LocationNode
    async fn row_to_node(&self, row: LocationRow) -> Result<LocationNode, String> {
        let canonical_level = str_to_location_level(&row.canonical_level)?;
        let status = str_to_location_status(&row.status)?;
        let tags: Vec<String> =
            serde_json::from_str(&row.tags).map_err(|e| format!("Failed to parse tags: {}", e))?;
        let metadata: serde_json::Value = serde_json::from_str(&row.metadata)
            .map_err(|e| format!("Failed to parse metadata: {}", e))?;
        let created_at: DateTime<Utc> = row
            .created_at
            .parse()
            .map_err(|e| format!("Failed to parse created_at: {}", e))?;
        let updated_at: DateTime<Utc> = row
            .updated_at
            .parse()
            .map_err(|e| format!("Failed to parse updated_at: {}", e))?;

        // Get aliases
        let aliases = self.get_aliases(&row.location_id).await?;

        Ok(LocationNode {
            location_id: row.location_id,
            name: row.name,
            aliases,
            polity_id: row.polity_id,
            parent_id: row.parent_id,
            canonical_level,
            type_label: row.type_label,
            tags,
            status,
            metadata,
            schema_version: row.schema_version,
            created_at,
            updated_at,
        })
    }

    /// Query all locations at a specific level
    pub async fn query_by_level(&self, level: LocationLevel) -> Result<Vec<LocationNode>, String> {
        let level_str = location_level_to_str(&level);
        let rows = sqlx::query_as::<_, LocationRow>(
            r#"
            SELECT location_id, name, polity_id, parent_id, canonical_level,
                   type_label, tags, status, metadata, schema_version,
                   created_at, updated_at
            FROM location_nodes
            WHERE canonical_level = ?
            ORDER BY name ASC
            "#,
        )
        .bind(&level_str)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to query by level: {}", e))?;

        let mut nodes = Vec::new();
        for row in rows {
            let node = self.row_to_node(row).await?;
            nodes.push(node);
        }

        Ok(nodes)
    }

    /// Search locations by name pattern
    pub async fn search(&self, pattern: &str, limit: usize) -> Result<Vec<LocationNode>, String> {
        let pattern = format!("%{}%", pattern.to_lowercase());

        let rows = sqlx::query_as::<_, LocationRow>(
            r#"
            SELECT location_id, name, polity_id, parent_id, canonical_level,
                   type_label, tags, status, metadata, schema_version,
                   created_at, updated_at
            FROM location_nodes
            WHERE LOWER(name) LIKE ?
            ORDER BY name ASC
            LIMIT ?
            "#,
        )
        .bind(&pattern)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to search locations: {}", e))?;

        let mut nodes = Vec::new();
        for row in rows {
            let node = self.row_to_node(row).await?;
            nodes.push(node);
        }

        Ok(nodes)
    }
}

/// Result of location resolution
#[derive(Debug, Clone)]
pub enum LocationResolveResult {
    /// Successfully resolved to a single location
    Resolved {
        node: LocationNode,
        ancestors: Vec<LocationNode>,
    },
    /// Multiple candidates found
    Ambiguous(LocationAmbiguity),
    /// No location found
    NotFound { raw_name: String },
}

// ===== Database row types =====

#[derive(FromRow)]
struct LocationIdRow {
    location_id: String,
}

#[derive(FromRow)]
struct LocationRow {
    location_id: String,
    name: String,
    polity_id: Option<String>,
    parent_id: Option<String>,
    canonical_level: String,
    type_label: String,
    tags: String,
    status: String,
    metadata: String,
    schema_version: String,
    created_at: String,
    updated_at: String,
}

#[derive(FromRow)]
struct AliasRow {
    alias: String,
    locale: Option<String>,
    normalized_alias: String,
}

// ===== Helper functions for enum serialization =====

fn location_level_to_str(level: &LocationLevel) -> String {
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
    .to_string()
}

fn str_to_location_level(s: &str) -> Result<LocationLevel, String> {
    match s {
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
        _ => Err(format!("Invalid location level: {}", s)),
    }
}

fn str_to_location_status(s: &str) -> Result<LocationStatus, String> {
    match s {
        "active" => Ok(LocationStatus::Active),
        "deprecated" => Ok(LocationStatus::Deprecated),
        "pending_confirmation" => Ok(LocationStatus::PendingConfirmation),
        _ => Err(format!("Invalid location status: {}", s)),
    }
}
