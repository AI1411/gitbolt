//! Diff view — progressive placeholder until issue #13.

use dioxus::prelude::*;

use crate::app::model::Loadable;
use crate::app::state::AppState;

/// Diff content placeholder that keeps prior framing while loading.
#[component]
pub fn DiffView(state: AppState) -> Element {
    let body = match &state.diff.content {
        Loadable::Ready(_) => "Diff ready.".to_string(),
        Loadable::Loading => "Loading diff…".to_string(),
        Loadable::Failed(err) => format!("Diff error: {err}"),
        Loadable::Idle => {
            if let Some(path) = state.selection.file.as_ref() {
                format!("Selected: {}", path.display())
            } else {
                "Select a file to view its diff.".into()
            }
        }
    };

    rsx! {
        div {
            style: "margin-bottom:1rem;padding:0.75rem;border:1px solid #243044;border-radius:6px;\
                    background:#151b24;font-family:ui-monospace,monospace;font-size:0.85rem;\
                    white-space:pre-wrap;min-height:4rem;",
            "{body}"
        }
    }
}
