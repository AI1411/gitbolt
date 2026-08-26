//! Changes (staging area) view — progressive placeholder until issue #12.

use dioxus::prelude::*;

use crate::app::state::AppState;

/// Staging / unstaged summary placeholder.
#[component]
pub fn ChangesView(state: AppState) -> Element {
    let staged = state.changes.staged.len();
    let unstaged = state.changes.unstaged.len() + state.changes.untracked.len();
    let loaded = state.changes.loaded;

    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:0.35rem;font-size:0.9rem;opacity:0.85;",
            if loaded {
                p { style: "margin:0;", "STAGED ({staged}) · UNSTAGED ({unstaged})" }
            } else {
                p { style: "margin:0;opacity:0.6;", "Loading status…" }
            }
            p { style: "margin:0;opacity:0.5;font-size:0.8rem;",
                "Full Changes list arrives in issue #12."
            }
        }
    }
}
