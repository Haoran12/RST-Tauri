//! LLM call logger

use crate::logging::context::LogContext;
use base64::Engine;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use uuid::Uuid;

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
    pub request_url: Option<String>,
    pub request_json: serde_json::Value,
    pub schema_json: Option<serde_json::Value>,
    pub response_json: Option<serde_json::Value>,
    pub reasoning_text: Option<String>,
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

/// LLM stream chunk log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmStreamChunk {
    pub chunk_id: String,
    pub request_id: String,
    pub chunk_index: u32,
    pub raw_chunk: String,
    pub received_at: String,
}

/// In-progress call tracking
#[derive(Debug, Clone)]
struct InProgressCall {
    mode: String,
    world_id: Option<String>,
    session_id: Option<String>,
    scene_turn_id: Option<String>,
    trace_id: Option<String>,
    character_id: Option<String>,
    llm_node: String,
    api_config_id: String,
    request_url: Option<String>,
    request_json: serde_json::Value,
    provider: String,
    model: String,
    call_type: String,
    schema_json: Option<serde_json::Value>,
    redaction_applied: bool,
    started_at: chrono::DateTime<Utc>,
    chunk_count: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

/// LLM call logger
pub struct LlmCallLogger {
    pool: SqlitePool,
    in_progress:
        std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, InProgressCall>>>,
}

impl LlmCallLogger {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            in_progress: std::sync::Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
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
                protected INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                completed_at TEXT
            );
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create llm_call_logs table: {}", e))?;

        // Migration: add missing columns if table already exists
        self.migrate_add_column("protected", "INTEGER DEFAULT 0").await?;
        self.migrate_add_column("redaction_applied", "INTEGER DEFAULT 0").await?;
        self.migrate_add_column("request_url", "TEXT").await?;
        self.migrate_add_column("reasoning_text", "TEXT").await?;

        // Create indexes
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_llm_logs_request_id ON llm_call_logs(request_id);
            CREATE INDEX IF NOT EXISTS idx_llm_logs_world_id ON llm_call_logs(world_id);
            CREATE INDEX IF NOT EXISTS idx_llm_logs_trace_id ON llm_call_logs(trace_id);
            CREATE INDEX IF NOT EXISTS idx_llm_logs_created_at ON llm_call_logs(created_at);
            CREATE INDEX IF NOT EXISTS idx_llm_logs_protected ON llm_call_logs(protected);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create llm_call_logs indexes: {}", e))?;

        // Create stream chunks table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS llm_stream_chunks (
                chunk_id TEXT PRIMARY KEY,
                request_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                raw_chunk TEXT NOT NULL,
                received_at TEXT NOT NULL,
                FOREIGN KEY (request_id) REFERENCES llm_call_logs(request_id)
            );
            CREATE INDEX IF NOT EXISTS idx_stream_chunks_request ON llm_stream_chunks(request_id, chunk_index);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create llm_stream_chunks table: {}", e))?;

        Ok(())
    }

    /// Add a column if it doesn't exist
    async fn migrate_add_column(&self, column: &str, definition: &str) -> Result<(), String> {
        use sqlx::Row;
        let row = sqlx::query(
            "SELECT COUNT(*) AS count FROM pragma_table_info('llm_call_logs') WHERE name = ?",
        )
        .bind(column)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to check column existence: {}", e))?;
        let has_column: bool = row.get::<i64, _>("count") != 0;

        if !has_column {
            let sql = format!("ALTER TABLE llm_call_logs ADD COLUMN {} {}", column, definition);
            sqlx::query(&sql)
                .execute(&self.pool)
                .await
                .map_err(|e| format!("Failed to add column {}: {}", column, e))?;
        }

        Ok(())
    }

    /// Log LLM call start
    pub async fn log_start(
        &self,
        context: &LogContext,
        request: &serde_json::Value,
        request_url: Option<&str>,
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

        // Build request with headers based on provider
        let request_with_headers = build_request_with_headers(provider, request_url, request);

        let (request_json, request_redacted) = redact_sensitive_value(&request_with_headers);
        let schema_json = schema.map(redact_sensitive_value).map(|(value, _)| value);

        let call = InProgressCall {
            mode: mode.to_string(),
            world_id: context.world_id.clone(),
            session_id: context.session_id.clone(),
            scene_turn_id: context.scene_turn_id.clone(),
            trace_id: context.trace_id.clone(),
            character_id: context.character_id.clone(),
            llm_node: llm_node.to_string(),
            api_config_id: context.api_config_id.clone(),
            request_url: request_url.map(str::to_string),
            request_json,
            provider: provider.to_string(),
            model: model.to_string(),
            call_type: call_type.to_string(),
            schema_json,
            redaction_applied: request_redacted,
            started_at: Utc::now(),
            chunk_count: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
        };

        tracing::info!(
            "[LlmLogger] log_start called, request_id: {}, inserting into in_progress",
            request_id
        );
        self.in_progress.write().await.insert(request_id, call);
        tracing::info!("[LlmLogger] log_start completed, request_id: {}", context.request_id);
    }

    /// Log a stream chunk for an in-progress call
    pub async fn log_stream_chunk(&self, request_id: &str, chunk: &str) {
        // Get chunk index atomically
        let chunk_count_arc = {
            let in_progress = self.in_progress.read().await;
            match in_progress.get(request_id) {
                Some(call) => call.chunk_count.clone(),
                None => return,
            }
        };

        let chunk_index = chunk_count_arc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let chunk_id = Uuid::new_v4().to_string();
        let received_at = Utc::now().to_rfc3339();

        // Redact sensitive data in chunk
        let raw_chunk = redact_sensitive_text(chunk);

        // Insert chunk
        sqlx::query(
            "INSERT INTO llm_stream_chunks (chunk_id, request_id, chunk_index, raw_chunk, received_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&chunk_id)
        .bind(request_id)
        .bind(chunk_index as i64)
        .bind(&raw_chunk)
        .bind(&received_at)
        .execute(&self.pool)
        .await
        .ok();
    }

    /// Get stream chunks for a request
    pub async fn get_stream_chunks(&self, request_id: &str) -> Result<Vec<LlmStreamChunk>, String> {
        let rows = sqlx::query(
            "SELECT chunk_id, request_id, chunk_index, raw_chunk, received_at FROM llm_stream_chunks WHERE request_id = ? ORDER BY chunk_index",
        )
        .bind(request_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to get stream chunks: {}", e))?;

        Ok(rows
            .iter()
            .map(|r| {
                use sqlx::Row;
                LlmStreamChunk {
                    chunk_id: r.get("chunk_id"),
                    request_id: r.get("request_id"),
                    chunk_index: r.get::<i64, _>("chunk_index") as u32,
                    raw_chunk: r.get("raw_chunk"),
                    received_at: r.get("received_at"),
                }
            })
            .collect())
    }

    /// Log LLM call success
    pub async fn log_success(
        &self,
        request_id: &str,
        response: &serde_json::Value,
        reasoning_text: Option<&str>,
        token_usage: Option<serde_json::Value>,
    ) {
        tracing::info!("[LlmLogger] log_success called, request_id: {}", request_id);
        let call = self.in_progress.read().await.get(request_id).cloned();
        tracing::info!(
            "[LlmLogger] log_success found in_progress call: {}",
            call.is_some()
        );
        if let Some(call) = call {
            self.in_progress.write().await.remove(request_id);

            let completed_at = Utc::now();
            let latency_ms = (completed_at - call.started_at).num_milliseconds() as u64;

            let (response_json, response_redacted) = redact_sensitive_value(response);
            let assembled_text = extract_text_from_response(&response_json);
            let readable_text = assemble_readable_text(&call.request_json, &response_json);
            let redaction_applied = call.redaction_applied || response_redacted;

            tracing::info!(
                "[LlmLogger] log_success inserting into database, request_id: {}, latency_ms: {}",
                request_id,
                latency_ms
            );
            let result = sqlx::query(
                r#"
                INSERT INTO llm_call_logs (
                    request_id, mode, world_id, session_id, scene_turn_id, trace_id,
                    character_id, llm_node, api_config_id, runtime_config_snapshot_id,
                    world_rules_snapshot_id, provider, model, call_type, request_url, request_json,
                    schema_json, response_json, reasoning_text, assembled_text, readable_text, status,
                    latency_ms, token_usage, retry_count, redaction_applied, created_at, completed_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(request_id)
            .bind(&call.mode)
            .bind(&call.world_id)
            .bind(&call.session_id)
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
            .bind(&call.request_url)
            .bind(serde_json::to_string(&call.request_json).unwrap_or_default())
            .bind(call.schema_json.as_ref().map(|s| serde_json::to_string(s).unwrap_or_default()))
            .bind(serde_json::to_string(&response_json).ok())
            .bind(reasoning_text)
            .bind(&assembled_text)
            .bind(&readable_text)
            .bind("success")
            .bind(latency_ms as i64)
            .bind(token_usage.as_ref().map(|t| serde_json::to_string(t).unwrap_or_default()))
            .bind(0i32) // retry_count
            .bind(if redaction_applied { 1 } else { 0 })
            .bind(call.started_at.to_rfc3339())
            .bind(completed_at.to_rfc3339())
            .execute(&self.pool)
            .await;
            tracing::info!(
                "[LlmLogger] log_success database insert result: {:?}",
                result.map(|r| r.rows_affected())
            );
        } else {
            tracing::warn!(
                "[LlmLogger] log_success request_id {} not found in in_progress",
                request_id
            );
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
                    world_rules_snapshot_id, provider, model, call_type, request_url, request_json,
                    schema_json, status, latency_ms, error_summary, retry_count,
                    redaction_applied, created_at, completed_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?, ?)
                "#,
            )
            .bind(request_id)
            .bind(&call.mode)
            .bind(&call.world_id)
            .bind(&call.session_id)
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
            .bind(&call.request_url)
            .bind(serde_json::to_string(&call.request_json).unwrap_or_default())
            .bind(
                call.schema_json
                    .as_ref()
                    .map(|s| serde_json::to_string(s).unwrap_or_default()),
            )
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
        let row = sqlx::query("SELECT * FROM llm_call_logs WHERE request_id = ?")
            .bind(request_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| format!("Failed to get log: {}", e))?;

        Ok(row.map(|r| self::row_to_log(&r)))
    }

    /// Get logs by trace ID
    pub async fn get_by_trace_id(&self, trace_id: &str) -> Result<Vec<LlmCallLog>, String> {
        let rows =
            sqlx::query("SELECT * FROM llm_call_logs WHERE trace_id = ? ORDER BY created_at")
                .bind(trace_id)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| format!("Failed to get logs: {}", e))?;

        Ok(rows.iter().map(|r| self::row_to_log(r)).collect())
    }

    /// Get recent logs
    pub async fn get_recent(&self, limit: i64) -> Result<Vec<LlmCallLog>, String> {
        let rows = sqlx::query("SELECT * FROM llm_call_logs ORDER BY created_at DESC LIMIT ?")
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("Failed to get logs: {}", e))?;

        Ok(rows.iter().map(|r| self::row_to_log(r)).collect())
    }

    /// Delete logs older than specified days
    pub async fn delete_old_logs(&self, days: i64) -> Result<u64, String> {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        let result = sqlx::query("DELETE FROM llm_call_logs WHERE created_at < ?")
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
        request_url: row.get("request_url"),
        request_json: serde_json::from_str(row.get::<&str, _>("request_json"))
            .unwrap_or(serde_json::Value::Null),
        schema_json: row
            .get::<Option<&str>, _>("schema_json")
            .and_then(|s| serde_json::from_str(s).ok()),
        response_json: row
            .get::<Option<&str>, _>("response_json")
            .and_then(|s| serde_json::from_str(s).ok()),
        reasoning_text: row.get("reasoning_text"),
        assembled_text: row.get("assembled_text"),
        readable_text: row.get("readable_text"),
        status: row.get("status"),
        latency_ms: row.get::<Option<i64>, _>("latency_ms").map(|v| v as u64),
        token_usage: row
            .get::<Option<&str>, _>("token_usage")
            .and_then(|s| serde_json::from_str(s).ok()),
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
                .filter_map(|c| {
                    c.get("message")
                        .and_then(|m| m.get("content"))
                        .and_then(|t| t.as_str())
                })
                .collect();
            if !texts.is_empty() {
                return Some(texts.join(""));
            }
        }
    }
    None
}

fn assemble_readable_text(
    request: &serde_json::Value,
    response: &serde_json::Value,
) -> Option<String> {
    let mut parts = Vec::new();

    // Extract system prompt if present (for providers that use separate system field)
    if let Some(system) = request.get("system") {
        let system_text = extract_text_value(system);
        if !system_text.trim().is_empty() {
            parts.push(format!("SYSTEM >\n{}", format_readable_content(&system_text)));
        }
    }

    // Extract messages from request body (for providers with messages in body)
    let messages = request
        .get("body")
        .and_then(|b| b.get("messages"))
        .or_else(|| request.get("messages"));

    if let Some(messages) = messages.and_then(|m| m.as_array()) {
        for msg in messages {
            if let (Some(role), Some(content)) =
                (msg.get("role").and_then(|r| r.as_str()), msg.get("content"))
            {
                let content_text = extract_text_value(content);
                if content_text.trim().is_empty() {
                    continue;
                }

                let prefix = match role.to_lowercase().as_str() {
                    "system" => "SYSTEM >",
                    "user" => "USER >",
                    "assistant" => "ASSISTANT >",
                    _ => continue, // Skip unknown roles
                };

                parts.push(format!("{}\n{}", prefix, format_readable_content(&content_text)));
            }
        }
    }

    // Extract response text
    if let Some(response_text) = extract_text_from_response(response) {
        if !response_text.trim().is_empty() {
            parts.push(format!("ASSISTANT >\n{}", format_readable_content(&response_text)));
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n---\n\n"))
    }
}

/// Extract text from a content field (string or array of content parts)
fn extract_text_value(content: &serde_json::Value) -> String {
    if let Some(text) = content.as_str() {
        text.to_string()
    } else if let Some(arr) = content.as_array() {
        arr.iter()
            .filter_map(|part| {
                // Handle different content part formats
                part.get("text")
                    .or_else(|| part.get("input_text"))
                    .and_then(|t| t.as_str())
            })
            .collect::<Vec<_>>()
            .join("")
    } else {
        String::new()
    }
}

/// Format content for readability:
/// - Convert escape sequences to actual characters
/// - Normalize whitespace and add paragraph breaks
fn format_readable_content(text: &str) -> String {
    let mut result = text.to_string();

    // Convert common escape sequences
    result = result.replace("\\n", "\n");
    result = result.replace("\\r", "\r");
    result = result.replace("\\t", "\t");
    result = result.replace("\\\"", "\"");
    result = result.replace("\\'", "'");
    result = result.replace("\\\\", "\\");

    // Normalize multiple consecutive newlines to double newlines (paragraph break)
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }

    // Trim leading/trailing whitespace from each line while preserving paragraph structure
    let lines: Vec<&str> = result.lines().collect();
    let mut formatted_lines = Vec::new();
    let mut in_paragraph = false;

    for line in lines {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            if in_paragraph {
                formatted_lines.push(String::new());
                in_paragraph = false;
            }
        } else {
            formatted_lines.push(trimmed.to_string());
            in_paragraph = true;
        }
    }

    formatted_lines.join("\n").trim().to_string()
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
                } else if is_binary_payload_key(key) {
                    let (summary, child_changed) = summarize_binary_payload(key, child);
                    changed |= child_changed;
                    redacted.insert(key.clone(), summary);
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

fn is_binary_payload_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "image_url" | "file_data" | "data"
    )
}

fn summarize_binary_payload(key: &str, value: &serde_json::Value) -> (serde_json::Value, bool) {
    let Some(raw) = value.as_str() else {
        return (value.clone(), false);
    };

    let (mime_type, encoded) = if key.eq_ignore_ascii_case("image_url") {
        match parse_data_url(raw) {
            Some(parts) => parts,
            None => return (value.clone(), false),
        }
    } else {
        let mime_type = infer_mime_type_from_sibling_context(value)
            .unwrap_or_else(|| "application/octet-stream".to_string());
        (mime_type, raw.to_string())
    };

    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(encoded.as_bytes()) else {
        return (value.clone(), false);
    };

    (
        serde_json::json!({
            "redacted": true,
            "transport": if key.eq_ignore_ascii_case("image_url") { "inline_data_url" } else { "inline_base64" },
            "mime_type": mime_type,
            "size_bytes": bytes.len(),
            "sha256": sha256_hex(&bytes),
        }),
        true,
    )
}

fn infer_mime_type_from_sibling_context(_value: &serde_json::Value) -> Option<String> {
    None
}

fn parse_data_url(raw: &str) -> Option<(String, String)> {
    let rest = raw.strip_prefix("data:")?;
    let (mime_type, encoded) = rest.split_once(";base64,")?;
    Some((mime_type.to_string(), encoded.to_string()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
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
    for marker in [
        "key=",
        "api_key=",
        "api-key=",
        "token=",
        "access_token=",
        "password=",
    ] {
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

/// Build a request JSON with headers based on provider type
fn build_request_with_headers(
    provider: &str,
    request_url: Option<&str>,
    request_body: &serde_json::Value,
) -> serde_json::Value {
    let mut headers = serde_json::Map::new();

    // Add provider-specific headers (will be redacted later)
    match provider {
        "anthropic" => {
            headers.insert("x-api-key".to_string(), serde_json::json!("[REDACTED]"));
            headers.insert("anthropic-version".to_string(), serde_json::json!("2023-06-01"));
            headers.insert("Content-Type".to_string(), serde_json::json!("application/json"));
        }
        "openai_chat" | "openai_responses" | "deepseek" => {
            headers.insert("Authorization".to_string(), serde_json::json!("Bearer [REDACTED]"));
            headers.insert("Content-Type".to_string(), serde_json::json!("application/json"));
        }
        "gemini" => {
            headers.insert("Content-Type".to_string(), serde_json::json!("application/json"));
            // API key is in URL query parameter, not header
        }
        "claude_code" => {
            headers.insert("Authorization".to_string(), serde_json::json!("Bearer [REDACTED]"));
            headers.insert("anthropic-version".to_string(), serde_json::json!("2023-06-01"));
            headers.insert("Content-Type".to_string(), serde_json::json!("application/json"));
        }
        _ => {
            headers.insert("Content-Type".to_string(), serde_json::json!("application/json"));
        }
    }

    let mut result = serde_json::Map::new();
    if let Some(url) = request_url {
        result.insert("url".to_string(), serde_json::json!(url));
    }
    result.insert("headers".to_string(), serde_json::Value::Object(headers));
    result.insert("body".to_string(), request_body.clone());

    serde_json::Value::Object(result)
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

    #[test]
    fn summarizes_inline_attachment_payloads_without_persisting_base64() {
        let input = serde_json::json!({
            "content": [
                {
                    "type": "image_url",
                    "image_url": "data:image/png;base64,aGVsbG8="
                },
                {
                    "type": "file",
                    "file": {
                        "file_data": "cGRmLWJ5dGVz"
                    }
                }
            ]
        });

        let (redacted, changed) = redact_sensitive_value(&input);

        assert!(changed);
        assert_eq!(redacted["content"][0]["image_url"]["redacted"], true);
        assert_eq!(redacted["content"][0]["image_url"]["size_bytes"], 5);
        assert_eq!(
            redacted["content"][1]["file"]["file_data"]["redacted"],
            true
        );
        assert_eq!(
            redacted["content"][1]["file"]["file_data"]["transport"],
            "inline_base64"
        );
        assert!(!redacted.to_string().contains("aGVsbG8="));
        assert!(!redacted.to_string().contains("cGRmLWJ5dGVz"));
    }

    #[test]
    fn assembles_readable_text_with_role_prefixes() {
        let request = serde_json::json!({
            "body": {
                "messages": [
                    {"role": "system", "content": "You are a helpful assistant."},
                    {"role": "user", "content": "Hello!"},
                    {"role": "assistant", "content": "Hi there!"}
                ]
            }
        });
        let response = serde_json::json!({
            "content": [{"type": "text", "text": "How can I help?"}]
        });

        let readable = assemble_readable_text(&request, &response).unwrap();

        assert!(readable.contains("SYSTEM >"));
        assert!(readable.contains("USER >"));
        assert!(readable.contains("ASSISTANT >"));
        assert!(readable.contains("You are a helpful assistant."));
        assert!(readable.contains("Hello!"));
        assert!(readable.contains("How can I help?"));
    }

    #[test]
    fn formats_escape_sequences_in_readable_text() {
        let text = "Line 1\\nLine 2\\n\\nParagraph 2 with \\\"quotes\\\" and \\\\backslash.";
        let formatted = format_readable_content(text);

        assert!(formatted.contains("Line 1\nLine 2"));
        assert!(formatted.contains("Paragraph 2 with \"quotes\" and \\backslash."));
    }

    #[test]
    fn extracts_text_from_anthropic_system_prompt() {
        let request = serde_json::json!({
            "system": "System instructions here.",
            "body": {
                "messages": [
                    {"role": "user", "content": "User message."}
                ]
            }
        });
        let response = serde_json::json!({});

        let readable = assemble_readable_text(&request, &response).unwrap();

        assert!(readable.contains("SYSTEM >"));
        assert!(readable.contains("System instructions here."));
        assert!(readable.contains("USER >"));
        assert!(readable.contains("User message."));
    }

    #[test]
    fn handles_array_content_format() {
        let request = serde_json::json!({
            "body": {
                "messages": [
                    {
                        "role": "user",
                        "content": [
                            {"type": "text", "text": "Part 1"},
                            {"type": "text", "text": " Part 2"}
                        ]
                    }
                ]
            }
        });
        let response = serde_json::json!({});

        let readable = assemble_readable_text(&request, &response).unwrap();

        assert!(readable.contains("Part 1 Part 2"));
    }
}
