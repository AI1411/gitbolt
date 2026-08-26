//! Executes [`Command`]s against the Git Service and returns [`AppMessage`]s.
//!
//! Issue #9 implements `OpenRepository` (and `LoadStatus` so the Ready follow-up
//! does not surface Unsupported errors). Other ops return empty success or
//! Unsupported until later MVP issues.

use std::path::Path;

use super::command::Command;
use super::message::{AppMessage, RemoteOp, RepositoryData, StatusData};
use super::model::{ChangeKind, FileChange, HeadInfo, Oid};
use crate::git::{ChangeStatus, GitError, GitService, GixService, RepoStatus};

/// Runs a single command and returns the corresponding worker message.
#[must_use]
pub fn execute(cmd: &Command, repo_path: Option<&Path>) -> AppMessage {
    match cmd {
        Command::OpenRepository { path, generation } => AppMessage::RepositoryOpened {
            generation: *generation,
            result: open_repository(path),
        },
        Command::LoadStatus { generation } => AppMessage::StatusLoaded {
            generation: *generation,
            result: load_status(repo_path),
        },
        Command::LoadHistoryPage { offset, generation } => AppMessage::HistoryPageLoaded {
            generation: *generation,
            offset: *offset,
            result: Ok(Vec::new()),
        },
        Command::LoadBranches { generation } => AppMessage::BranchesLoaded {
            generation: *generation,
            result: Ok((Vec::new(), None)),
        },
        Command::LoadWorktrees { generation } => AppMessage::WorktreesLoaded {
            generation: *generation,
            result: Ok(Vec::new()),
        },
        other => unsupported(other),
    }
}

fn open_repository(path: &Path) -> Result<RepositoryData, String> {
    let service = GixService::open(path).map_err(|e| e.user_message())?;
    let head = service.head().map_err(|e| e.user_message())?;
    Ok(RepositoryData {
        head: HeadInfo {
            branch: head.branch,
            oid: head.oid.map(Oid),
            detached: head.detached,
        },
    })
}

fn load_status(repo_path: Option<&Path>) -> Result<StatusData, String> {
    let path = repo_path.ok_or_else(|| "リポジトリが開かれていません".to_string())?;
    let service = GixService::open(path).map_err(|e| e.user_message())?;
    let status = service.status().map_err(|e| e.user_message())?;
    Ok(status_data(status))
}

fn status_data(status: RepoStatus) -> StatusData {
    StatusData {
        staged: map_changes(status.staged),
        unstaged: map_changes(status.unstaged),
        untracked: map_changes(status.untracked),
        conflicted: map_changes(status.conflicted),
    }
}

fn map_changes(changes: Vec<crate::git::FileChange>) -> Vec<FileChange> {
    changes
        .into_iter()
        .map(|c| FileChange::new(c.path, map_kind(c.status)))
        .collect()
}

fn map_kind(status: ChangeStatus) -> ChangeKind {
    match status {
        ChangeStatus::Added => ChangeKind::Added,
        ChangeStatus::Modified => ChangeKind::Modified,
        ChangeStatus::Deleted => ChangeKind::Deleted,
        ChangeStatus::Renamed => ChangeKind::Renamed,
        ChangeStatus::Copied => ChangeKind::Copied,
        ChangeStatus::TypeChange => ChangeKind::TypeChanged,
        ChangeStatus::Untracked => ChangeKind::Untracked,
        ChangeStatus::Conflicted => ChangeKind::Conflicted,
    }
}

fn unsupported(cmd: &Command) -> AppMessage {
    let generation = cmd.generation();
    let err = GitError::unsupported(command_name(cmd)).user_message();
    match cmd {
        Command::OpenRepository { .. }
        | Command::LoadStatus { .. }
        | Command::LoadHistoryPage { .. }
        | Command::LoadBranches { .. }
        | Command::LoadWorktrees { .. } => {
            unreachable!("handled in execute")
        }
        Command::LoadDiff { .. } => AppMessage::DiffLoaded {
            generation,
            result: Err(err),
        },
        Command::Stage { path, .. } => AppMessage::StageCompleted {
            generation,
            path: path.clone(),
            result: Err(err),
        },
        Command::Unstage { path, .. } => AppMessage::UnstageCompleted {
            generation,
            path: path.clone(),
            result: Err(err),
        },
        Command::StageAll { .. } => AppMessage::StageCompleted {
            generation,
            path: Path::new(".").to_path_buf(),
            result: Err(err),
        },
        Command::UnstageAll { .. } => AppMessage::UnstageCompleted {
            generation,
            path: Path::new(".").to_path_buf(),
            result: Err(err),
        },
        Command::Commit { .. } => AppMessage::CommitCompleted {
            generation,
            result: Err(err),
        },
        Command::CreateBranch { .. } => AppMessage::BranchCreated {
            generation,
            result: Err(err),
        },
        Command::Checkout { .. } => AppMessage::CheckoutCompleted {
            generation,
            result: Err(err),
        },
        Command::DeleteBranch { .. } => AppMessage::BranchDeleted {
            generation,
            result: Err(err),
        },
        Command::Fetch { .. } => AppMessage::RemoteCompleted {
            generation,
            op: RemoteOp::Fetch,
            result: Err(err),
        },
        Command::Pull { .. } => AppMessage::RemoteCompleted {
            generation,
            op: RemoteOp::Pull,
            result: Err(err),
        },
        Command::Push { .. } => AppMessage::RemoteCompleted {
            generation,
            op: RemoteOp::Push,
            result: Err(err),
        },
        Command::CreateWorktree { .. } => AppMessage::WorktreeCreated {
            generation,
            result: Err(err),
        },
    }
}

fn command_name(cmd: &Command) -> &'static str {
    match cmd {
        Command::OpenRepository { .. } => "open",
        Command::LoadStatus { .. } => "status",
        Command::LoadDiff { .. } => "diff",
        Command::LoadHistoryPage { .. } => "log",
        Command::LoadBranches { .. } => "branches",
        Command::LoadWorktrees { .. } => "worktrees",
        Command::Stage { .. } | Command::StageAll { .. } => "stage",
        Command::Unstage { .. } | Command::UnstageAll { .. } => "unstage",
        Command::Commit { .. } => "commit",
        Command::CreateBranch { .. } => "create_branch",
        Command::Checkout { .. } => "checkout",
        Command::DeleteBranch { .. } => "delete_branch",
        Command::Fetch { .. } => "fetch",
        Command::Pull { .. } => "pull",
        Command::Push { .. } => "push",
        Command::CreateWorktree { .. } => "create_worktree",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::model::Generation;
    use crate::git::fixture::TempRepo;

    #[test]
    fn open_repository_returns_head_metadata() {
        let repo = TempRepo::init();
        repo.write("README.md", "# hello\n");
        repo.stage("README.md");
        repo.commit("initial");

        let msg = execute(
            &Command::OpenRepository {
                path: repo.path().to_path_buf(),
                generation: Generation(1),
            },
            None,
        );

        match msg {
            AppMessage::RepositoryOpened {
                generation,
                result: Ok(data),
            } => {
                assert_eq!(generation, Generation(1));
                assert_eq!(data.head.branch.as_deref(), Some("main"));
                assert!(!data.head.detached);
                assert!(data.head.oid.is_some());
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn open_non_repository_returns_user_facing_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        let msg = execute(
            &Command::OpenRepository {
                path: dir.path().to_path_buf(),
                generation: Generation(2),
            },
            None,
        );

        match msg {
            AppMessage::RepositoryOpened {
                result: Err(err), ..
            } => {
                assert!(
                    err.contains("Git リポジトリではありません"),
                    "unexpected error: {err}"
                );
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn load_status_maps_working_tree_changes() {
        let repo = TempRepo::init();
        repo.write("tracked.txt", "one\n");
        repo.stage("tracked.txt");
        repo.commit("initial");
        repo.write("tracked.txt", "one\ntwo\n");
        repo.write("loose.txt", "x\n");

        let msg = execute(
            &Command::LoadStatus {
                generation: Generation(3),
            },
            Some(repo.path()),
        );

        match msg {
            AppMessage::StatusLoaded {
                result: Ok(status), ..
            } => {
                assert!(status
                    .unstaged
                    .iter()
                    .any(|f| f.path.ends_with("tracked.txt")));
                assert!(status
                    .untracked
                    .iter()
                    .any(|f| f.path.ends_with("loose.txt")));
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }
}
