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
    /// The repository path vanished or is unreadable.
    RepoMissing(PathBuf),
    /// The `git` binary could not be found for a CLI fallback.
    GitBinaryNotFound,
    /// Authentication with a remote failed.
    Auth(String),
    /// A merge / stash / pull produced conflicts.
    Conflict(String),
    /// Permission denied on a file or repository path.
    PermissionDenied(String),
    /// Objects or the `.git` directory appear corrupted.
    Corrupt(String),
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

    /// True when the repository can no longer be used (missing / not a repo).
    #[must_use]
    pub fn is_fatal_for_session(&self) -> bool {
        matches!(
            self,
            Self::NotARepository(_) | Self::RepoMissing(_) | Self::Corrupt(_)
        )
    }

    /// A concise, user-facing message (Japanese, matching the product UI).
    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            Self::NotARepository(path) => {
                format!("Git リポジトリではありません: {}", path.display())
            }
            Self::RepoMissing(path) => {
                format!(
                    "リポジトリにアクセスできません（削除または移動された可能性があります）: {}",
                    path.display()
                )
            }
            Self::GitBinaryNotFound => "git コマンドが見つかりません".to_string(),
            Self::Auth(_) => "認証に失敗しました。資格情報を確認してください".to_string(),
            Self::Conflict(detail) => {
                if detail.is_empty() {
                    "コンフリクトが発生しました".to_string()
                } else {
                    format!("コンフリクトが発生しました: {detail}")
                }
            }
            Self::PermissionDenied(_) => {
                "権限がありません。ファイルまたはリポジトリのアクセス権を確認してください"
                    .to_string()
            }
            Self::Corrupt(_) => {
                "リポジトリが破損している可能性があります。git fsck などで確認してください"
                    .to_string()
            }
            Self::Io(msg) | Self::Backend(msg) => summarize_backend(msg),
            Self::Unsupported(op) => format!("未対応の操作です: {op}"),
        }
    }
}

fn summarize_backend(msg: &str) -> String {
    let lower = msg.to_lowercase();
    if lower.contains("permission denied") || lower.contains("operation not permitted") {
        return GitError::PermissionDenied(msg.to_string()).user_message();
    }
    if lower.contains("corrupt")
        || lower.contains("invalid object")
        || lower.contains("broken link")
        || lower.contains("bad object")
    {
        return GitError::Corrupt(msg.to_string()).user_message();
    }
    if lower.contains("no such file") || lower.contains("not a git repository") {
        return "Git 操作に失敗しました: リポジトリまたはパスが見つかりません".to_string();
    }
    // Truncate very long CLI dumps for inline display.
    let trimmed = msg.trim();
    let short = if trimmed.len() > 240 {
        format!("{}…", &trimmed[..240])
    } else {
        trimmed.to_string()
    };
    format!("Git 操作に失敗しました: {short}")
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotARepository(path) => write!(f, "not a git repository: {}", path.display()),
            Self::RepoMissing(path) => write!(f, "repository missing: {}", path.display()),
            Self::GitBinaryNotFound => f.write_str("git binary not found"),
            Self::Auth(msg) => write!(f, "authentication failed: {msg}"),
            Self::Conflict(msg) => write!(f, "conflict: {msg}"),
            Self::PermissionDenied(msg) => write!(f, "permission denied: {msg}"),
            Self::Corrupt(msg) => write!(f, "corrupt repository: {msg}"),
            Self::Io(msg) => write!(f, "io error: {msg}"),
            Self::Backend(msg) => write!(f, "backend error: {msg}"),
            Self::Unsupported(op) => write!(f, "unsupported operation: {op}"),
        }
    }
}

impl std::error::Error for GitError {}

impl From<std::io::Error> for GitError {
    fn from(value: std::io::Error) -> Self {
        if value.kind() == std::io::ErrorKind::PermissionDenied {
            Self::PermissionDenied(value.to_string())
        } else {
            Self::Io(value.to_string())
        }
    }
}

/// Classifies a free-form backend/CLI message into a [`GitError`].
#[must_use]
pub fn classify_message(msg: &str) -> GitError {
    let lower = msg.to_lowercase();
    if lower.contains("authentication failed")
        || lower.contains("could not read username")
        || lower.contains("permission denied (publickey)")
        || lower.contains("terminal prompts disabled")
    {
        GitError::Auth(msg.trim().to_string())
    } else if lower.contains("conflict")
        || lower.contains("would be overwritten")
        || lower.contains("local changes")
        || lower.contains("your local changes to the following files would be overwritten")
    {
        GitError::Conflict(msg.trim().to_string())
    } else if (lower.contains("permission denied") && !lower.contains("publickey"))
        || lower.contains("operation not permitted")
    {
        GitError::PermissionDenied(msg.trim().to_string())
    } else if lower.contains("corrupt")
        || lower.contains("invalid object")
        || lower.contains("broken link")
        || lower.contains("bad object")
    {
        GitError::Corrupt(msg.trim().to_string())
    } else if lower.contains("not a git repository") {
        GitError::NotARepository(PathBuf::from("."))
    } else {
        GitError::Backend(msg.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_message_table_covers_core_variants() {
        assert!(GitError::GitBinaryNotFound
            .user_message()
            .contains("git コマンド"));
        assert!(GitError::Auth("x".into()).user_message().contains("認証"));
        assert!(GitError::Conflict(String::new())
            .user_message()
            .contains("コンフリクト"));
        assert!(GitError::PermissionDenied("x".into())
            .user_message()
            .contains("権限"));
        assert!(GitError::Corrupt("x".into())
            .user_message()
            .contains("破損"));
    }

    #[test]
    fn classify_message_maps_permission_and_conflict() {
        assert!(matches!(
            classify_message("fatal: Permission denied"),
            GitError::PermissionDenied(_)
        ));
        assert!(matches!(
            classify_message("error: CONFLICT (content)"),
            GitError::Conflict(_)
        ));
    }
}
