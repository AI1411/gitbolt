//! UI events forwarded from the view layer (UI -> State).
//!
//! Events never perform Git I/O directly; the reducer translates them into
//! optimistic state updates and [`crate::app::command::Command`]s
//! (see `docs/design/05-architecture.md` section 9).

use std::path::PathBuf;

use super::model::{DiffView, Oid, View};

/// A user-originated event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiEvent {
    /// Open the repository rooted at the given path.
    OpenRepository(PathBuf),
    /// Close the current repository and reset state.
    CloseRepository,

    /// Switch the active navigation view.
    SelectView(View),
    /// Toggle the right-hand context panel.
    ToggleContextPanel,

    /// Select a file (shows its diff). `staged` selects index↔HEAD vs worktree↔index.
    SelectFile { path: PathBuf, staged: bool },
    /// Move the Changes list selection by `delta` (−1 / +1).
    NavigateChanges { delta: i32 },
    /// Select a commit (shows its detail).
    SelectCommit(Oid),
    /// Select a branch.
    SelectBranch(String),
    /// Filter the branch list (Quick Open for branches).
    SetBranchFilter(String),
    /// Set or change the upstream tracking ref for a branch.
    SetUpstream { branch: String, upstream: String },
    /// Open divergence view comparing `other` against the current HEAD branch.
    ShowDivergence { other: String },
    /// Clear the divergence panel.
    ClearDivergence,

    /// Change the diff presentation.
    SetDiffView(DiffView),
    /// Move focused hunk by `delta` (−1 / +1) for `[` / `]`.
    NavigateHunk { delta: i32 },

    /// Toggle selection of a diff body line for line-stage.
    ToggleDiffLine(usize),
    /// Clear diff line selection.
    ClearDiffLineSelection,
    /// Stage currently selected diff lines.
    StageSelectedLines,
    /// Unstage currently selected diff lines (from staged diff).
    UnstageSelectedLines,

    /// Stage a single file (optimistic).
    StageFile(PathBuf),
    /// Unstage a single file (optimistic).
    UnstageFile(PathBuf),
    /// Stage everything.
    StageAll,
    /// Unstage everything.
    UnstageAll,

    /// Edit the in-progress commit message.
    SetCommitMessage(String),
    /// Commit the staged changes with the current message.
    Commit,

    /// Create a branch with the given name.
    CreateBranch(String),
    /// Checkout / switch to a branch.
    CheckoutBranch(String),
    /// Delete a branch.
    DeleteBranch(String),

    /// Fetch from the default remote.
    Fetch,
    /// Pull from the upstream.
    Pull,
    /// Push to the upstream.
    Push,

    /// Create a worktree for a branch at a path.
    CreateWorktree { branch: String, path: PathBuf },

    /// Load the next page of history.
    LoadMoreHistory,

    /// Update the search query.
    Search(String),
    /// Dismiss the current error banner.
    DismissError,
}
