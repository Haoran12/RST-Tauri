//! Effect validator
//!
//! Validates skill effects against contracts and hard constraints.
//! Ensures LLM-generated effects stay within declared bounds.

use crate::agent::knowledge::access_resolver::CharacterScopeMembership;
use crate::agent::models::{
    AccessPolicy, AccessScope, AttributeKind, CharacterRecord, CostProfile, EffectIntensityTier,
    EffectiveAttributeProfile, KnowledgeEntry, SceneModel, SkillEffect, SkillEffectContract,
    SkillEffectKind,
};

/// Effect validator - validates skill effects and state updates
pub struct EffectValidator;

impl EffectValidator {
    /// Validate a skill effect against its contract
    pub fn validate_effect(
        effect: &SkillEffect,
        contract: &SkillEffectContract,
        actor: &CharacterRecord,
        actor_attrs: &EffectiveAttributeProfile,
        scene: &SceneModel,
    ) -> EffectValidationResult {
        let mut violations = Vec::new();
        let mut blocked_effects = Vec::new();
        let mut soft_effects = Vec::new();

        // Check intensity tier
        if effect.intensity_tier > contract.max_intensity_tier {
            violations.push(ContractViolation {
                violation_kind: ContractViolationKind::IntensityExceeded,
                message: format!(
                    "效果强度 {:?} 超过契约上限 {:?}",
                    effect.intensity_tier, contract.max_intensity_tier
                ),
                severity: ViolationSeverity::Hard,
            });
            blocked_effects.push(effect.effect_id.clone());
        }

        // Check target kind
        if !contract
            .allowed_target_kinds
            .contains(&contract.target_kind)
        {
            violations.push(ContractViolation {
                violation_kind: ContractViolationKind::InvalidTarget,
                message: format!("目标类型 {:?} 不在契约允许范围内", contract.target_kind),
                severity: ViolationSeverity::Hard,
            });
        }

        // Check effect kind against contract flags
        let effect_allowed = Self::check_effect_kind_allowed(&effect.effect_kind, contract);
        if !effect_allowed {
            violations.push(ContractViolation {
                violation_kind: ContractViolationKind::EffectNotAllowed,
                message: format!("效果类型 {:?} 不在契约允许范围内", effect.effect_kind),
                severity: ViolationSeverity::Hard,
            });
            blocked_effects.push(effect.effect_id.clone());
        }

        // Check state domain
        if !contract
            .allowed_state_domains
            .contains(&effect.target_domain)
        {
            violations.push(ContractViolation {
                violation_kind: ContractViolationKind::InvalidDomain,
                message: format!("目标域 {} 不在契约允许范围内", effect.target_domain),
                severity: ViolationSeverity::Soft,
            });
            soft_effects.push(effect.effect_id.clone());
        }

        // Check resource availability
        Self::check_resource_constraints(actor, actor_attrs, effect, &mut violations);

        // Check position constraints
        Self::check_position_constraints(actor, scene, contract, effect, &mut violations);

        EffectValidationResult {
            effect_id: effect.effect_id.clone(),
            is_valid: violations
                .iter()
                .all(|v| v.severity != ViolationSeverity::Hard),
            violations,
            blocked_effects,
            soft_effects,
        }
    }

    /// Check if effect kind is allowed by contract
    fn check_effect_kind_allowed(
        effect_kind: &SkillEffectKind,
        contract: &SkillEffectContract,
    ) -> bool {
        match effect_kind {
            SkillEffectKind::Damage | SkillEffectKind::StatusApply => contract.allows_injury,
            SkillEffectKind::Movement => contract.allows_position_change,
            SkillEffectKind::KnowledgeReveal => contract.allows_knowledge_reveal,
            SkillEffectKind::Healing
            | SkillEffectKind::Buff
            | SkillEffectKind::Debuff
            | SkillEffectKind::StatusRemove
            | SkillEffectKind::Summon
            | SkillEffectKind::TerrainChange
            | SkillEffectKind::ManaFieldChange
            | SkillEffectKind::Social
            | SkillEffectKind::Utility => true,
        }
    }

    /// Check resource constraints
    fn check_resource_constraints(
        actor: &CharacterRecord,
        actor_attrs: &EffectiveAttributeProfile,
        effect: &SkillEffect,
        violations: &mut Vec<ContractViolation>,
    ) {
        // Check mana reserve
        let mana_power = actor_attrs
            .values
            .get(&AttributeKind::ManaPower)
            .copied()
            .unwrap_or(0.0);

        if effect.effect_kind == SkillEffectKind::Damage {
            // Damage effects require sufficient mana
            if mana_power < 100.0 && effect.intensity_tier >= EffectIntensityTier::Moderate {
                violations.push(ContractViolation {
                    violation_kind: ContractViolationKind::InsufficientResource,
                    message: "灵力不足以施展中等强度伤害效果".to_string(),
                    severity: ViolationSeverity::Soft,
                });
            }
        }

        // Check fatigue
        if actor.temporary_state.fatigue > 0.7 {
            violations.push(ContractViolation {
                violation_kind: ContractViolationKind::PhysicalConstraint,
                message: "高度疲惫状态限制效果强度".to_string(),
                severity: ViolationSeverity::Soft,
            });
        }
    }

    /// Check position constraints
    fn check_position_constraints(
        actor: &CharacterRecord,
        scene: &SceneModel,
        contract: &SkillEffectContract,
        effect: &SkillEffect,
        violations: &mut Vec<ContractViolation>,
    ) {
        let actor_entity = scene
            .entities
            .iter()
            .find(|e| e.entity_id == actor.character_id);
        let Some(actor_entity) = actor_entity else {
            violations.push(ContractViolation {
                violation_kind: ContractViolationKind::PositionConstraint,
                message: "角色不在当前场景中".to_string(),
                severity: ViolationSeverity::Hard,
            });
            return;
        };

        if effect.applies_to.is_empty() {
            violations.push(ContractViolation {
                violation_kind: ContractViolationKind::InvalidTarget,
                message: "效果缺少目标".to_string(),
                severity: ViolationSeverity::Hard,
            });
            return;
        }

        if let Some(range_m) = contract.range_m {
            for target_id in &effect.applies_to {
                let Some(target_entity) = scene.entities.iter().find(|e| &e.entity_id == target_id)
                else {
                    violations.push(ContractViolation {
                        violation_kind: ContractViolationKind::InvalidTarget,
                        message: format!("目标 {} 不在当前场景中", target_id),
                        severity: ViolationSeverity::Hard,
                    });
                    continue;
                };

                let dz = actor_entity.position.z.unwrap_or(0.0)
                    - target_entity.position.z.unwrap_or(0.0);
                let distance = ((actor_entity.position.x - target_entity.position.x).powi(2)
                    + (actor_entity.position.y - target_entity.position.y).powi(2)
                    + dz.powi(2))
                .sqrt();
                if distance > range_m {
                    violations.push(ContractViolation {
                        violation_kind: ContractViolationKind::PositionConstraint,
                        message: format!(
                            "目标 {} 距离 {:.1}m，超过技能范围 {:.1}m",
                            target_id, distance, range_m
                        ),
                        severity: ViolationSeverity::Hard,
                    });
                }
            }
        }
    }

    /// Validate cost profile against character state
    pub fn validate_cost(
        cost: &CostProfile,
        actor: &CharacterRecord,
        actor_attrs: &EffectiveAttributeProfile,
    ) -> CostValidationResult {
        Self::validate_cost_for_ability(cost, actor, actor_attrs, None)
    }

    /// Validate a cost profile and, when available, the currently activated ability.
    pub fn validate_cost_for_ability(
        cost: &CostProfile,
        actor: &CharacterRecord,
        actor_attrs: &EffectiveAttributeProfile,
        ability_id: Option<&str>,
    ) -> CostValidationResult {
        let mut violations = Vec::new();
        let mut can_pay = true;

        // Check mana reserve
        if let Some(mana_delta) = cost.mana_reserve_delta {
            if mana_delta < 0.0 {
                let current_reserve = actor.temporary_state.mana_reserve_current.unwrap_or(
                    actor_attrs
                        .values
                        .get(&AttributeKind::ManaPower)
                        .copied()
                        .unwrap_or(0.0),
                );
                if current_reserve < mana_delta.abs() {
                    violations.push("灵力储备不足以支付消耗".to_string());
                    can_pay = false;
                }
            }
        }

        // Check fatigue capacity
        if let Some(fatigue_delta) = cost.fatigue_delta {
            if actor.temporary_state.fatigue + fatigue_delta > 1.0 {
                violations.push("疲惫度将达到极限".to_string());
                can_pay = false;
            }
        }

        // Check existing cooldowns when the caller can identify the ability.
        if let Some(ability_id) = ability_id {
            if actor
                .temporary_state
                .cooldowns
                .iter()
                .any(|cooldown| cooldown.ability_id == ability_id && cooldown.remaining_turns > 0)
            {
                violations.push(format!("技能仍在冷却中: {}", ability_id));
                can_pay = false;
            }
        }

        // Check required conditions
        for condition in &cost.required_conditions {
            let has_condition = actor
                .temporary_state
                .active_conditions
                .iter()
                .any(|c| c.condition_kind == *condition || c.condition_id == *condition);
            if !has_condition {
                violations.push(format!("缺少必要条件: {}", condition));
                can_pay = false;
            }
        }

        // Check material components
        for material in &cost.material_refs {
            violations.push(format!("材料检查待实现: {}", material));
            can_pay = false;
        }

        CostValidationResult {
            can_pay,
            violations,
            mana_cost: cost.mana_reserve_delta,
            fatigue_cost: cost.fatigue_delta,
            cooldown_turns: cost.cooldown_turns,
        }
    }

    /// Validate knowledge access for reveal effects
    pub fn validate_knowledge_access(
        knowledge: &KnowledgeEntry,
        actor_id: &str,
        target_ids: &[String],
    ) -> KnowledgeAccessValidation {
        Self::validate_knowledge_access_with_memberships(knowledge, actor_id, target_ids, &[])
    }

    /// Validate knowledge access with explicit target scope memberships.
    ///
    /// Callers that need region/faction/realm/role/bloodline checks must provide
    /// the same membership facts used by `KnowledgeAccessResolver`.
    pub fn validate_knowledge_access_with_memberships(
        knowledge: &KnowledgeEntry,
        actor_id: &str,
        target_ids: &[String],
        target_memberships: &[CharacterScopeMembership],
    ) -> KnowledgeAccessValidation {
        let mut violations = Vec::new();
        let mut allowed_targets = Vec::new();
        let mut blocked_targets = Vec::new();

        // Check GodOnly
        let has_god_only = knowledge
            .access_policy
            .scope
            .iter()
            .any(|s| matches!(s, AccessScope::GodOnly));
        if has_god_only {
            violations.push("知识为 GodOnly，无法揭示".to_string());
            blocked_targets.extend(target_ids.iter().cloned());
            return KnowledgeAccessValidation {
                knowledge_id: knowledge.knowledge_id.clone(),
                actor_id: actor_id.to_string(),
                is_valid: false,
                violations,
                allowed_targets,
                blocked_targets,
            };
        }

        // Check known_by
        for target_id in target_ids {
            if knowledge.access_policy.known_by.contains(target_id) {
                allowed_targets.push(target_id.clone());
            } else {
                // Can be revealed if scope allows
                let scope_allows = Self::check_scope_access(
                    &knowledge.access_policy,
                    target_id,
                    target_memberships,
                );
                if scope_allows {
                    allowed_targets.push(target_id.clone());
                } else {
                    blocked_targets.push(target_id.clone());
                }
            }
        }

        KnowledgeAccessValidation {
            knowledge_id: knowledge.knowledge_id.clone(),
            actor_id: actor_id.to_string(),
            is_valid: violations.is_empty() && !allowed_targets.is_empty(),
            violations,
            allowed_targets,
            blocked_targets,
        }
    }

    /// Check if scope allows access for a target
    fn check_scope_access(
        policy: &AccessPolicy,
        target_id: &str,
        target_memberships: &[CharacterScopeMembership],
    ) -> bool {
        for scope in &policy.scope {
            match scope {
                AccessScope::Public => return true,
                AccessScope::GodOnly => continue, // Already checked above
                AccessScope::Region(region_id) => {
                    if Self::target_has_scope(target_memberships, target_id, "region", region_id) {
                        return true;
                    }
                }
                AccessScope::Faction(faction_id) => {
                    if Self::target_has_scope(target_memberships, target_id, "faction", faction_id)
                    {
                        return true;
                    }
                }
                AccessScope::Realm(realm_id) => {
                    if Self::target_has_scope(target_memberships, target_id, "realm", realm_id) {
                        return true;
                    }
                }
                AccessScope::Role(role_id) => {
                    if Self::target_has_scope(target_memberships, target_id, "role", role_id) {
                        return true;
                    }
                }
                AccessScope::Bloodline(bloodline_id) => {
                    if Self::target_has_scope(
                        target_memberships,
                        target_id,
                        "bloodline",
                        bloodline_id,
                    ) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn target_has_scope(
        memberships: &[CharacterScopeMembership],
        target_id: &str,
        scope_type: &str,
        scope_value: &str,
    ) -> bool {
        memberships.iter().any(|membership| {
            membership.character_id == target_id
                && membership.scope_type == scope_type
                && membership.scope_value == scope_value
        })
    }

    /// Validate state update plan
    pub fn validate_state_update(
        update: &StateUpdatePlan,
        actor: &CharacterRecord,
        actor_attrs: &EffectiveAttributeProfile,
        scene: &SceneModel,
    ) -> StateUpdateValidation {
        let mut violations = Vec::new();
        let mut blocked_updates = Vec::new();
        let mut soft_updates = Vec::new();

        for effect in &update.effects {
            let effect_validation =
                Self::validate_effect(effect, &update.effect_contract, actor, actor_attrs, scene);

            if !effect_validation.is_valid {
                blocked_updates.push(effect.effect_id.clone());
                violations.extend(effect_validation.violations);
            } else if !effect_validation.soft_effects.is_empty() {
                soft_updates.push(effect.effect_id.clone());
            }
        }

        // Validate cost
        let cost_validation = Self::validate_cost(&update.cost, actor, actor_attrs);
        if !cost_validation.can_pay {
            violations.push(ContractViolation {
                violation_kind: ContractViolationKind::InsufficientResource,
                message: "无法支付效果消耗".to_string(),
                severity: ViolationSeverity::Hard,
            });
        }

        StateUpdateValidation {
            plan_id: update.plan_id.clone(),
            is_valid: violations
                .iter()
                .all(|v| v.severity != ViolationSeverity::Hard),
            violations,
            blocked_updates,
            soft_updates,
            cost_validation,
        }
    }
}

/// Effect validation result
#[derive(Debug, Clone)]
pub struct EffectValidationResult {
    pub effect_id: String,
    pub is_valid: bool,
    pub violations: Vec<ContractViolation>,
    pub blocked_effects: Vec<String>,
    pub soft_effects: Vec<String>,
}

/// Contract violation
#[derive(Debug, Clone)]
pub struct ContractViolation {
    pub violation_kind: ContractViolationKind,
    pub message: String,
    pub severity: ViolationSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractViolationKind {
    IntensityExceeded,
    InvalidTarget,
    EffectNotAllowed,
    InvalidDomain,
    InsufficientResource,
    PhysicalConstraint,
    PositionConstraint,
    KnowledgeConstraint,
    TemporalConstraint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationSeverity {
    Hard, // Blocks effect entirely
    Soft, // Effect proceeds with limitation
}

/// Cost validation result
#[derive(Debug, Clone)]
pub struct CostValidationResult {
    pub can_pay: bool,
    pub violations: Vec<String>,
    pub mana_cost: Option<f64>,
    pub fatigue_cost: Option<f64>,
    pub cooldown_turns: Option<u32>,
}

/// Knowledge access validation result
#[derive(Debug, Clone)]
pub struct KnowledgeAccessValidation {
    pub knowledge_id: String,
    pub actor_id: String,
    pub is_valid: bool,
    pub violations: Vec<String>,
    pub allowed_targets: Vec<String>,
    pub blocked_targets: Vec<String>,
}

/// State update plan (placeholder for actual type)
#[derive(Debug, Clone)]
pub struct StateUpdatePlan {
    pub plan_id: String,
    pub effects: Vec<SkillEffect>,
    pub effect_contract: SkillEffectContract,
    pub cost: CostProfile,
}

/// State update validation result
#[derive(Debug, Clone)]
pub struct StateUpdateValidation {
    pub plan_id: String,
    pub is_valid: bool,
    pub violations: Vec<ContractViolation>,
    pub blocked_updates: Vec<String>,
    pub soft_updates: Vec<String>,
    pub cost_validation: CostValidationResult,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::models::*;
    use chrono::Utc;
    use std::collections::HashMap;

    fn character(id: &str) -> CharacterRecord {
        CharacterRecord {
            character_id: id.to_string(),
            base_attributes: BaseAttributes {
                physical: 100.0,
                agility: 100.0,
                endurance: 100.0,
                insight: 100.0,
                mana_power: 100.0,
                soul_strength: 100.0,
            },
            baseline_body_profile: BaselineBodyProfile {
                species: "human".to_string(),
                comfort_temperature_range: (18.0, 26.0),
                mana_sense_baseline: ManaSenseBaseline {
                    acuity: 0.4,
                    overload_threshold: 1000.0,
                    attribute_bias: None,
                },
                mana_attribute_affinity: Vec::new(),
                size_class: SizeClass::Humanoid,
            },
            mana_expression_tendency: ManaExpressionTendency::Neutral,
            mana_expression_tendency_factor_override: None,
            mind_model_card_knowledge_id: format!("mind-{id}"),
            temporary_state: TemporaryCharacterState::new(),
            schema_version: "0.1".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn attrs(id: &str) -> EffectiveAttributeProfile {
        let mut values = HashMap::new();
        values.insert(AttributeKind::ManaPower, 100.0);
        EffectiveAttributeProfile {
            character_id: id.to_string(),
            values,
            tiers: HashMap::new(),
            descriptors: HashMap::new(),
        }
    }

    #[test]
    fn validates_existing_ability_cooldown_when_ability_id_is_known() {
        let mut actor = character("actor");
        actor.temporary_state.cooldowns.push(CooldownState {
            ability_id: "skill-1".to_string(),
            remaining_turns: 1,
        });
        let cost = CostProfile {
            cooldown_turns: Some(2),
            ..CostProfile::default()
        };

        let result = EffectValidator::validate_cost_for_ability(
            &cost,
            &actor,
            &attrs("actor"),
            Some("skill-1"),
        );

        assert!(!result.can_pay);
        assert!(result
            .violations
            .iter()
            .any(|violation| violation.contains("skill-1")));
    }

    #[test]
    fn scoped_knowledge_access_requires_matching_target_membership() {
        let policy = AccessPolicy {
            known_by: Vec::new(),
            scope: vec![AccessScope::Faction("sect-a".to_string())],
            conditions: Vec::new(),
        };
        let target_id = "target-1";

        assert!(!EffectValidator::check_scope_access(
            &policy,
            target_id,
            &[]
        ));

        let memberships = vec![CharacterScopeMembership {
            character_id: target_id.to_string(),
            scope_type: "faction".to_string(),
            scope_value: "sect-a".to_string(),
        }];
        assert!(EffectValidator::check_scope_access(
            &policy,
            target_id,
            &memberships
        ));
    }
}
