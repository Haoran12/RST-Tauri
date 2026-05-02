//! LLM call logger

use crate::logging::context::LogContext;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// LLM call log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCallLog {
    pub request_id: String,
    pub mode: String,
    pub world_id: Option<String>,
    pub session_id: Option<String>,
    pub scene_turn_id: Option<String>,
    pub trace_id: Option<String>,
    pub character_id: Option<String>,
    pub llm_node: String,
    pub api_config_id: String,
    pub runtime_config_snapshot_id: Option<String>,
    pub world_rules_snapshot_id: Option<String>,
    pub provider: String,
    pub model: String,
    pub call_type: String,
    pub request_json: serde_json::Value,
    pub schema_json: Option<serde_json::Value>,
    pub response_json: Option<serde_json::Value>,
    pub assembled_text: Option<String>,
    pub readable_text: Option<String>,
    pub status: String,
    pub latency_ms: Option<u64>,
    pub token_usage: Option<serde_json::Value>,
    pub retry_count: u32,
    pub error_summary: Option<String>,
    pub redaction_applied: bool,
    pub created_at: String,
    pub completed_at: Option<String>,
}

/// In-progress call tracking
#[derive(Debug, Clone)]
struct InProgressCall {
    mode: String,
    world_id: Option<String>,
    scene_turn_id: Option<String>,
    trace_id: Option<String>,
    character_id: Option<String>,
    llm_node: String,
    api_config_id: String,
    request_json: serde_json::Value,
    provider: String,
    model: String,
    call_type: String,
    schema_json: Option<serde_json::Value>,
    redaction_applied: bool,
    started_at: chrono::DateTime<Utc>,
}

/// LLM call logger
pub struct LlmCallLogger {
    pool: SqlitePool,
    in_progress: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, InProgressCall>>>,
}

impl LlmCallLogger {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            in_progress: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Initialize the llm_call_logs table
    pub async fn init_schema(&self) -> Result<(), String> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS llm_call_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                request_id TEXT NOT NULL UNIQUE,
                mode TEXT NOT NULL,
                world_id TEXT,
                session_id TEXT,
                scene_turn_id TEXT,
                trace_id TEXT,
                character_id TEXT,
                llm_node TEXT NOT NULL,
                api_config_id TEXT NOT NULL,
                runtime_config_snapshot_id TEXT,
                world_rules_snapshot_id TEXT,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                call_type TEXT NOT NULL,
                request_json TEXT NOT NULL,
                schema_json TEXT,
                response_json TEXT,
                assembled_text TEXT,
                readable_text TEXT,
                status TEXT NOT NULL,
                latency_ms INTEGER,
                token_usage TEXT,
                retry_count INTEGER DEFAULT 0,
                error_summary TEXT,
                redaction_applied INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                completed_at TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_llm_logs_request_id ON llm_call_logs(request_id);
            CREATE INDEX IF NOT EXISTS idx_llm_logs_world_id ON llm_call_logs(world_id);
            CREATE INDEX IF NOT EXISTS idx_llm_logs_trace_id ON llm_call_logs(trace_id);
            CREATE INDEX IF NOT EXISTS idx_llm_logs_created_at ON llm_call_logs(created_at);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create llm_call_logs table: {}", e))?;

        Ok(())
    }

    /// Log LLM call start
    pub async fn log_start(
        &self,
        context: &LogContext,
        request: &serde_json::Value,
        provider: &str,
        model: &str,
        call_type: &str,
        schema: Option<&serde_json::Value>,
    ) {
        let request_id = context.request_id.clone();
        let mode = match context.mode {
            crate::logging::context::LogMode::St => "ST",
            crate::logging::context::LogMode::Agent => "Agent",
        };
        let llm_node = match context.llm_node {
            crate::logging::context::LlmNode::STChat => "STChat",
            crate::logging::context::LlmNode::SceneInitializer => "SceneInitializer",
            crate::logging::context::LlmNode::SceneStateExtractor => "SceneStateExtractor",
            crate::logging::context::LlmNode::CharacterCognitivePass => "CharacterCognitivePass",
            crate::logging::context::LlmNode::OutcomePlanner => "OutcomePlanner",
            crate::logging::context::LlmNode::SurfaceRealizer => "SurfaceRealizer",
        };

        let (request_json, request_redacted) = redact_sensitive_value(request);
        let schema_json = schema.map(redact_sensitive_value).map(|(value, _)| value);

        let call = InProgressCall {
            mode: mode.to_string(),
            world_id: context.world_id.clone(),
            scene_turn_id: context.scene_turn_id.clone(),
            trace_id: context.trace_id.clone(),
            character_id: context.character_id.clone(),
            llm_node: llm_node.to_string(),
            api_config_id: context.api_config_id.clone(),
            request_json,
            provider: provider.to_string(),
            model: model.to_string(),
            call_type: call_type.to_string(),
            schema_json,
            redaction_applied: request_redacted,
            started_at: Utc::now(),
        };

        self.in_progress.write().await.insert(request_id, call);
    }

    /// Log LLM call success
    pub async fn log_success(
        &self,
        request_id: &str,
        response: &serde_json::Value,
        token_usage: Option<serde_json::Value>,
    ) {
        let call = self.in_progress.read().await.get(request_id).cloned();
        if let Some(call) = call {
            self.in_progress.write().await.remove(request_id);

            let completed_at = Utc::now();
            let latency_ms = (completed_at - call.started_at).num_milliseconds() as u64;

            let (response_json, response_redacted) = redact_sensitive_value(response);
            let assembled_text = extract_text_from_response(&response_json);
            let readable_text = assemble_readable_text(&call.request_json, &response_json);
            let redaction_applied = call.redaction_applied || response_redacted;

            sqlx::query(
                r#"
                INSERT INTO llm_call_logs (
                    request_id, mode, world_id, session_id, scene_turn_id, trace_id,
                    character_id, llm_node, api_config_id, runtime_config_snapshot_id,
                    world_rules_snapshot_id, provider, model, call_type, request_json,
                    schema_json, response_json, assembled_text, readable_text, status,
                    latency_ms, token_usage, retry_count, redaction_applied, created_at, completed_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?, ?)
                "#,
            )
            .bind(request_id)
            .bind(&call.mode)
            .bind(&call.world_id)
            .bind(None::<String>) // session_id
            .bind(&call.scene_turn_id)
            .bind(&call.trace_id)
            .bind(&call.character_id)
            .bind(&call.llm_node)
            .bind(&call.api_config_id)
            .bind(None::<String>) // runtime_config_snapshot_id
            .bind(None::<String>) // world_rules_snapshot_id
            .bind(&call.provider)
            .bind(&call.model)
            .bind(&call.call_type)
            .bind(serde_json::to_string(&call.request_json).unwrap_or_default())
            .bind(call.schema_json.as_ref().map(|s| serde_json::to_string(s).unwrap_or_default()))
            .bind(serde_json::to_string(&response_json).ok())
            .bind(&assembled_text)
            .bind(&readable_text)
            .bind("success")
            .bind(latency_ms as i64)
            .bind(token_usage.as_ref().map(|t| serde_json::to_string(t).unwrap_or_default()))
            .bind(if redaction_applied { 1 } else { 0 })
            .bind(call.started_at.to_rfc3339())
            .bind(completed_at.to_rfc3339())
            .execute(&self.pool)
            .await
            .ok();
        }
    }

    /// Log LLM call failure
    pub async fn log_failure(&self, request_id: &str, error: &str) {
        let call = self.in_progress.read().await.get(request_id).cloned();
        if let Some(call) = call {
            self.in_progress.write().await.remove(request_id);

            let completed_at = Utc::now();
            let latency_ms = (completed_at - call.started_at).num_milliseconds() as u64;

            let error_summary = redact_sensitive_text(error);

            sqlx::query(
                r#"
                INSERT INTO llm_call_logs (
                    request_id, mode, world_id, session_id, scene_turn_id, trace_id,
                    character_id, llm_node, api_config_id, runtime_config_snapshot_id,
                    world_rules_snapshot_id, provider, model, call_type, request_json,
                    schema_json, status, latency_ms, error_summary, retry_count,
                    redaction_applied, created_at, completed_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?, ?)
                "#,
            )
            .bind(request_id)
            .bind(&call.mode)
            .bind(&call.world_id)
            .bind(None::<String>) // session_id
            .bind(&call.scene_turn_id)
            .bind(&call.trace_id)
            .bind(&call.character_id)
            .bind(&call.llm_node)
            .bind(&call.api_config_id)
            .bind(None::<String>) // runtime_config_snapshot_id
            .bind(None::<String>) // world_rules_snapshot_id
            .bind(&call.provider)
            .bind(&call.model)
            .bind(&call.call_type)
            .bind(serde_json::to_string(&call.request_json).unwrap_or_default())
            .bind(call.schema_json.as_ref().map(|s| serde_json::to_string(s).unwrap_or_default()))
            .bind("failure")
            .bind(latency_ms as i64)
            .bind(&error_summary)
            .bind(if call.redaction_applied { 1 } else { 0 })
            .bind(call.started_at.to_rfc3339())
            .bind(completed_at.to_rfc3339())
            .execute(&self.pool)
            .await
            .ok();
        }
    }

    /// Get logs by request ID
    pub async fn get_by_request_id(&self, request_id: &str) -> Result<Option<LlmCallLog>, String> {
        let row = sqlx::query(
            "SELECT * FROM llm_call_logs WHERE request_id = ?",
        )
        .bind(request_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to get log: {}", e))?;

        Ok(row.map(|r| self::row_to_log(&r)))
    }

    /// Get logs by trace ID
    pub async fn get_by_trace_id(&self, trace_id: &str) -> Result<Vec<LlmCallLog>, String> {
        let rows = sqlx::query(
            "SELECT * FROM llm_call_logs WHERE trace_id = ? ORDER BY created_at",
        )
        .bind(trace_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to get logs: {}", e))?;

        Ok(rows.iter().map(|r| self::row_to_log(r)).collect())
    }

    /// Get recent logs
    pub async fn get_recent(&self, limit: i64) -> Result<Vec<LlmCallLog>, String> {
        let rows = sqlx::query(
            "SELECT * FROM llm_call_logs ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to get logs: {}", e))?;

        Ok(rows.iter().map(|r| self::row_to_log(r)).collect())
    }

    /// Delete logs older than specified days
    pub async fn delete_old_logs(&self, days: i64) -> Result<u64, String> {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        let result = sqlx::query(
            "DELETE FROM llm_call_logs WHERE created_at < ?",
        )
        .bind(cutoff.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to delete old logs: {}", e))?;

        Ok(result.rows_affected())
    }
}

fn row_to_log(row: &sqlx::sqlite::SqliteRow) -> LlmCallLog {
    use sqlx::Row;
    LlmCallLog {
        request_id: row.get("request_id"),
        mode: row.get("mode"),
        world_id: row.get("world_id"),
        session_id: row.get("session_id"),
        scene_turn_id: row.get("scene_turn_id"),
        trace_id: row.get("trace_id"),
        character_id: row.get("character_id"),
        llm_node: row.get("llm_node"),
        api_config_id: row.get("api_config_id"),
        runtime_config_snapshot_id: row.get("runtime_config_snapshot_id"),
        world_rules_snapshot_id: row.get("world_rules_snapshot_id"),
        provider: row.get("provider"),
        model: row.get("model"),
        call_type: row.get("call_type"),
        request_json: serde_json::from_str(row.get::<&str, _>("request_json")).unwrap_or(serde_json::Value::Null),
        schema_json: row.get::<Option<&str>, _>("schema_json").and_then(|s| serde_json::from_str(s).ok()),
        response_json: row.get::<Option<&str>, _>("response_json").and_then(|s| serde_json::from_str(s).ok()),
        assembled_text: row.get("assembled_text"),
        readable_text: row.get("readable_text"),
        status: row.get("status"),
        latency_ms: row.get::<Option<i64>, _>("latency_ms").map(|v| v as u64),
        token_usage: row.get::<Option<&str>, _>("token_usage").and_then(|s| serde_json::from_str(s).ok()),
        retry_count: row.get::<i32, _>("retry_count") as u32,
        error_summary: row.get("error_summary"),
        redaction_applied: row.get::<i32, _>("redaction_applied") != 0,
        created_at: row.get("created_at"),
        completed_at: row.get("completed_at"),
    }
}

fn extract_text_from_response(response: &serde_json::Value) -> Option<String> {
    if let Some(content) = response.get("content") {
        if let Some(arr) = content.as_array() {
            let texts: Vec<&str> = arr
                .iter()
                .filter_map(|c| c.get("text").and_then(|t| t.as_str()))
                .collect();
            if !texts.is_empty() {
                return Some(texts.join(""));
            }
        }
        if let Some(text) = content.as_str() {
            return Some(text.to_string());
        }
    }
    if let Some(choices) = response.get("choices") {
        if let Some(arr) = choices.as_array() {
            let texts: Vec<&str> = arr
                .iter()
                .filter_map(|c| c.get("message").and_then(|m| m.get("content")).and_then(|t| t.as_str()))
                .collect();
            if !texts.is_empty() {
                return Some(texts.join(""));
            }
        }
    }
    None
}

fn assemble_readable_text(request: &serde_json::Value, response: &serde_json::Value) -> Option<String> {
    let mut parts = Vec::new();

    if let Some(messages) = request.get("messages").and_then(|m| m.as_array()) {
        for msg in messages {
            if let (Some(role), Some(content)) = (msg.get("role").and_then(|r| r.as_str()), msg.get("content")) {
                let content_text = if let Some(s) = content.as_str() {
                    s.to_string()
                } else if let Some(arr) = content.as_array() {
                    arr.iter()
                        .filter_map(|c| c.get("text").and_then(|t| t.as_str()))
                        .collect::<Vec<_>>()
                        .join("")
                } else {
                    continue;
                };
                parts.push(format!("[{}] {}", role.to_uppercase(), content_text));
            }
        }
    }

    if let Some(response_text) = extract_text_from_response(response) {
        parts.push(format!("[ASSISTANT] {}", response_text));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

fn redact_sensitive_value(value: &serde_json::Value) -> (serde_json::Value, bool) {
    match value {
        serde_json::Value::Object(map) => {
            let mut redacted = serde_json::Map::new();
            let mut changed = false;
            for (key, child) in map {
                if is_sensitive_key(key) {
                    redacted.insert(key.clone(), redacted_placeholder(child));
                    changed = true;
                } else {
                    let (child_value, child_changed) = redact_sensitive_value(child);
                    changed |= child_changed;
                    redacted.insert(key.clone(), child_value);
                }
            }
            (serde_json::Value::Object(redacted), changed)
        }
        serde_json::Value::Array(items) => {
            let mut changed = false;
            let redacted_items = items
                .iter()
                .map(|item| {
                    let (item_value, item_changed) = redact_sensitive_value(item);
                    changed |= item_changed;
                    item_value
                })
                .collect();
            (serde_json::Value::Array(redacted_items), changed)
        }
        _ => (value.clone(), false),
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace('_', "-");
    matches!(
        normalized.as_str(),
        "api-key"
            | "apikey"
            | "authorization"
            | "proxy-authorization"
            | "x-api-key"
            | "token"
            | "access-token"
            | "refresh-token"
            | "id-token"
            | "secret"
            | "provider-secret"
            | "password"
            | "proxy-username"
            | "proxy-password"
    )
}

fn redacted_placeholder(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::String(_) => serde_json::Value::String("[REDACTED]".to_string()),
        serde_json::Value::Number(_) => serde_json::json!(0),
        serde_json::Value::Bool(_) => serde_json::json!(false),
        serde_json::Value::Array(_) => serde_json::Value::Array(Vec::new()),
        serde_json::Value::Object(_) => serde_json::Value::Object(serde_json::Map::new()),
        serde_json::Value::Null => serde_json::Value::Null,
    }
}

fn redact_sensitive_text(text: &str) -> String {
    let mut result = text.to_string();
    for marker in ["key=", "api_key=", "api-key=", "token=", "access_token=", "password="] {
        result = redact_query_value(&result, marker);
    }
    result
}

fn redact_query_value(text: &str, marker: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut remaining = text;
    while let Some(index) = remaining.to_ascii_lowercase().find(marker) {
        let (before, after_before) = remaining.split_at(index);
        output.push_str(before);
        let (prefix, value_and_rest) = after_before.split_at(marker.len());
        output.push_str(prefix);
        output.push_str("[REDACTED]");
        let rest_index = value_and_rest
            .find(|c| matches!(c, '&' | ' ' | '\n' | '\r' | '\t' | '"' | '\''))
            .unwrap_or(value_and_rest.len());
        remaining = &value_and_rest[rest_index..];
    }
    output.push_str(remaining);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_sensitive_json_fields_recursively() {
        let input = serde_json::json!({
            "headers": {
                "Authorization": "Bearer secret",
                "x-api-key": "provider-key"
            },
            "body": {
                "messages": [{"content": "keep prompt"}],
                "token_usage": {"total_tokens": 12}
            }
        });

        let (redacted, changed) = redact_sensitive_value(&input);

        assert!(changed);
        assert_eq!(redacted["headers"]["Authorization"], "[REDACTED]");
        assert_eq!(redacted["headers"]["x-api-key"], "[REDACTED]");
        assert_eq!(redacted["body"]["messages"][0]["content"], "keep prompt");
        assert_eq!(redacted["body"]["token_usage"]["total_tokens"], 12);
    }

    #[test]
    fn redacts_query_secret_values() {
        let text = "request failed for https://example.test/v1?key=abc123&model=test";
        assert_eq!(
            redact_sensitive_text(text),
            "request failed for https://example.test/v1?key=[REDACTED]&model=test"
        );
    }
}
