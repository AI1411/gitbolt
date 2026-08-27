//! Single-window 3-pane shell (Navigation / Content / Context). Issue #10.

use dioxus::prelude::*;

use crate::app::event::UiEvent;
use crate::app::model::View;
use crate::app::state::AppState;
use crate::app::state::Overlay;
use crate::ui::branches::BranchesView;
use crate::ui::changes::ChangesView;
use crate::ui::context::ContextPane;
use crate::ui::diff::DiffView;
use crate::ui::error_banner::InlineErrorBanner;
use crate::ui::history::HistoryView;
use crate::ui::layout_model::clamp_context_width;
use crate::ui::layout_model::content_heading;
use crate::ui::layout_model::context_pane_width;
use crate::ui::layout_model::history_title;
use crate::ui::layout_model::CONTEXT_MAX;
use crate::ui::layout_model::CONTEXT_MIN;
use crate::ui::layout_model::NAV_MAX;
use crate::ui::layout_model::NAV_MIN;
use crate::ui::nav::NavPane;
use crate::ui::overlay::OverlayHost;
use crate::ui::pulse::PulseHeader;

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
    let prefs = crate::app::layout_prefs::load_layout_prefs();
    let mut nav_width = use_signal(|| prefs.nav_width.clamp(NAV_MIN, NAV_MAX));
    let mut context_width = use_signal(|| clamp_context_width(prefs.context_width));
    let mut drag = use_signal(|| None::<DragState>);

    let context_open = props.state.navigation.context_panel_open;
    let active = props.state.navigation.active_view;
    let heading = if active == View::History {
        history_title(&props.state.history.filter)
    } else {
        content_heading(active).to_string()
    };

    rsx! {
        div {
            class: "shell",
            style: "display:flex;flex-direction:column;width:100%;height:100%;\
                    background:var(--gb-bg);color:var(--gb-text);font-family:var(--gb-font);\
                    user-select:none;",
            tabindex: "0",
            autofocus: true,
            onkeydown: move |evt| {
                let mods = evt.data().modifiers();
                let shortcut = mods.contains(Modifiers::META) || mods.contains(Modifiers::CONTROL);
                let shift = mods.contains(Modifiers::SHIFT);
                let overlay_open = !matches!(props.state.ui.overlay, Overlay::None);
                let typing = props.state.ui.typing;

                if shortcut {
                    match evt.data().key() {
                        Key::Character(ch) if ch.eq_ignore_ascii_case("i") => {
                            evt.prevent_default();
                            props.on_event.call(UiEvent::ToggleContextPanel);
                        }
                        Key::Character(ch) if ch.eq_ignore_ascii_case("k") => {
                            evt.prevent_default();
                            props.on_event.call(UiEvent::OpenCommandPalette);
                        }
                        Key::Character(ch) if ch.eq_ignore_ascii_case("p") => {
                            evt.prevent_default();
                            props.on_event.call(UiEvent::OpenQuickOpen);
                        }
                        Key::Character(ch) if ch == "[" => {
                            evt.prevent_default();
                            props.on_event.call(UiEvent::NavigateCommit { delta: -1 });
                        }
                        Key::Character(ch) if ch == "]" => {
                            evt.prevent_default();
                            props.on_event.call(UiEvent::NavigateCommit { delta: 1 });
                        }
                        Key::Enter => {
                            evt.prevent_default();
                            props.on_event.call(UiEvent::Commit);
                        }
                        _ => {}
                    }
                    return;
                }

                if matches!(evt.data().key(), Key::Escape) {
                    evt.prevent_default();
                    props.on_event.call(UiEvent::Escape);
                    return;
                }

                if overlay_open {
                    match evt.data().key() {
                        Key::ArrowDown => {
                            evt.prevent_default();
                            props.on_event.call(UiEvent::NavigateOverlay { delta: 1 });
                        }
                        Key::ArrowUp => {
                            evt.prevent_default();
                            props.on_event.call(UiEvent::NavigateOverlay { delta: -1 });
                        }
                        Key::Enter => {
                            evt.prevent_default();
                            props.on_event.call(UiEvent::ConfirmOverlay);
                        }
                        _ => {}
                    }
                    return;
                }

                // While typing in inputs, only allow Esc (handled above).
                if typing {
                    return;
                }

                match evt.data().key() {
                    Key::Character(ch) if ch == "?" || (shift && ch == "/") => {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::OpenCheatSheet);
                    }
                    Key::Character(ch) if ch == "1" => {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::SelectView(View::Changes));
                    }
                    Key::Character(ch) if ch == "2" => {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::SelectView(View::History));
                    }
                    Key::Character(ch) if ch == "3" => {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::SelectView(View::Branches));
                    }
                    Key::Character(ch) if ch == "/" => {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::BeginListSearch);
                    }
                    Key::Character(ch) if ch.eq_ignore_ascii_case("b") => {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::SelectView(View::Branches));
                    }
                    Key::Character(ch) if ch.eq_ignore_ascii_case("h") && shift => {
                        if let Some(path) = props
                            .state
                            .diff
                            .target
                            .as_ref()
                            .map(|t| t.path.clone())
                            .or_else(|| props.state.selection.file.clone())
                        {
                            evt.prevent_default();
                            props.on_event.call(UiEvent::ShowFileHistory { path });
                        }
                    }
                    Key::Character(ch) if ch.eq_ignore_ascii_case("h") => {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::SelectView(View::History));
                    }
                    Key::Character(ch) if ch.eq_ignore_ascii_case("w") => {
                        evt.prevent_default();
                        let branch = props
                            .state
                            .selection
                            .branch
                            .clone()
                            .or_else(|| props.state.repository.head.branch.clone())
                            .filter(|name| {
                                !props
                                    .state
                                    .branch
                                    .branches
                                    .iter()
                                    .any(|b| b.name == *name && b.is_remote)
                            });
                        if let Some(branch) = branch {
                            props.on_event.call(UiEvent::InstantWorktree { branch });
                        } else {
                            props.on_event.call(UiEvent::SelectView(View::Branches));
                        }
                    }
                    Key::Character(ch) if ch.eq_ignore_ascii_case("f") => {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::Fetch);
                    }
                    Key::Character(ch)
                        if ch.eq_ignore_ascii_case("c")
                            && props.state.navigation.active_view == View::Changes =>
                    {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::FocusCommitInput);
                    }
                    Key::Character(ch)
                        if (ch == "j" || ch == "J")
                            && props.state.navigation.active_view == View::Changes =>
                    {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::NavigateChanges { delta: 1 });
                    }
                    Key::Character(ch)
                        if (ch == "k" || ch == "K")
                            && props.state.navigation.active_view == View::Changes =>
                    {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::NavigateChanges { delta: -1 });
                    }
                    Key::Character(ch)
                        if (ch == "j" || ch == "J")
                            && props.state.navigation.active_view == View::History =>
                    {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::NavigateHistory { delta: 1 });
                    }
                    Key::Character(ch)
                        if (ch == "k" || ch == "K")
                            && props.state.navigation.active_view == View::History =>
                    {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::NavigateHistory { delta: -1 });
                    }
                    Key::ArrowDown if props.state.navigation.active_view == View::Changes => {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::NavigateChanges { delta: 1 });
                    }
                    Key::ArrowUp if props.state.navigation.active_view == View::Changes => {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::NavigateChanges { delta: -1 });
                    }
                    Key::ArrowDown if props.state.navigation.active_view == View::History => {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::NavigateHistory { delta: 1 });
                    }
                    Key::ArrowUp if props.state.navigation.active_view == View::History => {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::NavigateHistory { delta: -1 });
                    }
                    Key::Character(ch)
                        if ch == "]" && props.state.navigation.active_view == View::Changes =>
                    {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::NavigateHunk { delta: 1 });
                    }
                    Key::Character(ch)
                        if ch == "[" && props.state.navigation.active_view == View::Changes =>
                    {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::NavigateHunk { delta: -1 });
                    }
                    Key::Character(ch)
                        if ch == " " && props.state.navigation.active_view == View::Changes =>
                    {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::ToggleStageSelection);
                    }
                    Key::Character(ch)
                        if (ch == "s" || ch == "S")
                            && props.state.navigation.active_view == View::Changes =>
                    {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::StageFocusedHunk);
                    }
                    _ => {}
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
                            context_width.set(clamp_context_width(state.start_width - delta));
                        }
                    }
                }
            },
            onmouseup: move |_| {
                if drag().is_some() {
                    crate::app::layout_prefs::save_layout_prefs(
                        &crate::app::layout_prefs::LayoutPrefs {
                            nav_width: nav_width(),
                            context_width: context_width(),
                            color_scheme: props.state.ui.color_scheme,
                        },
                    );
                }
                drag.set(None);
            },
            onmouseleave: move |_| {
                if drag().is_some() {
                    crate::app::layout_prefs::save_layout_prefs(
                        &crate::app::layout_prefs::LayoutPrefs {
                            nav_width: nav_width(),
                            context_width: context_width(),
                            color_scheme: props.state.ui.color_scheme,
                        },
                    );
                }
                drag.set(None);
            },

            OverlayHost {
                state: props.state.clone(),
                on_event: props.on_event,
            }

            PulseHeader {
                state: props.state.clone(),
                on_event: props.on_event,
            }

            InlineErrorBanner {
                message: props.state.ui.error_banner.clone(),
                on_event: props.on_event,
            }

            div {
                style: "flex:1 1 auto;display:flex;min-height:0;",

                div {
                    style: format!(
                        "flex:0 0 {}px;min-width:{}px;max-width:{}px;border-right:1px solid var(--gb-border);\
                         background:var(--gb-surface);",
                        nav_width(),
                        NAV_MIN,
                        NAV_MAX
                    ),
                    NavPane {
                        state: props.state.clone(),
                        on_event: props.on_event,
                    }
                }
                div {
                    class: "resize-handle",
                    style: "flex:0 0 5px;cursor:col-resize;background:transparent;",
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
                            padding:0.85rem;overflow:hidden;min-height:0;background:var(--gb-bg);",
                    h1 {
                        style: "margin:0 0 0.75rem;font-size:var(--gb-size-title);font-weight:var(--gb-weight-semibold);flex:0 0 auto;letter-spacing:-0.01em;",
                        "{heading}"
                    }
                    div {
                        style: "flex:1;min-height:0;overflow:auto;display:flex;flex-direction:column;",
                        ContentBody {
                            state: props.state.clone(),
                            on_event: props.on_event,
                        }
                    }
                }

                if context_open {
                    {
                        let showing_commit_diff = props.state.context.selected_file.is_some()
                            || !matches!(
                                props.state.context.file_diff,
                                crate::app::model::Loadable::Idle
                            );
                        let ctx_w = context_pane_width(context_width(), showing_commit_diff);
                        rsx! {
                    div {
                        class: "resize-handle",
                        style: "flex:0 0 5px;cursor:col-resize;background:transparent;",
                        onmousedown: move |evt| {
                            evt.prevent_default();
                            drag.set(Some(DragState {
                                target: DragTarget::Context,
                                start_x: evt.data().client_coordinates().x,
                                start_width: ctx_w,
                            }));
                        },
                    }
                    div {
                        style: format!(
                            "flex:0 0 {}px;min-width:{}px;max-width:{}px;background:var(--gb-surface);",
                            ctx_w,
                            CONTEXT_MIN,
                            CONTEXT_MAX
                        ),
                        ContextPane {
                            state: props.state.clone(),
                            on_event: props.on_event,
                        }
                    }
                        }
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
            div {
                style: "display:flex;flex-direction:row;align-items:stretch;gap:0;\
                        flex:1;min-height:0;height:100%;overflow:hidden;",
                div {
                    class: "changes-files-pane",
                    style: "flex:0 0 38%;min-width:12rem;max-width:24rem;overflow:auto;\
                            border-right:1px solid var(--gb-border);padding:var(--gb-space-3);\
                            box-sizing:border-box;background:var(--gb-surface);min-height:0;",
                    ChangesView {
                        state: state.clone(),
                        on_event: on_event,
                    }
                }
                div {
                    class: "changes-diff-pane",
                    style: "flex:1;min-width:0;overflow:auto;padding:var(--gb-space-3);\
                            box-sizing:border-box;background:var(--gb-bg);min-height:0;",
                    DiffView {
                        state: state,
                        on_event: on_event,
                    }
                }
            }
        },
        View::History => rsx! {
            HistoryView {
                state: state,
                on_event: on_event,
            }
        },
        View::Branches => rsx! {
            BranchesView {
                state: state,
                on_event: on_event,
            }
        },
    }
}
