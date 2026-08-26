//! Commit metadata and changed files (issue #25).

use std::path::{Path, PathBuf};

use super::cli::GitCli;
use super::error::GitError;

/// Full commit detail for the Context Panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitDetail {
    pub oid: String,
    pub summary: String,
    pub author: String,
    pub timestamp: i64,
    pub body: String,
    pub files: Vec<CommitFileChange>,
}

/// A path changed in a commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitFileChange {
    pub status: char,
    pub path: PathBuf,
}

/// Unified diff text for one path within a commit.
///
/// Uses `git show <oid> -- <path>` (works for root commits; no `oid^`).
///
/// # Errors
/// Propagates CLI failures.
pub fn show_file_diff(repo: &Path, oid: &str, path: &Path) -> Result<String, GitError> {
    let cli = GitCli::new(repo)?;
    let path_str = path.to_string_lossy();
    // `--format=` suppresses the commit header so output is a bare unified diff.
    cli.run(&["show", "--format=", "--patch", oid, "--", path_str.as_ref()])
}

/// Loads commit metadata and changed files for `oid`.
///
/// # Errors
/// Propagates CLI failures.
pub fn show_commit(repo: &Path, oid: &str) -> Result<CommitDetail, GitError> {
    let cli = GitCli::new(repo)?;
    let header = cli.run(&["show", "-s", "--format=%H%x09%an%x09%at%x09%s", oid])?;
    let body = cli
        .run(&["log", "-1", "--format=%B", oid])?
        .trim_end()
        .to_string();
    let files_out = cli.run(&["diff-tree", "--no-commit-id", "--name-status", "-r", oid])?;
    parse_commit(&header, body, &files_out)
}

fn parse_commit(header: &str, body: String, files_out: &str) -> Result<CommitDetail, GitError> {
    let line = header
        .lines()
        .next()
        .ok_or_else(|| GitError::Backend("empty commit header".into()))?;
    let mut parts = line.splitn(4, '\t');
    let oid = parts.next().unwrap_or("").to_string();
    let author = parts.next().unwrap_or("").to_string();
    let timestamp = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let summary = parts.next().unwrap_or("").to_string();

    let files = files_out
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(parse_name_status)
        .collect();

    Ok(CommitDetail {
        oid,
        summary,
        author,
        timestamp,
        body,
        files,
    })
}

fn parse_name_status(line: &str) -> Option<CommitFileChange> {
    let mut parts = line.splitn(2, '\t');
    let status = parts.next()?.chars().next()?;
    let path = parts.next()?.trim().to_string();
    if path.is_empty() {
        return None;
    }
    Some(CommitFileChange {
        status,
        path: PathBuf::from(path),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::fixture::TempRepo;

    #[test]
    fn show_commit_includes_metadata_and_files() {
        let repo = TempRepo::init();
        repo.write("a.txt", "one\n");
        repo.write("b.txt", "two\n");
        repo.stage("a.txt");
        repo.stage("b.txt");
        repo.commit("add both");

        repo.write("a.txt", "one\ntwo\n");
        repo.stage("a.txt");
        repo.run(&["commit", "-m", "second line\n\nBody paragraph."]);

        let head = repo.run(&["rev-parse", "HEAD"]);
        let detail = show_commit(repo.path(), head.trim()).expect("show");
        assert_eq!(detail.summary, "second line");
        assert!(detail.body.contains("Body paragraph"));
        assert!(detail
            .files
            .iter()
            .any(|f| f.path.ends_with("a.txt") && f.status == 'M'));

        let patch =
            show_file_diff(repo.path(), head.trim(), Path::new("a.txt")).expect("file diff");
        assert!(patch.contains("+two") || patch.contains("two"), "{patch}");
        assert!(patch.contains("@@"), "{patch}");
    }
}
