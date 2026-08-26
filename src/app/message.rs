//! Messages returned from background workers to update state
//! (Worker -> State).
//!
//! Each message carries the [`Generation`] of the command that produced it.
//! The reducer drops messages whose generation is older than the current one
//! (stale results), while still reconciling background bookkeeping.

use super::model::{
    BranchHealth, BranchInfo, CommitSummary, DiffContent, DiffTarget, FileChange, Generation,
    HeadInfo, Oid, WorktreeInfo,
};
use super::state::HistoryFilter;

/// Error payload carried by failed operations.
///
/// A dedicated `GitError` type is introduced with the Git Service (issue #5);
/// until then messages carry a user-facing string.
pub type Failure = String;

/// Repository metadata gathered when opening.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryData {
    pub head: HeadInfo,
}

/// Divergence payload between two tips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DivergenceData {
    pub merge_base: Option<Oid>,
    pub left_only: Vec<CommitSummary>,
    pub right_only: Vec<CommitSummary>,
}

/// Loaded branch list payload (issue #30 / #18).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchesData {
    pub branches: Vec<BranchInfo>,
    pub current: Option<String>,
    pub recent: Vec<String>,
    /// Local branch names still needing ahead/behind (P3 enrichment).
    pub pending_health: Vec<String>,
}

/// Ahead/behind enrichment for deferred Branch Health (issue #18).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchHealthUpdate {
    pub name: String,
    pub ahead: u32,
    pub behind: u32,
    pub health: BranchHealth,
}

/// A full working-tree status snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatusData {
    pub staged: Vec<FileChange>,
    pub unstaged: Vec<FileChange>,
    pub untracked: Vec<FileChange>,
    pub conflicted: Vec<FileChange>,
}

/// Which network remote operation completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteOp {
    Fetch,
    Pull,
    Push,
}

/// A worker-produced message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMessage {
    RepositoryOpened {
        generation: Generation,
        result: Result<RepositoryData, Failure>,
    },
    StatusLoaded {
        generation: Generation,
        result: Result<StatusData, Failure>,
    },
    DiffLoaded {
        generation: Generation,
        result: Result<DiffContent, Failure>,
    },
    BlameEnriched {
        generation: Generation,
        target: DiffTarget,
        origins: std::collections::HashMap<u32, CommitSummary>,
        remaining: Vec<u32>,
    },
    HistoryPageLoaded {
        generation: Generation,
        filter: HistoryFilter,
        offset: usize,
        result: Result<Vec<CommitSummary>, Failure>,
    },
    BranchesLoaded {
        generation: Generation,
        result: Result<BranchesData, Failure>,
    },
    BranchHealthEnriched {
        generation: Generation,
        result: Result<Vec<BranchHealthUpdate>, Failure>,
    },
    DivergenceLoaded {
        generation: Generation,
        left: String,
        right: String,
        result: Result<DivergenceData, Failure>,
    },
    WorktreesLoaded {
        generation: Generation,
        result: Result<Vec<WorktreeInfo>, Failure>,
    },
    StageCompleted {
        generation: Generation,
        path: std::path::PathBuf,
        result: Result<(), Failure>,
    },
    UnstageCompleted {
        generation: Generation,
        path: std::path::PathBuf,
        result: Result<(), Failure>,
    },
    CommitCompleted {
        generation: Generation,
        result: Result<Oid, Failure>,
    },
    CheckoutCompleted {
        generation: Generation,
        result: Result<HeadInfo, Failure>,
    },
    BranchCreated {
        generation: Generation,
        result: Result<(), Failure>,
    },
    BranchDeleted {
        generation: Generation,
        result: Result<(), Failure>,
    },
    UpstreamSet {
        generation: Generation,
        result: Result<(), Failure>,
    },
    WorktreeCreated {
        generation: Generation,
        result: Result<WorktreeInfo, Failure>,
    },
    WorktreeRemoved {
        generation: Generation,
        result: Result<(), Failure>,
    },
    RemoteCompleted {
        generation: Generation,
        op: RemoteOp,
        /// On pull success, the refreshed HEAD; otherwise `None`.
        result: Result<Option<HeadInfo>, Failure>,
    },
}

impl AppMessage {
    /// The generation of the command that produced this message.
    #[must_use]
    pub fn generation(&self) -> Generation {
        match self {
            Self::RepositoryOpened { generation, .. }
            | Self::StatusLoaded { generation, .. }
            | Self::DiffLoaded { generation, .. }
            | Self::BlameEnriched { generation, .. }
            | Self::HistoryPageLoaded { generation, .. }
            | Self::BranchesLoaded { generation, .. }
            | Self::BranchHealthEnriched { generation, .. }
            | Self::DivergenceLoaded { generation, .. }
            | Self::WorktreesLoaded { generation, .. }
            | Self::StageCompleted { generation, .. }
            | Self::UnstageCompleted { generation, .. }
            | Self::CommitCompleted { generation, .. }
            | Self::CheckoutCompleted { generation, .. }
            | Self::BranchCreated { generation, .. }
            | Self::BranchDeleted { generation, .. }
            | Self::UpstreamSet { generation, .. }
            | Self::WorktreeCreated { generation, .. }
            | Self::WorktreeRemoved { generation, .. }
            | Self::RemoteCompleted { generation, .. } => *generation,
        }
    }
}
