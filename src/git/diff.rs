//! Unified diff loading via the git CLI.

use std::path::Path;

use super::cli::GitCli;
use super::error::GitError;
use super::service::DiffText;

/// Returns the unified diff for `path` (`staged` → `--cached`).
///
/// # Errors
/// Propagates [`GitError`] from the CLI.
pub fn unified_diff(repo: &Path, path: &Path, staged: bool) -> Result<DiffText, GitError> {
    let cli = GitCli::new(repo)?;
    let path_str = path.to_string_lossy();
    let args: Vec<&str> = if staged {
        vec!["diff", "--cached", "--", path_str.as_ref()]
    } else {
        vec!["diff", "--", path_str.as_ref()]
    };
    let text = cli.run(&args)?;
    Ok(DiffText {
        path: path.to_path_buf(),
        staged,
        text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::fixture::TempRepo;

    #[test]
    fn unified_diff_shows_unstaged_edit() {
        let repo = TempRepo::init();
        repo.write("a.txt", "one\n");
        repo.stage("a.txt");
        repo.commit("initial");
        repo.write("a.txt", "one\ntwo\n");

        let diff = unified_diff(repo.path(), Path::new("a.txt"), false).expect("diff");
        assert!(diff.text.contains("+two"), "{}", diff.text);
    }
}
