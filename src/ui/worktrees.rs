//! Worktree management view — progressive placeholder until issue #20.

use dioxus::prelude::*;

use crate::app::state::AppState;

/// Worktrees placeholder.
#[component]
pub fn WorktreesView(state: AppState) -> Element {
    let count = state.worktree.worktrees.len();

    rsx! {
        div {
            style: "font-size:0.9rem;opacity:0.85;",
            if state.worktree.loaded {
                p { style: "margin:0;", "{count} worktrees." }
            } else {
                p { style: "margin:0;opacity:0.6;", "Loading worktrees…" }
            }
            p { style: "margin:0.5rem 0 0;opacity:0.5;font-size:0.8rem;",
                "Worktree First view arrives in issue #20."
            }
        }
    }
}
