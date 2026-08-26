//! The `GitService` trait: the single boundary through which the app performs
//! Git operations (see `docs/design/02-tech-and-performance.md` section 2 and
//! `docs/design/09-git-backend.md`).
//!
//! Implementations may use gix, the git CLI, or a mix per operation. Only
//! `open`, `head`, and `status` are implemented in this foundation
//! (issue #5); the remaining methods have default `Unsupported` bodies and are
//! filled in by later MVP issues.

use std::path::{Path, PathBuf};

use super::error::GitError;

/// The kind of change reported for a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChange,
    Untracked,
    Conflicted,
}

/// A single changed file with its status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub path: PathBuf,
    pub status: ChangeStatus,
}

impl FileChange {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>, status: ChangeStatus) -> Self {
        Self {
            path: path.into(),
            status,
        }
    }
}

/// Current HEAD summary.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Head {
    pub branch: Option<String>,
    pub oid: Option<String>,
    pub detached: bool,
}

/// Working-tree status grouped by area.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RepoStatus {
    pub staged: Vec<FileChange>,
    pub unstaged: Vec<FileChange>,
    pub untracked: Vec<FileChange>,
    pub conflicted: Vec<FileChange>,
}

impl RepoStatus {
    /// True when there are no changes of any kind.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.staged.is_empty()
            && self.unstaged.is_empty()
            && self.untracked.is_empty()
            && self.conflicted.is_empty()
    }

    /// Total number of changed entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.staged.len() + self.unstaged.len() + self.untracked.len() + self.conflicted.len()
    }

    /// True when [`Self::len`] is zero.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A commit as returned by `log`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitInfo {
    pub oid: String,
    pub summary: String,
    pub author: String,
    /// Unix timestamp (seconds).
    pub time: i64,
}

/// A branch reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchRef {
    pub name: String,
    pub is_head: bool,
    pub upstream: Option<String>,
}

/// A worktree reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeRef {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub is_primary: bool,
}

/// A stash entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashEntry {
    pub index: usize,
    pub message: String,
}

/// Rendered diff text for a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffText {
    pub path: PathBuf,
    pub staged: bool,
    pub text: String,
}

/// The boundary through which all Git operations flow.
///
/// Note: `open` is an associated constructor, so this trait is intentionally
/// not object-safe for now. A `dyn`-friendly split can be introduced later if
/// multiple backends need to coexist at runtime.
pub trait GitService: Sized {
    /// Open (and discover) the repository containing `path`.
    ///
    /// # Errors
    /// Returns [`GitError::NotARepository`] if no repository is found, or a
    /// backend error otherwise.
    fn open(path: &Path) -> Result<Self, GitError>;

    /// The current HEAD.
    ///
    /// # Errors
    /// Returns a backend error if HEAD cannot be resolved.
    fn head(&self) -> Result<Head, GitError>;

    /// The working-tree status.
    ///
    /// # Errors
    /// Returns a backend error if the status cannot be computed.
    fn status(&self) -> Result<RepoStatus, GitError>;

    /// The diff for a single file (worktree or staged).
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn diff(&self, _path: &Path, _staged: bool) -> Result<DiffText, GitError> {
        Err(GitError::unsupported("diff"))
    }

    /// Stage a single file.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn stage(&self, _path: &Path) -> Result<(), GitError> {
        Err(GitError::unsupported("stage"))
    }

    /// Unstage a single file.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn unstage(&self, _path: &Path) -> Result<(), GitError> {
        Err(GitError::unsupported("unstage"))
    }

    /// Stage only selected lines from a file's unified diff.
    ///
    /// `selected` are body-line indices (see [`super::patch`]). When
    /// `from_staged` is true, those lines are unstaged via a reverse apply.
    ///
    /// # Errors
    /// Returns a backend error when the patch cannot be applied.
    fn stage_lines(
        &self,
        _path: &Path,
        _from_staged: bool,
        _selected: &[usize],
    ) -> Result<(), GitError> {
        Err(GitError::unsupported("stage_lines"))
    }

    /// Stage all changes.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn stage_all(&self) -> Result<(), GitError> {
        Err(GitError::unsupported("stage_all"))
    }

    /// Unstage all changes.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn unstage_all(&self) -> Result<(), GitError> {
        Err(GitError::unsupported("unstage_all"))
    }

    /// Create a commit from the staged changes.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn commit(&self, _message: &str) -> Result<String, GitError> {
        Err(GitError::unsupported("commit"))
    }

    /// Read up to `limit` commits from HEAD.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn log(&self, _limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        Err(GitError::unsupported("log"))
    }

    /// Read `limit` commits starting at `skip` (newest-first).
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn log_page(&self, skip: usize, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        let _ = (skip, limit);
        Err(GitError::unsupported("log_page"))
    }

    /// Blame a file at HEAD (1-based line → commit).
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn blame(&self, _path: &Path) -> Result<std::collections::HashMap<u32, CommitInfo>, GitError> {
        Err(GitError::unsupported("blame"))
    }

    /// List branches.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn branches(&self) -> Result<Vec<BranchRef>, GitError> {
        Err(GitError::unsupported("branches"))
    }

    /// Merge-base OID of two tips.
    ///
    /// # Errors
    /// Returns a backend error when the tips cannot be resolved.
    fn merge_base(&self, _a: &str, _b: &str) -> Result<String, GitError> {
        Err(GitError::unsupported("merge_base"))
    }

    /// Commits on `tip` since `base` (exclusive).
    ///
    /// # Errors
    /// Returns a backend error when the walk fails.
    fn commits_not_in(
        &self,
        _tip: &str,
        _base: &str,
        _limit: usize,
    ) -> Result<Vec<CommitInfo>, GitError> {
        Err(GitError::unsupported("commits_not_in"))
    }

    /// Ahead/behind of `tip` vs `other`.
    ///
    /// # Errors
    /// Returns a backend error when counts cannot be computed.
    fn ahead_behind(&self, _tip: &str, _other: &str) -> Result<(u32, u32), GitError> {
        Err(GitError::unsupported("ahead_behind"))
    }

    /// Recent branch names from reflog.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn recent_branches(&self, _limit: usize) -> Result<Vec<String>, GitError> {
        Err(GitError::unsupported("recent_branches"))
    }

    /// Last commit on a branch tip.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn branch_last_commit(&self, _branch: &str) -> Result<Option<CommitInfo>, GitError> {
        Err(GitError::unsupported("branch_last_commit"))
    }

    /// Set upstream tracking for a branch.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn set_upstream(&self, _branch: &str, _upstream: &str) -> Result<(), GitError> {
        Err(GitError::unsupported("set_upstream"))
    }

    /// Create a branch.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn create_branch(&self, _name: &str) -> Result<(), GitError> {
        Err(GitError::unsupported("create_branch"))
    }

    /// Checkout / switch to a branch.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn checkout(&self, _name: &str) -> Result<Head, GitError> {
        Err(GitError::unsupported("checkout"))
    }

    /// Delete a branch.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn delete_branch(&self, _name: &str) -> Result<(), GitError> {
        Err(GitError::unsupported("delete_branch"))
    }

    /// Fetch from the default remote.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn fetch(&self) -> Result<(), GitError> {
        Err(GitError::unsupported("fetch"))
    }

    /// Pull from the upstream.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn pull(&self) -> Result<(), GitError> {
        Err(GitError::unsupported("pull"))
    }

    /// Push to the upstream.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn push(&self) -> Result<(), GitError> {
        Err(GitError::unsupported("push"))
    }

    /// List worktrees.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn worktrees(&self) -> Result<Vec<WorktreeRef>, GitError> {
        Err(GitError::unsupported("worktrees"))
    }

    /// Create a worktree for a branch at a path.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn create_worktree(&self, _branch: &str, _path: &Path) -> Result<WorktreeRef, GitError> {
        Err(GitError::unsupported("create_worktree"))
    }

    /// Remove a worktree.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn remove_worktree(&self, _path: &Path) -> Result<(), GitError> {
        Err(GitError::unsupported("remove_worktree"))
    }

    /// List stashes.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn stash_list(&self) -> Result<Vec<StashEntry>, GitError> {
        Err(GitError::unsupported("stash_list"))
    }

    /// Save a stash.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn stash_save(&self, _message: Option<&str>) -> Result<(), GitError> {
        Err(GitError::unsupported("stash_save"))
    }

    /// Apply a stash.
    ///
    /// # Errors
    /// Returns [`GitError::Unsupported`] until implemented.
    fn stash_apply(&self, _index: usize) -> Result<(), GitError> {
        Err(GitError::unsupported("stash_apply"))
    }
}
