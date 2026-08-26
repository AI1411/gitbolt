//! Branch listing, create / checkout / delete, and divergence helpers.

use std::collections::HashSet;
use std::path::Path;

use super::cli::GitCli;
use super::error::GitError;
use super::service::{BranchRef, CommitInfo, Head};

/// Lists local and remote-tracking branches.
///
/// # Errors
/// Propagates CLI failures.
pub fn list_branches(repo: &Path) -> Result<Vec<BranchRef>, GitError> {
    let cli = GitCli::new(repo)?;
    let out = cli.run(&[
        "for-each-ref",
        "--format=%(refname)%09%(refname:short)%09%(HEAD)%09%(upstream:short)",
        "refs/heads",
        "refs/remotes",
    ])?;
    let mut branches = Vec::new();
    for line in out.lines() {
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split('\t');
        let full = parts.next().unwrap_or("");
        let name = parts.next().unwrap_or("").to_string();
        let is_head = parts.next() == Some("*");
        let upstream = parts
            .next()
            .filter(|s| !s.is_empty())
            .map(std::string::ToString::to_string);
        let is_remote = full.starts_with("refs/remotes/");
        // Skip remote symbolic HEAD (origin/HEAD).
        if is_remote && (name.ends_with("/HEAD") || name == "HEAD") {
            continue;
        }
        branches.push(BranchRef {
            name,
            is_head,
            upstream,
            is_remote,
        });
    }
    Ok(branches)
}

/// Creates a new local branch at HEAD without switching.
///
/// # Errors
/// Propagates CLI failures or rejects empty / invalid names.
pub fn create_branch(repo: &Path, name: &str) -> Result<(), GitError> {
    let name = validate_branch_name(name)?;
    let cli = GitCli::new(repo)?;
    cli.run(&["branch", name])?;
    Ok(())
}

/// Preflight for checkout: dirty paths that also differ between HEAD and `target`
/// are treated as conflicts.
///
/// # Errors
/// Returns [`GitError::Conflict`] when a switch would overwrite local work.
pub fn checkout_preflight(repo: &Path, target: &str) -> Result<(), GitError> {
    let cli = GitCli::new(repo)?;
    let porcelain = cli.run(&["status", "--porcelain", "-uall"])?;
    if porcelain.trim().is_empty() {
        return Ok(());
    }

    let dirty = dirty_paths_from_porcelain(&porcelain);
    if dirty.is_empty() {
        return Ok(());
    }

    let delta = cli.run(&["diff", "--name-only", "HEAD", target])?;
    let mut conflicting: Vec<String> = delta
        .lines()
        .map(str::trim)
        .filter(|p| !p.is_empty() && dirty.contains(*p))
        .map(std::string::ToString::to_string)
        .collect();

    // Untracked paths that already exist in the target tree would be overwritten.
    let tree = cli.run(&["ls-tree", "-r", "--name-only", target])?;
    let in_target: HashSet<&str> = tree.lines().filter(|l| !l.is_empty()).collect();
    for path in &dirty {
        if in_target.contains(path.as_str()) && !conflicting.iter().any(|c| c == path) {
            // Only untracked/new paths that aren't already in the HEAD..target delta
            // but exist at the destination tip can still block checkout.
            let is_untracked = porcelain.lines().any(|line| {
                line.len() >= 3 && &line[..2] == "??" && line[3..].trim() == path.as_str()
            });
            if is_untracked {
                conflicting.push(path.clone());
            }
        }
    }

    if conflicting.is_empty() {
        return Ok(());
    }
    conflicting.sort();
    conflicting.dedup();
    Err(GitError::Conflict(format!(
        "local changes would be overwritten by checkout: {}",
        conflicting.join(", ")
    )))
}

/// Switches to `name` (`git switch`).
///
/// Local branch names (including those with `/`, e.g. `cursor/foo`) are
/// switched as-is. Remote-tracking names (`origin/feature/x`) create or
/// reuse a local branch with the remote prefix stripped.
///
/// # Errors
/// Propagates CLI failures after preflight.
pub fn checkout(repo: &Path, name: &str) -> Result<Head, GitError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(GitError::Backend("branch name is empty".into()));
    }
    checkout_preflight(repo, name)?;
    let cli = GitCli::new(repo)?;
    let branches = list_branches(repo)?;

    if branches.iter().any(|b| !b.is_remote && b.name == name) {
        cli.run(&["switch", "--", name])?;
        return read_head(repo);
    }

    if branches.iter().any(|b| b.is_remote && b.name == name) {
        let short = local_name_for_remote(name);
        if short.is_empty() {
            return Err(GitError::Backend(format!(
                "cannot derive a local name from remote ref {name}"
            )));
        }
        if branches.iter().any(|b| !b.is_remote && b.name == short) {
            cli.run(&["switch", "--", &short])?;
        } else {
            cli.run(&["switch", "-c", &short, "--track", name])?;
        }
        return read_head(repo);
    }

    cli.run(&["switch", "--", name])?;
    read_head(repo)
}

/// `origin/feature/nested` → `feature/nested`.
fn local_name_for_remote(remote_short: &str) -> String {
    remote_short
        .split_once('/')
        .map(|(_, rest)| rest.to_string())
        .filter(|rest| !rest.is_empty())
        .unwrap_or_else(|| remote_short.to_string())
}

/// Lists local branch names already merged into `base` (`git branch --merged`).
///
/// # Errors
/// Propagates CLI failures.
pub fn merged_into(repo: &Path, base: &str) -> Result<Vec<String>, GitError> {
    let base = validate_branch_name(base)?;
    let cli = GitCli::new(repo)?;
    let out = cli.run(&["branch", "--merged", base, "--format=%(refname:short)"])?;
    Ok(out
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && *l != base)
        .map(str::to_string)
        .collect())
}

/// Returns `true` when every commit unique to `branch` has a patch-equivalent
/// commit already on `base` (`git cherry`), i.e. squash / cherry-pick merged.
///
/// Branches that are true ancestors of `base` are reported by [`merged_into`]
/// instead; this helper returns `false` when `git cherry` prints nothing.
///
/// # Errors
/// Propagates CLI failures.
pub fn is_squash_merged(repo: &Path, branch: &str, base: &str) -> Result<bool, GitError> {
    let branch = validate_branch_name(branch)?;
    let base = validate_branch_name(base)?;
    let cli = GitCli::new(repo)?;
    let out = cli.run(&["cherry", base, branch])?;
    let mut saw = false;
    for line in out.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        saw = true;
        // `+` = unique patch still missing from base; `-` = equivalent exists.
        if line.starts_with('+') {
            return Ok(false);
        }
    }
    Ok(saw)
}

/// Deletes a local branch with `git branch -d` or `-D` when `force` is true.
///
/// # Errors
/// Propagates CLI failures; refuses deleting the current branch.
pub fn delete_branch(repo: &Path, name: &str, force: bool) -> Result<(), GitError> {
    let name = validate_branch_name(name)?;
    let head = read_head(repo)?;
    if head.branch.as_deref() == Some(name) {
        return Err(GitError::Backend(
            "cannot delete the branch currently checked out".into(),
        ));
    }
    let cli = GitCli::new(repo)?;
    let flag = if force { "-D" } else { "-d" };
    cli.run(&["branch", flag, name])?;
    Ok(())
}

/// Returns the current HEAD summary via porcelain.
///
/// # Errors
/// Propagates CLI failures.
pub fn read_head(repo: &Path) -> Result<Head, GitError> {
    let cli = GitCli::new(repo)?;
    let oid = cli.run(&["rev-parse", "HEAD"])?;
    let symbolic = cli.run(&["symbolic-ref", "-q", "--short", "HEAD"]);
    match symbolic {
        Ok(branch) if !branch.is_empty() => Ok(Head {
            branch: Some(branch),
            oid: Some(oid),
            detached: false,
        }),
        _ => Ok(Head {
            branch: None,
            oid: Some(oid),
            detached: true,
        }),
    }
}

fn validate_branch_name(name: &str) -> Result<&str, GitError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(GitError::Backend("branch name is empty".into()));
    }
    if name.contains(char::is_whitespace)
        || name.contains("..")
        || name.starts_with('-')
        || name.contains('\0')
    {
        return Err(GitError::Backend(format!("invalid branch name: {name}")));
    }
    Ok(name)
}

fn dirty_paths_from_porcelain(porcelain: &str) -> HashSet<String> {
    let mut paths = HashSet::new();
    for line in porcelain.lines() {
        if line.len() < 4 {
            continue;
        }
        // XY<space>path  or  XY<space>old -> new
        let rest = line[3..].trim();
        if let Some((from, to)) = rest.split_once(" -> ") {
            paths.insert(from.to_string());
            paths.insert(to.to_string());
        } else if !rest.is_empty() {
            paths.insert(rest.to_string());
        }
    }
    paths
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
        assert!(branches
            .iter()
            .any(|b| b.name == "main" && b.is_head && !b.is_remote));
        assert!(branches
            .iter()
            .any(|b| b.name == "feature" && !b.is_head && !b.is_remote));
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
        assert!(branches
            .iter()
            .any(|b| b.is_remote && b.name == "origin/main"));
    }

    #[test]
    fn create_checkout_delete_roundtrip() {
        let repo = TempRepo::init();
        repo.write("f.txt", "one\n");
        repo.stage("f.txt");
        repo.commit("first");

        create_branch(repo.path(), "topic").expect("create");
        let head = checkout(repo.path(), "topic").expect("checkout");
        assert_eq!(head.branch.as_deref(), Some("topic"));

        checkout(repo.path(), "main").expect("back");
        delete_branch(repo.path(), "topic", false).expect("delete");
        let names: Vec<_> = list_branches(repo.path())
            .expect("list")
            .into_iter()
            .map(|b| b.name)
            .collect();
        assert!(!names.iter().any(|n| n == "topic"));
    }

    #[test]
    fn checkout_keeps_local_branch_with_slash_in_name() {
        let repo = TempRepo::init();
        repo.write("f.txt", "one\n");
        repo.stage("f.txt");
        repo.commit("first");

        repo.run(&["switch", "-c", "cursor/feature-auth"]);
        repo.write("f.txt", "two\n");
        repo.stage("f.txt");
        repo.commit("on slash branch");
        repo.run(&["switch", "main"]);

        let head = checkout(repo.path(), "cursor/feature-auth").expect("checkout");
        assert_eq!(
            head.branch.as_deref(),
            Some("cursor/feature-auth"),
            "must switch to the existing local branch, not invent a last-segment name"
        );
        let names: Vec<_> = list_branches(repo.path())
            .expect("list")
            .into_iter()
            .filter(|b| !b.is_remote)
            .map(|b| b.name)
            .collect();
        assert!(
            !names.iter().any(|n| n == "feature-auth"),
            "must not create a shadow branch from the last path segment: {names:?}"
        );
    }

    #[test]
    fn checkout_remote_tracking_strips_only_the_remote_prefix() {
        let remote = TempRepo::init();
        remote.write("f.txt", "one\n");
        remote.stage("f.txt");
        remote.commit("first");
        remote.run(&["switch", "-c", "feature/nested"]);
        remote.write("f.txt", "two\n");
        remote.stage("f.txt");
        remote.commit("nested");
        remote.run(&["switch", "main"]);

        let local = TempRepo::init();
        local.run(&[
            "remote",
            "add",
            "origin",
            remote.path().to_str().expect("utf8 path"),
        ]);
        local.run(&["fetch", "origin"]);

        let head = checkout(local.path(), "origin/feature/nested").expect("track");
        assert_eq!(head.branch.as_deref(), Some("feature/nested"));
    }

    #[test]
    fn detects_squash_merged_via_cherry() {
        let repo = TempRepo::init();
        repo.write("a.txt", "base\n");
        repo.stage("a.txt");
        repo.commit("base");

        create_branch(repo.path(), "feature").expect("create");
        checkout(repo.path(), "feature").expect("co");
        repo.write("a.txt", "base\nfeature\n");
        repo.stage("a.txt");
        repo.commit("feature work");

        checkout(repo.path(), "main").expect("back");
        // Squash: apply same tree change as a single commit on main.
        repo.write("a.txt", "base\nfeature\n");
        repo.stage("a.txt");
        repo.commit("squash feature (#1)");

        assert!(
            is_squash_merged(repo.path(), "feature", "main").expect("cherry"),
            "feature patches should be equivalent on main"
        );
        assert!(
            !merged_into(repo.path(), "main")
                .expect("merged")
                .iter()
                .any(|n| n == "feature"),
            "regular merge detection should miss squash"
        );
    }

    #[test]
    fn checkout_preflight_blocks_overlapping_dirty_file() {
        let repo = diverged_repo();
        // On main; dirty a.txt which also differs on feature.
        repo.write("a.txt", "base\nmain\nlocal edit\n");
        let err = checkout_preflight(repo.path(), "feature").expect_err("conflict");
        assert!(matches!(err, GitError::Conflict(_)));
    }

    #[test]
    fn checkout_preflight_allows_unrelated_dirty_file() {
        let repo = diverged_repo();
        repo.write("only-local.txt", "scratch\n");
        checkout_preflight(repo.path(), "feature").expect("ok");
        let head = checkout(repo.path(), "feature").expect("switch");
        assert_eq!(head.branch.as_deref(), Some("feature"));
    }

    #[test]
    fn force_delete_removes_unmerged_branch() {
        let repo = TempRepo::init();
        repo.write("f.txt", "one\n");
        repo.stage("f.txt");
        repo.commit("first");
        repo.run(&["switch", "-c", "orphan"]);
        repo.write("f.txt", "orphan\n");
        repo.stage("f.txt");
        repo.commit("orphan work");
        repo.run(&["switch", "main"]);

        let err = delete_branch(repo.path(), "orphan", false).expect_err("safe");
        assert!(matches!(err, GitError::Backend(_)));

        delete_branch(repo.path(), "orphan", true).expect("force");
        let names: Vec<_> = list_branches(repo.path())
            .expect("list")
            .into_iter()
            .filter(|b| !b.is_remote)
            .map(|b| b.name)
            .collect();
        assert!(!names.iter().any(|n| n == "orphan"));
    }

    #[test]
    fn delete_refuses_current_branch() {
        let repo = TempRepo::init();
        repo.write("f.txt", "x\n");
        repo.stage("f.txt");
        repo.commit("c");
        let err = delete_branch(repo.path(), "main", false).expect_err("current");
        assert!(matches!(err, GitError::Backend(_)));
    }
}
