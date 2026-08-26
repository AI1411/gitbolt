//! Branch list with Branch Health badges and Divergence entry (issue #29).

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
    let current = props
        .state
        .branch
        .current
        .clone()
        .or_else(|| props.state.repository.head.branch.clone())
        .unwrap_or_else(|| "(none)".into());
    let show_div = props.state.divergence.left.is_some() || props.state.divergence.loading;

    rsx! {
        div {
            style: "font-size:0.9rem;opacity:0.95;display:flex;flex-direction:column;gap:0.75rem;",
            p { style: "margin:0;opacity:0.75;", "Current: {current}" }
            if !props.state.branch.loaded {
                p { style: "margin:0;opacity:0.6;", "Loading branches…" }
            } else {
                ul {
                    style: "list-style:none;margin:0;padding:0;display:flex;flex-direction:column;gap:0.3rem;",
                    for b in props.state.branch.branches.iter() {
                        BranchRow {
                            branch: b.clone(),
                            on_event: props.on_event,
                        }
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
fn BranchRow(branch: BranchInfo, on_event: EventHandler<UiEvent>) -> Element {
    let badge = health_badge(&branch);
    let name = branch.name.clone();
    let other = branch.name.clone();

    rsx! {
        li {
            style: "display:flex;align-items:center;gap:0.5rem;padding:0.25rem 0.35rem;\
                    border-radius:4px;",
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
                onclick: move |_| {
                    on_event.call(UiEvent::ShowDivergence {
                        other: other.clone(),
                    });
                },
                "Divergence"
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
