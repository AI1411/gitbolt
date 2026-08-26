//! Top-level application state and sub-states.
//!
//! See `docs/design/05-architecture.md` section 8. Git-derived data is kept
//! separate from UI state; large collections are held behind `Arc`.

use std::path::PathBuf;
use std::sync::Arc;

use super::model::{
    BranchInfo, CommitSummary, DiffContent, DiffTarget, DiffView, FileChange, Generation, HeadInfo,
    Loadable, Oid, Pane, View, WorktreeInfo,
};

/// Lifecycle of the currently opened repository.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RepositoryStatus {
    #[default]
    NotOpened,
    Opening,
    Ready,
    Error(String),
}

/// Repository identity and open lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RepositoryState {
    pub path: Option<PathBuf>,
    pub status: RepositoryStatus,
    pub head: HeadInfo,
    pub recent: Vec<PathBuf>,
}

/// Working-tree status split by area.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChangesState {
    pub staged: Arc<[FileChange]>,
    pub unstaged: Arc<[FileChange]>,
    pub untracked: Arc<[FileChange]>,
    pub conflicted: Arc<[FileChange]>,
    pub loaded: bool,
}

/// The diff currently shown in the content pane.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiffState {
    pub target: Option<DiffTarget>,
    pub view: DiffView,
    pub content: Loadable<DiffContent>,
}

/// Commit history with lazy paging.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HistoryState {
    pub commits: Vec<CommitSummary>,
    pub has_more: bool,
    pub loading: bool,
}

/// Branch list and current branch.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BranchState {
    pub branches: Arc<[BranchInfo]>,
    pub current: Option<String>,
    pub loaded: bool,
}

/// Worktree list.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorktreeState {
    pub worktrees: Arc<[WorktreeInfo]>,
    pub loaded: bool,
}

/// The user's current selection across panes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SelectionState {
    pub file: Option<PathBuf>,
    pub commit: Option<Oid>,
    pub branch: Option<String>,
    pub focused_pane: Pane,
}

/// Navigation / routing within the single window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationState {
    pub active_view: View,
    pub context_panel_open: bool,
    pub back_stack: Vec<View>,
}

impl Default for NavigationState {
    fn default() -> Self {
        Self {
            active_view: View::default(),
            context_panel_open: true,
            back_stack: Vec::new(),
        }
    }
}

/// Progress reflection for background work.
///
/// Detailed scheduling lives in `crate::task` (issue #6); this only mirrors
/// enough for the UI to show activity.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BackgroundTaskState {
    /// Number of dispatched commands that have not yet reported completion.
    pub inflight: u32,
    pub last_error: Option<String>,
}

/// Presentational / editable UI state not derived from Git.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UiState {
    /// The in-progress commit message. Preserved across failed commits
    /// (see `docs/design/07-runtime.md` section 13).
    pub commit_message: String,
    pub error_banner: Option<String>,
    pub search_query: String,
    pub searching: bool,
}

/// The whole application state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppState {
    /// Current generation; bumped on repository-context changes.
    pub generation: Generation,
    pub repository: RepositoryState,
    pub changes: ChangesState,
    pub diff: DiffState,
    pub history: HistoryState,
    pub branch: BranchState,
    pub worktree: WorktreeState,
    pub selection: SelectionState,
    pub navigation: NavigationState,
    pub background: BackgroundTaskState,
    pub ui: UiState,
}

impl AppState {
    /// Creates a fresh, empty state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true once a repository is open and ready.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.repository.status == RepositoryStatus::Ready
    }
}
