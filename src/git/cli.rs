//! `git` command-line fallback used for operations gix does not (yet) cover
//! (push, pull, worktree add/remove, stash apply/drop, …).
//!
//! See `docs/design/09-git-backend.md`. Network operations intentionally
//! delegate authentication to the user's git configuration (credential
//! helpers, ssh-agent) rather than reimplementing it.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::error::GitError;

/// A thin wrapper around the `git` binary scoped to one repository.
#[derive(Debug, Clone)]
pub struct GitCli {
    program: PathBuf,
    repo: PathBuf,
}

impl GitCli {
    /// Creates a wrapper for `repo`, verifying the `git` binary is available.
    ///
    /// # Errors
    /// Returns [`GitError::GitBinaryNotFound`] if `git` cannot be executed.
    pub fn new(repo: impl Into<PathBuf>) -> Result<Self, GitError> {
        let program = Self::discover()?;
        Ok(Self {
            program,
            repo: repo.into(),
        })
    }

    /// Locates the `git` binary by invoking `git --version`.
    ///
    /// # Errors
    /// Returns [`GitError::GitBinaryNotFound`] if the binary is missing.
    pub fn discover() -> Result<PathBuf, GitError> {
        let program = PathBuf::from("git");
        match Command::new(&program).arg("--version").output() {
            Ok(output) if output.status.success() => Ok(program),
            _ => Err(GitError::GitBinaryNotFound),
        }
    }

    /// Runs `git -C <repo> <args...>` and returns trimmed stdout on success.
    ///
    /// # Errors
    /// Maps a non-zero exit (or spawn failure) to a [`GitError`], classifying
    /// authentication and conflict failures from stderr.
    pub fn run(&self, args: &[&str]) -> Result<String, GitError> {
        let output = Command::new(&self.program)
            .arg("-C")
            .arg(&self.repo)
            .args(args)
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    GitError::GitBinaryNotFound
                } else {
                    GitError::Io(e.to_string())
                }
            })?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Ok(stdout.trim_end().to_string());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        Err(classify_stderr(&stderr))
    }
}

/// Classifies a git CLI stderr string into a [`GitError`].
fn classify_stderr(stderr: &str) -> GitError {
    let lower = stderr.to_lowercase();
    if lower.contains("authentication failed")
        || lower.contains("could not read username")
        || lower.contains("permission denied (publickey)")
        || lower.contains("terminal prompts disabled")
    {
        GitError::Auth(stderr.trim().to_string())
    } else if lower.contains("conflict") {
        GitError::Conflict(stderr.trim().to_string())
    } else {
        GitError::Backend(stderr.trim().to_string())
    }
}

/// Locate `git` without binding to a repository.
///
/// # Errors
/// Returns [`GitError::GitBinaryNotFound`] if the binary is missing.
pub fn git_available() -> Result<PathBuf, GitError> {
    GitCli::discover()
}

impl GitCli {
    /// The repository path this wrapper is scoped to.
    #[must_use]
    pub fn repo(&self) -> &Path {
        &self.repo
    }
}
