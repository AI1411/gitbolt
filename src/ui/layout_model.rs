//! Pure layout helpers for the single-window shell (issue #10).

use crate::app::model::View;

/// Left navigation pane width bounds (px).
pub const NAV_MIN: f64 = 140.0;
/// Left navigation pane width bounds (px).
pub const NAV_MAX: f64 = 360.0;
/// Right context pane minimum width (px).
pub const CONTEXT_MIN: f64 = 220.0;
/// Right context pane maximum width (px). Wide enough for commit file diffs.
pub const CONTEXT_MAX: f64 = 1100.0;
/// Floor used while a commit file diff is open in the context pane.
pub const COMMIT_DIFF_CONTEXT_MIN: f64 = 520.0;

/// Clamps a stored or dragged context-pane width.
#[must_use]
pub fn clamp_context_width(width: f64) -> f64 {
    width.clamp(CONTEXT_MIN, CONTEXT_MAX)
}

/// Width actually applied to the context pane.
///
/// Commit file diffs need more room than Instant Commit, so the floor is
/// raised while a file diff is showing — without exceeding [`CONTEXT_MAX`].
#[must_use]
pub fn context_pane_width(stored: f64, showing_commit_diff: bool) -> f64 {
    let width = clamp_context_width(stored);
    if showing_commit_diff {
        width.max(COMMIT_DIFF_CONTEXT_MIN)
    } else {
        width
    }
}

/// Navigation entries in display order.
#[must_use]
pub fn nav_items() -> &'static [(View, &'static str)] {
    &[
        (View::Changes, "Changes"),
        (View::History, "History"),
        (View::Branches, "Branches"),
    ]
}

/// Heading shown in the Content pane for `view`.
#[must_use]
pub fn content_heading(view: View) -> &'static str {
    match view {
        View::Changes => "Diff",
        View::History => "History",
        View::Branches => "Branches",
    }
}

/// History pane title reflecting the active filter (issue #23).
#[must_use]
pub fn history_title(filter: &crate::app::state::HistoryFilter) -> String {
    use crate::app::state::HistoryFilter;
    match filter {
        HistoryFilter::All => "History".into(),
        HistoryFilter::File { path } => format!("File History — {}", path.display()),
        HistoryFilter::Line { path, line } => {
            format!("Line {line} History — {}", path.display())
        }
    }
}

/// Short context-panel title for `view`.
#[must_use]
pub fn context_heading(view: View) -> &'static str {
    match view {
        View::Changes => "Commit / File",
        View::History => "Commit Detail",
        View::Branches => "Branch Context",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nav_items_cover_all_views_in_order() {
        let items = nav_items();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], (View::Changes, "Changes"));
        assert_eq!(items[1], (View::History, "History"));
        assert_eq!(items[2], (View::Branches, "Branches"));
    }

    #[test]
    fn content_heading_matches_design() {
        assert_eq!(content_heading(View::Changes), "Diff");
        assert_eq!(content_heading(View::History), "History");
        assert_eq!(content_heading(View::Branches), "Branches");
    }

    #[test]
    fn context_pane_can_grow_past_old_480_cap() {
        let near = |got: f64, want: f64| (got - want).abs() < 0.5;
        assert!(near(clamp_context_width(480.0), 480.0));
        assert!(near(clamp_context_width(900.0), 900.0));
        assert!(near(clamp_context_width(2000.0), CONTEXT_MAX));
        assert!(near(context_pane_width(280.0, false), 280.0));
        assert!(near(
            context_pane_width(280.0, true),
            COMMIT_DIFF_CONTEXT_MIN
        ));
        assert!(near(context_pane_width(800.0, true), 800.0));
    }
}
