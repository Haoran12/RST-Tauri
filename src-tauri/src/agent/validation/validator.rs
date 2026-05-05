//! Main validator
//!
//! Validates LLM outputs against rules.
//! Implements validation rules from docs/11_agent_runtime.md §9.

use std::collections::HashSet;

use crate::agent::models::{
    AccessSource, CharacterCognitivePassInput, CharacterCognitivePassOutput, OutcomePlannerInput,
    OutcomePlannerOutput, ReactionIntent, ReactionWindow, SceneStateExtractorInput,
    SceneStateExtractorOutput, SurfaceRealizerInput, SurfaceRealizerOutput,
};

/// Main validator - validates LLM outputs
pub struct Validator;

/// Validation issue
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub rule: String,
    pub severity: ValidationSeverity,
    pub description: String,
    pub field_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationSeverity {
    Warning,
    Error,
    Critical,
}

/// Aggregated validation result for a turn
#[derive(Debug, Clone)]
pub struct TurnValidationResult {
    /// Issues from SceneStateExtractor
    pub extractor_issues: Vec<ValidationIssue>,
    /// Issues from CognitivePass (per character)
    pub cognitive_issues: std::collections::HashMap<String, Vec<ValidationIssue>>,
    /// Issues from ReactionIntent
    pub reaction_issues: Vec<ValidationIssue>,
    /// Issues from OutcomePlanner
    pub outcome_issues: Vec<ValidationIssue>,
    /// Issues from SurfaceRealizer
    pub narrative_issues: Vec<ValidationIssue>,
    /// Overall pass/fail status
    pub passed: bool,
    /// Summary of critical errors
    pub critical_summary: Vec<String>,
}

impl Default for TurnValidationResult {
    fn default() -> Self {
        Self {
            extractor_issues: Vec::new(),
            cognitive_issues: std::collections::HashMap::new(),
            reaction_issues: Vec::new(),
            outcome_issues: Vec::new(),
            narrative_issues: Vec::new(),
            passed: true,
            critical_summary: Vec::new(),
        }
    }
}

impl TurnValidationResult {
    /// Create a new empty result
    pub fn new() -> Self {
        Self::default()
    }

    /// Add extractor issues
    pub fn add_extractor_issues(&mut self, issues: Vec<ValidationIssue>) {
        self.extractor_issues.extend(issues);
        self.update_passed();
    }

    /// Add cognitive issues for a character
    pub fn add_cognitive_issues(&mut self, character_id: String, issues: Vec<ValidationIssue>) {
        self.cognitive_issues
            .insert(character_id.clone(), issues.clone());
        self.update_passed();
    }

    /// Add reaction issues
    pub fn add_reaction_issues(&mut self, issues: Vec<ValidationIssue>) {
        self.reaction_issues.extend(issues);
        self.update_passed();
    }

    /// Add outcome issues
    pub fn add_outcome_issues(&mut self, issues: Vec<ValidationIssue>) {
        self.outcome_issues.extend(issues);
        self.update_passed();
    }

    /// Add narrative issues
    pub fn add_narrative_issues(&mut self, issues: Vec<ValidationIssue>) {
        self.narrative_issues.extend(issues);
        self.update_passed();
    }

    /// Update passed status based on issues
    fn update_passed(&mut self) {
        // Check for any critical or error issues
        let has_errors = self
            .extractor_issues
            .iter()
            .any(|i| i.severity >= ValidationSeverity::Error)
            || self
                .cognitive_issues
                .values()
                .flatten()
                .any(|i| i.severity >= ValidationSeverity::Error)
            || self
                .reaction_issues
                .iter()
                .any(|i| i.severity >= ValidationSeverity::Error)
            || self
                .outcome_issues
                .iter()
                .any(|i| i.severity >= ValidationSeverity::Error)
            || self
                .narrative_issues
                .iter()
                .any(|i| i.severity >= ValidationSeverity::Error);

        self.passed = !has_errors;

        // Collect critical summary
        self.critical_summary = self
            .extractor_issues
            .iter()
            .chain(self.cognitive_issues.values().flatten())
            .chain(self.reaction_issues.iter())
            .chain(self.outcome_issues.iter())
            .chain(self.narrative_issues.iter())
            .filter(|i| i.severity == ValidationSeverity::Critical)
            .map(|i| format!("[{}] {}", i.rule, i.description))
            .collect();
    }

    /// Get all issues flattened
    pub fn all_issues(&self) -> Vec<&ValidationIssue> {
        let mut all: Vec<&ValidationIssue> = Vec::new();
        all.extend(self.extractor_issues.iter());
        for issues in self.cognitive_issues.values() {
            all.extend(issues.iter());
        }
        all.extend(self.reaction_issues.iter());
        all.extend(self.outcome_issues.iter());
        all.extend(self.narrative_issues.iter());
        all
    }

    /// Count issues by severity
    pub fn count_by_severity(&self, severity: ValidationSeverity) -> usize {
        self.all_issues()
            .iter()
            .filter(|i| i.severity == severity)
            .count()
    }
}

impl Validator {
    // ========== Validation entry points ==========

    /// Validate SceneStateExtractor output
    pub fn validate_extractor(
        output: &SceneStateExtractorOutput,
        input: &SceneStateExtractorInput,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Check for empty output
        let has_scene_update = output.scene_update.is_some();
        let has_provisional_truths = !output.provisional_truth_candidates.is_empty();

        if !has_scene_update && !has_provisional_truths {
            issues.push(Self::issue(
                "empty_extraction",
                ValidationSeverity::Warning,
                "SceneStateExtractor produced no scene update or provisional truths",
                None,
            ));
        }

        // Check for entity reference validity in scene update
        if let Some(scene_update) = &output.scene_update {
            let known_entity_ids: HashSet<&str> = input
                .current_scene
                .entities
                .iter()
                .map(|e| e.entity_id.as_str())
                .collect();

            for delta in &scene_update.scene_delta.entity_deltas {
                if !known_entity_ids.contains(delta.entity_id.as_str()) && delta.delta_kind != "add"
                {
                    issues.push(Self::issue(
                        "invalid_entity_ref",
                        ValidationSeverity::Error,
                        format!(
                            "entity_delta references unknown entity '{}'",
                            delta.entity_id
                        ),
                        Some(format!(
                            "scene_update.scene_delta.entity_deltas[{}]",
                            delta.entity_id
                        )),
                    ));
                }
            }
        }

        issues
    }

    /// Validate CharacterCognitivePass output
    pub fn validate_cognitive(
        output: &CharacterCognitivePassOutput,
        input: &CharacterCognitivePassInput,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Run all cognitive validation rules
        issues.extend(Self::check_omniscience_leakage(output, input));
        issues.extend(Self::check_embodiment_ignored(output, input));
        issues.extend(Self::check_self_awareness(output, input));
        issues.extend(Self::check_god_only(output, input));
        issues.extend(Self::check_mana_sense(output, input));
        issues.extend(Self::check_apparent_vs_true(output, input));

        issues
    }

    /// Validate ReactionIntent
    pub fn validate_reaction(
        intent: &ReactionIntent,
        window: &ReactionWindow,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Check character eligibility
        let is_eligible = window
            .eligible_reactors
            .iter()
            .any(|e| e.character_id == intent.character_id);

        if !is_eligible {
            issues.push(Self::issue(
                "reaction_eligibility",
                ValidationSeverity::Error,
                format!(
                    "character '{}' is not eligible to react in this window",
                    intent.character_id
                ),
                Some("character_id".to_string()),
            ));
        }

        // Check option validity
        if let Some(eligibility) = window
            .eligible_reactors
            .iter()
            .find(|e| e.character_id == intent.character_id)
        {
            let option_exists = eligibility
                .available_reaction_options
                .iter()
                .any(|o| o.option_id == intent.chosen_option_id);

            if !option_exists {
                issues.push(Self::issue(
                    "invalid_reaction_option",
                    ValidationSeverity::Error,
                    format!(
                        "chosen option '{}' is not available",
                        intent.chosen_option_id
                    ),
                    Some("chosen_option_id".to_string()),
                ));
            }
        }

        issues
    }

    /// Validate OutcomePlanner output
    pub fn validate_outcome(
        output: &OutcomePlannerOutput,
        _input: &OutcomePlannerInput,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Check for empty outcome plan
        if output.outcome_plan.outward_actions.is_empty() {
            issues.push(Self::issue(
                "no_outward_actions",
                ValidationSeverity::Warning,
                "OutcomePlanner produced no outward actions",
                None,
            ));
        }

        // Check for conflicts
        if !output.conflict_reports.is_empty() {
            for (idx, conflict) in output.conflict_reports.iter().enumerate() {
                let summary_str = serde_json::to_string(&conflict.summary)
                    .unwrap_or_else(|_| "unknown".to_string());
                issues.push(Self::issue(
                    "outcome_conflict",
                    ValidationSeverity::Warning,
                    format!("conflict report {}: {}", idx, summary_str),
                    Some(format!("conflict_reports[{}]", idx)),
                ));
            }
        }

        // Check narratable facts
        if output.outcome_plan.narratable_facts.is_empty() {
            issues.push(Self::issue(
                "no_narratable_facts",
                ValidationSeverity::Warning,
                "OutcomePlanner produced no narratable facts",
                None,
            ));
        }

        issues
    }

    /// Validate SurfaceRealizer output
    pub fn validate_narrative(
        output: &SurfaceRealizerOutput,
        _input: &SurfaceRealizerInput,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Check for empty narrative
        if output.narrative_text.trim().is_empty() {
            issues.push(Self::issue(
                "empty_narrative",
                ValidationSeverity::Error,
                "SurfaceRealizer produced empty narrative text",
                Some("narrative_text".to_string()),
            ));
        }

        // Check narrative length (warning only)
        if output.narrative_text.len() > 10000 {
            issues.push(Self::issue(
                "long_narrative",
                ValidationSeverity::Warning,
                format!(
                    "narrative is very long ({} chars)",
                    output.narrative_text.len()
                ),
                Some("narrative_text".to_string()),
            ));
        }

        // Check for unresolved placeholders
        if output.narrative_text.contains("{{") || output.narrative_text.contains("}}") {
            issues.push(Self::issue(
                "unresolved_placeholders",
                ValidationSeverity::Warning,
                "narrative contains unresolved template placeholders",
                Some("narrative_text".to_string()),
            ));
        }

        issues
    }

    /// Validate a complete turn and aggregate results
    pub fn validate_turn(
        extractor_output: Option<(&SceneStateExtractorOutput, &SceneStateExtractorInput)>,
        cognitive_outputs: &std::collections::HashMap<
            String,
            (CharacterCognitivePassOutput, CharacterCognitivePassInput),
        >,
        reaction_intents: &[(ReactionIntent, ReactionWindow)],
        outcome_output: Option<(&OutcomePlannerOutput, &OutcomePlannerInput)>,
        narrative_output: Option<(&SurfaceRealizerOutput, &SurfaceRealizerInput)>,
    ) -> TurnValidationResult {
        let mut result = TurnValidationResult::new();

        // Validate extractor
        if let Some((output, input)) = extractor_output {
            result.add_extractor_issues(Self::validate_extractor(output, input));
        }

        // Validate cognitive passes
        for (character_id, (output, input)) in cognitive_outputs {
            result.add_cognitive_issues(
                character_id.clone(),
                Self::validate_cognitive(output, input),
            );
        }

        // Validate reactions
        for (intent, window) in reaction_intents {
            result.add_reaction_issues(Self::validate_reaction(intent, window));
        }

        // Validate outcome
        if let Some((output, input)) = outcome_output {
            result.add_outcome_issues(Self::validate_outcome(output, input));
        }

        // Validate narrative
        if let Some((output, input)) = narrative_output {
            result.add_narrative_issues(Self::validate_narrative(output, input));
        }

        result
    }

    // ========== Cognitive validation rules ==========

    /// Check omniscience leakage rule
    fn check_omniscience_leakage(
        output: &CharacterCognitivePassOutput,
        input: &CharacterCognitivePassInput,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        let visible_entities: HashSet<&str> = input
            .filtered_scene_view
            .observable_entities
            .iter()
            .map(|entity| entity.entity_id.as_str())
            .chain(std::iter::once(input.character_id.as_str()))
            .collect();
        let visible_refs: HashSet<&str> = input
            .accessible_knowledge
            .entries
            .iter()
            .map(|entry| entry.knowledge_id.as_str())
            .chain(
                input
                    .recent_event_delta
                    .iter()
                    .map(|event| event.event_id.as_str()),
            )
            .collect();

        for (index, target_ref) in output.intent_plan.target_refs.iter().enumerate() {
            if !visible_entities.contains(target_ref.as_str()) {
                issues.push(Self::issue(
                    "omniscience_leakage",
                    ValidationSeverity::Error,
                    format!("intent target_ref '{}' is not observable", target_ref),
                    Some(format!("intent_plan.target_refs[{}]", index)),
                ));
            }
        }

        for (action_index, action) in output.intent_plan.intended_actions.iter().enumerate() {
            for (target_index, target_ref) in action.target_refs.iter().enumerate() {
                if !visible_entities.contains(target_ref.as_str()) {
                    issues.push(Self::issue(
                        "omniscience_leakage",
                        ValidationSeverity::Error,
                        format!("action target_ref '{}' is not observable", target_ref),
                        Some(format!(
                            "intent_plan.intended_actions[{}].target_refs[{}]",
                            action_index, target_index
                        )),
                    ));
                }
            }
        }

        for (hypothesis_index, hypothesis) in output.belief_update.new_hypotheses.iter().enumerate()
        {
            for (ref_index, evidence_ref) in hypothesis.evidence_refs.iter().enumerate() {
                if !visible_refs.contains(evidence_ref.as_str()) {
                    issues.push(Self::issue(
                        "omniscience_leakage",
                        ValidationSeverity::Warning,
                        format!(
                            "evidence_ref '{}' is not in accessible knowledge or recent events",
                            evidence_ref
                        ),
                        Some(format!(
                            "belief_update.new_hypotheses[{}].evidence_refs[{}]",
                            hypothesis_index, ref_index
                        )),
                    ));
                }
            }
        }

        issues
    }

    /// Check embodiment ignored rule
    fn check_embodiment_ignored(
        output: &CharacterCognitivePassOutput,
        input: &CharacterCognitivePassInput,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        let text = Self::output_text(output);
        let sensory = &input.embodiment_state.sensory_capabilities;
        if sensory.vision.availability <= 0.05
            && Self::contains_any(
                &text,
                &["saw", "seen", "visual", "looked", "看见", "看到", "目睹"],
            )
        {
            issues.push(Self::issue(
                "embodiment_ignored",
                ValidationSeverity::Error,
                "output describes visual perception while vision is unavailable",
                Some("perception_delta".to_string()),
            ));
        }
        if sensory.hearing.availability <= 0.05
            && Self::contains_any(&text, &["heard", "sound", "voice", "听见", "听到", "声音"])
        {
            issues.push(Self::issue(
                "embodiment_ignored",
                ValidationSeverity::Error,
                "output describes auditory perception while hearing is unavailable",
                Some("perception_delta".to_string()),
            ));
        }
        if sensory.smell.availability <= 0.05
            && Self::contains_any(&text, &["smell", "scent", "odor", "闻到", "气味"])
        {
            issues.push(Self::issue(
                "embodiment_ignored",
                ValidationSeverity::Error,
                "output describes olfactory perception while smell is unavailable",
                Some("perception_delta".to_string()),
            ));
        }

        issues
    }

    /// Check self awareness rule
    fn check_self_awareness(
        output: &CharacterCognitivePassOutput,
        input: &CharacterCognitivePassInput,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        if output.intent_plan.character_id != input.character_id {
            issues.push(Self::issue(
                "self_awareness",
                ValidationSeverity::Critical,
                format!(
                    "intent_plan.character_id '{}' does not match input character_id '{}'",
                    output.intent_plan.character_id, input.character_id
                ),
                Some("intent_plan.character_id".to_string()),
            ));
        }
        if let Some(reaction) = &output.body_reaction_delta {
            if reaction.character_id != input.character_id {
                issues.push(Self::issue(
                    "self_awareness",
                    ValidationSeverity::Critical,
                    format!(
                        "body_reaction_delta.character_id '{}' does not match input character_id '{}'",
                        reaction.character_id, input.character_id
                    ),
                    Some("body_reaction_delta.character_id".to_string()),
                ));
            }
        }
        if input.filtered_scene_view.character_id != input.character_id
            || input.embodiment_state.character_id != input.character_id
            || input.accessible_knowledge.character_id != input.character_id
        {
            issues.push(Self::issue(
                "self_awareness",
                ValidationSeverity::Critical,
                "cognitive input contains mismatched character_id across filtered scene, embodiment, or knowledge",
                None,
            ));
        }

        issues
    }

    /// Check God only rule
    fn check_god_only(
        output: &CharacterCognitivePassOutput,
        input: &CharacterCognitivePassInput,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        let visible_refs: HashSet<&str> = input
            .accessible_knowledge
            .entries
            .iter()
            .map(|entry| entry.knowledge_id.as_str())
            .collect();

        for (index, belief_ref) in output
            .belief_update
            .decision_relevant_beliefs
            .iter()
            .enumerate()
        {
            if belief_ref.starts_with("god_only:")
                || (belief_ref.starts_with("knowledge:")
                    && !visible_refs.contains(belief_ref.trim_start_matches("knowledge:")))
            {
                issues.push(Self::issue(
                    "god_only",
                    ValidationSeverity::Error,
                    format!(
                        "decision relevant belief '{}' is not accessible",
                        belief_ref
                    ),
                    Some(format!(
                        "belief_update.decision_relevant_beliefs[{}]",
                        index
                    )),
                ));
            }
        }

        issues
    }

    /// Check mana sense rule
    fn check_mana_sense(
        output: &CharacterCognitivePassOutput,
        input: &CharacterCognitivePassInput,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        let mana = &input.embodiment_state.sensory_capabilities.mana;
        if (mana.availability <= 0.05 || mana.acuity <= 0.05)
            && (!input.filtered_scene_view.mana_signals.is_empty()
                || Self::contains_any(
                    &Self::output_text(output),
                    &[
                        "mana",
                        "aura",
                        "spiritual pressure",
                        "灵力",
                        "法力",
                        "气息",
                        "威压",
                    ],
                ))
        {
            issues.push(Self::issue(
                "mana_sense",
                ValidationSeverity::Error,
                "output uses clear mana perception while mana sense is unavailable",
                Some("perception_delta".to_string()),
            ));
        }

        issues
    }

    /// Check apparent vs true rule
    fn check_apparent_vs_true(
        output: &CharacterCognitivePassOutput,
        input: &CharacterCognitivePassInput,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        let text = Self::output_text(output);
        for entry in &input.accessible_knowledge.entries {
            if matches!(entry.source_hint, AccessSource::ApparentFromObservation) {
                let true_text = Self::compact_json_text(&entry.accessible_content);
                if true_text.len() > 24 && text.contains(&true_text) {
                    issues.push(Self::issue(
                        "apparent_vs_true",
                        ValidationSeverity::Warning,
                        format!(
                            "output appears to repeat exact hidden/apparent knowledge payload '{}'",
                            entry.knowledge_id
                        ),
                        Some("accessible_knowledge".to_string()),
                    ));
                }
            }
        }

        issues
    }

    // ========== Helper functions ==========

    fn issue(
        rule: impl Into<String>,
        severity: ValidationSeverity,
        description: impl Into<String>,
        field_path: Option<String>,
    ) -> ValidationIssue {
        ValidationIssue {
            rule: rule.into(),
            severity,
            description: description.into(),
            field_path,
        }
    }

    fn output_text(output: &CharacterCognitivePassOutput) -> String {
        let mut parts = Vec::new();
        parts.extend(output.perception_delta.new_observations.iter().cloned());
        parts.extend(output.perception_delta.updated_perceptions.iter().cloned());
        parts.extend(output.perception_delta.missed_observations.iter().cloned());
        parts.extend(
            output
                .belief_update
                .new_hypotheses
                .iter()
                .map(|item| item.proposition.clone()),
        );
        parts.extend(
            output
                .belief_update
                .decision_relevant_beliefs
                .iter()
                .cloned(),
        );
        parts.push(output.intent_plan.rationale.clone());
        for action in &output.intent_plan.intended_actions {
            parts.push(action.outward_description.clone());
            if let Some(spoken) = &action.spoken_text {
                parts.push(spoken.clone());
            }
        }
        if let Some(reaction) = &output.body_reaction_delta {
            parts.push(reaction.outward_signal.clone());
        }
        parts.join("\n").to_lowercase()
    }

    fn contains_any(haystack: &str, needles: &[&str]) -> bool {
        needles
            .iter()
            .any(|needle| haystack.contains(&needle.to_lowercase()))
    }

    fn compact_json_text(value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(value) => value.to_lowercase(),
            _ => serde_json::to_string(value)
                .unwrap_or_default()
                .to_lowercase(),
        }
    }
}
