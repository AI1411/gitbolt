//! Divergence dual-column presentational view (issue #29).

use dioxus::prelude::*;

use crate::app::event::UiEvent;
use crate::app::state::DivergenceState;

/// Props for the divergence panel.
#[derive(Props, Clone, PartialEq)]
pub struct DivergenceViewProps {
    pub state: DivergenceState,
    pub on_event: EventHandler<UiEvent>,
}

/// Shows commits unique to each side since the merge-base.
#[component]
pub fn DivergenceView(props: DivergenceViewProps) -> Element {
    let left = props.state.left.clone().unwrap_or_else(|| "?".into());
    let right = props.state.right.clone().unwrap_or_else(|| "?".into());
    let base = props.state.merge_base.as_ref().map_or_else(
        || "…".into(),
        |o| {
            let s = &o.0;
            if s.len() > 8 {
                s[..8].to_string()
            } else {
                s.clone()
            }
        },
    );

    rsx! {
        div {
            style: "margin-top:1rem;display:flex;flex-direction:column;gap:0.65rem;",
            div {
                style: "display:flex;align-items:center;gap:0.75rem;",
                h3 {
                    style: "margin:0;font-size:0.95rem;",
                    "Divergence · base {base}"
                }
                button {
                    style: "margin-left:auto;border:1px solid #334155;background:transparent;\
                            color:#9fb0c7;border-radius:4px;padding:0.25rem 0.55rem;cursor:pointer;\
                            font-size:0.75rem;",
                    onclick: move |_| props.on_event.call(UiEvent::ClearDivergence),
                    "Close"
                }
            }
            if props.state.loading {
                p { style: "margin:0;opacity:0.6;font-size:0.85rem;", "Computing merge-base…" }
            } else {
                div {
                    style: "display:grid;grid-template-columns:1fr 1fr;gap:0.75rem;",
                    SideList {
                        title: format!("{left} only ({})", props.state.left_only.len()),
                        commits: props.state.left_only.clone(),
                    }
                    SideList {
                        title: format!("{right} only ({})", props.state.right_only.len()),
                        commits: props.state.right_only.clone(),
                    }
                }
            }
        }
    }
}

#[component]
fn SideList(title: String, commits: Vec<crate::app::model::CommitSummary>) -> Element {
    rsx! {
        div {
            style: "border:1px solid #243044;border-radius:6px;background:#151b24;padding:0.55rem;",
            h4 {
                style: "margin:0 0 0.45rem;font-size:0.8rem;opacity:0.75;",
                "{title}"
            }
            if commits.is_empty() {
                p { style: "margin:0;opacity:0.45;font-size:0.8rem;", "—" }
            } else {
                ul {
                    style: "list-style:none;margin:0;padding:0;display:flex;flex-direction:column;gap:0.35rem;",
                    for c in commits.into_iter() {
                        {
                            let short = if c.oid.0.len() > 7 {
                                c.oid.0[..7].to_string()
                            } else {
                                c.oid.0.clone()
                            };
                            rsx! {
                                li {
                                    style: "font-size:0.8rem;line-height:1.35;",
                                    span {
                                        style: "font-family:ui-monospace,monospace;opacity:0.55;margin-right:0.4rem;",
                                        "{short}"
                                    }
                                    span { "{c.summary}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
