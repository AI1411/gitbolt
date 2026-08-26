//! Error type for the Git Service and its user-facing message conversion.
//!
//! See `docs/design/09-git-backend.md` (error conversion) and
//! `docs/design/07-runtime.md` section 13 (inline errors, resilience).

use std::fmt;
use std::path::PathBuf;

/// A Git operation failure, normalized across the gix backend and the git CLI
/// fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitError {
    /// The path is not inside a Git repository.
    NotARepository(PathBuf),
    /// The `git` binary could not be found for a CLI fallback.
    GitBinaryNotFound,
    /// Authentication with a remote failed.
    Auth(String),
    /// A merge / stash / pull produced conflicts.
    Conflict(String),
    /// An I/O error occurred.
    Io(String),
    /// The backend (gix or git CLI) reported an error.
    Backend(String),
    /// The operation is not yet implemented by this backend.
    Unsupported(&'static str),
}

impl GitError {
    /// Convenience constructor for [`GitError::Unsupported`].
    #[must_use]
    pub fn unsupported(op: &'static str) -> Self {
        Self::Unsupported(op)
    }

    /// A concise, user-facing message (Japanese, matching the product UI).
    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            Self::NotARepository(path) => {
                format!("Git リポジトリではありません: {}", path.display())
            }
            Self::GitBinaryNotFound => "git コマンドが見つかりません".to_string(),
            Self::Auth(_) => "認証に失敗しました。資格情報を確認してください".to_string(),
            Self::Conflict(_) => "コンフリクトが発生しました".to_string(),
            Self::Io(msg) | Self::Backend(msg) => format!("Git 操作に失敗しました: {msg}"),
            Self::Unsupported(op) => format!("未対応の操作です: {op}"),
        }
    }
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotARepository(path) => write!(f, "not a git repository: {}", path.display()),
            Self::GitBinaryNotFound => f.write_str("git binary not found"),
            Self::Auth(msg) => write!(f, "authentication failed: {msg}"),
            Self::Conflict(msg) => write!(f, "conflict: {msg}"),
            Self::Io(msg) => write!(f, "io error: {msg}"),
            Self::Backend(msg) => write!(f, "backend error: {msg}"),
            Self::Unsupported(op) => write!(f, "unsupported operation: {op}"),
        }
    }
}

impl std::error::Error for GitError {}

impl From<std::io::Error> for GitError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}
