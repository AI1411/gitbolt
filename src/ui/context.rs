//! Right context panel chrome (issue #10). Detail content arrives in #25.

use dioxus::prelude::*;

use crate::app::model::View;
use crate::app::state::AppState;
use crate::ui::layout_model::context_heading;

/// Props for the context panel.
#[derive(Props, Clone, PartialEq)]
pub struct ContextPaneProps {
    pub state: AppState,
}

/// Renders the Context column. Keeps prior framing while detail loads later.
#[component]
pub fn ContextPane(props: ContextPaneProps) -> Element {
    let heading = context_heading(props.state.navigation.active_view);
    let selection = selection_summary(&props.state);

    rsx! {
        aside {
            class: "context-pane",
            style: "display:flex;flex-direction:column;gap:0.75rem;padding:0.85rem;\
                    height:100%;box-sizing:border-box;overflow:auto;border-left:1px solid #243044;",
            h2 {
                style: "margin:0;font-size:0.8rem;letter-spacing:0.06em;text-transform:uppercase;\
                        opacity:0.65;font-weight:600;",
                "{heading}"
            }
            p {
                style: "margin:0;font-size:0.9rem;line-height:1.45;opacity:0.85;",
                "{selection}"
            }
            p {
                style: "margin:0;font-size:0.8rem;opacity:0.5;",
                "⌘I to hide · Detail views land in later MVP issues."
            }
        }
    }
}

fn selection_summary(state: &AppState) -> String {
    // Change Origin / commit selection takes precedence when set.
    if let Some(oid) = state.selection.commit.as_ref() {
        let short = if oid.0.len() > 7 { &oid.0[..7] } else { &oid.0 };
        let detail = state
            .diff
            .content
            .ready()
            .and_then(|c| {
                c.hunks.iter().flat_map(|h| h.lines.iter()).find_map(|l| {
                    l.change_origin.as_ref().and_then(|o| {
                        if o.oid == *oid {
                            Some(format!("{} · {}", o.author, o.summary))
                        } else {
                            None
                        }
                    })
                })
            })
            .unwrap_or_else(|| "Change Origin".into());
        return format!("Commit {short} — {detail}");
    }

    match state.navigation.active_view {
        View::Changes => state.selection.file.as_ref().map_or_else(
            || "No file selected".into(),
            |p| format!("File: {}", p.display()),
        ),
        View::History => "No commit selected".into(),
        View::Branches => state
            .selection
            .branch
            .clone()
            .map(|b| format!("Branch: {b}"))
            .or_else(|| {
                state
                    .repository
                    .head
                    .branch
                    .clone()
                    .map(|b| format!("Current: {b}"))
            })
            .unwrap_or_else(|| "No branch selected".into()),
        View::Worktrees => "Worktree context".into(),
        View::Stashes => "Stash context".into(),
    }
}
