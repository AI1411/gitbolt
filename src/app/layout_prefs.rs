//! Persist UI layout preferences (issue #90).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Saved shell layout widths.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayoutPrefs {
    pub nav_width: f64,
    pub context_width: f64,
}

impl Default for LayoutPrefs {
    fn default() -> Self {
        Self {
            nav_width: 200.0,
            context_width: 280.0,
        }
    }
}

fn prefs_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gitbolt")
        .join("layout.json")
}

/// Loads layout prefs or defaults.
#[must_use]
pub fn load_layout_prefs() -> LayoutPrefs {
    let Ok(bytes) = std::fs::read(prefs_path()) else {
        return LayoutPrefs::default();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

/// Persists layout prefs.
pub fn save_layout_prefs(prefs: &LayoutPrefs) {
    let path = prefs_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_vec_pretty(prefs) {
        let _ = std::fs::write(path, json);
    }
}

/// Splits a path into parent (dim) + file name (bright) for list display.
#[must_use]
pub fn split_path_display(path: &std::path::Path) -> (String, String) {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |s| s.to_string_lossy().into_owned(),
    );
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| format!("{}/", p.display()))
        .unwrap_or_default();
    (parent, name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn split_path_display_separates_parent() {
        let (parent, name) = split_path_display(Path::new("src/ui/changes.rs"));
        assert_eq!(name, "changes.rs");
        assert_eq!(parent, "src/ui/");
    }
}
