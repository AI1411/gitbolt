//! Synthetic repositories for local / CI benchmarks (issue #34).

use std::path::{Path, PathBuf};
use std::process::Command;

/// A disposable Git repository used by `gitbolt-bench`.
pub struct BenchRepo {
    root: PathBuf,
    owned: bool,
}

impl BenchRepo {
    /// Opens an existing repository path (not deleted on drop).
    #[must_use]
    pub fn open(path: impl Into<PathBuf>) -> Self {
        Self {
            root: path.into(),
            owned: false,
        }
    }

    /// Builds a synthetic repo with `files` tracked files and `commits` history.
    ///
    /// Layout is sized for CI: enough work for status/diff/blame/history without
    /// cloning rust-lang/rust. External large repos: pass `--repo`.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the temp directory or git commands fail.
    pub fn scale(files: usize, commits: usize) -> std::io::Result<Self> {
        let root = std::env::temp_dir().join(format!(
            "gitbolt-bench-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&root)?;
        let repo = Self {
            root: root.clone(),
            owned: true,
        };
        repo.git(&["init", "-b", "main"])?;
        repo.git(&["config", "user.name", "Bench"])?;
        repo.git(&["config", "user.email", "bench@example.com"])?;
        repo.git(&["config", "commit.gpgsign", "false"])?;

        let files = files.max(1);
        let commits = commits.max(1);
        for c in 0..commits {
            for f in 0..files {
                let rel = format!("src/f{f:04}.txt");
                let path = root.join(&rel);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(
                    &path,
                    format!("commit {c} file {f}\nline two\nline three\n"),
                )?;
            }
            repo.git(&["add", "-A"])?;
            repo.git(&["commit", "-m", &format!("c{c}")])?;
        }
        // Leave one dirty file for status/diff coverage.
        let dirty = root.join("src/f0000.txt");
        let mut body = std::fs::read_to_string(&dirty).unwrap_or_default();
        body.push_str("dirty\n");
        std::fs::write(dirty, body)?;
        Ok(repo)
    }

    /// Repository root.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.root
    }

    fn git(&self, args: &[&str]) -> std::io::Result<()> {
        let out = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(args)
            .output()?;
        if !out.status.success() {
            return Err(std::io::Error::other(format!(
                "git {args:?}: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        Ok(())
    }
}

impl Drop for BenchRepo {
    fn drop(&mut self) {
        if self.owned {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
}
