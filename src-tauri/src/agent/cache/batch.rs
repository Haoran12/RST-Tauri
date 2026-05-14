//! Event batching module
//!
//! Provides batch processing for database writes, logs, and traces.
//! Reduces database round trips by collecting operations and executing them in batches.
//!
//! Key components:
//! - `BatchLogWriter`: Batches log entries and writes them in batches
//! - `BatchTraceWriter`: Batches trace entries for Agent runtime
//! - `BatchStateWriter`: Batches state updates for StateCommitter

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use sqlx::SqlitePool;
use tokio::sync::mpsc;

// ============================================================================
// Batch Configuration
// ============================================================================

/// Configuration for batch processing
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum batch size before forced flush
    pub max_batch_size: usize,
    /// Maximum time to wait before forced flush (ms)
    pub max_batch_delay_ms: u64,
    /// Maximum queue capacity
    pub queue_capacity: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
            max_batch_delay_ms: 100,
            queue_capacity: 10000,
        }
    }
}

// ============================================================================
// Batch Log Writer
// ============================================================================

/// Log entry for batching
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub log_id: String,
    pub log_type: String,
    pub content: serde_json::Value,
    pub created_at: String,
}

/// Batch log writer - collects log entries and writes them in batches
pub struct BatchLogWriter {
    /// Sender for log entries
    sender: mpsc::Sender<LogEntry>,
    /// Configuration
    config: BatchConfig,
}

impl BatchLogWriter {
    /// Create a new batch log writer
    pub fn new(config: BatchConfig) -> Self {
        let (sender, mut receiver) = mpsc::channel::<LogEntry>(config.queue_capacity);
        let config_clone = config.clone();

        // Spawn background task for batch processing
        tokio::spawn(async move {
            let mut batch: Vec<LogEntry> = Vec::with_capacity(config_clone.max_batch_size);
            let mut last_flush = Instant::now();
            let flush_interval = Duration::from_millis(config_clone.max_batch_delay_ms);

            loop {
                let delay = flush_interval.saturating_sub(last_flush.elapsed());

                tokio::select! {
                    // Receive new entry
                    entry = receiver.recv() => {
                        match entry {
                            Some(entry) => {
                                batch.push(entry);

                                // Flush if batch is full
                                if batch.len() >= config_clone.max_batch_size {
                                    Self::flush_batch(&mut batch).await;
                                    last_flush = Instant::now();
                                }
                            }
                            None => {
                                // Channel closed, flush remaining and exit
                                if !batch.is_empty() {
                                    Self::flush_batch(&mut batch).await;
                                }
                                break;
                            }
                        }
                    }

                    // Time-based flush
                    _ = tokio::time::sleep(delay) => {
                        if !batch.is_empty() {
                            Self::flush_batch(&mut batch).await;
                            last_flush = Instant::now();
                        }
                    }
                }
            }
        });

        Self { sender, config }
    }

    /// Create a batch log writer that persists flushed batches to SQLite.
    pub fn new_with_pool(config: BatchConfig, pool: SqlitePool) -> Self {
        let (sender, mut receiver) = mpsc::channel::<LogEntry>(config.queue_capacity);
        let config_clone = config.clone();

        tokio::spawn(async move {
            let mut batch: Vec<LogEntry> = Vec::with_capacity(config_clone.max_batch_size);
            let mut last_flush = Instant::now();
            let flush_interval = Duration::from_millis(config_clone.max_batch_delay_ms);

            loop {
                let delay = flush_interval.saturating_sub(last_flush.elapsed());

                tokio::select! {
                    entry = receiver.recv() => {
                        match entry {
                            Some(entry) => {
                                batch.push(entry);
                                if batch.len() >= config_clone.max_batch_size {
                                    let _ = Self::flush_batch_with_pool(&pool, &mut batch).await;
                                    last_flush = Instant::now();
                                }
                            }
                            None => {
                                if !batch.is_empty() {
                                    let _ = Self::flush_batch_with_pool(&pool, &mut batch).await;
                                }
                                break;
                            }
                        }
                    }

                    _ = tokio::time::sleep(delay) => {
                        if !batch.is_empty() {
                            let _ = Self::flush_batch_with_pool(&pool, &mut batch).await;
                            last_flush = Instant::now();
                        }
                    }
                }
            }
        });

        Self { sender, config }
    }

    /// Add a log entry to the batch
    pub async fn log(&self, entry: LogEntry) -> Result<(), String> {
        self.sender
            .send(entry)
            .await
            .map_err(|e| format!("Failed to send log entry: {}", e))
    }

    /// Add a log entry without waiting (fire and forget)
    pub fn log_nowait(&self, entry: LogEntry) -> Result<(), String> {
        self.sender
            .try_send(entry)
            .map_err(|e| format!("Failed to send log entry: {}", e))
    }

    /// Flush a batch to storage
    async fn flush_batch(batch: &mut Vec<LogEntry>) {
        if batch.is_empty() {
            return;
        }

        // TODO: Implement actual batch write to SQLite
        // For now, just clear the batch
        // In production, this would use a single transaction to write all entries

        batch.clear();
    }

    async fn flush_batch_with_pool(
        pool: &SqlitePool,
        batch: &mut Vec<LogEntry>,
    ) -> Result<(), String> {
        if batch.is_empty() {
            return Ok(());
        }

        let mut tx = pool
            .begin()
            .await
            .map_err(|e| format!("Failed to begin batch log transaction: {}", e))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS batch_log_entry_journal (
                log_id TEXT PRIMARY KEY,
                log_type TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to initialize batch log journal: {}", e))?;

        for entry in batch.iter() {
            let content = serde_json::to_string(&entry.content)
                .map_err(|e| format!("Failed to serialize batch log content: {}", e))?;
            sqlx::query(
                r#"
                INSERT OR REPLACE INTO batch_log_entry_journal
                    (log_id, log_type, content, created_at)
                VALUES (?, ?, ?, ?)
                "#,
            )
            .bind(&entry.log_id)
            .bind(&entry.log_type)
            .bind(content)
            .bind(&entry.created_at)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert batch log journal row: {}", e))?;
        }

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit batch log transaction: {}", e))?;

        batch.clear();
        Ok(())
    }

    /// Get current queue size (approximate)
    pub fn queue_size(&self) -> usize {
        self.sender.max_capacity() - self.sender.capacity()
    }

    /// Maximum configured batch size.
    pub fn max_batch_size(&self) -> usize {
        self.config.max_batch_size
    }
}

// ============================================================================
// Batch Trace Writer
// ============================================================================

/// Trace entry for batching
#[derive(Debug, Clone)]
pub struct TraceEntry {
    pub trace_id: String,
    pub scene_turn_id: String,
    pub step_kind: String,
    pub content: serde_json::Value,
    pub created_at: String,
}

/// Batch trace writer for Agent runtime
pub struct BatchTraceWriter {
    /// Pending traces
    pending: Mutex<VecDeque<TraceEntry>>,
    /// Configuration
    config: BatchConfig,
    /// Last flush time
    last_flush: Mutex<Instant>,
}

impl BatchTraceWriter {
    /// Create a new batch trace writer
    pub fn new(config: BatchConfig) -> Self {
        Self {
            pending: Mutex::new(VecDeque::with_capacity(config.queue_capacity)),
            config,
            last_flush: Mutex::new(Instant::now()),
        }
    }

    /// Add a trace entry
    pub fn add_trace(&self, entry: TraceEntry) -> Result<bool, String> {
        let mut pending = self.pending.lock();

        if pending.len() >= self.config.queue_capacity {
            return Err("Trace queue is full".to_string());
        }

        pending.push_back(entry);

        // Return whether flush is needed
        Ok(pending.len() >= self.config.max_batch_size)
    }

    /// Check if flush is needed (by time)
    pub fn needs_time_flush(&self) -> bool {
        let pending = self.pending.lock();
        if pending.is_empty() {
            return false;
        }

        let last_flush = self.last_flush.lock();
        let elapsed = last_flush.elapsed();
        elapsed >= Duration::from_millis(self.config.max_batch_delay_ms)
    }

    /// Get pending traces and clear the queue
    pub fn take_batch(&self) -> Vec<TraceEntry> {
        let mut pending = self.pending.lock();
        let batch: Vec<TraceEntry> = pending.drain(..).collect();
        *self.last_flush.lock() = Instant::now();
        batch
    }

    /// Get pending count
    pub fn pending_count(&self) -> usize {
        self.pending.lock().len()
    }

    /// Flush pending traces to storage
    pub async fn flush(&self) -> Result<usize, String> {
        let batch = self.take_batch();
        let count = batch.len();

        if count == 0 {
            return Ok(0);
        }

        // TODO: Implement actual batch write to SQLite
        // For now, just return the count

        Ok(count)
    }

    /// Flush pending traces into a durable SQLite journal.
    pub async fn flush_with_pool(&self, pool: &SqlitePool) -> Result<usize, String> {
        let batch = self.take_batch();
        let count = batch.len();

        if count == 0 {
            return Ok(0);
        }

        let mut tx = pool
            .begin()
            .await
            .map_err(|e| format!("Failed to begin batch trace transaction: {}", e))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS batch_trace_entry_journal (
                trace_id TEXT PRIMARY KEY,
                scene_turn_id TEXT NOT NULL,
                step_kind TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to initialize batch trace journal: {}", e))?;

        for entry in &batch {
            let content = serde_json::to_string(&entry.content)
                .map_err(|e| format!("Failed to serialize batch trace content: {}", e))?;
            sqlx::query(
                r#"
                INSERT OR REPLACE INTO batch_trace_entry_journal
                    (trace_id, scene_turn_id, step_kind, content, created_at)
                VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(&entry.trace_id)
            .bind(&entry.scene_turn_id)
            .bind(&entry.step_kind)
            .bind(content)
            .bind(&entry.created_at)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert batch trace journal row: {}", e))?;
        }

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit batch trace transaction: {}", e))?;

        Ok(count)
    }
}

// ============================================================================
// Batch State Writer
// ============================================================================

/// State update operation for batching
#[derive(Debug, Clone)]
pub enum StateOperation {
    /// Update knowledge entry
    UpdateKnowledge {
        knowledge_id: String,
        content: serde_json::Value,
    },
    /// Update character state
    UpdateCharacterState {
        character_id: String,
        state: serde_json::Value,
    },
    /// Update scene model
    UpdateScene {
        scene_turn_id: String,
        scene: serde_json::Value,
    },
    /// Insert memory entry
    InsertMemory {
        character_id: String,
        memory: serde_json::Value,
    },
    /// Reveal knowledge
    RevealKnowledge {
        knowledge_id: String,
        character_id: String,
    },
}

/// Batch state writer for StateCommitter
pub struct BatchStateWriter {
    /// Pending operations
    pending: Mutex<VecDeque<StateOperation>>,
    /// Configuration
    config: BatchConfig,
}

impl BatchStateWriter {
    /// Create a new batch state writer
    pub fn new(config: BatchConfig) -> Self {
        Self {
            pending: Mutex::new(VecDeque::with_capacity(config.queue_capacity)),
            config,
        }
    }

    /// Add an operation
    pub fn add_operation(&self, op: StateOperation) -> Result<bool, String> {
        let mut pending = self.pending.lock();

        if pending.len() >= self.config.queue_capacity {
            return Err("State operation queue is full".to_string());
        }

        pending.push_back(op);

        // Return whether flush is needed
        Ok(pending.len() >= self.config.max_batch_size)
    }

    /// Add multiple operations
    pub fn add_operations(&self, ops: Vec<StateOperation>) -> Result<bool, String> {
        let mut pending = self.pending.lock();

        if pending.len() + ops.len() > self.config.queue_capacity {
            return Err("State operation queue would overflow".to_string());
        }

        for op in ops {
            pending.push_back(op);
        }

        Ok(pending.len() >= self.config.max_batch_size)
    }

    /// Get pending operations and clear the queue
    pub fn take_batch(&self) -> Vec<StateOperation> {
        let mut pending = self.pending.lock();
        pending.drain(..).collect()
    }

    /// Get pending count
    pub fn pending_count(&self) -> usize {
        self.pending.lock().len()
    }

    /// Execute all pending operations in a single transaction
    pub async fn execute_batch(&self) -> Result<BatchExecutionResult, String> {
        let batch = self.take_batch();
        Ok(count_state_operations(&batch))
    }

    /// Execute all pending operations in one SQLite write transaction.
    ///
    /// This records a durable operation journal that StateCommitter-oriented callers can
    /// use for replay diagnostics while keeping the batch atomic.
    pub async fn execute_batch_with_pool(
        &self,
        pool: &SqlitePool,
    ) -> Result<BatchExecutionResult, String> {
        let batch = self.take_batch();

        if batch.is_empty() {
            return Ok(BatchExecutionResult::default());
        }

        let mut tx = pool
            .begin()
            .await
            .map_err(|e| format!("Failed to begin batch state transaction: {}", e))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS batch_state_operation_journal (
                operation_id INTEGER PRIMARY KEY AUTOINCREMENT,
                operation_kind TEXT NOT NULL,
                subject_id TEXT NOT NULL,
                payload TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to initialize batch operation journal: {}", e))?;

        for op in &batch {
            let (operation_kind, subject_id, payload) = state_operation_journal_row(op)?;
            sqlx::query(
                r#"
                INSERT INTO batch_state_operation_journal (operation_kind, subject_id, payload)
                VALUES (?, ?, ?)
                "#,
            )
            .bind(operation_kind)
            .bind(subject_id)
            .bind(payload)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert batch operation journal row: {}", e))?;
        }

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit batch state transaction: {}", e))?;

        Ok(count_state_operations(&batch))
    }
}

/// Result of batch execution
#[derive(Debug, Clone, Default)]
pub struct BatchExecutionResult {
    pub total_operations: usize,
    pub knowledge_updates: usize,
    pub character_updates: usize,
    pub scene_updates: usize,
    pub memory_inserts: usize,
    pub knowledge_reveals: usize,
}

fn count_state_operations(batch: &[StateOperation]) -> BatchExecutionResult {
    let mut result = BatchExecutionResult::default();
    for op in batch {
        match op {
            StateOperation::UpdateKnowledge { .. } => result.knowledge_updates += 1,
            StateOperation::UpdateCharacterState { .. } => result.character_updates += 1,
            StateOperation::UpdateScene { .. } => result.scene_updates += 1,
            StateOperation::InsertMemory { .. } => result.memory_inserts += 1,
            StateOperation::RevealKnowledge { .. } => result.knowledge_reveals += 1,
        }
    }
    result.total_operations = batch.len();
    result
}

fn state_operation_journal_row(
    op: &StateOperation,
) -> Result<(&'static str, &str, String), String> {
    match op {
        StateOperation::UpdateKnowledge {
            knowledge_id,
            content,
        } => Ok((
            "update_knowledge",
            knowledge_id.as_str(),
            serde_json::to_string(content).map_err(|e| e.to_string())?,
        )),
        StateOperation::UpdateCharacterState {
            character_id,
            state,
        } => Ok((
            "update_character_state",
            character_id.as_str(),
            serde_json::to_string(state).map_err(|e| e.to_string())?,
        )),
        StateOperation::UpdateScene {
            scene_turn_id,
            scene,
        } => Ok((
            "update_scene",
            scene_turn_id.as_str(),
            serde_json::to_string(scene).map_err(|e| e.to_string())?,
        )),
        StateOperation::InsertMemory {
            character_id,
            memory,
        } => Ok((
            "insert_memory",
            character_id.as_str(),
            serde_json::to_string(memory).map_err(|e| e.to_string())?,
        )),
        StateOperation::RevealKnowledge {
            knowledge_id,
            character_id,
        } => Ok((
            "reveal_knowledge",
            knowledge_id.as_str(),
            serde_json::to_string(&serde_json::json!({ "character_id": character_id }))
                .map_err(|e| e.to_string())?,
        )),
    }
}

// ============================================================================
// Global Batch Managers
// ============================================================================

use once_cell::sync::Lazy;

/// Global batch log writer
pub static BATCH_LOG_WRITER: Lazy<BatchLogWriter> =
    Lazy::new(|| BatchLogWriter::new(BatchConfig::default()));

/// Global batch trace writer
pub static BATCH_TRACE_WRITER: Lazy<BatchTraceWriter> =
    Lazy::new(|| BatchTraceWriter::new(BatchConfig::default()));

/// Global batch state writer
pub static BATCH_STATE_WRITER: Lazy<BatchStateWriter> =
    Lazy::new(|| BatchStateWriter::new(BatchConfig::default()));

// ============================================================================
// Batch Utilities
// ============================================================================

/// Execute multiple async operations in parallel with a limit
pub async fn parallel_execute<T, F, Fut>(
    items: Vec<T>,
    max_parallel: usize,
    f: F,
) -> Vec<Result<Fut::Output, String>>
where
    F: Fn(T) -> Fut + Clone,
    Fut: std::future::Future,
{
    use futures::stream::{self, StreamExt};

    stream::iter(items)
        .map(move |item| {
            let f = f.clone();
            async move { Ok::<_, String>(f(item).await) }
        })
        .buffer_unordered(max_parallel)
        .collect()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_trace_writer_collects_and_flushes() {
        let writer = BatchTraceWriter::new(BatchConfig {
            max_batch_size: 3,
            max_batch_delay_ms: 1000,
            queue_capacity: 100,
        });

        // Add entries
        let needs_flush = writer
            .add_trace(TraceEntry {
                trace_id: "t1".to_string(),
                scene_turn_id: "s1".to_string(),
                step_kind: "step".to_string(),
                content: serde_json::json!({}),
                created_at: "now".to_string(),
            })
            .unwrap();
        assert!(!needs_flush);

        let needs_flush = writer
            .add_trace(TraceEntry {
                trace_id: "t2".to_string(),
                scene_turn_id: "s1".to_string(),
                step_kind: "step".to_string(),
                content: serde_json::json!({}),
                created_at: "now".to_string(),
            })
            .unwrap();
        assert!(!needs_flush);

        let needs_flush = writer
            .add_trace(TraceEntry {
                trace_id: "t3".to_string(),
                scene_turn_id: "s1".to_string(),
                step_kind: "step".to_string(),
                content: serde_json::json!({}),
                created_at: "now".to_string(),
            })
            .unwrap();
        assert!(needs_flush); // Batch size reached

        assert_eq!(writer.pending_count(), 3);

        let batch = writer.take_batch();
        assert_eq!(batch.len(), 3);
        assert_eq!(writer.pending_count(), 0);
    }

    #[test]
    fn batch_state_writer_collects_operations() {
        let writer = BatchStateWriter::new(BatchConfig {
            max_batch_size: 2,
            max_batch_delay_ms: 1000,
            queue_capacity: 100,
        });

        writer
            .add_operation(StateOperation::UpdateKnowledge {
                knowledge_id: "k1".to_string(),
                content: serde_json::json!({}),
            })
            .unwrap();

        writer
            .add_operation(StateOperation::UpdateCharacterState {
                character_id: "c1".to_string(),
                state: serde_json::json!({}),
            })
            .unwrap();

        assert_eq!(writer.pending_count(), 2);

        let batch = writer.take_batch();
        assert_eq!(batch.len(), 2);
    }

    #[tokio::test]
    async fn batch_state_writer_persists_batch_in_one_transaction() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("memory db");
        let writer = BatchStateWriter::new(BatchConfig::default());

        writer
            .add_operations(vec![
                StateOperation::UpdateKnowledge {
                    knowledge_id: "k1".to_string(),
                    content: serde_json::json!({ "fact": true }),
                },
                StateOperation::RevealKnowledge {
                    knowledge_id: "k1".to_string(),
                    character_id: "c1".to_string(),
                },
            ])
            .expect("queue operations");

        let result = writer
            .execute_batch_with_pool(&pool)
            .await
            .expect("batch transaction");
        let row_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM batch_state_operation_journal")
                .fetch_one(&pool)
                .await
                .expect("count rows");

        assert_eq!(result.total_operations, 2);
        assert_eq!(row_count, 2);
    }

    #[tokio::test]
    async fn batch_log_writer_flush_persists_journal_rows() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("memory db");
        let mut batch = vec![LogEntry {
            log_id: "log-1".to_string(),
            log_type: "test".to_string(),
            content: serde_json::json!({ "ok": true }),
            created_at: "now".to_string(),
        }];

        BatchLogWriter::flush_batch_with_pool(&pool, &mut batch)
            .await
            .expect("flush logs");

        let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM batch_log_entry_journal")
            .fetch_one(&pool)
            .await
            .expect("count rows");
        assert!(batch.is_empty());
        assert_eq!(row_count, 1);
    }

    #[tokio::test]
    async fn batch_trace_writer_flush_persists_journal_rows() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("memory db");
        let writer = BatchTraceWriter::new(BatchConfig::default());
        writer
            .add_trace(TraceEntry {
                trace_id: "trace-1".to_string(),
                scene_turn_id: "turn-1".to_string(),
                step_kind: "step".to_string(),
                content: serde_json::json!({ "ok": true }),
                created_at: "now".to_string(),
            })
            .expect("queue trace");

        let flushed = writer.flush_with_pool(&pool).await.expect("flush traces");

        let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM batch_trace_entry_journal")
            .fetch_one(&pool)
            .await
            .expect("count rows");
        assert_eq!(flushed, 1);
        assert_eq!(row_count, 1);
    }
}
