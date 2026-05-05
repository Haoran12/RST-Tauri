//! Tauri commands

pub mod agent_commands;
pub mod chat_commands;
pub mod log_commands;
pub mod runtime_commands;
pub mod st_commands;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::logging::event_logger::AppEventLog;
use crate::logging::llm_logger::LlmCallLog;
use crate::logging::retention::LogRetentionResult;
use crate::text_format::{
    format_request, validate_request, StructuredTextBackendRequest, StructuredTextValidationResult,
};
use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct GreetResponse {
    pub message: String,
}

fn clamp_log_limit(limit: i64) -> i64 {
    limit.clamp(1, 500)
}

#[tauri::command]
pub fn greet(name: &str) -> Result<GreetResponse, String> {
    Ok(GreetResponse {
        message: format!("Hello, {}! Welcome to RST.", name),
    })
}

/// Get recent LLM call logs
#[tauri::command]
pub async fn get_llm_logs(
    state: State<'_, std::sync::Arc<AppState>>,
    limit: i64,
) -> Result<Vec<LlmCallLog>, String> {
    let store_guard = state.sqlite_store.read().await;
    if let Some(store) = store_guard.as_ref() {
        store.llm_logger().get_recent(clamp_log_limit(limit)).await
    } else {
        Err("Database not initialized".to_string())
    }
}

/// Get recent event logs
#[tauri::command]
pub async fn get_event_logs(
    state: State<'_, std::sync::Arc<AppState>>,
    limit: i64,
) -> Result<Vec<AppEventLog>, String> {
    let store_guard = state.sqlite_store.read().await;
    if let Some(store) = store_guard.as_ref() {
        store
            .event_logger()
            .get_recent(clamp_log_limit(limit))
            .await
    } else {
        Err("Database not initialized".to_string())
    }
}

/// Run retention check
#[tauri::command]
pub async fn run_retention_check(
    state: State<'_, std::sync::Arc<AppState>>,
) -> Result<LogRetentionResult, String> {
    let store_guard = state.sqlite_store.read().await;
    if let Some(store) = store_guard.as_ref() {
        store.retention_manager().check_retention().await
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
pub async fn validate_structured_text(
    input: StructuredTextBackendRequest,
) -> Result<StructuredTextValidationResult, String> {
    validate_request(input)
}

#[tauri::command]
pub async fn format_structured_text(
    input: StructuredTextBackendRequest,
) -> Result<StructuredTextValidationResult, String> {
    format_request(input)
}

// Re-export ST commands
pub use st_commands::*;

// Re-export chat commands
pub use chat_commands::*;

// Re-export log commands
pub use log_commands::*;

// Re-export runtime commands
pub use runtime_commands::*;

// Re-export agent commands
pub use agent_commands::*;
