//! Executes [`Command`]s against the Git Service and returns [`AppMessage`]s.

use std::path::Path;

use super::branch_health::{classify_health, days_since, STALE_DAYS_THRESHOLD};
use super::command::Command;
use super::diff_parse::parse_diff_content;
use super::message::{
    AppMessage, BranchHealthUpdate, BranchesData, DivergenceData, RemoteOp, RepositoryData,
    StatusData,
};
use super::model::{
    BranchHealth, BranchInfo, ChangeKind, CommitDetail, CommitFileEntry, CommitSummary, DiffTarget,
    FileChange, HeadInfo, Oid, StashInfo,
};
use super::state::HistoryFilter;
use crate::git::{remote, remote_link};
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
        Command::EnrichBlame {
            target,
            lines,
            remaining,
            generation,
        } => enrich_blame_message(repo_path, target, lines, remaining, *generation),
        Command::LoadHistoryPage {
            filter,
            offset,
            generation,
        } => AppMessage::HistoryPageLoaded {
            generation: *generation,
            filter: filter.clone(),
            offset: *offset,
            result: load_history(repo_path, filter, *offset),
        },
        Command::LoadBranches { generation } => AppMessage::BranchesLoaded {
            generation: *generation,
            result: load_branches(repo_path),
        },
        Command::EnrichBranchHealth { names, generation } => AppMessage::BranchHealthEnriched {
            generation: *generation,
            result: enrich_branch_health(repo_path, names),
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
        Command::Stage { .. }
        | Command::Unstage { .. }
        | Command::StageLines { .. }
        | Command::StageAll { .. }
        | Command::UnstageAll { .. }
        | Command::Commit { .. } => execute_stage_commit(cmd, repo_path),
        Command::CreateBranch { .. } | Command::Checkout { .. } | Command::DeleteBranch { .. } => {
            execute_branch_mutation(cmd, repo_path)
        }
        Command::Fetch { .. }
        | Command::AutoFetch { .. }
        | Command::Pull { .. }
        | Command::Push { .. } => execute_remote(cmd, repo_path),
        Command::CreateWorktree { .. } | Command::RemoveWorktree { .. } => {
            execute_worktree_mutation(cmd, repo_path)
        }
        Command::LoadStashes { .. }
        | Command::LoadStashDiff { .. }
        | Command::StashSave { .. }
        | Command::StashApply { .. }
        | Command::StashPop { .. }
        | Command::StashDrop { .. } => execute_stash(cmd, repo_path),
        Command::LoadCommitDetail { oid, generation } => AppMessage::CommitDetailLoaded {
            generation: *generation,
            oid: oid.clone(),
            result: load_commit_detail(repo_path, oid),
        },
        Command::LoadCommitFileDiff {
            oid,
            path,
            generation,
        } => AppMessage::CommitFileDiffLoaded {
            generation: *generation,
            oid: oid.clone(),
            path: path.clone(),
            result: load_commit_file_diff(repo_path, oid, path),
        },
    }
}

fn execute_stash(cmd: &Command, repo_path: Option<&Path>) -> AppMessage {
    match cmd {
        Command::LoadStashes { generation } => AppMessage::StashesLoaded {
            generation: *generation,
            result: load_stashes(repo_path),
        },
        Command::LoadStashDiff { index, generation } => AppMessage::StashDiffLoaded {
            generation: *generation,
            index: *index,
            result: load_stash_diff(repo_path, *index),
        },
        Command::StashSave {
            message,
            generation,
        } => AppMessage::StashSaved {
            generation: *generation,
            result: with_service(repo_path, |svc| svc.stash_save(message.as_deref())),
        },
        Command::StashApply { index, generation } => AppMessage::StashApplied {
            generation: *generation,
            result: with_service(repo_path, |svc| svc.stash_apply(*index)),
        },
        Command::StashPop { index, generation } => AppMessage::StashPopped {
            generation: *generation,
            result: with_service(repo_path, |svc| svc.stash_pop(*index)),
        },
        Command::StashDrop { index, generation } => AppMessage::StashDropped {
            generation: *generation,
            result: with_service(repo_path, |svc| svc.stash_drop(*index)),
        },
        _ => unreachable!("stash command only"),
    }
}

fn execute_stage_commit(cmd: &Command, repo_path: Option<&Path>) -> AppMessage {
    match cmd {
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
        _ => unreachable!("stage/commit only"),
    }
}

fn execute_worktree_mutation(cmd: &Command, repo_path: Option<&Path>) -> AppMessage {
    match cmd {
        Command::CreateWorktree {
            branch,
            path,
            generation,
        } => AppMessage::WorktreeCreated {
            generation: *generation,
            result: with_service(repo_path, |svc| {
                let w = svc.create_worktree(branch, path)?;
                Ok(crate::app::model::WorktreeInfo {
                    path: w.path,
                    branch: w.branch,
                    is_primary: w.is_primary,
                })
            }),
        },
        Command::RemoveWorktree { path, generation } => AppMessage::WorktreeRemoved {
            generation: *generation,
            result: with_service(repo_path, |svc| svc.remove_worktree(path)),
        },
        _ => unreachable!("worktree mutation only"),
    }
}

fn execute_remote(cmd: &Command, repo_path: Option<&Path>) -> AppMessage {
    match cmd {
        Command::Fetch { generation } | Command::AutoFetch { generation } => {
            AppMessage::RemoteCompleted {
                generation: *generation,
                op: RemoteOp::Fetch,
                result: with_service(repo_path, |svc| {
                    svc.fetch()?;
                    Ok(None)
                }),
            }
        }
        Command::Pull { generation } => AppMessage::RemoteCompleted {
            generation: *generation,
            op: RemoteOp::Pull,
            result: with_service(repo_path, |svc| {
                svc.pull()?;
                let head = svc.head()?;
                Ok(Some(HeadInfo {
                    branch: head.branch,
                    oid: head.oid.map(Oid),
                    detached: head.detached,
                }))
            }),
        },
        Command::Push { generation } => AppMessage::RemoteCompleted {
            generation: *generation,
            op: RemoteOp::Push,
            result: with_service(repo_path, |svc| {
                svc.push()?;
                Ok(None)
            }),
        },
        _ => unreachable!("remote only"),
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

fn load_commit_detail(repo_path: Option<&Path>, oid: &Oid) -> Result<CommitDetail, String> {
    let path = repo_path.ok_or_else(|| "リポジトリが開かれていません".to_string())?;
    let service = GixService::open(path).map_err(|e| e.user_message())?;
    let detail = service
        .commit_detail(&oid.0)
        .map_err(|e| e.user_message())?;
    Ok(CommitDetail {
        oid: Oid(detail.oid),
        summary: detail.summary,
        author: detail.author,
        timestamp: detail.timestamp,
        body: detail.body,
        files: detail
            .files
            .into_iter()
            .map(|f| CommitFileEntry {
                status: f.status,
                path: f.path,
            })
            .collect(),
    })
}

fn load_commit_file_diff(
    repo_path: Option<&Path>,
    oid: &Oid,
    file: &Path,
) -> Result<crate::app::model::DiffContent, String> {
    let repo = repo_path.ok_or_else(|| "リポジトリが開かれていません".to_string())?;
    let patch = crate::git::commit_detail::show_file_diff(repo, &oid.0, file)
        .map_err(|e| e.user_message())?;
    Ok(parse_diff_content(
        DiffTarget {
            path: file.to_path_buf(),
            staged: false,
        },
        &patch,
    ))
}

fn load_stashes(repo_path: Option<&Path>) -> Result<Vec<StashInfo>, String> {
    let path = repo_path.ok_or_else(|| "リポジトリが開かれていません".to_string())?;
    let service = GixService::open(path).map_err(|e| e.user_message())?;
    let entries = service.stash_list().map_err(|e| e.user_message())?;
    Ok(entries
        .into_iter()
        .map(|e| StashInfo {
            index: e.index,
            message: e.message,
        })
        .collect())
}

fn load_stash_diff(
    repo_path: Option<&Path>,
    index: usize,
) -> Result<crate::app::model::DiffContent, String> {
    let path = repo_path.ok_or_else(|| "リポジトリが開かれていません".to_string())?;
    let service = GixService::open(path).map_err(|e| e.user_message())?;
    let stash_patch = service.stash_show(index).map_err(|e| e.user_message())?;
    Ok(parse_diff_content(
        DiffTarget {
            path: format!("stash@{{{index}}}").into(),
            staged: false,
        },
        &stash_patch,
    ))
}

fn load_history(
    repo_path: Option<&Path>,
    filter: &HistoryFilter,
    offset: usize,
) -> Result<Vec<crate::app::model::CommitSummary>, String> {
    let path = repo_path.ok_or_else(|| "リポジトリが開かれていません".to_string())?;
    let service = GixService::open(path).map_err(|e| e.user_message())?;
    let limit = crate::app::reducer::HISTORY_PAGE;
    let page = match filter {
        HistoryFilter::All => service.log_page(offset, limit),
        HistoryFilter::File { path: file } => service.file_log_page(file, offset, limit),
        HistoryFilter::Line { path: file, line } => {
            service.line_log_page(file, *line, offset, limit)
        }
    }
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
    let mut data = status_data(status);
    data.origin_web = remote::origin_url(path)
        .ok()
        .and_then(|u| remote_link::parse_remote_url(&u));
    Ok(data)
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
    // Blame is enriched progressively after DiffLoaded (issue #22).
    Ok(parse_diff_content(target.clone(), &diff.text))
}

fn enrich_blame_message(
    repo_path: Option<&Path>,
    target: &DiffTarget,
    lines: &[u32],
    remaining: &[u32],
    generation: crate::app::model::Generation,
) -> AppMessage {
    let result = enrich_blame(repo_path, &target.path, lines);
    match result {
        Ok(origins) => AppMessage::BlameEnriched {
            generation,
            target: target.clone(),
            origins,
            remaining: remaining.to_vec(),
        },
        Err(err) => {
            // Soft-fail: still clear remaining so we don't loop forever.
            let _ = err;
            AppMessage::BlameEnriched {
                generation,
                target: target.clone(),
                origins: std::collections::HashMap::new(),
                remaining: Vec::new(),
            }
        }
    }
}

fn enrich_blame(
    repo_path: Option<&Path>,
    path: &Path,
    lines: &[u32],
) -> Result<std::collections::HashMap<u32, crate::app::model::CommitSummary>, String> {
    if lines.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let repo = repo_path.ok_or_else(|| "リポジトリが開かれていません".to_string())?;
    let service = GixService::open(repo).map_err(|e| e.user_message())?;
    let map = service
        .blame_lines(path, lines)
        .map_err(|e| e.user_message())?;
    Ok(map
        .into_iter()
        .map(|(line, info)| (line, to_summary(info)))
        .collect())
}

fn unix_now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
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
    let now = unix_now_secs();

    let priority: std::collections::HashSet<&str> = recent
        .iter()
        .map(String::as_str)
        .chain(current.as_deref())
        .collect();

    let mut pending_health = Vec::new();
    let mut infos = Vec::new();
    for b in refs {
        let last_commit = service
            .branch_last_commit(&b.name)
            .ok()
            .flatten()
            .map(to_summary);
        let stale_days = last_commit
            .as_ref()
            .and_then(|c| days_since(c.timestamp, now));

        let compute_ab_now = !b.is_remote && (priority.contains(b.name.as_str()) || b.is_head);
        let (ahead, behind) = if b.is_remote {
            (0, 0)
        } else if compute_ab_now {
            if let Some(up) = b.upstream.as_deref() {
                service.ahead_behind(&b.name, up).unwrap_or((0, 0))
            } else {
                (0, 0)
            }
        } else {
            if b.upstream.is_some() {
                pending_health.push(b.name.clone());
            }
            (0, 0)
        };

        let health = if b.is_remote {
            BranchHealth::Local
        } else {
            classify_health(
                ahead,
                behind,
                b.upstream.is_some(),
                stale_days,
                STALE_DAYS_THRESHOLD,
            )
        };

        infos.push(BranchInfo {
            name: b.name,
            upstream: b.upstream,
            health,
            ahead,
            behind,
            last_commit,
            is_remote: b.is_remote,
            stale_days,
        });
    }

    let base = ["main", "master", "develop", "trunk"]
        .into_iter()
        .find(|name| infos.iter().any(|b| !b.is_remote && b.name == *name))
        .unwrap_or("main");
    let merged_into_base = crate::git::branch::merged_into(path, base).unwrap_or_default();

    Ok(BranchesData {
        branches: infos,
        current,
        recent,
        pending_health,
        merged_into_base,
    })
}

fn enrich_branch_health(
    repo_path: Option<&Path>,
    names: &[String],
) -> Result<Vec<BranchHealthUpdate>, String> {
    let path = repo_path.ok_or_else(|| "リポジトリが開かれていません".to_string())?;
    let service = GixService::open(path).map_err(|e| e.user_message())?;
    let refs = service.branches().map_err(|e| e.user_message())?;
    let now = unix_now_secs();

    let mut updates = Vec::new();
    for name in names {
        let Some(b) = refs.iter().find(|r| r.name == *name && !r.is_remote) else {
            continue;
        };
        let (ahead, behind) = if let Some(up) = b.upstream.as_deref() {
            service.ahead_behind(&b.name, up).unwrap_or((0, 0))
        } else {
            (0, 0)
        };
        let stale_days = service
            .branch_last_commit(&b.name)
            .ok()
            .flatten()
            .and_then(|c| days_since(c.time, now));
        let health = classify_health(
            ahead,
            behind,
            b.upstream.is_some(),
            stale_days,
            STALE_DAYS_THRESHOLD,
        );
        updates.push(BranchHealthUpdate {
            name: name.clone(),
            ahead,
            behind,
            health,
        });
    }
    Ok(updates)
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
        origin_web: None,
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
    fn load_status_parses_origin_web_host() {
        let repo = TempRepo::init();
        repo.write("a.txt", "a\n");
        repo.stage("a.txt");
        repo.commit("initial");
        repo.run(&[
            "remote",
            "add",
            "origin",
            "https://github.com/ai1411/gitbolt.git",
        ]);

        let msg = execute(
            &Command::LoadStatus {
                generation: Generation(1),
            },
            Some(repo.path()),
        );

        match msg {
            AppMessage::StatusLoaded {
                result: Ok(status), ..
            } => {
                let web = status.origin_web.expect("origin_web");
                assert_eq!(web.web_base, "https://github.com/ai1411/gitbolt");
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
    fn load_diff_then_enrich_blame_attaches_change_origin() {
        let repo = TempRepo::init();
        repo.write("f.txt", "alpha\nbeta\ngamma\n");
        repo.stage("f.txt");
        repo.commit("seed lines");
        let seed = repo.run(&["rev-parse", "HEAD"]);
        repo.write("f.txt", "alpha\nBETA\ngamma\n");

        let target = DiffTarget {
            path: Path::new("f.txt").to_path_buf(),
            staged: false,
        };
        let msg = execute(
            &Command::LoadDiff {
                target: target.clone(),
                generation: Generation(5),
            },
            Some(repo.path()),
        );

        let content = match msg {
            AppMessage::DiffLoaded {
                result: Ok(content),
                ..
            } => content,
            other => panic!("unexpected: {other:?}"),
        };
        let old_lines: Vec<u32> = content
            .hunks
            .iter()
            .flat_map(|h| h.lines.iter())
            .filter_map(|l| l.old_line)
            .collect();
        assert!(!old_lines.is_empty());

        let enrich = execute(
            &Command::EnrichBlame {
                target,
                lines: old_lines,
                remaining: Vec::new(),
                generation: Generation(5),
            },
            Some(repo.path()),
        );
        match enrich {
            AppMessage::BlameEnriched { origins, .. } => {
                assert!(origins.values().any(|c| c.oid.0 == seed));
            }
            other => panic!("unexpected enrich: {other:?}"),
        }
    }
}
