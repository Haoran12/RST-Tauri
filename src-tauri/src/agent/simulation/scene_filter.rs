//! Scene filter
//!
//! Derives Layer 2 FilteredSceneView from Layer 1 SceneModel.
//!
//! Key invariant: FilteredSceneView contains only tier/delta/descriptor data,
//! never raw Layer 1 values.
//!
//! Performance optimization: Supports caching via TurnScopedCache.

use std::sync::Arc;

use crate::agent::cache::{SceneDerivedKey, TurnScopedCache};
use crate::agent::models::{
    AmbientManaDensityTier, AttributeEvidenceKind, AttributeKind, AudioSignal,
    EffectiveAttributeProfile, EmbodimentState, FilteredSceneView, ManaEnvironmentSense,
    ManaPresenceSense, ManaSignal, ObservableEntity, OlfactorySignal, PerceivedAttributeProfile,
    PrecipitationDescriptor, PrecipitationIntensityTier, RespirationImpactTier, SceneModel,
    SpatialContext, SurfaceImpactTier, SurfaceVisualState, TactileSignal, TemperatureFeelTier,
    VisibilityTier, WeatherPerception, WindImpactTier,
};

/// Scene filter - derives Layer 2 filtered scene view
pub struct SceneFilter {
    /// Optional cache for performance optimization
    cache: Option<Arc<TurnScopedCache>>,
}

impl SceneFilter {
    /// Create a new filter without caching
    pub fn new() -> Self {
        Self { cache: None }
    }

    /// Create a new filter with caching enabled
    pub fn with_cache(cache: Arc<TurnScopedCache>) -> Self {
        Self { cache: Some(cache) }
    }

    /// Derive filtered scene view for a character
    pub fn filter_scene(
        &self,
        scene: &SceneModel,
        embodiment: &EmbodimentState,
        _effective_attrs: &EffectiveAttributeProfile,
    ) -> FilteredSceneView {
        // Check cache
        if let Some(ref cache) = self.cache {
            let key = SceneDerivedKey {
                character_id: embodiment.character_id.clone(),
                scene_turn_id: scene.scene_turn_id.clone(),
                embodiment_hash: 0, // Simplified
            };
            if let Some(cached) = cache.get_scene_filter(&key) {
                return (*cached).clone();
            }
        }

        // Compute
        let result = self.filter_scene_uncached(scene, embodiment, _effective_attrs);

        // Cache result
        if let Some(ref cache) = self.cache {
            let key = SceneDerivedKey {
                character_id: embodiment.character_id.clone(),
                scene_turn_id: scene.scene_turn_id.clone(),
                embodiment_hash: 0,
            };
            cache.insert_scene_filter(key, Arc::new(result.clone()));
        }

        result
    }

    /// Internal implementation without caching
    fn filter_scene_uncached(
        &self,
        scene: &SceneModel,
        embodiment: &EmbodimentState,
        effective_attrs: &EffectiveAttributeProfile,
    ) -> FilteredSceneView {
        // Step 1: Filter observable entities
        let observable_entities = Self::filter_entities(scene, embodiment);

        // Step 2: Derive perceived attributes
        let perceived_attributes = Self::derive_perceived_attributes(
            scene,
            embodiment,
            effective_attrs,
            &observable_entities,
        );

        // Step 3: Filter audible signals
        let audible_signals = Self::filter_audio(scene, embodiment);

        // Step 4: Filter olfactory signals
        let olfactory_signals = Self::filter_olfactory(scene, embodiment);

        // Step 5: Filter tactile signals
        let tactile_signals = Self::filter_tactile(scene, embodiment);

        // Step 6: Derive mana signals
        let mana_signals = Self::derive_mana_signals(scene, embodiment);

        // Step 7: Derive mana environment
        let mana_environment = Self::derive_mana_environment(scene, embodiment);

        // Step 8: Derive weather perception
        let weather_perception = Self::derive_weather_perception(scene);

        // Step 9: Derive spatial context
        let spatial_context = Self::derive_spatial_context(scene);

        FilteredSceneView {
            character_id: embodiment.character_id.clone(),
            scene_turn_id: scene.scene_turn_id.clone(),
            observable_entities,
            perceived_attributes,
            audible_signals,
            olfactory_signals,
            tactile_signals,
            mana_signals,
            mana_environment,
            weather_perception,
            spatial_context,
        }
    }

    /// Filter observable entities from scene
    fn filter_entities(scene: &SceneModel, embodiment: &EmbodimentState) -> Vec<ObservableEntity> {
        let visibility = embodiment.sensory_capabilities.vision.availability;

        scene
            .entities
            .iter()
            .filter(|entity| entity.entity_id != embodiment.character_id)
            .filter_map(|entity| {
                // Check visibility
                if visibility < 0.1 {
                    return None;
                }

                Some(ObservableEntity {
                    entity_id: entity.entity_id.clone(),
                    perception_score: visibility,
                    clarity: visibility,
                    observable_facets: entity.observable_facets.clone(),
                    notes: String::new(),
                })
            })
            .collect()
    }

    /// Derive tier/delta-only perceived attributes from visible facets and mana signals.
    ///
    /// This intentionally does not expose raw Layer 1 numbers. If a target has only
    /// visual evidence, the profile carries evidence/descriptors without a tier.
    /// Mana pressure can carry a relative delta because the source scene model
    /// already stores it as a qualitative tier.
    fn derive_perceived_attributes(
        scene: &SceneModel,
        embodiment: &EmbodimentState,
        effective_attrs: &EffectiveAttributeProfile,
        observable_entities: &[ObservableEntity],
    ) -> Vec<PerceivedAttributeProfile> {
        let mut profiles = Vec::new();
        let insight = effective_attrs
            .values
            .get(&AttributeKind::Insight)
            .copied()
            .unwrap_or(0.0);
        let read_confidence = (0.25 + insight / 4000.0).clamp(0.25, 0.85);

        for entity in observable_entities {
            let mut evidence = Vec::new();
            let mut descriptors = Vec::new();
            if entity
                .observable_facets
                .iter()
                .any(|facet| facet.contains("appearance") || facet.contains("body"))
            {
                evidence.push(AttributeEvidenceKind::Appearance);
                descriptors.push("visible body cues".to_string());
            }
            if entity
                .observable_facets
                .iter()
                .any(|facet| facet.contains("movement") || facet.contains("posture"))
            {
                evidence.push(AttributeEvidenceKind::Movement);
                descriptors.push("visible movement cues".to_string());
            }

            if !evidence.is_empty() {
                profiles.push(PerceivedAttributeProfile {
                    source_id: entity.entity_id.clone(),
                    attribute_kind: AttributeKind::Physical,
                    tier_assessment: None,
                    delta: None,
                    confidence: (entity.clarity * read_confidence).clamp(0.0, 1.0),
                    evidence,
                    descriptors,
                });
            }
        }

        let mana_availability = embodiment.sensory_capabilities.mana.availability;
        if mana_availability >= 0.1 {
            for presence in scene
                .mana_field
                .character_presences
                .iter()
                .filter(|presence| presence.character_id != embodiment.character_id)
            {
                let mut descriptors = presence.descriptors.clone();
                descriptors.push(format!("{:?} mana expression", presence.expression_mode));
                profiles.push(PerceivedAttributeProfile {
                    source_id: presence.character_id.clone(),
                    attribute_kind: AttributeKind::ManaPower,
                    tier_assessment: None,
                    delta: Some(presence.pressure_delta),
                    confidence: mana_availability.clamp(0.0, 1.0),
                    evidence: vec![AttributeEvidenceKind::ManaSignal],
                    descriptors,
                });
            }
        }

        profiles
    }

    /// Filter audio signals from scene
    fn filter_audio(scene: &SceneModel, embodiment: &EmbodimentState) -> Vec<AudioSignal> {
        let hearing = embodiment.sensory_capabilities.hearing.availability;

        scene
            .observable_signals
            .audio_signals
            .iter()
            .filter(|signal| hearing > 0.1)
            .map(|signal| AudioSignal {
                signal_id: signal.signal_id.clone(),
                source_entity_id: signal.source_entity_id.clone(),
                signal_type: signal.signal_type.clone(),
                description: signal.description.clone(),
                volume: signal.volume * hearing,
            })
            .collect()
    }

    /// Filter olfactory signals from scene
    fn filter_olfactory(scene: &SceneModel, embodiment: &EmbodimentState) -> Vec<OlfactorySignal> {
        let smell = embodiment.sensory_capabilities.smell.availability;

        scene
            .olfactory_field
            .dominant_scents
            .iter()
            .filter(|_scent| smell > 0.1)
            .map(|scent| OlfactorySignal {
                signal_id: scent.source_id.clone(),
                source_entity_id: Some(scent.source_id.clone()),
                signal_type: scent.scent_type.clone(),
                description: scent.scent_type.clone(),
                intensity: scent.intensity * smell,
            })
            .collect()
    }

    /// Filter tactile signals from scene
    fn filter_tactile(scene: &SceneModel, embodiment: &EmbodimentState) -> Vec<TactileSignal> {
        let touch = embodiment.sensory_capabilities.touch.availability;

        // Generate tactile signals from physical conditions
        let mut signals = Vec::new();

        let physical = &scene.physical_conditions;

        // Temperature
        if touch > 0.1 {
            signals.push(TactileSignal {
                signal_id: "tactile_temperature".to_string(),
                source_entity_id: None,
                signal_type: "temperature".to_string(),
                description: if physical.temperature.felt_celsius < 10.0 {
                    "寒冷".to_string()
                } else if physical.temperature.felt_celsius > 30.0 {
                    "炎热".to_string()
                } else {
                    "温度适宜".to_string()
                },
                intensity: touch,
            });

            // Wind
            if physical.wind.speed_ms > 5.0 {
                signals.push(TactileSignal {
                    signal_id: "tactile_wind".to_string(),
                    source_entity_id: None,
                    signal_type: "wind".to_string(),
                    description: "风吹".to_string(),
                    intensity: touch * (physical.wind.speed_ms / 10.0).min(1.0),
                });
            }
        }

        signals
    }

    /// Derive mana signals from scene
    fn derive_mana_signals(scene: &SceneModel, embodiment: &EmbodimentState) -> Vec<ManaSignal> {
        let mana_availability = embodiment.sensory_capabilities.mana.availability;
        if mana_availability < 0.1 {
            return Vec::new();
        }

        // Process character presences
        scene
            .mana_field
            .character_presences
            .iter()
            .filter(|presence| presence.character_id != embodiment.character_id)
            .map(|presence| {
                let description = format!("{:?}气息", presence.expression_mode);

                ManaSignal {
                    signal_id: format!("mana_{}", presence.character_id),
                    source_entity_id: Some(presence.character_id.clone()),
                    signal_type: "character_mana".to_string(),
                    description,
                    intensity: mana_availability,
                    attribute: None,
                }
            })
            .collect()
    }

    /// Derive mana environment from scene
    fn derive_mana_environment(
        scene: &SceneModel,
        embodiment: &EmbodimentState,
    ) -> ManaEnvironmentSense {
        let mana_availability = embodiment.sensory_capabilities.mana.availability;

        // Derive density tier
        let density_tier = Self::density_to_tier(scene.mana_field.ambient_density);

        // Derive character presences
        let character_presences: Vec<ManaPresenceSense> = scene
            .mana_field
            .character_presences
            .iter()
            .filter(|p| p.character_id != embodiment.character_id)
            .map(|p| ManaPresenceSense {
                source_id: p.character_id.clone(),
                expression_assessment: Some(p.expression_mode),
                radius_tier: p.radius_tier,
                pressure_delta: p.pressure_delta,
                cognitive_effect_hints: Vec::new(),
            })
            .collect();

        // Check for overload risk
        let overload_risk =
            mana_availability > 0.8 && matches!(density_tier, AmbientManaDensityTier::Saturated);

        ManaEnvironmentSense {
            density_tier,
            dominant_attribute: Some(scene.mana_field.ambient_attribute),
            character_presences,
            interferences: Vec::new(),
            overload_risk,
            descriptors: Vec::new(),
        }
    }

    /// Derive weather perception from scene
    fn derive_weather_perception(scene: &SceneModel) -> WeatherPerception {
        let physical = &scene.physical_conditions;

        // Wind tier
        let wind_tier = Self::wind_speed_to_tier(physical.wind.speed_ms);

        // Temperature tier
        let temperature_tier = Self::temperature_to_tier(physical.temperature.felt_celsius);

        // Visibility tier
        let visibility_tier =
            Self::visibility_to_tier(scene.physical_conditions.airborne.visibility_range_m);

        // Respiration tier
        let respiration_tier = Self::airborne_to_respiration_tier(&physical.airborne);

        // Surface tier
        let surface_tier = Self::surface_to_tier(&physical.surface_state);

        // Surface visual
        let surface_visual = Self::derive_surface_visual(&physical.surface_state);

        // Precipitation
        let precipitation = physical
            .precipitation
            .as_ref()
            .map(|p| PrecipitationDescriptor {
                kind: p.kind,
                intensity_tier: PrecipitationIntensityTier::Moderate,
                mana_attribute: p.mana_attribute,
            });

        // Effect hints
        let effect_hints =
            Self::generate_effect_hints(&wind_tier, &temperature_tier, &visibility_tier);

        WeatherPerception {
            wind_tier,
            temperature_tier,
            visibility_tier,
            respiration_tier,
            surface_visual,
            surface_tier,
            precipitation,
            effect_hints,
        }
    }

    /// Derive spatial context from scene
    fn derive_spatial_context(scene: &SceneModel) -> SpatialContext {
        SpatialContext {
            layout_type: scene.spatial_layout.layout_type.clone(),
            visible_zones: scene.spatial_layout.zones.clone(),
            visible_obstacles: scene.spatial_layout.obstacles.clone(),
            visible_entrances: scene.spatial_layout.entrances.clone(),
        }
    }

    // ===== Helper methods for tier conversions =====

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

    fn temperature_to_tier(felt_celsius: f64) -> TemperatureFeelTier {
        if felt_celsius < -15.0 {
            TemperatureFeelTier::SevereCold
        } else if felt_celsius < 5.0 {
            TemperatureFeelTier::Cold
        } else if felt_celsius < 15.0 {
            TemperatureFeelTier::Cool
        } else if felt_celsius < 25.0 {
            TemperatureFeelTier::Comfortable
        } else if felt_celsius < 32.0 {
            TemperatureFeelTier::Warm
        } else if felt_celsius < 38.0 {
            TemperatureFeelTier::Hot
        } else {
            TemperatureFeelTier::Sweltering
        }
    }

    fn visibility_to_tier(visibility_m: f64) -> VisibilityTier {
        if visibility_m > 100.0 {
            VisibilityTier::Clear
        } else if visibility_m > 20.0 {
            VisibilityTier::Hazy
        } else if visibility_m > 5.0 {
            VisibilityTier::Limited
        } else {
            VisibilityTier::Blind
        }
    }

    fn airborne_to_respiration_tier(
        airborne: &crate::agent::models::AirborneEffects,
    ) -> RespirationImpactTier {
        let total_density = airborne.fog_density + airborne.dust_density + airborne.smoke_density;

        if total_density > 0.8 {
            RespirationImpactTier::Suffocating
        } else if total_density > 0.5 {
            RespirationImpactTier::Choking
        } else if total_density > 0.2 {
            RespirationImpactTier::Irritating
        } else {
            RespirationImpactTier::Free
        }
    }

    fn surface_to_tier(surface: &crate::agent::models::SurfaceState) -> SurfaceImpactTier {
        if surface.slipperiness > 0.7 {
            SurfaceImpactTier::Treacherous
        } else if surface.slipperiness > 0.3 {
            SurfaceImpactTier::Slippery
        } else {
            SurfaceImpactTier::Stable
        }
    }

    fn derive_surface_visual(
        surface: &crate::agent::models::SurfaceState,
    ) -> Vec<SurfaceVisualState> {
        let mut visuals = Vec::new();

        if surface.wetness > 0.7 {
            visuals.push(SurfaceVisualState::Puddled);
        } else if surface.wetness > 0.3 {
            visuals.push(SurfaceVisualState::Wet);
        } else if surface.wetness > 0.1 {
            visuals.push(SurfaceVisualState::Damp);
        } else {
            visuals.push(SurfaceVisualState::Dry);
        }

        visuals
    }

    fn density_to_tier(density: f64) -> AmbientManaDensityTier {
        if density < 0.1 {
            AmbientManaDensityTier::Barren
        } else if density < 0.3 {
            AmbientManaDensityTier::Sparse
        } else if density < 0.5 {
            AmbientManaDensityTier::Normal
        } else if density < 0.7 {
            AmbientManaDensityTier::Rich
        } else if density < 0.9 {
            AmbientManaDensityTier::Dense
        } else {
            AmbientManaDensityTier::Saturated
        }
    }

    fn generate_effect_hints(
        wind_tier: &WindImpactTier,
        temp_tier: &TemperatureFeelTier,
        visibility_tier: &VisibilityTier,
    ) -> Vec<String> {
        let mut hints = Vec::new();

        match wind_tier {
            WindImpactTier::Strong => hints.push("风力较强，远程攻击受影响".to_string()),
            WindImpactTier::Gale => hints.push("大风天气，行动困难".to_string()),
            WindImpactTier::Storm | WindImpactTier::Hurricane => {
                hints.push("风暴天气，站立困难".to_string())
            }
            _ => {}
        }

        match temp_tier {
            TemperatureFeelTier::SevereCold => hints.push("严寒，长时间暴露有冻伤风险".to_string()),
            TemperatureFeelTier::Sweltering => hints.push("酷热，长时间暴露有中暑风险".to_string()),
            _ => {}
        }

        match visibility_tier {
            VisibilityTier::Limited => hints.push("能见度低，远程观察困难".to_string()),
            VisibilityTier::Blind => hints.push("几乎无法视物".to_string()),
            _ => {}
        }

        hints
    }
}

impl Default for SceneFilter {
    fn default() -> Self {
        Self::new()
    }
}
