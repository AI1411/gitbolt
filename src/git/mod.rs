//! Git service layer — all Git operations are routed through here.
//!
//! See `docs/design/02-tech-and-performance.md` and `docs/design/05-architecture.md`.

pub mod blame;
pub mod branch;
pub mod cli;
pub mod commit;
pub mod commit_detail;
pub mod diff;
pub mod error;
pub mod history;
pub mod patch;
pub mod remote;
pub mod repository;
pub mod service;
pub mod stage;
pub mod stash;
pub mod status;
pub mod worktree;

#[cfg(test)]
pub(crate) mod fixture;

pub use cli::GitCli;
pub use error::GitError;
pub use repository::GixService;
pub use service::{
    BranchRef, ChangeStatus, CommitInfo, DiffText, FileChange, GitService, Head, RepoStatus,
    StashEntry, WorktreeRef,
};
