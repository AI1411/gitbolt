//! Repository Pulse header (issue #11 / #112).

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
    let behind = pulse.behind.unwrap_or(0);

    let brand =
        "border:0;background:transparent;color:var(--gb-text-muted);cursor:pointer;padding:0;\
                 font:inherit;font-weight:var(--gb-weight-regular);";
    let branch = "border:0;background:transparent;color:var(--gb-text);cursor:pointer;padding:0;\
                  font:inherit;font-weight:var(--gb-weight-semibold);font-size:0.95em;";
    let meta =
        "border:0;background:transparent;color:var(--gb-text-muted);cursor:pointer;padding:0;\
                font:inherit;font-weight:var(--gb-weight-regular);opacity:0.9;";
    let action = "border:1px solid var(--gb-border-strong);background:transparent;color:var(--gb-text-muted);\
                  cursor:pointer;padding:0.15rem 0.5rem;font:inherit;font-size:var(--gb-size-hint);\
                  font-weight:var(--gb-weight-semibold);border-radius:var(--gb-radius);";
    let push_style = if ahead > 0 {
        "border:0;background:var(--gb-accent);color:white;cursor:pointer;padding:0.15rem 0.5rem;\
         font:inherit;font-size:var(--gb-size-hint);font-weight:var(--gb-weight-semibold);\
         border-radius:var(--gb-radius);"
    } else {
        action
    };

    rsx! {
        header {
            style: "flex:0 0 auto;padding:0.45rem 0.85rem;border-bottom:1px solid var(--gb-border);\
                    background:var(--gb-surface);font-size:var(--gb-size-pulse);display:flex;flex-wrap:wrap;\
                    gap:0.4rem 0.65rem;align-items:center;letter-spacing:-0.01em;",
            span {
                style: "opacity:0.7;cursor:pointer;font-weight:var(--gb-weight-regular);",
                title: "Switch repository",
                onclick: move |_| props.on_event.call(UiEvent::CloseRepository),
                "GitBolt"
            }
            span { style: "opacity:0.35;", "/" }
            button {
                style: "{branch}",
                title: "Branches",
                onclick: move |_| {
                    props
                        .on_event
                        .call(UiEvent::SelectView(segment_view(PulseSegment::Branch)));
                },
                "{pulse.branch_label}"
            }
            if ahead > 0 || behind > 0 {
                button {
                    style: "border:0;background:transparent;cursor:pointer;padding:0;display:flex;gap:0.25rem;",
                    title: "Branch health",
                    onclick: move |_| {
                        props.on_event.call(UiEvent::SelectView(segment_view(
                            PulseSegment::Divergence,
                        )));
                    },
                    if ahead > 0 {
                        span {
                            style: "padding:0.05rem 0.4rem;border-radius:var(--gb-radius-pill);\
                                    background:var(--gb-add-bg);color:var(--gb-add);font-size:var(--gb-size-label);\
                                    font-weight:var(--gb-weight-semibold);",
                            "↑{ahead}"
                        }
                    }
                    if behind > 0 {
                        span {
                            style: "padding:0.05rem 0.4rem;border-radius:var(--gb-radius-pill);\
                                    background:var(--gb-del-bg);color:var(--gb-del);font-size:var(--gb-size-label);\
                                    font-weight:var(--gb-weight-semibold);",
                            "↓{behind}"
                        }
                    }
                }
            } else if !divergence.is_empty() {
                button {
                    style: "border:0;background:var(--gb-chip);color:var(--gb-chip-text);cursor:pointer;\
                            padding:0.05rem 0.45rem;border-radius:var(--gb-radius-pill);\
                            font-size:var(--gb-size-label);font-weight:var(--gb-weight-semibold);",
                    title: "Branch health",
                    onclick: move |_| {
                        props.on_event.call(UiEvent::SelectView(segment_view(
                            PulseSegment::Divergence,
                        )));
                    },
                    "{divergence}"
                }
            }
            span {
                style: "display:flex;gap:0.55rem;align-items:center;opacity:0.85;",
                button {
                    style: "{meta}",
                    title: "Changes",
                    onclick: move |_| {
                        props
                            .on_event
                            .call(UiEvent::SelectView(segment_view(PulseSegment::Changes)));
                    },
                    "{changes_label}"
                }
                button {
                    style: "{meta}",
                    title: "Staged",
                    onclick: move |_| {
                        props
                            .on_event
                            .call(UiEvent::SelectView(segment_view(PulseSegment::Staged)));
                    },
                    "{staged_label}"
                }
                button {
                    style: "{meta}",
                    title: "Worktrees",
                    onclick: move |_| {
                        props.on_event.call(UiEvent::SelectView(segment_view(
                            PulseSegment::Worktrees,
                        )));
                    },
                    "{worktrees_label}"
                }
            }
            span {
                style: "margin-left:auto;display:flex;gap:0.35rem;align-items:center;",
                if let Some(label) = props.state.background.remote_label.clone() {
                    span { style: "opacity:0.8;font-size:var(--gb-size-hint);", "{label}" }
                } else if let Some(status) = props.state.ui.remote_status.clone() {
                    span { style: "opacity:0.85;color:var(--gb-add);font-size:var(--gb-size-hint);", "{status}" }
                } else if props.state.background.inflight > 0 {
                    span { style: "opacity:0.7;font-size:var(--gb-size-hint);", "working…" }
                }
                button {
                    style: "{action}",
                    title: "Fetch (F)",
                    onclick: move |_| props.on_event.call(UiEvent::Fetch),
                    "Fetch"
                }
                button {
                    style: "{action}",
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
                    style: "{brand}",
                    title: "Switch repository",
                    onclick: move |_| props.on_event.call(UiEvent::CloseRepository),
                    "Repos…"
                }
                span {
                    style: "opacity:0.4;font-size:var(--gb-size-hint);font-weight:var(--gb-weight-regular);\
                            margin-left:0.25rem;",
                    "{crate::platform::mod_key_label()}I · ?"
                }
            }
        }
    }
}

/// Placeholder retained for module stability; prefer [`PulseHeader`].
pub struct PulseView;
