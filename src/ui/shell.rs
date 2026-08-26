//! Single-window 3-pane shell (Navigation / Content / Context). Issue #10.

use dioxus::prelude::*;

use crate::app::event::UiEvent;
use crate::app::model::View;
use crate::app::state::AppState;
use crate::ui::branches::BranchesView;
use crate::ui::changes::ChangesView;
use crate::ui::context::ContextPane;
use crate::ui::diff::DiffView;
use crate::ui::history::HistoryView;
use crate::ui::layout_model::content_heading;
use crate::ui::nav::NavPane;
use crate::ui::worktrees::WorktreesView;

const NAV_MIN: f64 = 140.0;
const NAV_MAX: f64 = 360.0;
const CONTEXT_MIN: f64 = 180.0;
const CONTEXT_MAX: f64 = 480.0;

/// Props for the ready-state shell.
#[derive(Props, Clone, PartialEq)]
pub struct ShellProps {
    pub state: AppState,
    pub on_event: EventHandler<UiEvent>,
}

#[derive(Clone, Copy, PartialEq)]
struct DragState {
    target: DragTarget,
    start_x: f64,
    start_width: f64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DragTarget {
    Nav,
    Context,
}

/// Resizable Navigation / Content / Context layout.
#[component]
pub fn Shell(props: ShellProps) -> Element {
    let mut nav_width = use_signal(|| 200.0_f64);
    let mut context_width = use_signal(|| 280.0_f64);
    let mut drag = use_signal(|| None::<DragState>);

    let branch = props
        .state
        .repository
        .head
        .branch
        .clone()
        .unwrap_or_else(|| {
            if props.state.repository.head.detached {
                "detached HEAD".into()
            } else {
                "(unknown)".into()
            }
        });
    let context_open = props.state.navigation.context_panel_open;
    let active = props.state.navigation.active_view;
    let heading = content_heading(active);

    rsx! {
        div {
            class: "shell",
            style: "display:flex;flex-direction:column;width:100%;height:100%;\
                    background:#0f1419;color:#e8eef7;font-family:ui-sans-serif,system-ui,sans-serif;\
                    user-select:none;",
            tabindex: "0",
            autofocus: true,
            onkeydown: move |evt| {
                let mods = evt.data().modifiers();
                let shortcut = mods.contains(Modifiers::META) || mods.contains(Modifiers::CONTROL);
                if shortcut {
                    if let Key::Character(ch) = evt.data().key() {
                        if ch.eq_ignore_ascii_case("i") {
                            evt.prevent_default();
                            props.on_event.call(UiEvent::ToggleContextPanel);
                        }
                    }
                }
            },
            onmousemove: move |evt| {
                if let Some(state) = drag() {
                    let x = evt.data().client_coordinates().x;
                    let delta = x - state.start_x;
                    match state.target {
                        DragTarget::Nav => {
                            nav_width.set((state.start_width + delta).clamp(NAV_MIN, NAV_MAX));
                        }
                        DragTarget::Context => {
                            context_width
                                .set((state.start_width - delta).clamp(CONTEXT_MIN, CONTEXT_MAX));
                        }
                    }
                }
            },
            onmouseup: move |_| drag.set(None),
            onmouseleave: move |_| drag.set(None),

            header {
                style: "flex:0 0 auto;padding:0.55rem 0.85rem;border-bottom:1px solid #243044;\
                        font-weight:600;font-size:0.95rem;display:flex;gap:0.75rem;align-items:center;",
                span { "GitBolt / {branch}" }
                span {
                    style: "opacity:0.45;font-weight:500;font-size:0.8rem;margin-left:auto;",
                    "⌘I context"
                }
            }

            div {
                style: "flex:1 1 auto;display:flex;min-height:0;",

                div {
                    style: format!(
                        "flex:0 0 {}px;min-width:{}px;max-width:{}px;border-right:1px solid #243044;\
                         background:#121820;",
                        nav_width(),
                        NAV_MIN,
                        NAV_MAX
                    ),
                    NavPane {
                        active: active,
                        on_event: props.on_event,
                    }
                }
                div {
                    class: "resize-handle",
                    style: "flex:0 0 4px;cursor:col-resize;background:transparent;",
                    onmousedown: move |evt| {
                        evt.prevent_default();
                        drag.set(Some(DragState {
                            target: DragTarget::Nav,
                            start_x: evt.data().client_coordinates().x,
                            start_width: nav_width(),
                        }));
                    },
                }

                main {
                    style: "flex:1 1 auto;min-width:0;display:flex;flex-direction:column;\
                            padding:0.85rem;overflow:auto;",
                    h1 {
                        style: "margin:0 0 0.75rem;font-size:1.05rem;font-weight:600;",
                        "{heading}"
                    }
                    ContentBody {
                        state: props.state.clone(),
                        on_event: props.on_event,
                    }
                }

                if context_open {
                    div {
                        class: "resize-handle",
                        style: "flex:0 0 4px;cursor:col-resize;background:transparent;",
                        onmousedown: move |evt| {
                            evt.prevent_default();
                            drag.set(Some(DragState {
                                target: DragTarget::Context,
                                start_x: evt.data().client_coordinates().x,
                                start_width: context_width(),
                            }));
                        },
                    }
                    div {
                        style: format!(
                            "flex:0 0 {}px;min-width:{}px;max-width:{}px;background:#121820;",
                            context_width(),
                            CONTEXT_MIN,
                            CONTEXT_MAX
                        ),
                        ContextPane { state: props.state.clone() }
                    }
                }
            }
        }
    }
}

#[component]
fn ContentBody(state: AppState, on_event: EventHandler<UiEvent>) -> Element {
    match state.navigation.active_view {
        View::Changes => rsx! {
            DiffView {
                state: state.clone(),
                on_event: on_event,
            }
            ChangesView {
                state: state,
                on_event: on_event,
            }
        },
        View::History => rsx! { HistoryView { state: state } },
        View::Branches => rsx! {
            BranchesView {
                state: state,
                on_event: on_event,
            }
        },
        View::Worktrees => rsx! { WorktreesView { state: state } },
        View::Stashes => rsx! {
            div {
                style: "opacity:0.7;font-size:0.9rem;",
                "Stashes — list/apply arrives with the Stash MVP issue."
            }
        },
    }
}
