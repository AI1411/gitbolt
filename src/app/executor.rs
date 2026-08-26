//! Executes [`Command`]s against the Git Service and returns [`AppMessage`]s.

use std::path::Path;

use super::command::Command;
use super::diff_parse::parse_diff_content;
use super::message::{
    AppMessage, BranchesData, DivergenceData, RemoteOp, RepositoryData, StatusData,
};
use super::model::{
    BranchHealth, BranchInfo, ChangeKind, CommitSummary, DiffTarget, FileChange, HeadInfo, Oid,
};
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
        Command::LoadDiff { target, generation } => AppMessage::DiffLoaded {
            generation: *generation,
            result: load_diff(repo_path, target),
        },
        Command::Stage { path, generation } => AppMessage::StageCompleted {
            generation: *generation,
            path: path.clone(),
            result: with_service(repo_path, |svc| svc.stage(path)),
        },
        Command::Unstage { path, generation } => AppMessage::UnstageCompleted {
            generation: *generation,
            path: path.clone(),
            result: with_service(repo_path, |svc| svc.unstage(path)),
        },
        Command::StageLines {
            path,
            from_staged,
            lines,
            generation,
        } => AppMessage::StageCompleted {
            generation: *generation,
            path: path.clone(),
            result: with_service(repo_path, |svc| svc.stage_lines(path, *from_staged, lines)),
        },
        Command::LoadHistoryPage { offset, generation } => AppMessage::HistoryPageLoaded {
            generation: *generation,
            offset: *offset,
            result: load_history(repo_path, *offset),
        },
        Command::LoadBranches { generation } => AppMessage::BranchesLoaded {
            generation: *generation,
            result: load_branches(repo_path),
        },
        Command::LoadDivergence {
            left,
            right,
            generation,
        } => AppMessage::DivergenceLoaded {
            generation: *generation,
            left: left.clone(),
            right: right.clone(),
            result: load_divergence(repo_path, left, right),
        },
        Command::SetUpstream {
            branch,
            upstream,
            generation,
        } => AppMessage::UpstreamSet {
            generation: *generation,
            result: with_service(repo_path, |svc| svc.set_upstream(branch, upstream)),
        },
        Command::LoadWorktrees { generation } => AppMessage::WorktreesLoaded {
            generation: *generation,
            result: load_worktrees(repo_path),
        },
        Command::StageAll { generation } => AppMessage::StageCompleted {
            generation: *generation,
            path: Path::new(".").to_path_buf(),
            result: with_service(repo_path, GitService::stage_all),
        },
        Command::UnstageAll { generation } => AppMessage::UnstageCompleted {
            generation: *generation,
            path: Path::new(".").to_path_buf(),
            result: with_service(repo_path, GitService::unstage_all),
        },
        Command::Commit {
            message,
            generation,
        } => AppMessage::CommitCompleted {
            generation: *generation,
            result: with_service(repo_path, |svc| svc.commit(message).map(Oid)),
        },
        Command::CreateBranch { .. } | Command::Checkout { .. } | Command::DeleteBranch { .. } => {
            execute_branch_mutation(cmd, repo_path)
        }
        other => unsupported(other),
    }
}

fn execute_branch_mutation(cmd: &Command, repo_path: Option<&Path>) -> AppMessage {
    match cmd {
        Command::CreateBranch { name, generation } => AppMessage::BranchCreated {
            generation: *generation,
            result: with_service(repo_path, |svc| svc.create_branch(name)),
        },
        Command::Checkout { name, generation } => AppMessage::CheckoutCompleted {
            generation: *generation,
            result: with_service(repo_path, |svc| {
                svc.checkout(name).map(|head| HeadInfo {
                    branch: head.branch,
                    oid: head.oid.map(Oid),
                    detached: head.detached,
                })
            }),
        },
        Command::DeleteBranch { name, generation } => AppMessage::BranchDeleted {
            generation: *generation,
            result: with_service(repo_path, |svc| svc.delete_branch(name)),
        },
        _ => unreachable!("branch mutation only"),
    }
}

fn load_history(
    repo_path: Option<&Path>,
    offset: usize,
) -> Result<Vec<crate::app::model::CommitSummary>, String> {
    let path = repo_path.ok_or_else(|| "リポジトリが開かれていません".to_string())?;
    let service = GixService::open(path).map_err(|e| e.user_message())?;
    let page = service
        .log_page(offset, crate::app::reducer::HISTORY_PAGE)
        .map_err(|e| e.user_message())?;
    Ok(page.into_iter().map(to_summary).collect())
}

fn load_worktrees(
    repo_path: Option<&Path>,
) -> Result<Vec<crate::app::model::WorktreeInfo>, String> {
    let path = repo_path.ok_or_else(|| "リポジトリが開かれていません".to_string())?;
    let service = GixService::open(path).map_err(|e| e.user_message())?;
    let trees = service.worktrees().map_err(|e| e.user_message())?;
    Ok(trees
        .into_iter()
        .map(|w| crate::app::model::WorktreeInfo {
            path: w.path,
            branch: w.branch,
            is_primary: w.is_primary,
        })
        .collect())
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

fn load_diff(
    repo_path: Option<&Path>,
    target: &DiffTarget,
) -> Result<crate::app::model::DiffContent, String> {
    let path = repo_path.ok_or_else(|| "リポジトリが開かれていません".to_string())?;
    let service = GixService::open(path).map_err(|e| e.user_message())?;
    let diff = service
        .diff(&target.path, target.staged)
        .map_err(|e| e.user_message())?;
    let content = parse_diff_content(target.clone(), &diff.text);
    let blame = service.blame(&target.path).unwrap_or_default();
    Ok(super::diff_parse::attach_change_origins(content, &blame))
}

fn load_branches(repo_path: Option<&Path>) -> Result<BranchesData, String> {
    let path = repo_path.ok_or_else(|| "リポジトリが開かれていません".to_string())?;
    let service = GixService::open(path).map_err(|e| e.user_message())?;
    let refs = service.branches().map_err(|e| e.user_message())?;
    let current = refs
        .iter()
        .find(|b| b.is_head && !b.is_remote)
        .map(|b| b.name.clone());
    let recent = service.recent_branches(10).map_err(|e| e.user_message())?;
    let mut infos = Vec::new();
    for b in refs {
        let (ahead, behind, health) = if b.is_remote {
            (0, 0, BranchHealth::Local)
        } else if let Some(up) = b.upstream.as_deref() {
            match service.ahead_behind(&b.name, up) {
                Ok((a, be)) => {
                    let health = match (a, be) {
                        (0, 0) => BranchHealth::Synced,
                        (_, 0) => BranchHealth::Ahead,
                        (0, _) => BranchHealth::Behind,
                        _ => BranchHealth::Diverged,
                    };
                    (a, be, health)
                }
                Err(_) => (0, 0, BranchHealth::Local),
            }
        } else {
            (0, 0, BranchHealth::Local)
        };
        let last_commit = service
            .branch_last_commit(&b.name)
            .ok()
            .flatten()
            .map(to_summary);
        infos.push(BranchInfo {
            name: b.name,
            upstream: b.upstream,
            health,
            ahead,
            behind,
            last_commit,
            is_remote: b.is_remote,
        });
    }
    Ok(BranchesData {
        branches: infos,
        current,
        recent,
    })
}

fn load_divergence(
    repo_path: Option<&Path>,
    left: &str,
    right: &str,
) -> Result<DivergenceData, String> {
    let path = repo_path.ok_or_else(|| "リポジトリが開かれていません".to_string())?;
    let service = GixService::open(path).map_err(|e| e.user_message())?;
    let base = service
        .merge_base(left, right)
        .map_err(|e| e.user_message())?;
    let left_only = service
        .commits_not_in(left, &base, 100)
        .map_err(|e| e.user_message())?
        .into_iter()
        .map(to_summary)
        .collect();
    let right_only = service
        .commits_not_in(right, &base, 100)
        .map_err(|e| e.user_message())?
        .into_iter()
        .map(to_summary)
        .collect();
    Ok(DivergenceData {
        merge_base: Some(Oid(base)),
        left_only,
        right_only,
    })
}

fn to_summary(c: crate::git::CommitInfo) -> CommitSummary {
    CommitSummary {
        oid: Oid(c.oid),
        summary: c.summary,
        author: c.author,
        timestamp: c.time,
    }
}

fn with_service<T, F>(repo_path: Option<&Path>, f: F) -> Result<T, String>
where
    F: FnOnce(&GixService) -> Result<T, GitError>,
{
    let path = repo_path.ok_or_else(|| "リポジトリが開かれていません".to_string())?;
    let service = GixService::open(path).map_err(|e| e.user_message())?;
    f(&service).map_err(|e| e.user_message())
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
        | Command::LoadDiff { .. }
        | Command::Stage { .. }
        | Command::Unstage { .. }
        | Command::StageLines { .. }
        | Command::LoadHistoryPage { .. }
        | Command::LoadBranches { .. }
        | Command::LoadDivergence { .. }
        | Command::SetUpstream { .. }
        | Command::LoadWorktrees { .. }
        | Command::StageAll { .. }
        | Command::UnstageAll { .. }
        | Command::Commit { .. }
        | Command::CreateBranch { .. }
        | Command::Checkout { .. }
        | Command::DeleteBranch { .. } => unreachable!("handled in execute"),
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
        Command::LoadDivergence { .. } => "divergence",
        Command::SetUpstream { .. } => "set_upstream",
        Command::LoadWorktrees { .. } => "worktrees",
        Command::Stage { .. } | Command::StageAll { .. } | Command::StageLines { .. } => "stage",
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

    #[test]
    fn stage_lines_command_stages_partial_file() {
        let repo = TempRepo::init();
        repo.write("f.txt", "a\n");
        repo.stage("f.txt");
        repo.commit("initial");
        repo.write("f.txt", "a\nb\nc\n");

        let diff =
            crate::git::diff::unified_diff(repo.path(), Path::new("f.txt"), false).expect("diff");
        let body: Vec<_> = diff
            .text
            .lines()
            .skip_while(|l| !l.starts_with("+++ "))
            .skip(1)
            .collect();
        let b_idx = body.iter().position(|l| *l == "+b").expect("+b");

        let msg = execute(
            &Command::StageLines {
                path: Path::new("f.txt").to_path_buf(),
                from_staged: false,
                lines: vec![b_idx],
                generation: Generation(4),
            },
            Some(repo.path()),
        );
        assert!(matches!(
            msg,
            AppMessage::StageCompleted { result: Ok(()), .. }
        ));
    }

    #[test]
    fn load_diff_attaches_change_origin_from_head_blame() {
        let repo = TempRepo::init();
        repo.write("f.txt", "alpha\nbeta\ngamma\n");
        repo.stage("f.txt");
        repo.commit("seed lines");
        let seed = repo.run(&["rev-parse", "HEAD"]);
        repo.write("f.txt", "alpha\nBETA\ngamma\n");

        let msg = execute(
            &Command::LoadDiff {
                target: DiffTarget {
                    path: Path::new("f.txt").to_path_buf(),
                    staged: false,
                },
                generation: Generation(5),
            },
            Some(repo.path()),
        );

        match msg {
            AppMessage::DiffLoaded {
                result: Ok(content),
                ..
            } => {
                let deleted = content
                    .hunks
                    .iter()
                    .flat_map(|h| h.lines.iter())
                    .find(|l| l.origin == '-' && l.content == "beta")
                    .expect("deleted beta");
                let origin = deleted.change_origin.as_ref().expect("origin");
                assert_eq!(origin.oid.0, seed);
                assert!(origin.summary.contains("seed"));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }
}
