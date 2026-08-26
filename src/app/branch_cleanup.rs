//! Outdated / merged local branch cleanup candidates (issue #78).

use crate::app::model::{BranchHealth, BranchInfo};

/// Branch names that must never appear in cleanup lists.
pub const PROTECTED: &[&str] = &["main", "master", "develop", "trunk", "release"];

/// Why a branch was offered for cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupReason {
    Merged,
    Stale,
    Both,
}

/// A cleanup candidate with its reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupCandidate {
    pub name: String,
    pub reason: CleanupReason,
}

/// Returns whether `name` is a protected branch.
#[must_use]
pub fn is_protected(name: &str) -> bool {
    PROTECTED.iter().any(|p| p.eq_ignore_ascii_case(name))
}

/// Builds cleanup candidates from branch list + merged-into-base names.
#[must_use]
pub fn candidates(
    branches: &[BranchInfo],
    current: Option<&str>,
    merged_names: &[String],
) -> Vec<CleanupCandidate> {
    let mut out = Vec::new();
    for b in branches.iter().filter(|b| !b.is_remote) {
        if current.is_some_and(|c| c == b.name) || is_protected(&b.name) {
            continue;
        }
        let merged = merged_names.iter().any(|m| m == &b.name);
        let stale = b.health == BranchHealth::Stale
            || b.stale_days
                .is_some_and(|d| d >= crate::app::branch_health::STALE_DAYS_THRESHOLD);
        let reason = match (merged, stale) {
            (true, true) => CleanupReason::Both,
            (true, false) => CleanupReason::Merged,
            (false, true) => CleanupReason::Stale,
            (false, false) => continue,
        };
        out.push(CleanupCandidate {
            name: b.name.clone(),
            reason,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Human label for a reason.
#[must_use]
pub fn reason_label(reason: CleanupReason) -> &'static str {
    match reason {
        CleanupReason::Merged => "merged",
        CleanupReason::Stale => "stale",
        CleanupReason::Both => "merged + stale",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::model::BranchHealth;

    fn local(name: &str, health: BranchHealth, stale_days: Option<u32>) -> BranchInfo {
        BranchInfo {
            name: name.into(),
            upstream: None,
            health,
            ahead: 0,
            behind: 0,
            last_commit: None,
            is_remote: false,
            stale_days,
        }
    }

    #[test]
    fn excludes_current_and_protected() {
        let branches = vec![
            local("main", BranchHealth::Synced, None),
            local("feature", BranchHealth::Synced, None),
            local("old", BranchHealth::Stale, Some(40)),
        ];
        let merged = vec!["feature".into(), "main".into()];
        let c = candidates(&branches, Some("feature"), &merged);
        assert!(c.iter().all(|x| x.name != "main" && x.name != "feature"));
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].name, "old");
        assert_eq!(c[0].reason, CleanupReason::Stale);
    }

    #[test]
    fn marks_merged() {
        let branches = vec![local("done", BranchHealth::Local, None)];
        let merged = vec!["done".into()];
        let c = candidates(&branches, Some("main"), &merged);
        assert_eq!(c[0].reason, CleanupReason::Merged);
    }
}
