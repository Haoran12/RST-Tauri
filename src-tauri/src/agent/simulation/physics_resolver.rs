//! Physics resolver
//!
//! Handles physical interactions, movement, and environmental physics.
//! Provides the physics skeleton for action validation.

use crate::agent::models::{
    AttributeKind, BodyConstraints, CharacterRecord, EffectiveAttributeProfile,
    EnvironmentalStrain, PhysicalConditions, SceneModel, SizeClass, SurfaceImpactTier,
    SurfaceState, WindImpactTier, WindState,
};

/// Physics resolver - handles physical calculations
pub struct PhysicsResolver;

impl PhysicsResolver {
    /// Calculate movement cost for a distance
    pub fn calculate_movement_cost(
        distance_m: f64,
        body: &BodyConstraints,
        environmental: &EnvironmentalStrain,
    ) -> MovementCost {
        // Base cost from distance and mobility
        let base_cost = distance_m * (1.0 / body.mobility.max(0.1));

        // Environmental penalty
        let env_penalty = distance_m * environmental.movement_penalty;

        // Balance penalty affects precision movement
        let balance_penalty = if body.balance < 0.5 {
            distance_m * 0.2 * (1.0 - body.balance)
        } else {
            0.0
        };

        let total_fatigue_cost = (base_cost + env_penalty + balance_penalty) * 0.001;

        MovementCost {
            distance_m,
            fatigue_cost: total_fatigue_cost.min(0.3),
            time_cost_seconds: distance_m / (2.0 * body.mobility.max(0.1)), // Base walking speed ~2 m/s
            requires_balance_check: body.balance < 0.7
                || environmental.surface_tier != SurfaceImpactTier::Stable,
            movement_kind: Self::determine_movement_kind(body, environmental),
        }
    }

    /// Determine movement kind based on constraints
    fn determine_movement_kind(
        body: &BodyConstraints,
        environmental: &EnvironmentalStrain,
    ) -> MovementKind {
        if body.mobility < 0.3 {
            MovementKind::Immobile
        } else if environmental.surface_tier == SurfaceImpactTier::Treacherous {
            MovementKind::Careful
        } else if body.balance < 0.5 {
            MovementKind::Unsteady
        } else if environmental.movement_penalty > 0.5 {
            MovementKind::Difficult
        } else {
            MovementKind::Normal
        }
    }

    /// Calculate physical interaction outcome
    pub fn calculate_physical_interaction(
        actor: &CharacterRecord,
        actor_attrs: &EffectiveAttributeProfile,
        target: &CharacterRecord,
        target_attrs: &EffectiveAttributeProfile,
        interaction_kind: PhysicalInteractionKind,
    ) -> PhysicalInteractionResult {
        let actor_physical = actor_attrs
            .values
            .get(&AttributeKind::Physical)
            .copied()
            .unwrap_or(100.0);
        let target_physical = target_attrs
            .values
            .get(&AttributeKind::Physical)
            .copied()
            .unwrap_or(100.0);

        // Size factor
        let actor_size_factor = Self::size_factor(&actor.baseline_body_profile.size_class);
        let target_size_factor = Self::size_factor(&target.baseline_body_profile.size_class);

        // Calculate base outcome
        let base_power = actor_physical * actor_size_factor;
        let base_resistance = target_physical * target_size_factor;

        let (success_probability, force_ratio) = match interaction_kind {
            PhysicalInteractionKind::Grapple => {
                let ratio = base_power / base_resistance.max(1.0);
                let prob = (0.5 + (ratio - 1.0) * 0.3).clamp(0.1, 0.9);
                (prob, ratio)
            }
            PhysicalInteractionKind::Shove => {
                let ratio = base_power / base_resistance.max(1.0);
                let prob = (0.4 + (ratio - 1.0) * 0.35).clamp(0.1, 0.9);
                (prob, ratio)
            }
            PhysicalInteractionKind::Lift => {
                let ratio = base_power / (base_resistance * 1.5).max(1.0); // Lifting is harder
                let prob = (0.3 + (ratio - 1.0) * 0.4).clamp(0.0, 0.8);
                (prob, ratio)
            }
            PhysicalInteractionKind::Pin => {
                let ratio = base_power / (base_resistance * 1.3).max(1.0);
                let prob = (0.35 + (ratio - 1.0) * 0.35).clamp(0.05, 0.85);
                (prob, ratio)
            }
        };

        PhysicalInteractionResult {
            interaction_kind,
            actor_id: actor.character_id.clone(),
            target_id: target.character_id.clone(),
            success_probability,
            force_ratio,
            actor_fatigue_cost: 0.05 * force_ratio,
            target_impact: if force_ratio > 1.5 {
                ImpactLevel::Significant
            } else if force_ratio > 1.0 {
                ImpactLevel::Moderate
            } else {
                ImpactLevel::Minor
            },
        }
    }

    /// Get size factor for physical calculations
    fn size_factor(size: &SizeClass) -> f64 {
        match size {
            SizeClass::Tiny => 0.25,
            SizeClass::Small => 0.5,
            SizeClass::Humanoid => 1.0,
            SizeClass::Large => 1.5,
            SizeClass::Huge => 2.5,
            SizeClass::Kaiju => 5.0,
        }
    }

    /// Calculate fall damage
    pub fn calculate_fall_damage(
        height_m: f64,
        body: &BodyConstraints,
        surface: &SurfaceState,
    ) -> FallDamageResult {
        // Terminal velocity consideration (simplified)
        let effective_height = height_m.min(50.0); // Cap at terminal velocity

        // Base damage from height
        let base_damage = effective_height * 0.1;

        // Surface modifier
        let surface_modifier = if surface.slipperiness > 0.5 {
            0.8 // Slippery surfaces absorb some impact
        } else {
            1.0
        };

        // Agility helps reduce fall damage
        let agility_factor = 1.0 - (body.fine_control * 0.3).min(0.5);

        let total_damage = base_damage * surface_modifier * agility_factor;

        FallDamageResult {
            height_m,
            damage_amount: total_damage,
            injury_probability: (total_damage / 0.5).min(1.0),
            requires_check: height_m > 2.0,
        }
    }

    /// Calculate throwing range and accuracy
    pub fn calculate_throw(
        actor_attrs: &EffectiveAttributeProfile,
        object_weight_kg: f64,
        target_distance_m: f64,
        wind: &WindState,
    ) -> ThrowResult {
        let physical = actor_attrs
            .values
            .get(&AttributeKind::Physical)
            .copied()
            .unwrap_or(100.0);
        let agility = actor_attrs
            .values
            .get(&AttributeKind::Agility)
            .copied()
            .unwrap_or(100.0);

        // Base throwing range (simplified physics)
        let base_range_m = (physical / 10.0).sqrt() * 10.0 / (object_weight_kg.max(0.1)).sqrt();

        // Wind effect
        let wind_penalty = match Self::wind_speed_to_tier(wind.speed_ms) {
            WindImpactTier::Calm | WindImpactTier::Breeze => 0.0,
            WindImpactTier::Moderate => 0.1,
            WindImpactTier::Strong => 0.2,
            WindImpactTier::Gale => 0.35,
            WindImpactTier::Storm | WindImpactTier::Hurricane => 0.5,
        };

        let effective_range = base_range_m * (1.0 - wind_penalty);

        // Accuracy decreases with distance
        let accuracy = if target_distance_m <= effective_range * 0.5 {
            0.9
        } else if target_distance_m <= effective_range {
            0.7 - (target_distance_m / effective_range - 0.5) * 0.4
        } else {
            0.3 * (effective_range / target_distance_m)
        };

        // Agility affects accuracy
        let final_accuracy = accuracy * (0.5 + agility / 200.0).min(1.0);

        ThrowResult {
            max_range_m: effective_range,
            target_distance_m,
            accuracy: final_accuracy,
            can_reach: target_distance_m <= effective_range,
            wind_effect: wind_penalty,
        }
    }

    /// Convert wind speed to tier
    fn wind_speed_to_tier(speed_ms: f64) -> WindImpactTier {
        if speed_ms < 0.5 {
            WindImpactTier::Calm
        } else if speed_ms < 5.0 {
            WindImpactTier::Breeze
        } else if speed_ms < 10.0 {
            WindImpactTier::Moderate
        } else if speed_ms < 17.0 {
            WindImpactTier::Strong
        } else if speed_ms < 25.0 {
            WindImpactTier::Gale
        } else if speed_ms < 32.0 {
            WindImpactTier::Storm
        } else {
            WindImpactTier::Hurricane
        }
    }
}

/// Movement cost result
#[derive(Debug, Clone)]
pub struct MovementCost {
    pub distance_m: f64,
    pub fatigue_cost: f64,
    pub time_cost_seconds: f64,
    pub requires_balance_check: bool,
    pub movement_kind: MovementKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementKind {
    Normal,
    Careful,
    Unsteady,
    Difficult,
    Immobile,
}

/// Physical interaction kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalInteractionKind {
    Grapple,
    Shove,
    Lift,
    Pin,
}

/// Physical interaction result
#[derive(Debug, Clone)]
pub struct PhysicalInteractionResult {
    pub interaction_kind: PhysicalInteractionKind,
    pub actor_id: String,
    pub target_id: String,
    pub success_probability: f64,
    pub force_ratio: f64,
    pub actor_fatigue_cost: f64,
    pub target_impact: ImpactLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImpactLevel {
    Minor,
    Moderate,
    Significant,
}

/// Fall damage result
#[derive(Debug, Clone)]
pub struct FallDamageResult {
    pub height_m: f64,
    pub damage_amount: f64,
    pub injury_probability: f64,
    pub requires_check: bool,
}

/// Throw result
#[derive(Debug, Clone)]
pub struct ThrowResult {
    pub max_range_m: f64,
    pub target_distance_m: f64,
    pub accuracy: f64,
    pub can_reach: bool,
    pub wind_effect: f64,
}
