//! gix-backed [`GitService`] implementation.
//!
//! Only `open`, `head`, and `status` are implemented here (issue #5); other
//! operations use the trait's default `Unsupported` bodies until later issues
//! wire them (some via gix, some via the [`super::cli`] fallback — see
//! `docs/design/09-git-backend.md`).

use std::path::Path;

use gix::bstr::BString;

use super::blame;
use super::branch;
use super::commit;
use super::diff::{self as diff_mod};
use super::error::GitError;
use super::service::{
    BranchRef, ChangeStatus, CommitInfo, FileChange, GitService, Head, RepoStatus, WorktreeRef,
};
use super::stage;
use super::worktree;

/// A repository opened through gitoxide.
pub struct GixService {
    repo: gix::Repository,
}

impl GixService {
    /// The underlying gix repository.
    #[must_use]
    pub fn repo(&self) -> &gix::Repository {
        &self.repo
    }
}

impl GitService for GixService {
    fn open(path: &Path) -> Result<Self, GitError> {
        let repo = gix::discover(path).map_err(|_| GitError::NotARepository(path.to_path_buf()))?;
        Ok(Self { repo })
    }

    fn head(&self) -> Result<Head, GitError> {
        let head = self
            .repo
            .head()
            .map_err(|e| GitError::Backend(e.to_string()))?;
        let branch = head.referent_name().map(|name| name.shorten().to_string());
        let detached = head.is_detached();
        let oid = head.id().map(|id| id.detach().to_string());
        Ok(Head {
            branch,
            oid,
            detached,
        })
    }

    fn status(&self) -> Result<RepoStatus, GitError> {
        use gix::diff::index::Change as TreeChange;
        use gix::status::plumbing::index_as_worktree::{Change as WtChange, EntryStatus};
        use gix::status::{index_worktree, Item};

        let platform = self
            .repo
            .status(gix::progress::Discard)
            .map_err(|e| GitError::Backend(e.to_string()))?;
        let iter = platform
            .into_iter(Vec::<BString>::new())
            .map_err(|e| GitError::Backend(e.to_string()))?;

        let mut status = RepoStatus::default();
        for item in iter {
            let item = item.map_err(|e| GitError::Backend(e.to_string()))?;
            let path = gix::path::from_bstr(item.location()).into_owned();

            match item {
                Item::TreeIndex(change) => {
                    let kind = match change {
                        TreeChange::Addition { .. } => ChangeStatus::Added,
                        TreeChange::Deletion { .. } => ChangeStatus::Deleted,
                        TreeChange::Modification { .. } => ChangeStatus::Modified,
                        TreeChange::Rewrite { copy, .. } => {
                            if copy {
                                ChangeStatus::Copied
                            } else {
                                ChangeStatus::Renamed
                            }
                        }
                    };
                    status.staged.push(FileChange::new(path, kind));
                }
                Item::IndexWorktree(index_worktree::Item::Modification { status: st, .. }) => {
                    match st {
                        EntryStatus::Conflict { .. } => {
                            status
                                .conflicted
                                .push(FileChange::new(path, ChangeStatus::Conflicted));
                        }
                        EntryStatus::Change(change) => {
                            let kind = match change {
                                WtChange::Removed => ChangeStatus::Deleted,
                                WtChange::Type { .. } => ChangeStatus::TypeChange,
                                WtChange::Modification { .. }
                                | WtChange::SubmoduleModification(_) => ChangeStatus::Modified,
                            };
                            status.unstaged.push(FileChange::new(path, kind));
                        }
                        EntryStatus::NeedsUpdate(_) | EntryStatus::IntentToAdd => {}
                    }
                }
                Item::IndexWorktree(index_worktree::Item::DirectoryContents { .. }) => {
                    status
                        .untracked
                        .push(FileChange::new(path, ChangeStatus::Untracked));
                }
                Item::IndexWorktree(index_worktree::Item::Rewrite { .. }) => {
                    status
                        .unstaged
                        .push(FileChange::new(path, ChangeStatus::Renamed));
                }
            }
        }
        Ok(status)
    }

    fn diff(&self, path: &Path, staged: bool) -> Result<super::service::DiffText, GitError> {
        let root = self.workdir()?;
        diff_mod::unified_diff(root, path, staged)
    }

    fn stage(&self, path: &Path) -> Result<(), GitError> {
        stage::stage_file(self.workdir()?, path)
    }

    fn unstage(&self, path: &Path) -> Result<(), GitError> {
        stage::unstage_file(self.workdir()?, path)
    }

    fn stage_lines(
        &self,
        path: &Path,
        from_staged: bool,
        selected: &[usize],
    ) -> Result<(), GitError> {
        stage::stage_lines(self.workdir()?, path, from_staged, selected)
    }

    fn stage_all(&self) -> Result<(), GitError> {
        stage::stage_all(self.workdir()?)
    }

    fn unstage_all(&self) -> Result<(), GitError> {
        stage::unstage_all(self.workdir()?)
    }

    fn commit(&self, message: &str) -> Result<String, GitError> {
        commit::commit(self.workdir()?, message)
    }

    fn branches(&self) -> Result<Vec<BranchRef>, GitError> {
        branch::list_branches(self.workdir()?)
    }

    fn merge_base(&self, a: &str, b: &str) -> Result<String, GitError> {
        branch::merge_base(self.workdir()?, a, b)
    }

    fn commits_not_in(
        &self,
        tip: &str,
        base: &str,
        limit: usize,
    ) -> Result<Vec<CommitInfo>, GitError> {
        branch::commits_not_in(self.workdir()?, tip, base, limit)
    }

    fn ahead_behind(&self, tip: &str, other: &str) -> Result<(u32, u32), GitError> {
        branch::ahead_behind(self.workdir()?, tip, other)
    }

    fn recent_branches(&self, limit: usize) -> Result<Vec<String>, GitError> {
        branch::recent_branches(self.workdir()?, limit)
    }

    fn branch_last_commit(&self, name: &str) -> Result<Option<CommitInfo>, GitError> {
        branch::branch_last_commit(self.workdir()?, name)
    }

    fn set_upstream(&self, name: &str, upstream: &str) -> Result<(), GitError> {
        branch::set_upstream(self.workdir()?, name, upstream)
    }

    fn worktrees(&self) -> Result<Vec<WorktreeRef>, GitError> {
        worktree::list_worktrees(self.workdir()?)
    }

    fn blame(&self, path: &Path) -> Result<std::collections::HashMap<u32, CommitInfo>, GitError> {
        blame::blame_at_head(self.workdir()?, path)
    }
}

impl GixService {
    fn workdir(&self) -> Result<&Path, GitError> {
        self.repo
            .workdir()
            .ok_or_else(|| GitError::Backend("bare repository has no worktree".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::fixture::TempRepo;

    #[test]
    fn open_non_repository_errors() {
        let dir = tempfile::tempdir().expect("temp dir");
        let result = GixService::open(dir.path());
        assert!(matches!(result, Err(GitError::NotARepository(_))));
    }

    #[test]
    fn open_and_head_report_current_branch() {
        let repo = TempRepo::init();
        repo.write("README.md", "# hello\n");
        repo.stage("README.md");
        repo.commit("initial");

        let service = GixService::open(repo.path()).expect("open repo");
        let head = service.head().expect("head");
        assert_eq!(head.branch.as_deref(), Some("main"));
        assert!(!head.detached);
        assert!(head.oid.is_some());
    }

    #[test]
    fn status_groups_staged_unstaged_and_untracked() {
        let repo = TempRepo::init();
        repo.write("tracked.txt", "one\n");
        repo.stage("tracked.txt");
        repo.commit("initial");

        // Modify a tracked file (unstaged), stage a new file, add an untracked file.
        repo.write("tracked.txt", "one\ntwo\n");
        repo.write("staged_new.txt", "new\n");
        repo.stage("staged_new.txt");
        repo.write("untracked.txt", "loose\n");

        let service = GixService::open(repo.path()).expect("open repo");
        let status = service.status().expect("status");

        assert!(
            status
                .staged
                .iter()
                .any(|f| f.path.ends_with("staged_new.txt") && f.status == ChangeStatus::Added),
            "expected staged addition, got {:?}",
            status.staged
        );
        assert!(
            status
                .unstaged
                .iter()
                .any(|f| f.path.ends_with("tracked.txt") && f.status == ChangeStatus::Modified),
            "expected unstaged modification, got {:?}",
            status.unstaged
        );
        assert!(
            status
                .untracked
                .iter()
                .any(|f| f.path.ends_with("untracked.txt")),
            "expected untracked file, got {:?}",
            status.untracked
        );
        assert!(!status.is_clean());
    }
}
