//! Input Preparser for Agent mode
//!
//! Lightweight parsing of user input before it reaches SceneStateExtractor.
//! Segments input into Plain / Quoted / InnerThought / DirectorBlock / Command.
//!
//! This is NOT semantic parsing - it only identifies structural markers.
//! Semantic interpretation is left to SceneStateExtractor.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Preparsed segment type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentKind {
    /// Plain text (no special markers)
    Plain,
    /// Quoted text (dialogue candidate)
    Quoted,
    /// Inner thought (*...* or similar markers)
    InnerThought,
    /// Director block ([[...]])
    DirectorBlock,
    /// Meta command (/command args)
    Command,
}

/// A preparsed segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparsedSegment {
    pub kind: SegmentKind,
    pub text: String,
    /// Start position in original input
    pub start: usize,
    /// End position in original input
    pub end: usize,
}

/// Preparsed user input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparsedUserInput {
    pub original_text: String,
    pub segments: Vec<PreparsedSegment>,
    /// Whether the input starts with / (command mode)
    pub is_command_mode: bool,
    /// Parsed command if is_command_mode
    pub command: Option<ParsedCommand>,
    /// Warnings generated during parsing
    pub warnings: Vec<String>,
}

/// Parsed meta command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCommand {
    pub command: String,
    pub args: Vec<String>,
    pub raw: String,
}

/// Input preparser
pub struct InputPreparser {
    /// Quote markers to recognize
    quote_markers: Vec<(char, char)>,
    /// Inner thought markers
    thought_markers: Vec<(String, String)>,
    /// Director block markers
    director_markers: Vec<(String, String)>,
}

impl Default for InputPreparser {
    fn default() -> Self {
        Self::new()
    }
}

impl InputPreparser {
    /// Create a new preparser with default markers
    pub fn new() -> Self {
        Self {
            quote_markers: vec![('"', '"'), ('「', '」'), ('『', '』')],
            thought_markers: vec![("*".to_string(), "*".to_string())],
            director_markers: vec![("[[".to_string(), "]]".to_string())],
        }
    }

    /// Parse user input into segments
    pub fn parse(&self, text: &str) -> PreparsedUserInput {
        let trimmed = text.trim_start();

        // Check for command mode first
        if trimmed.starts_with('/') {
            return self.parse_command(text);
        }

        let mut segments = Vec::new();
        let mut warnings = Vec::new();
        let mut pos = 0;
        let chars: Vec<char> = text.chars().collect();

        while pos < chars.len() {
            // Skip whitespace
            while pos < chars.len() && chars[pos].is_whitespace() {
                pos += 1;
            }
            if pos >= chars.len() {
                break;
            }

            // Try to match each pattern
            let remaining: String = chars[pos..].iter().collect();

            // Try director block first (highest priority)
            if let Some((segment, end)) = self.try_director_block(&remaining, pos) {
                segments.push(segment);
                pos = end;
                continue;
            }

            // Try inner thought
            if let Some((segment, end)) = self.try_inner_thought(&remaining, pos) {
                segments.push(segment);
                pos = end;
                continue;
            }

            // Try quoted text
            if let Some((segment, end)) = self.try_quoted(&remaining, pos) {
                segments.push(segment);
                pos = end;
                continue;
            }

            // Default to plain text - read until next special marker or end
            let (segment, end) = self.read_plain_text(&remaining, pos);

            // If segment is empty and end == pos, we need to consume at least one character
            // to avoid infinite loop (this happens with unclosed markers)
            if segment.text.is_empty() && end == pos {
                // Consume the rest of the text as plain
                let remaining_text: String = chars[pos..].iter().collect();
                if !remaining_text.is_empty() {
                    segments.push(PreparsedSegment {
                        kind: SegmentKind::Plain,
                        text: remaining_text,
                        start: pos,
                        end: chars.len(),
                    });
                    warnings.push("Unclosed marker detected, treated as plain text".to_string());
                }
                break;
            }

            segments.push(segment);
            pos = end;
        }

        // Validate and add warnings
        self.validate_segments(&segments, &mut warnings);

        PreparsedUserInput {
            original_text: text.to_string(),
            segments,
            is_command_mode: false,
            command: None,
            warnings,
        }
    }

    /// Parse command input
    fn parse_command(&self, text: &str) -> PreparsedUserInput {
        let trimmed = text.trim();
        let mut parts = trimmed.split_whitespace();

        let command_raw = parts.next().unwrap_or("");
        let command = command_raw
            .strip_prefix('/')
            .unwrap_or(command_raw)
            .to_string();
        let args: Vec<String> = parts.map(|s| s.to_string()).collect();

        let command = ParsedCommand {
            command,
            args,
            raw: trimmed.to_string(),
        };

        PreparsedUserInput {
            original_text: text.to_string(),
            segments: vec![PreparsedSegment {
                kind: SegmentKind::Command,
                text: trimmed.to_string(),
                start: 0,
                end: text.len(),
            }],
            is_command_mode: true,
            command: Some(command),
            warnings: Vec::new(),
        }
    }

    /// Try to match director block [[...]]
    fn try_director_block(&self, text: &str, start: usize) -> Option<(PreparsedSegment, usize)> {
        for (open, close) in &self.director_markers {
            if text.starts_with(open) {
                // Find closing marker
                if let Some(end_offset) = text.find(close) {
                    let content = &text[open.len()..end_offset];
                    let segment = PreparsedSegment {
                        kind: SegmentKind::DirectorBlock,
                        text: content.to_string(),
                        start,
                        end: start + end_offset + close.len(),
                    };
                    return Some((segment, start + end_offset + close.len()));
                } else {
                    // Unclosed - return as plain with warning
                    return None;
                }
            }
        }
        None
    }

    /// Try to match inner thought *...*
    fn try_inner_thought(&self, text: &str, start: usize) -> Option<(PreparsedSegment, usize)> {
        for (open, close) in &self.thought_markers {
            if text.starts_with(open) {
                // Find closing marker (skip the opening)
                let search_text = &text[open.len()..];
                if let Some(end_offset) = search_text.find(close) {
                    let content = &search_text[..end_offset];
                    let segment = PreparsedSegment {
                        kind: SegmentKind::InnerThought,
                        text: content.to_string(),
                        start,
                        end: start + open.len() + end_offset + close.len(),
                    };
                    return Some((segment, start + open.len() + end_offset + close.len()));
                } else {
                    // Unclosed - return as plain
                    return None;
                }
            }
        }
        None
    }

    /// Try to match quoted text
    fn try_quoted(&self, text: &str, start: usize) -> Option<(PreparsedSegment, usize)> {
        let first_char = text.chars().next()?;
        for (open, close) in &self.quote_markers {
            if first_char == *open {
                // Find closing quote
                let chars: Vec<char> = text.chars().collect();
                for (i, &c) in chars.iter().enumerate().skip(1) {
                    if c == *close {
                        let content: String = chars[1..i].iter().collect();
                        let segment = PreparsedSegment {
                            kind: SegmentKind::Quoted,
                            text: content,
                            start,
                            end: start + i + 1,
                        };
                        return Some((segment, start + i + 1));
                    }
                }
                // Unclosed quote
                return None;
            }
        }
        None
    }

    /// Read plain text until next special marker
    fn read_plain_text(&self, text: &str, start: usize) -> (PreparsedSegment, usize) {
        let chars: Vec<char> = text.chars().collect();

        // If text starts with a special marker, return empty and let caller handle it
        let first_char = match chars.first() {
            Some(&c) => c,
            None => {
                return (
                    PreparsedSegment {
                        kind: SegmentKind::Plain,
                        text: String::new(),
                        start,
                        end: start,
                    },
                    start,
                );
            }
        };

        // Check if first char is a special marker
        for (open, _) in &self.director_markers {
            if text.starts_with(open) {
                return (
                    PreparsedSegment {
                        kind: SegmentKind::Plain,
                        text: String::new(),
                        start,
                        end: start,
                    },
                    start,
                );
            }
        }
        for (open, _) in &self.thought_markers {
            if text.starts_with(open) {
                return (
                    PreparsedSegment {
                        kind: SegmentKind::Plain,
                        text: String::new(),
                        start,
                        end: start,
                    },
                    start,
                );
            }
        }
        for (open, _) in &self.quote_markers {
            if first_char == *open {
                return (
                    PreparsedSegment {
                        kind: SegmentKind::Plain,
                        text: String::new(),
                        start,
                        end: start,
                    },
                    start,
                );
            }
        }

        // Find the next special marker
        for (i, &c) in chars.iter().enumerate().skip(1) {
            // Check if this position starts a special marker
            let remaining: String = chars[i..].iter().collect();

            // Check for director block
            for (open, _) in &self.director_markers {
                if remaining.starts_with(open) {
                    let content: String = chars[..i].iter().collect();
                    return (
                        PreparsedSegment {
                            kind: SegmentKind::Plain,
                            text: content.trim().to_string(),
                            start,
                            end: start + i,
                        },
                        start + i,
                    );
                }
            }

            // Check for inner thought
            for (open, _) in &self.thought_markers {
                if remaining.starts_with(open) {
                    let content: String = chars[..i].iter().collect();
                    return (
                        PreparsedSegment {
                            kind: SegmentKind::Plain,
                            text: content.trim().to_string(),
                            start,
                            end: start + i,
                        },
                        start + i,
                    );
                }
            }

            // Check for quote
            for (open, _) in &self.quote_markers {
                if c == *open {
                    let content: String = chars[..i].iter().collect();
                    return (
                        PreparsedSegment {
                            kind: SegmentKind::Plain,
                            text: content.trim().to_string(),
                            start,
                            end: start + i,
                        },
                        start + i,
                    );
                }
            }
        }

        // No special markers found, return all as plain
        let content: String = chars.iter().collect();
        (
            PreparsedSegment {
                kind: SegmentKind::Plain,
                text: content.trim().to_string(),
                start,
                end: start + chars.len(),
            },
            start + chars.len(),
        )
    }

    /// Validate segments and add warnings
    fn validate_segments(&self, segments: &[PreparsedSegment], warnings: &mut Vec<String>) {
        // Check for empty segments
        for seg in segments {
            if seg.text.trim().is_empty() {
                warnings.push(format!("Empty {:?} segment found", seg.kind));
            }
        }

        // Check for unclosed markers (detected by mixing of markers in plain text)
        for seg in segments {
            if seg.kind == SegmentKind::Plain {
                for (open, close) in &self.director_markers {
                    if seg.text.contains(open) && !seg.text.contains(close) {
                        warnings.push(format!(
                            "Possible unclosed director block marker '{}'",
                            open
                        ));
                    }
                }
                for (open, close) in &self.thought_markers {
                    if seg.text.contains(open) && !seg.text.contains(close) {
                        warnings.push(format!("Possible unclosed thought marker '{}'", open));
                    }
                }
            }
        }
    }
}

/// User input delta from SceneStateExtractor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInputDelta {
    /// Authority class for this input
    pub authority_class: InputAuthorityClass,
    /// Notes about authority resolution
    pub authority_notes: Vec<String>,
    /// The preparsed input
    pub preparsed: PreparsedUserInput,
    /// Interpreted content (from SceneStateExtractor)
    pub interpreted_content: Option<serde_json::Value>,
}

/// Authority class for user input
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputAuthorityClass {
    /// Character roleplay (player is roleplaying a character)
    CharacterRoleplay,
    /// Scene narration (director mode scene description)
    SceneNarration,
    /// Meta command (/scene, /back, /fork, etc.)
    MetaCommand,
    /// Director hint (outcome bias, style override)
    DirectorHint,
    /// Ambiguous (needs clarification)
    Ambiguous,
    /// Rejected (invalid for current mode)
    Rejected,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plain_text() {
        let preparser = InputPreparser::new();
        let result = preparser.parse("Hello world");

        assert!(!result.is_command_mode);
        assert_eq!(result.segments.len(), 1);
        assert_eq!(result.segments[0].kind, SegmentKind::Plain);
        assert_eq!(result.segments[0].text, "Hello world");
    }

    #[test]
    fn parse_quoted_text() {
        let preparser = InputPreparser::new();
        let result = preparser.parse("\"Hello there\" she said.");

        assert_eq!(result.segments.len(), 2);
        assert_eq!(result.segments[0].kind, SegmentKind::Quoted);
        assert_eq!(result.segments[0].text, "Hello there");
        assert_eq!(result.segments[1].kind, SegmentKind::Plain);
    }

    #[test]
    fn parse_inner_thought() {
        let preparser = InputPreparser::new();
        let result = preparser.parse("*This is a thought*");

        assert_eq!(result.segments.len(), 1);
        assert_eq!(result.segments[0].kind, SegmentKind::InnerThought);
        assert_eq!(result.segments[0].text, "This is a thought");
    }

    #[test]
    fn parse_director_block() {
        let preparser = InputPreparser::new();
        let result = preparser.parse("[[Set the scene to a dark forest]]");

        assert_eq!(result.segments.len(), 1);
        assert_eq!(result.segments[0].kind, SegmentKind::DirectorBlock);
        assert_eq!(result.segments[0].text, "Set the scene to a dark forest");
    }

    #[test]
    fn parse_command() {
        let preparser = InputPreparser::new();
        let result = preparser.parse("/scene forest");

        assert!(result.is_command_mode);
        assert!(result.command.is_some());
        let cmd = result.command.unwrap();
        assert_eq!(cmd.command, "scene");
        assert_eq!(cmd.args, vec!["forest"]);
    }

    #[test]
    fn parse_mixed_input() {
        let preparser = InputPreparser::new();
        let result = preparser.parse("\"Hello!\" *she thought* [[make it darker]]");

        assert_eq!(result.segments.len(), 3);
        assert_eq!(result.segments[0].kind, SegmentKind::Quoted);
        assert_eq!(result.segments[1].kind, SegmentKind::InnerThought);
        assert_eq!(result.segments[2].kind, SegmentKind::DirectorBlock);
    }

    #[test]
    fn parse_unclosed_marker_warning() {
        let preparser = InputPreparser::new();
        let result = preparser.parse("*this is unclosed");

        // Should fall back to plain text
        assert_eq!(result.segments.len(), 1);
        assert_eq!(result.segments[0].kind, SegmentKind::Plain);
        // Should have warning about unclosed marker
        assert!(!result.warnings.is_empty());
    }
}
