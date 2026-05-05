use serde_json::Value;

use super::{
    validate_value_shape, StructuredTextBinding, StructuredTextDiagnostic,
    StructuredTextDiagnosticCode, StructuredTextSeverity, StructuredTextValidationResult,
};

pub fn validate_yaml(
    text: &str,
    binding: &StructuredTextBinding,
) -> Result<StructuredTextValidationResult, String> {
    let mut diagnostics = collect_yaml_indent_diagnostics(text);
    if let Some(feature_diagnostic) = collect_yaml_feature_diagnostic(text, binding) {
        diagnostics.push(feature_diagnostic);
    }

    match serde_yaml::from_str::<Value>(text) {
        Ok(value) => {
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
            let location = error.location();
            let line = location.as_ref().map(|loc| loc.line()).unwrap_or(1);
            let column = location.as_ref().map(|loc| loc.column()).unwrap_or(1);
            diagnostics.push(StructuredTextDiagnostic {
                severity: StructuredTextSeverity::Blocker,
                code: StructuredTextDiagnosticCode::ParseError,
                message: error.to_string(),
                line,
                column,
                length: Some(1),
            });

            Ok(StructuredTextValidationResult {
                text: text.to_string(),
                diagnostics,
                parsed_value: None,
            })
        }
    }
}

pub fn format_yaml(
    text: &str,
    binding: &StructuredTextBinding,
) -> Result<StructuredTextValidationResult, String> {
    let value: Value = serde_yaml::from_str(text)
        .map_err(|e| format!("Failed to parse YAML for formatting: {}", e))?;

    let mut diagnostics = collect_yaml_indent_diagnostics(text);
    if let Some(feature_diagnostic) = collect_yaml_feature_diagnostic(text, binding) {
        diagnostics.push(feature_diagnostic);
    }
    if let Some(shape_diagnostic) = validate_value_shape(&value, binding) {
        diagnostics.push(shape_diagnostic);
    }

    let formatted = serde_yaml::to_string(&value)
        .map_err(|e| format!("Failed to format YAML: {}", e))?
        .trim_end()
        .to_string();

    Ok(StructuredTextValidationResult {
        text: formatted,
        diagnostics,
        parsed_value: Some(value),
    })
}

fn collect_yaml_indent_diagnostics(text: &str) -> Vec<StructuredTextDiagnostic> {
    let mut diagnostics = Vec::new();

    for (index, line) in text.lines().enumerate() {
        if let Some(tab_prefix) = line
            .chars()
            .take_while(|ch| *ch == '\t')
            .count()
            .checked_sub(0)
        {
            if tab_prefix > 0 {
                diagnostics.push(StructuredTextDiagnostic {
                    severity: StructuredTextSeverity::Blocker,
                    code: StructuredTextDiagnosticCode::ParseError,
                    message: "YAML 缩进不能使用 tab。".to_string(),
                    line: index + 1,
                    column: 1,
                    length: Some(tab_prefix),
                });
            }
        }

        if has_colon_without_space(line) {
            let column = line.find(':').map(|value| value + 1).unwrap_or(1);
            diagnostics.push(StructuredTextDiagnostic {
                severity: StructuredTextSeverity::Warning,
                code: StructuredTextDiagnosticCode::AutoFixAvailable,
                message: "建议在 YAML 冒号后补空格。".to_string(),
                line: index + 1,
                column,
                length: Some(1),
            });
        }
    }

    diagnostics
}

fn has_colon_without_space(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') || !trimmed.contains(':') {
        return false;
    }

    if let Some(index) = trimmed.find(':') {
        let after = trimmed.chars().nth(index + 1);
        return !matches!(after, None | Some(' ') | Some('\n'));
    }

    false
}

fn collect_yaml_feature_diagnostic(
    text: &str,
    binding: &StructuredTextBinding,
) -> Option<StructuredTextDiagnostic> {
    let has_anchor_like = text.contains('&') || text.contains('*');
    let has_tag_like = text.lines().any(|line| line.trim_start().starts_with('!'));
    if !has_anchor_like && !has_tag_like {
        return None;
    }

    let blocker = matches!(
        binding.storage_kind,
        super::StructuredTextStorageKind::JsonValue
    );
    Some(StructuredTextDiagnostic {
        severity: if blocker {
            StructuredTextSeverity::Blocker
        } else {
            StructuredTextSeverity::Warning
        },
        code: StructuredTextDiagnosticCode::UnsupportedYamlFeature,
        message: if blocker {
            "当前字段保存为结构化值，暂不支持 YAML anchor / alias / tag。".to_string()
        } else {
            "YAML anchor / alias / tag 仅作为文本保留，不会参与业务结构展开。".to_string()
        },
        line: 1,
        column: 1,
        length: None,
    })
}
