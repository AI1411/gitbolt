//! Changes (staging area) view — lists files and opens diffs (issue #12).

use dioxus::prelude::*;

use crate::app::event::UiEvent;
use crate::app::model::{ChangeKind, FileChange};
use crate::app::state::AppState;

/// Props for the changes list.
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

    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:0.75rem;font-size:0.9rem;margin-top:0.75rem;",
            if !loaded {
                p { style: "margin:0;opacity:0.6;", "Loading status…" }
            }
            p {
                style: "margin:0;opacity:0.45;font-size:0.75rem;",
                "j / k or ↑ / ↓ to move · click to open diff"
            }
            FileSection {
                title: "STAGED",
                files: staged.to_vec(),
                staged_area: true,
                selected: selected.clone(),
                on_event: props.on_event,
            }
            FileSection {
                title: "CONFLICTED",
                files: conflicted.to_vec(),
                staged_area: false,
                selected: selected.clone(),
                on_event: props.on_event,
            }
            FileSection {
                title: "UNSTAGED",
                files: unstaged,
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
                p { style: "margin:0;opacity:0.45;font-size:0.8rem;", "—" }
            } else {
                ul {
                    style: "list-style:none;margin:0;padding:0;display:flex;flex-direction:column;gap:0.2rem;",
                    for file in files.into_iter() {
                        {
                            let path = file.path.clone();
                            let mark = status_mark(file.kind);
                            let label = path.display().to_string();
                            let is_sel = selected.as_ref().is_some_and(|t| {
                                t.path == path && t.staged == staged_area
                            });
                            let bg = if is_sel { "#1e3a5f" } else { "transparent" };
                            rsx! {
                                li {
                                    button {
                                        style: format!(
                                            "width:100%;text-align:left;border:0;background:{bg};\
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
                                        "{label}"
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
