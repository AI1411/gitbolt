//! In-memory caches for Git-derived data and their invalidation rules.
//!
//! Implements the caching model from `docs/design/07-runtime.md` section 11:
//!
//! - Diff and Blame caches are keyed by `repository + HEAD OID + file path`
//!   (the repository is implied by the owning [`RepoCaches`] instance).
//! - A working-tree change invalidates only the caches for the changed paths.
//! - A HEAD change invalidates Diff, Blame, History (HEAD), Branch Health, and
//!   Ahead/Behind.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::app::model::{BranchInfo, CommitSummary, DiffContent, Oid};

/// Cache key: HEAD OID + repository-relative (or absolute) file path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub head: Oid,
    pub path: PathBuf,
}

impl CacheKey {
    #[must_use]
    pub fn new(head: Oid, path: impl Into<PathBuf>) -> Self {
        Self {
            head,
            path: path.into(),
        }
    }
}

/// A generic `(HEAD, path)`-keyed cache holding shared values.
#[derive(Debug)]
pub struct Cache<V> {
    entries: HashMap<CacheKey, Arc<V>>,
}

impl<V> Default for Cache<V> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl<V> Cache<V> {
    /// Creates an empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Looks up a cached value.
    #[must_use]
    pub fn get(&self, key: &CacheKey) -> Option<Arc<V>> {
        self.entries.get(key).map(Arc::clone)
    }

    /// Inserts a value and returns the shared handle.
    pub fn insert(&mut self, key: CacheKey, value: V) -> Arc<V> {
        let shared = Arc::new(value);
        self.entries.insert(key, Arc::clone(&shared));
        shared
    }

    /// Invalidates every entry for `path`, regardless of HEAD.
    pub fn invalidate_path(&mut self, path: &Path) {
        self.entries.retain(|key, _| key.path != path);
    }

    /// Clears the entire cache (used on HEAD change, since entries are
    /// HEAD-scoped).
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Number of cached entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A single value scoped to a specific HEAD OID (e.g. history page, branch
/// health snapshot). Reads miss automatically once HEAD moves.
#[derive(Debug)]
pub struct HeadScoped<V> {
    head: Option<Oid>,
    value: Option<Arc<V>>,
}

impl<V> Default for HeadScoped<V> {
    fn default() -> Self {
        Self {
            head: None,
            value: None,
        }
    }
}

impl<V> HeadScoped<V> {
    /// Creates an empty holder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the value if it matches `current` HEAD.
    #[must_use]
    pub fn get(&self, current: &Oid) -> Option<Arc<V>> {
        match (&self.head, &self.value) {
            (Some(head), Some(value)) if head == current => Some(Arc::clone(value)),
            _ => None,
        }
    }

    /// Stores a value for `head`.
    pub fn set(&mut self, head: Oid, value: V) -> Arc<V> {
        let shared = Arc::new(value);
        self.head = Some(head);
        self.value = Some(Arc::clone(&shared));
        shared
    }

    /// Invalidates the stored value.
    pub fn invalidate(&mut self) {
        self.head = None;
        self.value = None;
    }

    /// True when nothing is cached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.value.is_none()
    }
}

/// Value type held by the blame cache (commit info per file, refined later).
pub type BlameValue = Vec<crate::git::CommitInfo>;
/// Ahead/behind counts per branch name.
pub type AheadBehind = Vec<(String, u32, u32)>;

/// All caches for one repository, with the invalidation policy applied.
#[derive(Debug, Default)]
pub struct RepoCaches {
    pub diff: Cache<DiffContent>,
    pub blame: Cache<BlameValue>,
    pub history: HeadScoped<Vec<CommitSummary>>,
    pub branch_health: HeadScoped<Vec<BranchInfo>>,
    pub ahead_behind: HeadScoped<AheadBehind>,
}

impl RepoCaches {
    /// Creates empty caches.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Invalidates only the caches related to the changed working-tree paths.
    pub fn on_working_tree_change<I, P>(&mut self, paths: I)
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        for path in paths {
            let path = path.as_ref();
            self.diff.invalidate_path(path);
            self.blame.invalidate_path(path);
        }
    }

    /// Invalidates everything that depends on HEAD: Diff, Blame, History,
    /// Branch Health, and Ahead/Behind.
    pub fn on_head_change(&mut self) {
        self.diff.clear();
        self.blame.clear();
        self.history.invalidate();
        self.branch_health.invalidate();
        self.ahead_behind.invalidate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn diff_for(path: &str) -> DiffContent {
        DiffContent {
            target: crate::app::model::DiffTarget {
                path: path.into(),
                staged: false,
            },
            hunks: Arc::from([] as [crate::app::model::DiffHunk; 0]),
        }
    }

    #[test]
    fn working_tree_change_invalidates_only_changed_paths() {
        let mut caches = RepoCaches::new();
        let head = Oid("deadbeef".into());
        caches
            .diff
            .insert(CacheKey::new(head.clone(), "a.rs"), diff_for("a.rs"));
        caches
            .diff
            .insert(CacheKey::new(head.clone(), "b.rs"), diff_for("b.rs"));
        assert_eq!(caches.diff.len(), 2);

        caches.on_working_tree_change([PathBuf::from("a.rs")]);

        assert!(caches
            .diff
            .get(&CacheKey::new(head.clone(), "a.rs"))
            .is_none());
        assert!(caches.diff.get(&CacheKey::new(head, "b.rs")).is_some());
        assert_eq!(caches.diff.len(), 1);
    }

    #[test]
    fn head_change_invalidates_head_scoped_caches() {
        let mut caches = RepoCaches::new();
        let head = Oid("h1".into());
        caches
            .diff
            .insert(CacheKey::new(head.clone(), "a.rs"), diff_for("a.rs"));
        caches.history.set(head.clone(), vec![]);
        caches.branch_health.set(head.clone(), vec![]);
        caches.ahead_behind.set(head.clone(), vec![]);
        assert!(!caches.diff.is_empty());
        assert!(caches.history.get(&head).is_some());

        caches.on_head_change();

        assert!(caches.diff.is_empty());
        assert!(caches.history.is_empty());
        assert!(caches.branch_health.is_empty());
        assert!(caches.ahead_behind.is_empty());
    }

    #[test]
    fn head_scoped_misses_after_head_moves() {
        let mut scoped: HeadScoped<u32> = HeadScoped::new();
        scoped.set(Oid("old".into()), 7);
        assert_eq!(scoped.get(&Oid("old".into())).as_deref(), Some(&7));
        assert!(scoped.get(&Oid("new".into())).is_none());
    }
}
