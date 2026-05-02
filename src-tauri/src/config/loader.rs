//! Configuration loader

use std::path::PathBuf;

/// Load configuration from file
pub fn load_config(_path: &PathBuf) -> Result<serde_json::Value, String> {
    // TODO: Implement configuration loading
    Ok(serde_json::json!({}))
}
