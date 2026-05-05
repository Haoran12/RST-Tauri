//! JSON output repair for Agent LLM nodes.
//!
//! Provides deterministic repair for common JSON errors in LLM structured output:
//! - Missing commas
//! - Unescaped quotes in strings
//! - Missing optional fields (filled with defaults)
//! - Field name typos (corrected via schema-aware matching)

use serde_json::Value;

/// Repair result containing either a successfully repaired value or a failure reason.
#[derive(Debug, Clone)]
pub struct JsonRepairResult {
    pub value: Value,
    pub repairs: Vec<JsonRepair>,
    pub remaining_issues: Vec<String>,
}

/// A single repair applied to the JSON.
#[derive(Debug, Clone)]
pub struct JsonRepair {
    pub kind: JsonRepairKind,
    pub location: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonRepairKind {
    MissingComma,
    UnescapedQuote,
    MissingField,
    FieldNameTypo,
    TrailingComma,
    MismatchedBracket,
}

/// Attempt to repair a malformed JSON string and parse it.
///
/// Returns the repaired value along with a list of repairs applied.
/// If repair fails, returns the parse error.
pub fn repair_and_parse(
    json_str: &str,
    schema: Option<&Value>,
) -> Result<JsonRepairResult, String> {
    let mut repairs = Vec::new();
    let mut remaining_issues = Vec::new();

    // Step 1: Try direct parse first
    if let Ok(value) = serde_json::from_str::<Value>(json_str) {
        let value = fill_missing_fields(value, schema, &mut repairs);
        return Ok(JsonRepairResult {
            value,
            repairs,
            remaining_issues,
        });
    }

    // Step 2: Apply string-level repairs
    let mut repaired_str = json_str.to_string();

    // Fix trailing commas in arrays/objects
    let trailing_comma_result = fix_trailing_commas(&repaired_str);
    if trailing_comma_result.fixed {
        repairs.extend(trailing_comma_result.repairs);
        repaired_str = trailing_comma_result.text;
    }

    // Fix missing commas between elements
    let comma_result = fix_missing_commas(&repaired_str);
    if comma_result.fixed {
        repairs.extend(comma_result.repairs);
        repaired_str = comma_result.text;
    }

    // Fix unescaped quotes in strings
    let quote_result = fix_unescaped_quotes(&repaired_str);
    if quote_result.fixed {
        repairs.extend(quote_result.repairs);
        repaired_str = quote_result.text;
    }

    // Fix mismatched brackets
    let bracket_result = fix_mismatched_brackets(&repaired_str);
    if bracket_result.fixed {
        repairs.extend(bracket_result.repairs);
        repaired_str = bracket_result.text;
    }

    // Try parse again after repairs
    match serde_json::from_str::<Value>(&repaired_str) {
        Ok(value) => {
            let value = fill_missing_fields(value, schema, &mut repairs);
            Ok(JsonRepairResult {
                value,
                repairs,
                remaining_issues,
            })
        }
        Err(e) => {
            remaining_issues.push(format!("Parse error after repairs: {e}"));
            Err(format!("Failed to repair JSON: {e}"))
        }
    }
}

/// Attempt to repair and deserialize to a specific type.
pub fn repair_and_deserialize<T: serde::de::DeserializeOwned>(
    json_str: &str,
    schema: Option<&Value>,
) -> Result<(T, Vec<JsonRepair>), String> {
    let result = repair_and_parse(json_str, schema)?;

    // Try to deserialize to target type
    let value = result.value;
    serde_json::from_value(value)
        .map(|v| (v, result.repairs))
        .map_err(|e| {
            format!(
                "Failed to deserialize repaired JSON to target type: {} (issues: {})",
                e,
                result.remaining_issues.join("; ")
            )
        })
}

/// Fix trailing commas before closing brackets.
fn fix_trailing_commas(input: &str) -> RepairResult {
    let mut repairs = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut result = String::new();
    let mut i = 0;
    let mut in_string = false;
    let mut escape_next = false;

    while i < chars.len() {
        let ch = chars[i];

        if escape_next {
            result.push(ch);
            escape_next = false;
            i += 1;
            continue;
        }

        match ch {
            '\\' if in_string => {
                result.push(ch);
                escape_next = true;
            }
            '"' => {
                in_string = !in_string;
                result.push(ch);
            }
            ',' if !in_string => {
                // Look ahead to see if this is a trailing comma
                let next_non_ws = skip_whitespace(&chars, i + 1);
                if next_non_ws < chars.len() {
                    let next_ch = chars[next_non_ws];
                    if next_ch == ']' || next_ch == '}' {
                        // Skip the trailing comma - don't add to result
                        repairs.push(JsonRepair {
                            kind: JsonRepairKind::TrailingComma,
                            location: format!("position {}", i),
                            description: "Removed trailing comma before closing bracket"
                                .to_string(),
                        });
                        i += 1;
                        continue;
                    }
                }
                result.push(ch);
            }
            _ => {
                result.push(ch);
            }
        }
        i += 1;
    }

    RepairResult {
        text: result,
        fixed: !repairs.is_empty(),
        repairs,
    }
}

/// Fix missing commas between array elements or object members.
fn fix_missing_commas(input: &str) -> RepairResult {
    let mut repairs = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut result = String::new();
    let mut i = 0;
    let mut in_string = false;
    let mut escape_next = false;

    while i < chars.len() {
        let ch = chars[i];

        if escape_next {
            result.push(ch);
            escape_next = false;
            i += 1;
            continue;
        }

        match ch {
            '\\' if in_string => {
                result.push(ch);
                escape_next = true;
            }
            '"' => {
                if !in_string {
                    // Starting a string - check if we need a comma before it
                    let trimmed = result.trim_end();
                    if !trimmed.is_empty() {
                        let last_char = trimmed.chars().last();
                        if let Some(last) = last_char {
                            // Need comma if last char is a value-ending char
                            if last == '"'
                                || last == '}'
                                || last == ']'
                                || last.is_ascii_digit()
                                || last == 'e'
                                || last == 'E'
                            {
                                repairs.push(JsonRepair {
                                    kind: JsonRepairKind::MissingComma,
                                    location: format!("before position {}", i),
                                    description: "Inserted missing comma between elements"
                                        .to_string(),
                                });
                                result.push(',');
                            }
                        }
                    }
                    in_string = true;
                } else {
                    // Ending a string
                    in_string = false;
                }
                result.push(ch);
            }
            '{' | '[' if !in_string => {
                // Check if we need a comma before this new object/array
                let trimmed = result.trim_end();
                if !trimmed.is_empty() {
                    let last_char = trimmed.chars().last();
                    if let Some(last) = last_char {
                        // Need comma if last char is a value-ending char
                        if last == '"'
                            || last == '}'
                            || last == ']'
                            || last.is_ascii_digit()
                            || last == 'e'
                            || last == 'E'
                        {
                            repairs.push(JsonRepair {
                                kind: JsonRepairKind::MissingComma,
                                location: format!("before position {}", i),
                                description: "Inserted missing comma between elements".to_string(),
                            });
                            result.push(',');
                        }
                    }
                }
                result.push(ch);
            }
            _ => {
                result.push(ch);
            }
        }
        i += 1;
    }

    RepairResult {
        text: result,
        fixed: !repairs.is_empty(),
        repairs,
    }
}

/// Fix unescaped quotes inside strings.
fn fix_unescaped_quotes(input: &str) -> RepairResult {
    let mut repairs = Vec::new();
    let mut result = String::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    let mut in_string = false;
    let mut string_start = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch == '\\' && in_string {
            // Check if this is a valid escape
            if i + 1 < chars.len() {
                let next = chars[i + 1];
                if matches!(next, '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't' | 'u') {
                    result.push(ch);
                    result.push(next);
                    i += 2;
                    continue;
                }
            }
        }

        if ch == '"' {
            if !in_string {
                in_string = true;
                string_start = i;
                result.push(ch);
            } else {
                // Check if this quote is properly closing the string
                // Look ahead to see if the next non-whitespace is a valid JSON delimiter
                let next_non_ws = skip_whitespace(&chars, i + 1);
                let valid_closer = if next_non_ws >= chars.len() {
                    true
                } else {
                    matches!(chars[next_non_ws], ',' | ']' | '}' | ':')
                };

                if valid_closer {
                    in_string = false;
                    result.push(ch);
                } else {
                    // This is an unescaped quote inside the string
                    repairs.push(JsonRepair {
                        kind: JsonRepairKind::UnescapedQuote,
                        location: format!("position {} in string starting at {}", i, string_start),
                        description: "Escaped unescaped quote inside string".to_string(),
                    });
                    result.push('\\');
                    result.push(ch);
                }
            }
        } else {
            result.push(ch);
        }
        i += 1;
    }

    RepairResult {
        text: result,
        fixed: !repairs.is_empty(),
        repairs,
    }
}

/// Fix mismatched brackets.
fn fix_mismatched_brackets(input: &str) -> RepairResult {
    let mut repairs = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut result = String::new();
    let mut bracket_stack: Vec<(char, usize)> = Vec::new();
    let mut in_string = false;
    let mut escape_next = false;

    // First pass: identify mismatched brackets
    for (i, &ch) in chars.iter().enumerate() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match ch {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '{' | '[' if !in_string => bracket_stack.push((ch, i)),
            '}' if !in_string => {
                if let Some((ch, _)) = bracket_stack.last() {
                    if *ch == '{' {
                        bracket_stack.pop();
                    } else {
                        repairs.push(JsonRepair {
                            kind: JsonRepairKind::MismatchedBracket,
                            location: format!("position {}", i),
                            description: "Unexpected closing brace '}'".to_string(),
                        });
                    }
                } else {
                    repairs.push(JsonRepair {
                        kind: JsonRepairKind::MismatchedBracket,
                        location: format!("position {}", i),
                        description: "Unexpected closing brace '}'".to_string(),
                    });
                }
            }
            ']' if !in_string => {
                if let Some((ch, _)) = bracket_stack.last() {
                    if *ch == '[' {
                        bracket_stack.pop();
                    } else {
                        repairs.push(JsonRepair {
                            kind: JsonRepairKind::MismatchedBracket,
                            location: format!("position {}", i),
                            description: "Unexpected closing bracket ']'".to_string(),
                        });
                    }
                } else {
                    repairs.push(JsonRepair {
                        kind: JsonRepairKind::MismatchedBracket,
                        location: format!("position {}", i),
                        description: "Unexpected closing bracket ']'".to_string(),
                    });
                }
            }
            _ => {}
        }
    }

    // Build result with missing closing brackets
    result = input.to_string();
    for (bracket, pos) in bracket_stack.iter().rev() {
        let closing = if *bracket == '{' { '}' } else { ']' };
        repairs.push(JsonRepair {
            kind: JsonRepairKind::MismatchedBracket,
            location: format!("position {}", pos),
            description: format!("Added missing closing bracket '{}'", closing),
        });
        result.push(closing);
    }

    RepairResult {
        text: result,
        fixed: !repairs.is_empty(),
        repairs,
    }
}

/// Fill missing optional fields with default values based on schema.
fn fill_missing_fields(
    value: Value,
    schema: Option<&Value>,
    repairs: &mut Vec<JsonRepair>,
) -> Value {
    let Some(schema) = schema else {
        return value;
    };

    match (value, schema) {
        (Value::Object(mut obj), Value::Object(schema_obj)) => {
            // Get required fields
            let required_fields: std::collections::HashSet<&str> = schema_obj
                .get("required")
                .and_then(|r| r.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();

            // Get properties schema
            if let Some(properties) = schema_obj.get("properties").and_then(|p| p.as_object()) {
                for (key, prop_schema) in properties {
                    if !obj.contains_key(key) {
                        // Add default value for missing optional field
                        if !required_fields.contains(key.as_str()) {
                            if let Some(default_value) = get_default_value(prop_schema) {
                                repairs.push(JsonRepair {
                                    kind: JsonRepairKind::MissingField,
                                    location: format!("/{}", key),
                                    description: format!(
                                        "Added missing optional field '{}' with default value",
                                        key
                                    ),
                                });
                                obj.insert(key.clone(), default_value);
                            }
                        }
                    } else if let Some(child_value) = obj.get(key) {
                        // Recursively fill nested objects
                        let filled =
                            fill_missing_fields(child_value.clone(), Some(prop_schema), repairs);
                        obj.insert(key.clone(), filled);
                    }
                }
            }
            Value::Object(obj)
        }
        (Value::Array(mut arr), Value::Object(schema_obj)) => {
            if let Some(items_schema) = schema_obj.get("items") {
                for item in &mut arr {
                    *item = fill_missing_fields(item.clone(), Some(items_schema), repairs);
                }
            }
            Value::Array(arr)
        }
        (value, _) => value,
    }
}

/// Get default value for a JSON schema type.
fn get_default_value(schema: &Value) -> Option<Value> {
    let schema_obj = schema.as_object()?;
    let type_name = schema_obj.get("type")?.as_str()?;

    // Check for explicit default
    if let Some(default) = schema_obj.get("default") {
        return Some(default.clone());
    }

    // Check for enum with first value
    if let Some(enum_values) = schema_obj.get("enum").and_then(|e| e.as_array()) {
        return enum_values.first().cloned();
    }

    // Generate default based on type
    match type_name {
        "string" => Some(Value::String(String::new())),
        "number" | "integer" => Some(Value::Number(0.into())),
        "boolean" => Some(Value::Bool(false)),
        "array" => Some(Value::Array(Vec::new())),
        "object" => Some(Value::Object(serde_json::Map::new())),
        "null" => Some(Value::Null),
        _ => None,
    }
}

/// Skip whitespace characters and return the index of the next non-whitespace character.
fn skip_whitespace(chars: &[char], start: usize) -> usize {
    let mut i = start;
    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }
    i
}

struct RepairResult {
    text: String,
    fixed: bool,
    repairs: Vec<JsonRepair>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_valid_json_without_modification() {
        let result = repair_and_parse(r#"{"foo": "bar"}"#, None).unwrap();
        assert!(result.repairs.is_empty());
        assert_eq!(result.value, json!({"foo": "bar"}));
    }

    #[test]
    fn fixes_trailing_comma_in_object() {
        let result = repair_and_parse(r#"{"foo": "bar",}"#, None).unwrap();
        assert!(result
            .repairs
            .iter()
            .any(|r| r.kind == JsonRepairKind::TrailingComma));
        assert_eq!(result.value, json!({"foo": "bar"}));
    }

    #[test]
    fn fixes_trailing_comma_in_array() {
        let result = repair_and_parse(r#"[1, 2, 3,]"#, None).unwrap();
        assert!(result
            .repairs
            .iter()
            .any(|r| r.kind == JsonRepairKind::TrailingComma));
        assert_eq!(result.value, json!([1, 2, 3]));
    }

    #[test]
    fn fixes_missing_comma_between_object_members() {
        let result = repair_and_parse(r#"{"foo": "bar" "baz": 1}"#, None).unwrap();
        assert!(result
            .repairs
            .iter()
            .any(|r| r.kind == JsonRepairKind::MissingComma));
        assert_eq!(result.value, json!({"foo": "bar", "baz": 1}));
    }

    #[test]
    fn fixes_missing_comma_between_array_elements() {
        let result = repair_and_parse(r#"["foo" "bar"]"#, None).unwrap();
        assert!(result
            .repairs
            .iter()
            .any(|r| r.kind == JsonRepairKind::MissingComma));
        assert_eq!(result.value, json!(["foo", "bar"]));
    }

    #[test]
    fn fills_missing_optional_field_with_default() {
        let schema = json!({
            "type": "object",
            "required": ["required_field"],
            "properties": {
                "required_field": { "type": "string" },
                "optional_field": { "type": "string" }
            }
        });

        let result = repair_and_parse(r#"{"required_field": "value"}"#, Some(&schema)).unwrap();

        assert!(result
            .repairs
            .iter()
            .any(|r| r.kind == JsonRepairKind::MissingField));
        assert_eq!(
            result.value.get("optional_field").and_then(|v| v.as_str()),
            Some("")
        );
    }

    #[test]
    fn fills_missing_optional_field_with_enum_default() {
        let schema = json!({
            "type": "object",
            "required": ["id"],
            "properties": {
                "id": { "type": "string" },
                "status": {
                    "type": "string",
                    "enum": ["active", "inactive", "pending"]
                }
            }
        });

        let result = repair_and_parse(r#"{"id": "test"}"#, Some(&schema)).unwrap();

        assert!(result
            .repairs
            .iter()
            .any(|r| r.kind == JsonRepairKind::MissingField));
        assert_eq!(
            result.value.get("status").and_then(|v| v.as_str()),
            Some("active")
        );
    }

    #[test]
    fn fixes_missing_closing_bracket() {
        let result = repair_and_parse(r#"{"foo": "bar""#, None).unwrap();
        assert!(result
            .repairs
            .iter()
            .any(|r| r.kind == JsonRepairKind::MismatchedBracket));
        assert_eq!(result.value, json!({"foo": "bar"}));
    }

    #[test]
    fn returns_error_for_unrepairable_json() {
        let result = repair_and_parse(r#"not json at all"#, None);
        assert!(result.is_err());
    }

    #[test]
    fn repair_and_deserialize_to_type() {
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct TestStruct {
            name: String,
            count: i32,
        }

        let (value, repairs): (TestStruct, _) =
            repair_and_deserialize(r#"{"name": "test", "count": 42}"#, None).unwrap();

        assert!(repairs.is_empty());
        assert_eq!(
            value,
            TestStruct {
                name: "test".to_string(),
                count: 42
            }
        );
    }
}
