//! Branch list view — progressive placeholder until issue #17.

use dioxus::prelude::*;

use crate::app::state::AppState;

/// Branches placeholder.
#[component]
pub fn BranchesView(state: AppState) -> Element {
    let count = state.branch.branches.len();
    let current = state
        .branch
        .current
        .clone()
        .or_else(|| state.repository.head.branch.clone())
        .unwrap_or_else(|| "(none)".into());

    rsx! {
        div {
            style: "font-size:0.9rem;opacity:0.85;",
            p { style: "margin:0;", "Current: {current}" }
            if state.branch.loaded {
                p { style: "margin:0.35rem 0 0;", "{count} branches." }
            } else {
                p { style: "margin:0.35rem 0 0;opacity:0.6;", "Loading branches…" }
            }
            p { style: "margin:0.5rem 0 0;opacity:0.5;font-size:0.8rem;",
                "Branch actions arrive in issue #17."
            }
        }
    }
}
