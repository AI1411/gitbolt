//! Command Palette and Quick Open overlays (issue #26).

use dioxus::prelude::*;

use crate::app::event::UiEvent;
use crate::app::palette::filter_commands;
use crate::app::quick_open::{collect_items, filter_items, QuickOpenKind};
use crate::app::state::{AppState, Overlay};

/// Props for the overlay host.
#[derive(Props, Clone, PartialEq)]
pub struct OverlayHostProps {
    pub state: AppState,
    pub on_event: EventHandler<UiEvent>,
}

/// Renders Command Palette, Quick Open, or Cheat Sheet when active.
#[component]
pub fn OverlayHost(props: OverlayHostProps) -> Element {
    let modi = crate::platform::mod_key_label();
    match &props.state.ui.overlay {
        Overlay::None => rsx! {},
        Overlay::CheatSheet => {
            rsx! {
                div {
                    style: "position:fixed;inset:0;z-index:50;display:flex;align-items:flex-start;\
                            justify-content:center;padding-top:10vh;background:rgba(0,0,0,0.45);",
                    onclick: move |_| props.on_event.call(UiEvent::CloseOverlay),
                    div {
                        style: "width:min(40rem,92vw);background:#121820;border:1px solid #334155;\
                                border-radius:8px;box-shadow:0 12px 40px rgba(0,0,0,0.45);\
                                padding:1rem 1.1rem;max-height:70vh;overflow:auto;",
                        onclick: move |evt| evt.stop_propagation(),
                        h2 {
                            style: "margin:0 0 0.75rem;font-size:1rem;",
                            "Keyboard shortcuts"
                        }
                        CheatSheetBody { modi: modi.to_string() }
                        OverlayFooter { hint: format!("? · Esc · {modi}K palette · {modi}P quick open") }
                    }
                }
            }
        }
        Overlay::CommandPalette { query, selected } => {
            let items = filter_commands(query);
            let selected = *selected;
            rsx! {
                div {
                    style: "position:fixed;inset:0;z-index:50;display:flex;align-items:flex-start;\
                            justify-content:center;padding-top:12vh;background:rgba(0,0,0,0.45);",
                    onclick: move |_| props.on_event.call(UiEvent::CloseOverlay),
                    div {
                        style: "width:min(36rem,92vw);background:#121820;border:1px solid #334155;\
                                border-radius:8px;box-shadow:0 12px 40px rgba(0,0,0,0.45);\
                                display:flex;flex-direction:column;max-height:60vh;",
                        onclick: move |evt| evt.stop_propagation(),
                        OverlayHeader {
                            title: String::from("Command Palette"),
                            query: query.clone(),
                            on_event: props.on_event,
                        }
                        div {
                            style: "overflow:auto;flex:1;padding:0.25rem 0;",
                            for (i, cmd) in items.into_iter().enumerate() {
                                {
                                    let bg = if i == selected { "#1e3a5f" } else { "transparent" };
                                    let label = cmd.label;
                                    let keys = cmd.keys;
                                    rsx! {
                                        button {
                                            key: "{cmd.id}",
                                            style: format!(
                                                "width:100%;display:flex;justify-content:space-between;gap:0.75rem;\
                                                 text-align:left;border:0;background:{bg};color:#e8eef7;cursor:pointer;\
                                                 padding:0.4rem 0.85rem;font-size:0.85rem;"
                                            ),
                                            onclick: move |_| {
                                                props.on_event.call(UiEvent::SelectOverlayItem(i));
                                            },
                                            span { "{label}" }
                                            if !keys.is_empty() {
                                                span {
                                                    style: "font-size:0.72rem;opacity:0.5;font-family:ui-monospace,monospace;",
                                                    "{keys}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        OverlayFooter { hint: format!("{modi}K · ↑↓ · Enter · Esc") }
                    }
                }
            }
        }
        Overlay::QuickOpen { query, selected } => {
            let all = collect_items(&props.state);
            let items = filter_items(&all, query);
            let selected = *selected;
            rsx! {
                div {
                    style: "position:fixed;inset:0;z-index:50;display:flex;align-items:flex-start;\
                            justify-content:center;padding-top:12vh;background:rgba(0,0,0,0.45);",
                    onclick: move |_| props.on_event.call(UiEvent::CloseOverlay),
                    div {
                        style: "width:min(36rem,92vw);background:#121820;border:1px solid #334155;\
                                border-radius:8px;box-shadow:0 12px 40px rgba(0,0,0,0.45);\
                                display:flex;flex-direction:column;max-height:60vh;",
                        onclick: move |evt| evt.stop_propagation(),
                        OverlayHeader {
                            title: String::from("Quick Open"),
                            query: query.clone(),
                            on_event: props.on_event,
                        }
                        div {
                            style: "overflow:auto;flex:1;padding:0.25rem 0;",
                            for (i, item) in items.into_iter().enumerate() {
                                {
                                    let bg = if i == selected { "#1e3a5f" } else { "transparent" };
                                    let kind = match item.kind {
                                        QuickOpenKind::File => "file",
                                        QuickOpenKind::Branch => "branch",
                                        QuickOpenKind::Commit => "commit",
                                    };
                                    let label = item.label.clone();
                                    let detail = item.detail.clone();
                                    rsx! {
                                        button {
                                            key: "{kind}-{label}",
                                            style: format!(
                                                "width:100%;display:flex;align-items:baseline;gap:0.65rem;\
                                                 text-align:left;border:0;background:{bg};color:#e8eef7;cursor:pointer;\
                                                 padding:0.4rem 0.85rem;font-size:0.85rem;"
                                            ),
                                            onclick: move |_| {
                                                props.on_event.call(UiEvent::SelectOverlayItem(i));
                                            },
                                            span {
                                                style: "font-size:0.68rem;opacity:0.5;text-transform:uppercase;min-width:3.5rem;",
                                                "{kind}"
                                            }
                                            span {
                                                style: "flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;",
                                                "{label}"
                                            }
                                            span { style: "font-size:0.72rem;opacity:0.45;", "{detail}" }
                                        }
                                    }
                                }
                            }
                        }
                        OverlayFooter {
                            hint: format!("{modi}P · files / branches / commits · Esc"),
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn OverlayHeader(title: String, query: String, on_event: EventHandler<UiEvent>) -> Element {
    rsx! {
        div {
            style: "padding:0.65rem 0.85rem;border-bottom:1px solid #243044;",
            div {
                style: "font-size:0.72rem;letter-spacing:0.06em;text-transform:uppercase;\
                        opacity:0.55;margin-bottom:0.35rem;",
                "{title}"
            }
            input {
                autofocus: true,
                style: "width:100%;box-sizing:border-box;padding:0.45rem 0.55rem;\
                        border-radius:4px;border:1px solid #334155;background:#0f1419;\
                        color:#e8eef7;font-size:0.9rem;font-family:inherit;",
                placeholder: "Type to filter…",
                value: "{query}",
                onfocus: move |_| on_event.call(UiEvent::SetTyping(true)),
                onblur: move |_| on_event.call(UiEvent::SetTyping(false)),
                oninput: move |evt| {
                    on_event.call(UiEvent::SetOverlayQuery(evt.value()));
                },
                onkeydown: move |evt| {
                    match evt.data().key() {
                        Key::Escape => {
                            evt.prevent_default();
                            on_event.call(UiEvent::CloseOverlay);
                        }
                        Key::ArrowDown => {
                            evt.prevent_default();
                            on_event.call(UiEvent::NavigateOverlay { delta: 1 });
                        }
                        Key::ArrowUp => {
                            evt.prevent_default();
                            on_event.call(UiEvent::NavigateOverlay { delta: -1 });
                        }
                        Key::Enter => {
                            evt.prevent_default();
                            on_event.call(UiEvent::ConfirmOverlay);
                        }
                        _ => {
                            evt.stop_propagation();
                        }
                    }
                },
            }
        }
    }
}

#[component]
fn OverlayFooter(hint: String) -> Element {
    rsx! {
        div {
            style: "padding:0.4rem 0.85rem;border-top:1px solid #243044;\
                    font-size:0.72rem;opacity:0.5;",
            "{hint}"
        }
    }
}

#[component]
fn CheatSheetBody(modi: String) -> Element {
    let rows: Vec<(String, String)> = vec![
        (
            "1 / 2 / 3 / 4 / 5".into(),
            "Changes / History / Branches / Worktrees / Stashes".into(),
        ),
        ("B / H / W".into(), "Branches / History / Worktrees".into()),
        (
            "j k · Space · [ ] · s".into(),
            "Changes: move · stage · hunk · stage hunk".into(),
        ),
        ("C".into(), "Focus commit message (Changes)".into()),
        ("F".into(), "Fetch".into()),
        ("Shift+H".into(), "File history for selected file".into()),
        (format!("{modi}K"), "Command Palette".into()),
        (format!("{modi}P"), "Quick Open".into()),
        ("/".into(), "In-view list search".into()),
        (format!("{modi}I"), "Toggle Context".into()),
        (format!("{modi}Enter"), "Commit".into()),
        ("?".into(), "This cheat sheet".into()),
        ("Esc".into(), "Close overlay / cancel".into()),
    ];
    rsx! {
        table {
            style: "width:100%;border-collapse:collapse;font-size:0.85rem;",
            for (keys, desc) in rows {
                tr {
                    td {
                        style: "padding:0.28rem 0.5rem 0.28rem 0;font-family:ui-monospace,monospace;\
                                opacity:0.85;white-space:nowrap;vertical-align:top;",
                        "{keys}"
                    }
                    td {
                        style: "padding:0.28rem 0;opacity:0.75;",
                        "{desc}"
                    }
                }
            }
        }
    }
}
