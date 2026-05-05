pub mod json;
pub mod yaml;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuredTextSeverity {
    Info,
    Warning,
    Blocker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuredTextDiagnosticCode {
    UnmatchedBracket,
    UnclosedQuote,
    InvalidEscape,
    ParseError,
    UnsupportedYamlFeature,
    AutoFixAvailable,
    AutoFixApplied,
    SchemaTypeMismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredTextDiagnostic {
    pub severity: StructuredTextSeverity,
    pub code: StructuredTextDiagnosticCode,
    pub message: String,
    pub line: usize,
    pub column: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StructuredTextStorageKind {
    String,
    JsonValue,
    YamlFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RequiredValueShape {
    String,
    Object,
    Array,
    Any,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredTextBinding {
    pub resource_kind: String,
    pub field_path: String,
    pub allowed_modes: Vec<String>,
    pub default_mode: String,
    pub storage_kind: StructuredTextStorageKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_value_shape: Option<RequiredValueShape>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredTextBackendRequest {
    pub text: String,
    pub mode: String,
    pub binding: StructuredTextBinding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredTextValidationResult {
    pub text: String,
    pub diagnostics: Vec<StructuredTextDiagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parsed_value: Option<Value>,
}

pub fn validate_request(
    input: StructuredTextBackendRequest,
) -> Result<StructuredTextValidationResult, String> {
    match input.mode.as_str() {
        "plain" => Ok(validate_plain(input)),
        "json" => json::validate_json(&input.text, &input.binding),
        "yaml" => yaml::validate_yaml(&input.text, &input.binding),
        other => Err(format!("Unsupported structured text mode: {}", other)),
    }
}

pub fn format_request(
    input: StructuredTextBackendRequest,
) -> Result<StructuredTextValidationResult, String> {
    match input.mode.as_str() {
        "plain" => Ok(validate_plain(input)),
        "json" => json::format_json(&input.text, &input.binding),
        "yaml" => yaml::format_yaml(&input.text, &input.binding),
        other => Err(format!("Unsupported structured text mode: {}", other)),
    }
}

fn validate_plain(input: StructuredTextBackendRequest) -> StructuredTextValidationResult {
    let mut diagnostics = collect_plain_balance_diagnostics(&input.text);

    if input.binding.storage_kind == StructuredTextStorageKind::JsonValue {
        diagnostics.push(StructuredTextDiagnostic {
            severity: StructuredTextSeverity::Blocker,
            code: StructuredTextDiagnosticCode::SchemaTypeMismatch,
            message: "当前字段要求结构化值，不能以 Plain 模式保存。".to_string(),
            line: 1,
            column: 1,
            length: None,
        });
    }

    StructuredTextValidationResult {
        text: input.text,
        diagnostics,
        parsed_value: None,
    }
}

pub fn validate_value_shape(
    value: &Value,
    binding: &StructuredTextBinding,
) -> Option<StructuredTextDiagnostic> {
    if binding.storage_kind != StructuredTextStorageKind::JsonValue
        && binding.storage_kind != StructuredTextStorageKind::YamlFile
    {
        return None;
    }

    let required = binding
        .required_value_shape
        .clone()
        .unwrap_or(RequiredValueShape::Any);

    if required == RequiredValueShape::Any {
        return None;
    }

    let actual = match value {
        Value::String(_) => RequiredValueShape::String,
        Value::Array(_) => RequiredValueShape::Array,
        Value::Object(_) => RequiredValueShape::Object,
        _ => RequiredValueShape::Any,
    };

    if actual == required {
        return None;
    }

    Some(StructuredTextDiagnostic {
        severity: StructuredTextSeverity::Blocker,
        code: StructuredTextDiagnosticCode::SchemaTypeMismatch,
        message: format!(
            "字段要求 {}，当前解析结果为 {}。",
            shape_label(&required),
            shape_label(&actual)
        ),
        line: 1,
        column: 1,
        length: None,
    })
}

fn shape_label(shape: &RequiredValueShape) -> &'static str {
    match shape {
        RequiredValueShape::String => "string",
        RequiredValueShape::Object => "object",
        RequiredValueShape::Array => "array",
        RequiredValueShape::Any => "any",
    }
}

fn collect_plain_balance_diagnostics(text: &str) -> Vec<StructuredTextDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut stack: Vec<(char, usize)> = Vec::new();
    let mut active_quote: Option<(char, usize)> = None;
    let mut escaped = false;

    for (index, ch) in text.char_indices() {
        if let Some((quote, start_index)) = active_quote {
            if escaped {
                escaped = false;
                continue;
            }

            if ch == '\\' {
                escaped = true;
                continue;
            }

            if ch == quote {
                active_quote = None;
            } else {
                active_quote = Some((quote, start_index));
            }
            continue;
        }

        match ch {
            '"' | '\'' | '`' => active_quote = Some((ch, index)),
            '(' | '[' | '{' => stack.push((ch, index)),
            ')' | ']' | '}' => {
                let expected = match ch {
                    ')' => '(',
                    ']' => '[',
                    '}' => '{',
                    _ => unreachable!(),
                };

                let last = stack.pop();
                if !matches!(last, Some((actual, _)) if actual == expected) {
                    let (line, column) = offset_to_position(text, index);
                    diagnostics.push(StructuredTextDiagnostic {
                        severity: StructuredTextSeverity::Warning,
                        code: StructuredTextDiagnosticCode::UnmatchedBracket,
                        message: format!("未匹配的括号 {}", ch),
                        line,
                        column,
                        length: Some(1),
                    });
                }
            }
            _ => {}
        }
    }

    if let Some((quote, index)) = active_quote {
        let (line, column) = offset_to_position(text, index);
        diagnostics.push(StructuredTextDiagnostic {
            severity: StructuredTextSeverity::Warning,
            code: StructuredTextDiagnosticCode::UnclosedQuote,
            message: format!("未闭合的引号 {}", quote),
            line,
            column,
            length: Some(1),
        });
    }

    for (ch, index) in stack {
        let (line, column) = offset_to_position(text, index);
        diagnostics.push(StructuredTextDiagnostic {
            severity: StructuredTextSeverity::Warning,
            code: StructuredTextDiagnosticCode::UnmatchedBracket,
            message: format!("未闭合的括号 {}", ch),
            line,
            column,
            length: Some(1),
        });
    }

    diagnostics
}

pub fn offset_to_position(text: &str, offset: usize) -> (usize, usize) {
    let safe_offset = offset.min(text.len());
    let prefix = &text[..safe_offset];
    let line = prefix.lines().count().max(1);
    let column = prefix
        .lines()
        .last()
        .map(|line_text| line_text.chars().count() + 1)
        .unwrap_or(1);
    (line, column)
}
