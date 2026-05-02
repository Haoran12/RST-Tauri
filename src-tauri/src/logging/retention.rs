//! Log retention management

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Log retention state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRetentionState {
    pub retention_id: String,
    pub scope: String,
    pub world_id: Option<String>,
    pub runtime_config_snapshot_id: Option<String>,
    pub size_limit_bytes: u64,
    pub current_size_bytes: Option<u64>,
    pub last_checked_at: Option<String>,
    pub last_cleanup_at: Option<String>,
    pub cleanup_needed: bool,
    pub user_prompt_required: bool,
}

/// Log retention manager
pub struct LogRetentionManager {
    pool: SqlitePool,
    default_retention_days: i64,
    default_size_limit_bytes: u64,
}

impl LogRetentionManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            default_retention_days: 30,
            default_size_limit_bytes: 100 * 1024 * 1024, // 100 MB
        }
    }

    /// Initialize retention state table
    pub async fn init_schema(&self) -> Result<(), String> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS log_retention_states (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                retention_id TEXT NOT NULL UNIQUE,
                scope TEXT NOT NULL,
                world_id TEXT,
                runtime_config_snapshot_id TEXT,
                size_limit_bytes INTEGER NOT NULL,
                current_size_bytes INTEGER,
                last_checked_at TEXT,
                last_cleanup_at TEXT,
                cleanup_needed INTEGER DEFAULT 0,
                user_prompt_required INTEGER DEFAULT 0
            );
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create log_retention_states table: {}", e))?;

        Ok(())
    }

    /// Check and perform log retention
    pub async fn check_retention(&self) -> Result<LogRetentionResult, String> {
        let now = Utc::now();

        // Get current database size
        let current_size = self.get_database_size().await?;

        // Delete old LLM logs
        let llm_deleted = self.delete_old_llm_logs().await?;

        // Delete old event logs
        let events_deleted = self.delete_old_event_logs().await?;

        // Update retention state
        self.update_retention_state(&now, current_size).await?;

        Ok(LogRetentionResult {
            llm_logs_deleted: llm_deleted,
            event_logs_deleted: events_deleted,
            size_before_bytes: current_size,
            size_after_bytes: self.get_database_size().await?,
        })
    }

    /// Get database file size
    async fn get_database_size(&self) -> Result<u64, String> {
        let row = sqlx::query(
            "SELECT SUM(pgsize) as size FROM dbstat WHERE aggregate = TRUE",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to get database size: {}", e))?;

        use sqlx::Row;
        Ok(row.and_then(|r| r.get::<Option<i64>, _>("size")).unwrap_or(0) as u64)
    }

    /// Delete old LLM logs based on retention policy
    async fn delete_old_llm_logs(&self) -> Result<u64, String> {
        let cutoff = Utc::now() - chrono::Duration::days(self.default_retention_days);
        let result = sqlx::query(
            "DELETE FROM llm_call_logs WHERE created_at < ?",
        )
        .bind(cutoff.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to delete old LLM logs: {}", e))?;

        Ok(result.rows_affected())
    }

    /// Delete old event logs based on retention policy
    async fn delete_old_event_logs(&self) -> Result<u64, String> {
        let cutoff = Utc::now() - chrono::Duration::days(self.default_retention_days);
        let result = sqlx::query(
            "DELETE FROM app_event_logs WHERE created_at < ?",
        )
        .bind(cutoff.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to delete old event logs: {}", e))?;

        Ok(result.rows_affected())
    }

    /// Update retention state
    async fn update_retention_state(&self, now: &chrono::DateTime<Utc>, size: u64) -> Result<(), String> {
        let retention_id = uuid::Uuid::new_v4().to_string();
        let cleanup_needed = size > self.default_size_limit_bytes;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO log_retention_states (
                retention_id, scope, size_limit_bytes, current_size_bytes,
                last_checked_at, last_cleanup_at, cleanup_needed, user_prompt_required
            ) VALUES (?, 'global', ?, ?, ?, ?, ?, 0)
            "#,
        )
        .bind(&retention_id)
        .bind(self.default_size_limit_bytes as i64)
        .bind(size as i64)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(if cleanup_needed { 1 } else { 0 })
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update retention state: {}", e))?;

        Ok(())
    }

    /// Force cleanup to reduce size
    pub async fn force_cleanup(&self, target_size_bytes: u64) -> Result<u64, String> {
        let current_size = self.get_database_size().await?;
        if current_size <= target_size_bytes {
            return Ok(0);
        }

        // Delete oldest logs first until under limit
        let mut total_deleted = 0u64;
        while self.get_database_size().await? > target_size_bytes {
            // Delete in batches of 1000
            let result = sqlx::query(
                "DELETE FROM llm_call_logs WHERE id IN (SELECT id FROM llm_call_logs ORDER BY created_at ASC LIMIT 1000)",
            )
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to delete logs: {}", e))?;

            let deleted = result.rows_affected();
            total_deleted += deleted;

            if deleted == 0 {
                break;
            }
        }

        // Vacuum database to reclaim space
        sqlx::query("VACUUM")
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to vacuum database: {}", e))?;

        Ok(total_deleted)
    }
}

/// Result of retention check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRetentionResult {
    pub llm_logs_deleted: u64,
    pub event_logs_deleted: u64,
    pub size_before_bytes: u64,
    pub size_after_bytes: u64,
}
