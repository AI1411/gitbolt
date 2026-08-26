//! Top-level application state and sub-states.
//!
//! See `docs/design/05-architecture.md` section 8. Git-derived data is kept
//! separate from UI state; large collections are held behind `Arc`.

use std::path::PathBuf;
use std::sync::Arc;

use super::model::{
    BranchInfo, CommitDetail, CommitSummary, DiffContent, DiffTarget, DiffView, FileChange,
    Generation, HeadInfo, Loadable, Oid, Pane, StashInfo, View, WorktreeInfo,
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
    /// Selected body-line indices for line-level stage (issue #28).
    pub selected_lines: Vec<usize>,
    /// Focused hunk index for `[` / `]` navigation (issue #13).
    pub focused_hunk: usize,
    /// Show blame heatmap gutter (issue #33).
    pub heatmap_enabled: bool,
}

/// Commit history with lazy paging.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum HistoryFilter {
    #[default]
    All,
    File {
        path: PathBuf,
    },
    Line {
        path: PathBuf,
        line: u32,
    },
}

/// Commit history with lazy paging.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HistoryState {
    pub commits: Vec<CommitSummary>,
    pub has_more: bool,
    pub loading: bool,
    pub filter: HistoryFilter,
}

/// Branch list and current branch.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BranchState {
    pub branches: Arc<[BranchInfo]>,
    pub current: Option<String>,
    pub loaded: bool,
    /// Reflog-ordered recent branch names (issue #30).
    pub recent: Vec<String>,
    /// Quick Open filter for branch names.
    pub filter: String,
}

/// Divergence between two tips (issue #29).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DivergenceState {
    pub left: Option<String>,
    pub right: Option<String>,
    pub merge_base: Option<Oid>,
    pub left_only: Vec<CommitSummary>,
    pub right_only: Vec<CommitSummary>,
    pub loading: bool,
}

/// Worktree list.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorktreeState {
    pub worktrees: Arc<[WorktreeInfo]>,
    pub loaded: bool,
}

/// Stash list and selected entry diff (issue #24).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StashState {
    pub entries: Arc<[StashInfo]>,
    pub loaded: bool,
    pub selected: Option<usize>,
    pub diff: Loadable<DiffContent>,
}

/// Context panel payloads (issue #25).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContextState {
    pub commit: Loadable<CommitDetail>,
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
    /// Previously visited commits (issue #32 Back). Newest visit is last.
    pub commit_back: Vec<Oid>,
    /// Commits skipped by Back, available via Forward.
    pub commit_forward: Vec<Oid>,
}

impl Default for NavigationState {
    fn default() -> Self {
        Self {
            active_view: View::default(),
            context_panel_open: true,
            back_stack: Vec::new(),
            commit_back: Vec::new(),
            commit_forward: Vec::new(),
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
    /// Active remote operation label for Pulse (`fetch` / `pull` / `push`).
    pub remote_label: Option<String>,
}

/// Overlay for Command Palette / Quick Open (issue #26).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Overlay {
    #[default]
    None,
    CommandPalette {
        query: String,
        selected: usize,
    },
    QuickOpen {
        query: String,
        selected: usize,
    },
}

/// Presentational / editable UI state not derived from Git.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiState {
    /// The in-progress commit message. Preserved across failed commits
    /// (see `docs/design/07-runtime.md` section 13).
    pub commit_message: String,
    pub error_banner: Option<String>,
    pub search_query: String,
    pub searching: bool,
    /// Bumped to request focus on the commit message field (issue #15).
    pub commit_focus_token: u64,
    /// Draft name for creating a branch (issue #17).
    pub new_branch_name: String,
    /// Pending destructive delete confirmation (local branch name).
    pub confirm_delete_branch: Option<String>,
    /// Pending worktree removal path (issue #20).
    pub confirm_remove_worktree: Option<PathBuf>,
    /// Pending stash drop confirmation index (issue #24).
    pub confirm_drop_stash: Option<usize>,
    /// When true, Instant Worktree opens the new worktree after create (#21).
    pub open_after_instant_worktree: bool,
    /// Path to open after a successful Instant Worktree create.
    pub pending_open_worktree: Option<PathBuf>,
    /// Auto-fetch interval in seconds (issue #19). `0` disables.
    pub auto_fetch_secs: u64,
    /// Brief copy confirmation (issue #25).
    pub copy_feedback: Option<String>,
    /// Command Palette / Quick Open overlay (issue #26).
    pub overlay: Overlay,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            commit_message: String::new(),
            error_banner: None,
            search_query: String::new(),
            searching: false,
            commit_focus_token: 0,
            new_branch_name: String::new(),
            confirm_delete_branch: None,
            confirm_remove_worktree: None,
            confirm_drop_stash: None,
            open_after_instant_worktree: false,
            pending_open_worktree: None,
            auto_fetch_secs: 300,
            copy_feedback: None,
            overlay: Overlay::None,
        }
    }
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
    pub divergence: DivergenceState,
    pub worktree: WorktreeState,
    pub stash: StashState,
    pub context: ContextState,
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
