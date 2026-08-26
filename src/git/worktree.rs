//! Worktree list / create / remove via `git worktree` (issue #20).

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

/// Default path: `<repo-parent>/<repo-name>-worktrees/<branch-with-slashes-as-dashes>`.
#[must_use]
pub fn default_worktree_path(repo: &Path, branch: &str) -> PathBuf {
    let repo_name = repo
        .file_name()
        .map_or_else(|| "repo".into(), |s| s.to_string_lossy().into_owned());
    let parent = repo.parent().unwrap_or(repo);
    let safe: String = branch
        .chars()
        .map(|c| if c == '/' || c == '\\' { '-' } else { c })
        .collect();
    parent.join(format!("{repo_name}-worktrees")).join(safe)
}

/// Creates a worktree for an existing `branch` at `path`.
///
/// # Errors
/// Path collision, branch already checked out, or CLI failure.
pub fn create_worktree(repo: &Path, branch: &str, path: &Path) -> Result<WorktreeRef, GitError> {
    let branch = branch.trim();
    if branch.is_empty() {
        return Err(GitError::Backend("branch name is empty".into()));
    }
    if path.exists() {
        return Err(GitError::Backend(format!(
            "worktree path already exists: {}",
            path.display()
        )));
    }
    let existing = list_worktrees(repo)?;
    if existing.iter().any(|w| w.branch.as_deref() == Some(branch)) {
        return Err(GitError::Backend(format!(
            "branch '{branch}' is already checked out in another worktree"
        )));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(GitError::from)?;
    }
    let cli = GitCli::new(repo)?;
    let path_str = path
        .to_str()
        .ok_or_else(|| GitError::Backend("worktree path is not valid UTF-8".into()))?;
    cli.run(&["worktree", "add", path_str, branch])?;
    let trees = list_worktrees(repo)?;
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    trees
        .into_iter()
        .find(|w| {
            w.path == path
                || w.path.canonicalize().ok().as_ref() == Some(&canonical)
                || w.branch.as_deref() == Some(branch)
        })
        .ok_or_else(|| GitError::Backend("worktree created but not listed".into()))
}

/// Removes a linked worktree (not the primary).
///
/// # Errors
/// Refuses primary worktree; propagates CLI failures.
pub fn remove_worktree(repo: &Path, path: &Path) -> Result<(), GitError> {
    let trees = list_worktrees(repo)?;
    let target = trees
        .iter()
        .find(|w| w.path == path)
        .ok_or_else(|| GitError::Backend(format!("worktree not found: {}", path.display())))?;
    if target.is_primary {
        return Err(GitError::Backend(
            "cannot remove the primary worktree".into(),
        ));
    }
    let cli = GitCli::new(repo)?;
    let path_str = path
        .to_str()
        .ok_or_else(|| GitError::Backend("worktree path is not valid UTF-8".into()))?;
    cli.run(&["worktree", "remove", "--force", path_str])?;
    Ok(())
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
            let canonical_hint = primary_hint
                .canonicalize()
                .unwrap_or_else(|_| primary_hint.to_path_buf());
            let canonical_p = p.canonicalize().unwrap_or_else(|_| p.clone());
            let is_primary = canonical_p == canonical_hint || result.is_empty();
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
    // Recompute primary against hint so linked trees are never marked primary by order alone.
    if let Ok(hint) = primary_hint.canonicalize() {
        for w in &mut result {
            w.is_primary = w.path.canonicalize().ok().as_ref() == Some(&hint);
        }
        if !result.iter().any(|w| w.is_primary) {
            if let Some(first) = result.first_mut() {
                first.is_primary = true;
            }
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

    #[test]
    fn create_and_remove_linked_worktree() {
        let repo = TempRepo::init();
        repo.write("a.txt", "x\n");
        repo.stage("a.txt");
        repo.commit("init");
        repo.run(&["branch", "feature"]);

        let path = default_worktree_path(repo.path(), "feature");
        let created = create_worktree(repo.path(), "feature", &path).expect("create");
        assert!(!created.is_primary);
        assert_eq!(created.branch.as_deref(), Some("feature"));
        assert!(path.exists());

        let trees = list_worktrees(repo.path()).expect("list");
        assert_eq!(trees.len(), 2);

        remove_worktree(repo.path(), &path).expect("remove");
        assert!(!path.exists() || !path.join(".git").exists());
        let trees = list_worktrees(repo.path()).expect("list2");
        assert_eq!(trees.len(), 1);
    }

    #[test]
    fn remove_refuses_primary() {
        let repo = TempRepo::init();
        repo.write("a.txt", "x\n");
        repo.stage("a.txt");
        repo.commit("init");
        let err = remove_worktree(repo.path(), repo.path()).expect_err("primary");
        assert!(matches!(err, GitError::Backend(_)));
    }

    #[test]
    fn default_path_sanitizes_branch() {
        let repo = Path::new("/tmp/app");
        let p = default_worktree_path(repo, "feature/auth");
        assert!(p.ends_with("app-worktrees/feature-auth"));
    }
}
