//! Shared domain value types used across application state, events, commands,
//! and messages.
//!
//! Git-derived data is kept separate from UI state and large payloads are held
//! behind `Arc` (see `docs/design/05-architecture.md` section 8).

use std::path::PathBuf;
use std::sync::Arc;

/// Monotonic generation counter used to discard stale asynchronous results.
///
/// It is bumped whenever the repository context that in-flight work depends on
/// changes (repository open, HEAD move after checkout/commit). Any
/// [`crate::app::message::AppMessage`] carrying an older generation is dropped
/// by the reducer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Generation(pub u64);

impl Generation {
    /// Returns the next generation.
    #[must_use]
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// A Git object id (commit/tree/blob), stored as its hex string for now.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Oid(pub String);

/// The kind of change reported for a file in the working tree or index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Untracked,
    Conflicted,
}

/// A single changed file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub path: PathBuf,
    pub kind: ChangeKind,
}

impl FileChange {
    /// Convenience constructor.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>, kind: ChangeKind) -> Self {
        Self {
            path: path.into(),
            kind,
        }
    }
}

/// Loading state for lazily-fetched, potentially large Git data.
///
/// Existing content is never replaced by a bare "loading" placeholder in the
/// UI (see `docs/design/02-tech-and-performance.md` section 3); this type lets
/// views keep showing `Ready` data while a refresh is in flight.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Loadable<T> {
    #[default]
    Idle,
    Loading,
    Ready(T),
    Failed(String),
}

impl<T> Loadable<T> {
    /// Returns the value when in the `Ready` state.
    pub fn ready(&self) -> Option<&T> {
        match self {
            Self::Ready(value) => Some(value),
            _ => None,
        }
    }

    /// Returns true when a fetch is in progress.
    #[must_use]
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }
}

/// How a diff should be presented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiffView {
    #[default]
    Unified,
    Split,
}

/// Identifies which file/diff is being viewed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffTarget {
    pub path: PathBuf,
    /// True when viewing the staged (index vs HEAD) diff, false for worktree.
    pub staged: bool,
}

/// A single line within a diff hunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    /// ` `, `+`, or `-`.
    pub origin: char,
    pub content: String,
    /// Index into the unified-diff body (after `+++`), for line-stage patches.
    pub body_index: usize,
    /// 1-based line number on the old (HEAD / index) side, when applicable.
    pub old_line: Option<u32>,
    /// Commit that last authored `old_line` at HEAD (Change Origin, issue #31).
    pub change_origin: Option<CommitSummary>,
}

/// A diff hunk with its `@@` header and lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

/// Rendered diff content for a single file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffContent {
    pub target: DiffTarget,
    pub hunks: Arc<[DiffHunk]>,
    /// Optional banner (binary file, truncated, empty).
    pub notice: Option<String>,
}

/// A commit as shown in the history list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitSummary {
    pub oid: Oid,
    pub summary: String,
    pub author: String,
    /// Unix timestamp (seconds).
    pub timestamp: i64,
}

/// Health of a branch relative to its upstream (GitBolt Branch Health).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchHealth {
    Synced,
    Ahead,
    Behind,
    Diverged,
    Stale,
    /// No upstream configured.
    Local,
}

/// A branch and its tracking information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchInfo {
    pub name: String,
    pub upstream: Option<String>,
    pub health: BranchHealth,
    pub ahead: u32,
    pub behind: u32,
    pub last_commit: Option<CommitSummary>,
}

/// A linked or primary worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub is_primary: bool,
}

/// Current HEAD summary.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HeadInfo {
    pub branch: Option<String>,
    pub oid: Option<Oid>,
    pub detached: bool,
}

/// Top-level views selectable from the navigation pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Changes,
    History,
    Branches,
    Worktrees,
    Stashes,
}

/// Which of the three panes currently has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pane {
    #[default]
    Navigation,
    Content,
    Context,
}
