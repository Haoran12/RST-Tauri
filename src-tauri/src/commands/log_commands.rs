//! Tauri commands for runtime logs and Agent Trace inspection.

use crate::storage::paths::{app_data_root, validate_path_component};
use crate::AppState;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, State};

const DEFAULT_PAGE_SIZE: i64 = 50;
const MAX_PAGE_SIZE: i64 = 200;
const EXPORT_LIMIT: i64 = 1000;
const GLOBAL_LOG_LIMIT_BYTES: u64 = 1024 * 1024 * 1024;
const WORLD_STALE_DAYS: i64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRecordFilter {
    pub source_scope: Option<String>,
    pub record_kind: Option<String>,
    pub level: Option<String>,
    pub status: Option<String>,
    pub mode: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub world_id: Option<String>,
    pub session_id: Option<String>,
    pub scene_turn_id: Option<String>,
    pub trace_id: Option<String>,
    pub request_id: Option<String>,
    pub character_id: Option<String>,
    pub llm_node: Option<String>,
    pub search: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogPageInput {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogSourceRef {
    pub source_kind: String,
    pub world_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRecordRef {
    pub record_kind: String,
    pub source: LogSourceRef,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRecordSummary {
    pub record_ref: LogRecordRef,
    pub created_at: String,
    pub title: String,
    pub summary: Option<String>,
    pub status: Option<String>,
    pub level: Option<String>,
    pub mode: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub world_id: Option<String>,
    pub session_id: Option<String>,
    pub scene_turn_id: Option<String>,
    pub trace_id: Option<String>,
    pub request_id: Option<String>,
    pub character_id: Option<String>,
    pub llm_node: Option<String>,
    pub latency_ms: Option<i64>,
    pub token_usage: Option<serde_json::Value>,
    pub stream_chunk_count: Option<i64>,
    pub step_count: Option<i64>,
    pub protected: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRecordPage {
    pub records: Vec<LogRecordSummary>,
    pub offset: i64,
    pub limit: i64,
    pub has_more: bool,
    pub total_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmLogDetail {
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
    pub latency_ms: Option<i64>,
    pub token_usage: Option<serde_json::Value>,
    pub retry_count: i64,
    pub error_summary: Option<String>,
    pub redaction_applied: bool,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLogDetail {
    pub event_id: String,
    pub level: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStepDetail {
    pub step_trace_id: String,
    pub trace_id: String,
    pub scene_turn_id: String,
    pub character_id: Option<String>,
    pub step_name: String,
    pub step_status: String,
    pub input_summary: Option<serde_json::Value>,
    pub output_summary: Option<serde_json::Value>,
    pub decision_json: Option<serde_json::Value>,
    pub linked_request_id: Option<String>,
    pub error_event_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceDetail {
    pub trace_id: String,
    pub scene_turn_id: String,
    pub session_id: Option<String>,
    pub story_time_anchor: Option<serde_json::Value>,
    pub runtime_turn_status: String,
    pub trace_kind: String,
    pub character_id: Option<String>,
    pub runtime_config_snapshot_id: String,
    pub world_rules_snapshot_id: Option<String>,
    pub summary: serde_json::Value,
    pub linked_request_ids: serde_json::Value,
    pub linked_event_ids: serde_json::Value,
    pub created_at: String,
    pub steps: Vec<TraceStepDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRecordDetail {
    pub record_ref: LogRecordRef,
    pub llm: Option<LlmLogDetail>,
    pub event: Option<EventLogDetail>,
    pub trace: Option<TraceDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunkPage {
    pub chunks: Vec<StreamChunkDetail>,
    pub offset: i64,
    pub limit: i64,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunkDetail {
    pub chunk_id: String,
    pub request_id: String,
    pub chunk_index: i64,
    pub raw_chunk: String,
    pub received_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogStorageSummary {
    pub global: LogScopeStorageSummary,
    pub worlds: Vec<LogScopeStorageSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogScopeStorageSummary {
    pub scope: String,
    pub world_id: Option<String>,
    pub size_bytes: u64,
    pub size_limit_bytes: Option<u64>,
    pub llm_count: i64,
    pub event_count: i64,
    pub trace_count: i64,
    pub stream_chunk_count: i64,
    pub last_updated_at: Option<String>,
    pub stale_prompt_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportLogsResult {
    pub format: String,
    pub filename: String,
    pub content: String,
    pub record_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogCleanupPreviewInput {
    pub scope: Option<String>,
    pub older_than_days: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogCleanupPreview {
    pub plan_id: String,
    pub scope: String,
    pub older_than_days: i64,
    pub llm_logs_to_delete: i64,
    pub event_logs_to_delete: i64,
    pub stream_chunks_affected: i64,
    pub protected_trace_records: i64,
    pub notes: Vec<String>,
}

#[tauri::command]
pub async fn query_log_records(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    filter: LogRecordFilter,
    page: Option<LogPageInput>,
) -> Result<LogRecordPage, String> {
    let (offset, limit) = normalize_page(page);

    // 先获取总数（不带分页限制）
    let all_records = collect_log_records(&app, state.inner(), &filter, i64::MAX).await?;
    let total_count = all_records.len() as i64;

    // 排序后分页
    let mut sorted_records = all_records;
    sorted_records.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let has_more = total_count > offset + limit;
    let records = sorted_records
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect();

    Ok(LogRecordPage {
        records,
        offset,
        limit,
        has_more,
        total_count,
    })
}

#[tauri::command]
pub async fn get_log_record_detail(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    record_ref: LogRecordRef,
) -> Result<LogRecordDetail, String> {
    match record_ref.record_kind.as_str() {
        "llm" => {
            let pool = pool_for_source(&app, state.inner(), &record_ref.source).await?;
            let llm = get_llm_detail(&pool, &record_ref.id).await?;
            Ok(LogRecordDetail {
                record_ref,
                llm,
                event: None,
                trace: None,
            })
        }
        "event" => {
            let pool = pool_for_source(&app, state.inner(), &record_ref.source).await?;
            let event = get_event_detail(&pool, &record_ref.id).await?;
            Ok(LogRecordDetail {
                record_ref,
                llm: None,
                event,
                trace: None,
            })
        }
        "trace" => {
            let world_id = record_ref
                .source
                .world_id
                .clone()
                .ok_or_else(|| "Trace detail requires world_id".to_string())?;
            let trace = Some(get_trace_detail_inner(&app, &world_id, &record_ref.id).await?);
            Ok(LogRecordDetail {
                record_ref,
                llm: None,
                event: None,
                trace,
            })
        }
        other => Err(format!("Unsupported log record kind: {}", other)),
    }
}

#[tauri::command]
pub async fn get_stream_chunks(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    request_id: String,
    source: LogSourceRef,
    page: Option<LogPageInput>,
) -> Result<StreamChunkPage, String> {
    let (offset, limit) = normalize_page(page);
    let pool = pool_for_source(&app, state.inner(), &source).await?;
    let rows = sqlx::query(
        r#"
        SELECT chunk_id, request_id, chunk_index, raw_chunk, received_at
        FROM llm_stream_chunks
        WHERE request_id = ?
        ORDER BY chunk_index ASC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(&request_id)
    .bind(limit + 1)
    .bind(offset)
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to query stream chunks: {}", e))?;

    let has_more = rows.len() as i64 > limit;
    let chunks = rows
        .into_iter()
        .take(limit as usize)
        .map(|row| StreamChunkDetail {
            chunk_id: row.get("chunk_id"),
            request_id: row.get("request_id"),
            chunk_index: row.get("chunk_index"),
            raw_chunk: row.get("raw_chunk"),
            received_at: row.get("received_at"),
        })
        .collect();

    Ok(StreamChunkPage {
        chunks,
        offset,
        limit,
        has_more,
    })
}

#[tauri::command]
pub async fn get_trace_detail(
    app: AppHandle,
    world_id: String,
    trace_id: String,
) -> Result<TraceDetail, String> {
    get_trace_detail_inner(&app, &world_id, &trace_id).await
}

#[tauri::command]
pub async fn get_log_storage_summary(app: AppHandle) -> Result<LogStorageSummary, String> {
    let data_root = app_data_root(&app)?;
    let global_path = data_root.join("logs").join("app_logs.sqlite");
    let global = summarize_log_database("global", None, &global_path, Some(GLOBAL_LOG_LIMIT_BYTES))
        .await?;

    let mut worlds = Vec::new();
    let worlds_root = data_root.join("worlds");
    if worlds_root.exists() {
        let entries = std::fs::read_dir(&worlds_root)
            .map_err(|e| format!("Failed to read worlds directory: {}", e))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read world entry: {}", e))?;
            if !entry
                .file_type()
                .map_err(|e| format!("Failed to read world entry type: {}", e))?
                .is_dir()
            {
                continue;
            }
            let world_id = entry.file_name().to_string_lossy().to_string();
            if validate_path_component(&world_id).is_err() {
                continue;
            }
            let path = entry.path().join("world.sqlite");
            if path.exists() {
                worlds.push(
                    summarize_log_database("world", Some(world_id), &path, None).await?,
                );
            }
        }
    }

    worlds.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    Ok(LogStorageSummary { global, worlds })
}

#[tauri::command]
pub async fn export_logs(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    filter: LogRecordFilter,
    format: Option<String>,
) -> Result<ExportLogsResult, String> {
    let format = format.unwrap_or_else(|| "json".to_string());
    let records = collect_log_records(&app, state.inner(), &filter, EXPORT_LIMIT).await?;
    let filename = format!("rst-logs-{}.{}", Utc::now().format("%Y%m%d-%H%M%S"), format);
    let content = match format.as_str() {
        "jsonl" => records
            .iter()
            .map(|record| serde_json::to_string(record).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n"),
        "csv" => export_csv(&records),
        "json" => serde_json::to_string_pretty(&records)
            .map_err(|e| format!("Failed to serialize log export: {}", e))?,
        other => return Err(format!("Unsupported export format: {}", other)),
    };

    Ok(ExportLogsResult {
        format,
        filename,
        content,
        record_count: records.len(),
    })
}

#[tauri::command]
pub async fn run_log_retention_now(
    state: State<'_, Arc<AppState>>,
    scope: Option<String>,
) -> Result<crate::logging::retention::LogRetentionResult, String> {
    if scope.as_deref().unwrap_or("global") != "global" {
        return Err("Only global log retention can be run automatically".to_string());
    }
    let store_guard = state.sqlite_store.read().await;
    let store = store_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;
    store.retention_manager().check_retention().await
}

#[tauri::command]
pub async fn preview_log_cleanup(
    state: State<'_, Arc<AppState>>,
    input: Option<LogCleanupPreviewInput>,
) -> Result<LogCleanupPreview, String> {
    let input = input.unwrap_or(LogCleanupPreviewInput {
        scope: Some("global".to_string()),
        older_than_days: Some(30),
    });
    let scope = input.scope.unwrap_or_else(|| "global".to_string());
    if scope != "global" {
        return Err("Only global log cleanup preview is supported in MVP".to_string());
    }

    let older_than_days = input.older_than_days.unwrap_or(30).clamp(1, 3650);
    let cutoff = (Utc::now() - Duration::days(older_than_days)).to_rfc3339();
    let store_guard = state.sqlite_store.read().await;
    let store = store_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let llm_logs_to_delete = count_where(
        store.pool(),
        "SELECT COUNT(*) AS count FROM llm_call_logs WHERE created_at < ?",
        &cutoff,
    )
    .await?;
    let event_logs_to_delete = count_where(
        store.pool(),
        "SELECT COUNT(*) AS count FROM app_event_logs WHERE created_at < ?",
        &cutoff,
    )
    .await?;
    let stream_chunks_affected = count_where(
        store.pool(),
        r#"
        SELECT COUNT(*) AS count
        FROM llm_stream_chunks
        WHERE request_id IN (SELECT request_id FROM llm_call_logs WHERE created_at < ?)
        "#,
        &cutoff,
    )
    .await
    .unwrap_or(0);

    Ok(LogCleanupPreview {
        plan_id: uuid::Uuid::new_v4().to_string(),
        scope,
        older_than_days,
        llm_logs_to_delete,
        event_logs_to_delete,
        stream_chunks_affected,
        protected_trace_records: 0,
        notes: vec![
            "仅清理全局运行 Logs，不自动删除 World 内 Agent Trace".to_string(),
            "凭证脱敏发生在写入层；导出和清理不会提供未脱敏内容入口".to_string(),
        ],
    })
}

#[tauri::command]
pub async fn confirm_log_cleanup(
    state: State<'_, Arc<AppState>>,
    plan_id: String,
    scope: Option<String>,
) -> Result<crate::logging::retention::LogRetentionResult, String> {
    if plan_id.trim().is_empty() {
        return Err("Cleanup plan_id must not be empty".to_string());
    }
    run_log_retention_now(state, scope).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetLogProtectionInput {
    pub record_kind: String,
    pub record_id: String,
    pub protected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetLogProtectionResult {
    pub record_kind: String,
    pub record_id: String,
    pub protected: bool,
}

#[tauri::command]
pub async fn set_log_protection(
    state: State<'_, Arc<AppState>>,
    input: SetLogProtectionInput,
) -> Result<SetLogProtectionResult, String> {
    let store_guard = state.sqlite_store.read().await;
    let store = store_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let protected_value = if input.protected { 1 } else { 0 };

    match input.record_kind.as_str() {
        "llm" => {
            let result = sqlx::query("UPDATE llm_call_logs SET protected = ? WHERE request_id = ?")
                .bind(protected_value)
                .bind(&input.record_id)
                .execute(store.pool())
                .await
                .map_err(|e| format!("Failed to update LLM log protection: {}", e))?;

            if result.rows_affected() == 0 {
                return Err(format!("LLM log not found: {}", input.record_id));
            }
        }
        "event" => {
            let result = sqlx::query("UPDATE app_event_logs SET protected = ? WHERE event_id = ?")
                .bind(protected_value)
                .bind(&input.record_id)
                .execute(store.pool())
                .await
                .map_err(|e| format!("Failed to update event log protection: {}", e))?;

            if result.rows_affected() == 0 {
                return Err(format!("Event log not found: {}", input.record_id));
            }
        }
        other => return Err(format!("Unsupported log kind for protection: {}", other)),
    }

    Ok(SetLogProtectionResult {
        record_kind: input.record_kind,
        record_id: input.record_id,
        protected: input.protected,
    })
}

#[tauri::command]
pub async fn get_log_protection(
    state: State<'_, Arc<AppState>>,
    record_kind: String,
    record_id: String,
) -> Result<bool, String> {
    let store_guard = state.sqlite_store.read().await;
    let store = store_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    use sqlx::Row;

    match record_kind.as_str() {
        "llm" => {
            let row = sqlx::query("SELECT protected FROM llm_call_logs WHERE request_id = ?")
                .bind(&record_id)
                .fetch_optional(store.pool())
                .await
                .map_err(|e| format!("Failed to get LLM log protection: {}", e))?;

            match row {
                Some(r) => Ok(r.get::<i64, _>("protected") != 0),
                None => Err(format!("LLM log not found: {}", record_id)),
            }
        }
        "event" => {
            let row = sqlx::query("SELECT protected FROM app_event_logs WHERE event_id = ?")
                .bind(&record_id)
                .fetch_optional(store.pool())
                .await
                .map_err(|e| format!("Failed to get event log protection: {}", e))?;

            match row {
                Some(r) => Ok(r.get::<i64, _>("protected") != 0),
                None => Err(format!("Event log not found: {}", record_id)),
            }
        }
        other => Err(format!("Unsupported log kind for protection: {}", other)),
    }
}

async fn collect_log_records(
    app: &AppHandle,
    state: &Arc<AppState>,
    filter: &LogRecordFilter,
    source_limit: i64,
) -> Result<Vec<LogRecordSummary>, String> {
    validate_filter_ids(filter)?;
    let mut records = Vec::new();
    let scope = filter.source_scope.as_deref().unwrap_or("all");

    if matches!(scope, "all" | "global") && filter.world_id.is_none() {
        let store_guard = state.sqlite_store.read().await;
        if let Some(store) = store_guard.as_ref() {
            let source = LogSourceRef {
                source_kind: "global".to_string(),
                world_id: None,
            };
            collect_pool_records(store.pool(), &source, filter, source_limit, &mut records).await?;
        }
    }

    if matches!(scope, "all" | "world" | "trace") {
        let worlds = world_log_targets(app, filter.world_id.as_deref())?;
        for (world_id, path) in worlds {
            let pool = open_sqlite_pool(&path).await?;
            if matches!(scope, "all" | "world") {
                let source = LogSourceRef {
                    source_kind: "world".to_string(),
                    world_id: Some(world_id.clone()),
                };
                collect_pool_records(&pool, &source, filter, source_limit, &mut records).await?;
            }
            if matches!(scope, "all" | "trace") {
                let source = LogSourceRef {
                    source_kind: "trace".to_string(),
                    world_id: Some(world_id.clone()),
                };
                collect_trace_records(&pool, &source, filter, source_limit, &mut records).await?;
            }
        }
    }

    Ok(records)
}

async fn collect_pool_records(
    pool: &SqlitePool,
    source: &LogSourceRef,
    filter: &LogRecordFilter,
    limit: i64,
    records: &mut Vec<LogRecordSummary>,
) -> Result<(), String> {
    if filter.record_kind.as_deref().is_none()
        || matches!(filter.record_kind.as_deref(), Some("all" | "llm"))
    {
        records.extend(query_llm_summaries(pool, source, filter, limit).await?);
    }
    if filter.record_kind.as_deref().is_none()
        || matches!(filter.record_kind.as_deref(), Some("all" | "event"))
    {
        records.extend(query_event_summaries(pool, source, filter, limit).await?);
    }
    Ok(())
}

async fn query_llm_summaries(
    pool: &SqlitePool,
    source: &LogSourceRef,
    filter: &LogRecordFilter,
    limit: i64,
) -> Result<Vec<LogRecordSummary>, String> {
    let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(
        r#"
        SELECT request_id, mode, world_id, session_id, scene_turn_id, trace_id,
               character_id, llm_node, provider, model, call_type, status,
               latency_ms, token_usage, error_summary, created_at, protected,
               (SELECT COUNT(*) FROM llm_stream_chunks c WHERE c.request_id = l.request_id) AS stream_chunk_count
        FROM llm_call_logs l
        "#,
    );
    append_llm_where(&mut builder, filter);
    builder.push(" ORDER BY created_at DESC LIMIT ");
    builder.push_bind(limit);

    let rows = builder
        .build()
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to query LLM logs: {}", e))?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let request_id: String = row.get("request_id");
            let provider: String = row.get("provider");
            let model: String = row.get("model");
            let llm_node: String = row.get("llm_node");
            let protected: Option<i64> = row.get("protected");
            LogRecordSummary {
                record_ref: LogRecordRef {
                    record_kind: "llm".to_string(),
                    source: source.clone(),
                    id: request_id.clone(),
                },
                created_at: row.get("created_at"),
                title: format!("{} / {}", provider, model),
                summary: Some(format!("{} · {}", llm_node, row.get::<String, _>("call_type"))),
                status: Some(row.get("status")),
                level: None,
                mode: Some(row.get("mode")),
                provider: Some(provider),
                model: Some(model),
                world_id: row.get("world_id"),
                session_id: row.get("session_id"),
                scene_turn_id: row.get("scene_turn_id"),
                trace_id: row.get("trace_id"),
                request_id: Some(request_id),
                character_id: row.get("character_id"),
                llm_node: Some(llm_node),
                latency_ms: row.get("latency_ms"),
                token_usage: parse_optional_json(row.get::<Option<String>, _>("token_usage")),
                stream_chunk_count: Some(row.get("stream_chunk_count")),
                step_count: None,
                protected: protected.map(|p| p != 0),
            }
        })
        .collect())
}

async fn query_event_summaries(
    pool: &SqlitePool,
    source: &LogSourceRef,
    filter: &LogRecordFilter,
    limit: i64,
) -> Result<Vec<LogRecordSummary>, String> {
    let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(
        r#"
        SELECT event_id, level, event_type, message, source_module, request_id,
               world_id, session_id, scene_turn_id, trace_id, character_id, created_at, protected
        FROM app_event_logs e
        "#,
    );
    append_event_where(&mut builder, filter);
    builder.push(" ORDER BY created_at DESC LIMIT ");
    builder.push_bind(limit);

    let rows = builder
        .build()
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to query event logs: {}", e))?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let event_id: String = row.get("event_id");
            let protected: Option<i64> = row.get("protected");
            LogRecordSummary {
                record_ref: LogRecordRef {
                    record_kind: "event".to_string(),
                    source: source.clone(),
                    id: event_id,
                },
                created_at: row.get("created_at"),
                title: row.get("event_type"),
                summary: Some(row.get("message")),
                status: None,
                level: Some(row.get("level")),
                mode: Some("app".to_string()),
                provider: None,
                model: None,
                world_id: row.get("world_id"),
                session_id: row.get("session_id"),
                scene_turn_id: row.get("scene_turn_id"),
                trace_id: row.get("trace_id"),
                request_id: row.get("request_id"),
                character_id: row.get("character_id"),
                llm_node: Some(row.get("source_module")),
                latency_ms: None,
                token_usage: None,
                stream_chunk_count: None,
                step_count: None,
                protected: protected.map(|p| p != 0),
            }
        })
        .collect())
}

async fn collect_trace_records(
    pool: &SqlitePool,
    source: &LogSourceRef,
    filter: &LogRecordFilter,
    limit: i64,
    records: &mut Vec<LogRecordSummary>,
) -> Result<(), String> {
    if !matches!(
        filter.record_kind.as_deref(),
        None | Some("all" | "trace")
    ) {
        return Ok(());
    }

    let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(
        r#"
        SELECT trace_id, scene_turn_id, session_id, runtime_turn_status, trace_kind,
               character_id, summary, linked_request_ids, linked_event_ids, created_at,
               (SELECT COUNT(*) FROM agent_step_traces s WHERE s.trace_id = t.trace_id) AS step_count
        FROM turn_traces t
        "#,
    );
    append_trace_where(&mut builder, filter);
    builder.push(" ORDER BY created_at DESC LIMIT ");
    builder.push_bind(limit);

    let rows = builder
        .build()
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to query Agent Trace: {}", e))?;

    records.extend(rows.into_iter().map(|row| {
        let trace_id: String = row.get("trace_id");
        let summary = parse_json(row.get::<Option<String>, _>("summary")).unwrap_or_default();
        LogRecordSummary {
            record_ref: LogRecordRef {
                record_kind: "trace".to_string(),
                source: source.clone(),
                id: trace_id.clone(),
            },
            created_at: row.get("created_at"),
            title: format!("Trace {}", short_id(&trace_id)),
            summary: Some(trace_summary_text(&summary, row.get::<String, _>("trace_kind"))),
            status: Some(row.get("runtime_turn_status")),
            level: None,
            mode: Some("Agent".to_string()),
            provider: None,
            model: None,
            world_id: source.world_id.clone(),
            session_id: row.get("session_id"),
            scene_turn_id: Some(row.get("scene_turn_id")),
            trace_id: Some(trace_id),
            request_id: None,
            character_id: row.get("character_id"),
            llm_node: Some("AgentTrace".to_string()),
            latency_ms: None,
            token_usage: None,
            stream_chunk_count: None,
            step_count: Some(row.get("step_count")),
            protected: None,
        }
    }));

    Ok(())
}

async fn get_llm_detail(
    pool: &SqlitePool,
    request_id: &str,
) -> Result<Option<LlmLogDetail>, String> {
    let row = sqlx::query("SELECT * FROM llm_call_logs WHERE request_id = ?")
        .bind(request_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Failed to get LLM log detail: {}", e))?;

    Ok(row.map(|row| LlmLogDetail {
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
        request_json: parse_json(row.get::<Option<String>, _>("request_json")).unwrap_or_default(),
        schema_json: parse_json(row.get("schema_json")),
        response_json: parse_json(row.get("response_json")),
        assembled_text: row.get("assembled_text"),
        readable_text: row.get("readable_text"),
        status: row.get("status"),
        latency_ms: row.get("latency_ms"),
        token_usage: parse_optional_json(row.get("token_usage")),
        retry_count: row.get("retry_count"),
        error_summary: row.get("error_summary"),
        redaction_applied: row.get::<i64, _>("redaction_applied") != 0,
        created_at: row.get("created_at"),
        completed_at: row.get("completed_at"),
    }))
}

async fn get_event_detail(
    pool: &SqlitePool,
    event_id: &str,
) -> Result<Option<EventLogDetail>, String> {
    let row = sqlx::query("SELECT * FROM app_event_logs WHERE event_id = ?")
        .bind(event_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Failed to get event log detail: {}", e))?;

    Ok(row.map(|row| EventLogDetail {
        event_id: row.get("event_id"),
        level: row.get("level"),
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
        detail_json: parse_json(row.get("detail_json")),
        created_at: row.get("created_at"),
    }))
}

async fn get_trace_detail_inner(
    app: &AppHandle,
    world_id: &str,
    trace_id: &str,
) -> Result<TraceDetail, String> {
    validate_path_component(world_id)
        .map_err(|e| format!("Invalid world_id '{}': {}", world_id, e))?;
    let pool = open_sqlite_pool(&world_db_path(app, world_id)?).await?;

    let row = sqlx::query("SELECT * FROM turn_traces WHERE trace_id = ?")
        .bind(trace_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| format!("Failed to get trace detail: {}", e))?
        .ok_or_else(|| "Trace not found".to_string())?;

    let steps = sqlx::query(
        r#"
        SELECT * FROM agent_step_traces
        WHERE trace_id = ?
        ORDER BY created_at ASC
        "#,
    )
    .bind(trace_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to get trace steps: {}", e))?
    .into_iter()
    .map(|row| TraceStepDetail {
        step_trace_id: row.get("step_trace_id"),
        trace_id: row.get("trace_id"),
        scene_turn_id: row.get("scene_turn_id"),
        character_id: row.get("character_id"),
        step_name: row.get("step_name"),
        step_status: row.get("step_status"),
        input_summary: parse_json(row.get("input_summary")),
        output_summary: parse_json(row.get("output_summary")),
        decision_json: parse_json(row.get("decision_json")),
        linked_request_id: row.get("linked_request_id"),
        error_event_id: row.get("error_event_id"),
        created_at: row.get("created_at"),
    })
    .collect();

    Ok(TraceDetail {
        trace_id: row.get("trace_id"),
        scene_turn_id: row.get("scene_turn_id"),
        session_id: row.get("session_id"),
        story_time_anchor: parse_json(row.get("story_time_anchor")),
        runtime_turn_status: row.get("runtime_turn_status"),
        trace_kind: row.get("trace_kind"),
        character_id: row.get("character_id"),
        runtime_config_snapshot_id: row.get("runtime_config_snapshot_id"),
        world_rules_snapshot_id: row.get("world_rules_snapshot_id"),
        summary: parse_json(row.get::<Option<String>, _>("summary")).unwrap_or_default(),
        linked_request_ids: parse_json(row.get::<Option<String>, _>("linked_request_ids"))
            .unwrap_or_else(|| serde_json::Value::Array(Vec::new())),
        linked_event_ids: parse_json(row.get::<Option<String>, _>("linked_event_ids"))
            .unwrap_or_else(|| serde_json::Value::Array(Vec::new())),
        created_at: row.get("created_at"),
        steps,
    })
}

fn append_common_context_where(
    builder: &mut QueryBuilder<Sqlite>,
    prefix: &str,
    filter: &LogRecordFilter,
    has_where: &mut bool,
) {
    push_optional_eq(builder, has_where, prefix, "world_id", &filter.world_id);
    push_optional_eq(builder, has_where, prefix, "session_id", &filter.session_id);
    push_optional_eq(
        builder,
        has_where,
        prefix,
        "scene_turn_id",
        &filter.scene_turn_id,
    );
    push_optional_eq(builder, has_where, prefix, "trace_id", &filter.trace_id);
    push_optional_eq(
        builder,
        has_where,
        prefix,
        "character_id",
        &filter.character_id,
    );
    push_optional_time(builder, has_where, prefix, "created_at", ">=", &filter.since);
    push_optional_time(builder, has_where, prefix, "created_at", "<=", &filter.until);
}

fn append_llm_where(builder: &mut QueryBuilder<Sqlite>, filter: &LogRecordFilter) {
    let mut has_where = false;
    append_common_context_where(builder, "l", filter, &mut has_where);
    push_optional_eq(builder, &mut has_where, "l", "request_id", &filter.request_id);
    push_optional_eq(builder, &mut has_where, "l", "status", &filter.status);
    push_optional_eq(builder, &mut has_where, "l", "mode", &filter.mode);
    push_optional_eq(builder, &mut has_where, "l", "provider", &filter.provider);
    push_optional_eq(builder, &mut has_where, "l", "model", &filter.model);
    push_optional_eq(builder, &mut has_where, "l", "llm_node", &filter.llm_node);
    if let Some(search) = clean_search(&filter.search) {
        push_and(builder, &mut has_where);
        let like = format!("%{}%", search);
        builder
            .push(" (l.request_id LIKE ")
            .push_bind(like.clone())
            .push(" OR l.provider LIKE ")
            .push_bind(like.clone())
            .push(" OR l.model LIKE ")
            .push_bind(like.clone())
            .push(" OR l.llm_node LIKE ")
            .push_bind(like.clone())
            .push(" OR l.error_summary LIKE ")
            .push_bind(like)
            .push(")");
    }
}

fn append_event_where(builder: &mut QueryBuilder<Sqlite>, filter: &LogRecordFilter) {
    let mut has_where = false;
    append_common_context_where(builder, "e", filter, &mut has_where);
    push_optional_eq(builder, &mut has_where, "e", "request_id", &filter.request_id);
    push_optional_eq(builder, &mut has_where, "e", "level", &filter.level);
    if let Some(search) = clean_search(&filter.search) {
        push_and(builder, &mut has_where);
        let like = format!("%{}%", search);
        builder
            .push(" (e.event_id LIKE ")
            .push_bind(like.clone())
            .push(" OR e.event_type LIKE ")
            .push_bind(like.clone())
            .push(" OR e.message LIKE ")
            .push_bind(like.clone())
            .push(" OR e.source_module LIKE ")
            .push_bind(like)
            .push(")");
    }
}

fn append_trace_where(builder: &mut QueryBuilder<Sqlite>, filter: &LogRecordFilter) {
    let mut has_where = false;
    push_optional_eq(builder, &mut has_where, "t", "session_id", &filter.session_id);
    push_optional_eq(
        builder,
        &mut has_where,
        "t",
        "scene_turn_id",
        &filter.scene_turn_id,
    );
    push_optional_eq(builder, &mut has_where, "t", "trace_id", &filter.trace_id);
    push_optional_eq(
        builder,
        &mut has_where,
        "t",
        "character_id",
        &filter.character_id,
    );
    push_optional_eq(
        builder,
        &mut has_where,
        "t",
        "runtime_turn_status",
        &filter.status,
    );
    push_optional_time(builder, &mut has_where, "t", "created_at", ">=", &filter.since);
    push_optional_time(builder, &mut has_where, "t", "created_at", "<=", &filter.until);
    if let Some(search) = clean_search(&filter.search) {
        push_and(builder, &mut has_where);
        let like = format!("%{}%", search);
        builder
            .push(" (t.trace_id LIKE ")
            .push_bind(like.clone())
            .push(" OR t.scene_turn_id LIKE ")
            .push_bind(like.clone())
            .push(" OR t.summary LIKE ")
            .push_bind(like)
            .push(")");
    }
}

fn push_optional_eq(
    builder: &mut QueryBuilder<Sqlite>,
    has_where: &mut bool,
    prefix: &str,
    column: &str,
    value: &Option<String>,
) {
    if let Some(value) = value.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        push_and(builder, has_where);
        builder
            .push(format!("{}.{} = ", prefix, column))
            .push_bind(value.to_string());
    }
}

fn push_optional_time(
    builder: &mut QueryBuilder<Sqlite>,
    has_where: &mut bool,
    prefix: &str,
    column: &str,
    op: &str,
    value: &Option<String>,
) {
    if let Some(value) = value.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        push_and(builder, has_where);
        builder
            .push(format!("{}.{} {} ", prefix, column, op))
            .push_bind(value.to_string());
    }
}

fn push_and(builder: &mut QueryBuilder<Sqlite>, has_where: &mut bool) {
    if *has_where {
        builder.push(" AND ");
    } else {
        builder.push(" WHERE ");
        *has_where = true;
    }
}

fn clean_search(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.chars().take(128).collect())
}

fn normalize_page(page: Option<LogPageInput>) -> (i64, i64) {
    let offset = page.as_ref().and_then(|p| p.offset).unwrap_or(0).max(0);
    let limit = page
        .and_then(|p| p.limit)
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    (offset, limit)
}

async fn pool_for_source(
    app: &AppHandle,
    state: &Arc<AppState>,
    source: &LogSourceRef,
) -> Result<SqlitePool, String> {
    match source.source_kind.as_str() {
        "global" => {
            let store_guard = state.sqlite_store.read().await;
            let store = store_guard
                .as_ref()
                .ok_or_else(|| "Database not initialized".to_string())?;
            Ok(store.pool().clone())
        }
        "world" | "trace" => {
            let world_id = source
                .world_id
                .as_deref()
                .ok_or_else(|| "World log source requires world_id".to_string())?;
            open_sqlite_pool(&world_db_path(app, world_id)?).await
        }
        other => Err(format!("Unsupported log source: {}", other)),
    }
}

fn world_db_path(app: &AppHandle, world_id: &str) -> Result<PathBuf, String> {
    validate_path_component(world_id)
        .map_err(|e| format!("Invalid world_id '{}': {}", world_id, e))?;
    Ok(app_data_root(app)?
        .join("worlds")
        .join(world_id)
        .join("world.sqlite"))
}

fn world_log_targets(
    app: &AppHandle,
    requested_world_id: Option<&str>,
) -> Result<Vec<(String, PathBuf)>, String> {
    if let Some(world_id) = requested_world_id {
        let path = world_db_path(app, world_id)?;
        return Ok(if path.exists() {
            vec![(world_id.to_string(), path)]
        } else {
            Vec::new()
        });
    }

    let worlds_root = app_data_root(app)?.join("worlds");
    if !worlds_root.exists() {
        return Ok(Vec::new());
    }

    let mut targets = Vec::new();
    for entry in
        std::fs::read_dir(&worlds_root).map_err(|e| format!("Failed to read worlds: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read world entry: {}", e))?;
        if !entry
            .file_type()
            .map_err(|e| format!("Failed to inspect world entry: {}", e))?
            .is_dir()
        {
            continue;
        }
        let world_id = entry.file_name().to_string_lossy().to_string();
        if validate_path_component(&world_id).is_err() {
            continue;
        }
        let path = entry.path().join("world.sqlite");
        if path.exists() {
            targets.push((world_id, path));
        }
    }
    Ok(targets)
}

async fn open_sqlite_pool(path: &Path) -> Result<SqlitePool, String> {
    if !path.exists() {
        return Err(format!("Log database does not exist: {}", path.display()));
    }
    SqlitePoolOptions::new()
        .max_connections(2)
        .connect(&format!("sqlite:{}?mode=ro", path.display()))
        .await
        .map_err(|e| format!("Failed to open log database: {}", e))
}

async fn summarize_log_database(
    scope: &str,
    world_id: Option<String>,
    path: &Path,
    size_limit_bytes: Option<u64>,
) -> Result<LogScopeStorageSummary, String> {
    let size_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let modified = std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(DateTime::<Utc>::from)
        .map(|dt| dt.to_rfc3339());

    if !path.exists() {
        return Ok(LogScopeStorageSummary {
            scope: scope.to_string(),
            world_id,
            size_bytes,
            size_limit_bytes,
            llm_count: 0,
            event_count: 0,
            trace_count: 0,
            stream_chunk_count: 0,
            last_updated_at: modified,
            stale_prompt_required: false,
        });
    }

    let pool = open_sqlite_pool(path).await?;
    let llm_count = table_count(&pool, "llm_call_logs").await.unwrap_or(0);
    let event_count = table_count(&pool, "app_event_logs").await.unwrap_or(0);
    let trace_count = table_count(&pool, "turn_traces").await.unwrap_or(0);
    let stream_chunk_count = table_count(&pool, "llm_stream_chunks").await.unwrap_or(0);
    let last_updated_at =
        latest_timestamp(&pool, scope == "world").await.or(modified.clone());
    let stale_prompt_required = scope == "world"
        && size_bytes > GLOBAL_LOG_LIMIT_BYTES / 10
        && last_updated_at
            .as_deref()
            .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
            .map(|dt| Utc::now().signed_duration_since(dt.with_timezone(&Utc)) > Duration::days(WORLD_STALE_DAYS))
            .unwrap_or(false);

    Ok(LogScopeStorageSummary {
        scope: scope.to_string(),
        world_id,
        size_bytes,
        size_limit_bytes,
        llm_count,
        event_count,
        trace_count,
        stream_chunk_count,
        last_updated_at,
        stale_prompt_required,
    })
}

async fn table_count(pool: &SqlitePool, table: &str) -> Result<i64, String> {
    let sql = format!("SELECT COUNT(*) AS count FROM {}", table);
    let row = sqlx::query(&sql)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(row.get("count"))
}

async fn count_where(pool: &SqlitePool, sql: &str, value: &str) -> Result<i64, String> {
    let row = sqlx::query(sql)
        .bind(value)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Failed to count cleanup candidates: {}", e))?;
    Ok(row.get("count"))
}

async fn latest_timestamp(pool: &SqlitePool, include_trace: bool) -> Option<String> {
    let mut candidates = Vec::new();
    for table in ["llm_call_logs", "app_event_logs"] {
        if let Ok(row) = sqlx::query(&format!("SELECT MAX(created_at) AS value FROM {}", table))
            .fetch_one(pool)
            .await
        {
            if let Some(value) = row.get::<Option<String>, _>("value") {
                candidates.push(value);
            }
        }
    }
    if include_trace {
        if let Ok(row) = sqlx::query("SELECT MAX(created_at) AS value FROM turn_traces")
            .fetch_one(pool)
            .await
        {
            if let Some(value) = row.get::<Option<String>, _>("value") {
                candidates.push(value);
            }
        }
    }
    candidates.into_iter().max()
}

fn validate_filter_ids(filter: &LogRecordFilter) -> Result<(), String> {
    for value in [
        filter.world_id.as_deref(),
        filter.session_id.as_deref(),
        filter.scene_turn_id.as_deref(),
        filter.trace_id.as_deref(),
        filter.request_id.as_deref(),
        filter.character_id.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if value.contains('/') || value.contains('\\') || value.contains('\0') {
            return Err("Log filter IDs must not contain path separators".to_string());
        }
    }
    Ok(())
}

fn parse_json(value: Option<String>) -> Option<serde_json::Value> {
    value.and_then(|v| serde_json::from_str(&v).ok())
}

fn parse_optional_json(value: Option<String>) -> Option<serde_json::Value> {
    parse_json(value)
}

fn trace_summary_text(summary: &serde_json::Value, fallback: String) -> String {
    summary
        .get("summary")
        .and_then(|v| v.as_str())
        .or_else(|| summary.get("text").and_then(|v| v.as_str()))
        .map(str::to_string)
        .unwrap_or(fallback)
}

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

fn export_csv(records: &[LogRecordSummary]) -> String {
    let mut lines = vec![
        "created_at,source,kind,id,title,status,level,mode,provider,model,world_id,session_id,scene_turn_id,trace_id,request_id".to_string(),
    ];
    for record in records {
        lines.push(
            [
                record.created_at.as_str(),
                record.record_ref.source.source_kind.as_str(),
                record.record_ref.record_kind.as_str(),
                record.record_ref.id.as_str(),
                record.title.as_str(),
                record.status.as_deref().unwrap_or(""),
                record.level.as_deref().unwrap_or(""),
                record.mode.as_deref().unwrap_or(""),
                record.provider.as_deref().unwrap_or(""),
                record.model.as_deref().unwrap_or(""),
                record.world_id.as_deref().unwrap_or(""),
                record.session_id.as_deref().unwrap_or(""),
                record.scene_turn_id.as_deref().unwrap_or(""),
                record.trace_id.as_deref().unwrap_or(""),
                record.request_id.as_deref().unwrap_or(""),
            ]
            .into_iter()
            .map(csv_escape)
            .collect::<Vec<_>>()
            .join(","),
        );
    }
    lines.join("\n")
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}
