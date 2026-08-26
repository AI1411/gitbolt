//! Remote fetch / pull / push via git CLI (issue #19).
//!
//! Authentication is delegated to the user's git/ssh environment
//! (credential helpers, ssh-agent). See `docs/design/09-git-backend.md`.

use std::path::Path;

use super::cli::GitCli;
use super::error::GitError;

/// Returns the configured URL for the `origin` remote.
///
/// # Errors
/// Propagates CLI failures (including missing `origin`).
pub fn origin_url(repo: &Path) -> Result<String, GitError> {
    let cli = GitCli::new(repo)?;
    cli.run(&["remote", "get-url", "origin"])
}

/// Fetches from all remotes (`git fetch --all --prune`).
///
/// # Errors
/// Propagates CLI / auth failures.
pub fn fetch(repo: &Path) -> Result<(), GitError> {
    let cli = GitCli::new(repo)?;
    cli.run(&["fetch", "--all", "--prune"])?;
    Ok(())
}

/// Pulls the current branch from its upstream (`git pull --ff-only`).
///
/// # Errors
/// Propagates CLI / auth / conflict failures.
pub fn pull(repo: &Path) -> Result<(), GitError> {
    let cli = GitCli::new(repo)?;
    cli.run(&["pull", "--ff-only"])?;
    Ok(())
}

/// Pushes the current branch to its upstream (`git push --porcelain`).
///
/// # Errors
/// Propagates CLI / auth failures.
pub fn push(repo: &Path) -> Result<(), GitError> {
    let cli = GitCli::new(repo)?;
    cli.run(&["push", "--porcelain"])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::branch::list_branches;
    use crate::git::fixture::TempRepo;

    #[test]
    fn fetch_updates_remote_tracking_refs() {
        let remote = TempRepo::init();
        remote.write("a.txt", "one\n");
        remote.stage("a.txt");
        remote.commit("one");

        let local = TempRepo::init();
        local.write("b.txt", "local\n");
        local.stage("b.txt");
        local.commit("local");
        local.run(&[
            "remote",
            "add",
            "origin",
            remote.path().to_str().expect("utf8"),
        ]);

        fetch(local.path()).expect("fetch");
        let branches = list_branches(local.path()).expect("list");
        assert!(branches
            .iter()
            .any(|b| b.is_remote && b.name == "origin/main"));
    }

    #[test]
    fn push_and_pull_roundtrip_on_tracking_branch() {
        let bare_dir = tempfile::tempdir().expect("tmpdir");
        let bare_path = bare_dir.path().to_path_buf();
        std::process::Command::new("git")
            .args(["init", "--bare"])
            .arg(&bare_path)
            .output()
            .expect("bare init");

        let alice = TempRepo::init();
        alice.write("f.txt", "v1\n");
        alice.stage("f.txt");
        alice.commit("v1");
        alice.run(&["remote", "add", "origin", bare_path.to_str().expect("utf8")]);
        alice.run(&["push", "-u", "origin", "main"]);

        let bob = TempRepo::init();
        bob.run(&["remote", "add", "origin", bare_path.to_str().expect("utf8")]);
        bob.run(&["fetch", "origin"]);
        bob.run(&["checkout", "-B", "main", "origin/main"]);
        bob.run(&["branch", "-u", "origin/main"]);

        alice.write("f.txt", "v2\n");
        alice.stage("f.txt");
        alice.commit("v2");
        push(alice.path()).expect("push");

        pull(bob.path()).expect("pull");
        let contents = std::fs::read_to_string(bob.path().join("f.txt")).expect("read");
        assert_eq!(contents, "v2\n");
    }
}
