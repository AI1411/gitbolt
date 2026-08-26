//! Stash operations via `git stash` CLI (issue #24).
//!
//! apply / drop are CLI-only per `docs/design/09-git-backend.md`.

use std::path::Path;

use super::cli::GitCli;
use super::error::GitError;
use super::service::StashEntry;

fn stash_ref(index: usize) -> String {
    format!("stash@{{{index}}}")
}

/// Lists stash entries (newest first, index 0 = top).
///
/// # Errors
/// Propagates CLI failures.
pub fn list_stashes(repo: &Path) -> Result<Vec<StashEntry>, GitError> {
    let cli = GitCli::new(repo)?;
    let out = cli.run(&["stash", "list", "--format=%gd%x09%gs"])?;
    Ok(parse_list(&out))
}

/// Saves working-tree changes to the stash.
///
/// # Errors
/// Propagates CLI failures (including nothing to stash).
pub fn stash_push(repo: &Path, message: Option<&str>) -> Result<(), GitError> {
    let cli = GitCli::new(repo)?;
    match message {
        Some(msg) if !msg.trim().is_empty() => {
            cli.run(&["stash", "push", "-m", msg.trim()])?;
        }
        _ => {
            cli.run(&["stash", "push"])?;
        }
    }
    Ok(())
}

/// Applies a stash without removing it.
///
/// # Errors
/// Propagates CLI failures including conflicts.
pub fn stash_apply(repo: &Path, index: usize) -> Result<(), GitError> {
    let cli = GitCli::new(repo)?;
    let reference = stash_ref(index);
    cli.run(&["stash", "apply", &reference])?;
    Ok(())
}

/// Applies a stash and removes it from the list.
///
/// # Errors
/// Propagates CLI failures including conflicts.
pub fn stash_pop(repo: &Path, index: usize) -> Result<(), GitError> {
    let cli = GitCli::new(repo)?;
    let reference = stash_ref(index);
    cli.run(&["stash", "pop", &reference])?;
    Ok(())
}

/// Drops a stash entry.
///
/// # Errors
/// Propagates CLI failures.
pub fn stash_drop(repo: &Path, index: usize) -> Result<(), GitError> {
    let cli = GitCli::new(repo)?;
    let reference = stash_ref(index);
    cli.run(&["stash", "drop", &reference])?;
    Ok(())
}

/// Returns the unified patch for a stash entry.
///
/// # Errors
/// Propagates CLI failures.
pub fn stash_show(repo: &Path, index: usize) -> Result<String, GitError> {
    let cli = GitCli::new(repo)?;
    let reference = stash_ref(index);
    cli.run(&["stash", "show", "-p", &reference])
}

fn parse_list(out: &str) -> Vec<StashEntry> {
    let mut entries = Vec::new();
    for line in out.lines() {
        if line.is_empty() {
            continue;
        }
        let Some((ref_name, message)) = line.split_once('\t') else {
            continue;
        };
        let index = ref_name
            .strip_prefix("stash@{")
            .and_then(|s| s.strip_suffix('}'))
            .and_then(|s| s.parse().ok())
            .unwrap_or(entries.len());
        entries.push(StashEntry {
            index,
            message: message.to_string(),
        });
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::fixture::TempRepo;

    #[test]
    fn push_list_show_apply_pop_roundtrip() {
        let repo = TempRepo::init();
        repo.write("a.txt", "base\n");
        repo.stage("a.txt");
        repo.commit("init");
        repo.write("a.txt", "wip\n");

        stash_push(repo.path(), Some("wip changes")).expect("push");
        let list = list_stashes(repo.path()).expect("list");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].index, 0);
        assert!(list[0].message.contains("wip changes"));

        let patch = stash_show(repo.path(), 0).expect("show");
        assert!(patch.contains("wip"));

        stash_apply(repo.path(), 0).expect("apply");
        assert_eq!(
            std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
            "wip\n"
        );
        assert_eq!(list_stashes(repo.path()).expect("list").len(), 1);

        repo.run(&["checkout", "--", "a.txt"]);
        stash_pop(repo.path(), 0).expect("pop");
        assert_eq!(
            std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
            "wip\n"
        );
        assert!(list_stashes(repo.path()).expect("empty").is_empty());
    }

    #[test]
    fn drop_removes_entry() {
        let repo = TempRepo::init();
        repo.write("a.txt", "x\n");
        repo.stage("a.txt");
        repo.commit("init");
        repo.write("a.txt", "y\n");

        stash_push(repo.path(), None).expect("push");
        stash_drop(repo.path(), 0).expect("drop");
        assert!(list_stashes(repo.path()).expect("list").is_empty());
    }
}
