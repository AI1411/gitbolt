//! Changes (staging area) view — lists files and opens diffs (issue #12).

use dioxus::prelude::*;

use crate::app::event::UiEvent;
use crate::app::layout_prefs::split_path_display;
use crate::app::model::{ChangeKind, FileChange};
use crate::app::state::AppState;
use crate::ui::error_banner::ConfirmPanel;
use crate::ui::list_search::{matches_query, ListSearchBar};
#[derive(Props, Clone, PartialEq)]
pub struct ChangesViewProps {
    pub state: AppState,
    pub on_event: EventHandler<UiEvent>,
}

/// Staging / unstaged / conflicted file list.
#[component]
pub fn ChangesView(props: ChangesViewProps) -> Element {
    let staged = props.state.changes.staged.clone();
    let unstaged: Vec<FileChange> = props
        .state
        .changes
        .unstaged
        .iter()
        .chain(props.state.changes.untracked.iter())
        .cloned()
        .collect();
    let conflicted = props.state.changes.conflicted.clone();
    let loaded = props.state.changes.loaded;
    let selected = props.state.diff.target.clone();
    let q = props.state.ui.search_query.clone();
    let staged: Vec<FileChange> = staged
        .iter()
        .filter(|f| matches_query(&f.path.display().to_string(), &q))
        .cloned()
        .collect();
    let unstaged: Vec<FileChange> = unstaged
        .into_iter()
        .filter(|f| matches_query(&f.path.display().to_string(), &q))
        .collect();
    let conflicted: Vec<FileChange> = conflicted
        .iter()
        .filter(|f| matches_query(&f.path.display().to_string(), &q))
        .cloned()
        .collect();

    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:0.75rem;font-size:0.9rem;",
            ListSearchBar {
                state: props.state.clone(),
                on_event: props.on_event,
                placeholder: "Filter files…".to_string(),
            }
            if let Some(kind) = props.state.ui.confirm_bulk {
                ConfirmPanel {
                    message: match kind {
                        crate::app::state::BulkConfirm::StageAll => {
                            "Stage all files?".to_string()
                        }
                        crate::app::state::BulkConfirm::UnstageAll => {
                            "Unstage all files?".to_string()
                        }
                        crate::app::state::BulkConfirm::StashSave => {
                            "Stash working tree changes?".to_string()
                        }
                    },
                    confirm_label: "Confirm".to_string(),
                    on_confirm: move |()| props.on_event.call(UiEvent::ConfirmBulk),
                    on_cancel: move |()| props.on_event.call(UiEvent::CancelBulk),
                }
            }
            if !loaded {
                p { style: "margin:0;opacity:0.6;", "Loading status…" }
            }
            div {
                style: "display:flex;gap:0.4rem;flex-wrap:wrap;",
                button {
                    style: "padding:0.3rem 0.55rem;border:0;border-radius:4px;cursor:pointer;\
                            background:#3d8bfd;color:white;font-size:0.75rem;font-weight:600;",
                    onclick: move |_| props.on_event.call(UiEvent::StageAll),
                    "Stage all"
                }
                button {
                    style: "padding:0.3rem 0.55rem;border:1px solid #334155;border-radius:4px;cursor:pointer;\
                            background:transparent;color:#9fb0c7;font-size:0.75rem;",
                    onclick: move |_| props.on_event.call(UiEvent::UnstageAll),
                    "Unstage all"
                }
            }
            p {
                style: "margin:0;opacity:0.45;font-size:0.75rem;",
                "j / k move · Space stage/unstage · click opens diff"
            }
            FileSection {
                title: "STAGED",
                files: staged.clone(),
                staged_area: true,
                selected: selected.clone(),
                on_event: props.on_event,
            }
            FileSection {
                title: "CONFLICTED",
                files: conflicted.clone(),
                staged_area: false,
                selected: selected.clone(),
                on_event: props.on_event,
            }
            FileSection {
                title: "UNSTAGED",
                files: unstaged.clone(),
                staged_area: false,
                selected: selected,
                on_event: props.on_event,
            }
        }
    }
}

#[component]
fn FileSection(
    title: &'static str,
    files: Vec<FileChange>,
    staged_area: bool,
    selected: Option<crate::app::model::DiffTarget>,
    on_event: EventHandler<UiEvent>,
) -> Element {
    // Hide empty conflicted section to reduce noise.
    if title == "CONFLICTED" && files.is_empty() {
        return rsx! {};
    }

    rsx! {
        div {
            h3 {
                style: "margin:0 0 0.35rem;font-size:0.75rem;letter-spacing:0.06em;\
                        text-transform:uppercase;opacity:0.6;",
                "{title} ({files.len()})"
            }
            if files.is_empty() {
                p {
                    style: "margin:0;opacity:0.55;font-size:0.8rem;line-height:1.4;",
                    if title == "STAGED" {
                        "Stage したファイルがここに出ます。Space で移動"
                    } else if title == "UNSTAGED" {
                        "未 stage の変更がここに出ます。Space で STAGED へ"
                    } else {
                        "—"
                    }
                }
            } else {
                ul {
                    style: "list-style:none;margin:0;padding:0;display:flex;flex-direction:column;gap:0.2rem;",
                    for file in files.into_iter() {
                        {
                            let path = file.path.clone();
                            let path_stage = file.path.clone();
                            let mark = status_mark(file.kind);
                            let (parent, name) = split_path_display(&path);
                            let is_conflict = title == "CONFLICTED";
                            let is_sel = selected.as_ref().is_some_and(|t| {
                                t.path == path && t.staged == staged_area
                            });
                            let bg = if is_sel { "#1e3a5f" } else { "transparent" };
                            rsx! {
                                li {
                                    style: "display:flex;align-items:center;gap:0.35rem;",
                                    button {
                                        style: format!(
                                            "flex:1;text-align:left;border:0;background:{bg};\
                                             color:#e8eef7;cursor:pointer;font-family:ui-monospace,monospace;\
                                             font-size:0.82rem;padding:0.2rem 0.35rem;border-radius:3px;"
                                        ),
                                        onclick: move |_| {
                                            on_event.call(UiEvent::SelectFile {
                                                path: path.clone(),
                                                staged: staged_area,
                                            });
                                        },
                                        span { style: "opacity:0.7;margin-right:0.5rem;", "{mark}" }
                                        span { style: "opacity:0.45;", "{parent}" }
                                        span { "{name}" }
                                    }
                                    if is_conflict {
                                        span {
                                            style: "font-size:0.68rem;opacity:0.55;white-space:nowrap;",
                                            "resolve in editor"
                                        }
                                    }
                                    button {
                                        style: "border:1px solid #334155;background:transparent;color:#9fb0c7;\
                                                border-radius:4px;padding:0.1rem 0.4rem;cursor:pointer;font-size:0.7rem;",
                                        onclick: move |_| {
                                            if staged_area {
                                                on_event.call(UiEvent::UnstageFile(path_stage.clone()));
                                            } else {
                                                on_event.call(UiEvent::StageFile(path_stage.clone()));
                                            }
                                        },
                                        if staged_area { "Unstage" } else { "Stage" }
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

fn status_mark(kind: ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Added => "A",
        ChangeKind::Modified => "M",
        ChangeKind::Deleted => "D",
        ChangeKind::Renamed => "R",
        ChangeKind::Copied => "C",
        ChangeKind::TypeChanged => "T",
        ChangeKind::Untracked => "?",
        ChangeKind::Conflicted => "U",
    }
}
