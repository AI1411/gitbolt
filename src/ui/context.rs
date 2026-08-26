//! Right context panel — Instant Commit / Branch context (issues #10 / #15 / #25).

use dioxus::prelude::*;

use crate::app::branch_health::format_badge;
use crate::app::event::UiEvent;
use crate::app::model::{BranchInfo, CommitDetail, Loadable, View};
use crate::app::state::{AppState, HistoryFilter};
use crate::ui::layout_model::context_heading;

/// Props for the context panel.
#[derive(Props, Clone, PartialEq)]
pub struct ContextPaneProps {
    pub state: AppState,
    pub on_event: EventHandler<UiEvent>,
}

/// Renders the Context column; content varies by view and selection.
#[component]
pub fn ContextPane(props: ContextPaneProps) -> Element {
    let heading = context_heading(props.state.navigation.active_view);
    let view = props.state.navigation.active_view;
    let staged_n = props.state.changes.staged.len();
    let autofocus_key = props.state.ui.commit_focus_token;
    let copy_feedback = props.state.ui.copy_feedback.clone();

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

            if let Some(msg) = copy_feedback {
                p {
                    style: "margin:0;font-size:0.75rem;color:#86efac;",
                    "{msg}"
                }
            }

            if props.state.selection.commit.is_some() {
                CommitDetailPanel {
                    state: props.state.clone(),
                    on_event: props.on_event,
                }
            } else {
                match view {
                    View::Changes => rsx! {
                        FileContext { state: props.state.clone() }
                        CommitBox {
                            staged_n: staged_n,
                            autofocus_key: autofocus_key,
                            message: props.state.ui.commit_message.clone(),
                            error: props.state.ui.error_banner.clone(),
                            on_event: props.on_event,
                        }
                    },
                    View::History => rsx! {
                        HistoryContext { state: props.state.clone() }
                    },
                    View::Branches => rsx! {
                        BranchContextPanel {
                            state: props.state.clone(),
                            on_event: props.on_event,
                        }
                    },
                    View::Worktrees => rsx! {
                        WorktreeContext { state: props.state.clone() }
                    },
                    View::Stashes => rsx! {
                        StashContext { state: props.state.clone() }
                    },
                }
            }

            p {
                style: "margin:0;font-size:0.8rem;opacity:0.5;",
                "⌘I to hide"
            }
        }
    }
}

#[component]
fn CommitDetailPanel(state: AppState, on_event: EventHandler<UiEvent>) -> Element {
    let oid = state.selection.commit.clone();
    match &state.context.commit {
        Loadable::Loading => rsx! {
            p { style: "margin:0;opacity:0.6;font-size:0.85rem;", "Loading commit…" }
        },
        Loadable::Failed(err) => rsx! {
            p { style: "margin:0;color:#fca5a5;font-size:0.85rem;", "Error: {err}" }
        },
        Loadable::Idle => rsx! {
            p { style: "margin:0;opacity:0.6;font-size:0.85rem;", "Select a commit." }
        },
        Loadable::Ready(detail) => rsx! {
            CommitDetailBody {
                detail: detail.clone(),
                oid: oid.clone(),
                on_event: on_event,
            }
        },
    }
}

#[component]
fn CommitDetailBody(
    detail: CommitDetail,
    oid: Option<crate::app::model::Oid>,
    on_event: EventHandler<UiEvent>,
) -> Element {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(0);
    let rel = relative_time(detail.timestamp, now);
    let full_oid = oid
        .as_ref()
        .map_or(detail.oid.0.as_str(), |o| o.0.as_str());
    let short = if full_oid.len() > 12 {
        &full_oid[..12]
    } else {
        full_oid
    };

    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:0.55rem;font-size:0.85rem;",
            div {
                style: "display:flex;align-items:center;gap:0.4rem;flex-wrap:wrap;",
                span {
                    style: "font-family:ui-monospace,monospace;font-size:0.78rem;opacity:0.75;",
                    "{short}"
                }
                CopyButton {
                    label: "Copy hash",
                    text: full_oid.to_string(),
                    on_event: on_event,
                }
            }
            div {
                style: "font-weight:600;font-size:0.95rem;",
                "{detail.summary}"
            }
            div {
                style: "opacity:0.7;font-size:0.8rem;",
                "{detail.author} · {rel}"
            }
            if !detail.body.is_empty() && detail.body != detail.summary {
                pre {
                    style: "margin:0;white-space:pre-wrap;font-family:inherit;font-size:0.82rem;\
                            opacity:0.85;line-height:1.45;",
                    "{detail.body}"
                }
            }
            if !detail.files.is_empty() {
                div {
                    style: "margin-top:0.25rem;",
                    div {
                        style: "font-size:0.72rem;letter-spacing:0.05em;text-transform:uppercase;opacity:0.55;",
                        "Changed files ({detail.files.len()})"
                    }
                    ul {
                        style: "list-style:none;margin:0.25rem 0 0;padding:0;display:flex;flex-direction:column;gap:0.15rem;",
                        for f in detail.files.iter() {
                            li {
                                style: "font-family:ui-monospace,monospace;font-size:0.78rem;opacity:0.85;",
                                span { style: "opacity:0.55;margin-right:0.35rem;", "{f.status}" }
                                "{f.path.display()}"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn BranchContextPanel(state: AppState, on_event: EventHandler<UiEvent>) -> Element {
    let branch_name = state
        .selection
        .branch
        .clone()
        .or_else(|| state.repository.head.branch.clone());

    let Some(name) = branch_name else {
        return rsx! {
            p { style: "margin:0;opacity:0.6;", "No branch selected." }
        };
    };

    let info = state.branch.branches.iter().find(|b| b.name == name);
    let worktree_path = state
        .worktree
        .worktrees
        .iter()
        .find(|w| w.branch.as_deref() == Some(name.as_str()))
        .map(|w| w.path.display().to_string());

    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:0.5rem;font-size:0.85rem;",
            div {
                style: "display:flex;align-items:center;gap:0.45rem;flex-wrap:wrap;",
                span { style: "font-weight:600;font-size:0.95rem;", "{name}" }
                CopyButton {
                    label: "Copy branch",
                    text: name.clone(),
                    on_event: on_event,
                }
            }
            if let Some(b) = info {
                BranchMeta { branch: b.clone() }
            }
            if let Some(path) = worktree_path {
                div {
                    style: "font-size:0.78rem;opacity:0.7;font-family:ui-monospace,monospace;",
                    "Worktree: {path}"
                }
            } else {
                div {
                    style: "font-size:0.78rem;opacity:0.55;",
                    "No linked worktree"
                }
            }
        }
    }
}

#[component]
fn BranchMeta(branch: BranchInfo) -> Element {
    let badge = format_badge(
        branch.health,
        branch.ahead,
        branch.behind,
        branch.stale_days,
    );
    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:0.3rem;",
            div {
                style: "font-size:0.8rem;opacity:0.75;",
                "Health: {badge}"
            }
            if let Some(up) = branch.upstream.as_ref() {
                div {
                    style: "font-size:0.8rem;opacity:0.75;",
                    "Upstream: {up}"
                }
            } else {
                div { style: "font-size:0.8rem;opacity:0.55;", "No upstream" }
            }
            if branch.ahead > 0 || branch.behind > 0 {
                div {
                    style: "font-size:0.8rem;opacity:0.75;",
                    "Ahead {branch.ahead} · Behind {branch.behind}"
                }
            }
            if let Some(c) = branch.last_commit.as_ref() {
                div {
                    style: "font-size:0.78rem;opacity:0.65;margin-top:0.15rem;",
                    "Last: {c.summary}"
                }
            }
        }
    }
}

#[component]
fn FileContext(state: AppState) -> Element {
    rsx! {
        p {
            style: "margin:0;font-size:0.9rem;line-height:1.45;opacity:0.85;",
            if let Some(p) = state.selection.file.as_ref() {
                "File: {p.display()}\nH → file history · Shift+click blame → line history"
            } else {
                "No file selected"
            }
        }
    }
}

#[component]
fn HistoryContext(state: AppState) -> Element {
    rsx! {
        p {
            style: "margin:0;font-size:0.9rem;line-height:1.45;opacity:0.85;",
            {history_hint(&state)}
        }
    }
}

#[component]
fn WorktreeContext(state: AppState) -> Element {
    let n = state.worktree.worktrees.len();
    rsx! {
        p {
            style: "margin:0;font-size:0.9rem;opacity:0.85;",
            "{n} worktree(s). Select a branch for Branch Context."
        }
    }
}

#[component]
fn StashContext(state: AppState) -> Element {
    rsx! {
        p {
            style: "margin:0;font-size:0.9rem;opacity:0.85;",
            {
                if let Some(index) = state.stash.selected {
                    if let Some(entry) = state.stash.entries.iter().find(|e| e.index == index) {
                        format!("stash@{{{index}}}\n{}", entry.message)
                    } else {
                        format!("{} stash(es)", state.stash.entries.len())
                    }
                } else {
                    format!("{} stash(es)", state.stash.entries.len())
                }
            }
        }
    }
}

#[component]
fn CommitBox(
    staged_n: usize,
    autofocus_key: u64,
    message: String,
    error: Option<String>,
    on_event: EventHandler<UiEvent>,
) -> Element {
    rsx! {
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
                value: "{message}",
                oninput: move |evt| {
                    on_event.call(UiEvent::SetCommitMessage(evt.value()));
                },
                onkeydown: move |evt| {
                    let mods = evt.data().modifiers();
                    let shortcut =
                        mods.contains(Modifiers::META) || mods.contains(Modifiers::CONTROL);
                    if shortcut && matches!(evt.data().key(), Key::Enter) {
                        evt.prevent_default();
                        on_event.call(UiEvent::Commit);
                    }
                },
            }
            div {
                style: "display:flex;align-items:center;gap:0.5rem;",
                button {
                    style: "border:0;background:#3d8bfd;color:white;border-radius:4px;\
                            padding:0.35rem 0.75rem;cursor:pointer;font-size:0.8rem;font-weight:600;",
                    disabled: staged_n == 0,
                    onclick: move |_| on_event.call(UiEvent::Commit),
                    "Commit"
                }
                span {
                    style: "font-size:0.75rem;opacity:0.55;",
                    "{staged_n} staged"
                }
            }
            if let Some(err) = error {
                p {
                    style: "margin:0;color:#fca5a5;font-size:0.8rem;",
                    "{err}"
                }
            }
            p {
                style: "margin:0;font-size:0.75rem;opacity:0.5;",
                "C focus · ⌘Enter commit"
            }
        }
    }
}

#[component]
fn CopyButton(label: String, text: String, on_event: EventHandler<UiEvent>) -> Element {
    rsx! {
        button {
            style: "border:1px solid #334155;background:transparent;color:#9fb0c7;\
                    border-radius:4px;padding:0.15rem 0.45rem;cursor:pointer;font-size:0.68rem;",
            onclick: move |_| on_event.call(UiEvent::CopyText(text.clone())),
            "{label}"
        }
    }
}

fn history_hint(state: &AppState) -> String {
    match &state.history.filter {
        HistoryFilter::File { path } => {
            format!(
                "File history: {}\n{} commit(s)\nClick a commit for detail.",
                path.display(),
                state.history.commits.len()
            )
        }
        HistoryFilter::Line { path, line } => {
            format!(
                "Line {line}: {}\n{} commit(s)\nClick a commit for detail.",
                path.display(),
                state.history.commits.len()
            )
        }
        HistoryFilter::All => "Click a commit to see detail here.".into(),
    }
}

fn relative_time(ts: i64, now: i64) -> String {
    let delta = (now - ts).max(0);
    if delta < 60 {
        return format!("{delta}s ago");
    }
    if delta < 3600 {
        return format!("{}m ago", delta / 60);
    }
    if delta < 86400 {
        return format!("{}h ago", delta / 3600);
    }
    if delta < 86400 * 30 {
        return format!("{}d ago", delta / 86400);
    }
    format!("{}mo ago", delta / (86400 * 30))
}
