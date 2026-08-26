//! Pure layout helpers for the single-window shell (issue #10).

use crate::app::model::View;

/// Navigation entries in display order.
#[must_use]
pub fn nav_items() -> &'static [(View, &'static str)] {
    &[
        (View::Changes, "Changes"),
        (View::History, "History"),
        (View::Branches, "Branches"),
        (View::Worktrees, "Worktrees"),
        (View::Stashes, "Stashes"),
    ]
}

/// Heading shown in the Content pane for `view`.
#[must_use]
pub fn content_heading(view: View) -> &'static str {
    match view {
        View::Changes => "Diff",
        View::History => "History",
        View::Branches => "Branches",
        View::Worktrees => "Worktrees",
        View::Stashes => "Stashes",
    }
}

/// Short context-panel title for `view`.
#[must_use]
pub fn context_heading(view: View) -> &'static str {
    match view {
        View::Changes => "Commit / File",
        View::History => "Commit Detail",
        View::Branches => "Branch Context",
        View::Worktrees => "Worktree Context",
        View::Stashes => "Stash Context",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nav_items_cover_all_views_in_order() {
        let items = nav_items();
        assert_eq!(items.len(), 5);
        assert_eq!(items[0], (View::Changes, "Changes"));
        assert_eq!(items[1], (View::History, "History"));
        assert_eq!(items[2], (View::Branches, "Branches"));
        assert_eq!(items[3], (View::Worktrees, "Worktrees"));
        assert_eq!(items[4], (View::Stashes, "Stashes"));
    }

    #[test]
    fn content_heading_matches_design() {
        assert_eq!(content_heading(View::Changes), "Diff");
        assert_eq!(content_heading(View::History), "History");
        assert_eq!(content_heading(View::Branches), "Branches");
    }
}
