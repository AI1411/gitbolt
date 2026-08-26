//! Repository Pulse header (issue #11).

use dioxus::prelude::*;

use crate::app::event::UiEvent;
use crate::app::pulse::{format_divergence, segment_view, summary, PulseSegment};
use crate::app::state::AppState;

/// Props for the pulse header strip.
#[derive(Props, Clone, PartialEq)]
pub struct PulseHeaderProps {
    pub state: AppState,
    pub on_event: EventHandler<UiEvent>,
}

/// Always-visible one-line repository summary.
#[component]
pub fn PulseHeader(props: PulseHeaderProps) -> Element {
    let pulse = summary(&props.state);
    let divergence = format_divergence(&pulse);
    let changes_label = format!("{} changes", pulse.changes);
    let staged_label = format!("{} staged", pulse.staged);
    let worktrees_label = format!("{} worktrees", pulse.worktrees);

    let btn = "border:0;background:transparent;color:inherit;cursor:pointer;padding:0;\
               font:inherit;font-weight:600;";
    let muted = "border:0;background:transparent;color:inherit;cursor:pointer;padding:0;\
                 font:inherit;font-weight:500;opacity:0.75;";

    rsx! {
        header {
            style: "flex:0 0 auto;padding:0.55rem 0.85rem;border-bottom:1px solid #243044;\
                    font-weight:600;font-size:0.9rem;display:flex;flex-wrap:wrap;gap:0.35rem 0.55rem;\
                    align-items:center;",
            span { style: "opacity:0.9;", "GitBolt /" }
            button {
                style: "{btn}",
                title: "Branches",
                onclick: move |_| {
                    props
                        .on_event
                        .call(UiEvent::SelectView(segment_view(PulseSegment::Branch)));
                },
                "{pulse.branch_label}"
            }
            if !divergence.is_empty() {
                button {
                    style: "{muted}",
                    title: "Branch health",
                    onclick: move |_| {
                        props.on_event.call(UiEvent::SelectView(segment_view(
                            PulseSegment::Divergence,
                        )));
                    },
                    "{divergence}"
                }
            }
            span { style: "opacity:0.35;", "·" }
            button {
                style: "{muted}",
                title: "Changes",
                onclick: move |_| {
                    props
                        .on_event
                        .call(UiEvent::SelectView(segment_view(PulseSegment::Changes)));
                },
                "{changes_label}"
            }
            span { style: "opacity:0.35;", "·" }
            button {
                style: "{muted}",
                title: "Staged",
                onclick: move |_| {
                    props
                        .on_event
                        .call(UiEvent::SelectView(segment_view(PulseSegment::Staged)));
                },
                "{staged_label}"
            }
            span { style: "opacity:0.35;", "·" }
            button {
                style: "{muted}",
                title: "Worktrees",
                onclick: move |_| {
                    props.on_event.call(UiEvent::SelectView(segment_view(
                        PulseSegment::Worktrees,
                    )));
                },
                "{worktrees_label}"
            }
            span {
                style: "opacity:0.45;font-weight:500;font-size:0.8rem;margin-left:auto;",
                "⌘I context"
            }
        }
    }
}

/// Placeholder retained for module stability; prefer [`PulseHeader`].
pub struct PulseView;
