//! Configuration validator

use serde_json::Value;

/// Validate configuration against schema
pub fn validate_config(_config: &Value) -> Result<(), String> {
    // TODO: Implement configuration validation
    Ok(())
}
