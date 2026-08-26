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
    let ahead = pulse.ahead.unwrap_or(0);

    let btn = "border:0;background:transparent;color:inherit;cursor:pointer;padding:0;\
               font:inherit;font-weight:600;";
    let muted = "border:0;background:transparent;color:inherit;cursor:pointer;padding:0;\
                 font:inherit;font-weight:500;opacity:0.75;";
    let push_style = if ahead > 0 {
        "border:0;background:var(--gb-accent);color:white;cursor:pointer;padding:0.15rem 0.45rem;\
         font:inherit;font-weight:600;border-radius:var(--gb-radius);"
    } else {
        muted
    };

    rsx! {
        header {
            style: "flex:0 0 auto;padding:0.55rem 0.85rem;border-bottom:1px solid var(--gb-border);\
                    font-weight:var(--gb-weight-semibold);font-size:var(--gb-size-pulse);display:flex;flex-wrap:wrap;gap:0.35rem 0.55rem;\
                    align-items:center;letter-spacing:-0.01em;",
            span {
                style: "opacity:0.9;cursor:pointer;",
                title: "Switch repository",
                onclick: move |_| props.on_event.call(UiEvent::CloseRepository),
                "GitBolt /"
            }
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
                style: "opacity:0.45;font-weight:500;font-size:0.8rem;margin-left:auto;\
                        display:flex;gap:0.45rem;align-items:center;",
                if let Some(label) = props.state.background.remote_label.clone() {
                    span { style: "opacity:0.8;", "{label}" }
                } else if let Some(status) = props.state.ui.remote_status.clone() {
                    span { style: "opacity:0.85;color:var(--gb-add);", "{status}" }
                } else if props.state.background.inflight > 0 {
                    span { style: "opacity:0.7;", "working…" }
                }
                button {
                    style: "{muted}",
                    title: "Fetch (F)",
                    onclick: move |_| props.on_event.call(UiEvent::Fetch),
                    "Fetch"
                }
                button {
                    style: "{muted}",
                    title: "Pull",
                    onclick: move |_| props.on_event.call(UiEvent::Pull),
                    "Pull"
                }
                button {
                    style: "{push_style}",
                    title: if ahead > 0 {
                        format!("Push ({ahead} ahead)")
                    } else {
                        "Push".into()
                    },
                    onclick: move |_| props.on_event.call(UiEvent::Push),
                    if ahead > 0 { "Push ↑{ahead}" } else { "Push" }
                }
                button {
                    style: "{muted}",
                    title: "Switch repository",
                    onclick: move |_| props.on_event.call(UiEvent::CloseRepository),
                    "Repos…"
                }
                span { "{crate::platform::mod_key_label()}I context · ? help" }
            }
        }
    }
}

/// Placeholder retained for module stability; prefer [`PulseHeader`].
pub struct PulseView;
