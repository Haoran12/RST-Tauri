//! SQLite storage for Agent mode

use sqlx::SqlitePool;

use crate::logging::event_logger::EventLogger;
use crate::logging::llm_logger::LlmCallLogger;
use crate::logging::retention::LogRetentionManager;

/// SQLite store for Agent mode
pub struct SqliteStore {
    pool: SqlitePool,
    llm_logger: LlmCallLogger,
    event_logger: EventLogger,
    retention_manager: LogRetentionManager,
}

impl SqliteStore {
    pub async fn new(database_url: &str) -> Result<Self, String> {
        let pool = SqlitePool::connect(database_url)
            .await
            .map_err(|e| format!("Failed to connect to database: {}", e))?;

        let llm_logger = LlmCallLogger::new(pool.clone());
        let event_logger = EventLogger::new(pool.clone());
        let retention_manager = LogRetentionManager::new(pool.clone());

        Ok(Self {
            pool,
            llm_logger,
            event_logger,
            retention_manager,
        })
    }

    /// Initialize database schema
    pub async fn init_schema(&self) -> Result<(), String> {
        self.init_logging_schema().await?;
        self.init_agent_schema().await?;

        Ok(())
    }

    /// Initialize only global logging tables.
    pub async fn init_logging_schema(&self) -> Result<(), String> {
        // Initialize logging tables
        self.llm_logger.init_schema().await?;
        self.event_logger.init_schema().await?;
        self.retention_manager.init_schema().await?;

        Ok(())
    }

    /// Initialize Agent mode specific tables
    async fn init_agent_schema(&self) -> Result<(), String> {
        sqlx::query(
            r#"
            -- World table
            CREATE TABLE IF NOT EXISTS worlds (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                settings_json TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- Character table (Agent mode)
            CREATE TABLE IF NOT EXISTS agent_characters (
                id TEXT PRIMARY KEY,
                world_id TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                avatar_url TEXT,
                system_prompt TEXT,
                personality_json TEXT,
                attributes_json TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (world_id) REFERENCES worlds(id)
            );

            -- Scene table
            CREATE TABLE IF NOT EXISTS scenes (
                id TEXT PRIMARY KEY,
                world_id TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL DEFAULT 'draft',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (world_id) REFERENCES worlds(id)
            );

            -- Scene turn table
            CREATE TABLE IF NOT EXISTS scene_turns (
                id TEXT PRIMARY KEY,
                scene_id TEXT NOT NULL,
                turn_number INTEGER NOT NULL,
                input_json TEXT,
                output_json TEXT,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (scene_id) REFERENCES scenes(id)
            );

            -- Runtime config snapshot table
            CREATE TABLE IF NOT EXISTS runtime_config_snapshots (
                id TEXT PRIMARY KEY,
                world_id TEXT NOT NULL,
                config_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (world_id) REFERENCES worlds(id)
            );

            -- World rules snapshot table
            CREATE TABLE IF NOT EXISTS world_rules_snapshots (
                id TEXT PRIMARY KEY,
                world_id TEXT NOT NULL,
                rules_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (world_id) REFERENCES worlds(id)
            );

            -- Create indexes
            CREATE INDEX IF NOT EXISTS idx_characters_world_id ON agent_characters(world_id);
            CREATE INDEX IF NOT EXISTS idx_scenes_world_id ON scenes(world_id);
            CREATE INDEX IF NOT EXISTS idx_scene_turns_scene_id ON scene_turns(scene_id);
            CREATE INDEX IF NOT EXISTS idx_runtime_snapshots_world_id ON runtime_config_snapshots(world_id);
            CREATE INDEX IF NOT EXISTS idx_rules_snapshots_world_id ON world_rules_snapshots(world_id);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create Agent schema: {}", e))?;

        Ok(())
    }

    /// Get database pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Get LLM logger
    pub fn llm_logger(&self) -> &LlmCallLogger {
        &self.llm_logger
    }

    /// Get event logger
    pub fn event_logger(&self) -> &EventLogger {
        &self.event_logger
    }

    /// Get retention manager
    pub fn retention_manager(&self) -> &LogRetentionManager {
        &self.retention_manager
    }
}
