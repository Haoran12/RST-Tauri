//! Configuration loader

use std::path::PathBuf;

/// Load configuration from file
pub fn load_config(path: &PathBuf) -> Result<serde_json::Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read config {}: {}", path.display(), e))?;

    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
    {
        Some(ext) if ext == "yaml" || ext == "yml" => {
            serde_yaml::from_str::<serde_json::Value>(&text)
                .map_err(|e| format!("Failed to parse YAML config {}: {}", path.display(), e))
        }
        _ => serde_json::from_str::<serde_json::Value>(&text)
            .map_err(|e| format!("Failed to parse JSON config {}: {}", path.display(), e)),
    }
}

#[cfg(test)]
mod tests {
    use super::load_config;

    #[test]
    fn loads_json_config_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("config.json");
        std::fs::write(
            &path,
            r#"{ "schema_version": "1", "logs": { "max_size_bytes": 1024 } }"#,
        )
        .expect("write config");

        let config = load_config(&path).expect("load json config");
        assert_eq!(config["schema_version"], "1");
        assert_eq!(config["logs"]["max_size_bytes"], 1024);
    }

    #[test]
    fn loads_yaml_config_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("config.yaml");
        std::fs::write(&path, "schema_version: '1'\nlogs:\n  retention_days: 30\n")
            .expect("write config");

        let config = load_config(&path).expect("load yaml config");
        assert_eq!(config["schema_version"], "1");
        assert_eq!(config["logs"]["retention_days"], 30);
    }
}
