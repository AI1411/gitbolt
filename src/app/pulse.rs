//! Repository Pulse — one-line header summary (issue #11 / #18).

use crate::app::branch_health::format_badge;
use crate::app::model::{BranchHealth, View};
use crate::app::state::AppState;

/// Derived pulse values for the shell header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PulseSnapshot {
    /// Branch name, `detached HEAD`, or short OID.
    pub branch_label: String,
    pub ahead: Option<u32>,
    pub behind: Option<u32>,
    /// Unstaged + untracked + conflicted file count.
    pub changes: u32,
    pub staged: u32,
    pub worktrees: u32,
    pub has_upstream: bool,
    pub detached: bool,
    pub health: Option<BranchHealth>,
    pub stale_days: Option<u32>,
}

fn clamp_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Builds a [`PulseSnapshot`] from current app state.
#[must_use]
pub fn summary(state: &AppState) -> PulseSnapshot {
    let head = &state.repository.head;
    let detached = head.detached;
    let branch_label = if detached {
        head.oid.as_ref().map_or_else(
            || "detached HEAD".into(),
            |o| {
                let short = if o.0.len() > 7 { &o.0[..7] } else { &o.0 };
                format!("detached {short}")
            },
        )
    } else {
        head.branch
            .clone()
            .or_else(|| state.branch.current.clone())
            .unwrap_or_else(|| "(no branch)".into())
    };

    let current_name = head.branch.as_deref().or(state.branch.current.as_deref());
    let current =
        current_name.and_then(|name| state.branch.branches.iter().find(|b| b.name == name));

    let (ahead, behind, has_upstream, health, stale_days) = match current {
        Some(b) => (
            Some(b.ahead),
            Some(b.behind),
            b.upstream.is_some(),
            Some(b.health),
            b.stale_days,
        ),
        None => (None, None, false, None, None),
    };

    let staged = clamp_u32(state.changes.staged.len());
    let changes = clamp_u32(
        state.changes.unstaged.len()
            + state.changes.untracked.len()
            + state.changes.conflicted.len(),
    );

    let worktrees = if state.worktree.loaded {
        clamp_u32(state.worktree.worktrees.len().max(1))
    } else {
        1
    };

    PulseSnapshot {
        branch_label,
        ahead,
        behind,
        changes,
        staged,
        worktrees,
        has_upstream,
        detached,
        health,
        stale_days,
    }
}

/// Formats the divergence segment (`↑3 ↓1`, `◌ 42d`, `no upstream`, or empty when synced).
#[must_use]
pub fn format_divergence(pulse: &PulseSnapshot) -> String {
    if pulse.detached {
        return String::new();
    }
    if let Some(BranchHealth::Stale) = pulse.health {
        return format_badge(
            BranchHealth::Stale,
            pulse.ahead.unwrap_or(0),
            pulse.behind.unwrap_or(0),
            pulse.stale_days,
        );
    }
    if !pulse.has_upstream {
        return "no upstream".into();
    }
    let a = pulse.ahead.unwrap_or(0);
    let b = pulse.behind.unwrap_or(0);
    if a == 0 && b == 0 {
        return "✓".into();
    }
    let mut parts = Vec::new();
    if a > 0 {
        parts.push(format!("↑{a}"));
    }
    if b > 0 {
        parts.push(format!("↓{b}"));
    }
    parts.join(" ")
}

/// Which navigation view a pulse segment opens.
#[must_use]
pub fn segment_view(segment: PulseSegment) -> View {
    match segment {
        PulseSegment::Branch | PulseSegment::Divergence => View::Branches,
        PulseSegment::Changes | PulseSegment::Staged => View::Changes,
        PulseSegment::Worktrees => View::Worktrees,
    }
}

/// Clickable pulse segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PulseSegment {
    Branch,
    Divergence,
    Changes,
    Staged,
    Worktrees,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::model::{BranchHealth, BranchInfo, ChangeKind, FileChange, HeadInfo, Oid};
    use crate::app::state::AppState;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn change(path: &str) -> FileChange {
        FileChange {
            path: PathBuf::from(path),
            kind: ChangeKind::Modified,
        }
    }

    #[test]
    fn summary_counts_and_branch_health() {
        let mut state = AppState::new();
        state.repository.head = HeadInfo {
            branch: Some("feature/auth".into()),
            oid: Some(Oid("abc".into())),
            detached: false,
        };
        state.branch.branches = Arc::from([BranchInfo {
            name: "feature/auth".into(),
            upstream: Some("origin/feature/auth".into()),
            health: BranchHealth::Diverged,
            ahead: 3,
            behind: 1,
            last_commit: None,
            is_remote: false,
            stale_days: None,
        }]);
        state.changes.staged = Arc::from([change("a"), change("b"), change("c"), change("d")]);
        state.changes.unstaged = Arc::from([change("e"), change("f")]);
        state.changes.untracked = Arc::from([change("g")]);
        state.worktree.loaded = true;
        state.worktree.worktrees = Arc::from([
            crate::app::model::WorktreeInfo {
                path: PathBuf::from("/a"),
                branch: Some("main".into()),
                is_primary: true,
            },
            crate::app::model::WorktreeInfo {
                path: PathBuf::from("/b"),
                branch: Some("feature/auth".into()),
                is_primary: false,
            },
            crate::app::model::WorktreeInfo {
                path: PathBuf::from("/c"),
                branch: None,
                is_primary: false,
            },
        ]);

        let p = summary(&state);
        assert_eq!(p.branch_label, "feature/auth");
        assert_eq!(p.ahead, Some(3));
        assert_eq!(p.behind, Some(1));
        assert_eq!(p.staged, 4);
        assert_eq!(p.changes, 3);
        assert_eq!(p.worktrees, 3);
        assert_eq!(format_divergence(&p), "↑3 ↓1");
    }

    #[test]
    fn detached_and_no_upstream() {
        let mut state = AppState::new();
        state.repository.head = HeadInfo {
            branch: None,
            oid: Some(Oid("deadbeef0123".into())),
            detached: true,
        };
        let p = summary(&state);
        assert!(p.branch_label.contains("detached"));
        assert!(format_divergence(&p).is_empty());

        state.repository.head.detached = false;
        state.repository.head.branch = Some("local-only".into());
        state.branch.branches = Arc::from([BranchInfo {
            name: "local-only".into(),
            upstream: None,
            health: BranchHealth::Local,
            ahead: 0,
            behind: 0,
            last_commit: None,
            is_remote: false,
            stale_days: None,
        }]);
        let p = summary(&state);
        assert!(!p.has_upstream);
        assert_eq!(format_divergence(&p), "no upstream");
    }

    #[test]
    fn stale_branch_shows_days_in_pulse() {
        let mut state = AppState::new();
        state.repository.head = HeadInfo {
            branch: Some("experiment".into()),
            oid: Some(Oid("abc".into())),
            detached: false,
        };
        state.branch.branches = Arc::from([BranchInfo {
            name: "experiment".into(),
            upstream: Some("origin/experiment".into()),
            health: BranchHealth::Stale,
            ahead: 0,
            behind: 0,
            last_commit: None,
            is_remote: false,
            stale_days: Some(42),
        }]);
        let p = summary(&state);
        assert_eq!(format_divergence(&p), "◌ 42d");
    }

    #[test]
    fn segment_views_match_navigation() {
        assert_eq!(segment_view(PulseSegment::Branch), View::Branches);
        assert_eq!(segment_view(PulseSegment::Changes), View::Changes);
        assert_eq!(segment_view(PulseSegment::Worktrees), View::Worktrees);
    }
}
