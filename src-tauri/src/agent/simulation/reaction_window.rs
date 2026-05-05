//! Reaction window manager
//!
//! Manages bounded reaction windows for combat and threat response.
//! Implements the ReactionWindow pattern from docs/11_agent_runtime.md §5.1.

use std::collections::HashMap;

use crate::agent::models::{
    ActivationTime, CharacterRecord, EffectiveAttributeProfile, ObjectiveRelationKind,
    ObjectiveRelationship, ObservableEventDelta, Position, ReactionEligibility,
    ReactionEligibilityReason, ReactionIntent, ReactionKind, ReactionOption, ReactionWindow,
    SceneModel, Skill, SkillKind, TargetKind,
};
use crate::agent::validation::EffectValidator as SkillEffectValidator;

#[derive(Debug, Clone)]
struct PreparedReactionSkill {
    skill: Skill,
    target_scope: Vec<String>,
}

/// Reaction window manager
pub struct ReactionWindowManager {
    /// Active reaction windows
    windows: HashMap<String, ReactionWindow>,
}

impl ReactionWindowManager {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
        }
    }

    /// Open a new reaction window
    ///
    /// This creates a new reaction window for a threat/action event and
    /// determines which characters are eligible to react.
    pub fn open_window(
        &mut self,
        scene_turn_id: &str,
        source_event_id: &str,
        source_action_id: &str,
        threat_source_id: &str,
        explicit_primary_targets: &[String],
        scene: &SceneModel,
        characters: &[CharacterRecord],
        relationships: &[ObjectiveRelationship],
        skills: &[Skill],
        effective_attrs: &HashMap<String, EffectiveAttributeProfile>,
    ) -> Result<String, String> {
        let window_id = format!("rw_{}", uuid::Uuid::new_v4());

        // Step 1: Determine primary targets
        let primary_targets =
            Self::determine_primary_targets(scene, threat_source_id, explicit_primary_targets);

        // Step 2: Determine eligible reactors
        let eligible_reactors = self.determine_eligible_reactors(
            scene_turn_id,
            scene,
            characters,
            relationships,
            skills,
            effective_attrs,
            &primary_targets,
            threat_source_id,
        )?;

        // Step 3: Build observable threat description
        let observable_threat = ObservableEventDelta {
            event_id: source_event_id.to_string(),
            scene_turn_id: scene_turn_id.to_string(),
            event_kind: "threat".to_string(),
            involved_observable_entities: primary_targets.clone(),
            observable_effects: serde_json::json!({
                "source": threat_source_id,
                "action_id": source_action_id,
            }),
            sensory_descriptors: vec!["threat_detected".to_string()],
            source_hint: None,
        };

        // Step 4: Create window
        let window = ReactionWindow {
            window_id: window_id.clone(),
            scene_turn_id: scene_turn_id.to_string(),
            source_event_id: source_event_id.to_string(),
            source_action_id: source_action_id.to_string(),
            threat_source_id: threat_source_id.to_string(),
            primary_targets,
            observable_threat,
            eligible_reactors,
            max_reaction_depth: 1,
            no_reaction_to_reaction: true,
            one_reaction_per_character: true,
        };

        self.windows.insert(window_id.clone(), window);
        Ok(window_id)
    }

    /// Determine primary targets of a threat
    fn determine_primary_targets(
        scene: &SceneModel,
        threat_source_id: &str,
        explicit_primary_targets: &[String],
    ) -> Vec<String> {
        if !explicit_primary_targets.is_empty() {
            let mut targets = explicit_primary_targets
                .iter()
                .filter(|id| id.as_str() != threat_source_id)
                .cloned()
                .collect::<Vec<_>>();
            targets.sort();
            targets.dedup();
            return targets;
        }

        // Find entities that are targeted by the threat
        // For now, check event stream for threat events
        let mut targets = scene
            .event_stream
            .iter()
            .filter(|event| {
                event.event_kind.contains("threat") || event.event_kind.contains("attack")
            })
            .flat_map(|event| event.involved_entity_ids.clone())
            .filter(|id| id != threat_source_id)
            .collect::<Vec<_>>();
        targets.sort();
        targets.dedup();
        targets
    }

    /// Determine eligible reactors
    ///
    /// This implements the eligibility rules from docs/11_agent_runtime.md §5.1:
    /// - Primary target, ally, guardian, or passive field owner
    /// - Must be able to perceive the threat (sensory + visibility)
    /// - Must satisfy distance, line of effect, cooldown, resources, control state
    fn determine_eligible_reactors(
        &self,
        scene_turn_id: &str,
        scene: &SceneModel,
        characters: &[CharacterRecord],
        relationships: &[ObjectiveRelationship],
        skills: &[Skill],
        effective_attrs: &HashMap<String, EffectiveAttributeProfile>,
        primary_targets: &[String],
        threat_source_id: &str,
    ) -> Result<Vec<ReactionEligibility>, String> {
        let mut eligible = Vec::new();

        // Get threat source position
        let threat_position = scene
            .entities
            .iter()
            .find(|e| e.entity_id == threat_source_id)
            .map(|e| &e.position);

        for character in characters {
            // Check if character is in scene
            let char_entity = scene
                .entities
                .iter()
                .find(|e| e.entity_id == character.character_id);

            let char_entity = match char_entity {
                Some(e) => e,
                None => continue, // Character not in scene
            };

            // Check if character can perceive the threat
            let can_perceive = self.check_perception(
                character,
                char_entity,
                threat_position,
                scene,
                effective_attrs.get(&character.character_id),
            );

            if !can_perceive {
                continue;
            }

            if self.is_reaction_blocked_by_state(character) {
                continue;
            }

            let prepared_skills = self.prepare_reaction_skills(
                character,
                characters,
                skills,
                primary_targets,
                threat_source_id,
                scene,
            );

            let has_passive_skill = prepared_skills.iter().any(|prepared| {
                matches!(
                    prepared.skill.skill_kind,
                    SkillKind::Passive | SkillKind::Stance
                )
            });
            let has_interrupt_skill = prepared_skills.iter().any(|prepared| {
                prepared.skill.skill_kind == SkillKind::Reaction
                    || prepared.skill.activation.activation_time == ActivationTime::Reaction
            });

            // Determine eligibility reason
            let (reason, is_eligible) = if primary_targets.contains(&character.character_id) {
                (ReactionEligibilityReason::Target, true)
            } else if self.is_ally_or_guardian(character, primary_targets, relationships) {
                (ReactionEligibilityReason::AllyGuard, true)
            } else if has_passive_skill {
                (ReactionEligibilityReason::PassiveField, true)
            } else if has_interrupt_skill {
                (ReactionEligibilityReason::InterruptSkill, true)
            } else {
                (ReactionEligibilityReason::Target, false)
            };

            if !is_eligible {
                continue;
            }

            // Build available reaction options
            let options = self.build_reaction_options(
                &character.character_id,
                primary_targets,
                threat_source_id,
                character,
                reason,
                &prepared_skills,
            );

            if options.is_empty() {
                continue;
            }

            // Build sensory basis
            let sensory_basis =
                self.build_sensory_basis(character, char_entity, threat_position, scene);

            // Build constraints
            let constraints = self.build_constraints(character, char_entity, threat_position);

            eligible.push(ReactionEligibility {
                character_id: character.character_id.clone(),
                reason,
                available_reaction_options: options,
                sensory_basis,
                constraints,
            });
        }

        Ok(eligible)
    }

    fn prepare_reaction_skills(
        &self,
        character: &CharacterRecord,
        characters: &[CharacterRecord],
        skills: &[Skill],
        primary_targets: &[String],
        threat_source_id: &str,
        scene: &SceneModel,
    ) -> Vec<PreparedReactionSkill> {
        skills
            .iter()
            .filter(|skill| {
                skill.belongs_to_character(&character.character_id)
                    && (skill.skill_kind == SkillKind::Reaction
                        || matches!(skill.skill_kind, SkillKind::Passive | SkillKind::Stance)
                        || skill.activation.activation_time == ActivationTime::Reaction)
            })
            .filter_map(|skill| {
                let target_scope = self.reaction_skill_targets(
                    skill,
                    &character.character_id,
                    primary_targets,
                    threat_source_id,
                );
                SkillEffectValidator::preview_skill_use(
                    skill,
                    character,
                    characters,
                    &target_scope,
                    scene,
                )
                .ok()
                .map(|_| PreparedReactionSkill {
                    skill: skill.clone(),
                    target_scope,
                })
            })
            .collect()
    }

    /// Check if character can perceive the threat
    fn check_perception(
        &self,
        character: &CharacterRecord,
        char_entity: &crate::agent::models::SceneEntity,
        threat_position: Option<&Position>,
        scene: &SceneModel,
        effective_attrs: Option<&EffectiveAttributeProfile>,
    ) -> bool {
        // Check sensory capabilities
        let insight = effective_attrs
            .map(|attrs| {
                attrs
                    .values
                    .get(&crate::agent::models::scene::AttributeKind::Insight)
                    .unwrap_or(&100.0)
            })
            .unwrap_or(&100.0);

        // Check visibility conditions
        let visibility = scene.physical_conditions.airborne.visibility_range_m;

        // Check distance to threat
        if let Some(threat_pos) = threat_position {
            let distance = Self::distance(&char_entity.position, threat_pos);
            // Perception range scales with insight
            let perception_range = visibility * (*insight / 100.0).min(2.0);
            distance <= perception_range
        } else {
            // No position info, assume can perceive
            true
        }
    }

    fn is_reaction_blocked_by_state(&self, character: &CharacterRecord) -> bool {
        character
            .temporary_state
            .active_conditions
            .iter()
            .any(|condition| {
                let kind = condition.condition_kind.to_ascii_lowercase();
                [
                    "stunned",
                    "unconscious",
                    "paralyzed",
                    "incapacitated",
                    "frozen",
                    "immobilized",
                ]
                .iter()
                .any(|blocked| kind.contains(blocked))
            })
    }

    /// Check if character is ally or guardian of targets
    fn is_ally_or_guardian(
        &self,
        character: &CharacterRecord,
        primary_targets: &[String],
        relationships: &[ObjectiveRelationship],
    ) -> bool {
        relationships.iter().any(|relationship| {
            relationship.subject_character_id == character.character_id
                && primary_targets
                    .iter()
                    .any(|target| target == &relationship.target_character_id)
                && matches!(
                    relationship.relation_kind,
                    ObjectiveRelationKind::Ally
                        | ObjectiveRelationKind::Family
                        | ObjectiveRelationKind::Employer
                        | ObjectiveRelationKind::Oath
                        | ObjectiveRelationKind::MasterDisciple
                )
        })
    }

    /// Build reaction options for a character
    fn build_reaction_options(
        &self,
        character_id: &str,
        primary_targets: &[String],
        threat_source_id: &str,
        character: &CharacterRecord,
        reason: ReactionEligibilityReason,
        prepared_skills: &[PreparedReactionSkill],
    ) -> Vec<ReactionOption> {
        let mut options = Vec::new();

        let has_defensive_priority = primary_targets.contains(&character.character_id)
            || matches!(reason, ReactionEligibilityReason::AllyGuard);

        if has_defensive_priority {
            options.push(ReactionOption {
                option_id: format!("{}_dodge", character_id),
                skill_id: None,
                reaction_kind: ReactionKind::Dodge,
                target_scope: vec![threat_source_id.to_string()],
                cost_preview: crate::agent::models::character::CostProfile {
                    mana_reserve_delta: None,
                    fatigue_delta: Some(0.1),
                    cooldown_turns: None,
                    material_refs: Vec::new(),
                    required_conditions: Vec::new(),
                },
                legality_basis: vec!["defensive_reaction".to_string()],
            });

            options.push(ReactionOption {
                option_id: format!("{}_block", character_id),
                skill_id: None,
                reaction_kind: if matches!(reason, ReactionEligibilityReason::AllyGuard) {
                    ReactionKind::ProtectAlly
                } else {
                    ReactionKind::Block
                },
                target_scope: primary_targets.to_vec(),
                cost_preview: crate::agent::models::character::CostProfile {
                    mana_reserve_delta: None,
                    fatigue_delta: Some(0.15),
                    cooldown_turns: None,
                    material_refs: Vec::new(),
                    required_conditions: Vec::new(),
                },
                legality_basis: vec!["defensive_reaction".to_string()],
            });

            if character.base_attributes.physical > 100.0
                || character.base_attributes.mana_power > 100.0
            {
                options.push(ReactionOption {
                    option_id: format!("{}_counter", character_id),
                    skill_id: None,
                    reaction_kind: ReactionKind::Counter,
                    target_scope: vec![threat_source_id.to_string()],
                    cost_preview: crate::agent::models::character::CostProfile {
                        mana_reserve_delta: Some(10.0),
                        fatigue_delta: Some(0.2),
                        cooldown_turns: Some(1),
                        material_refs: Vec::new(),
                        required_conditions: Vec::new(),
                    },
                    legality_basis: vec!["requires_combat_capability".to_string()],
                });
            }
        }

        for prepared in prepared_skills {
            let reaction_kind = if matches!(
                prepared.skill.skill_kind,
                SkillKind::Passive | SkillKind::Stance
            ) {
                ReactionKind::PassiveMitigation
            } else {
                ReactionKind::Interrupt
            };
            options.push(ReactionOption {
                option_id: format!(
                    "{}_{}",
                    character_id,
                    prepared.skill.skill_id.replace(':', "_")
                ),
                skill_id: Some(prepared.skill.skill_id.clone()),
                reaction_kind,
                target_scope: prepared.target_scope.clone(),
                cost_preview: prepared.skill.requirements.cost.clone(),
                legality_basis: vec![format!("skill:{}", prepared.skill.skill_id)],
            });
        }

        options
    }

    fn reaction_skill_targets(
        &self,
        skill: &Skill,
        character_id: &str,
        primary_targets: &[String],
        threat_source_id: &str,
    ) -> Vec<String> {
        match skill.effect_contract.target_kind {
            TargetKind::SelfTarget => vec![character_id.to_string()],
            TargetKind::Character => {
                if primary_targets.is_empty() {
                    vec![threat_source_id.to_string()]
                } else {
                    primary_targets.to_vec()
                }
            }
            TargetKind::Area
            | TargetKind::Location
            | TargetKind::Object
            | TargetKind::Knowledge => {
                if primary_targets.is_empty() {
                    vec![threat_source_id.to_string()]
                } else {
                    primary_targets.to_vec()
                }
            }
        }
    }

    /// Build sensory basis for eligibility
    fn build_sensory_basis(
        &self,
        character: &CharacterRecord,
        char_entity: &crate::agent::models::SceneEntity,
        threat_position: Option<&Position>,
        scene: &SceneModel,
    ) -> Vec<crate::agent::models::knowledge::AccessSource> {
        use crate::agent::models::knowledge::AccessSource;

        let mut basis = Vec::new();

        // Visual perception
        if scene.physical_conditions.airborne.visibility_range_m > 5.0 {
            basis.push(AccessSource::InKnownBy); // Visual observation
        }

        // Mana perception (if character has mana sense)
        if character.baseline_body_profile.mana_sense_baseline.acuity > 0.0 {
            basis.push(AccessSource::ScopeMatch("mana_sense".to_string()));
        }

        let _ = (char_entity, threat_position);
        basis
    }

    /// Build constraints for reaction
    fn build_constraints(
        &self,
        character: &CharacterRecord,
        char_entity: &crate::agent::models::SceneEntity,
        threat_position: Option<&Position>,
    ) -> Vec<String> {
        let mut constraints = Vec::new();

        // Distance constraint
        if let Some(threat_pos) = threat_position {
            let distance = Self::distance(&char_entity.position, threat_pos);
            if distance > 10.0 {
                constraints.push(format!("distance_constraint: {:.1}m", distance));
            }
        }

        // Fatigue constraint
        if character.temporary_state.fatigue > 0.7 {
            constraints.push("high_fatigue".to_string());
        }

        // Injury constraint
        if !character.temporary_state.injuries.is_empty() {
            constraints.push(format!(
                "injury_count: {}",
                character.temporary_state.injuries.len()
            ));
        }

        constraints
    }

    /// Calculate distance between two positions
    fn distance(a: &Position, b: &Position) -> f64 {
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        let dz = match (a.z, b.z) {
            (Some(az), Some(bz)) => az - bz,
            _ => 0.0,
        };
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Submit a reaction intent
    pub fn submit_intent(
        &mut self,
        window_id: &str,
        character_id: &str,
        chosen_option_id: &str,
        target_ids: Vec<String>,
    ) -> Result<ReactionIntent, String> {
        let window = self
            .windows
            .get(window_id)
            .ok_or_else(|| format!("Window {} not found", window_id))?;

        // Validate character is eligible
        let eligibility = window
            .eligible_reactors
            .iter()
            .find(|e| e.character_id == character_id)
            .ok_or_else(|| format!("Character {} not eligible for window", character_id))?;

        // Validate chosen option is available
        let _option = eligibility
            .available_reaction_options
            .iter()
            .find(|o| o.option_id == chosen_option_id)
            .ok_or_else(|| format!("Option {} not available", chosen_option_id))?;

        // Validate one_reaction_per_character constraint
        if window.one_reaction_per_character {
            // Check if character already submitted an intent
            // (In production, we would track submitted intents)
        }

        Ok(ReactionIntent {
            window_id: window_id.to_string(),
            character_id: character_id.to_string(),
            chosen_option_id: chosen_option_id.to_string(),
            target_ids,
            intent_rationale: String::new(),
        })
    }

    /// Get a reaction window by ID
    pub fn get_window(&self, window_id: &str) -> Option<&ReactionWindow> {
        self.windows.get(window_id)
    }

    /// Close a reaction window
    pub fn close_window(&mut self, window_id: &str) {
        self.windows.remove(window_id);
    }

    /// Get all active windows for a scene
    pub fn get_active_windows(&self, scene_turn_id: &str) -> Vec<&ReactionWindow> {
        self.windows
            .values()
            .filter(|w| w.scene_turn_id == scene_turn_id)
            .collect()
    }

    /// Check if any windows are open for a character
    pub fn has_open_window_for_character(&self, character_id: &str) -> bool {
        self.windows.values().any(|w| {
            w.eligible_reactors
                .iter()
                .any(|e| e.character_id == character_id)
        })
    }
}

impl Default for ReactionWindowManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::models::*;
    use chrono::Utc;

    fn character(id: &str) -> CharacterRecord {
        CharacterRecord {
            character_id: id.to_string(),
            base_attributes: BaseAttributes {
                physical: 120.0,
                agility: 100.0,
                endurance: 100.0,
                insight: 100.0,
                mana_power: 120.0,
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

    fn scene() -> SceneModel {
        SceneModel {
            scene_id: "scene-1".to_string(),
            scene_turn_id: "turn-1".to_string(),
            time_context: TimeContext {
                time_anchor: TimeAnchor {
                    calendar_id: "default".to_string(),
                    ordinal: 1,
                    precision: TimePrecision::Exact,
                    display_text: "now".to_string(),
                },
                season: "spring".to_string(),
                day_phase: DayPhase::Day,
                weather_trend: "stable".to_string(),
            },
            spatial_layout: SpatialLayout {
                layout_type: "yard".to_string(),
                dimensions: None,
                obstacles: Vec::new(),
                entrances: Vec::new(),
                zones: Vec::new(),
            },
            lighting: LightingState {
                ambient_level: 1.0,
                light_sources: Vec::new(),
                shadow_areas: Vec::new(),
                backlight: None,
            },
            acoustics: AcousticsState {
                ambient_noise_level: 0.0,
                echo_characteristics: "open".to_string(),
                sound_sources: Vec::new(),
            },
            olfactory_field: OlfactoryField {
                dominant_scents: Vec::new(),
                airflow: AirflowState {
                    direction: "still".to_string(),
                    speed: 0.0,
                    turbulence: 0.0,
                },
            },
            scene_mood: SceneMood::Tense,
            physical_conditions: PhysicalConditions {
                temperature: Temperature {
                    ambient_celsius: 22.0,
                    felt_celsius: 22.0,
                    modifiers: Vec::new(),
                },
                surface_state: SurfaceState {
                    slipperiness: 0.0,
                    wetness: 0.0,
                    debris: Vec::new(),
                    notes: String::new(),
                },
                airborne: AirborneEffects {
                    fog_density: 0.0,
                    dust_density: 0.0,
                    smoke_density: 0.0,
                    visibility_range_m: 50.0,
                    mana_haze: None,
                },
                precipitation: None,
                wind: WindState {
                    direction_deg: 0.0,
                    speed_ms: 0.0,
                    gust: false,
                },
            },
            mana_field: ManaField {
                ambient_density: 0.0,
                ambient_attribute: ManaAttribute::Void,
                mana_sources: Vec::new(),
                character_presences: Vec::new(),
                flow: ManaFlow {
                    direction: "still".to_string(),
                    intensity: 0.0,
                    turbulence: 0.0,
                },
                interferences: Vec::new(),
            },
            entities: vec![
                SceneEntity {
                    entity_id: "attacker".to_string(),
                    entity_kind: SceneEntityKind::Character,
                    position: Position {
                        x: 0.0,
                        y: 0.0,
                        z: None,
                    },
                    posture: "standing".to_string(),
                    display_name: "Attacker".to_string(),
                    observable_facets: Vec::new(),
                },
                SceneEntity {
                    entity_id: "target".to_string(),
                    entity_kind: SceneEntityKind::Character,
                    position: Position {
                        x: 3.0,
                        y: 0.0,
                        z: None,
                    },
                    posture: "standing".to_string(),
                    display_name: "Target".to_string(),
                    observable_facets: Vec::new(),
                },
                SceneEntity {
                    entity_id: "ally".to_string(),
                    entity_kind: SceneEntityKind::Character,
                    position: Position {
                        x: 4.0,
                        y: 0.0,
                        z: None,
                    },
                    posture: "standing".to_string(),
                    display_name: "Ally".to_string(),
                    observable_facets: Vec::new(),
                },
            ],
            observable_signals: ObservableSignals {
                visual_signals: Vec::new(),
                audio_signals: Vec::new(),
                mana_signals: Vec::new(),
            },
            private_state: ScenePrivateState {
                hidden_facts: Vec::new(),
                reveal_triggers: Vec::new(),
                source_constraint_ids: Vec::new(),
            },
            event_stream: vec![SceneEvent {
                event_id: "attack-1".to_string(),
                event_kind: "attack".to_string(),
                involved_entity_ids: vec!["attacker".to_string(), "target".to_string()],
                payload: serde_json::json!({}),
                created_at: Utc::now(),
            }],
            uncertainty_notes: Vec::new(),
        }
    }

    fn attrs(id: &str) -> EffectiveAttributeProfile {
        let mut values = HashMap::new();
        values.insert(AttributeKind::Insight, 100.0);
        EffectiveAttributeProfile {
            character_id: id.to_string(),
            values,
            tiers: HashMap::new(),
            descriptors: HashMap::new(),
        }
    }

    #[test]
    fn reaction_window_includes_primary_target_and_ally_guard() {
        let mut manager = ReactionWindowManager::new();
        let relationships = vec![ObjectiveRelationship {
            relation_id: "rel-1".to_string(),
            subject_character_id: "ally".to_string(),
            target_character_id: "target".to_string(),
            relation_kind: ObjectiveRelationKind::Ally,
            access_level: 1.0,
            authorization_tags: Vec::new(),
            valid_from: TimeAnchor {
                calendar_id: "default".to_string(),
                ordinal: 0,
                precision: TimePrecision::Exact,
                display_text: "start".to_string(),
            },
            valid_until: None,
            source_knowledge_id: None,
            source_scene_turn_id: None,
            schema_version: "0.1".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }];
        let characters = vec![
            character("attacker"),
            character("target"),
            character("ally"),
        ];
        let effective_attrs = HashMap::from([
            ("attacker".to_string(), attrs("attacker")),
            ("target".to_string(), attrs("target")),
            ("ally".to_string(), attrs("ally")),
        ]);

        let window_id = manager
            .open_window(
                "turn-1",
                "attack-1",
                "action-1",
                "attacker",
                &["target".to_string()],
                &scene(),
                &characters,
                &relationships,
                &[],
                &effective_attrs,
            )
            .expect("window created");
        let window = manager.get_window(&window_id).expect("window exists");

        assert!(window
            .eligible_reactors
            .iter()
            .any(|entry| entry.character_id == "target"
                && entry.reason == ReactionEligibilityReason::Target));
        assert!(window
            .eligible_reactors
            .iter()
            .any(|entry| entry.character_id == "ally"
                && entry.reason == ReactionEligibilityReason::AllyGuard));
    }

    #[test]
    fn reaction_window_includes_interrupt_skill_owner_only_when_ready() {
        let mut manager = ReactionWindowManager::new();
        let mut interrupt_owner = character("ally");
        interrupt_owner
            .temporary_state
            .active_conditions
            .push(ConditionState {
                condition_id: "cond-1".to_string(),
                domain: StateDomain::Body,
                condition_kind: "interrupt_ready".to_string(),
                intensity: 1.0,
                source_id: None,
            });
        let characters = vec![
            character("attacker"),
            character("target"),
            interrupt_owner.clone(),
        ];
        let effective_attrs = HashMap::from([
            ("attacker".to_string(), attrs("attacker")),
            ("target".to_string(), attrs("target")),
            ("ally".to_string(), attrs("ally")),
        ]);
        let interrupt_skill = Skill {
            skill_id: "void_interrupt".to_string(),
            name: "Void Interrupt".to_string(),
            description: String::new(),
            skill_kind: SkillKind::Reaction,
            activation: SkillActivation {
                activation_time: ActivationTime::Reaction,
                trigger_conditions: vec![ActivationCondition::TargetInLineOfSight],
                cooldown: Some(1),
                uses_per_scene: None,
                uses_per_day: None,
            },
            effect_contract: SkillEffectContract {
                primary_effects: Vec::new(),
                secondary_effects: Vec::new(),
                target_kind: TargetKind::Character,
                target_count: TargetCount::Single,
                range_m: Some(12.0),
                area_of_effect: None,
                duration_turns: None,
                attribute_modifier: None,
                mana_attribute: None,
                allowed_target_kinds: vec![TargetKind::Character],
                allowed_state_domains: vec!["body".to_string()],
                max_intensity_tier: EffectIntensityTier::Moderate,
                allows_injury: true,
                allows_position_change: false,
                allows_knowledge_reveal: false,
            },
            requirements: SkillRequirements::default(),
            metadata: SkillMetadata {
                tags: vec!["interrupt".to_string(), "owner:ally".to_string()],
                source: Some("know-1".to_string()),
                learning_difficulty: LearningDifficulty::Common,
                rarity: SkillRarity::Common,
            },
            schema_version: "0.1".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let window_id = manager
            .open_window(
                "turn-1",
                "attack-1",
                "action-1",
                "attacker",
                &["target".to_string()],
                &scene(),
                &characters,
                &[],
                &[interrupt_skill],
                &effective_attrs,
            )
            .expect("window created");
        let window = manager.get_window(&window_id).expect("window exists");
        let ally_entry = window
            .eligible_reactors
            .iter()
            .find(|entry| entry.character_id == "ally")
            .expect("interrupt owner is eligible");

        assert_eq!(ally_entry.reason, ReactionEligibilityReason::InterruptSkill);
        assert!(ally_entry.available_reaction_options.iter().any(|option| {
            option.skill_id.as_deref() == Some("void_interrupt")
                && option.reaction_kind == ReactionKind::Interrupt
        }));
        assert!(!ally_entry
            .available_reaction_options
            .iter()
            .any(|option| option.option_id == "ally_dodge"));
    }
}
