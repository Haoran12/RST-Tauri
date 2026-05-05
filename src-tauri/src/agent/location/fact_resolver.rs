//! Location fact resolver
//!
//! Handles fact inheritance along parent chain and natural region influence.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;
use sqlx::SqlitePool;

use crate::agent::models::{
    CharacterFacetType, FactConfidence, FactInheritance, KnowledgeEntry, KnowledgeKind,
    LocationNode, RegionFactContent, TimeAnchor,
};

use super::resolver::LocationResolver;

/// Location fact resolver - inherits facts along parent chain
pub struct LocationFactResolver {
    pool: SqlitePool,
    location_resolver: LocationResolver,
}

impl LocationFactResolver {
    pub fn new(pool: SqlitePool) -> Self {
        let location_resolver = LocationResolver::new(pool.clone());
        Self {
            pool,
            location_resolver,
        }
    }

    /// Get inherited public facts for a location
    pub async fn get_inherited_facts(
        &self,
        location_id: &str,
        time_anchor: &TimeAnchor,
    ) -> Result<Vec<InheritedFact>, String> {
        let time_json = serde_json::to_string(time_anchor)
            .map_err(|e| format!("Failed to serialize time anchor: {}", e))?;

        // Step 1: Get ancestor chain (from parent to root)
        let ancestors = self.location_resolver.get_ancestors(location_id).await?;

        // Step 2: Build list of location IDs to check (current + ancestors)
        let mut location_ids = vec![location_id.to_string()];
        for ancestor in &ancestors {
            location_ids.push(ancestor.location_id.clone());
        }

        // Step 3: Query RegionFact knowledge for each location
        let mut facts = Vec::new();

        for (depth, loc_id) in location_ids.iter().enumerate() {
            let location_facts = self.query_region_facts(loc_id, &time_json).await?;

            for entry in location_facts {
                // Parse content to check inheritance rules
                let content: RegionFactContent = match serde_json::from_value(entry.content.clone())
                {
                    Ok(c) => c,
                    Err(_) => continue, // Skip unparseable content
                };

                // Check if this fact should be inherited
                let inheritance = match &content.inheritance {
                    Some(i) => i,
                    None => continue, // No inheritance info, skip
                };

                if !inheritance.inheritable || !inheritance.applies_to_descendants {
                    continue;
                }

                // Check max_depth
                if let Some(max_depth) = inheritance.max_depth {
                    if depth > max_depth as usize {
                        continue;
                    }
                }

                // Check blocked_location_ids
                if inheritance
                    .blocked_location_ids
                    .contains(&location_id.to_string())
                {
                    continue;
                }

                facts.push(InheritedFact {
                    knowledge_id: entry.knowledge_id.clone(),
                    fact_type: content.fact_type.clone(),
                    summary_text: content.summary_text.clone(),
                    inherited_from_location_id: loc_id.clone(),
                    confidence: format!("{:?}", content.confidence),
                });
            }
        }

        // Step 4: Apply override policy (child_overrides_parent)
        let mut seen_fact_types = std::collections::HashSet::new();
        let mut result = Vec::new();

        // Process from child to parent (facts closer to current location take precedence)
        for fact in facts.into_iter().rev() {
            if !seen_fact_types.contains(&fact.fact_type) {
                seen_fact_types.insert(fact.fact_type.clone());
                result.push(fact);
            }
        }

        result.reverse();
        Ok(result)
    }

    /// Get natural region facts affecting a location
    pub async fn get_natural_region_facts(
        &self,
        location_id: &str,
        time_anchor: &TimeAnchor,
    ) -> Result<Vec<NaturalRegionFact>, String> {
        let time_json = serde_json::to_string(time_anchor)
            .map_err(|e| format!("Failed to serialize time anchor: {}", e))?;

        // Step 1: Query location_spatial_relations for overlapping/crossing natural regions
        let relations = sqlx::query_as::<_, SpatialRelationRow>(
            r#"
            SELECT relation_id, source_location_id, target_location_id, relation,
                   coverage, confidence, source
            FROM location_spatial_relations
            WHERE (source_location_id = ? OR target_location_id = ?)
              AND relation IN ('overlaps', 'crosses', 'source_contains_part_of_target',
                               'source_partly_within_target', 'within_natural_band')
            "#,
        )
        .bind(location_id)
        .bind(location_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to query spatial relations: {}", e))?;

        let mut facts = Vec::new();

        for rel in relations {
            // Determine which location is the natural region
            let natural_region_id = if rel.relation == "within_natural_band"
                || rel.relation == "source_partly_within_target"
            {
                // Target is the natural region
                rel.target_location_id.clone()
            } else {
                // Source might be the natural region
                rel.source_location_id.clone()
            };

            // Check if this location is a NaturalRegion
            let is_natural = self.is_natural_region(&natural_region_id).await?;
            if !is_natural {
                continue;
            }

            // Step 2: Get RegionFact for each natural region
            let region_facts = self
                .query_region_facts(&natural_region_id, &time_json)
                .await?;

            for entry in region_facts {
                let content: RegionFactContent = match serde_json::from_value(entry.content.clone())
                {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                let coverage = rel
                    .coverage
                    .as_ref()
                    .and_then(|c| serde_json::from_str::<CoverageData>(c).ok());

                facts.push(NaturalRegionFact {
                    knowledge_id: entry.knowledge_id.clone(),
                    natural_region_id: natural_region_id.clone(),
                    fact_type: content.fact_type.clone(),
                    summary_text: content.summary_text.clone(),
                    relation: rel.relation.clone(),
                    coverage: coverage.map(|c| c.percentage),
                });
            }
        }

        Ok(facts)
    }

    /// Build location context for SceneInitializer
    pub async fn build_location_context(
        &self,
        location_id: &str,
        time_anchor: &TimeAnchor,
    ) -> Result<LocationContext, String> {
        // Get the location node
        let node = self
            .location_resolver
            .get_location(location_id)
            .await?
            .ok_or_else(|| format!("Location {} not found", location_id))?;

        // Get ancestors
        let ancestors = self.location_resolver.get_ancestors(location_id).await?;

        // Get inherited facts
        let inherited_public_facts = self.get_inherited_facts(location_id, time_anchor).await?;

        // Get local facts (facts directly attached to this location)
        let time_json = serde_json::to_string(time_anchor)
            .map_err(|e| format!("Failed to serialize time anchor: {}", e))?;
        let local_entries = self.query_region_facts(location_id, &time_json).await?;
        let local_public_facts: Vec<InheritedFact> = local_entries
            .into_iter()
            .filter_map(|entry| {
                let content: RegionFactContent =
                    serde_json::from_value(entry.content.clone()).ok()?;
                Some(InheritedFact {
                    knowledge_id: entry.knowledge_id.clone(),
                    fact_type: content.fact_type.clone(),
                    summary_text: content.summary_text.clone(),
                    inherited_from_location_id: location_id.to_string(),
                    confidence: format!("{:?}", content.confidence),
                })
            })
            .collect();

        // Get natural region facts
        let natural_region_facts = self
            .get_natural_region_facts(location_id, time_anchor)
            .await?;

        // Get covering natural regions
        let covering_natural_regions = self.get_covering_natural_regions(location_id).await?;

        // Build ancestor briefs
        let ancestor_briefs: Vec<LocationBrief> = ancestors
            .iter()
            .map(|n| LocationBrief {
                location_id: n.location_id.clone(),
                name: n.name.clone(),
                canonical_level: format!("{:?}", n.canonical_level),
                type_label: n.type_label.clone(),
            })
            .collect();

        Ok(LocationContext {
            location_id: location_id.to_string(),
            resolved_name: node.name,
            ancestors: ancestor_briefs,
            covering_natural_regions,
            inherited_public_facts,
            local_public_facts,
            natural_region_facts,
        })
    }

    /// Query region facts for a location
    async fn query_region_facts(
        &self,
        location_id: &str,
        time_json: &str,
    ) -> Result<Vec<KnowledgeEntry>, String> {
        let rows = sqlx::query_as::<_, KnowledgeRow>(
            r#"
            SELECT knowledge_id, kind, subject_type, subject_id, facet_type,
                   content, apparent_content, access_policy, subject_awareness, metadata,
                   valid_from, valid_until, source_session_id, source_scene_turn_id,
                   derived_from_event_id, schema_version, created_at, updated_at
            FROM knowledge_entries
            WHERE kind = 'region_fact'
              AND subject_type = 'region'
              AND subject_id = ?
              AND (valid_from IS NULL OR valid_from <= ?)
              AND (valid_until IS NULL OR valid_until >= ?)
            ORDER BY created_at ASC
            "#,
        )
        .bind(location_id)
        .bind(time_json)
        .bind(time_json)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to query region facts: {}", e))?;

        self.rows_to_entries(rows)
    }

    /// Check if a location is a NaturalRegion
    async fn is_natural_region(&self, location_id: &str) -> Result<bool, String> {
        let row = sqlx::query_as::<_, LevelRow>(
            r#"
            SELECT canonical_level FROM location_nodes WHERE location_id = ?
            "#,
        )
        .bind(location_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to check location level: {}", e))?;

        Ok(row
            .map(|r| r.canonical_level == "natural_region")
            .unwrap_or(false))
    }

    /// Get covering natural regions for a location
    async fn get_covering_natural_regions(
        &self,
        location_id: &str,
    ) -> Result<Vec<LocationBrief>, String> {
        let rows = sqlx::query_as::<_, LocationBriefRow>(
            r#"
            SELECT DISTINCT ln.location_id, ln.name, ln.canonical_level, ln.type_label
            FROM location_nodes ln
            INNER JOIN location_spatial_relations lsr
                ON ln.location_id = lsr.source_location_id
                OR ln.location_id = lsr.target_location_id
            WHERE ln.canonical_level = 'natural_region'
              AND (lsr.source_location_id = ? OR lsr.target_location_id = ?)
            "#,
        )
        .bind(location_id)
        .bind(location_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to get covering natural regions: {}", e))?;

        Ok(rows
            .into_iter()
            .map(|r| LocationBrief {
                location_id: r.location_id,
                name: r.name,
                canonical_level: r.canonical_level,
                type_label: r.type_label,
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
        let subject = str_to_subject(&row.subject_type, row.subject_id, row.facet_type)?;
        let content: serde_json::Value = serde_json::from_str(&row.content)
            .map_err(|e| format!("Failed to parse content: {}", e))?;
        let apparent_content = row
            .apparent_content
            .map(|v| serde_json::from_str(&v))
            .transpose()
            .map_err(|e| format!("Failed to parse apparent_content: {}", e))?;
        let access_policy = serde_json::from_str(&row.access_policy)
            .map_err(|e| format!("Failed to parse access_policy: {}", e))?;
        let subject_awareness = serde_json::from_str(&row.subject_awareness)
            .map_err(|e| format!("Failed to parse subject_awareness: {}", e))?;
        let metadata = serde_json::from_str(&row.metadata)
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

/// Inherited fact from parent location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InheritedFact {
    pub knowledge_id: String,
    pub fact_type: String,
    pub summary_text: String,
    pub inherited_from_location_id: String,
    pub confidence: String,
}

/// Natural region fact affecting a location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NaturalRegionFact {
    pub knowledge_id: String,
    pub natural_region_id: String,
    pub fact_type: String,
    pub summary_text: String,
    pub relation: String,
    pub coverage: Option<f64>,
}

/// Location context for SceneInitializer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationContext {
    pub location_id: String,
    pub resolved_name: String,
    pub ancestors: Vec<LocationBrief>,
    pub covering_natural_regions: Vec<LocationBrief>,
    pub inherited_public_facts: Vec<InheritedFact>,
    pub local_public_facts: Vec<InheritedFact>,
    pub natural_region_facts: Vec<NaturalRegionFact>,
}

/// Brief location info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationBrief {
    pub location_id: String,
    pub name: String,
    pub canonical_level: String,
    pub type_label: String,
}

// ===== Database row types =====

#[derive(FromRow)]
struct SpatialRelationRow {
    relation_id: String,
    source_location_id: String,
    target_location_id: String,
    relation: String,
    coverage: Option<String>,
    confidence: String,
    source: String,
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

#[derive(FromRow)]
struct LevelRow {
    canonical_level: String,
}

#[derive(FromRow)]
struct LocationBriefRow {
    location_id: String,
    name: String,
    canonical_level: String,
    type_label: String,
}

#[derive(Deserialize)]
struct CoverageData {
    percentage: f64,
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
) -> Result<crate::agent::models::KnowledgeSubject, String> {
    use crate::agent::models::{CharacterFacetType, KnowledgeSubject};

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
