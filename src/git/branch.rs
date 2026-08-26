//! Branch listing, merge-base, and divergence commit walks (issue #29).

use std::path::Path;

use super::cli::GitCli;
use super::error::GitError;
use super::service::{BranchRef, CommitInfo};

/// Lists local branches (`git branch --format`).
///
/// # Errors
/// Propagates CLI failures.
pub fn list_branches(repo: &Path) -> Result<Vec<BranchRef>, GitError> {
    let cli = GitCli::new(repo)?;
    let out = cli.run(&[
        "branch",
        "--format=%(refname:short)%09%(HEAD)%09%(upstream:short)",
    ])?;
    let mut branches = Vec::new();
    for line in out.lines() {
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split('\t');
        let name = parts.next().unwrap_or("").to_string();
        let is_head = parts.next() == Some("*");
        let upstream = parts
            .next()
            .filter(|s| !s.is_empty())
            .map(std::string::ToString::to_string);
        branches.push(BranchRef {
            name,
            is_head,
            upstream,
        });
    }
    Ok(branches)
}

/// Returns the merge-base OID hex of `a` and `b`.
///
/// # Errors
/// Propagates CLI failures.
pub fn merge_base(repo: &Path, a: &str, b: &str) -> Result<String, GitError> {
    let cli = GitCli::new(repo)?;
    cli.run(&["merge-base", a, b])
}

/// Commits reachable from `tip` but not from `base` (`git log base..tip`).
///
/// # Errors
/// Propagates CLI failures.
pub fn commits_not_in(
    repo: &Path,
    tip: &str,
    base: &str,
    limit: usize,
) -> Result<Vec<CommitInfo>, GitError> {
    let cli = GitCli::new(repo)?;
    let range = format!("{base}..{tip}");
    let max = format!("-{limit}");
    let out = cli.run(&["log", &max, "--format=%H%x09%s%x09%an%x09%at", &range])?;
    Ok(parse_log(&out))
}

/// Ahead/behind counts of `tip` relative to `upstream` (`rev-list --left-right --count`).
///
/// # Errors
/// Propagates CLI failures.
pub fn ahead_behind(repo: &Path, tip: &str, upstream: &str) -> Result<(u32, u32), GitError> {
    let cli = GitCli::new(repo)?;
    let out = cli.run(&[
        "rev-list",
        "--left-right",
        "--count",
        &format!("{tip}...{upstream}"),
    ])?;
    // Format: "<left>\t<right>" where left = tip not in upstream (ahead),
    // right = upstream not in tip (behind).
    let mut parts = out.split_whitespace();
    let ahead = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let behind = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    Ok((ahead, behind))
}

/// Recent branch names from checkout reflog (most recent first, deduped).
///
/// # Errors
/// Propagates CLI failures.
pub fn recent_branches(repo: &Path, limit: usize) -> Result<Vec<String>, GitError> {
    let cli = GitCli::new(repo)?;
    // Prefer checkout messages; fall back to any reflog entry that names a branch.
    let out = cli.run(&["reflog", "--format=%gs", "-n", "100"])?;
    let mut recent = Vec::new();
    for line in out.lines() {
        if let Some(name) = parse_checkout_branch(line) {
            if !recent.iter().any(|n| n == &name) {
                recent.push(name);
            }
        }
        if recent.len() >= limit {
            break;
        }
    }
    Ok(recent)
}

fn parse_checkout_branch(reflog_summary: &str) -> Option<String> {
    // "checkout: moving from main to feature"
    let lower = reflog_summary.to_ascii_lowercase();
    if !lower.starts_with("checkout: moving from ") {
        return None;
    }
    let to = reflog_summary.rsplit(" to ").next()?.trim();
    if to.is_empty() || to == "HEAD" {
        return None;
    }
    Some(to.to_string())
}

/// Last commit on `branch` (summary / author / time).
///
/// # Errors
/// Propagates CLI failures.
pub fn branch_last_commit(repo: &Path, branch: &str) -> Result<Option<CommitInfo>, GitError> {
    let cli = GitCli::new(repo)?;
    let out = cli.run(&["log", "-1", "--format=%H%x09%s%x09%an%x09%at", branch])?;
    Ok(parse_log(&out).into_iter().next())
}

/// Sets `branch` to track `upstream` (`git branch -u upstream branch`).
///
/// # Errors
/// Propagates CLI failures.
pub fn set_upstream(repo: &Path, branch: &str, upstream: &str) -> Result<(), GitError> {
    let cli = GitCli::new(repo)?;
    cli.run(&["branch", "-u", upstream, branch])?;
    Ok(())
}

fn parse_log(out: &str) -> Vec<CommitInfo> {
    let mut commits = Vec::new();
    for line in out.lines() {
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(4, '\t');
        let oid = parts.next().unwrap_or("").to_string();
        let summary = parts.next().unwrap_or("").to_string();
        let author = parts.next().unwrap_or("").to_string();
        let time = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        commits.push(CommitInfo {
            oid,
            summary,
            author,
            time,
        });
    }
    commits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::fixture::TempRepo;

    fn diverged_repo() -> TempRepo {
        let repo = TempRepo::init();
        repo.write("a.txt", "base\n");
        repo.stage("a.txt");
        repo.commit("base");
        repo.run(&["checkout", "-b", "feature"]);
        repo.write("a.txt", "base\nfeature\n");
        repo.stage("a.txt");
        repo.commit("on feature");
        repo.run(&["checkout", "main"]);
        repo.write("a.txt", "base\nmain\n");
        repo.stage("a.txt");
        repo.commit("on main");
        repo
    }

    #[test]
    fn merge_base_and_side_commits() {
        let repo = diverged_repo();
        let base = merge_base(repo.path(), "main", "feature").expect("merge-base");
        assert_eq!(base.len(), 40);

        let on_main = commits_not_in(repo.path(), "main", &base, 50).expect("main side");
        let on_feature = commits_not_in(repo.path(), "feature", &base, 50).expect("feature side");
        assert_eq!(on_main.len(), 1);
        assert_eq!(on_feature.len(), 1);
        assert!(on_main[0].summary.contains("on main"));
        assert!(on_feature[0].summary.contains("on feature"));
    }

    #[test]
    fn ahead_behind_counts_divergence() {
        let repo = diverged_repo();
        let (ahead, behind) = ahead_behind(repo.path(), "main", "feature").expect("ab");
        assert_eq!(ahead, 1);
        assert_eq!(behind, 1);
    }

    #[test]
    fn list_branches_marks_head() {
        let repo = diverged_repo();
        let branches = list_branches(repo.path()).expect("branches");
        assert!(branches.iter().any(|b| b.name == "main" && b.is_head));
        assert!(branches.iter().any(|b| b.name == "feature" && !b.is_head));
    }

    #[test]
    fn recent_branches_from_reflog_checkout_order() {
        let repo = diverged_repo();
        // diverged_repo ends on main after visiting feature.
        let recent = recent_branches(repo.path(), 5).expect("recent");
        assert!(
            recent.first().is_some_and(|n| n == "main"),
            "expected main first, got {recent:?}"
        );
        assert!(recent.iter().any(|n| n == "feature"));
    }

    #[test]
    fn branch_last_commit_reads_tip_summary() {
        let repo = diverged_repo();
        let tip = branch_last_commit(repo.path(), "feature")
            .expect("last")
            .expect("some");
        assert!(tip.summary.contains("on feature"));
    }

    #[test]
    fn set_upstream_writes_tracking_ref() {
        let remote = TempRepo::init();
        remote.write("r.txt", "remote\n");
        remote.stage("r.txt");
        remote.commit("remote tip");

        let local = TempRepo::init();
        local.write("l.txt", "local\n");
        local.stage("l.txt");
        local.commit("local tip");
        local.run(&[
            "remote",
            "add",
            "origin",
            remote.path().to_str().expect("utf8 path"),
        ]);
        local.run(&["fetch", "origin"]);

        set_upstream(local.path(), "main", "origin/main").expect("set upstream");
        let branches = list_branches(local.path()).expect("list");
        let main = branches.iter().find(|b| b.name == "main").expect("main");
        assert_eq!(main.upstream.as_deref(), Some("origin/main"));
    }
}
