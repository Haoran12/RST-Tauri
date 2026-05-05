//! Temporal consistency validator
//!
//! Validates past timeline against TruthGuidance.
//! Implements constraint checking for RequiredOutcome, ForbiddenOutcome, and KnownAfterEffect.

use std::collections::{HashMap, HashSet};

use crate::agent::models::{
    AfterEffect, ConflictReport, ConflictSeverity, EventOutcome, HistoricalEventContent,
    OutcomeDomain, ProvisionalTruthCandidate, TimeAnchor, TruthConstraint, TruthConstraintKind,
    TruthGuidance,
};

/// Temporal consistency validator - validates past timeline
pub struct TemporalConsistencyValidator;

impl TemporalConsistencyValidator {
    /// Validate provisional truth candidates against truth guidance
    pub fn validate(
        candidates: &[ProvisionalTruthCandidate],
        truth_guidance: &TruthGuidance,
    ) -> Result<Vec<ConflictReport>, String> {
        let mut conflicts = Vec::new();

        for candidate in candidates {
            // Check against hard constraints
            for constraint in &truth_guidance.hard_constraints {
                if let Some(conflict) = Self::check_constraint_violation(candidate, constraint) {
                    conflicts.push(conflict);
                }
            }
        }

        Ok(conflicts)
    }

    /// Check if a candidate violates a constraint, returning a ConflictReport if so
    fn check_constraint_violation(
        candidate: &ProvisionalTruthCandidate,
        constraint: &TruthConstraint,
    ) -> Option<ConflictReport> {
        let violates = match constraint.constraint_kind {
            TruthConstraintKind::RequiredOutcome => {
                Self::violates_required_outcome(candidate, constraint)
            }
            TruthConstraintKind::ForbiddenOutcome => {
                Self::violates_forbidden_outcome(candidate, constraint)
            }
            TruthConstraintKind::KnownAfterEffect => {
                Self::violates_known_after_effect(candidate, constraint)
            }
        };

        if violates {
            Some(ConflictReport {
                conflict_id: crate::agent::models::generate_id("conflict"),
                session_id: candidate.source_session_id.clone(),
                session_turn_id: candidate.source_session_turn_id.clone(),
                scene_turn_id: candidate.source_scene_turn_id.clone(),
                severity: ConflictSeverity::Hard,
                source_constraint_ids: vec![constraint.constraint_id.clone()],
                affected_provisional_ids: vec![candidate.provisional_id.clone()],
                policy_decision: None,
                summary: serde_json::json!({
                    "message": format!("Candidate violates constraint: {}", constraint.constraint_id),
                    "constraint_kind": format!("{:?}", constraint.constraint_kind),
                    "constraint_source": constraint.source_knowledge_id,
                }),
                created_at: chrono::Utc::now(),
                resolved_at: None,
            })
        } else {
            None
        }
    }

    /// Check if a candidate violates a RequiredOutcome constraint
    ///
    /// RequiredOutcome: The candidate must produce or be consistent with the required outcome.
    /// Violation occurs when the candidate explicitly contradicts or negates the required outcome.
    fn violates_required_outcome(
        candidate: &ProvisionalTruthCandidate,
        constraint: &TruthConstraint,
    ) -> bool {
        let payload = &constraint.structured_payload;

        // Extract outcome details from constraint
        let required_domain = payload.get("domain").and_then(|v| v.as_str());
        let required_subject = payload.get("subject_id").and_then(|v| v.as_str());
        let required_target = payload.get("target_id").and_then(|v| v.as_str());
        let required_description = payload.get("description").and_then(|v| v.as_str());

        // Check if candidate explicitly negates the required outcome
        if let Some(candidate_outcomes) =
            candidate.payload.get("outcomes").and_then(|v| v.as_array())
        {
            for outcome in candidate_outcomes {
                // Check for explicit negation
                if let Some(negates) = outcome.get("negates_outcome_id").and_then(|v| v.as_str()) {
                    if negates == constraint.constraint_id {
                        return true;
                    }
                }

                // Check for contradictory outcome
                if Self::is_contradictory_outcome(
                    outcome,
                    required_domain,
                    required_subject,
                    required_target,
                    required_description,
                ) {
                    return true;
                }
            }
        }

        // Check if candidate's state changes contradict required outcome
        if let Some(state_changes) = candidate
            .payload
            .get("state_changes")
            .and_then(|v| v.as_array())
        {
            for change in state_changes {
                if Self::contradicts_required_state(change, payload) {
                    return true;
                }
            }
        }

        // Check if candidate marks required outcome as "prevented" or "failed"
        if let Some(prevented) = candidate
            .payload
            .get("prevented_outcomes")
            .and_then(|v| v.as_array())
        {
            for prevented_id in prevented {
                if prevented_id.as_str() == Some(&constraint.constraint_id) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a candidate violates a ForbiddenOutcome constraint
    ///
    /// ForbiddenOutcome: The candidate must not produce the forbidden outcome.
    /// Violation occurs when the candidate explicitly produces or enables the forbidden outcome.
    fn violates_forbidden_outcome(
        candidate: &ProvisionalTruthCandidate,
        constraint: &TruthConstraint,
    ) -> bool {
        let payload = &constraint.structured_payload;

        let forbidden_domain = payload.get("domain").and_then(|v| v.as_str());
        let forbidden_subject = payload.get("subject_id").and_then(|v| v.as_str());
        let forbidden_target = payload.get("target_id").and_then(|v| v.as_str());
        let forbidden_description = payload.get("description").and_then(|v| v.as_str());

        // Check if candidate produces the forbidden outcome
        if let Some(candidate_outcomes) =
            candidate.payload.get("outcomes").and_then(|v| v.as_array())
        {
            for outcome in candidate_outcomes {
                if Self::matches_forbidden_outcome(
                    outcome,
                    forbidden_domain,
                    forbidden_subject,
                    forbidden_target,
                    forbidden_description,
                ) {
                    return true;
                }
            }
        }

        // Check if candidate's state changes produce forbidden outcome
        if let Some(state_changes) = candidate
            .payload
            .get("state_changes")
            .and_then(|v| v.as_array())
        {
            for change in state_changes {
                if Self::produces_forbidden_state(change, payload) {
                    return true;
                }
            }
        }

        // Check if candidate explicitly enables a forbidden outcome
        if let Some(enabled) = candidate
            .payload
            .get("enabled_outcomes")
            .and_then(|v| v.as_array())
        {
            for enabled_id in enabled {
                if enabled_id.as_str() == Some(&constraint.constraint_id) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a candidate violates a KnownAfterEffect constraint
    ///
    /// KnownAfterEffect: The candidate must be consistent with facts that become true after an event.
    /// Violation occurs when the candidate contradicts established after-effects.
    fn violates_known_after_effect(
        candidate: &ProvisionalTruthCandidate,
        constraint: &TruthConstraint,
    ) -> bool {
        let payload = &constraint.structured_payload;

        // Extract after-effect details
        let fact_ref = payload.get("fact_ref").and_then(|v| v.as_str());
        let valid_from_ordinal = payload.get("valid_from_ordinal").and_then(|v| v.as_i64());

        // Check time consistency
        if let (Some(valid_from), Some(candidate_time)) = (
            valid_from_ordinal,
            candidate
                .payload
                .get("story_time_ordinal")
                .and_then(|v| v.as_i64()),
        ) {
            // Candidate occurs before the after-effect becomes valid
            // This is only a violation if the candidate claims to affect the after-effect
            if candidate_time < valid_from {
                // Check if candidate tries to modify the after-effect fact
                if let Some(affected_facts) = candidate
                    .payload
                    .get("affected_fact_refs")
                    .and_then(|v| v.as_array())
                {
                    for fact in affected_facts {
                        if fact.as_str() == fact_ref {
                            return true;
                        }
                    }
                }
            }
        }

        // Check if candidate contradicts the after-effect content
        if let Some(after_effect_content) = payload.get("after_effect_content") {
            if let Some(candidate_facts) = candidate
                .payload
                .get("established_facts")
                .and_then(|v| v.as_array())
            {
                for fact in candidate_facts {
                    if Self::contradicts_after_effect(fact, after_effect_content) {
                        return true;
                    }
                }
            }
        }

        // Check for explicit contradiction markers
        if let Some(contradicts) = candidate
            .payload
            .get("contradicts_facts")
            .and_then(|v| v.as_array())
        {
            for fact_id in contradicts {
                if fact_id.as_str() == fact_ref {
                    return true;
                }
            }
        }

        false
    }

    /// Check if an outcome contradicts a required outcome
    fn is_contradictory_outcome(
        outcome: &serde_json::Value,
        required_domain: Option<&str>,
        required_subject: Option<&str>,
        required_target: Option<&str>,
        required_description: Option<&str>,
    ) -> bool {
        // Must match domain, subject, and target to be potentially contradictory
        let domain_matches = required_domain.map_or(false, |rd| {
            outcome.get("domain").and_then(|v| v.as_str()) == Some(rd)
        });

        let subject_matches = required_subject.map_or(false, |rs| {
            outcome.get("subject_id").and_then(|v| v.as_str()) == Some(rs)
        });

        let target_matches = required_target.map_or(true, |rt| {
            outcome.get("target_id").and_then(|v| v.as_str()) == Some(rt)
        });

        if domain_matches && subject_matches && target_matches {
            // Check if outcome description is contradictory
            if let (Some(outcome_desc), Some(req_desc)) = (
                outcome.get("description").and_then(|v| v.as_str()),
                required_description,
            ) {
                // Simple contradiction detection via negation keywords
                Self::is_negation_of(outcome_desc, req_desc)
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Check if a state change contradicts required state
    fn contradicts_required_state(
        change: &serde_json::Value,
        required_payload: &serde_json::Value,
    ) -> bool {
        let required_subject = required_payload.get("subject_id").and_then(|v| v.as_str());
        let required_domain = required_payload.get("domain").and_then(|v| v.as_str());

        let change_subject = change.get("subject_id").and_then(|v| v.as_str());
        let change_domain = change.get("domain").and_then(|v| v.as_str());

        // Must affect same subject and domain
        if change_subject == required_subject && change_domain == required_domain {
            // Check for contradictory values
            if let (Some(required_value), Some(change_value)) = (
                required_payload.get("required_value"),
                change.get("new_value"),
            ) {
                return Self::values_contradict(change_value, required_value);
            }
        }

        false
    }

    /// Check if an outcome matches a forbidden outcome
    fn matches_forbidden_outcome(
        outcome: &serde_json::Value,
        forbidden_domain: Option<&str>,
        forbidden_subject: Option<&str>,
        forbidden_target: Option<&str>,
        forbidden_description: Option<&str>,
    ) -> bool {
        // Check domain match
        let domain_matches = forbidden_domain.map_or(true, |fd| {
            outcome.get("domain").and_then(|v| v.as_str()) == Some(fd)
        });

        // Check subject match
        let subject_matches = forbidden_subject.map_or(true, |fs| {
            outcome.get("subject_id").and_then(|v| v.as_str()) == Some(fs)
        });

        // Check target match (optional)
        let target_matches = forbidden_target.map_or(true, |ft| {
            outcome.get("target_id").and_then(|v| v.as_str()) == Some(ft)
        });

        // Check description similarity (fuzzy match)
        let description_matches = forbidden_description.map_or(true, |fd| {
            outcome
                .get("description")
                .and_then(|v| v.as_str())
                .map(|od| Self::descriptions_similar(od, fd))
                .unwrap_or(false)
        });

        domain_matches && subject_matches && target_matches && description_matches
    }

    /// Check if a state change produces a forbidden state
    fn produces_forbidden_state(
        change: &serde_json::Value,
        forbidden_payload: &serde_json::Value,
    ) -> bool {
        let forbidden_subject = forbidden_payload.get("subject_id").and_then(|v| v.as_str());
        let forbidden_domain = forbidden_payload.get("domain").and_then(|v| v.as_str());

        let change_subject = change.get("subject_id").and_then(|v| v.as_str());
        let change_domain = change.get("domain").and_then(|v| v.as_str());

        // Must affect same subject and domain
        if change_subject == forbidden_subject && change_domain == forbidden_domain {
            // Check if change produces the forbidden value
            if let (Some(forbidden_value), Some(change_value)) = (
                forbidden_payload.get("forbidden_value"),
                change.get("new_value"),
            ) {
                return change_value == forbidden_value;
            }
        }

        false
    }

    /// Check if a fact contradicts an after-effect
    fn contradicts_after_effect(
        fact: &serde_json::Value,
        after_effect: &serde_json::Value,
    ) -> bool {
        // Check if fact directly negates after-effect
        if let (Some(fact_key), Some(ae_key)) = (
            fact.get("key").and_then(|v| v.as_str()),
            after_effect.get("key").and_then(|v| v.as_str()),
        ) {
            if fact_key == ae_key {
                if let (Some(fact_value), Some(ae_value)) =
                    (fact.get("value"), after_effect.get("value"))
                {
                    return Self::values_contradict(fact_value, ae_value);
                }
            }
        }

        false
    }

    /// Check if two values contradict each other
    fn values_contradict(a: &serde_json::Value, b: &serde_json::Value) -> bool {
        // Direct inequality for primitives
        match (a, b) {
            (serde_json::Value::Bool(a_bool), serde_json::Value::Bool(b_bool)) => a_bool != b_bool,
            (serde_json::Value::Number(a_num), serde_json::Value::Number(b_num)) => a_num != b_num,
            (serde_json::Value::String(a_str), serde_json::Value::String(b_str)) => {
                // Check for explicit negation
                Self::is_negation_of(a_str, b_str)
            }
            _ => false,
        }
    }

    /// Check if text_a is a negation of text_b
    fn is_negation_of(text_a: &str, text_b: &str) -> bool {
        let a_lower = text_a.to_lowercase();
        let b_lower = text_b.to_lowercase();

        // Check for explicit negation patterns
        let negation_patterns = [
            ("not ", ""),
            ("never ", ""),
            ("fails to ", ""),
            ("cannot ", "can "),
            ("won't ", "will "),
            ("didn't ", "did "),
            ("doesn't ", "does "),
            ("isn't ", "is "),
            ("wasn't ", "was "),
        ];

        for (neg_prefix, pos_prefix) in negation_patterns {
            if a_lower.starts_with(neg_prefix) && b_lower.starts_with(pos_prefix) {
                let a_remainder = &a_lower[neg_prefix.len()..];
                let b_remainder = &b_lower[pos_prefix.len()..];
                if a_remainder == b_remainder {
                    return true;
                }
            }
            if b_lower.starts_with(neg_prefix) && a_lower.starts_with(pos_prefix) {
                let a_remainder = &a_lower[pos_prefix.len()..];
                let b_remainder = &b_lower[neg_prefix.len()..];
                if a_remainder == b_remainder {
                    return true;
                }
            }
        }

        false
    }

    /// Check if two descriptions are similar enough to be considered the same outcome
    fn descriptions_similar(a: &str, b: &str) -> bool {
        let a_lower = a.to_lowercase();
        let b_lower = b.to_lowercase();

        // Exact match
        if a_lower == b_lower {
            return true;
        }

        // One contains the other
        if a_lower.contains(&b_lower) || b_lower.contains(&a_lower) {
            return true;
        }

        // Word overlap ratio
        let a_words: HashSet<&str> = a_lower.split_whitespace().collect();
        let b_words: HashSet<&str> = b_lower.split_whitespace().collect();

        if a_words.is_empty() || b_words.is_empty() {
            return false;
        }

        let intersection = a_words.intersection(&b_words).count();
        let union = a_words.union(&b_words).count();

        // Jaccard similarity > 0.5
        (intersection as f64) / (union as f64) > 0.5
    }

    /// Check if a time anchor is in valid range
    pub fn is_time_valid(
        candidate_time: &TimeAnchor,
        valid_from: Option<&TimeAnchor>,
        valid_until: Option<&TimeAnchor>,
    ) -> bool {
        if let Some(from) = valid_from {
            if candidate_time.ordinal < from.ordinal {
                return false;
            }
        }
        if let Some(until) = valid_until {
            if candidate_time.ordinal > until.ordinal {
                return false;
            }
        }
        true
    }

    /// Validate candidate against historical event constraints
    pub fn validate_against_historical_event(
        candidate: &ProvisionalTruthCandidate,
        event: &HistoricalEventContent,
    ) -> Result<Vec<ConflictReport>, String> {
        let mut conflicts = Vec::new();

        // Check required outcomes
        for required in &event.required_outcomes {
            let constraint = TruthConstraint {
                constraint_id: format!("event_{}_required_{}", event.event_id, required.outcome_id),
                source_knowledge_id: event.event_id.clone(),
                constraint_kind: TruthConstraintKind::RequiredOutcome,
                applies_to_refs: vec![required.subject_id.clone()],
                structured_payload: serde_json::json!({
                    "domain": format!("{:?}", required.domain),
                    "subject_id": required.subject_id,
                    "target_id": required.target_id,
                    "description": required.description,
                }),
            };

            if let Some(conflict) = Self::check_constraint_violation(candidate, &constraint) {
                conflicts.push(conflict);
            }
        }

        // Check forbidden outcomes
        for forbidden in &event.forbidden_outcomes {
            let constraint = TruthConstraint {
                constraint_id: format!(
                    "event_{}_forbidden_{}",
                    event.event_id, forbidden.outcome_id
                ),
                source_knowledge_id: event.event_id.clone(),
                constraint_kind: TruthConstraintKind::ForbiddenOutcome,
                applies_to_refs: vec![forbidden.subject_id.clone()],
                structured_payload: serde_json::json!({
                    "domain": format!("{:?}", forbidden.domain),
                    "subject_id": forbidden.subject_id,
                    "target_id": forbidden.target_id,
                    "description": forbidden.description,
                }),
            };

            if let Some(conflict) = Self::check_constraint_violation(candidate, &constraint) {
                conflicts.push(conflict);
            }
        }

        // Check known after-effects
        for after_effect in &event.known_after_effects {
            let constraint = TruthConstraint {
                constraint_id: format!(
                    "event_{}_aftereffect_{}",
                    event.event_id, after_effect.fact_ref
                ),
                source_knowledge_id: event.event_id.clone(),
                constraint_kind: TruthConstraintKind::KnownAfterEffect,
                applies_to_refs: vec![after_effect.fact_ref.clone()],
                structured_payload: serde_json::json!({
                    "fact_ref": after_effect.fact_ref,
                    "valid_from_ordinal": after_effect.valid_from_ordinal,
                }),
            };

            if let Some(conflict) = Self::check_constraint_violation(candidate, &constraint) {
                conflicts.push(conflict);
            }
        }

        Ok(conflicts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::models::{generate_id, TruthGuidance};

    fn make_test_candidate(payload: serde_json::Value) -> ProvisionalTruthCandidate {
        ProvisionalTruthCandidate {
            provisional_id: generate_id("prov"),
            source_session_id: "test_session".to_string(),
            source_session_turn_id: "test_turn".to_string(),
            source_scene_turn_id: Some("test_scene_turn".to_string()),
            source_kind: "test".to_string(),
            payload,
            confidence: 0.8,
            constraints: Vec::new(),
        }
    }

    fn make_test_constraint(
        kind: TruthConstraintKind,
        payload: serde_json::Value,
    ) -> TruthConstraint {
        TruthConstraint {
            constraint_id: generate_id("constraint"),
            source_knowledge_id: "test_event".to_string(),
            constraint_kind: kind,
            applies_to_refs: vec!["char_1".to_string()],
            structured_payload: payload,
        }
    }

    #[test]
    fn detects_required_outcome_violation() {
        let candidate = make_test_candidate(serde_json::json!({
            "prevented_outcomes": ["constraint_test"],
        }));

        let constraint = make_test_constraint(
            TruthConstraintKind::RequiredOutcome,
            serde_json::json!({
                "domain": "CharacterLifeState",
                "subject_id": "char_1",
                "description": "survives the battle",
            }),
        );
        // Manually set constraint_id to match
        let constraint = TruthConstraint {
            constraint_id: "constraint_test".to_string(),
            ..constraint
        };

        let guidance = TruthGuidance {
            session_id: "test".to_string(),
            period_anchor: TimeAnchor {
                calendar_id: "default".to_string(),
                ordinal: 100,
                precision: crate::agent::models::TimePrecision::Day,
                display_text: "Day 100".to_string(),
            },
            related_event_ids: vec!["event_1".to_string()],
            hard_constraints: vec![constraint],
            soft_context: Vec::new(),
            open_detail_slots: Vec::new(),
            future_knowledge_warnings: Vec::new(),
        };

        let conflicts = TemporalConsistencyValidator::validate(&[candidate], &guidance).unwrap();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].severity, ConflictSeverity::Hard);
    }

    #[test]
    fn detects_forbidden_outcome_violation() {
        let candidate = make_test_candidate(serde_json::json!({
            "outcomes": [{
                "domain": "CharacterLifeState",
                "subject_id": "char_1",
                "description": "dies in battle",
            }],
        }));

        let constraint = make_test_constraint(
            TruthConstraintKind::ForbiddenOutcome,
            serde_json::json!({
                "domain": "CharacterLifeState",
                "subject_id": "char_1",
                "description": "dies in battle",
            }),
        );

        let guidance = TruthGuidance {
            session_id: "test".to_string(),
            period_anchor: TimeAnchor {
                calendar_id: "default".to_string(),
                ordinal: 100,
                precision: crate::agent::models::TimePrecision::Day,
                display_text: "Day 100".to_string(),
            },
            related_event_ids: vec!["event_1".to_string()],
            hard_constraints: vec![constraint],
            soft_context: Vec::new(),
            open_detail_slots: Vec::new(),
            future_knowledge_warnings: Vec::new(),
        };

        let conflicts = TemporalConsistencyValidator::validate(&[candidate], &guidance).unwrap();
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn allows_consistent_candidates() {
        let candidate = make_test_candidate(serde_json::json!({
            "outcomes": [{
                "domain": "Relationship",
                "subject_id": "char_1",
                "target_id": "char_2",
                "description": "forms alliance",
            }],
        }));

        let constraint = make_test_constraint(
            TruthConstraintKind::RequiredOutcome,
            serde_json::json!({
                "domain": "CharacterLifeState",
                "subject_id": "char_3",
                "description": "survives",
            }),
        );

        let guidance = TruthGuidance {
            session_id: "test".to_string(),
            period_anchor: TimeAnchor {
                calendar_id: "default".to_string(),
                ordinal: 100,
                precision: crate::agent::models::TimePrecision::Day,
                display_text: "Day 100".to_string(),
            },
            related_event_ids: vec!["event_1".to_string()],
            hard_constraints: vec![constraint],
            soft_context: Vec::new(),
            open_detail_slots: Vec::new(),
            future_knowledge_warnings: Vec::new(),
        };

        let conflicts = TemporalConsistencyValidator::validate(&[candidate], &guidance).unwrap();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn time_validation_works() {
        let candidate_time = TimeAnchor {
            calendar_id: "default".to_string(),
            ordinal: 150,
            precision: crate::agent::models::TimePrecision::Day,
            display_text: "Day 150".to_string(),
        };

        let valid_from = TimeAnchor {
            calendar_id: "default".to_string(),
            ordinal: 100,
            precision: crate::agent::models::TimePrecision::Day,
            display_text: "Day 100".to_string(),
        };

        let valid_until = TimeAnchor {
            calendar_id: "default".to_string(),
            ordinal: 200,
            precision: crate::agent::models::TimePrecision::Day,
            display_text: "Day 200".to_string(),
        };

        assert!(TemporalConsistencyValidator::is_time_valid(
            &candidate_time,
            Some(&valid_from),
            Some(&valid_until)
        ));
        assert!(!TemporalConsistencyValidator::is_time_valid(
            &candidate_time,
            Some(&valid_until),
            None
        ));
        assert!(!TemporalConsistencyValidator::is_time_valid(
            &candidate_time,
            None,
            Some(&valid_from)
        ));
    }
}
