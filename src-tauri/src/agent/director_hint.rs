//! Director Hint - outcome bias and style override for Director mode
//!
//! Director hints allow the player (in Director mode) to influence:
//! - Outcome planning: bias towards certain results
//! - Narrative style: override tone, pacing, focus
//!
//! Director hints are NOT commands - they are suggestions that the runtime
//! considers but does not have to follow if they conflict with:
//! - Character cognition
//! - Physical/skill constraints
//! - Canon truth
//! - Hidden facts

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Director hint input from player
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorHint {
    /// Unique hint ID
    pub hint_id: String,
    /// Outcome bias suggestions
    pub outcome_bias: Option<OutcomeBias>,
    /// Style override suggestions
    pub style_override: Option<StyleOverride>,
    /// Target scope for this hint
    pub target_scope: HintTargetScope,
    /// Priority level (higher = more important)
    pub priority: i32,
    /// Whether this hint is mandatory (will fail if cannot apply)
    pub mandatory: bool,
    /// Notes from the director
    pub notes: Option<String>,
}

/// Outcome bias - influence the result planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeBias {
    /// Suggested outcome direction
    pub direction: Option<OutcomeDirection>,
    /// Characters to favor
    pub favor_characters: Vec<String>,
    /// Characters to disfavor
    pub disfavor_characters: Vec<String>,
    /// Suggested events to occur
    pub suggested_events: Vec<SuggestedEvent>,
    /// Events to avoid
    pub avoid_events: Vec<String>,
    /// Desired tension level (0.0 = relaxed, 1.0 = maximum tension)
    pub tension_level: Option<f64>,
    /// Desired pacing (slow, normal, fast)
    pub pacing: Option<PacingHint>,
}

/// Outcome direction hint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutcomeDirection {
    /// Things should go well for the protagonists
    Positive,
    /// Things should go poorly
    Negative,
    /// Unexpected twist
    Twist,
    /// Bittersweet outcome
    Bittersweet,
    /// Ambiguous outcome
    Ambiguous,
}

/// Suggested event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedEvent {
    /// Event description (for LLM to interpret)
    pub description: String,
    /// Target characters
    pub target_characters: Vec<String>,
    /// Importance (0.0-1.0)
    pub importance: f64,
    /// Whether this is mandatory
    pub mandatory: bool,
}

/// Pacing hint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PacingHint {
    Slow,
    Normal,
    Fast,
}

/// Style override - influence narrative presentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleOverride {
    /// Tone hints
    pub tone: Option<ToneHint>,
    /// Perspective hints
    pub perspective: Option<PerspectiveHint>,
    /// Detail level
    pub detail_level: Option<DetailLevel>,
    /// Focus areas
    pub focus_areas: Vec<FocusArea>,
    /// Things to de-emphasize
    pub de_emphasize: Vec<String>,
    /// Narrative voice hints
    pub voice_hints: Option<String>,
    /// Formatting hints (e.g., more dialogue, more description)
    pub formatting_hints: Vec<FormattingHint>,
}

/// Tone hint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToneHint {
    Serious,
    Lighthearted,
    Dramatic,
    Mysterious,
    Romantic,
    Tense,
    Melancholic,
    Humorous,
}

/// Perspective hint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PerspectiveHint {
    /// First person (from a character's view)
    FirstPerson,
    /// Third person limited (one character's view)
    ThirdPersonLimited,
    /// Third person omniscient
    ThirdPersonOmniscient,
    /// Cinematic (external observer)
    Cinematic,
}

/// Detail level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetailLevel {
    Minimal,
    Normal,
    Detailed,
    Exhaustive,
}

/// Focus area for narrative
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusArea {
    /// What to focus on
    pub focus: String,
    /// Importance weight
    pub weight: f64,
}

/// Formatting hint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormattingHint {
    MoreDialogue,
    LessDialogue,
    MoreDescription,
    LessDescription,
    MoreAction,
    LessAction,
    ShorterSentences,
    LongerSentences,
    MoreInternalThought,
    LessInternalThought,
}

/// Target scope for a director hint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HintTargetScope {
    /// Target characters (empty = all)
    pub target_characters: Vec<String>,
    /// Target locations (empty = all)
    pub target_locations: Vec<String>,
    /// Target events (empty = all)
    pub target_events: Vec<String>,
    /// Turn scope (how many turns this hint applies to)
    pub turn_scope: TurnScope,
}

/// Turn scope for a hint
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnScope {
    /// Only this turn
    ThisTurn,
    /// Next N turns
    NextTurns(u32),
    /// Until explicitly cancelled
    UntilCancelled,
    /// Until a condition is met
    UntilCondition(String),
}

/// Director hint collection for a turn
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DirectorHintCollection {
    /// All active hints
    pub hints: Vec<DirectorHint>,
    /// Compiled outcome bias (merged from all hints)
    pub compiled_outcome_bias: Option<OutcomeBias>,
    /// Compiled style override (merged from all hints)
    pub compiled_style_override: Option<StyleOverride>,
}

impl DirectorHintCollection {
    /// Create an empty collection
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a hint
    pub fn add_hint(&mut self, hint: DirectorHint) {
        self.hints.push(hint);
        self.recompile();
    }

    /// Remove a hint by ID
    pub fn remove_hint(&mut self, hint_id: &str) {
        self.hints.retain(|h| h.hint_id != hint_id);
        self.recompile();
    }

    /// Clear all hints
    pub fn clear(&mut self) {
        self.hints.clear();
        self.compiled_outcome_bias = None;
        self.compiled_style_override = None;
    }

    /// Recompile the merged hints
    fn recompile(&mut self) {
        // Sort by priority (higher first)
        self.hints.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Merge outcome biases
        let mut outcome_bias = OutcomeBias {
            direction: None,
            favor_characters: Vec::new(),
            disfavor_characters: Vec::new(),
            suggested_events: Vec::new(),
            avoid_events: Vec::new(),
            tension_level: None,
            pacing: None,
        };

        for hint in &self.hints {
            if let Some(bias) = &hint.outcome_bias {
                // Take first non-None direction
                if outcome_bias.direction.is_none() {
                    outcome_bias.direction = bias.direction;
                }
                // Merge character lists
                for c in &bias.favor_characters {
                    if !outcome_bias.favor_characters.contains(c) {
                        outcome_bias.favor_characters.push(c.clone());
                    }
                }
                for c in &bias.disfavor_characters {
                    if !outcome_bias.disfavor_characters.contains(c) {
                        outcome_bias.disfavor_characters.push(c.clone());
                    }
                }
                // Merge events
                for event in &bias.suggested_events {
                    outcome_bias.suggested_events.push(event.clone());
                }
                for event in &bias.avoid_events {
                    if !outcome_bias.avoid_events.contains(event) {
                        outcome_bias.avoid_events.push(event.clone());
                    }
                }
                // Take first non-None tension and pacing
                if outcome_bias.tension_level.is_none() {
                    outcome_bias.tension_level = bias.tension_level;
                }
                if outcome_bias.pacing.is_none() {
                    outcome_bias.pacing = bias.pacing;
                }
            }
        }

        self.compiled_outcome_bias = if outcome_bias.direction.is_some()
            || !outcome_bias.favor_characters.is_empty()
            || !outcome_bias.disfavor_characters.is_empty()
            || !outcome_bias.suggested_events.is_empty()
            || !outcome_bias.avoid_events.is_empty()
        {
            Some(outcome_bias)
        } else {
            None
        };

        // Merge style overrides
        let mut style_override = StyleOverride {
            tone: None,
            perspective: None,
            detail_level: None,
            focus_areas: Vec::new(),
            de_emphasize: Vec::new(),
            voice_hints: None,
            formatting_hints: Vec::new(),
        };

        for hint in &self.hints {
            if let Some(style) = &hint.style_override {
                // Take first non-None values
                if style_override.tone.is_none() {
                    style_override.tone = style.tone;
                }
                if style_override.perspective.is_none() {
                    style_override.perspective = style.perspective;
                }
                if style_override.detail_level.is_none() {
                    style_override.detail_level = style.detail_level;
                }
                if style_override.voice_hints.is_none() {
                    style_override.voice_hints = style.voice_hints.clone();
                }
                // Merge lists
                for focus in &style.focus_areas {
                    style_override.focus_areas.push(focus.clone());
                }
                for de in &style.de_emphasize {
                    if !style_override.de_emphasize.contains(de) {
                        style_override.de_emphasize.push(de.clone());
                    }
                }
                for fmt in &style.formatting_hints {
                    if !style_override.formatting_hints.contains(fmt) {
                        style_override.formatting_hints.push(*fmt);
                    }
                }
            }
        }

        self.compiled_style_override = if style_override.tone.is_some()
            || style_override.perspective.is_some()
            || style_override.detail_level.is_some()
            || !style_override.focus_areas.is_empty()
            || style_override.voice_hints.is_some()
            || !style_override.formatting_hints.is_empty()
        {
            Some(style_override)
        } else {
            None
        };
    }

    /// Get the compiled outcome bias
    pub fn get_outcome_bias(&self) -> Option<&OutcomeBias> {
        self.compiled_outcome_bias.as_ref()
    }

    /// Get the compiled style override
    pub fn get_style_override(&self) -> Option<&StyleOverride> {
        self.compiled_style_override.as_ref()
    }

    /// Check if there are any mandatory hints
    pub fn has_mandatory_hints(&self) -> bool {
        self.hints.iter().any(|h| h.mandatory)
    }

    /// Get mandatory hints
    pub fn get_mandatory_hints(&self) -> Vec<&DirectorHint> {
        self.hints.iter().filter(|h| h.mandatory).collect()
    }
}

/// Helper function to generate a hint ID
pub fn generate_hint_id() -> String {
    format!("hint_{}", chrono::Utc::now().timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_director_hint() {
        let hint = DirectorHint {
            hint_id: generate_hint_id(),
            outcome_bias: Some(OutcomeBias {
                direction: Some(OutcomeDirection::Positive),
                favor_characters: vec!["hero".to_string()],
                disfavor_characters: Vec::new(),
                suggested_events: Vec::new(),
                avoid_events: Vec::new(),
                tension_level: Some(0.5),
                pacing: Some(PacingHint::Normal),
            }),
            style_override: None,
            target_scope: HintTargetScope {
                target_characters: Vec::new(),
                target_locations: Vec::new(),
                target_events: Vec::new(),
                turn_scope: TurnScope::ThisTurn,
            },
            priority: 1,
            mandatory: false,
            notes: None,
        };

        assert!(hint.outcome_bias.is_some());
    }

    #[test]
    fn hint_collection_merges_hints() {
        let mut collection = DirectorHintCollection::new();

        collection.add_hint(DirectorHint {
            hint_id: "hint1".to_string(),
            outcome_bias: Some(OutcomeBias {
                direction: Some(OutcomeDirection::Positive),
                favor_characters: vec!["hero".to_string()],
                disfavor_characters: Vec::new(),
                suggested_events: Vec::new(),
                avoid_events: Vec::new(),
                tension_level: None,
                pacing: None,
            }),
            style_override: None,
            target_scope: HintTargetScope {
                target_characters: Vec::new(),
                target_locations: Vec::new(),
                target_events: Vec::new(),
                turn_scope: TurnScope::ThisTurn,
            },
            priority: 1,
            mandatory: false,
            notes: None,
        });

        collection.add_hint(DirectorHint {
            hint_id: "hint2".to_string(),
            outcome_bias: Some(OutcomeBias {
                direction: Some(OutcomeDirection::Twist),
                favor_characters: vec!["villain".to_string()],
                disfavor_characters: Vec::new(),
                suggested_events: Vec::new(),
                avoid_events: Vec::new(),
                tension_level: Some(0.8),
                pacing: Some(PacingHint::Fast),
            }),
            style_override: None,
            target_scope: HintTargetScope {
                target_characters: Vec::new(),
                target_locations: Vec::new(),
                target_events: Vec::new(),
                turn_scope: TurnScope::ThisTurn,
            },
            priority: 2, // Higher priority
            mandatory: false,
            notes: None,
        });

        let bias = collection.get_outcome_bias().unwrap();
        // Higher priority hint's direction should be taken
        assert_eq!(bias.direction, Some(OutcomeDirection::Twist));
        // Both characters should be in favor list
        assert!(bias.favor_characters.contains(&"hero".to_string()));
        assert!(bias.favor_characters.contains(&"villain".to_string()));
        // Higher priority tension and pacing
        assert_eq!(bias.tension_level, Some(0.8));
        assert_eq!(bias.pacing, Some(PacingHint::Fast));
    }
}
