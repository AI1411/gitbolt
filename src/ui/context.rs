//! Right context panel — commit box + selection summary (issues #10 / #15 / #25).

use dioxus::prelude::*;

use crate::app::event::UiEvent;
use crate::app::model::View;
use crate::app::state::AppState;
use crate::ui::layout_model::context_heading;

/// Props for the context panel.
#[derive(Props, Clone, PartialEq)]
pub struct ContextPaneProps {
    pub state: AppState,
    pub on_event: EventHandler<UiEvent>,
}

/// Renders the Context column with commit input on Changes.
#[component]
pub fn ContextPane(props: ContextPaneProps) -> Element {
    let heading = context_heading(props.state.navigation.active_view);
    let selection = selection_summary(&props.state);
    let staged_n = props.state.changes.staged.len();
    let autofocus_key = props.state.ui.commit_focus_token;

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

            div {
                style: "display:flex;flex-direction:column;gap:0.4rem;",
                label {
                    style: "font-size:0.75rem;opacity:0.6;letter-spacing:0.04em;text-transform:uppercase;",
                    "Commit message"
                }
                textarea {
                    key: "{autofocus_key}",
                    autofocus: autofocus_key > 0,
                    style: "width:100%;min-height:5.5rem;box-sizing:border-box;resize:vertical;\
                            padding:0.45rem 0.55rem;border-radius:4px;border:1px solid #334155;\
                            background:#0f1419;color:#e8eef7;font-size:0.85rem;font-family:inherit;",
                    placeholder: "Commit message… (C to focus, ⌘Enter to commit)",
                    value: "{props.state.ui.commit_message}",
                    oninput: move |evt| {
                        props.on_event.call(UiEvent::SetCommitMessage(evt.value()));
                    },
                    onkeydown: move |evt| {
                        let mods = evt.data().modifiers();
                        let shortcut =
                            mods.contains(Modifiers::META) || mods.contains(Modifiers::CONTROL);
                        if shortcut && matches!(evt.data().key(), Key::Enter) {
                            evt.prevent_default();
                            props.on_event.call(UiEvent::Commit);
                        }
                    },
                }
                div {
                    style: "display:flex;align-items:center;gap:0.5rem;",
                    button {
                        style: "border:0;background:#3d8bfd;color:white;border-radius:4px;\
                                padding:0.35rem 0.75rem;cursor:pointer;font-size:0.8rem;font-weight:600;",
                        disabled: staged_n == 0,
                        onclick: move |_| props.on_event.call(UiEvent::Commit),
                        "Commit"
                    }
                    span {
                        style: "font-size:0.75rem;opacity:0.55;",
                        "{staged_n} staged"
                    }
                }
                if let Some(err) = props.state.ui.error_banner.as_ref() {
                    p {
                        style: "margin:0;color:#fca5a5;font-size:0.8rem;",
                        "{err}"
                    }
                }
            }

            p {
                style: "margin:0;font-size:0.8rem;opacity:0.5;",
                "⌘I to hide · C focus · ⌘Enter commit"
            }
        }
    }
}

fn selection_summary(state: &AppState) -> String {
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
