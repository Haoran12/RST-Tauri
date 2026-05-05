use serde_json::Value;

use super::{
    offset_to_position, validate_value_shape, StructuredTextBinding, StructuredTextDiagnostic,
    StructuredTextDiagnosticCode, StructuredTextSeverity, StructuredTextValidationResult,
};

pub fn validate_json(
    text: &str,
    binding: &StructuredTextBinding,
) -> Result<StructuredTextValidationResult, String> {
    match serde_json::from_str::<Value>(text) {
        Ok(value) => {
            let mut diagnostics = Vec::new();
            if let Some(shape_diagnostic) = validate_value_shape(&value, binding) {
                diagnostics.push(shape_diagnostic);
            }

            Ok(StructuredTextValidationResult {
                text: text.to_string(),
                diagnostics,
                parsed_value: Some(value),
            })
        }
        Err(error) => {
            let mut diagnostics = vec![parse_error_diagnostic(text, &error.to_string())];
            let normalized = normalize_json_candidate(text);
            if normalized != text && serde_json::from_str::<Value>(&normalized).is_ok() {
                diagnostics.push(StructuredTextDiagnostic {
                    severity: StructuredTextSeverity::Info,
                    code: StructuredTextDiagnosticCode::AutoFixAvailable,
                    message: "检测到可安全修复的 JSON key 引号问题，可使用 Format 自动整理。"
                        .to_string(),
                    line: 1,
                    column: 1,
                    length: None,
                });
            }

            Ok(StructuredTextValidationResult {
                text: text.to_string(),
                diagnostics,
                parsed_value: None,
            })
        }
    }
}

pub fn format_json(
    text: &str,
    binding: &StructuredTextBinding,
) -> Result<StructuredTextValidationResult, String> {
    let raw = text.trim();
    let normalized = normalize_json_candidate(raw);
    let source = if normalized.is_empty() {
        raw
    } else {
        &normalized
    };
    let value: Value = serde_json::from_str(source)
        .map_err(|e| format!("Failed to parse JSON for formatting: {}", e))?;

    let mut diagnostics = Vec::new();
    if normalized != raw {
        diagnostics.push(StructuredTextDiagnostic {
            severity: StructuredTextSeverity::Info,
            code: StructuredTextDiagnosticCode::AutoFixApplied,
            message: "已应用安全的 JSON key 引号修复。".to_string(),
            line: 1,
            column: 1,
            length: None,
        });
    }

    if let Some(shape_diagnostic) = validate_value_shape(&value, binding) {
        diagnostics.push(shape_diagnostic);
    }

    let formatted = serde_json::to_string_pretty(&value)
        .map_err(|e| format!("Failed to format JSON: {}", e))?;

    Ok(StructuredTextValidationResult {
        text: formatted,
        diagnostics,
        parsed_value: Some(value),
    })
}

fn parse_error_diagnostic(text: &str, message: &str) -> StructuredTextDiagnostic {
    let offset = message
        .split(" at line ")
        .nth(1)
        .and_then(|rest| {
            let mut parts = rest.split(" column ");
            let line = parts.next()?.parse::<usize>().ok()?;
            let column = parts.next()?.parse::<usize>().ok()?;
            Some(position_to_offset(text, line, column))
        })
        .unwrap_or(0);
    let (line, column) = offset_to_position(text, offset);

    StructuredTextDiagnostic {
        severity: StructuredTextSeverity::Blocker,
        code: StructuredTextDiagnosticCode::ParseError,
        message: message.to_string(),
        line,
        column,
        length: Some(1),
    }
}

fn position_to_offset(text: &str, line: usize, column: usize) -> usize {
    let mut current_line = 1usize;
    let mut current_column = 1usize;

    for (index, ch) in text.char_indices() {
        if current_line == line && current_column == column {
            return index;
        }

        if ch == '\n' {
            current_line += 1;
            current_column = 1;
        } else {
            current_column += 1;
        }
    }

    text.len()
}

pub fn normalize_json_candidate(text: &str) -> String {
    let mut index = 0usize;
    let mut output = String::with_capacity(text.len());
    let mut stack: Vec<JsonContext> = Vec::new();

    while index < text.len() {
        if matches!(
            stack.last(),
            Some(JsonContext::Object {
                expecting_key: true
            })
        ) {
            if let Some(key) = read_json_like_key(text, index) {
                output.push_str(&text[index..key.start]);
                output.push('"');
                output.push_str(&escape_json_key(&key.key));
                output.push('"');
                output.push_str(&text[key.end..=key.colon_index]);
                if let Some(JsonContext::Object { expecting_key }) = stack.last_mut() {
                    *expecting_key = false;
                }
                index = key.colon_index + 1;
                continue;
            }
        }

        let Some(ch) = next_char(text, index) else {
            break;
        };

        match ch {
            '"' => {
                let end = read_json_string_end(text, index, '"');
                let safe_end = end.min(text.len());
                output.push_str(&text[index..safe_end]);
                index = safe_end;
            }
            '{' => {
                stack.push(JsonContext::Object {
                    expecting_key: true,
                });
                output.push(ch);
                index += ch.len_utf8();
            }
            '[' => {
                stack.push(JsonContext::Array);
                output.push(ch);
                index += ch.len_utf8();
            }
            '}' | ']' => {
                stack.pop();
                output.push(ch);
                index += ch.len_utf8();
            }
            ':' => {
                if let Some(JsonContext::Object { expecting_key }) = stack.last_mut() {
                    *expecting_key = false;
                }
                output.push(ch);
                index += ch.len_utf8();
            }
            ',' => {
                if let Some(JsonContext::Object { expecting_key }) = stack.last_mut() {
                    *expecting_key = true;
                }
                output.push(ch);
                index += ch.len_utf8();
            }
            _ => {
                output.push(ch);
                index += ch.len_utf8();
            }
        }
    }

    output
}

enum JsonContext {
    Object { expecting_key: bool },
    Array,
}

struct JsonLikeKey {
    start: usize,
    end: usize,
    colon_index: usize,
    key: String,
}

fn read_json_like_key(text: &str, start_index: usize) -> Option<JsonLikeKey> {
    let start = skip_json_whitespace(text, start_index);
    let first = next_char(text, start)?;

    if first == '\'' {
        let end = read_json_string_end(text, start, '\'');
        if end > text.len() || !text[..end].ends_with('\'') {
            return None;
        }

        let colon_index = skip_json_whitespace(text, end);
        if next_char(text, colon_index)? != ':' {
            return None;
        }

        return Some(JsonLikeKey {
            start,
            end,
            colon_index,
            key: unescape_single_quoted_key(&text[start + 1..end - 1]),
        });
    }

    if !is_json_bare_key_start(first) {
        return None;
    }

    let mut end = start + first.len_utf8();
    while let Some(ch) = next_char(text, end) {
        if !is_json_bare_key_part(ch) {
            break;
        }
        end += ch.len_utf8();
    }

    let colon_index = skip_json_whitespace(text, end);
    if next_char(text, colon_index)? != ':' {
        return None;
    }

    Some(JsonLikeKey {
        start,
        end,
        colon_index,
        key: text[start..end].to_string(),
    })
}

fn skip_json_whitespace(text: &str, index: usize) -> usize {
    let mut current = index;
    while let Some(ch) = next_char(text, current) {
        if !ch.is_whitespace() {
            break;
        }
        current += ch.len_utf8();
    }
    current
}

fn read_json_string_end(text: &str, start: usize, quote: char) -> usize {
    let mut escaped = false;
    let mut index = start + quote.len_utf8();

    while let Some(ch) = next_char(text, index) {
        if escaped {
            escaped = false;
            index += ch.len_utf8();
            continue;
        }

        if ch == '\\' {
            escaped = true;
            index += ch.len_utf8();
            continue;
        }

        index += ch.len_utf8();
        if ch == quote {
            return index;
        }
    }

    text.len() + 1
}

fn next_char(text: &str, index: usize) -> Option<char> {
    text.get(index..)?.chars().next()
}

fn is_json_bare_key_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_json_bare_key_part(ch: char) -> bool {
    ch == '_' || ch == '-' || ch == '.' || ch.is_alphanumeric()
}

fn unescape_single_quoted_key(key: &str) -> String {
    key.replace("\\'", "'").replace("\\\\", "\\")
}

fn escape_json_key(key: &str) -> String {
    key.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_bare_keys_inside_single_line_objects() {
        let normalized = normalize_json_candidate("{foo: 1, bar_baz: {inner-key: 2}}");
        assert_eq!(
            normalized,
            r#"{"foo": 1, "bar_baz": {"inner-key": 2}}"#
        );
        serde_json::from_str::<Value>(&normalized).unwrap();
    }

    #[test]
    fn normalizes_keys_inside_array_objects() {
        let normalized = normalize_json_candidate("[{name: \"A\"}, {'display name': 2}]");
        assert_eq!(normalized, r#"[{"name": "A"}, {"display name": 2}]"#);
        serde_json::from_str::<Value>(&normalized).unwrap();
    }

    #[test]
    fn does_not_rewrite_string_values_or_array_values() {
        let source = r#"{"text": "foo: 1", "items": [foo, "bar: 2"]}"#;
        assert_eq!(normalize_json_candidate(source), source);
    }

    #[test]
    fn format_json_preserves_object_field_order() {
        let binding = StructuredTextBinding {
            resource_kind: "generic_extensions".to_string(),
            field_path: "extensions".to_string(),
            allowed_modes: vec!["json".to_string()],
            default_mode: "json".to_string(),
            storage_kind: super::super::StructuredTextStorageKind::JsonValue,
            required_value_shape: Some(super::super::RequiredValueShape::Object),
        };

        let result = format_json(r#"{"z":1,"a":2,"nested":{"c":3,"b":4}}"#, &binding).unwrap();
        assert_eq!(
            result.text,
            "{\n  \"z\": 1,\n  \"a\": 2,\n  \"nested\": {\n    \"c\": 3,\n    \"b\": 4\n  }\n}"
        );
    }
}
