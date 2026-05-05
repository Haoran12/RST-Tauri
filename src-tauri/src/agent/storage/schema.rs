//! Agent SQLite schema definitions
//!
//! Table structures for Agent mode persistence.

use sqlx::SqlitePool;

/// Agent schema manager
pub struct AgentSchema;

impl AgentSchema {
    /// Get the full Agent schema SQL
    pub fn full_schema() -> &'static str {
        include_str!("schema.sql")
    }

    /// Initialize Agent schema in the database
    pub async fn init(pool: &SqlitePool) -> Result<(), String> {
        sqlx::query(Self::full_schema())
            .execute(pool)
            .await
            .map_err(|e| format!("Failed to create Agent schema: {}", e))?;

        Ok(())
    }
}
