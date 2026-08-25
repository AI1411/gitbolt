//! UI components for each major view.
//!
//! See `docs/design/05-architecture.md` section 15.

pub mod blame;
pub mod branches;
pub mod changes;
pub mod diff;
pub mod history;
pub mod pulse;
pub mod worktrees;

use dioxus::prelude::*;

/// Root application shell — renders an empty window for the Phase 0 foundation.
#[component]
pub fn App() -> Element {
    rsx! {
        div {
            width: "100vw",
            height: "100vh",
        }
    }
}
