//! Diff view with line selection and Change Origin (issues #28 / #31).

use dioxus::prelude::*;

use crate::app::event::UiEvent;
use crate::app::model::{Loadable, Oid};
use crate::app::state::AppState;

/// Props for the interactive diff pane.
#[derive(Props, Clone, PartialEq)]
pub struct DiffViewProps {
    pub state: AppState,
    pub on_event: EventHandler<UiEvent>,
}

/// Diff content with clickable +/- lines, stage-selected action, and origins.
#[component]
pub fn DiffView(props: DiffViewProps) -> Element {
    let selected = &props.state.diff.selected_lines;
    let staged = props.state.diff.target.as_ref().is_some_and(|t| t.staged);

    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:0.5rem;min-height:4rem;",

            div {
                style: "display:flex;gap:0.5rem;align-items:center;",
                if !selected.is_empty() {
                    button {
                        style: "padding:0.35rem 0.75rem;border:0;border-radius:4px;cursor:pointer;\
                                background:#3d8bfd;color:white;font-size:0.8rem;font-weight:600;",
                        onclick: move |_| {
                            if staged {
                                props.on_event.call(UiEvent::UnstageSelectedLines);
                            } else {
                                props.on_event.call(UiEvent::StageSelectedLines);
                            }
                        },
                        if staged {
                            "Unstage {selected.len()} line(s)"
                        } else {
                            "Stage {selected.len()} line(s)"
                        }
                    }
                    button {
                        style: "padding:0.35rem 0.75rem;border:1px solid #334155;border-radius:4px;\
                                cursor:pointer;background:transparent;color:#9fb0c7;font-size:0.8rem;",
                        onclick: move |_| props.on_event.call(UiEvent::ClearDiffLineSelection),
                        "Clear"
                    }
                } else {
                    span {
                        style: "opacity:0.55;font-size:0.8rem;",
                        "Click + / − lines to select · origin chips show HEAD blame"
                    }
                }
            }

            match &props.state.diff.content {
                Loadable::Ready(content) => rsx! {
                    div {
                        style: "padding:0.5rem 0;border:1px solid #243044;border-radius:6px;\
                                background:#151b24;font-family:ui-monospace,monospace;font-size:0.82rem;\
                                overflow:auto;max-height:50vh;",
                        for hunk in content.hunks.iter() {
                            div {
                                style: "padding:0.25rem 0.65rem;opacity:0.55;background:#0f1419;",
                                "{hunk.header}"
                            }
                            for line in hunk.lines.iter() {
                                {
                                    let idx = line.body_index;
                                    let is_sel = selected.contains(&idx);
                                    let stageable = line.origin == '+' || line.origin == '-';
                                    let bg = if is_sel {
                                        "#1e3a5f"
                                    } else if line.origin == '+' {
                                        "#13281c"
                                    } else if line.origin == '-' {
                                        "#2a1518"
                                    } else {
                                        "transparent"
                                    };
                                    let color = match line.origin {
                                        '+' => "#86efac",
                                        '-' => "#fca5a5",
                                        _ => "#cbd5e1",
                                    };
                                    let display = format!("{}{}", line.origin, line.content);
                                    let origin = line.change_origin.clone();
                                    rsx! {
                                        div {
                                            key: "{idx}",
                                            style: format!(
                                                "display:flex;align-items:baseline;gap:0.65rem;\
                                                 padding:0.05rem 0.65rem;white-space:pre;cursor:{};\
                                                 background:{};color:{};",
                                                if stageable { "pointer" } else { "default" },
                                                bg,
                                                color,
                                            ),
                                            onclick: move |_| {
                                                if stageable {
                                                    props.on_event.call(UiEvent::ToggleDiffLine(idx));
                                                }
                                            },
                                            span { style: "flex:1;min-width:0;", "{display}" }
                                            if let Some(origin) = origin {
                                                {
                                                    let oid = Oid(origin.oid.0.clone());
                                                    let short = if origin.oid.0.len() > 7 {
                                                        origin.oid.0[..7].to_string()
                                                    } else {
                                                        origin.oid.0.clone()
                                                    };
                                                    let label = format!("{short} · {}", origin.summary);
                                                    rsx! {
                                                        button {
                                                            style: "flex:0 0 auto;max-width:14rem;overflow:hidden;\
                                                                    text-overflow:ellipsis;border:0;background:transparent;\
                                                                    color:#7dd3fc;cursor:pointer;font-size:0.7rem;\
                                                                    font-family:ui-monospace,monospace;padding:0;",
                                                            title: "Change Origin — select commit",
                                                            onclick: move |evt| {
                                                                evt.stop_propagation();
                                                                props.on_event.call(UiEvent::SelectCommit(oid.clone()));
                                                            },
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
                },
                Loadable::Loading => rsx! {
                    div { style: "opacity:0.6;font-size:0.85rem;", "Loading diff…" }
                },
                Loadable::Failed(err) => rsx! {
                    div { style: "color:#fca5a5;font-size:0.85rem;", "Diff error: {err}" }
                },
                Loadable::Idle => rsx! {
                    div {
                        style: "opacity:0.7;font-size:0.85rem;",
                        if let Some(path) = props.state.selection.file.as_ref() {
                            "Selected: {path.display()}"
                        } else {
                            "Select a file to view its diff."
                        }
                    }
                },
            }
        }
    }
}
