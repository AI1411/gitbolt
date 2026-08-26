//! Temporary repository fixtures for Git Service tests.
//!
//! Repositories are created with the `git` CLI (always available in dev/CI),
//! which keeps fixtures independent of the code under test.

use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

/// A throwaway Git repository in a temporary directory.
pub struct TempRepo {
    dir: TempDir,
}

impl TempRepo {
    /// Creates an initialized repository on branch `main` with a deterministic
    /// identity and no signing.
    #[must_use]
    pub fn init() -> Self {
        let dir = tempfile::tempdir().expect("create temp dir");
        let repo = Self { dir };
        repo.run(&["init", "-b", "main"]);
        repo.run(&["config", "user.name", "Test"]);
        repo.run(&["config", "user.email", "test@example.com"]);
        repo.run(&["config", "commit.gpgsign", "false"]);
        repo
    }

    /// The repository root path.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Runs `git -C <repo> <args...>`, panicking on failure.
    pub fn run(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.path())
            .args(args)
            .output()
            .expect("spawn git");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_string()
    }

    /// Writes `contents` to `rel` (relative to the repo root).
    pub fn write(&self, rel: &str, contents: &str) {
        let full = self.path().join(rel);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).expect("create parent dirs");
        }
        std::fs::write(full, contents).expect("write file");
    }

    /// Stages `rel`.
    pub fn stage(&self, rel: &str) {
        self.run(&["add", "--", rel]);
    }

    /// Commits the staged changes with `message`.
    pub fn commit(&self, message: &str) {
        self.run(&["commit", "-m", message]);
    }
}
