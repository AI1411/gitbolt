//! Worktree First view: list / create / remove / open (issue #20).

use std::path::PathBuf;

use dioxus::prelude::*;

use crate::app::event::UiEvent;
use crate::app::model::WorktreeInfo;
use crate::app::state::AppState;
use crate::git::worktree::default_worktree_path;
use crate::ui::error_banner::ConfirmPanel;

/// Props for the worktrees pane.
#[derive(Props, Clone, PartialEq)]
pub struct WorktreesViewProps {
    pub state: AppState,
    pub on_event: EventHandler<UiEvent>,
}

/// Worktree First list with create form and branches without worktrees.
#[component]
pub fn WorktreesView(props: WorktreesViewProps) -> Element {
    let mut branch_draft = use_signal(String::new);
    let mut path_draft = use_signal(String::new);
    let repo_path = props.state.repository.path.clone();
    let current = repo_path.clone();
    let pending = props.state.ui.confirm_remove_worktree.clone();
    let trees = props.state.worktree.worktrees.clone();
    let loaded = props.state.worktree.loaded;

    let without: Vec<String> = branches_without_worktree(&props.state);

    rsx! {
        div {
            style: "font-size:0.9rem;opacity:0.95;display:flex;flex-direction:column;gap:0.75rem;",

            // Create
            div {
                style: "display:flex;flex-direction:column;gap:0.35rem;",
                h3 {
                    style: "margin:0;font-size:0.75rem;letter-spacing:0.06em;\
                            text-transform:uppercase;opacity:0.6;",
                    "Create worktree"
                }
                div {
                    style: "display:flex;gap:0.4rem;flex-wrap:wrap;align-items:center;",
                    input {
                        style: "flex:1;min-width:7rem;padding:0.35rem 0.5rem;border-radius:4px;\
                                border:1px solid #334155;background:#0f1419;color:#e8eef7;font-size:0.85rem;",
                        placeholder: "branch",
                        value: "{branch_draft()}",
                        oninput: move |evt| {
                            let name = evt.value();
                            branch_draft.set(name.clone());
                            if let Some(repo) = &repo_path {
                                if !name.trim().is_empty() {
                                    path_draft.set(
                                        default_worktree_path(repo, name.trim())
                                            .display()
                                            .to_string(),
                                    );
                                }
                            }
                        },
                    }
                    input {
                        style: "flex:2;min-width:10rem;padding:0.35rem 0.5rem;border-radius:4px;\
                                border:1px solid #334155;background:#0f1419;color:#e8eef7;font-size:0.85rem;",
                        placeholder: "path",
                        value: "{path_draft()}",
                        oninput: move |evt| path_draft.set(evt.value()),
                    }
                    button {
                        style: "border:0;background:#3d8bfd;color:white;border-radius:4px;\
                                padding:0.35rem 0.7rem;cursor:pointer;font-size:0.8rem;",
                        onclick: move |_| {
                            let branch = branch_draft().trim().to_string();
                            let path = PathBuf::from(path_draft().trim());
                            if !branch.is_empty() && !path.as_os_str().is_empty() {
                                props.on_event.call(UiEvent::CreateWorktree { branch, path });
                                branch_draft.set(String::new());
                                path_draft.set(String::new());
                            }
                        },
                        "Create"
                    }
                }
            }

            if let Some(path) = pending.clone() {
                ConfirmPanel {
                    message: format!("Remove worktree {}?", path.display()),
                    confirm_label: String::from("Remove"),
                    on_confirm: move |()| props.on_event.call(UiEvent::ConfirmRemoveWorktree),
                    on_cancel: move |()| props.on_event.call(UiEvent::CancelRemoveWorktree),
                }
            }

            if !loaded {
                p { style: "margin:0;opacity:0.6;", "Loading worktrees…" }
            } else {
                ul {
                    style: "list-style:none;margin:0;padding:0;display:flex;flex-direction:column;gap:0.35rem;",
                    for w in trees.iter().cloned() {
                        {
                            let key = w.path.display().to_string();
                            rsx! {
                                WorktreeRow {
                                    key: "{key}",
                                    worktree: w,
                                    current: current.clone(),
                                    on_event: props.on_event,
                                }
                            }
                        }
                    }
                }
            }

            if !without.is_empty() {
                div {
                    h3 {
                        style: "margin:0.35rem 0;font-size:0.75rem;letter-spacing:0.06em;\
                                text-transform:uppercase;opacity:0.6;",
                        "Branches without Worktree"
                    }
                    ul {
                        style: "list-style:none;margin:0;padding:0;display:flex;flex-direction:column;gap:0.25rem;",
                        for name in without {
                            {
                                let branch = name.clone();
                                let repo = props.state.repository.path.clone();
                                rsx! {
                                    li {
                                        style: "display:flex;align-items:center;gap:0.5rem;",
                                        span {
                                            style: "font-family:ui-monospace,monospace;font-size:0.85rem;",
                                            "{name}"
                                        }
                                        button {
                                            style: "border:1px solid #334155;background:transparent;color:#9fb0c7;\
                                                    border-radius:4px;padding:0.15rem 0.45rem;cursor:pointer;font-size:0.72rem;",
                                            onclick: move |_| {
                                                if let Some(repo) = &repo {
                                                    let path = default_worktree_path(repo, &branch);
                                                    props.on_event.call(UiEvent::CreateWorktree {
                                                        branch: branch.clone(),
                                                        path,
                                                    });
                                                }
                                            },
                                            "Create worktree"
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
fn WorktreeRow(
    worktree: WorktreeInfo,
    current: Option<PathBuf>,
    on_event: EventHandler<UiEvent>,
) -> Element {
    let path = worktree.path.clone();
    let open_path = worktree.path.clone();
    let remove_path = worktree.path.clone();
    let is_current = current
        .as_ref()
        .and_then(|c| c.canonicalize().ok())
        .zip(worktree.path.canonicalize().ok())
        .is_some_and(|(a, b)| a == b);
    let branch = worktree
        .branch
        .clone()
        .unwrap_or_else(|| "(detached)".into());
    let marker = if is_current || worktree.is_primary {
        "●"
    } else {
        "○"
    };

    rsx! {
        li {
            style: "display:flex;align-items:center;gap:0.5rem;flex-wrap:wrap;\
                    padding:0.3rem 0.35rem;border-radius:4px;border:1px solid #1e293b;",
            span { style: "opacity:0.7;", "{marker}" }
            span {
                style: "font-family:ui-monospace,monospace;font-size:0.85rem;min-width:7rem;",
                "{branch}"
            }
            span {
                style: "flex:1;font-family:ui-monospace,monospace;font-size:0.75rem;opacity:0.65;",
                "{path.display()}"
            }
            if !is_current {
                button {
                    style: "border:1px solid #334155;background:transparent;color:#9fb0c7;\
                            border-radius:4px;padding:0.15rem 0.45rem;cursor:pointer;font-size:0.72rem;",
                    onclick: move |_| {
                        on_event.call(UiEvent::OpenRepository(open_path.clone()));
                    },
                    "Open"
                }
            }
            if !worktree.is_primary {
                button {
                    style: "border:1px solid #7f1d1d;background:transparent;color:#fca5a5;\
                            border-radius:4px;padding:0.15rem 0.45rem;cursor:pointer;font-size:0.72rem;",
                    onclick: move |_| {
                        on_event.call(UiEvent::RequestRemoveWorktree(remove_path.clone()));
                    },
                    "Remove"
                }
            }
        }
    }
}

fn branches_without_worktree(state: &AppState) -> Vec<String> {
    let occupied: std::collections::HashSet<&str> = state
        .worktree
        .worktrees
        .iter()
        .filter_map(|w| w.branch.as_deref())
        .collect();
    state
        .branch
        .branches
        .iter()
        .filter(|b| !b.is_remote && !occupied.contains(b.name.as_str()))
        .map(|b| b.name.clone())
        .collect()
}
