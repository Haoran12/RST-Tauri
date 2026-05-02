//! Event logger for application events

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Event level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventLevel {
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl EventLevel {
    fn as_str(&self) -> &'static str {
        match self {
            EventLevel::Debug => "debug",
            EventLevel::Info => "info",
            EventLevel::Warn => "warn",
            EventLevel::Error => "error",
            EventLevel::Fatal => "fatal",
        }
    }
}

/// Application event log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEventLog {
    pub event_id: String,
    pub level: EventLevel,
    pub event_type: String,
    pub message: String,
    pub source_module: String,
    pub request_id: Option<String>,
    pub world_id: Option<String>,
    pub session_id: Option<String>,
    pub scene_turn_id: Option<String>,
    pub trace_id: Option<String>,
    pub character_id: Option<String>,
    pub runtime_config_snapshot_id: Option<String>,
    pub world_rules_snapshot_id: Option<String>,
    pub detail_json: Option<serde_json::Value>,
    pub created_at: String,
}

/// Event logger
pub struct EventLogger {
    pool: SqlitePool,
}

impl EventLogger {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Initialize the app_event_logs table
    pub async fn init_schema(&self) -> Result<(), String> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS app_event_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_id TEXT NOT NULL UNIQUE,
                level TEXT NOT NULL,
                event_type TEXT NOT NULL,
                message TEXT NOT NULL,
                source_module TEXT NOT NULL,
                request_id TEXT,
                world_id TEXT,
                session_id TEXT,
                scene_turn_id TEXT,
                trace_id TEXT,
                character_id TEXT,
                runtime_config_snapshot_id TEXT,
                world_rules_snapshot_id TEXT,
                detail_json TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_event_logs_event_id ON app_event_logs(event_id);
            CREATE INDEX IF NOT EXISTS idx_event_logs_level ON app_event_logs(level);
            CREATE INDEX IF NOT EXISTS idx_event_logs_event_type ON app_event_logs(event_type);
            CREATE INDEX IF NOT EXISTS idx_event_logs_created_at ON app_event_logs(created_at);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create app_event_logs table: {}", e))?;

        Ok(())
    }

    /// Log an event
    pub async fn log(&self, event: &AppEventLog) -> Result<(), String> {
        sqlx::query(
            r#"
            INSERT INTO app_event_logs (
                event_id, level, event_type, message, source_module,
                request_id, world_id, session_id, scene_turn_id, trace_id,
                character_id, runtime_config_snapshot_id, world_rules_snapshot_id,
                detail_json, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&event.event_id)
        .bind(event.level.as_str())
        .bind(&event.event_type)
        .bind(&event.message)
        .bind(&event.source_module)
        .bind(&event.request_id)
        .bind(&event.world_id)
        .bind(&event.session_id)
        .bind(&event.scene_turn_id)
        .bind(&event.trace_id)
        .bind(&event.character_id)
        .bind(&event.runtime_config_snapshot_id)
        .bind(&event.world_rules_snapshot_id)
        .bind(event.detail_json.as_ref().map(|d| serde_json::to_string(d).unwrap_or_default()))
        .bind(&event.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to log event: {}", e))?;

        Ok(())
    }

    /// Log a simple info event
    pub async fn info(&self, event_type: &str, message: &str, source_module: &str) -> Result<(), String> {
        self.log(&AppEventLog {
            event_id: uuid::Uuid::new_v4().to_string(),
            level: EventLevel::Info,
            event_type: event_type.to_string(),
            message: message.to_string(),
            source_module: source_module.to_string(),
            request_id: None,
            world_id: None,
            session_id: None,
            scene_turn_id: None,
            trace_id: None,
            character_id: None,
            runtime_config_snapshot_id: None,
            world_rules_snapshot_id: None,
            detail_json: None,
            created_at: Utc::now().to_rfc3339(),
        }).await
    }

    /// Log a warning event
    pub async fn warn(&self, event_type: &str, message: &str, source_module: &str) -> Result<(), String> {
        self.log(&AppEventLog {
            event_id: uuid::Uuid::new_v4().to_string(),
            level: EventLevel::Warn,
            event_type: event_type.to_string(),
            message: message.to_string(),
            source_module: source_module.to_string(),
            request_id: None,
            world_id: None,
            session_id: None,
            scene_turn_id: None,
            trace_id: None,
            character_id: None,
            runtime_config_snapshot_id: None,
            world_rules_snapshot_id: None,
            detail_json: None,
            created_at: Utc::now().to_rfc3339(),
        }).await
    }

    /// Log an error event
    pub async fn error(&self, event_type: &str, message: &str, source_module: &str) -> Result<(), String> {
        self.log(&AppEventLog {
            event_id: uuid::Uuid::new_v4().to_string(),
            level: EventLevel::Error,
            event_type: event_type.to_string(),
            message: message.to_string(),
            source_module: source_module.to_string(),
            request_id: None,
            world_id: None,
            session_id: None,
            scene_turn_id: None,
            trace_id: None,
            character_id: None,
            runtime_config_snapshot_id: None,
            world_rules_snapshot_id: None,
            detail_json: None,
            created_at: Utc::now().to_rfc3339(),
        }).await
    }

    /// Get recent events
    pub async fn get_recent(&self, limit: i64) -> Result<Vec<AppEventLog>, String> {
        let rows = sqlx::query(
            "SELECT * FROM app_event_logs ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to get events: {}", e))?;

        Ok(rows.iter().map(|r| row_to_event(r)).collect())
    }

    /// Get events by level
    pub async fn get_by_level(&self, level: &EventLevel, limit: i64) -> Result<Vec<AppEventLog>, String> {
        let rows = sqlx::query(
            "SELECT * FROM app_event_logs WHERE level = ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(level.as_str())
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to get events: {}", e))?;

        Ok(rows.iter().map(|r| row_to_event(r)).collect())
    }

    /// Delete events older than specified days
    pub async fn delete_old_events(&self, days: i64) -> Result<u64, String> {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        let result = sqlx::query(
            "DELETE FROM app_event_logs WHERE created_at < ?",
        )
        .bind(cutoff.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to delete old events: {}", e))?;

        Ok(result.rows_affected())
    }
}

fn row_to_event(row: &sqlx::sqlite::SqliteRow) -> AppEventLog {
    use sqlx::Row;
    let level_str: &str = row.get("level");
    let level = match level_str {
        "debug" => EventLevel::Debug,
        "info" => EventLevel::Info,
        "warn" => EventLevel::Warn,
        "error" => EventLevel::Error,
        "fatal" => EventLevel::Fatal,
        _ => EventLevel::Info,
    };

    AppEventLog {
        event_id: row.get("event_id"),
        level,
        event_type: row.get("event_type"),
        message: row.get("message"),
        source_module: row.get("source_module"),
        request_id: row.get("request_id"),
        world_id: row.get("world_id"),
        session_id: row.get("session_id"),
        scene_turn_id: row.get("scene_turn_id"),
        trace_id: row.get("trace_id"),
        character_id: row.get("character_id"),
        runtime_config_snapshot_id: row.get("runtime_config_snapshot_id"),
        world_rules_snapshot_id: row.get("world_rules_snapshot_id"),
        detail_json: row.get::<Option<&str>, _>("detail_json").and_then(|s| serde_json::from_str(s).ok()),
        created_at: row.get("created_at"),
    }
}
