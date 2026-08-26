//! Persistent recent-repositories list.
//!
//! See issue #9 and `docs/design/03-features.md` (Repository / Recent).

use std::path::{Path, PathBuf};

const MAX_RECENT: usize = 10;

/// Moves `path` to the front of `recent`, deduplicating and capping at 10.
pub fn push_recent(recent: &mut Vec<PathBuf>, path: PathBuf) {
    recent.retain(|p| p != &path);
    recent.insert(0, path);
    recent.truncate(MAX_RECENT);
}

/// Path of the Recent JSON store (`$XDG_CONFIG_HOME/gitbolt/recent.json`).
#[must_use]
pub fn recent_store_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gitbolt")
        .join("recent.json")
}

/// Loads recent paths from the default store. Missing/invalid → empty.
#[must_use]
pub fn load_recent() -> Vec<PathBuf> {
    load_recent_from(&recent_store_path())
}

/// Loads recent paths from `path`. Missing/invalid → empty.
#[must_use]
pub fn load_recent_from(path: &Path) -> Vec<PathBuf> {
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    serde_json::from_slice::<Vec<PathBuf>>(&bytes).unwrap_or_default()
}

/// Persists recent paths to the default store.
///
/// # Errors
/// Returns an I/O error when the directory cannot be created or the file
/// cannot be written.
pub fn save_recent(paths: &[PathBuf]) -> std::io::Result<()> {
    save_recent_to(&recent_store_path(), paths)
}

/// Persists recent paths to `path`.
///
/// # Errors
/// Returns an I/O error when the directory cannot be created or the file
/// cannot be written.
pub fn save_recent_to(path: &Path, paths: &[PathBuf]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec_pretty(paths)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_recent_dedupes_and_caps_at_ten() {
        let mut recent = Vec::new();
        for i in 0..12 {
            push_recent(&mut recent, PathBuf::from(format!("/r{i}")));
        }
        assert_eq!(recent.len(), 10);
        assert_eq!(recent[0], PathBuf::from("/r11"));
        push_recent(&mut recent, PathBuf::from("/r5"));
        assert_eq!(recent[0], PathBuf::from("/r5"));
        assert_eq!(recent.iter().filter(|p| *p == Path::new("/r5")).count(), 1);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = dir.path().join("recent.json");
        let paths = vec![PathBuf::from("/a"), PathBuf::from("/b")];
        save_recent_to(&store, &paths).expect("save");
        let loaded = load_recent_from(&store);
        assert_eq!(loaded, paths);
    }

    #[test]
    fn load_missing_file_is_empty() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = dir.path().join("missing.json");
        assert!(load_recent_from(&store).is_empty());
    }
}
