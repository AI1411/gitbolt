//! Commit history via `git log` (issue #16).

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
}
