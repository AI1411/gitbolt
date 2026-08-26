//! Branch list with Recent, last commit, tracking, and Quick Open (issue #30).

use dioxus::prelude::*;

use crate::app::event::UiEvent;
use crate::app::model::{BranchHealth, BranchInfo};
use crate::app::state::AppState;
use crate::ui::divergence::DivergenceView;

/// Props for the branches pane.
#[derive(Props, Clone, PartialEq)]
pub struct BranchesViewProps {
    pub state: AppState,
    pub on_event: EventHandler<UiEvent>,
}

/// Branches list + optional Divergence panel.
#[component]
pub fn BranchesView(props: BranchesViewProps) -> Element {
    let mut upstream_draft = use_signal(String::new);
    let mut upstream_target = use_signal(|| None::<String>);

    let current = props
        .state
        .branch
        .current
        .clone()
        .or_else(|| props.state.repository.head.branch.clone())
        .unwrap_or_else(|| "(none)".into());
    let show_div = props.state.divergence.left.is_some() || props.state.divergence.loading;
    let filter = props.state.branch.filter.to_ascii_lowercase();
    let filtered: Vec<BranchInfo> = props
        .state
        .branch
        .branches
        .iter()
        .filter(|b| filter.is_empty() || b.name.to_ascii_lowercase().contains(&filter))
        .cloned()
        .collect();

    rsx! {
        div {
            style: "font-size:0.9rem;opacity:0.95;display:flex;flex-direction:column;gap:0.75rem;",
            p { style: "margin:0;opacity:0.75;", "Current: {current}" }

            // Quick Open filter for branches
            input {
                style: "width:100%;box-sizing:border-box;padding:0.4rem 0.55rem;border-radius:4px;\
                        border:1px solid #334155;background:#0f1419;color:#e8eef7;font-size:0.85rem;",
                placeholder: "Quick Open branches (/)",
                value: "{props.state.branch.filter}",
                oninput: move |evt| {
                    props.on_event.call(UiEvent::SetBranchFilter(evt.value()));
                },
            }

            if !props.state.branch.recent.is_empty() {
                div {
                    h3 {
                        style: "margin:0 0 0.35rem;font-size:0.75rem;letter-spacing:0.06em;\
                                text-transform:uppercase;opacity:0.6;",
                        "Recent"
                    }
                    ul {
                        style: "list-style:none;margin:0;padding:0;display:flex;flex-wrap:wrap;gap:0.35rem;",
                        for name in props.state.branch.recent.iter().cloned() {
                            {
                                let n = name.clone();
                                rsx! {
                                    li {
                                        button {
                                            style: "border:1px solid #334155;background:#151b24;color:#cbd5e1;\
                                                    border-radius:4px;padding:0.2rem 0.5rem;cursor:pointer;\
                                                    font-family:ui-monospace,monospace;font-size:0.75rem;",
                                            onclick: move |_| {
                                                props.on_event.call(UiEvent::SelectBranch(n.clone()));
                                            },
                                            "{name}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !props.state.branch.loaded {
                p { style: "margin:0;opacity:0.6;", "Loading branches…" }
            } else {
                ul {
                    style: "list-style:none;margin:0;padding:0;display:flex;flex-direction:column;gap:0.35rem;",
                    for b in filtered.into_iter() {
                        BranchRow {
                            branch: b,
                            on_event: props.on_event,
                            on_track: move |name| {
                                upstream_target.set(Some(name));
                                upstream_draft.set(String::new());
                            },
                        }
                    }
                }
            }

            if let Some(branch) = upstream_target() {
                div {
                    style: "display:flex;gap:0.4rem;align-items:center;flex-wrap:wrap;",
                    span { style: "font-size:0.8rem;opacity:0.7;", "Track for {branch}:" }
                    input {
                        style: "flex:1;min-width:8rem;padding:0.3rem 0.45rem;border-radius:4px;\
                                border:1px solid #334155;background:#0f1419;color:#e8eef7;font-size:0.8rem;",
                        placeholder: "origin/main",
                        value: "{upstream_draft()}",
                        oninput: move |evt| upstream_draft.set(evt.value()),
                    }
                    button {
                        style: "border:0;background:#3d8bfd;color:white;border-radius:4px;\
                                padding:0.3rem 0.65rem;cursor:pointer;font-size:0.78rem;",
                        onclick: move |_| {
                            let upstream = upstream_draft().trim().to_string();
                            if !upstream.is_empty() {
                                props.on_event.call(UiEvent::SetUpstream {
                                    branch: branch.clone(),
                                    upstream,
                                });
                                upstream_target.set(None);
                            }
                        },
                        "Set upstream"
                    }
                    button {
                        style: "border:1px solid #334155;background:transparent;color:#9fb0c7;\
                                border-radius:4px;padding:0.3rem 0.55rem;cursor:pointer;font-size:0.78rem;",
                        onclick: move |_| upstream_target.set(None),
                        "Cancel"
                    }
                }
            }

            if show_div {
                DivergenceView {
                    state: props.state.divergence.clone(),
                    on_event: props.on_event,
                }
            }
        }
    }
}

#[component]
fn BranchRow(
    branch: BranchInfo,
    on_event: EventHandler<UiEvent>,
    on_track: EventHandler<String>,
) -> Element {
    let badge = health_badge(&branch);
    let name = branch.name.clone();
    let other = branch.name.clone();
    let track_name = branch.name.clone();
    let last = branch.last_commit.as_ref().map_or_else(String::new, |c| {
        let short = if c.oid.0.len() > 7 {
            c.oid.0[..7].to_string()
        } else {
            c.oid.0.clone()
        };
        format!("{short} · {}", c.summary)
    });
    let upstream = branch
        .upstream
        .clone()
        .unwrap_or_else(|| "no upstream".into());

    rsx! {
        li {
            style: "display:flex;flex-direction:column;gap:0.15rem;padding:0.3rem 0.35rem;\
                    border-radius:4px;border:1px solid #1e293b;",
            div {
                style: "display:flex;align-items:center;gap:0.5rem;",
                button {
                    style: "flex:1;text-align:left;border:0;background:transparent;color:#e8eef7;\
                            cursor:pointer;font-family:ui-monospace,monospace;font-size:0.85rem;",
                    onclick: move |_| on_event.call(UiEvent::SelectBranch(name.clone())),
                    "{branch.name}"
                }
                span { style: "opacity:0.7;font-size:0.8rem;min-width:3.5rem;", "{badge}" }
                button {
                    style: "border:1px solid #334155;background:transparent;color:#9fb0c7;\
                            border-radius:4px;padding:0.15rem 0.45rem;cursor:pointer;font-size:0.72rem;",
                    onclick: move |_| on_track.call(track_name.clone()),
                    "Track"
                }
                button {
                    style: "border:1px solid #334155;background:transparent;color:#9fb0c7;\
                            border-radius:4px;padding:0.15rem 0.45rem;cursor:pointer;font-size:0.72rem;",
                    onclick: move |_| {
                        on_event.call(UiEvent::ShowDivergence {
                            other: other.clone(),
                        });
                    },
                    "Divergence"
                }
            }
            div {
                style: "font-size:0.72rem;opacity:0.55;font-family:ui-monospace,monospace;",
                "{upstream}"
                if !last.is_empty() {
                    span { " · {last}" }
                }
            }
        }
    }
}

fn health_badge(b: &BranchInfo) -> String {
    match b.health {
        BranchHealth::Synced => "✓".into(),
        BranchHealth::Ahead => format!("↑{}", b.ahead),
        BranchHealth::Behind => format!("↓{}", b.behind),
        BranchHealth::Diverged => format!("↑{}↓{}", b.ahead, b.behind),
        BranchHealth::Stale => "◌".into(),
        BranchHealth::Local => "local".into(),
    }
}
