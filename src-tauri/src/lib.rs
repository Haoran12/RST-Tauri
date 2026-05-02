//! RST-Tauri Backend
//!
//! Ran's SmartTavern - Dual-mode AI chat application

pub mod api;
pub mod commands;
pub mod config;
pub mod error;
pub mod logging;
pub mod st;
pub mod storage;

use std::sync::Arc;
use tokio::sync::RwLock;

use storage::paths::app_data_root;
use storage::sqlite_store::SqliteStore;

/// Application state
pub struct AppState {
    pub sqlite_store: Arc<RwLock<Option<SqliteStore>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            sqlite_store: Arc::new(RwLock::new(None)),
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
                "presets/samplers",
                "presets/instruct",
                "presets/context",
                "presets/sysprompt",
                "presets/reasoning",
                "presets/prompts",
                "chats",
                "characters",
                "settings",
                "api_configs",
                "worlds",
            ];

            for subdir in subdirs {
                std::fs::create_dir_all(data_dir.join(subdir))?;
            }

            // Initialize global runtime logs.
            let db_path = data_dir.join("logs/app_logs.sqlite");
            let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

            let state_clone = state.clone();
            tauri::async_runtime::spawn(async move {
                let sqlite_store = SqliteStore::new(&db_url).await;
                match sqlite_store {
                    Ok(store) => {
                        if let Err(e) = store.init_logging_schema().await {
                            tracing::error!("Failed to initialize logging database schema: {}", e);
                        }
                        *state_clone.sqlite_store.write().await = Some(store);
                    }
                    Err(e) => {
                        tracing::error!("Failed to initialize SQLite database: {}", e);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            // API Config commands
            commands::list_api_configs,
            commands::get_api_config,
            commands::save_api_config,
            commands::delete_api_config,
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
            // Logging commands
            commands::get_llm_logs,
            commands::get_event_logs,
            commands::run_retention_check,
            // Chat commands
            commands::send_chat_message,
            commands::send_structured_chat_message,
            // Runtime assembly commands
            commands::get_global_state,
            commands::save_global_state,
            commands::set_active_api_config,
            commands::set_active_presets,
            commands::load_sampler_preset,
            commands::load_instruct_template,
            commands::load_context_template,
            commands::load_system_prompt,
            commands::load_reasoning_template,
            commands::load_prompt_preset,
            commands::assemble_st_request,
            commands::run_world_info_injection,
            commands::map_request_to_provider,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
