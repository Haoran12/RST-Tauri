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
use crate::agent::world_editor::commit::WorldEditorChanges;
use crate::agent::world_editor::validator::ValidationSeverity;
use crate::agent::world_editor::{WorldEditorCommitter, WorldEditorValidator};
use crate::agent::runtime::{AgentRuntime, TurnResult};
use crate::agent::simulation::canon_status_manager::{PromotionEvaluationResult, PromotionResult};
use crate::agent::simulation::provisional_truth_manager::{
    DetailSlotFillRequest, DetailSlotFillResult,
};
use crate::agent::simulation::{
    CanonStatusManager, HistoricalTruthResolver, ProvisionalTruthManager,
};
use crate::agent::storage::agent_store::AgentStore;
use crate::config::world_argument::{
    ensure_world_argument_file, load_world_argument_from_dir, parse_world_argument_yaml,
    WORLD_ARGUMENT_FILE_NAME,
};
use crate::storage::paths::{app_data_root, safe_join, validate_path_component};
use crate::AppState;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use std::collections::HashSet;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentWorldInput {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEditorSnapshotDto {
    pub world_id: String,
    pub editor_revision: u64,
    pub world_status: String,
    pub locations: Vec<WorldEditorLocationSummaryDto>,
    pub knowledges: Vec<WorldEditorKnowledgeSummaryDto>,
    pub characters: Vec<WorldEditorCharacterSummaryDto>,
    pub relationships: Vec<WorldEditorRelationshipSummaryDto>,
    pub world_rules_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEditorLocationSummaryDto {
    pub location_id: String,
    pub name: String,
    pub canonical_level: String,
    pub parent_id: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEditorKnowledgeSummaryDto {
    pub knowledge_id: String,
    pub kind: String,
    pub subject_type: String,
    pub subject_id: Option<String>,
    pub facet_type: Option<String>,
    pub summary_text: String,
    pub has_god_only: bool,
    pub has_apparent_content: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEditorCharacterSummaryDto {
    pub character_id: String,
    pub base_attributes_summary: String,
    pub mana_expression_tendency: String,
    pub temporary_state_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEditorRelationshipSummaryDto {
    pub relation_id: String,
    pub subject_character_id: String,
    pub target_character_id: String,
    pub relation_kind: String,
    pub access_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendWorldEditorPatch {
    pub world_id: String,
    pub base_editor_revision: u64,
    pub operations: Vec<FrontendWorldEditorOperation>,
    pub author_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendWorldEditorOperation {
    pub kind: String,
    pub payload: Option<Value>,
    pub location_id: Option<String>,
    pub knowledge_id: Option<String>,
    pub character_id: Option<String>,
    pub relation_id: Option<String>,
    pub state_record_id: Option<String>,
    pub normalized_alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEditorValidationItemDto {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub field_path: Option<String>,
    pub entity_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEditorValidationResultDto {
    pub is_valid: bool,
    pub blockers: Vec<WorldEditorValidationItemDto>,
    pub warnings: Vec<WorldEditorValidationItemDto>,
    pub info: Vec<WorldEditorValidationItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEditorCommitResultDto {
    pub success: bool,
    pub commit_id: Option<String>,
    pub new_revision: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEditorImpactItemDto {
    pub kind: String,
    pub target_entity_type: String,
    pub target_entity_id: String,
    pub description: String,
    pub affected_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendKnowledgeEntryDto {
    pub knowledge_id: String,
    pub kind: String,
    pub subject_type: String,
    pub subject_id: Option<String>,
    pub facet_type: Option<String>,
    pub content: Value,
    pub apparent_content: Option<Value>,
    pub access_policy: Value,
    pub subject_awareness: Value,
    pub metadata: Value,
    pub valid_from: Option<Value>,
    pub valid_until: Option<Value>,
    pub source_session_id: Option<String>,
    pub source_scene_turn_id: Option<String>,
    pub derived_from_event_id: Option<String>,
    pub schema_version: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTraceEventDto {
    pub event_id: String,
    pub event_type: String,
    pub timestamp: String,
    pub scene_turn_id: Option<String>,
    pub character_id: Option<String>,
    pub summary: String,
    pub details: Value,
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionWindowEntryDto {
    pub entry_id: String,
    pub scene_turn_id: String,
    pub character_id: String,
    pub reaction_type: String,
    pub content: String,
    pub confidence: f64,
    pub latency_ms: u32,
    pub created_at: String,
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

/// 创建一个新的 Agent World，并初始化目录、world.sqlite 与 world_argument.yaml。
#[tauri::command]
pub async fn create_agent_world(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: CreateAgentWorldInput,
) -> Result<AgentWorldListItem, String> {
    let trimmed_name = input.name.trim();
    if trimmed_name.is_empty() {
        return Err("World name must not be empty".to_string());
    }

    let data_dir = get_data_dir(&app)?;
    let worlds_dir = data_dir.join("worlds");
    std::fs::create_dir_all(&worlds_dir)
        .map_err(|e| format!("Failed to create worlds directory: {}", e))?;

    let existing_world_ids = collect_existing_world_ids(&worlds_dir)?;
    let world_id = allocate_world_id(trimmed_name, &existing_world_ids)?;
    let world_dir = safe_join(&data_dir, &format!("worlds/{}", world_id))?;
    let assets_dir = safe_join(&world_dir, "assets")?;
    std::fs::create_dir_all(&assets_dir)
        .map_err(|e| format!("Failed to create world directory '{}': {}", world_id, e))?;

    ensure_world_argument_file(&world_dir, trimmed_name)?;

    let store = get_agent_store(&app, state.inner(), &world_id).await?;
    let mainline_cursor = store.get_mainline_cursor().await?;

    Ok(AgentWorldListItem {
        world_id,
        session_count: 0,
        active_session_count: 0,
        character_count: 0,
        mainline_time_anchor: Some(mainline_cursor.mainline_time_anchor),
        updated_at: Some(mainline_cursor.updated_at.to_rfc3339()),
    })
}

#[tauri::command]
pub async fn get_world_editor_snapshot(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    world_id: String,
) -> Result<WorldEditorSnapshotDto, String> {
    let store = get_agent_store(&app, state.inner(), &world_id).await?;
    let pool = store.pool();

    let locations = sqlx::query(
        r#"
        SELECT location_id, name, canonical_level, parent_id, status
        FROM location_nodes
        ORDER BY created_at ASC, location_id ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to load world editor locations: {}", e))?
    .into_iter()
    .map(|row| WorldEditorLocationSummaryDto {
        location_id: row.get("location_id"),
        name: row.get("name"),
        canonical_level: row.get("canonical_level"),
        parent_id: row.get("parent_id"),
        status: row.get("status"),
    })
    .collect::<Vec<_>>();

    let knowledges = sqlx::query(
        r#"
        SELECT knowledge_id, kind, subject_type, subject_id, facet_type, content,
               apparent_content, access_policy, updated_at
        FROM knowledge_entries
        ORDER BY updated_at DESC, knowledge_id ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to load world editor knowledges: {}", e))?
    .into_iter()
    .map(|row| {
        let content_text: String = row.get("content");
        let apparent_content_text: Option<String> = row.get("apparent_content");
        let access_policy_text: String = row.get("access_policy");
        let content_value: Value = serde_json::from_str(&content_text).unwrap_or(Value::Null);
        let summary_text = content_value
            .get("summary_text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let has_god_only = access_policy_text.contains("GodOnly");

        WorldEditorKnowledgeSummaryDto {
            knowledge_id: row.get("knowledge_id"),
            kind: row.get("kind"),
            subject_type: row.get("subject_type"),
            subject_id: row.get("subject_id"),
            facet_type: row.get("facet_type"),
            summary_text,
            has_god_only,
            has_apparent_content: apparent_content_text.is_some(),
            updated_at: row.get("updated_at"),
        }
    })
    .collect::<Vec<_>>();

    let characters = store
        .list_characters()
        .await?
        .into_iter()
        .map(|character| WorldEditorCharacterSummaryDto {
            character_id: character.character_id,
            base_attributes_summary: format!(
                "体{:.0}/敏{:.0}/耐{:.0}/悟{:.0}/灵{:.0}/魂{:.0}",
                character.base_attributes.physical,
                character.base_attributes.agility,
                character.base_attributes.endurance,
                character.base_attributes.insight,
                character.base_attributes.mana_power,
                character.base_attributes.soul_strength,
            ),
            mana_expression_tendency: format!("{:?}", character.mana_expression_tendency),
            temporary_state_summary: format!(
                "疲劳 {:.0}% / 痛感 {:.0}%",
                character.temporary_state.fatigue * 100.0,
                character.temporary_state.pain_load * 100.0,
            ),
        })
        .collect::<Vec<_>>();

    let relationships = store
        .list_objective_relationships()
        .await?
        .into_iter()
        .map(|relation| WorldEditorRelationshipSummaryDto {
            relation_id: relation.relation_id,
            subject_character_id: relation.subject_character_id,
            target_character_id: relation.target_character_id,
            relation_kind: format!("{:?}", relation.relation_kind),
            access_level: relation.access_level,
        })
        .collect::<Vec<_>>();

    let editor_revision = load_world_editor_revision(pool, &world_id).await?;
    let world_status = detect_world_editor_status(pool, &world_id).await?;

    Ok(WorldEditorSnapshotDto {
        world_id,
        editor_revision,
        world_status,
        locations,
        knowledges,
        characters,
        relationships,
        world_rules_keys: vec![WORLD_ARGUMENT_FILE_NAME.to_string()],
    })
}

#[tauri::command]
pub async fn validate_world_editor_patch(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    world_id: String,
    patch: FrontendWorldEditorPatch,
) -> Result<WorldEditorValidationResultDto, String> {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let mut info = Vec::new();

    if patch.world_id != world_id {
        blockers.push(WorldEditorValidationItemDto {
            severity: "blocker".to_string(),
            code: "world_id_mismatch".to_string(),
            message: "请求中的 world_id 与路由 worldId 不一致。".to_string(),
            field_path: Some("world_id".to_string()),
            entity_id: None,
        });
    }

    let store = get_agent_store(&app, state.inner(), &world_id).await?;
    let pool = store.pool().clone();

    if let Some(yaml_text) = extract_world_rules_yaml(&patch)? {
        match parse_world_argument_yaml(&yaml_text) {
            Ok(_) => info.push(WorldEditorValidationItemDto {
                severity: "info".to_string(),
                code: "world_argument_valid".to_string(),
                message: "world_argument.yaml 已通过 YAML 解析与 schema 校验。".to_string(),
                field_path: Some(WORLD_ARGUMENT_FILE_NAME.to_string()),
                entity_id: None,
            }),
            Err(error) => blockers.push(WorldEditorValidationItemDto {
                severity: "blocker".to_string(),
                code: "invalid_world_argument".to_string(),
                message: error,
                field_path: Some(WORLD_ARGUMENT_FILE_NAME.to_string()),
                entity_id: None,
            }),
        }
    }

    match build_world_editor_changes(&pool, &patch).await {
        Ok(changes) => {
            append_validation_items(
                &mut blockers,
                &mut warnings,
                &mut info,
                validate_world_editor_changes(&changes)?,
            );
        }
        Err(error) => blockers.push(WorldEditorValidationItemDto {
            severity: "blocker".to_string(),
            code: "invalid_patch".to_string(),
            message: error,
            field_path: Some("operations".to_string()),
            entity_id: None,
        }),
    }

    Ok(WorldEditorValidationResultDto {
        is_valid: blockers.is_empty(),
        blockers,
        warnings,
        info,
    })
}

#[tauri::command]
pub async fn commit_world_editor_patch(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    world_id: String,
    patch: FrontendWorldEditorPatch,
) -> Result<WorldEditorCommitResultDto, String> {
    let store = get_agent_store(&app, state.inner(), &world_id).await?;
    let pool = store.pool().clone();
    let current_revision = load_world_editor_revision(store.pool(), &world_id).await?;

    if patch.base_editor_revision != current_revision {
        return Ok(WorldEditorCommitResultDto {
            success: false,
            commit_id: None,
            new_revision: current_revision,
            error: Some(format!(
                "editor revision 已过期，当前 revision={}，提交基线={}",
                current_revision, patch.base_editor_revision
            )),
        });
    }

    let world_rules_yaml = extract_world_rules_yaml(&patch)?;
    if let Some(yaml_text) = world_rules_yaml {
        parse_world_argument_yaml(&yaml_text)
            .map_err(|e| format!("Refusing to write invalid {}: {}", WORLD_ARGUMENT_FILE_NAME, e))?;
        let data_dir = get_data_dir(&app)?;
        let world_argument_path =
            safe_join(&data_dir, &format!("worlds/{}/{}", world_id, WORLD_ARGUMENT_FILE_NAME))?;
        std::fs::write(&world_argument_path, yaml_text)
            .map_err(|e| format!("Failed to write {}: {}", WORLD_ARGUMENT_FILE_NAME, e))?;
    }

    let changes = match build_world_editor_changes(&pool, &patch).await {
        Ok(changes) => changes,
        Err(error) => {
            return Ok(WorldEditorCommitResultDto {
                success: false,
                commit_id: None,
                new_revision: current_revision,
                error: Some(error),
            })
        }
    };

    let committer = WorldEditorCommitter::new(pool);
    match committer
        .commit(&world_id, patch.base_editor_revision, changes)
        .await
    {
        Ok(result) => Ok(WorldEditorCommitResultDto {
            success: true,
            commit_id: Some(result.commit_id),
            new_revision: result.resulting_revision,
            error: None,
        }),
        Err(error) => Ok(WorldEditorCommitResultDto {
            success: false,
            commit_id: None,
            new_revision: current_revision,
            error: Some(error),
        }),
    }
}

#[tauri::command]
pub async fn get_world_argument_detail(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    world_id: String,
) -> Result<String, String> {
    validate_path_component(&world_id)
        .map_err(|e| format!("Invalid world_id '{}': {}", world_id, e))?;

    let data_dir = get_data_dir(&app)?;
    let world_dir = safe_join(&data_dir, &format!("worlds/{}", world_id))?;
    let (config, path) = load_world_argument_from_dir(&world_dir)?;

    let source_text = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    parse_world_argument_yaml(&source_text)?;

    serde_yaml::to_string(&config)
        .map_err(|e| format!("Failed to serialize {}: {}", WORLD_ARGUMENT_FILE_NAME, e))
}

#[tauri::command]
pub async fn analyze_world_editor_impact(
    _app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    _world_id: String,
    _entity_type: String,
    _entity_id: String,
) -> Result<Vec<WorldEditorImpactItemDto>, String> {
    Ok(Vec::new())
}

#[tauri::command]
pub async fn get_location_node_detail(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    world_id: String,
    location_id: String,
) -> Result<Value, String> {
    let store = get_agent_store(&app, state.inner(), &world_id).await?;
    let row = sqlx::query(
        r#"
        SELECT location_id, name, polity_id, parent_id, canonical_level,
               type_label, tags, status, metadata, schema_version, created_at, updated_at
        FROM location_nodes
        WHERE location_id = ?
        "#,
    )
    .bind(&location_id)
    .fetch_optional(store.pool())
    .await
    .map_err(|e| format!("Failed to load location detail: {}", e))?
    .ok_or_else(|| format!("Location not found: {}", location_id))?;

    let aliases = sqlx::query(
        "SELECT alias, locale, normalized_alias FROM location_aliases WHERE location_id = ? ORDER BY alias ASC",
    )
    .bind(&location_id)
    .fetch_all(store.pool())
    .await
    .map_err(|e| format!("Failed to load location aliases: {}", e))?
    .into_iter()
    .map(|alias_row| {
        serde_json::json!({
            "alias": alias_row.get::<String, _>("alias"),
            "locale": alias_row.get::<Option<String>, _>("locale"),
            "normalized_alias": alias_row.get::<String, _>("normalized_alias"),
        })
    })
    .collect::<Vec<_>>();

    Ok(serde_json::json!({
        "location_id": row.get::<String, _>("location_id"),
        "name": row.get::<String, _>("name"),
        "aliases": aliases,
        "polity_id": row.get::<Option<String>, _>("polity_id"),
        "parent_id": row.get::<Option<String>, _>("parent_id"),
        "canonical_level": row.get::<String, _>("canonical_level"),
        "type_label": row.get::<String, _>("type_label"),
        "tags": parse_json_value(row.get("tags")),
        "status": row.get::<String, _>("status"),
        "metadata": parse_json_value(row.get("metadata")),
        "schema_version": row.get::<String, _>("schema_version"),
        "created_at": row.get::<String, _>("created_at"),
        "updated_at": row.get::<String, _>("updated_at"),
    }))
}

#[tauri::command]
pub async fn get_knowledge_entry_detail(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    world_id: String,
    knowledge_id: String,
) -> Result<FrontendKnowledgeEntryDto, String> {
    let store = get_agent_store(&app, state.inner(), &world_id).await?;
    let row = sqlx::query(
        r#"
        SELECT knowledge_id, kind, subject_type, subject_id, facet_type, content,
               apparent_content, access_policy, subject_awareness, metadata,
               valid_from, valid_until, source_session_id, source_scene_turn_id,
               derived_from_event_id, schema_version, created_at, updated_at
        FROM knowledge_entries
        WHERE knowledge_id = ?
        "#,
    )
    .bind(&knowledge_id)
    .fetch_optional(store.pool())
    .await
    .map_err(|e| format!("Failed to load knowledge detail: {}", e))?
    .ok_or_else(|| format!("Knowledge not found: {}", knowledge_id))?;

    Ok(FrontendKnowledgeEntryDto {
        knowledge_id: row.get("knowledge_id"),
        kind: row.get("kind"),
        subject_type: row.get("subject_type"),
        subject_id: row.get("subject_id"),
        facet_type: row.get("facet_type"),
        content: parse_json_value(row.get("content")),
        apparent_content: row
            .get::<Option<String>, _>("apparent_content")
            .map(parse_json_value),
        access_policy: parse_json_value(row.get("access_policy")),
        subject_awareness: parse_json_value(row.get("subject_awareness")),
        metadata: parse_json_value(row.get("metadata")),
        valid_from: row.get::<Option<String>, _>("valid_from").map(parse_json_value),
        valid_until: row.get::<Option<String>, _>("valid_until").map(parse_json_value),
        source_session_id: row.get("source_session_id"),
        source_scene_turn_id: row.get("source_scene_turn_id"),
        derived_from_event_id: row.get("derived_from_event_id"),
        schema_version: row.get("schema_version"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

#[tauri::command]
pub async fn get_agent_trace_events(
    _app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    _world_id: String,
    _limit: Option<u32>,
) -> Result<Vec<AgentTraceEventDto>, String> {
    Ok(Vec::new())
}

#[tauri::command]
pub async fn get_reaction_window_entries(
    _app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    _world_id: String,
    _session_id: Option<String>,
) -> Result<Vec<ReactionWindowEntryDto>, String> {
    Ok(Vec::new())
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
pub struct DeleteAgentSessionInput {
    pub world_id: String,
    pub session_id: String,
}

/// 删除 Agent 会话及其所有回合记录。
#[tauri::command]
pub async fn delete_agent_session(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: DeleteAgentSessionInput,
) -> Result<(), String> {
    let store = get_agent_store(&app, state.inner(), &input.world_id).await?;
    let session = store
        .get_session(&input.session_id)
        .await?
        .ok_or_else(|| "Session not found".to_string())?;
    if session.world_id != input.world_id {
        return Err("Session does not belong to requested world".to_string());
    }
    store.delete_session(&input.session_id).await
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

fn collect_existing_world_ids(worlds_dir: &PathBuf) -> Result<HashSet<String>, String> {
    let mut ids = HashSet::new();
    let entries = std::fs::read_dir(worlds_dir)
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
        if validate_path_component(&world_id).is_ok() {
            ids.insert(world_id);
        }
    }
    Ok(ids)
}

async fn load_world_editor_revision(
    pool: &sqlx::SqlitePool,
    world_id: &str,
) -> Result<u64, String> {
    let revision: Option<i64> = sqlx::query_scalar(
        "SELECT MAX(resulting_editor_revision) FROM world_editor_commits WHERE world_id = ?",
    )
    .bind(world_id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to load world editor revision: {}", e))?;

    Ok(revision.unwrap_or(0).max(0) as u64)
}

async fn detect_world_editor_status(
    pool: &sqlx::SqlitePool,
    world_id: &str,
) -> Result<String, String> {
    let active_turns: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM world_turns wt
        INNER JOIN agent_sessions s ON s.session_id = wt.session_id
        WHERE s.world_id = ? AND wt.status = 'active'
        "#,
    )
    .bind(world_id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to count active turns: {}", e))?;

    if active_turns > 0 {
        return Ok("active_turn".to_string());
    }

    let pending_calls: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM llm_call_logs WHERE world_id = ? AND status = 'started'",
    )
    .bind(world_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if pending_calls > 0 {
        return Ok("pending_llm".to_string());
    }

    Ok("paused".to_string())
}

fn parse_json_value(raw: String) -> Value {
    serde_json::from_str(&raw).unwrap_or(Value::Null)
}

fn parse_rfc3339_timestamp(value: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| format!("Invalid RFC3339 timestamp '{}': {}", value, e))
}

fn append_validation_items(
    blockers: &mut Vec<WorldEditorValidationItemDto>,
    warnings: &mut Vec<WorldEditorValidationItemDto>,
    info: &mut Vec<WorldEditorValidationItemDto>,
    issues: Vec<crate::agent::world_editor::validator::ValidationIssue>,
) {
    for issue in issues {
        let item = WorldEditorValidationItemDto {
            severity: match issue.severity {
                ValidationSeverity::Error => "blocker".to_string(),
                ValidationSeverity::Warning => "warning".to_string(),
            },
            code: "world_editor_validation".to_string(),
            message: issue.message,
            field_path: Some(issue.field_path),
            entity_id: None,
        };
        match issue.severity {
            ValidationSeverity::Error => blockers.push(item),
            ValidationSeverity::Warning => warnings.push(item),
        }
    }

    info.push(WorldEditorValidationItemDto {
        severity: "info".to_string(),
        code: "world_editor_validation_complete".to_string(),
        message: "World Editor patch 已执行真实模型校验。".to_string(),
        field_path: None,
        entity_id: None,
    });
}

fn validate_world_editor_changes(
    changes: &WorldEditorChanges,
) -> Result<Vec<crate::agent::world_editor::validator::ValidationIssue>, String> {
    let mut issues = Vec::new();

    for location in changes
        .location_creates
        .iter()
        .chain(changes.location_updates.iter())
    {
        issues.extend(WorldEditorValidator::validate_location(location)?);
    }

    for knowledge in changes
        .knowledge_creates
        .iter()
        .chain(changes.knowledge_updates.iter())
    {
        issues.extend(WorldEditorValidator::validate_knowledge(knowledge)?);
    }

    for character in changes
        .character_creates
        .iter()
        .chain(changes.character_updates.iter())
    {
        issues.extend(WorldEditorValidator::validate_character(character)?);
    }

    issues.extend(validate_world_editor_cross_references(changes));

    Ok(issues)
}

fn validate_world_editor_cross_references(
    changes: &WorldEditorChanges,
) -> Vec<crate::agent::world_editor::validator::ValidationIssue> {
    use crate::agent::models::knowledge::{CharacterFacetType, KnowledgeKind, KnowledgeSubject};
    use crate::agent::world_editor::validator::ValidationIssue;
    use std::collections::{HashMap, HashSet};

    let mut issues = Vec::new();

    let patch_character_ids: HashSet<&str> = changes
        .character_creates
        .iter()
        .chain(changes.character_updates.iter())
        .map(|character| character.character_id.as_str())
        .collect();

    let patch_knowledge: HashMap<&str, (&KnowledgeKind, &KnowledgeSubject)> = changes
        .knowledge_creates
        .iter()
        .chain(changes.knowledge_updates.iter())
        .map(|entry| (entry.knowledge_id.as_str(), (&entry.kind, &entry.subject)))
        .collect();

    for entry in changes
        .knowledge_creates
        .iter()
        .chain(changes.knowledge_updates.iter())
    {
        if let KnowledgeSubject::Character { id, .. } = &entry.subject {
            if id.trim().is_empty() {
                continue;
            }
            if !patch_character_ids.contains(id.as_str()) {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Warning,
                    field_path: "subject_id".to_string(),
                    message: format!(
                        "CharacterFacet 引用的角色 '{}' 未包含在当前 patch 中；若库内也不存在，提交阶段会失败。",
                        id
                    ),
                });
            }
        }
    }

    for character in changes
        .character_creates
        .iter()
        .chain(changes.character_updates.iter())
    {
        let knowledge_id = character.mind_model_card_knowledge_id.trim();
        if knowledge_id.is_empty() {
            continue;
        }

        if let Some((kind, subject)) = patch_knowledge.get(knowledge_id) {
            let subject_matches = matches!(
                (kind, subject),
                (
                    KnowledgeKind::CharacterFacet,
                    KnowledgeSubject::Character {
                        id,
                        facet: CharacterFacetType::MindModelCard
                    }
                ) if id == &character.character_id
            );

            if !subject_matches {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    field_path: "mind_model_card_knowledge_id".to_string(),
                    message: format!(
                        "mind_model_card_knowledge_id '{}' 在当前 patch 中存在，但不是该角色的 MindModelCard CharacterFacet。",
                        knowledge_id
                    ),
                });
            }
        }
    }

    issues
}

fn extract_world_rules_yaml(patch: &FrontendWorldEditorPatch) -> Result<Option<String>, String> {
    let mut yaml: Option<String> = None;
    for operation in &patch.operations {
        if operation.kind != "UpsertWorldRules" {
            continue;
        }
        let payload = operation.payload.clone().unwrap_or(Value::Null);
        let yaml_text = payload
            .as_str()
            .ok_or_else(|| "UpsertWorldRules payload 必须是 YAML 字符串".to_string())?;
        yaml = Some(yaml_text.to_string());
    }
    Ok(yaml)
}

async fn build_world_editor_changes(
    pool: &sqlx::SqlitePool,
    patch: &FrontendWorldEditorPatch,
) -> Result<WorldEditorChanges, String> {
    let mut changes = WorldEditorChanges::default();

    for operation in &patch.operations {
        match operation.kind.as_str() {
            "UpsertKnowledgeEntry" => {
                let payload = operation
                    .payload
                    .clone()
                    .ok_or_else(|| "UpsertKnowledgeEntry 缺少 payload".to_string())?;
                let entry = frontend_knowledge_to_model(payload)?;
                if knowledge_exists(pool, &entry.knowledge_id).await? {
                    changes.knowledge_updates.push(entry);
                } else {
                    changes.knowledge_creates.push(entry);
                }
            }
            "DeleteKnowledgeEntry" => {
                let knowledge_id = operation
                    .knowledge_id
                    .clone()
                    .ok_or_else(|| "DeleteKnowledgeEntry 缺少 knowledge_id".to_string())?;
                changes.knowledge_deletes.push(knowledge_id);
            }
            "UpsertCharacterRecord" => {
                let payload = operation
                    .payload
                    .clone()
                    .ok_or_else(|| "UpsertCharacterRecord 缺少 payload".to_string())?;
                let character = serde_json::from_value::<CharacterRecord>(payload)
                    .map_err(|e| format!("Invalid CharacterRecord payload: {}", e))?;
                if character_exists(pool, &character.character_id).await? {
                    changes.character_updates.push(character);
                } else {
                    changes.character_creates.push(character);
                }
            }
            "DeleteCharacterRecord" => {
                let character_id = operation
                    .character_id
                    .clone()
                    .ok_or_else(|| "DeleteCharacterRecord 缺少 character_id".to_string())?;
                changes.character_deletes.push(character_id);
            }
            "UpsertLocationNode" => {
                let payload = operation
                    .payload
                    .clone()
                    .ok_or_else(|| "UpsertLocationNode 缺少 payload".to_string())?;
                let location = frontend_location_to_model(payload)?;
                if location_exists(pool, &location.location_id).await? {
                    changes.location_updates.push(location);
                } else {
                    changes.location_creates.push(location);
                }
            }
            "DeleteLocationNode" => {
                let location_id = operation
                    .location_id
                    .clone()
                    .ok_or_else(|| "DeleteLocationNode 缺少 location_id".to_string())?;
                changes.location_deletes.push(location_id);
            }
            "UpsertWorldRules" => {}
            other => {
                return Err(format!("暂不支持的 World Editor 操作: {}", other));
            }
        }
    }

    Ok(changes)
}

fn frontend_knowledge_to_model(payload: Value) -> Result<crate::agent::models::knowledge::KnowledgeEntry, String> {
    let dto: FrontendKnowledgeEntryDto = serde_json::from_value(payload)
        .map_err(|e| format!("Invalid KnowledgeEntry payload: {}", e))?;

    let kind = parse_knowledge_kind(&dto.kind)?;
    let subject = parse_knowledge_subject(&dto.subject_type, dto.subject_id.clone(), dto.facet_type.clone())?;
    let access_policy = serde_json::from_value(dto.access_policy)
        .map_err(|e| format!("Invalid access_policy payload: {}", e))?;
    let subject_awareness = serde_json::from_value(dto.subject_awareness)
        .map_err(|e| format!("Invalid subject_awareness payload: {}", e))?;
    let metadata = serde_json::from_value(dto.metadata)
        .map_err(|e| format!("Invalid metadata payload: {}", e))?;

    Ok(crate::agent::models::knowledge::KnowledgeEntry {
        knowledge_id: dto.knowledge_id,
        kind,
        subject,
        content: dto.content,
        apparent_content: dto.apparent_content,
        access_policy,
        subject_awareness,
        metadata,
        valid_from: parse_optional_time_anchor_value(dto.valid_from)?,
        valid_until: parse_optional_time_anchor_value(dto.valid_until)?,
        source_session_id: dto.source_session_id,
        source_scene_turn_id: dto.source_scene_turn_id,
        derived_from_event_id: dto.derived_from_event_id,
        schema_version: dto.schema_version,
        created_at: parse_rfc3339_timestamp(&dto.created_at)?,
        updated_at: parse_rfc3339_timestamp(&dto.updated_at)?,
    })
}

fn frontend_location_to_model(payload: Value) -> Result<crate::agent::models::location::LocationNode, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "Invalid LocationNode payload: expected object".to_string())?;

    let location_id = required_string(object, "location_id")?;
    let name = required_string(object, "name")?;
    let canonical_level_raw = required_string(object, "canonical_level")?;
    let canonical_level = parse_location_level(&canonical_level_raw)?;
    let status_raw = required_string(object, "status")?;
    let status = parse_location_status(&status_raw)?;
    let type_label = optional_string(object, "type_label").unwrap_or_else(|| canonical_level_raw.clone());
    let schema_version = optional_string(object, "schema_version").unwrap_or_else(|| "0.1".to_string());
    let created_at = parse_optional_datetime_string(optional_string(object, "created_at"))?.unwrap_or_else(Utc::now);
    let updated_at = parse_optional_datetime_string(optional_string(object, "updated_at"))?.unwrap_or_else(Utc::now);
    let polity_id = nullable_string_field(object, "polity_id")?;
    let parent_id = nullable_string_field(object, "parent_id")?;
    let tags = object
        .get("tags")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let metadata = object
        .get("metadata")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let aliases = object
        .get("aliases")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));

    Ok(crate::agent::models::location::LocationNode {
        location_id,
        name,
        aliases: serde_json::from_value(aliases)
            .map_err(|e| format!("Invalid location aliases payload: {}", e))?,
        polity_id,
        parent_id,
        canonical_level,
        type_label,
        tags: serde_json::from_value(tags).map_err(|e| format!("Invalid location tags payload: {}", e))?,
        status,
        metadata,
        schema_version,
        created_at,
        updated_at,
    })
}

async fn knowledge_exists(pool: &sqlx::SqlitePool, knowledge_id: &str) -> Result<bool, String> {
    exists_by_id(pool, "knowledge_entries", "knowledge_id", knowledge_id).await
}

async fn character_exists(pool: &sqlx::SqlitePool, character_id: &str) -> Result<bool, String> {
    exists_by_id(pool, "character_records", "character_id", character_id).await
}

async fn location_exists(pool: &sqlx::SqlitePool, location_id: &str) -> Result<bool, String> {
    exists_by_id(pool, "location_nodes", "location_id", location_id).await
}

async fn exists_by_id(
    pool: &sqlx::SqlitePool,
    table: &str,
    column: &str,
    value: &str,
) -> Result<bool, String> {
    let sql = format!("SELECT 1 FROM {} WHERE {} = ? LIMIT 1", table, column);
    let exists = sqlx::query_scalar::<_, i64>(&sql)
        .bind(value)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Failed to check {} existence: {}", table, e))?;
    Ok(exists.is_some())
}

fn parse_knowledge_kind(
    raw: &str,
) -> Result<crate::agent::models::knowledge::KnowledgeKind, String> {
    use crate::agent::models::knowledge::KnowledgeKind;
    match raw {
        "world_fact" => Ok(KnowledgeKind::WorldFact),
        "region_fact" => Ok(KnowledgeKind::RegionFact),
        "faction_fact" => Ok(KnowledgeKind::FactionFact),
        "character_facet" => Ok(KnowledgeKind::CharacterFacet),
        "historical_event" => Ok(KnowledgeKind::HistoricalEvent),
        "memory" => Ok(KnowledgeKind::Memory),
        _ => Err(format!("Invalid knowledge kind: {}", raw)),
    }
}

fn parse_knowledge_subject(
    subject_type: &str,
    subject_id: Option<String>,
    facet_type: Option<String>,
) -> Result<crate::agent::models::knowledge::KnowledgeSubject, String> {
    use crate::agent::models::knowledge::KnowledgeSubject;
    Ok(match subject_type {
        "world" => KnowledgeSubject::World,
        "region" => KnowledgeSubject::Region(
            subject_id.ok_or_else(|| "Region knowledge requires subject_id".to_string())?,
        ),
        "faction" => KnowledgeSubject::Faction(
            subject_id.ok_or_else(|| "Faction knowledge requires subject_id".to_string())?,
        ),
        "character" => KnowledgeSubject::Character {
            id: subject_id.ok_or_else(|| "Character knowledge requires subject_id".to_string())?,
            facet: parse_character_facet_type(
                &facet_type.ok_or_else(|| "Character knowledge requires facet_type".to_string())?,
            )?,
        },
        "event" => KnowledgeSubject::Event {
            event_id: subject_id.ok_or_else(|| "Event knowledge requires subject_id".to_string())?,
        },
        _ => return Err(format!("Invalid knowledge subject_type: {}", subject_type)),
    })
}

fn parse_character_facet_type(
    raw: &str,
) -> Result<crate::agent::models::knowledge::CharacterFacetType, String> {
    use crate::agent::models::knowledge::CharacterFacetType;
    match raw {
        "Appearance" | "appearance" => Ok(CharacterFacetType::Appearance),
        "Identity" | "identity" => Ok(CharacterFacetType::Identity),
        "TrueName" | "true_name" => Ok(CharacterFacetType::TrueName),
        "Species" | "species" => Ok(CharacterFacetType::Species),
        "Bloodline" | "bloodline" => Ok(CharacterFacetType::Bloodline),
        "CultivationRealm" | "cultivation_realm" => Ok(CharacterFacetType::CultivationRealm),
        "KnownAbility" | "known_ability" => Ok(CharacterFacetType::KnownAbility),
        "HiddenAbility" | "hidden_ability" => Ok(CharacterFacetType::HiddenAbility),
        "Personality" | "personality" => Ok(CharacterFacetType::Personality),
        "Background" | "background" => Ok(CharacterFacetType::Background),
        "Motivation" | "motivation" => Ok(CharacterFacetType::Motivation),
        "Trauma" | "trauma" => Ok(CharacterFacetType::Trauma),
        "MindModelCard" | "mind_model_card" => Ok(CharacterFacetType::MindModelCard),
        _ => Err(format!("Invalid character facet type: {}", raw)),
    }
}

fn parse_location_level(
    raw: &str,
) -> Result<crate::agent::models::location::LocationLevel, String> {
    use crate::agent::models::location::LocationLevel;
    match raw {
        "WorldRoot" => Ok(LocationLevel::WorldRoot),
        "Realm" => Ok(LocationLevel::Realm),
        "Continent" => Ok(LocationLevel::Continent),
        "NaturalRegion" => Ok(LocationLevel::NaturalRegion),
        "Polity" => Ok(LocationLevel::Polity),
        "MajorRegion" => Ok(LocationLevel::MajorRegion),
        "LocalRegion" => Ok(LocationLevel::LocalRegion),
        "Settlement" => Ok(LocationLevel::Settlement),
        "DistrictOrSite" => Ok(LocationLevel::DistrictOrSite),
        "RoomOrSubsite" => Ok(LocationLevel::RoomOrSubsite),
        _ => Err(format!("Invalid location level: {}", raw)),
    }
}

fn parse_location_status(
    raw: &str,
) -> Result<crate::agent::models::location::LocationStatus, String> {
    use crate::agent::models::location::LocationStatus;
    match raw {
        "Active" | "active" => Ok(LocationStatus::Active),
        "Deprecated" | "deprecated" => Ok(LocationStatus::Deprecated),
        "PendingConfirmation" | "pending_confirmation" => Ok(LocationStatus::PendingConfirmation),
        _ => Err(format!("Invalid location status: {}", raw)),
    }
}

fn parse_optional_time_anchor_value(value: Option<Value>) -> Result<Option<TimeAnchor>, String> {
    value
        .map(|inner| {
            serde_json::from_value(inner).map_err(|e| format!("Invalid TimeAnchor payload: {}", e))
        })
        .transpose()
}

fn parse_optional_datetime_string(value: Option<String>) -> Result<Option<DateTime<Utc>>, String> {
    value.map(|raw| parse_rfc3339_timestamp(&raw)).transpose()
}

fn required_string(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<String, String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("Location payload missing string field '{}'", key))
}

fn optional_string(object: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    object.get(key).and_then(Value::as_str).map(str::to_string)
}

fn nullable_string_field(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<String>, String> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(format!("Location payload field '{}' must be string or null", key)),
    }
}

fn allocate_world_id(name: &str, existing: &HashSet<String>) -> Result<String, String> {
    let base = slugify_world_name(name);
    let candidate = if base.is_empty() {
        "world".to_string()
    } else {
        format!("world_{}", base)
    };
    validate_path_component(&candidate)
        .map_err(|e| format!("Generated invalid world_id '{}': {}", candidate, e))?;
    if !existing.contains(&candidate) {
        return Ok(candidate);
    }

    for index in 2..10_000 {
        let next = format!("{}_{}", candidate, index);
        validate_path_component(&next)
            .map_err(|e| format!("Generated invalid world_id '{}': {}", next, e))?;
        if !existing.contains(&next) {
            return Ok(next);
        }
    }

    Err("Failed to allocate a unique world_id".to_string())
}

fn slugify_world_name(name: &str) -> String {
    let mut output = String::new();
    let mut last_was_sep = false;

    for ch in name.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            Some(ch.to_ascii_lowercase())
        } else if ch.is_ascii_whitespace() || matches!(ch, '-' | '_') {
            Some('_')
        } else if !ch.is_ascii() {
            Some('_')
        } else {
            None
        };

        match mapped {
            Some('_') => {
                if !last_was_sep && !output.is_empty() {
                    output.push('_');
                    last_was_sep = true;
                }
            }
            Some(value) => {
                output.push(value);
                last_was_sep = false;
            }
            None => {}
        }
    }

    output.trim_matches('_').to_string()
}
#[cfg(test)]
mod tests {
    use super::{allocate_world_id, slugify_world_name};
    use crate::storage::paths::validate_path_component;
    use std::collections::HashSet;

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

    #[test]
    fn slugifies_world_name_into_safe_id_suffix() {
        assert_eq!(slugify_world_name("My First World"), "my_first_world");
        assert_eq!(slugify_world_name("仙侠 测试"), "");
        assert_eq!(slugify_world_name("world!!!demo"), "worlddemo");
    }

    #[test]
    fn allocates_incremental_world_ids_when_collision_exists() {
        let existing = HashSet::from([
            "world_demo".to_string(),
            "world_demo_2".to_string(),
        ]);
        let world_id = allocate_world_id("Demo", &existing).expect("world id");
        assert_eq!(world_id, "world_demo_3");
    }
}
