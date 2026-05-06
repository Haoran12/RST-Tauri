import { invoke } from '@tauri-apps/api/core'

export interface LogRecordFilter {
  source_scope?: 'all' | 'global' | 'world' | 'trace'
  record_kind?: 'all' | 'llm' | 'event' | 'trace'
  level?: string
  status?: string
  mode?: string
  provider?: string
  model?: string
  world_id?: string
  session_id?: string
  scene_turn_id?: string
  trace_id?: string
  request_id?: string
  character_id?: string
  llm_node?: string
  search?: string
  since?: string
  until?: string
}

export interface LogPageInput {
  offset?: number
  limit?: number
}

export interface LogSourceRef {
  source_kind: 'global' | 'world' | 'trace'
  world_id?: string | null
}

export interface LogRecordRef {
  record_kind: 'llm' | 'event' | 'trace'
  source: LogSourceRef
  id: string
}

export interface LogRecordSummary {
  record_ref: LogRecordRef
  created_at: string
  title: string
  summary?: string | null
  status?: string | null
  level?: string | null
  mode?: string | null
  provider?: string | null
  model?: string | null
  world_id?: string | null
  session_id?: string | null
  scene_turn_id?: string | null
  trace_id?: string | null
  request_id?: string | null
  character_id?: string | null
  llm_node?: string | null
  latency_ms?: number | null
  token_usage?: unknown
  stream_chunk_count?: number | null
  step_count?: number | null
  protected?: boolean | null
}

export interface LogRecordPage {
  records: LogRecordSummary[]
  offset: number
  limit: number
  has_more: boolean
  total_count?: number
}

export interface LlmLogDetail {
  request_id: string
  mode: string
  world_id?: string | null
  session_id?: string | null
  scene_turn_id?: string | null
  trace_id?: string | null
  character_id?: string | null
  llm_node: string
  api_config_id: string
  runtime_config_snapshot_id?: string | null
  world_rules_snapshot_id?: string | null
  provider: string
  model: string
  call_type: string
  request_json: unknown
  schema_json?: unknown
  response_json?: unknown
  assembled_text?: string | null
  readable_text?: string | null
  status: string
  latency_ms?: number | null
  token_usage?: unknown
  retry_count: number
  error_summary?: string | null
  redaction_applied: boolean
  created_at: string
  completed_at?: string | null
}

export interface EventLogDetail {
  event_id: string
  level: string
  event_type: string
  message: string
  source_module: string
  request_id?: string | null
  world_id?: string | null
  session_id?: string | null
  scene_turn_id?: string | null
  trace_id?: string | null
  character_id?: string | null
  detail_json?: unknown
  created_at: string
}

export interface TraceStepDetail {
  step_trace_id: string
  trace_id: string
  scene_turn_id: string
  character_id?: string | null
  step_name: string
  step_status: string
  input_summary?: unknown
  output_summary?: unknown
  decision_json?: unknown
  linked_request_id?: string | null
  error_event_id?: string | null
  created_at: string
}

export interface TraceDetail {
  trace_id: string
  scene_turn_id: string
  session_id?: string | null
  story_time_anchor?: unknown
  runtime_turn_status: string
  trace_kind: string
  character_id?: string | null
  runtime_config_snapshot_id: string
  world_rules_snapshot_id?: string | null
  summary: unknown
  linked_request_ids: unknown
  linked_event_ids: unknown
  created_at: string
  steps: TraceStepDetail[]
}

export interface LogRecordDetail {
  record_ref: LogRecordRef
  llm?: LlmLogDetail | null
  event?: EventLogDetail | null
  trace?: TraceDetail | null
}

export interface StreamChunkDetail {
  chunk_id: string
  request_id: string
  chunk_index: number
  raw_chunk: string
  received_at: string
}

export interface StreamChunkPage {
  chunks: StreamChunkDetail[]
  offset: number
  limit: number
  has_more: boolean
}

export interface LogScopeStorageSummary {
  scope: string
  world_id?: string | null
  size_bytes: number
  size_limit_bytes?: number | null
  llm_count: number
  event_count: number
  trace_count: number
  stream_chunk_count: number
  last_updated_at?: string | null
  stale_prompt_required: boolean
}

export interface LogStorageSummary {
  global: LogScopeStorageSummary
  worlds: LogScopeStorageSummary[]
}

export interface ExportLogsResult {
  format: string
  filename: string
  content: string
  record_count: number
}

export interface LogRetentionResult {
  llm_logs_deleted: number
  event_logs_deleted: number
  size_before_bytes: number
  size_after_bytes: number
}

export interface LogCleanupPreview {
  plan_id: string
  scope: string
  older_than_days: number
  llm_logs_to_delete: number
  event_logs_to_delete: number
  stream_chunks_affected: number
  protected_trace_records: number
  notes: string[]
}

export async function queryLogRecords(filter: LogRecordFilter, page?: LogPageInput) {
  return await invoke<LogRecordPage>('query_log_records', { filter, page })
}

export async function getLogRecordDetail(recordRef: LogRecordRef) {
  return await invoke<LogRecordDetail>('get_log_record_detail', { recordRef })
}

export async function getStreamChunks(requestId: string, source: LogSourceRef, page?: LogPageInput) {
  return await invoke<StreamChunkPage>('get_stream_chunks', { requestId, source, page })
}

export async function getTraceDetail(worldId: string, traceId: string) {
  return await invoke<TraceDetail>('get_trace_detail', { worldId, traceId })
}

export async function getLogStorageSummary() {
  return await invoke<LogStorageSummary>('get_log_storage_summary')
}

export async function exportLogs(filter: LogRecordFilter, format: 'json' | 'jsonl' | 'csv' = 'json') {
  return await invoke<ExportLogsResult>('export_logs', { filter, format })
}

export async function runLogRetentionNow(scope: 'global' = 'global') {
  return await invoke<LogRetentionResult>('run_log_retention_now', { scope })
}

export async function previewLogCleanup() {
  return await invoke<LogCleanupPreview>('preview_log_cleanup', {
    input: { scope: 'global', older_than_days: 30 },
  })
}

export async function confirmLogCleanup(planId: string) {
  return await invoke<LogRetentionResult>('confirm_log_cleanup', {
    planId,
    scope: 'global',
  })
}

export interface SetLogProtectionInput {
  record_kind: 'llm' | 'event'
  record_id: string
  protected: boolean
}

export interface SetLogProtectionResult {
  record_kind: string
  record_id: string
  protected: boolean
}

export async function setLogProtection(input: SetLogProtectionInput) {
  return await invoke<SetLogProtectionResult>('set_log_protection', { input })
}

export async function getLogProtection(recordKind: 'llm' | 'event', recordId: string) {
  return await invoke<boolean>('get_log_protection', {
    record_kind: recordKind,
    record_id: recordId,
  })
}

// ============================================================================
// Frontend Event Logging
// ============================================================================

export interface FrontendEventInput {
  level: 'debug' | 'info' | 'warn' | 'error' | 'fatal'
  event_type: string
  message: string
  detail_json?: unknown
}

/**
 * Log an event from frontend to application logs
 */
export async function logFrontendEvent(input: FrontendEventInput): Promise<void> {
  return await invoke('log_frontend_event', { input })
}

/**
 * Log an error from frontend to application logs
 */
export async function logFrontendError(
  eventType: string,
  message: string,
  detail?: unknown,
): Promise<void> {
  return await logFrontendEvent({
    level: 'error',
    event_type: eventType,
    message,
    detail_json: detail,
  })
}

/**
 * Log a warning from frontend to application logs
 */
export async function logFrontendWarn(
  eventType: string,
  message: string,
  detail?: unknown,
): Promise<void> {
  return await logFrontendEvent({
    level: 'warn',
    event_type: eventType,
    message,
    detail_json: detail,
  })
}
