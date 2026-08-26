//! History / commit graph view — progressive placeholder until issue #16.

use dioxus::prelude::*;

use crate::app::state::AppState;

/// History list placeholder.
#[component]
pub fn HistoryView(state: AppState) -> Element {
    let count = state.history.commits.len();
    let loading = state.history.loading;

    rsx! {
        div {
            style: "font-size:0.9rem;opacity:0.85;",
            if count > 0 {
                p { style: "margin:0;", "{count} commits loaded." }
            } else if loading {
                p { style: "margin:0;opacity:0.6;", "Loading history…" }
            } else {
                p { style: "margin:0;", "No commits loaded yet." }
            }
            p { style: "margin:0.5rem 0 0;opacity:0.5;font-size:0.8rem;",
                "Commit graph arrives in issue #16."
            }
        }
    }
}
