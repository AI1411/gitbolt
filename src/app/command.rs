//! Side-effecting commands produced by the reducer and executed by background
//! workers / the Git Service (State -> Worker).
//!
//! Every command carries the [`Generation`] it was issued under so that the
//! resulting [`crate::app::message::AppMessage`] can be discarded if the
//! repository context has since changed.

use std::path::PathBuf;

use super::model::{DiffTarget, Generation};

/// A unit of work to be executed off the UI thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    OpenRepository {
        path: PathBuf,
        generation: Generation,
    },
    LoadStatus {
        generation: Generation,
    },
    LoadDiff {
        target: DiffTarget,
        generation: Generation,
    },
    LoadHistoryPage {
        offset: usize,
        generation: Generation,
    },
    LoadBranches {
        generation: Generation,
    },
    LoadDivergence {
        left: String,
        right: String,
        generation: Generation,
    },
    LoadWorktrees {
        generation: Generation,
    },
    Stage {
        path: PathBuf,
        generation: Generation,
    },
    Unstage {
        path: PathBuf,
        generation: Generation,
    },
    StageLines {
        path: PathBuf,
        /// When true, reverse-apply against the staged diff (unstage lines).
        from_staged: bool,
        lines: Vec<usize>,
        generation: Generation,
    },
    StageAll {
        generation: Generation,
    },
    UnstageAll {
        generation: Generation,
    },
    Commit {
        message: String,
        generation: Generation,
    },
    CreateBranch {
        name: String,
        generation: Generation,
    },
    Checkout {
        name: String,
        generation: Generation,
    },
    DeleteBranch {
        name: String,
        generation: Generation,
    },
    Fetch {
        generation: Generation,
    },
    Pull {
        generation: Generation,
    },
    Push {
        generation: Generation,
    },
    CreateWorktree {
        branch: String,
        path: PathBuf,
        generation: Generation,
    },
}

impl Command {
    /// The generation under which this command was issued.
    #[must_use]
    pub fn generation(&self) -> Generation {
        match self {
            Self::OpenRepository { generation, .. }
            | Self::LoadStatus { generation }
            | Self::LoadDiff { generation, .. }
            | Self::LoadHistoryPage { generation, .. }
            | Self::LoadBranches { generation }
            | Self::LoadDivergence { generation, .. }
            | Self::LoadWorktrees { generation }
            | Self::Stage { generation, .. }
            | Self::Unstage { generation, .. }
            | Self::StageLines { generation, .. }
            | Self::StageAll { generation }
            | Self::UnstageAll { generation }
            | Self::Commit { generation, .. }
            | Self::CreateBranch { generation, .. }
            | Self::Checkout { generation, .. }
            | Self::DeleteBranch { generation, .. }
            | Self::Fetch { generation }
            | Self::Pull { generation }
            | Self::Push { generation }
            | Self::CreateWorktree { generation, .. } => *generation,
        }
    }
}
