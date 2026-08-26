//! Branch Health classification (Synced / Ahead / Behind / Diverged / Stale).
//!
//! See issue #18 and `docs/design/04-ui-design.md` (Branch Health).

use super::model::BranchHealth;

/// Default stale threshold in days (last tip commit age).
pub const STALE_DAYS_THRESHOLD: u32 = 30;

const SECONDS_PER_DAY: i64 = 86_400;

/// Whole days elapsed since `timestamp` (unix seconds) relative to `now`.
#[must_use]
pub fn days_since(timestamp: i64, now: i64) -> Option<u32> {
    if timestamp <= 0 || now < timestamp {
        return None;
    }
    let days = (now - timestamp) / SECONDS_PER_DAY;
    u32::try_from(days).ok()
}

/// Classifies branch health. Stale wins when tip age ≥ threshold.
#[must_use]
pub fn classify_health(
    ahead: u32,
    behind: u32,
    has_upstream: bool,
    stale_days: Option<u32>,
    threshold: u32,
) -> BranchHealth {
    if stale_days.is_some_and(|d| d >= threshold) {
        return BranchHealth::Stale;
    }
    if !has_upstream {
        return BranchHealth::Local;
    }
    match (ahead, behind) {
        (0, 0) => BranchHealth::Synced,
        (_, 0) => BranchHealth::Ahead,
        (0, _) => BranchHealth::Behind,
        _ => BranchHealth::Diverged,
    }
}

/// Formats the Branch Health badge text (`✓`, `↑3`, `◌ 42d`, …).
#[must_use]
pub fn format_badge(
    health: BranchHealth,
    ahead: u32,
    behind: u32,
    stale_days: Option<u32>,
) -> String {
    match health {
        BranchHealth::Synced => "✓".into(),
        BranchHealth::Ahead => format!("↑{ahead}"),
        BranchHealth::Behind => format!("↓{behind}"),
        BranchHealth::Diverged => format!("↑{ahead}↓{behind}"),
        BranchHealth::Stale => match stale_days {
            Some(d) => format!("◌ {d}d"),
            None => "◌".into(),
        },
        BranchHealth::Local => "local".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_since_counts_whole_days() {
        let now = 1_700_000_000;
        assert_eq!(days_since(now - 42 * SECONDS_PER_DAY, now), Some(42));
        assert_eq!(days_since(now, now), Some(0));
        assert_eq!(days_since(0, now), None);
    }

    #[test]
    fn stale_overrides_upstream_health() {
        assert_eq!(
            classify_health(0, 0, true, Some(42), STALE_DAYS_THRESHOLD),
            BranchHealth::Stale
        );
        assert_eq!(
            classify_health(2, 1, true, Some(10), STALE_DAYS_THRESHOLD),
            BranchHealth::Diverged
        );
        assert_eq!(
            classify_health(0, 0, true, Some(29), STALE_DAYS_THRESHOLD),
            BranchHealth::Synced
        );
    }

    #[test]
    fn format_badge_stale_includes_days() {
        assert_eq!(format_badge(BranchHealth::Stale, 0, 0, Some(42)), "◌ 42d");
        assert_eq!(format_badge(BranchHealth::Synced, 0, 0, None), "✓");
        assert_eq!(format_badge(BranchHealth::Ahead, 3, 0, None), "↑3");
    }
}
