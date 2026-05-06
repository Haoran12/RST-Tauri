//! RST-Tauri Backend
//!
//! Ran's SmartTavern - Dual-mode AI chat application

pub mod agent;
pub mod api;
pub mod commands;
pub mod config;
pub mod error;
pub mod logging;
pub mod st;
pub mod storage;
pub mod text_format;

use std::sync::Arc;
use tokio::sync::RwLock;

use config::llm_contracts::{
    load_llm_api_contracts_snapshot, load_llm_api_contracts_snapshot_from_str,
    LlmApiContractsSnapshot, ProviderContractCache,
};
use storage::paths::app_data_root;
use storage::sqlite_store::SqliteStore;
use tauri::webview::WebviewWindowBuilder;

/// Application state
pub struct AppState {
    pub sqlite_store: Arc<RwLock<Option<SqliteStore>>>,
    pub llm_api_contracts: Arc<RwLock<Option<Arc<LlmApiContractsSnapshot>>>>,
    pub provider_contract_cache: Arc<ProviderContractCache>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            sqlite_store: Arc::new(RwLock::new(None)),
            llm_api_contracts: Arc::new(RwLock::new(None)),
            provider_contract_cache: Arc::new(ProviderContractCache::new()),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = Arc::new(AppState::new());

    tauri::Builder::default()
        .manage(state.clone())
        .setup(move |app| {
            // Initialize data directory
            let app_handle = app.handle();
            let data_dir = app_data_root(app_handle)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            std::fs::create_dir_all(&data_dir)?;

            // Initialize subdirectories
            let subdirs = [
                "logs",
                "logs/archives",
                "lores",
                "presets",
                "chats",
                "characters",
                "settings",
                "api_configs",
                "chat_attachments",
                "worlds",
                "webview",
            ];

            for subdir in subdirs {
                std::fs::create_dir_all(data_dir.join(subdir))?;
            }

            // Load bundled LLM API contracts once at startup.
            let contracts_path = std::path::PathBuf::from("config").join("llm_api_contracts.json");
            let contracts_result = load_llm_api_contracts_snapshot(&contracts_path).or_else(|e| {
                tracing::warn!(
                    "Failed to load runtime llm_api_contracts.json from {}: {}; using embedded snapshot",
                    contracts_path.display(),
                    e
                );
                load_llm_api_contracts_snapshot_from_str(include_str!(
                    "../../config/llm_api_contracts.json"
                ))
            });

            match contracts_result {
                Ok(snapshot) => {
                    tracing::info!(
                        "Loaded llm_api_contracts.json snapshot_id={} schema_version={} hash={}",
                        snapshot.llm_api_contracts_snapshot_id,
                        snapshot.schema_version,
                        snapshot.contracts_hash
                    );
                    let state_clone = state.clone();
                    tauri::async_runtime::block_on(async move {
                        *state_clone.llm_api_contracts.write().await = Some(Arc::new(snapshot));
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to load llm_api_contracts.json: {}", e);
                }
            }

            // Initialize global runtime logs.
            let db_path = data_dir.join("logs/app_logs.sqlite");
            let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

            let sqlite_store = tauri::async_runtime::block_on(SqliteStore::new(&db_url))
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            tauri::async_runtime::block_on(async {
                sqlite_store
                    .init_logging_schema()
                    .await
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                *state.sqlite_store.write().await = Some(sqlite_store);
                Ok::<(), std::io::Error>(())
            })?;

            let main_window_config = app.config().app.windows.first().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::Other, "Missing main window config")
            })?;
            WebviewWindowBuilder::from_config(app_handle, main_window_config)?
                .data_directory(data_dir.join("webview"))
                .build()?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            // API Config commands
            commands::list_api_configs,
            commands::get_api_config,
            commands::save_api_config,
            commands::delete_api_config,
            commands::list_models,
            // Character commands
            commands::list_characters,
            commands::get_character,
            commands::save_character,
            commands::delete_character,
            commands::import_character_from_png,
            commands::import_character_from_json,
            commands::export_character_as_png,
            commands::export_character_as_json,
            commands::import_embedded_worldbook,
            commands::update_character_avatar,
            commands::get_character_avatar,
            // Worldbook commands
            commands::list_worldbooks,
            commands::get_worldbook,
            commands::create_worldbook,
            commands::save_worldbook,
            commands::delete_worldbook,
            commands::update_worldbook_meta,
            // WorldInfo Entry commands
            commands::create_worldbook_entry,
            commands::update_worldbook_entry,
            commands::delete_worldbook_entry,
            commands::reorder_worldbook_entries,
            commands::import_worldbook,
            commands::export_worldbook,
            // Chat session commands
            commands::list_chat_sessions,
            commands::get_chat_session,
            commands::save_chat_session,
            commands::delete_chat_session,
            commands::save_chat_attachment,
            commands::get_chat_attachment,
            commands::get_chat_attachment_blob,
            commands::list_attachment_upload_cache,
            commands::clear_attachment_upload_cache,
            // Logging commands
            commands::get_llm_logs,
            commands::get_event_logs,
            commands::run_retention_check,
            commands::query_log_records,
            commands::get_log_record_detail,
            commands::get_stream_chunks,
            commands::get_trace_detail,
            commands::get_log_storage_summary,
            commands::export_logs,
            commands::run_log_retention_now,
            commands::preview_log_cleanup,
            commands::confirm_log_cleanup,
            commands::set_log_protection,
            commands::get_log_protection,
            commands::validate_structured_text,
            commands::format_structured_text,
            commands::log_frontend_event,
            // Runtime assembly commands
            commands::get_global_state,
            commands::save_global_state,
            commands::set_active_api_config,
            commands::set_active_preset,
            commands::load_sampler_preset,
            commands::load_instruct_template,
            commands::load_context_template,
            commands::load_system_prompt,
            commands::load_reasoning_template,
            commands::load_prompt_preset,
            commands::list_presets,
            commands::save_preset,
            commands::delete_preset,
            commands::load_preset,
            commands::assemble_st_request,
            commands::send_assembled_st_chat_message,
            commands::run_world_info_injection,
            commands::map_request_to_provider,
            // Agent session commands
            commands::list_agent_worlds,
            commands::create_agent_world,
            commands::create_agent_session,
            commands::list_agent_sessions,
            commands::get_agent_session,
            commands::list_agent_session_turns,
            commands::update_agent_session_turn,
            commands::delete_agent_session_turn,
            commands::process_agent_turn,
            commands::update_session_player_mode,
            commands::get_world_mainline_cursor,
            commands::advance_world_mainline,
            commands::list_world_characters,
            commands::create_time_anchor,
            commands::compare_time_anchors,
            // Past timeline commands
            commands::get_truth_guidance,
            commands::get_open_detail_slots,
            commands::fill_detail_slot,
            commands::get_provisional_candidates,
            commands::promote_provisional_candidates,
            commands::mark_provisional_non_canon,
            // Canon status commands
            commands::evaluate_canon_eligibility,
            commands::promote_to_canon,
            commands::get_session_conflicts,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
