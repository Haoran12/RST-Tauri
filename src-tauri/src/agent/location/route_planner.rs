//! Route planner
//!
//! Calculates routes and travel times using LocationEdge graph.

use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;
use sqlx::SqlitePool;

use crate::agent::models::{FactConfidence, TravelMode};

/// Route planner - calculates routes between locations
pub struct RoutePlanner {
    pool: SqlitePool,
}

impl RoutePlanner {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Plan a route between two locations
    pub async fn plan_route(
        &self,
        from_location_id: &str,
        to_location_id: &str,
        travel_mode: TravelMode,
    ) -> Result<RouteResult, String> {
        // Step 1: Check if locations exist
        let from_exists = self.location_exists(from_location_id).await?;
        let to_exists = self.location_exists(to_location_id).await?;

        if !from_exists || !to_exists {
            return Ok(RouteResult {
                from_location_id: from_location_id.to_string(),
                to_location_id: to_location_id.to_string(),
                path: Vec::new(),
                total_distance_km: None,
                total_time: None,
                risk_tags: Vec::new(),
                confidence: FactConfidence::Low,
                unreachable: true,
                reason: if !from_exists {
                    "Origin location not found"
                } else {
                    "Destination location not found"
                }
                .to_string(),
            });
        }

        // Step 2: Get all edges and build adjacency graph
        let edges = self.get_all_edges().await?;
        let graph = self.build_graph(&edges, &travel_mode);

        // Step 3: Run Dijkstra's algorithm
        let path = self.dijkstra(&graph, from_location_id, to_location_id)?;

        if path.is_empty() {
            return Ok(RouteResult {
                from_location_id: from_location_id.to_string(),
                to_location_id: to_location_id.to_string(),
                path: Vec::new(),
                total_distance_km: None,
                total_time: None,
                risk_tags: Vec::new(),
                confidence: FactConfidence::Low,
                unreachable: true,
                reason: "No route found between locations".to_string(),
            });
        }

        // Step 4: Build route segments and aggregate metrics
        let segments = self
            .build_route_segments(&path, &edges, &travel_mode)
            .await?;
        let (total_distance, total_time, risk_tags, confidence) =
            self.aggregate_route_metrics(&segments);

        Ok(RouteResult {
            from_location_id: from_location_id.to_string(),
            to_location_id: to_location_id.to_string(),
            path: segments,
            total_distance_km: total_distance,
            total_time,
            risk_tags,
            confidence,
            unreachable: false,
            reason: String::new(),
        })
    }

    /// Get adjacent locations
    pub async fn get_adjacent(&self, location_id: &str) -> Result<Vec<AdjacentLocation>, String> {
        let rows = sqlx::query_as::<_, AdjacentRow>(
            r#"
            SELECT
                e.edge_id,
                CASE
                    WHEN e.from_location_id = ? THEN e.to_location_id
                    ELSE e.from_location_id
                END as other_location_id,
                ln.name,
                e.relation,
                e.bidirectional
            FROM location_edges e
            INNER JOIN location_nodes ln
                ON (CASE WHEN e.from_location_id = ? THEN e.to_location_id ELSE e.from_location_id END) = ln.location_id
            WHERE (e.from_location_id = ? OR e.to_location_id = ?)
              AND (e.bidirectional = 1 OR e.from_location_id = ?)
            "#,
        )
        .bind(location_id)
        .bind(location_id)
        .bind(location_id)
        .bind(location_id)
        .bind(location_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to get adjacent locations: {}", e))?;

        Ok(rows
            .into_iter()
            .map(|r| AdjacentLocation {
                location_id: r.other_location_id,
                name: r.name,
                edge_type: r.relation,
                bidirectional: r.bidirectional != 0,
            })
            .collect())
    }

    /// Get proximity hints (low confidence)
    pub async fn get_proximity_hints(
        &self,
        location_id: &str,
    ) -> Result<Vec<ProximityHint>, String> {
        let mut hints = Vec::new();

        // Hint 1: Same parent region
        let same_parent = sqlx::query_as::<_, ProximityRow>(
            r#"
            SELECT sibling.location_id, sibling.name, 'same_parent_region' as relation
            FROM location_nodes current
            INNER JOIN location_nodes sibling
                ON sibling.parent_id = current.parent_id
                AND sibling.location_id != current.location_id
            WHERE current.location_id = ?
            LIMIT 5
            "#,
        )
        .bind(location_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to get same parent hints: {}", e))?;

        for row in same_parent {
            hints.push(ProximityHint {
                location_id: row.location_id,
                name: row.name,
                relation: row.relation,
                confidence: FactConfidence::Low,
                notes: "Same parent region, actual distance unknown".to_string(),
            });
        }

        // Hint 2: Same level (siblings at same canonical_level)
        let same_level = sqlx::query_as::<_, ProximityRow>(
            r#"
            SELECT sibling.location_id, sibling.name, 'same_level' as relation
            FROM location_nodes current
            INNER JOIN location_nodes sibling
                ON sibling.canonical_level = current.canonical_level
                AND sibling.location_id != current.location_id
                AND sibling.parent_id != current.parent_id
            WHERE current.location_id = ?
            LIMIT 5
            "#,
        )
        .bind(location_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to get same level hints: {}", e))?;

        for row in same_level {
            hints.push(ProximityHint {
                location_id: row.location_id,
                name: row.name,
                relation: row.relation,
                confidence: FactConfidence::Low,
                notes: "Same level, actual proximity unknown".to_string(),
            });
        }

        Ok(hints)
    }

    /// Check if a location exists
    async fn location_exists(&self, location_id: &str) -> Result<bool, String> {
        let row = sqlx::query_as::<_, CountRow>(
            "SELECT COUNT(*) as count FROM location_nodes WHERE location_id = ?",
        )
        .bind(location_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to check location existence: {}", e))?;

        Ok(row.count > 0)
    }

    /// Get all edges from database
    async fn get_all_edges(&self) -> Result<Vec<EdgeData>, String> {
        let rows = sqlx::query_as::<_, EdgeRow>(
            r#"
            SELECT edge_id, from_location_id, to_location_id, relation,
                   bidirectional, distance_km, travel_time, terrain_cost,
                   safety_cost, seasonal_modifiers, allowed_modes, confidence
            FROM location_edges
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to get edges: {}", e))?;

        let mut edges = Vec::new();
        for row in rows {
            let distance_km: Option<DistanceEstimate> = row
                .distance_km
                .as_ref()
                .and_then(|d| serde_json::from_str(d).ok());
            let travel_time: Option<TravelTimeEstimate> = row
                .travel_time
                .as_ref()
                .and_then(|t| serde_json::from_str(t).ok());
            let allowed_modes: Vec<String> = row
                .allowed_modes
                .as_ref()
                .and_then(|m| serde_json::from_str(m).ok())
                .unwrap_or_default();
            let seasonal_modifiers: Vec<SeasonalModifier> = row
                .seasonal_modifiers
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();

            edges.push(EdgeData {
                edge_id: row.edge_id,
                from_location_id: row.from_location_id,
                to_location_id: row.to_location_id,
                relation: row.relation,
                bidirectional: row.bidirectional != 0,
                distance_km,
                travel_time,
                terrain_cost: row.terrain_cost,
                safety_cost: row.safety_cost,
                seasonal_modifiers,
                allowed_modes,
                confidence: row.confidence,
            });
        }

        Ok(edges)
    }

    /// Build adjacency graph from edges
    fn build_graph(
        &self,
        edges: &[EdgeData],
        travel_mode: &TravelMode,
    ) -> std::collections::HashMap<String, Vec<(String, f64, String)>> {
        use std::collections::HashMap;

        let mut graph: HashMap<String, Vec<(String, f64, String)>> = HashMap::new();
        let mode_str = travel_mode_to_str(travel_mode);

        for edge in edges {
            // Check if this mode is allowed
            if !edge.allowed_modes.is_empty() && !edge.allowed_modes.contains(&mode_str) {
                continue;
            }

            // Calculate cost (terrain_cost + safety_cost)
            let cost = edge.terrain_cost + edge.safety_cost;

            // Add forward edge
            graph
                .entry(edge.from_location_id.clone())
                .or_insert_with(Vec::new)
                .push((edge.to_location_id.clone(), cost, edge.edge_id.clone()));

            // Add reverse edge if bidirectional
            if edge.bidirectional {
                graph
                    .entry(edge.to_location_id.clone())
                    .or_insert_with(Vec::new)
                    .push((edge.from_location_id.clone(), cost, edge.edge_id.clone()));
            }
        }

        graph
    }

    /// Dijkstra's algorithm for shortest path
    fn dijkstra(
        &self,
        graph: &std::collections::HashMap<String, Vec<(String, f64, String)>>,
        from: &str,
        to: &str,
    ) -> Result<Vec<(String, String)>, String> {
        use std::cmp::Ordering;
        use std::collections::{BinaryHeap, HashMap};

        #[derive(Clone, Eq, PartialEq)]
        struct State {
            cost: u64,
            location: String,
            path: Vec<(String, String)>,
        }

        impl Ord for State {
            fn cmp(&self, other: &Self) -> Ordering {
                other.cost.cmp(&self.cost)
            }
        }

        impl PartialOrd for State {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        let mut dist: HashMap<String, u64> = HashMap::new();
        let mut heap = BinaryHeap::new();

        heap.push(State {
            cost: 0,
            location: from.to_string(),
            path: Vec::new(),
        });

        while let Some(State {
            cost,
            location,
            path,
        }) = heap.pop()
        {
            if location == to {
                return Ok(path);
            }

            if let Some(d) = dist.get(&location) {
                if cost > *d {
                    continue;
                }
            }

            dist.insert(location.clone(), cost);

            if let Some(neighbors) = graph.get(&location) {
                for (next, edge_cost, edge_id) in neighbors {
                    let next_cost = cost + (*edge_cost * 1000.0) as u64;
                    let mut next_path = path.clone();
                    next_path.push((location.clone(), edge_id.clone()));

                    if dist.get(next).map_or(true, |d| next_cost < *d) {
                        heap.push(State {
                            cost: next_cost,
                            location: next.clone(),
                            path: next_path,
                        });
                    }
                }
            }
        }

        Ok(Vec::new())
    }

    /// Build route segments from path
    async fn build_route_segments(
        &self,
        path: &[(String, String)],
        edges: &[EdgeData],
        travel_mode: &TravelMode,
    ) -> Result<Vec<RouteSegment>, String> {
        let mut segments = Vec::new();

        for (from_loc, edge_id) in path {
            // Find the edge
            let edge = edges.iter().find(|e| &e.edge_id == edge_id);
            if let Some(edge) = edge {
                let to_loc = if edge.from_location_id == *from_loc {
                    &edge.to_location_id
                } else {
                    &edge.from_location_id
                };

                let distance = edge
                    .distance_km
                    .as_ref()
                    .and_then(|d| Some((d.min_km + d.max_km) / 2.0));

                segments.push(RouteSegment {
                    from_location_id: from_loc.clone(),
                    to_location_id: to_loc.clone(),
                    edge_type: edge.relation.clone(),
                    distance_km: distance,
                    travel_mode: *travel_mode,
                });
            }
        }

        Ok(segments)
    }

    /// Aggregate route metrics
    fn aggregate_route_metrics(
        &self,
        segments: &[RouteSegment],
    ) -> (
        Option<DistanceRange>,
        Option<TravelTimeRange>,
        Vec<String>,
        FactConfidence,
    ) {
        if segments.is_empty() {
            return (None, None, Vec::new(), FactConfidence::Low);
        }

        let mut total_min_km = 0.0;
        let mut total_max_km = 0.0;
        let mut total_min_hours = 0.0;
        let mut total_max_hours = 0.0;
        let mut risk_tags = Vec::new();

        for segment in segments {
            if let Some(km) = segment.distance_km {
                total_min_km += km * 0.9;
                total_max_km += km * 1.1;
            }
            // Estimate travel time based on mode
            let speed = match segment.travel_mode {
                TravelMode::Walking => 5.0, // km/h
                TravelMode::Horse => 40.0,
                TravelMode::Carriage => 25.0,
                TravelMode::Boat => 15.0,
                TravelMode::Flying => 100.0,
                TravelMode::Teleport => f64::INFINITY,
            };
            if let Some(km) = segment.distance_km {
                if speed.is_finite() {
                    total_min_hours += km * 0.9 / speed;
                    total_max_hours += km * 1.1 / speed;
                }
            }
        }

        let distance = if total_min_km > 0.0 {
            Some(DistanceRange {
                min_km: total_min_km,
                max_km: total_max_km,
                confidence: FactConfidence::Medium,
            })
        } else {
            None
        };

        let time = if total_min_hours > 0.0 {
            Some(TravelTimeRange {
                min_hours: total_min_hours,
                max_hours: total_max_hours,
                confidence: FactConfidence::Medium,
            })
        } else {
            None
        };

        let confidence = if segments.len() > 3 {
            FactConfidence::Medium
        } else {
            FactConfidence::Low
        };

        (distance, time, risk_tags, confidence)
    }
}

/// Route calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResult {
    pub from_location_id: String,
    pub to_location_id: String,
    pub path: Vec<RouteSegment>,
    pub total_distance_km: Option<DistanceRange>,
    pub total_time: Option<TravelTimeRange>,
    pub risk_tags: Vec<String>,
    pub confidence: FactConfidence,
    pub unreachable: bool,
    pub reason: String,
}

/// Single segment of a route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteSegment {
    pub from_location_id: String,
    pub to_location_id: String,
    pub edge_type: String,
    pub distance_km: Option<f64>,
    pub travel_mode: TravelMode,
}

/// Distance range estimate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistanceRange {
    pub min_km: f64,
    pub max_km: f64,
    pub confidence: FactConfidence,
}

/// Travel time range estimate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TravelTimeRange {
    pub min_hours: f64,
    pub max_hours: f64,
    pub confidence: FactConfidence,
}

/// Adjacent location info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjacentLocation {
    pub location_id: String,
    pub name: String,
    pub edge_type: String,
    pub bidirectional: bool,
}

/// Proximity hint (low confidence)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProximityHint {
    pub location_id: String,
    pub name: String,
    pub relation: String,
    pub confidence: FactConfidence,
    pub notes: String,
}

// ===== Database row types =====

#[derive(FromRow)]
struct CountRow {
    count: i64,
}

#[derive(FromRow)]
struct EdgeRow {
    edge_id: String,
    from_location_id: String,
    to_location_id: String,
    relation: String,
    bidirectional: i32,
    distance_km: Option<String>,
    travel_time: Option<String>,
    terrain_cost: f64,
    safety_cost: f64,
    seasonal_modifiers: Option<String>,
    allowed_modes: Option<String>,
    confidence: String,
}

#[derive(FromRow)]
struct AdjacentRow {
    other_location_id: String,
    name: String,
    relation: String,
    bidirectional: i32,
}

#[derive(FromRow)]
struct ProximityRow {
    location_id: String,
    name: String,
    relation: String,
}

// ===== Internal types =====

struct EdgeData {
    edge_id: String,
    from_location_id: String,
    to_location_id: String,
    relation: String,
    bidirectional: bool,
    distance_km: Option<DistanceEstimate>,
    travel_time: Option<TravelTimeEstimate>,
    terrain_cost: f64,
    safety_cost: f64,
    seasonal_modifiers: Vec<SeasonalModifier>,
    allowed_modes: Vec<String>,
    confidence: String,
}

#[derive(Deserialize)]
struct DistanceEstimate {
    min_km: f64,
    max_km: f64,
}

#[derive(Deserialize)]
struct TravelTimeEstimate {
    walking: Option<DurationEstimate>,
    horse: Option<DurationEstimate>,
    carriage: Option<DurationEstimate>,
    boat: Option<DurationEstimate>,
    flying: Option<DurationEstimate>,
}

#[derive(Deserialize)]
struct DurationEstimate {
    min_hours: f64,
    max_hours: f64,
}

#[derive(Deserialize)]
struct SeasonalModifier {
    season: String,
    available: bool,
}

// ===== Helper functions =====

fn travel_mode_to_str(mode: &TravelMode) -> String {
    match mode {
        TravelMode::Walking => "walking",
        TravelMode::Horse => "horse",
        TravelMode::Carriage => "carriage",
        TravelMode::Boat => "boat",
        TravelMode::Flying => "flying",
        TravelMode::Teleport => "teleport",
    }
    .to_string()
}
