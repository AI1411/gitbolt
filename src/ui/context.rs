//! Right context panel — Instant Commit / Branch context (issues #10 / #15 / #25).

use dioxus::prelude::*;

use crate::app::branch_health::format_badge;
use crate::app::event::UiEvent;
use crate::app::model::{BranchInfo, CommitDetail, DiffHunk, Loadable, View};
use crate::app::state::{AppState, HistoryFilter};
use crate::ui::diff::tint_line;
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

    rsx! {
        aside {
            class: "context-pane",
            style: "display:flex;flex-direction:column;gap:0.75rem;padding:0.85rem;\
                    height:100%;box-sizing:border-box;overflow:auto;border-left:1px solid var(--gb-border);",
            h2 {
                style: "margin:0;font-size:0.8rem;letter-spacing:0.06em;text-transform:uppercase;\
                        opacity:0.65;font-weight:600;",
                "{heading}"
            }

            if props.state.selection.commit.is_some() {
                CommitDetailPanel {
                    state: props.state.clone(),
                    on_event: props.on_event,
                }
            } else {
                match view {
                    View::Changes => {
                        let paths: Vec<std::path::PathBuf> = props
                            .state
                            .changes
                            .staged
                            .iter()
                            .chain(props.state.changes.unstaged.iter())
                            .chain(props.state.changes.untracked.iter())
                            .map(|f| f.path.clone())
                            .collect();
                        rsx! {
                            FileContext {
                                state: props.state.clone(),
                                on_event: props.on_event,
                            }
                            CommitBox {
                                staged_n: staged_n,
                                autofocus_key: autofocus_key,
                                message: props.state.ui.commit_message.clone(),
                                changed_paths: paths,
                                on_event: props.on_event,
                            }
                        }
                    },
                    View::History => rsx! {
                        HistoryContext {
                            state: props.state.clone(),
                            on_event: props.on_event,
                        }
                    },
                    View::Branches => rsx! {
                        BranchContextPanel {
                            state: props.state.clone(),
                            on_event: props.on_event,
                        }
                    },
                    View::Worktrees => rsx! {
                        WorktreeContext {
                            state: props.state.clone(),
                            on_event: props.on_event,
                        }
                    },
                    View::Stashes => rsx! {
                        StashContext {
                            state: props.state.clone(),
                            on_event: props.on_event,
                        }
                    },
                }
            }

            p {
                style: "margin:0;font-size:0.8rem;opacity:0.5;",
                "{crate::platform::mod_key_label()}I to hide"
            }
        }
    }
}

#[component]
fn CommitDetailPanel(state: AppState, on_event: EventHandler<UiEvent>) -> Element {
    let oid = state.selection.commit.clone();
    let can_back = !state.navigation.commit_back.is_empty();
    let can_forward = !state.navigation.commit_forward.is_empty();
    let selected_file = state.context.selected_file.clone();
    let file_diff = state.context.file_diff.clone();
    match &state.context.commit {
        Loadable::Loading => rsx! {
            p { style: "margin:0;opacity:0.6;font-size:0.85rem;", "Loading commit…" }
        },
        Loadable::Failed(err) => rsx! {
            p { style: "margin:0;color:var(--gb-danger);font-size:0.85rem;", "Error: {err}" }
        },
        Loadable::Idle => rsx! {
            p { style: "margin:0;opacity:0.6;font-size:0.85rem;", "Select a commit." }
        },
        Loadable::Ready(detail) => {
            let remote_commit_url = state.repository.origin_web.as_ref().map(|web| {
                let id = oid.as_ref().map_or(detail.oid.0.as_str(), |o| o.0.as_str());
                crate::git::remote_link::commit_url(web, id)
            });
            rsx! {
                CommitDetailBody {
                    detail: detail.clone(),
                    oid: oid.clone(),
                    can_back: can_back,
                    can_forward: can_forward,
                    selected_file: selected_file,
                    file_diff: file_diff,
                    remote_commit_url: remote_commit_url,
                    origin_web: state.repository.origin_web.clone(),
                    on_event: on_event,
                }
            }
        }
    }
}

#[component]
fn CommitDetailBody(
    detail: CommitDetail,
    oid: Option<crate::app::model::Oid>,
    can_back: bool,
    can_forward: bool,
    selected_file: Option<std::path::PathBuf>,
    file_diff: Loadable<crate::app::model::DiffContent>,
    remote_commit_url: Option<String>,
    origin_web: Option<crate::git::remote_link::RemoteWeb>,
    on_event: EventHandler<UiEvent>,
) -> Element {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(0);
    let rel = relative_time(detail.timestamp, now);
    let full_oid = oid.as_ref().map_or(detail.oid.0.as_str(), |o| o.0.as_str());
    let short = if full_oid.len() > 12 {
        &full_oid[..12]
    } else {
        full_oid
    };

    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:0.55rem;font-size:0.85rem;",
            div {
                style: "display:flex;align-items:center;gap:0.35rem;",
                button {
                    r#type: "button",
                    title: "Commit Back (⌘[)",
                    disabled: !can_back,
                    style: if can_back {
                        "border:1px solid var(--gb-border-strong);background:var(--gb-chip);color:var(--gb-chip-strong);border-radius:var(--gb-radius);\
                         padding:0.15rem 0.45rem;cursor:pointer;font-size:0.75rem;"
                    } else {
                        "border:1px solid var(--gb-chip);background:transparent;color:var(--gb-text-disabled);border-radius:var(--gb-radius);\
                         padding:0.15rem 0.45rem;cursor:default;font-size:0.75rem;opacity:0.5;"
                    },
                    onclick: move |_| on_event.call(UiEvent::NavigateCommit { delta: -1 }),
                    "← Back"
                }
                button {
                    r#type: "button",
                    title: "Commit Forward (⌘])",
                    disabled: !can_forward,
                    style: if can_forward {
                        "border:1px solid var(--gb-border-strong);background:var(--gb-chip);color:var(--gb-chip-strong);border-radius:var(--gb-radius);\
                         padding:0.15rem 0.45rem;cursor:pointer;font-size:0.75rem;"
                    } else {
                        "border:1px solid var(--gb-chip);background:transparent;color:var(--gb-text-disabled);border-radius:var(--gb-radius);\
                         padding:0.15rem 0.45rem;cursor:default;font-size:0.75rem;opacity:0.5;"
                    },
                    onclick: move |_| on_event.call(UiEvent::NavigateCommit { delta: 1 }),
                    "Forward →"
                }
            }
            div {
                style: "display:flex;align-items:center;gap:0.4rem;flex-wrap:wrap;",
                span {
                    style: "font-family:var(--gb-mono);font-size:0.78rem;opacity:0.75;",
                    "{short}"
                }
                CopyButton {
                    label: "Copy hash",
                    text: full_oid.to_string(),
                    on_event: on_event,
                }
            }
            if let Some(url) = remote_commit_url {
                RemoteLinkActions { url: url, on_event: on_event }
            }
            IssueLinkList {
                text: format!("{}\n{}", detail.summary, detail.body),
                origin_web: origin_web,
                on_event: on_event,
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
                    p {
                        style: "margin:0.2rem 0 0;font-size:0.72rem;opacity:0.5;",
                        "Click a file to view its diff below"
                    }
                    ul {
                        style: "list-style:none;margin:0.25rem 0 0;padding:0;display:flex;flex-direction:column;gap:0.15rem;",
                        for f in detail.files.iter() {
                            {
                                let path = f.path.clone();
                                let path_label = f.path.display().to_string();
                                let status = f.status;
                                let selected = selected_file.as_ref() == Some(&f.path);
                                let bg = if selected { "var(--gb-selected)" } else { "transparent" };
                                rsx! {
                                    li {
                                        key: "{path_label}",
                                        button {
                                            r#type: "button",
                                            style: format!(
                                                "display:flex;align-items:center;gap:0.35rem;width:100%;text-align:left;\
                                                 border:0;border-radius:var(--gb-radius);padding:0.2rem 0.3rem;cursor:pointer;\
                                                 background:{bg};color:var(--gb-chip-strong);font-family:var(--gb-mono);\
                                                 font-size:0.78rem;"
                                            ),
                                            onclick: move |_| {
                                                on_event.call(UiEvent::SelectCommitFile(path.clone()));
                                            },
                                            span { style: "opacity:0.55;flex:0 0 auto;", "{status}" }
                                            span { style: "opacity:0.9;overflow:hidden;text-overflow:ellipsis;", "{path_label}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if selected_file.is_some() || !matches!(file_diff, Loadable::Idle) {
                CommitFileDiffPreview {
                    path: selected_file.clone(),
                    diff: file_diff,
                    on_event: on_event,
                }
            }
        }
    }
}

#[component]
fn CommitFileDiffPreview(
    path: Option<std::path::PathBuf>,
    diff: Loadable<crate::app::model::DiffContent>,
    on_event: EventHandler<UiEvent>,
) -> Element {
    let title = path
        .as_ref()
        .map_or_else(|| "Commit file".into(), |p| p.display().to_string());
    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:0.35rem;margin-top:0.35rem;\
                    padding-top:0.45rem;border-top:1px solid var(--gb-border);",
            div {
                style: "display:flex;align-items:center;gap:0.5rem;flex-wrap:wrap;",
                h3 {
                    style: "margin:0;font-size:0.72rem;letter-spacing:0.06em;text-transform:uppercase;opacity:0.6;",
                    "File diff"
                }
                span {
                    style: "font-family:var(--gb-mono);font-size:0.75rem;opacity:0.85;\
                            overflow:hidden;text-overflow:ellipsis;min-width:0;",
                    "{title}"
                }
                button {
                    style: "margin-left:auto;padding:0.15rem 0.5rem;border:1px solid var(--gb-border-strong);border-radius:var(--gb-radius);\
                            cursor:pointer;background:transparent;color:var(--gb-text-muted);font-size:0.72rem;",
                    onclick: move |_| on_event.call(UiEvent::ClearCommitFileDiff),
                    "Close"
                }
            }
            match diff {
                Loadable::Idle => rsx! {
                    p { style: "margin:0;opacity:0.55;font-size:0.8rem;", "Select a changed file." }
                },
                Loadable::Loading => rsx! {
                    p { style: "margin:0;opacity:0.6;font-size:0.8rem;", "Loading file diff…" }
                },
                Loadable::Failed(err) => rsx! {
                    p { style: "margin:0;color:var(--gb-danger);font-size:0.8rem;", "Diff error: {err}" }
                },
                Loadable::Ready(content) => rsx! {
                    if let Some(notice) = content.notice.as_ref() {
                        div {
                            style: "padding:0.35rem 0.55rem;border-radius:var(--gb-radius);background:var(--gb-chip);\
                                    color:var(--gb-warning);font-size:0.78rem;",
                            "{notice}"
                        }
                    }
                    if content.hunks.is_empty() {
                        p {
                            style: "margin:0;opacity:0.6;font-size:0.8rem;",
                            "No textual diff (rename, mode change, or binary)."
                        }
                    } else {
                        div {
                            style: "padding:0.35rem 0;border:1px solid var(--gb-border);border-radius:var(--gb-radius-lg);\
                                    background:var(--gb-surface-raised);font-family:var(--gb-mono);font-size:0.75rem;\
                                    overflow:auto;max-height:50vh;",
                            for (hi, hunk) in content.hunks.iter().enumerate() {
                                CommitHunkBlock { key: "{hi}", hunk: hunk.clone() }
                            }
                        }
                    }
                },
            }
        }
    }
}

#[component]
fn CommitHunkBlock(hunk: DiffHunk) -> Element {
    rsx! {
        div {
            div {
                style: "padding:0.2rem 0.5rem;opacity:0.55;background:var(--gb-bg);",
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
                                "display:flex;gap:0.5rem;padding:0.05rem 0.5rem;white-space:pre;color:{color};"
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
            IssueLinkList {
                text: name.clone(),
                origin_web: state.repository.origin_web.clone(),
                on_event: on_event,
            }
            if let Some(b) = info {
                BranchMeta { branch: b.clone() }
            }
            if let Some(path) = worktree_path {
                div {
                    style: "font-size:0.78rem;opacity:0.7;font-family:var(--gb-mono);",
                    "Worktree: {path}"
                }
            } else {
                div {
                    style: "font-size:0.78rem;opacity:0.55;",
                    "No linked worktree"
                }
            }
            div {
                style: "display:flex;gap:0.35rem;flex-wrap:wrap;margin-top:0.25rem;",
                {
                    let checkout = name.clone();
                    let instant = name.clone();
                    rsx! {
                        button {
                            r#type: "button",
                            style: "border:1px solid var(--gb-border-strong);background:var(--gb-chip);color:var(--gb-chip-strong);border-radius:var(--gb-radius);\
                                    padding:0.25rem 0.55rem;cursor:pointer;font-size:0.75rem;",
                            onclick: move |_| on_event.call(UiEvent::CheckoutBranch(checkout.clone())),
                            "Checkout"
                        }
                        button {
                            r#type: "button",
                            style: "border:1px solid var(--gb-border-strong);background:transparent;color:var(--gb-text-muted);border-radius:var(--gb-radius);\
                                    padding:0.25rem 0.55rem;cursor:pointer;font-size:0.75rem;",
                            onclick: move |_| on_event.call(UiEvent::InstantWorktree { branch: instant.clone() }),
                            "Instant Worktree"
                        }
                    }
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
fn FileContext(state: AppState, on_event: EventHandler<UiEvent>) -> Element {
    let file_link = remote_file_link(&state);
    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:0.45rem;",
            p {
                style: "margin:0;font-size:0.9rem;line-height:1.45;opacity:0.85;",
                if let Some(p) = state.selection.file.as_ref() {
                    "File: {p.display()}\nH → file history · Shift+click blame → line history"
                } else {
                    "No file selected"
                }
            }
            if let Some(url) = file_link {
                RemoteLinkActions { url: url, on_event: on_event }
            }
        }
    }
}

#[component]
fn HistoryContext(state: AppState, on_event: EventHandler<UiEvent>) -> Element {
    let link = remote_history_link(&state);
    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:0.45rem;",
            p {
                style: "margin:0;font-size:0.9rem;line-height:1.45;opacity:0.85;",
                {history_hint(&state)}
            }
            if let Some(url) = link {
                RemoteLinkActions { url: url, on_event: on_event }
            }
        }
    }
}

#[component]
fn WorktreeContext(state: AppState, on_event: EventHandler<UiEvent>) -> Element {
    let n = state.worktree.worktrees.len();
    let selected_branch = state.selection.branch.clone();
    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:0.45rem;",
            p {
                style: "margin:0;font-size:0.9rem;opacity:0.85;",
                "{n} worktree(s). Select a branch for Instant Worktree."
            }
            if let Some(branch) = selected_branch {
                button {
                    r#type: "button",
                    style: "align-self:flex-start;border:1px solid var(--gb-border-strong);background:var(--gb-chip);color:var(--gb-chip-strong);\
                            border-radius:var(--gb-radius);padding:0.25rem 0.55rem;cursor:pointer;font-size:0.75rem;",
                    onclick: move |_| {
                        on_event.call(UiEvent::InstantWorktree {
                            branch: branch.clone(),
                        });
                    },
                    "Instant Worktree: {branch}"
                }
            }
        }
    }
}

#[component]
fn StashContext(state: AppState, on_event: EventHandler<UiEvent>) -> Element {
    let selected = state.stash.selected;
    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:0.45rem;font-size:0.9rem;",
            p {
                style: "margin:0;opacity:0.85;",
                {
                    if let Some(index) = selected {
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
            if let Some(index) = selected {
                div {
                    style: "display:flex;gap:0.35rem;flex-wrap:wrap;",
                    button {
                        r#type: "button",
                        style: "border:1px solid var(--gb-border-strong);background:var(--gb-chip);color:var(--gb-chip-strong);border-radius:var(--gb-radius);\
                                padding:0.25rem 0.55rem;cursor:pointer;font-size:0.75rem;",
                        onclick: move |_| on_event.call(UiEvent::StashApply(index)),
                        "Apply"
                    }
                    button {
                        r#type: "button",
                        style: "border:1px solid var(--gb-border-strong);background:transparent;color:var(--gb-text-muted);border-radius:var(--gb-radius);\
                                padding:0.25rem 0.55rem;cursor:pointer;font-size:0.75rem;",
                        onclick: move |_| on_event.call(UiEvent::StashPop(index)),
                        "Pop"
                    }
                    button {
                        r#type: "button",
                        style: "border:1px solid var(--gb-danger-border);background:transparent;color:var(--gb-danger);border-radius:var(--gb-radius);\
                                padding:0.25rem 0.55rem;cursor:pointer;font-size:0.75rem;",
                        onclick: move |_| on_event.call(UiEvent::RequestDropStash(index)),
                        "Drop…"
                    }
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
    changed_paths: Vec<std::path::PathBuf>,
    on_event: EventHandler<UiEvent>,
) -> Element {
    let type_suggestions = crate::app::conventional::suggest_types(&message);
    let scope_suggestions = crate::app::conventional::suggest_scopes(&message, &changed_paths);
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
                        padding:0.45rem 0.55rem;border-radius:var(--gb-radius);border:1px solid var(--gb-border-strong);\
                        background:var(--gb-bg);color:var(--gb-text);font-size:0.85rem;font-family:inherit;",
                placeholder: format!(
                    "feat(scope): … (type chips below, {}Enter to commit)",
                    crate::platform::mod_key_label()
                ),
                value: "{message}",
                onfocus: move |_| on_event.call(UiEvent::SetTyping(true)),
                onblur: move |_| on_event.call(UiEvent::SetTyping(false)),
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
                    } else if !shortcut {
                        // Keep typing local — don't let shell keybinds steal characters.
                        evt.stop_propagation();
                    }
                },
            }
            if !type_suggestions.is_empty() {
                div {
                    style: "display:flex;flex-wrap:wrap;gap:0.3rem;",
                    for ty in type_suggestions {
                        {
                            let ty_s = ty.to_string();
                            let msg = message.clone();
                            rsx! {
                                button {
                                    key: "{ty}",
                                    r#type: "button",
                                    style: "border:1px solid var(--gb-border-strong);background:var(--gb-chip);color:var(--gb-chip-text);\
                                            border-radius:var(--gb-radius);padding:0.15rem 0.4rem;cursor:pointer;font-size:0.7rem;",
                                    onclick: move |_| {
                                        let next = crate::app::conventional::apply_type(&msg, &ty_s);
                                        on_event.call(UiEvent::SetCommitMessage(next));
                                    },
                                    "{ty}"
                                }
                            }
                        }
                    }
                }
            }
            if !scope_suggestions.is_empty() {
                div {
                    style: "display:flex;flex-wrap:wrap;gap:0.3rem;",
                    for scope in scope_suggestions {
                        {
                            let scope_s = scope.clone();
                            let msg = message.clone();
                            rsx! {
                                button {
                                    key: "{scope}",
                                    r#type: "button",
                                    style: "border:1px solid var(--gb-border-strong);background:transparent;color:var(--gb-text-muted);\
                                            border-radius:var(--gb-radius);padding:0.15rem 0.4rem;cursor:pointer;font-size:0.7rem;",
                                    onclick: move |_| {
                                        let next = crate::app::conventional::apply_scope(&msg, &scope_s);
                                        on_event.call(UiEvent::SetCommitMessage(next));
                                    },
                                    "({scope})"
                                }
                            }
                        }
                    }
                }
            }
            div {
                style: "display:flex;align-items:center;gap:0.5rem;",
                button {
                    style: "border:0;background:var(--gb-accent);color:white;border-radius:var(--gb-radius);\
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
            p {
                style: "margin:0;font-size:0.75rem;opacity:0.5;",
                "C focus · ⌘Enter commit · conventional type chips"
            }
        }
    }
}

#[component]
fn CopyButton(label: String, text: String, on_event: EventHandler<UiEvent>) -> Element {
    rsx! {
        button {
            style: "border:1px solid var(--gb-border-strong);background:transparent;color:var(--gb-text-muted);\
                    border-radius:var(--gb-radius);padding:0.15rem 0.45rem;cursor:pointer;font-size:0.68rem;",
            onclick: move |_| on_event.call(UiEvent::CopyText(text.clone())),
            "{label}"
        }
    }
}

#[component]
fn RemoteLinkActions(url: String, on_event: EventHandler<UiEvent>) -> Element {
    rsx! {
        div {
            style: "display:flex;gap:0.35rem;flex-wrap:wrap;align-items:center;",
            button {
                style: "border:1px solid var(--gb-border-strong);background:transparent;color:var(--gb-text-muted);\
                        border-radius:var(--gb-radius);padding:0.15rem 0.45rem;cursor:pointer;font-size:0.68rem;",
                onclick: move |_| on_event.call(UiEvent::OpenUrl(url.clone())),
                "Open"
            }
            CopyButton {
                label: "Copy link".to_string(),
                text: url.clone(),
                on_event: on_event,
            }
        }
    }
}

#[component]
fn IssueLinkList(
    text: String,
    origin_web: Option<crate::git::remote_link::RemoteWeb>,
    on_event: EventHandler<UiEvent>,
) -> Element {
    let refs = crate::app::issue_link::detect_issue_refs(&text);
    if refs.is_empty() {
        return rsx! { Fragment {} };
    }
    let items: Vec<(String, Option<String>)> = refs
        .into_iter()
        .map(|r| {
            let url = origin_web
                .as_ref()
                .and_then(|web| crate::app::issue_link::resolve_issue_url(web, &r));
            (r.raw, url)
        })
        .collect();
    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:0.3rem;",
            div {
                style: "font-size:0.72rem;letter-spacing:0.05em;text-transform:uppercase;opacity:0.55;",
                "Issues / PRs"
            }
            for (label, url) in items {
                div {
                    key: "{label}",
                    style: "display:flex;align-items:center;gap:0.4rem;flex-wrap:wrap;font-size:0.8rem;",
                    span { style: "opacity:0.85;font-family:var(--gb-mono);", "{label}" }
                    if let Some(url) = url {
                        RemoteLinkActions { url: url, on_event: on_event }
                    }
                }
            }
        }
    }
}

fn remote_file_link(state: &AppState) -> Option<String> {
    let web = state.repository.origin_web.as_ref()?;
    let path = state.selection.file.as_ref()?;
    let rev = state
        .repository
        .head
        .oid
        .as_ref()
        .map(|o| o.0.as_str())
        .or(state.repository.head.branch.as_deref())?;
    let path_str = path.to_string_lossy();
    Some(crate::git::remote_link::file_url(web, rev, &path_str))
}

fn remote_history_link(state: &AppState) -> Option<String> {
    let web = state.repository.origin_web.as_ref()?;
    let rev = state
        .repository
        .head
        .oid
        .as_ref()
        .map(|o| o.0.as_str())
        .or(state.repository.head.branch.as_deref())?;
    match &state.history.filter {
        HistoryFilter::File { path } => {
            let path_str = path.to_string_lossy();
            Some(crate::git::remote_link::file_url(web, rev, &path_str))
        }
        HistoryFilter::Line { path, line } => {
            let path_str = path.to_string_lossy();
            Some(crate::git::remote_link::line_url(
                web, rev, &path_str, *line,
            ))
        }
        HistoryFilter::All => None,
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
