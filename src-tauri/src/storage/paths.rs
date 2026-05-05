//! Application data path helpers.

#[cfg(windows)]
use std::path::Prefix;
use std::path::{Component, Path, PathBuf};

use tauri::AppHandle;

/// Return the portable application data root.
///
/// The default root is `./data` in development and `<exe-dir>/data` in a
/// bundled app. `RST_DATA_DIR` is reserved for an explicit user override.
pub fn app_data_root(_app: &AppHandle) -> Result<PathBuf, String> {
    let app_root = app_install_root()?;

    if let Ok(explicit) = std::env::var("RST_DATA_DIR") {
        let explicit = explicit.trim();
        if !explicit.is_empty() {
            let root = absolutize_from_app_root(&app_root, PathBuf::from(explicit))?;
            ensure_data_root_allowed(&root, &app_root)?;
            return Ok(root);
        }
    }

    let root = app_root.join("data");
    ensure_data_root_allowed(&root, &app_root)?;
    Ok(root)
}

fn app_install_root() -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        let cwd = std::env::current_dir()
            .map_err(|e| format!("Failed to get current directory: {}", e))?;
        Ok(
            if cwd.file_name().and_then(|n| n.to_str()) == Some("src-tauri") {
                cwd.parent().map(Path::to_path_buf).unwrap_or(cwd)
            } else {
                cwd
            },
        )
    } else {
        let exe =
            std::env::current_exe().map_err(|e| format!("Failed to get executable path: {}", e))?;
        let exe_dir = exe
            .parent()
            .ok_or_else(|| "Failed to resolve executable directory".to_string())?;
        Ok(exe_dir.to_path_buf())
    }
}

fn absolutize_from_app_root(app_root: &Path, path: PathBuf) -> Result<PathBuf, String> {
    reject_parent_components(&path)?;
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(app_root.join(path))
    }
}

fn reject_parent_components(path: &Path) -> Result<(), String> {
    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            return Err("Data directory must not contain parent traversal".to_string());
        }
    }
    Ok(())
}

fn ensure_data_root_allowed(data_root: &Path, app_root: &Path) -> Result<(), String> {
    reject_parent_components(data_root)?;

    #[cfg(windows)]
    {
        if is_c_drive_path(data_root) && !data_root.starts_with(app_root) {
            return Err(
                "C: drive data writes are only allowed inside the application install directory"
                    .to_string(),
            );
        }
    }

    Ok(())
}

#[cfg(windows)]
fn is_c_drive_path(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Prefix(prefix) => match prefix.kind() {
            Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => drive.eq_ignore_ascii_case(&b'C'),
            _ => false,
        },
        _ => false,
    })
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

    let invalid = ['<', '>', ':', '"', '|', '?', '*', '/', '\\', '\0'];
    if component
        .chars()
        .any(|c| invalid.contains(&c) || c.is_control())
    {
        return Err(format!("Invalid path component: {}", component));
    }

    if component.ends_with(' ') || component.ends_with('.') {
        return Err(format!(
            "Invalid trailing character in path component: {}",
            component
        ));
    }

    let stem = component
        .split('.')
        .next()
        .unwrap_or(component)
        .to_ascii_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
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
    let stem = if trimmed.is_empty() {
        fallback_id
    } else {
        trimmed
    };
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
        assert_eq!(
            path,
            PathBuf::from("data")
                .join("characters")
                .join("abc-123.json")
        );
    }

    #[cfg(windows)]
    #[test]
    fn c_drive_data_root_must_be_under_app_root() {
        let app_root = PathBuf::from("E:\\RST-Tauri");
        let outside = PathBuf::from("C:\\Users\\Z\\AppData\\Local\\RST-Tauri");
        assert!(ensure_data_root_allowed(&outside, &app_root).is_err());

        let installed_on_c = PathBuf::from("C:\\RST-Tauri");
        let own_data = installed_on_c.join("data");
        assert!(ensure_data_root_allowed(&own_data, &installed_on_c).is_ok());
    }
}
