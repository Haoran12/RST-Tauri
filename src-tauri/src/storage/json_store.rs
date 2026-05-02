//! JSON file storage for ST mode

use std::fs;
use std::path::PathBuf;

use crate::storage::paths::safe_join;

/// JSON store for ST mode resources
pub struct JsonStore {
    base_path: PathBuf,
}

impl JsonStore {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    /// Read JSON file
    pub fn read(&self, relative_path: &str) -> Result<serde_json::Value, String> {
        let path = safe_join(&self.base_path, relative_path)?;
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {}", relative_path, e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse JSON from {}: {}", relative_path, e))
    }

    /// Write JSON file
    pub fn write(&self, relative_path: &str, value: &serde_json::Value) -> Result<(), String> {
        let path = safe_join(&self.base_path, relative_path)?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        let content = serde_json::to_string_pretty(value)
            .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

        fs::write(&path, content)
            .map_err(|e| format!("Failed to write {}: {}", relative_path, e))
    }

    /// Delete JSON file
    pub fn delete(&self, relative_path: &str) -> Result<(), String> {
        let path = safe_join(&self.base_path, relative_path)?;
        fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete {}: {}", relative_path, e))
    }

    /// List files in directory
    pub fn list(&self, relative_path: &str) -> Result<Vec<String>, String> {
        let path = safe_join(&self.base_path, relative_path)?;

        if !path.exists() {
            return Ok(Vec::new());
        }

        let entries = fs::read_dir(&path)
            .map_err(|e| format!("Failed to list {}: {}", relative_path, e))?;

        let mut files = Vec::new();
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(name) = entry.file_name().to_str() {
                    files.push(name.to_string());
                }
            }
        }

        Ok(files)
    }
}
