//! Commit history via `git log` (issue #16).
//! File / line scoped history (issue #23).

use std::path::Path;

use super::cli::GitCli;
use super::error::GitError;
use super::service::CommitInfo;

/// Returns up to `limit` commits starting at `skip` (newest first).
///
/// # Errors
/// Propagates CLI failures.
pub fn log_page(repo: &Path, skip: usize, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
    let cli = GitCli::new(repo)?;
    let skip_s = skip.to_string();
    let limit_s = limit.to_string();
    let out = cli.run(&[
        "log",
        "--skip",
        &skip_s,
        "-n",
        &limit_s,
        "--format=%H%x09%s%x09%an%x09%at%x09%P",
    ])?;
    Ok(parse_log(&out))
}

/// Commits touching `path`, following renames (`--follow`).
///
/// # Errors
/// Propagates CLI failures.
pub fn file_log_page(
    repo: &Path,
    path: &Path,
    skip: usize,
    limit: usize,
) -> Result<Vec<CommitInfo>, GitError> {
    let cli = GitCli::new(repo)?;
    let path_str = path
        .to_str()
        .ok_or_else(|| GitError::Backend("path is not valid UTF-8".into()))?;
    let skip_s = skip.to_string();
    let limit_s = limit.to_string();
    let out = cli.run(&[
        "log",
        "--follow",
        "--skip",
        &skip_s,
        "-n",
        &limit_s,
        "--format=%H%x09%s%x09%an%x09%at%x09%P",
        "--",
        path_str,
    ])?;
    Ok(parse_log(&out))
}

/// Commits that changed `line` (1-based) in `path`.
///
/// # Errors
/// Propagates CLI failures.
pub fn line_log_page(
    repo: &Path,
    path: &Path,
    line: u32,
    skip: usize,
    limit: usize,
) -> Result<Vec<CommitInfo>, GitError> {
    if line == 0 {
        return Err(GitError::Backend("line number must be >= 1".into()));
    }
    let cli = GitCli::new(repo)?;
    let path_str = path
        .to_str()
        .ok_or_else(|| GitError::Backend("path is not valid UTF-8".into()))?;
    let skip_s = skip.to_string();
    let limit_s = limit.to_string();
    let line_spec = format!("{line},{line}:{path_str}");
    let out = cli.run(&[
        "log",
        "-L",
        &line_spec,
        "--no-patch",
        "--skip",
        &skip_s,
        "-n",
        &limit_s,
        "--format=%H%x09%s%x09%an%x09%at%x09%P",
    ])?;
    Ok(parse_log(&out))
}

fn parse_log(out: &str) -> Vec<CommitInfo> {
    let mut commits = Vec::new();
    for line in out.lines() {
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(5, '\t');
        let oid = parts.next().unwrap_or("").to_string();
        let summary = parts.next().unwrap_or("").to_string();
        let author = parts.next().unwrap_or("").to_string();
        let time = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let _parents = parts.next().unwrap_or("");
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

    #[test]
    fn log_page_returns_newest_first_with_skip() {
        let repo = TempRepo::init();
        repo.write("a.txt", "1\n");
        repo.stage("a.txt");
        repo.commit("first");
        repo.write("a.txt", "2\n");
        repo.stage("a.txt");
        repo.commit("second");
        repo.write("a.txt", "3\n");
        repo.stage("a.txt");
        repo.commit("third");

        let page = log_page(repo.path(), 0, 2).expect("page");
        assert_eq!(page.len(), 2);
        assert!(page[0].summary.contains("third"));
        assert!(page[1].summary.contains("second"));

        let next = log_page(repo.path(), 2, 2).expect("next");
        assert_eq!(next.len(), 1);
        assert!(next[0].summary.contains("first"));
    }

    #[test]
    fn file_log_follows_rename() {
        let repo = TempRepo::init();
        repo.write("old.txt", "line one\n");
        repo.stage("old.txt");
        repo.commit("add");
        repo.run(&["mv", "old.txt", "new.txt"]);
        repo.stage("new.txt");
        repo.commit("rename");

        let page = file_log_page(repo.path(), Path::new("new.txt"), 0, 10).expect("file log");
        assert_eq!(page.len(), 2);
        assert!(page[0].summary.contains("rename"));
        assert!(page[1].summary.contains("add"));
    }

    #[test]
    fn line_log_tracks_line_edits() {
        let repo = TempRepo::init();
        repo.write("a.txt", "alpha\n");
        repo.stage("a.txt");
        repo.commit("first");
        repo.write("a.txt", "beta\n");
        repo.stage("a.txt");
        repo.commit("second");
        repo.write("a.txt", "gamma\n");
        repo.stage("a.txt");
        repo.commit("third");

        let page = line_log_page(repo.path(), Path::new("a.txt"), 1, 0, 10).expect("line log");
        assert_eq!(page.len(), 3);
        assert!(page[0].summary.contains("third"));
        assert!(page[1].summary.contains("second"));
        assert!(page[2].summary.contains("first"));
    }
}
