//! Filesystem watcher with debouncing and change classification.
//!
//! Implements the watching half of `docs/design/07-runtime.md` section 11:
//! it watches the working tree and `.git`, debounces bursts of raw events, and
//! emits high-level [`WatchEvent`]s. Noisy paths (`.git` internals other than
//! `HEAD`/refs, and build directories like `target/`) are suppressed so a
//! rebuild does not flood the app with events.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

/// A debounced, classified change to the repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEvent {
    /// One or more working-tree files changed (invalidate their diff/blame,
    /// refresh status).
    WorkingTree(Vec<PathBuf>),
    /// HEAD or a ref moved (invalidate HEAD-scoped caches).
    Head,
}

/// An error starting the watcher.
#[derive(Debug)]
pub struct WatchError(pub String);

impl std::fmt::Display for WatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "watch error: {}", self.0)
    }
}

impl std::error::Error for WatchError {}

/// Classification of a single raw path event.
#[derive(Debug, PartialEq, Eq)]
enum Change {
    WorkingTree(PathBuf),
    Head,
    Ignored,
}

/// A running repository watcher. Dropping it stops watching and joins the
/// debounce thread.
pub struct RepoWatcher {
    watcher: Option<RecommendedWatcher>,
    handle: Option<JoinHandle<()>>,
}

impl RepoWatcher {
    /// Starts watching `repo_root` recursively, coalescing events that arrive
    /// within `debounce` of each other, and returns the event receiver.
    ///
    /// # Errors
    /// Returns [`WatchError`] if the platform watcher cannot be created or the
    /// path cannot be watched.
    pub fn start(
        repo_root: &Path,
        debounce: Duration,
    ) -> Result<(Self, Receiver<WatchEvent>), WatchError> {
        // Canonicalize so event paths (which platform backends report in
        // canonical form, e.g. macOS /private/var symlinks) share our prefix.
        let root = std::fs::canonicalize(repo_root).map_err(|e| WatchError(e.to_string()))?;
        let (raw_tx, raw_rx) = mpsc::channel::<Vec<PathBuf>>();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                let _ = raw_tx.send(event.paths);
            }
        })
        .map_err(|e| WatchError(e.to_string()))?;

        watcher
            .watch(&root, RecursiveMode::Recursive)
            .map_err(|e| WatchError(e.to_string()))?;

        let (event_tx, event_rx) = mpsc::channel::<WatchEvent>();
        let handle = std::thread::Builder::new()
            .name("gitbolt-watcher".into())
            .spawn(move || debounce_loop(&root, debounce, &raw_rx, &event_tx))
            .map_err(|e| WatchError(e.to_string()))?;

        Ok((
            Self {
                watcher: Some(watcher),
                handle: Some(handle),
            },
            event_rx,
        ))
    }
}

impl Drop for RepoWatcher {
    fn drop(&mut self) {
        // Dropping the notify watcher stops events and drops the raw sender,
        // which disconnects the debounce loop so the thread can exit.
        self.watcher.take();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Reads raw path batches, coalesces them within the debounce window, and emits
/// classified [`WatchEvent`]s until the raw channel disconnects.
fn debounce_loop(
    root: &Path,
    debounce: Duration,
    raw_rx: &Receiver<Vec<PathBuf>>,
    event_tx: &Sender<WatchEvent>,
) {
    loop {
        let Ok(first) = raw_rx.recv() else {
            return;
        };
        let mut paths: HashSet<PathBuf> = HashSet::new();
        let mut head = false;
        ingest(root, first, &mut paths, &mut head);

        loop {
            match raw_rx.recv_timeout(debounce) {
                Ok(batch) => ingest(root, batch, &mut paths, &mut head),
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => {
                    flush(paths, head, event_tx);
                    return;
                }
            }
        }
        flush(paths, head, event_tx);
    }
}

/// Emits the accumulated events (HEAD first, then working-tree paths).
fn flush(paths: HashSet<PathBuf>, head: bool, event_tx: &Sender<WatchEvent>) {
    if head {
        let _ = event_tx.send(WatchEvent::Head);
    }
    if !paths.is_empty() {
        let _ = event_tx.send(WatchEvent::WorkingTree(paths.into_iter().collect()));
    }
}

/// Classifies each raw path and folds it into the accumulator.
fn ingest(root: &Path, batch: Vec<PathBuf>, paths: &mut HashSet<PathBuf>, head: &mut bool) {
    for path in batch {
        match classify(root, &path) {
            Change::WorkingTree(p) => {
                paths.insert(p);
            }
            Change::Head => *head = true,
            Change::Ignored => {}
        }
    }
}

/// Classifies a single filesystem path relative to the repository root.
fn classify(root: &Path, path: &Path) -> Change {
    let Ok(rel) = path.strip_prefix(root) else {
        return Change::Ignored;
    };
    let mut comps = rel.components().map(std::path::Component::as_os_str);
    match comps.next() {
        Some(first) if first == ".git" => {
            let sub: PathBuf = comps.collect();
            if sub == Path::new("HEAD")
                || sub == Path::new("packed-refs")
                || sub.starts_with("refs")
            {
                Change::Head
            } else {
                Change::Ignored
            }
        }
        // Suppress noisy build/dependency directories anywhere in the path.
        _ if rel
            .components()
            .any(|c| matches!(c.as_os_str().to_str(), Some("target" | "node_modules"))) =>
        {
            Change::Ignored
        }
        Some(_) => Change::WorkingTree(path.to_path_buf()),
        None => Change::Ignored,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_distinguishes_working_tree_head_and_noise() {
        let root = Path::new("/repo");
        assert_eq!(
            classify(root, Path::new("/repo/src/main.rs")),
            Change::WorkingTree(PathBuf::from("/repo/src/main.rs"))
        );
        assert_eq!(classify(root, Path::new("/repo/.git/HEAD")), Change::Head);
        assert_eq!(
            classify(root, Path::new("/repo/.git/refs/heads/main")),
            Change::Head
        );
        assert_eq!(
            classify(root, Path::new("/repo/.git/objects/ab/cdef")),
            Change::Ignored
        );
        assert_eq!(
            classify(root, Path::new("/repo/target/debug/build.rs")),
            Change::Ignored
        );
        assert_eq!(classify(root, Path::new("/other/x")), Change::Ignored);
    }
}

#[cfg(test)]
mod integration {
    use super::*;
    use crate::app::model::{DiffContent, DiffHunk, DiffTarget, Oid};
    use crate::cache::{CacheKey, RepoCaches};
    use crate::git::fixture::TempRepo;
    use std::sync::Arc;
    use std::time::Instant;

    fn diff_for(path: &Path) -> DiffContent {
        DiffContent {
            target: DiffTarget {
                path: path.to_path_buf(),
                staged: false,
            },
            hunks: Arc::from([] as [DiffHunk; 0]),
        }
    }

    /// Waits (up to `budget`) for a [`WatchEvent`] of the requested kind,
    /// skipping events of the other kind.
    fn recv_kind(
        rx: &Receiver<WatchEvent>,
        want_head: bool,
        budget: Duration,
    ) -> Option<WatchEvent> {
        let deadline = Instant::now() + budget;
        while Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(ev) => {
                    if matches!(ev, WatchEvent::Head) == want_head {
                        return Some(ev);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return None,
            }
        }
        None
    }

    #[test]
    fn file_change_invalidates_only_that_paths_cache() {
        let repo = TempRepo::init();
        repo.write("file.txt", "one\n");
        repo.write("other.txt", "keep\n");
        repo.stage("file.txt");
        repo.stage("other.txt");
        repo.commit("init");

        let root = std::fs::canonicalize(repo.path()).expect("canonicalize");
        let file = root.join("file.txt");
        let other = root.join("other.txt");
        let head = Oid("h".into());

        let mut caches = RepoCaches::new();
        caches
            .diff
            .insert(CacheKey::new(head.clone(), file.clone()), diff_for(&file));
        caches
            .diff
            .insert(CacheKey::new(head.clone(), other.clone()), diff_for(&other));
        caches.history.set(head.clone(), vec![]);

        let (_watcher, rx) =
            RepoWatcher::start(repo.path(), Duration::from_millis(80)).expect("start watcher");
        std::thread::sleep(Duration::from_millis(200));

        repo.write("file.txt", "one\ntwo\n");

        let ev = recv_kind(&rx, false, Duration::from_secs(10)).expect("working-tree event");
        let WatchEvent::WorkingTree(paths) = ev else {
            panic!("expected WorkingTree event");
        };
        assert!(
            paths.iter().any(|p| p == &file),
            "expected {file:?} in {paths:?}"
        );
        caches.on_working_tree_change(paths);

        assert!(caches
            .diff
            .get(&CacheKey::new(head.clone(), file))
            .is_none());
        assert!(caches
            .diff
            .get(&CacheKey::new(head.clone(), other))
            .is_some());
        // A working-tree change must not disturb HEAD-scoped caches.
        assert!(caches.history.get(&head).is_some());
    }

    #[test]
    fn commit_emits_head_event() {
        let repo = TempRepo::init();
        repo.write("a.txt", "x\n");
        repo.stage("a.txt");
        repo.commit("init");

        let head = Oid("h".into());
        let mut caches = RepoCaches::new();
        caches.history.set(head.clone(), vec![]);

        let (_watcher, rx) =
            RepoWatcher::start(repo.path(), Duration::from_millis(80)).expect("start watcher");
        std::thread::sleep(Duration::from_millis(200));

        repo.write("a.txt", "x\ny\n");
        repo.stage("a.txt");
        repo.commit("second");

        let ev = recv_kind(&rx, true, Duration::from_secs(10)).expect("head event");
        assert_eq!(ev, WatchEvent::Head);
        caches.on_head_change();
        assert!(caches.history.is_empty());
    }
}
