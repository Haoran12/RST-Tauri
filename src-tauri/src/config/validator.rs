//! Configuration validator

use serde_json::Value;

/// Validate configuration against schema
pub fn validate_config(config: &Value) -> Result<(), String> {
    let root = config
        .as_object()
        .ok_or_else(|| "Configuration root must be a JSON object".to_string())?;

    if let Some(version) = root.get("schema_version") {
        if !version.is_string() {
            return Err("schema_version must be a string".to_string());
        }
    }

    if let Some(logs) = root.get("logs") {
        let logs = logs
            .as_object()
            .ok_or_else(|| "logs must be an object".to_string())?;

        if let Some(max_bytes) = logs.get("max_size_bytes") {
            let value = max_bytes
                .as_u64()
                .ok_or_else(|| "logs.max_size_bytes must be an unsigned integer".to_string())?;
            if value == 0 {
                return Err("logs.max_size_bytes must be greater than 0".to_string());
            }
        }

        if let Some(retention_days) = logs.get("retention_days") {
            let value = retention_days
                .as_u64()
                .ok_or_else(|| "logs.retention_days must be an unsigned integer".to_string())?;
            if value == 0 {
                return Err("logs.retention_days must be greater than 0".to_string());
            }
        }
    }

    if let Some(request_budget) = root.get("request_budget") {
        let request_budget = request_budget
            .as_object()
            .ok_or_else(|| "request_budget must be an object".to_string())?;

        if let Some(max_context_tokens) = request_budget.get("max_context_tokens") {
            let value = max_context_tokens.as_u64().ok_or_else(|| {
                "request_budget.max_context_tokens must be an unsigned integer".to_string()
            })?;
            if value == 0 {
                return Err("request_budget.max_context_tokens must be greater than 0".to_string());
            }
        }

        if let Some(soft_limit_tokens) = request_budget.get("soft_limit_tokens") {
            let value = soft_limit_tokens.as_u64().ok_or_else(|| {
                "request_budget.soft_limit_tokens must be an unsigned integer".to_string()
            })?;
            if value == 0 {
                return Err("request_budget.soft_limit_tokens must be greater than 0".to_string());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_config;

    #[test]
    fn accepts_minimal_object_config() {
        validate_config(&serde_json::json!({ "schema_version": "1" })).expect("valid config");
    }

    #[test]
    fn rejects_non_object_root() {
        let error =
            validate_config(&serde_json::json!(["bad"])).expect_err("non-object root should fail");
        assert!(error.contains("root must be a JSON object"));
    }

    #[test]
    fn rejects_zero_log_size() {
        let error = validate_config(&serde_json::json!({
            "logs": { "max_size_bytes": 0 }
        }))
        .expect_err("zero max_size_bytes should fail");
        assert!(error.contains("greater than 0"));
    }
}
