//! Stash list / save / apply / pop / drop (issue #24).

use dioxus::prelude::*;

use crate::app::event::UiEvent;
use crate::app::model::{DiffHunk, Loadable};
use crate::app::state::AppState;
use crate::ui::diff::tint_line;
use crate::ui::error_banner::ConfirmPanel;
use crate::ui::list_search::{matches_query, ListSearchBar};

/// Props for the stashes pane.
#[derive(Props, Clone, PartialEq)]
pub struct StashesViewProps {
    pub state: AppState,
    pub on_event: EventHandler<UiEvent>,
}

/// Stash operations with diff preview for the selected entry.
#[component]
pub fn StashesView(props: StashesViewProps) -> Element {
    let mut message_draft = use_signal(String::new);
    let entries: Vec<_> = props
        .state
        .stash
        .entries
        .iter()
        .filter(|e| matches_query(&e.message, &props.state.ui.search_query))
        .cloned()
        .collect();
    let loaded = props.state.stash.loaded;
    let selected = props.state.stash.selected;
    let pending_drop = props.state.ui.confirm_drop_stash;
    let diff = props.state.stash.diff.clone();

    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:0.75rem;font-size:0.9rem;",
            ListSearchBar {
                state: props.state.clone(),
                on_event: props.on_event,
                placeholder: "Filter stashes…".to_string(),
            }

            div {
                style: "display:flex;flex-direction:column;gap:0.35rem;",
                h3 {
                    style: "margin:0;font-size:0.75rem;letter-spacing:0.06em;\
                            text-transform:uppercase;opacity:0.6;",
                    "Save stash"
                }
                div {
                    style: "display:flex;gap:0.4rem;flex-wrap:wrap;align-items:center;",
                    input {
                        style: "flex:1;min-width:10rem;padding:0.35rem 0.5rem;border-radius:var(--gb-radius);\
                                border:1px solid var(--gb-border-strong);background:var(--gb-bg);color:var(--gb-text);font-size:0.85rem;",
                        placeholder: "message (optional)",
                        value: "{message_draft()}",
                        oninput: move |evt| message_draft.set(evt.value()),
                    }
                    button {
                        style: "border:0;background:var(--gb-accent);color:white;border-radius:var(--gb-radius);\
                                padding:0.35rem 0.75rem;cursor:pointer;font-size:0.8rem;font-weight:600;",
                        onclick: move |_| {
                            let msg = message_draft().trim().to_string();
                            let message = if msg.is_empty() { None } else { Some(msg) };
                            props.on_event.call(UiEvent::StashSave { message });
                            message_draft.set(String::new());
                        },
                        "Stash"
                    }
                }
            }

            if let Some(index) = pending_drop {
                ConfirmPanel {
                    message: format!("Drop stash@{{{index}}}? This cannot be undone."),
                    confirm_label: String::from("Drop"),
                    on_confirm: move |()| props.on_event.call(UiEvent::ConfirmDropStash),
                    on_cancel: move |()| props.on_event.call(UiEvent::CancelDropStash),
                }
            }

            StashDiffPreview { diff: diff }

            if !loaded {
                p { style: "margin:0;opacity:0.6;", "Loading stashes…" }
            } else if entries.is_empty() {
                p { style: "margin:0;opacity:0.6;", "No stashes yet." }
            } else {
                ul {
                    style: "list-style:none;margin:0;padding:0;display:flex;flex-direction:column;gap:0.35rem;",
                    for entry in entries.iter().cloned() {
                        {
                            let index = entry.index;
                            let is_sel = selected == Some(index);
                            let bg = if is_sel { "var(--gb-selected)" } else { "transparent" };
                            rsx! {
                                li {
                                    key: "stash-{index}",
                                    style: format!(
                                        "display:flex;align-items:center;gap:0.5rem;flex-wrap:wrap;\
                                         padding:0.35rem 0.45rem;border-radius:var(--gb-radius);border:1px solid var(--gb-chip);\
                                         background:{bg};"
                                    ),
                                    button {
                                        style: "flex:1;text-align:left;border:0;background:transparent;\
                                                color:var(--gb-text);cursor:pointer;padding:0;min-width:0;",
                                        onclick: move |_| props.on_event.call(UiEvent::SelectStash(index)),
                                        span {
                                            style: "font-family:var(--gb-mono);font-size:0.75rem;\
                                                    opacity:0.6;margin-right:0.45rem;",
                                            "stash@{{{index}}}"
                                        }
                                        span { "{entry.message}" }
                                    }
                                    if is_sel {
                                        button {
                                            style: action_style(),
                                            onclick: move |_| props.on_event.call(UiEvent::StashApply(index)),
                                            "Apply"
                                        }
                                        button {
                                            style: action_style(),
                                            onclick: move |_| props.on_event.call(UiEvent::StashPop(index)),
                                            "Pop"
                                        }
                                        button {
                                            style: "border:1px solid var(--gb-danger-border);background:transparent;color:var(--gb-danger);\
                                                    border-radius:var(--gb-radius);padding:0.15rem 0.45rem;cursor:pointer;font-size:0.72rem;",
                                            onclick: move |_| props.on_event.call(UiEvent::RequestDropStash(index)),
                                            "Drop"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StashDiffPreview(diff: Loadable<crate::app::model::DiffContent>) -> Element {
    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:0.35rem;",
            h3 {
                style: "margin:0;font-size:0.75rem;letter-spacing:0.06em;text-transform:uppercase;opacity:0.6;",
                "Stash diff"
            }
            match diff {
                Loadable::Idle => rsx! {
                    p { style: "margin:0;opacity:0.55;font-size:0.85rem;", "Select a stash to preview." }
                },
                Loadable::Loading => rsx! {
                    p { style: "margin:0;opacity:0.6;font-size:0.85rem;", "Loading stash diff…" }
                },
                Loadable::Failed(err) => rsx! {
                    p { style: "margin:0;color:var(--gb-danger);font-size:0.85rem;", "Diff error: {err}" }
                },
                Loadable::Ready(content) => rsx! {
                    if let Some(notice) = content.notice.as_ref() {
                        div {
                            style: "padding:0.35rem 0.55rem;border-radius:var(--gb-radius);background:var(--gb-chip);\
                                    color:var(--gb-warning);font-size:0.8rem;",
                            "{notice}"
                        }
                    }
                    div {
                        style: "padding:0.5rem 0;border:1px solid var(--gb-border);border-radius:var(--gb-radius-lg);\
                                background:var(--gb-surface-raised);font-family:var(--gb-mono);font-size:0.82rem;\
                                overflow:auto;max-height:40vh;",
                        for (hi, hunk) in content.hunks.iter().enumerate() {
                            HunkBlock { key: "{hi}", hunk: hunk.clone() }
                        }
                    }
                },
            }
        }
    }
}

#[component]
fn HunkBlock(hunk: DiffHunk) -> Element {
    rsx! {
        div {
            div {
                style: "padding:0.25rem 0.65rem;opacity:0.55;background:var(--gb-bg);",
                "{hunk.header}"
            }
            for line in hunk.lines.iter() {
                {
                    let color = match line.origin {
                        '+' => "var(--gb-add)",
                        '-' => "var(--gb-danger)",
                        _ => "var(--gb-chip-text)",
                    };
                    let tinted = tint_line(&line.content);
                    rsx! {
                        div {
                            style: format!(
                                "display:flex;gap:0.65rem;padding:0.05rem 0.65rem;white-space:pre;color:{color};"
                            ),
                            span { style: "flex:0 0 1ch;opacity:0.7;", "{line.origin}" }
                            span {
                                style: "flex:1;min-width:0;",
                                dangerous_inner_html: "{tinted}",
                            }
                        }
                    }
                }
            }
        }
    }
}

fn action_style() -> &'static str {
    "border:1px solid var(--gb-border-strong);background:transparent;color:var(--gb-text-muted);\
     border-radius:var(--gb-radius);padding:0.15rem 0.45rem;cursor:pointer;font-size:0.72rem;"
}
