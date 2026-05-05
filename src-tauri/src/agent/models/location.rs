//! Location models - Layer 1 Truth Store
//!
//! LocationNode, LocationSpatialRelation, LocationEdge, LocationAlias

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::common::*;

/// Location node - hierarchical location in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationNode {
    pub location_id: String,
    pub name: String,
    pub aliases: Vec<LocationAlias>,
    pub polity_id: Option<String>,
    pub parent_id: Option<String>,
    pub canonical_level: LocationLevel,
    /// Display label (e.g., "州", "县", "城", "宗门")
    pub type_label: String,
    pub tags: Vec<String>,
    pub status: LocationStatus,
    pub metadata: serde_json::Value,
    pub schema_version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocationLevel {
    /// Technical root node for a World
    WorldRoot,
    /// Realm / Continent / Plane / Star / Major World
    Realm,
    /// Continent level (optional if Realm is already continent)
    Continent,
    /// Natural region (mountain, plain, desert, forest, sea, etc.)
    NaturalRegion,
    /// Country / Empire / Kingdom / City-state Alliance
    Polity,
    /// State / Province / Major Region
    MajorRegion,
    /// County / Prefecture / Territory
    LocalRegion,
    /// Settlement (town, city, village, camp)
    Settlement,
    /// District / Site (building, sect gate, port, ruin)
    DistrictOrSite,
    /// Room / Subsite (room, courtyard, cave chamber)
    RoomOrSubsite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocationStatus {
    Active,
    Deprecated,
    PendingConfirmation,
}

/// Location alias for name resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationAlias {
    pub alias: String,
    pub locale: Option<String>,
    pub normalized_alias: String,
}

/// Location spatial relation - non-tree spatial relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationSpatialRelation {
    pub relation_id: String,
    pub source_location_id: String,
    pub target_location_id: String,
    pub relation: LocationSpatialRelationKind,
    pub coverage: Option<CoverageEstimate>,
    pub confidence: FactConfidence,
    pub source: FactSource,
    pub schema_version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocationSpatialRelationKind {
    /// Two areas overlap
    Overlaps,
    /// Source crosses through target
    Crosses,
    /// Source contains part of target
    SourceContainsPartOfTarget,
    /// Source is partly within target
    SourcePartlyWithinTarget,
    /// Spatially adjacent (not implying traversable route)
    AdjacentTo,
    /// Source is within natural band of target
    WithinNaturalBand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageEstimate {
    pub percentage: f64,
    pub confidence: FactConfidence,
    pub notes: String,
}

/// Location edge - traversable route between locations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationEdge {
    pub edge_id: String,
    pub from_location_id: String,
    pub to_location_id: String,
    pub relation: LocationEdgeRelation,
    pub bidirectional: bool,
    pub distance_km: Option<DistanceEstimate>,
    pub travel_time: Option<TravelTimeEstimate>,
    pub terrain_cost: f64,
    pub safety_cost: f64,
    pub seasonal_modifiers: Vec<SeasonalRouteModifier>,
    pub allowed_modes: Vec<TravelMode>,
    pub confidence: FactConfidence,
    pub source: FactSource,
    pub schema_version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocationEdgeRelation {
    Adjacent,
    Road,
    RiverRoute,
    SeaRoute,
    MountainPass,
    ForestTrail,
    BorderCrossing,
    TeleportGate,
    ContainsShortcut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistanceEstimate {
    pub min_km: f64,
    pub max_km: f64,
    pub confidence: FactConfidence,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TravelTimeEstimate {
    pub walking: Option<DurationEstimate>,
    pub horse: Option<DurationEstimate>,
    pub carriage: Option<DurationEstimate>,
    pub boat: Option<DurationEstimate>,
    pub flying: Option<DurationEstimate>,
    pub teleport: Option<DurationEstimate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationEstimate {
    pub min_hours: f64,
    pub max_hours: f64,
    pub confidence: FactConfidence,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalRouteModifier {
    pub season: String,
    pub terrain_cost_modifier: f64,
    pub safety_cost_modifier: f64,
    pub available: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TravelMode {
    Walking,
    Horse,
    Carriage,
    Boat,
    Flying,
    Teleport,
}

/// Polity template for display labels and validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationPolityTemplate {
    pub polity_id: String,
    /// canonical_level -> display label mapping
    pub level_labels: serde_json::Value,
    /// Allowed parent-child level pairs
    pub allowed_parent_child: Vec<(LocationLevel, LocationLevel)>,
    pub override_rules: serde_json::Value,
    pub updated_at: DateTime<Utc>,
}

/// Location ambiguity result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationAmbiguity {
    pub raw_name: String,
    pub candidate_location_ids: Vec<String>,
    pub reason: String,
}

/// Route hint for location context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteHint {
    pub from_location_id: String,
    pub to_location_id: String,
    pub route_summary: String,
    pub travel_mode: TravelMode,
    pub distance_km: Option<String>,
    pub travel_time: Option<String>,
    pub risk_tags: Vec<String>,
    pub confidence: FactConfidence,
}

/// Proximity hint (low confidence)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProximityHint {
    pub location_id: String,
    pub relation: String,
    pub confidence: FactConfidence,
    pub notes: String,
}
