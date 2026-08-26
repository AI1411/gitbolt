//! Commit creation via `git commit` (issue #15).

use std::path::Path;

use super::cli::GitCli;
use super::error::GitError;

/// Creates a commit with `message` using the user's git identity / signing config.
///
/// Returns the new HEAD OID.
///
/// # Errors
/// Propagates CLI failures (nothing staged, hooks, etc.).
pub fn commit(repo: &Path, message: &str) -> Result<String, GitError> {
    let cli = GitCli::new(repo)?;
    // Prefer -m for non-interactive agents; signing follows user config.
    cli.run(&["commit", "-m", message])?;
    let oid = cli.run(&["rev-parse", "HEAD"])?;
    Ok(oid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::fixture::TempRepo;
    use crate::git::{GitService, GixService};

    #[test]
    fn commit_creates_head_and_clears_index() {
        let repo = TempRepo::init();
        repo.write("a.txt", "one\n");
        repo.stage("a.txt");
        let oid = commit(repo.path(), "add a").expect("commit");
        assert_eq!(oid.len(), 40);

        let status = GixService::open(repo.path())
            .expect("open")
            .status()
            .expect("status");
        assert!(status.is_empty());

        let head = GixService::open(repo.path())
            .expect("open")
            .head()
            .expect("head");
        assert_eq!(head.oid.as_deref(), Some(oid.as_str()));
    }
}
