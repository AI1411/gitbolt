//! File and line staging via the git CLI (`git add` / `git apply --cached`).

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use super::cli::GitCli;
use super::diff::unified_diff;
use super::error::GitError;
use super::patch::build_partial_patch;

/// Stages an entire file (`git add -- path`).
///
/// # Errors
/// Propagates CLI failures.
pub fn stage_file(repo: &Path, path: &Path) -> Result<(), GitError> {
    let cli = GitCli::new(repo)?;
    let path_str = path.to_string_lossy();
    cli.run(&["add", "--", path_str.as_ref()])?;
    Ok(())
}

/// Unstages an entire file (`git restore --staged -- path`).
///
/// # Errors
/// Propagates CLI failures.
pub fn unstage_file(repo: &Path, path: &Path) -> Result<(), GitError> {
    let cli = GitCli::new(repo)?;
    let path_str = path.to_string_lossy();
    cli.run(&["restore", "--staged", "--", path_str.as_ref()])?;
    Ok(())
}

/// Stages all changes (`git add -A`).
///
/// # Errors
/// Propagates CLI failures.
pub fn stage_all(repo: &Path) -> Result<(), GitError> {
    let cli = GitCli::new(repo)?;
    cli.run(&["add", "-A"])?;
    Ok(())
}

/// Unstages everything (`git restore --staged :/`).
///
/// # Errors
/// Propagates CLI failures.
pub fn unstage_all(repo: &Path) -> Result<(), GitError> {
    let cli = GitCli::new(repo)?;
    cli.run(&["restore", "--staged", ":/"])?;
    Ok(())
}

/// Stages only the selected lines of an unstaged (or staged) file diff.
///
/// `selected` indexes match the body lines of `git diff` / `git diff --cached`
/// (see [`super::patch`]). When `from_staged` is true, the reverse patch is
/// applied to unstage those lines from the index.
///
/// # Errors
/// Propagates patch build or `git apply` failures.
pub fn stage_lines(
    repo: &Path,
    path: &Path,
    from_staged: bool,
    selected: &[usize],
) -> Result<(), GitError> {
    let diff = unified_diff(repo, path, from_staged)?;
    if diff.text.trim().is_empty() {
        return Err(GitError::Backend("差分がありません".into()));
    }
    let partial = build_partial_patch(&diff.text, selected)?;
    apply_cached(repo, &partial, from_staged)
}

fn apply_cached(repo: &Path, patch: &str, reverse: bool) -> Result<(), GitError> {
    let git = GitCli::discover()?;
    let repo_str = repo.to_string_lossy();
    let mut cmd = Command::new(git);
    cmd.arg("-C")
        .arg(repo_str.as_ref())
        .arg("apply")
        .arg("--cached");
    if reverse {
        cmd.arg("--reverse");
    }
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            GitError::GitBinaryNotFound
        } else {
            GitError::Io(e.to_string())
        }
    })?;

    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| GitError::Io("failed to open git apply stdin".into()))?;
        stdin
            .write_all(patch.as_bytes())
            .map_err(|e| GitError::Io(e.to_string()))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| GitError::Io(e.to_string()))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    Err(GitError::Backend(stderr))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::fixture::TempRepo;
    use crate::git::{ChangeStatus, GitService, GixService};

    #[test]
    fn stage_single_added_line_leaves_other_additions_unstaged() {
        let repo = TempRepo::init();
        repo.write("f.txt", "a\n");
        repo.stage("f.txt");
        repo.commit("initial");
        repo.write("f.txt", "a\nb\nc\n");

        let diff = unified_diff(repo.path(), Path::new("f.txt"), false).expect("diff");
        // Find the body index of "+b"
        let body = diff
            .text
            .lines()
            .skip_while(|l| !l.starts_with("+++ "))
            .skip(1)
            .collect::<Vec<_>>();
        let b_idx = body
            .iter()
            .position(|l| *l == "+b")
            .expect("+b line in diff");

        stage_lines(repo.path(), Path::new("f.txt"), false, &[b_idx]).expect("stage line");

        let status = GixService::open(repo.path())
            .expect("open")
            .status()
            .expect("status");

        assert!(
            status
                .staged
                .iter()
                .any(|f| f.path.ends_with("f.txt") && f.status == ChangeStatus::Modified),
            "expected staged modification, got {:?}",
            status.staged
        );
        assert!(
            status
                .unstaged
                .iter()
                .any(|f| f.path.ends_with("f.txt") && f.status == ChangeStatus::Modified),
            "expected remaining unstaged modification, got {:?}",
            status.unstaged
        );

        // Index should contain a and b but not c.
        let cli = GitCli::new(repo.path()).expect("cli");
        let indexed = cli.run(&["show", ":f.txt"]).expect("show index");
        assert_eq!(indexed, "a\nb");
        let work = std::fs::read_to_string(repo.path().join("f.txt")).expect("read");
        assert_eq!(work, "a\nb\nc\n");
    }

    #[test]
    fn stage_all_and_unstage_all() {
        let repo = TempRepo::init();
        repo.write("a.txt", "a\n");
        repo.stage("a.txt");
        repo.commit("init");
        repo.write("a.txt", "a\nb\n");
        repo.write("b.txt", "new\n");

        stage_all(repo.path()).expect("stage all");
        let status = GixService::open(repo.path())
            .expect("open")
            .status()
            .expect("status");
        assert!(status.unstaged.is_empty());
        assert!(status.untracked.is_empty());
        assert!(!status.staged.is_empty());

        unstage_all(repo.path()).expect("unstage all");
        let status = GixService::open(repo.path())
            .expect("open")
            .status()
            .expect("status");
        assert!(status.staged.is_empty());
    }
}
