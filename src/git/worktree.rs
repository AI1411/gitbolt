//! Worktree listing via `git worktree list --porcelain` (issue #11 / #20).

use std::path::{Path, PathBuf};

use super::cli::GitCli;
use super::error::GitError;
use super::service::WorktreeRef;

/// Lists worktrees in `repo`.
///
/// # Errors
/// Propagates CLI failures.
pub fn list_worktrees(repo: &Path) -> Result<Vec<WorktreeRef>, GitError> {
    let cli = GitCli::new(repo)?;
    let out = cli.run(&["worktree", "list", "--porcelain"])?;
    Ok(parse_porcelain(&out, repo))
}

fn parse_porcelain(out: &str, primary_hint: &Path) -> Vec<WorktreeRef> {
    let mut result = Vec::new();
    let mut path: Option<PathBuf> = None;
    let mut branch: Option<String> = None;
    let mut bare = false;

    let flush = |path: &mut Option<PathBuf>,
                 branch: &mut Option<String>,
                 bare: &mut bool,
                 result: &mut Vec<WorktreeRef>,
                 primary_hint: &Path| {
        if let Some(p) = path.take() {
            let is_primary = p == primary_hint || result.is_empty();
            result.push(WorktreeRef {
                path: p,
                branch: branch.take(),
                is_primary,
            });
            *bare = false;
        }
    };

    for line in out.lines() {
        if line.is_empty() {
            flush(&mut path, &mut branch, &mut bare, &mut result, primary_hint);
            continue;
        }
        if let Some(rest) = line.strip_prefix("worktree ") {
            flush(&mut path, &mut branch, &mut bare, &mut result, primary_hint);
            path = Some(PathBuf::from(rest));
        } else if let Some(rest) = line.strip_prefix("branch ") {
            // refs/heads/name
            let name = rest.strip_prefix("refs/heads/").unwrap_or(rest).to_string();
            branch = Some(name);
        } else if line == "bare" {
            bare = true;
            let _ = bare;
        } else if line == "detached" {
            branch = None;
        }
    }
    flush(&mut path, &mut branch, &mut bare, &mut result, primary_hint);

    let has_primary = result.iter().any(|w| w.is_primary);
    if !has_primary {
        if let Some(first) = result.first_mut() {
            first.is_primary = true;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::fixture::TempRepo;

    #[test]
    fn list_includes_primary_worktree() {
        let repo = TempRepo::init();
        repo.write("a.txt", "x\n");
        repo.stage("a.txt");
        repo.commit("init");
        let trees = list_worktrees(repo.path()).expect("list");
        assert_eq!(trees.len(), 1);
        assert!(trees[0].is_primary);
        assert_eq!(trees[0].branch.as_deref(), Some("main"));
    }
}
