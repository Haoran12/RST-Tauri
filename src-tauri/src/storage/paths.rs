//! Application data path helpers.

use std::path::{Component, Path, PathBuf};

use tauri::AppHandle;

/// Return the portable application data root.
///
/// The default root is `./data` in development and `<exe-dir>/data` in a
/// bundled app. `RST_DATA_DIR` is reserved for an explicit user override.
pub fn app_data_root(_app: &AppHandle) -> Result<PathBuf, String> {
    if let Ok(explicit) = std::env::var("RST_DATA_DIR") {
        if !explicit.trim().is_empty() {
            return Ok(PathBuf::from(explicit));
        }
    }

    if cfg!(debug_assertions) {
        let cwd = std::env::current_dir()
            .map_err(|e| format!("Failed to get current directory: {}", e))?;
        let root = if cwd.file_name().and_then(|n| n.to_str()) == Some("src-tauri") {
            cwd.parent().map(Path::to_path_buf).unwrap_or(cwd)
        } else {
            cwd
        };
        Ok(root.join("data"))
    } else {
        let exe = std::env::current_exe()
            .map_err(|e| format!("Failed to get executable path: {}", e))?;
        let exe_dir = exe
            .parent()
            .ok_or_else(|| "Failed to resolve executable directory".to_string())?;
        Ok(exe_dir.join("data"))
    }
}

/// Join a trusted base path with an application-relative path.
///
/// This rejects absolute paths, parent traversal, Windows prefixes, and empty
/// segments before the filesystem is touched.
pub fn safe_join(base: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let relative = Path::new(relative_path);
    if relative_path.trim().is_empty() || relative.is_absolute() {
        return Err("Path must be a non-empty relative path".to_string());
    }

    let mut joined = PathBuf::from(base);
    for component in relative.components() {
        match component {
            Component::Normal(part) => {
                let part = part
                    .to_str()
                    .ok_or_else(|| "Path contains non-UTF-8 component".to_string())?;
                validate_path_component(part)?;
                joined.push(part);
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err("Path traversal is not allowed".to_string());
            }
        }
    }

    Ok(joined)
}

/// Validate one filename or directory-name component.
pub fn validate_path_component(component: &str) -> Result<(), String> {
    if component.is_empty() || component == "." || component == ".." {
        return Err("Invalid empty or dot path component".to_string());
    }

    let invalid = ['<', '>', ':', '"', '|', '?', '*', '\0'];
    if component.chars().any(|c| invalid.contains(&c) || c.is_control()) {
        return Err(format!("Invalid path component: {}", component));
    }

    if component.ends_with(' ') || component.ends_with('.') {
        return Err(format!("Invalid trailing character in path component: {}", component));
    }

    let stem = component
        .split('.')
        .next()
        .unwrap_or(component)
        .to_ascii_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
        "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
        "LPT9",
    ];
    if reserved.contains(&stem.as_str()) {
        return Err(format!("Reserved path component: {}", component));
    }

    Ok(())
}

/// Build a safe PNG filename from untrusted import names.
pub fn safe_png_filename_from_import(filename: &str, fallback_id: &str) -> String {
    let candidate = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(fallback_id);
    let sanitized: String = candidate
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ' ') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches([' ', '.', '_']);
    let stem = if trimmed.is_empty() { fallback_id } else { trimmed };
    format!("{}.png", stem)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_join_rejects_parent_traversal() {
        let base = PathBuf::from("data");
        assert!(safe_join(&base, "../outside.json").is_err());
        assert!(safe_join(&base, "characters/../../outside.json").is_err());
    }

    #[test]
    fn safe_join_rejects_absolute_paths() {
        let base = PathBuf::from("data");
        assert!(safe_join(&base, "C:\\temp\\secret.json").is_err());
        assert!(safe_join(&base, "/tmp/secret.json").is_err());
    }

    #[test]
    fn safe_join_allows_expected_resource_paths() {
        let base = PathBuf::from("data");
        let path = safe_join(&base, "characters/abc-123.json").unwrap();
        assert_eq!(path, PathBuf::from("data").join("characters").join("abc-123.json"));
    }
}
