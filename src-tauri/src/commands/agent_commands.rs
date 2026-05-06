//! Tauri commands for Agent session management
//!
//! Agent 会话创建、管理、时间线操作等命令。

use crate::agent::models::character::CharacterRecord;
use crate::agent::models::common::{PlayerMode, TimeAnchor, TimePrecision};
use crate::agent::models::knowledge::{OpenDetailSlot, TruthGuidance};
use crate::agent::models::session::{
    AgentSession, ProvisionalSessionTruth, SessionTurn, TurnRole, WorldMainlineCursor,
};
use crate::agent::models::{RuntimeTurnCanonStatus, SessionTurnCanonStatus};
use crate::agent::runtime::{AgentRuntime, TurnResult};
use crate::agent::simulation::canon_status_manager::{PromotionEvaluationResult, PromotionResult};
use crate::agent::simulation::provisional_truth_manager::{
    DetailSlotFillRequest, DetailSlotFillResult,
};
use crate::agent::simulation::{
    CanonStatusManager, HistoricalTruthResolver, ProvisionalTruthManager,
};
use crate::agent::storage::agent_store::AgentStore;
use crate::storage::paths::{app_data_root, validate_path_component};
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::RwLock;

/// Get the data directory path
fn get_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app_data_root(app)
}

/// Get or create Agent store for a world
async fn get_agent_store(
    app: &AppHandle,
    state: &Arc<AppState>,
    world_id: &str,
) -> Result<AgentStore, String> {
    validate_path_component(world_id)
        .map_err(|e| format!("Invalid world_id '{}': {}", world_id, e))?;

    let data_dir = get_data_dir(app)?;
    let world_db_path = data_dir.join("worlds").join(world_id).join("world.sqlite");

    // Ensure parent directory exists
    if let Some(parent) = world_db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create world directory: {}", e))?;
    }

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&format!("sqlite:{}?mode=rwc", world_db_path.display()))
        .await
        .map_err(|e| format!("Failed to connect to world database: {}", e))?;

    AgentStore::new(pool, world_id.to_string()).await
}

// ============================================================================
// World 列表命令
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentWorldListItem {
    pub world_id: String,
    pub session_count: i64,
    pub active_session_count: i64,
    pub character_count: i64,
    pub mainline_time_anchor: Option<TimeAnchor>,
    pub updated_at: Option<String>,
}

/// 列出 Agent Worlds 及首页所需摘要。
#[tauri::command]
pub async fn list_agent_worlds(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<AgentWorldListItem>, String> {
    let data_dir = get_data_dir(&app)?;
    let worlds_dir = data_dir.join("worlds");
    std::fs::create_dir_all(&worlds_dir)
        .map_err(|e| format!("Failed to create worlds directory: {}", e))?;

    let mut worlds = Vec::new();
    let entries = std::fs::read_dir(&worlds_dir)
        .map_err(|e| format!("Failed to read worlds directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read world entry: {}", e))?;
        let file_type = entry
            .file_type()
            .map_err(|e| format!("Failed to read world entry type: {}", e))?;
        if !file_type.is_dir() {
            continue;
        }

        let world_id = entry.file_name().to_string_lossy().to_string();
        validate_path_component(&world_id)
            .map_err(|e| format!("Invalid world_id '{}': {}", world_id, e))?;

        let store = get_agent_store(&app, state.inner(), &world_id).await?;

        let session_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_sessions")
            .fetch_one(store.pool())
            .await
            .map_err(|e| format!("Failed to count sessions for world '{}': {}", world_id, e))?;

        let active_session_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM agent_sessions WHERE status = 'active'")
                .fetch_one(store.pool())
                .await
                .map_err(|e| {
                    format!(
                        "Failed to count active sessions for world '{}': {}",
                        world_id, e
                    )
                })?;

        let character_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM character_records")
            .fetch_one(store.pool())
            .await
            .map_err(|e| format!("Failed to count characters for world '{}': {}", world_id, e))?;

        let mainline_time_anchor = match store.get_mainline_cursor().await {
            Ok(cursor) => Some(cursor.mainline_time_anchor),
            Err(_) => None,
        };

        let updated_at: Option<String> = sqlx::query_scalar(
            "SELECT MAX(updated_at) FROM (
                SELECT updated_at FROM agent_sessions
                UNION ALL
                SELECT updated_at FROM world_mainline_cursor
            )",
        )
        .fetch_one(store.pool())
        .await
        .map_err(|e| format!("Failed to read updated_at for world '{}': {}", world_id, e))?;

        worlds.push(AgentWorldListItem {
            world_id,
            session_count,
            active_session_count,
            character_count,
            mainline_time_anchor,
            updated_at,
        });
    }

    worlds.sort_by(|a, b| {
        b.updated_at
            .as_deref()
            .cmp(&a.updated_at.as_deref())
            .then_with(|| a.world_id.cmp(&b.world_id))
    });

    Ok(worlds)
}

// ============================================================================
// 会话管理命令
// ============================================================================

/// 创建会话的输入参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionInput {
    pub world_id: String,
    pub title: String,
    pub player_mode: PlayerMode,
    pub player_character_id: Option<String>,
    pub period_anchor: TimeAnchor,
}

/// 创建新的 Agent 会话
#[tauri::command]
pub async fn create_agent_session(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: CreateSessionInput,
) -> Result<AgentSession, String> {
    // Validate player_mode and player_character_id consistency
    match input.player_mode {
        PlayerMode::Character => {
            if input.player_character_id.is_none() {
                return Err("Character mode requires player_character_id".to_string());
            }
        }
        PlayerMode::Director => {
            if input.player_character_id.is_some() {
                return Err("Director mode must not have player_character_id".to_string());
            }
        }
    }

    let store = get_agent_store(&app, state.inner(), &input.world_id).await?;

    // Determine session kind from period anchor and mainline cursor
    let cursor = store.get_mainline_cursor().await?;
    let session_kind = if input.period_anchor.ordinal < cursor.mainline_time_anchor.ordinal {
        crate::agent::models::common::AgentSessionKind::Retrospective
    } else if input.period_anchor.ordinal > cursor.mainline_time_anchor.ordinal {
        crate::agent::models::common::AgentSessionKind::FuturePreview
    } else {
        crate::agent::models::common::AgentSessionKind::Mainline
    };

    // Create session with explicit player mode
    let session = AgentSession::new_with_mode(
        input.world_id.clone(),
        input.title,
        session_kind,
        input.period_anchor,
        input.player_mode,
        input.player_character_id,
    )?;

    store.create_session(&session).await?;
    Ok(session)
}

/// 获取世界中的所有会话
#[tauri::command]
pub async fn list_agent_sessions(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    world_id: String,
) -> Result<Vec<AgentSession>, String> {
    let store = get_agent_store(&app, state.inner(), &world_id).await?;
    store.list_sessions().await
}

/// 获取单个会话
#[tauri::command]
pub async fn get_agent_session(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    world_id: String,
    session_id: String,
) -> Result<Option<AgentSession>, String> {
    let store = get_agent_store(&app, state.inner(), &world_id).await?;
    store.get_session(&session_id).await
}

/// 获取会话消息列表
#[tauri::command]
pub async fn list_agent_session_turns(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    world_id: String,
    session_id: String,
) -> Result<Vec<SessionTurn>, String> {
    let store = get_agent_store(&app, state.inner(), &world_id).await?;
    let session = store
        .get_session(&session_id)
        .await?
        .ok_or_else(|| "Session not found".to_string())?;
    if session.world_id != world_id {
        return Err("Session does not belong to requested world".to_string());
    }
    store.list_session_turns(&session_id).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAgentSessionTurnInput {
    pub world_id: String,
    pub session_id: String,
    pub session_turn_id: String,
    pub content: String,
}

/// 修改会话可见消息文本；不回写已经提交的 WorldTurn / Trace。
#[tauri::command]
pub async fn update_agent_session_turn(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: UpdateAgentSessionTurnInput,
) -> Result<SessionTurn, String> {
    let store = get_agent_store(&app, state.inner(), &input.world_id).await?;
    let session = store
        .get_session(&input.session_id)
        .await?
        .ok_or_else(|| "Session not found".to_string())?;
    if session.world_id != input.world_id {
        return Err("Session does not belong to requested world".to_string());
    }

    let existing = store
        .get_session_turn(&input.session_id, &input.session_turn_id)
        .await?
        .ok_or_else(|| "Session turn not found".to_string())?;

    let mut message_json = existing.message_json;
    match &mut message_json {
        serde_json::Value::Object(map) => {
            map.insert("content".to_string(), serde_json::Value::String(input.content));
        }
        serde_json::Value::String(value) => {
            *value = input.content;
        }
        _ => {
            message_json = serde_json::json!({ "content": input.content });
        }
    }

    store
        .update_session_turn_message(&input.session_id, &input.session_turn_id, message_json)
        .await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteAgentSessionTurnInput {
    pub world_id: String,
    pub session_id: String,
    pub session_turn_id: String,
}

/// 删除会话可见消息；不删除已经提交的 WorldTurn / Trace。
#[tauri::command]
pub async fn delete_agent_session_turn(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: DeleteAgentSessionTurnInput,
) -> Result<(), String> {
    let store = get_agent_store(&app, state.inner(), &input.world_id).await?;
    let session = store
        .get_session(&input.session_id)
        .await?
        .ok_or_else(|| "Session not found".to_string())?;
    if session.world_id != input.world_id {
        return Err("Session does not belong to requested world".to_string());
    }
    store
        .delete_session_turn(&input.session_id, &input.session_turn_id)
        .await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessAgentTurnInput {
    pub world_id: String,
    pub session_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessAgentTurnOutput {
    pub result: TurnResult,
    pub user_turn: SessionTurn,
    pub assistant_turn: SessionTurn,
}

/// 运行一个 Agent 回合，并把用户/助手消息写入 session_turns。
#[tauri::command]
pub fn process_agent_turn(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: ProcessAgentTurnInput,
) -> Result<ProcessAgentTurnOutput, String> {
    let app_state = state.inner().clone();
    tauri::async_runtime::block_on(
        async move { process_agent_turn_inner(app, app_state, input).await },
    )
}

async fn process_agent_turn_inner(
    app: AppHandle,
    state: Arc<AppState>,
    input: ProcessAgentTurnInput,
) -> Result<ProcessAgentTurnOutput, String> {
    if input.content.trim().is_empty() {
        return Err("Agent turn content must not be empty".to_string());
    }

    let store = get_agent_store(&app, &state, &input.world_id).await?;
    let session = store
        .get_session(&input.session_id)
        .await?
        .ok_or_else(|| "Session not found".to_string())?;
    if session.world_id != input.world_id {
        return Err("Session does not belong to requested world".to_string());
    }

    let user_message = serde_json::json!({
        "content": input.content.trim(),
        "input_kind": match session.player_mode {
            PlayerMode::Character => "character_roleplay",
            PlayerMode::Director => "director_or_scene",
        },
        "player_mode": session.player_mode,
        "player_character_id": session.player_character_id,
    });

    let runtime_store = AgentStore::new(store.pool().clone(), input.world_id.clone()).await?;
    let mut runtime = AgentRuntime::new(Arc::new(RwLock::new(runtime_store)));
    let result = runtime
        .process_turn(&input.session_id, user_message.clone())
        .await?;

    let turn_status = session_turn_status_from_runtime(result.canon_status);
    let user_turn = store
        .append_session_turn(
            &input.session_id,
            Some(&result.scene_turn_id),
            TurnRole::User,
            user_message,
            turn_status,
        )
        .await?;
    let assistant_turn = store
        .append_session_turn(
            &input.session_id,
            Some(&result.scene_turn_id),
            TurnRole::Assistant,
            serde_json::json!({
                "content": result.narrative_text,
                "runtime_config_snapshot_id": result.runtime_config_snapshot_id,
                "world_rules_snapshot_id": result.world_rules_snapshot_id,
            }),
            turn_status,
        )
        .await?;

    Ok(ProcessAgentTurnOutput {
        result,
        user_turn,
        assistant_turn,
    })
}

/// 更新会话的玩家模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePlayerModeInput {
    pub world_id: String,
    pub session_id: String,
    pub player_mode: PlayerMode,
    pub player_character_id: Option<String>,
}

/// 更新会话的玩家模式
#[tauri::command]
pub async fn update_session_player_mode(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: UpdatePlayerModeInput,
) -> Result<AgentSession, String> {
    // Validate player_mode and player_character_id consistency
    match input.player_mode {
        PlayerMode::Character => {
            if input.player_character_id.is_none() {
                return Err("Character mode requires player_character_id".to_string());
            }
        }
        PlayerMode::Director => {
            if input.player_character_id.is_some() {
                return Err("Director mode must not have player_character_id".to_string());
            }
        }
    }

    let store = get_agent_store(&app, state.inner(), &input.world_id).await?;
    let mut session = store
        .get_session(&input.session_id)
        .await?
        .ok_or_else(|| "Session not found".to_string())?;

    session.player_mode = input.player_mode;
    session.player_character_id = input.player_character_id;
    session.updated_at = chrono::Utc::now();

    // Validate the updated session
    session.validate()?;

    store.update_session(&session).await?;
    Ok(session)
}

fn session_turn_status_from_runtime(status: RuntimeTurnCanonStatus) -> SessionTurnCanonStatus {
    match status {
        RuntimeTurnCanonStatus::Canon | RuntimeTurnCanonStatus::ProvisionalPromoted => {
            SessionTurnCanonStatus::CanonPromoted
        }
        RuntimeTurnCanonStatus::ProvisionalOnly => SessionTurnCanonStatus::CanonCandidate,
        RuntimeTurnCanonStatus::NonCanon | RuntimeTurnCanonStatus::FuturePreview => {
            SessionTurnCanonStatus::NonCanon
        }
    }
}

// ============================================================================
// 时间线命令
// ============================================================================

/// 获取世界主线光标
#[tauri::command]
pub async fn get_world_mainline_cursor(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    world_id: String,
) -> Result<WorldMainlineCursor, String> {
    let store = get_agent_store(&app, state.inner(), &world_id).await?;
    store.get_mainline_cursor().await
}

/// 推进主线光标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvanceMainlineInput {
    pub world_id: String,
    pub turn_id: String,
    pub new_time_anchor: TimeAnchor,
}

/// 推进主线光标到新的回合
#[tauri::command]
pub async fn advance_world_mainline(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: AdvanceMainlineInput,
) -> Result<WorldMainlineCursor, String> {
    let store = get_agent_store(&app, state.inner(), &input.world_id).await?;
    store
        .advance_mainline(&input.turn_id, &input.new_time_anchor)
        .await?;
    store.get_mainline_cursor().await
}

// ============================================================================
// 角色列表命令
// ============================================================================

/// 获取世界中的所有角色（用于角色选择）
#[tauri::command]
pub async fn list_world_characters(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    world_id: String,
) -> Result<Vec<CharacterRecord>, String> {
    let store = get_agent_store(&app, state.inner(), &world_id).await?;
    store.list_characters().await
}

// ============================================================================
// 时间锚点辅助命令
// ============================================================================

/// 创建时间锚点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTimeAnchorInput {
    pub ordinal: i64,
    pub display_text: String,
    pub precision: Option<TimePrecision>,
    pub calendar_id: Option<String>,
}

/// 创建时间锚点
#[tauri::command]
pub fn create_time_anchor(input: CreateTimeAnchorInput) -> TimeAnchor {
    TimeAnchor {
        calendar_id: input.calendar_id.unwrap_or_else(|| "default".to_string()),
        ordinal: input.ordinal,
        precision: input.precision.unwrap_or(TimePrecision::Exact),
        display_text: input.display_text,
    }
}

/// 比较两个时间锚点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareTimeAnchorsInput {
    pub anchor1: TimeAnchor,
    pub anchor2: TimeAnchor,
}

/// 比较两个时间锚点
#[tauri::command]
pub fn compare_time_anchors(input: CompareTimeAnchorsInput) -> i64 {
    input.anchor1.ordinal - input.anchor2.ordinal
}

// ============================================================================
// 过去线补完命令
// ============================================================================

/// 获取过去线会话的真理引导
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTruthGuidanceInput {
    pub world_id: String,
    pub session_id: String,
}

/// 获取过去线会话的真理引导（包含开放细节槽）
#[tauri::command]
pub async fn get_truth_guidance(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: GetTruthGuidanceInput,
) -> Result<TruthGuidance, String> {
    let store = get_agent_store(&app, state.inner(), &input.world_id).await?;

    // Get session
    let session = store
        .get_session(&input.session_id)
        .await?
        .ok_or_else(|| "Session not found".to_string())?;

    // Get mainline cursor
    let cursor = store.get_mainline_cursor().await?;

    // Create resolver
    let resolver = HistoricalTruthResolver::new(store.pool().clone());

    // Generate truth guidance
    resolver
        .generate_truth_guidance(
            &input.session_id,
            &session.period_anchor,
            &cursor.mainline_time_anchor,
            &input.world_id,
        )
        .await
}

/// 获取事件的开放细节槽
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOpenDetailSlotsInput {
    pub world_id: String,
    pub session_id: String,
}

/// 获取过去线会话可用的开放细节槽
#[tauri::command]
pub async fn get_open_detail_slots(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: GetOpenDetailSlotsInput,
) -> Result<Vec<OpenDetailSlot>, String> {
    let store = get_agent_store(&app, state.inner(), &input.world_id).await?;

    // Get session
    let session = store
        .get_session(&input.session_id)
        .await?
        .ok_or_else(|| "Session not found".to_string())?;

    // Create resolver
    let resolver = HistoricalTruthResolver::new(store.pool().clone());

    // Get open detail slots
    resolver
        .get_open_detail_slots_for_session(&input.world_id, &session.period_anchor)
        .await
}

/// 填充细节槽的输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillDetailSlotInput {
    pub world_id: String,
    pub session_id: String,
    pub session_turn_id: String,
    pub scene_turn_id: Option<String>,
    pub event_id: String,
    pub slot_id: String,
    pub detail_kind: String,
    pub fill_content: serde_json::Value,
}

/// 填充历史事件的开放细节槽
#[tauri::command]
pub async fn fill_detail_slot(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: FillDetailSlotInput,
) -> Result<DetailSlotFillResult, String> {
    let store = get_agent_store(&app, state.inner(), &input.world_id).await?;

    // Get session for time anchor
    let session = store
        .get_session(&input.session_id)
        .await?
        .ok_or_else(|| "Session not found".to_string())?;

    // Parse detail kind
    let detail_kind = match input.detail_kind.as_str() {
        "motive" => crate::agent::models::knowledge::DetailKind::Motive,
        "dialogue" => crate::agent::models::knowledge::DetailKind::Dialogue,
        "witness" => crate::agent::models::knowledge::DetailKind::Witness,
        "route" => crate::agent::models::knowledge::DetailKind::Route,
        "local_cause" => crate::agent::models::knowledge::DetailKind::LocalCause,
        _ => return Err(format!("Unknown detail kind: {}", input.detail_kind)),
    };

    // Create manager
    let manager = ProvisionalTruthManager::new(store.pool().clone());

    // Create fill request
    let request = DetailSlotFillRequest {
        session_id: input.session_id,
        session_turn_id: input.session_turn_id,
        scene_turn_id: input.scene_turn_id,
        event_id: input.event_id,
        slot_id: input.slot_id,
        detail_kind,
        fill_content: input.fill_content,
    };

    // Fill the slot
    manager
        .fill_detail_slot(request, &session.period_anchor)
        .await
}

/// 获取会话的候选事实列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetProvisionalCandidatesInput {
    pub world_id: String,
    pub session_id: String,
    pub status_filter: Option<String>,
}

/// 获取会话的候选事实列表
#[tauri::command]
pub async fn get_provisional_candidates(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: GetProvisionalCandidatesInput,
) -> Result<Vec<ProvisionalSessionTruth>, String> {
    let store = get_agent_store(&app, state.inner(), &input.world_id).await?;
    let manager = ProvisionalTruthManager::new(store.pool().clone());

    match input.status_filter.as_deref() {
        Some("pending") => manager.get_pending_candidates(&input.session_id).await,
        _ => manager.get_session_candidates(&input.session_id).await,
    }
}

/// 提升候选事实为正史
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromoteCandidatesInput {
    pub world_id: String,
    pub provisional_ids: Vec<String>,
    pub scene_turn_id: String,
}

/// 提升候选事实为正史知识条目
#[tauri::command]
pub async fn promote_provisional_candidates(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: PromoteCandidatesInput,
) -> Result<Vec<String>, String> {
    let store = get_agent_store(&app, state.inner(), &input.world_id).await?;
    let manager = ProvisionalTruthManager::new(store.pool().clone());

    manager
        .batch_promote(&input.provisional_ids, &input.scene_turn_id)
        .await
}

/// 标记候选事实为非正史
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkNonCanonInput {
    pub world_id: String,
    pub provisional_id: String,
}

/// 标记候选事实为非正史
#[tauri::command]
pub async fn mark_provisional_non_canon(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: MarkNonCanonInput,
) -> Result<(), String> {
    let store = get_agent_store(&app, state.inner(), &input.world_id).await?;
    let manager = ProvisionalTruthManager::new(store.pool().clone());

    manager.mark_non_canon(&input.provisional_id).await
}

// ============================================================================
// 正史资格提升命令
// ============================================================================

/// 评估会话的正史资格
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluateCanonEligibilityInput {
    pub world_id: String,
    pub session_id: String,
}

/// 评估会话的正史资格
#[tauri::command]
pub async fn evaluate_canon_eligibility(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: EvaluateCanonEligibilityInput,
) -> Result<PromotionEvaluationResult, String> {
    let store = get_agent_store(&app, state.inner(), &input.world_id).await?;

    // Get session
    let session = store
        .get_session(&input.session_id)
        .await?
        .ok_or_else(|| "Session not found".to_string())?;

    // Get mainline cursor
    let cursor = store.get_mainline_cursor().await?;

    // Create resolver and generate truth guidance
    let resolver = HistoricalTruthResolver::new(store.pool().clone());
    let guidance = resolver
        .generate_truth_guidance(
            &input.session_id,
            &session.period_anchor,
            &cursor.mainline_time_anchor,
            &input.world_id,
        )
        .await?;

    // Evaluate for promotion
    let manager = CanonStatusManager::new(store.pool().clone());
    manager.evaluate_for_promotion(&session, &guidance).await
}

/// 提升候选事实为正史
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromoteToCanonInput {
    pub world_id: String,
    pub session_id: String,
    pub scene_turn_id: String,
}

/// 提升符合条件的候选事实为正史
#[tauri::command]
pub async fn promote_to_canon(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: PromoteToCanonInput,
) -> Result<PromotionResult, String> {
    let store = get_agent_store(&app, state.inner(), &input.world_id).await?;

    // Get session
    let session = store
        .get_session(&input.session_id)
        .await?
        .ok_or_else(|| "Session not found".to_string())?;

    // Get mainline cursor
    let cursor = store.get_mainline_cursor().await?;

    // Create resolver and generate truth guidance
    let resolver = HistoricalTruthResolver::new(store.pool().clone());
    let guidance = resolver
        .generate_truth_guidance(
            &input.session_id,
            &session.period_anchor,
            &cursor.mainline_time_anchor,
            &input.world_id,
        )
        .await?;

    // Promote eligible truths
    let manager = CanonStatusManager::new(store.pool().clone());
    manager
        .promote_eligible_truths(&session, &guidance, &input.scene_turn_id)
        .await
}

/// 获取会话的冲突报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSessionConflictsInput {
    pub world_id: String,
    pub session_id: String,
}

/// 获取会话的所有冲突报告
#[tauri::command]
pub async fn get_session_conflicts(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: GetSessionConflictsInput,
) -> Result<Vec<crate::agent::models::session::ConflictReport>, String> {
    let store = get_agent_store(&app, state.inner(), &input.world_id).await?;
    let manager = CanonStatusManager::new(store.pool().clone());

    manager.get_session_conflicts(&input.session_id).await
}

#[cfg(test)]
mod tests {
    use crate::storage::paths::validate_path_component;

    #[test]
    fn rejects_world_id_with_parent_traversal() {
        let error = validate_path_component("..").expect_err("parent traversal should fail");
        assert!(error.contains("Invalid"));
    }

    #[test]
    fn rejects_world_id_with_path_separator_payload() {
        let error =
            validate_path_component("world/name").expect_err("separator payload should fail");
        assert!(error.contains("Invalid path component"));
    }
}
