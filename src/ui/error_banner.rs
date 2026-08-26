//! Shared inline error + confirm UI (issue #27).

use dioxus::prelude::*;

use crate::app::event::UiEvent;

/// Props for the always-visible inline error strip.
#[derive(Props, Clone, PartialEq)]
pub struct InlineErrorProps {
    pub message: Option<String>,
    pub on_event: EventHandler<UiEvent>,
}

/// Inline error banner — not a toast; dismiss with Esc or the button.
#[component]
pub fn InlineErrorBanner(props: InlineErrorProps) -> Element {
    let Some(message) = props.message.clone() else {
        return rsx! {};
    };
    rsx! {
        div {
            role: "alert",
            style: "flex:0 0 auto;margin:0.55rem 0.85rem 0;padding:0.55rem 0.75rem;\
                    border:1px solid var(--gb-danger-border);background:var(--gb-danger-bg);border-radius:var(--gb-radius-lg);\
                    display:flex;align-items:flex-start;gap:0.65rem;color:var(--gb-danger-fg);font-size:0.85rem;",
            p {
                style: "margin:0;flex:1;line-height:1.4;white-space:pre-wrap;",
                "{message}"
            }
            button {
                style: "border:1px solid var(--gb-danger-border);background:transparent;color:var(--gb-danger);\
                        border-radius:var(--gb-radius);padding:0.15rem 0.5rem;cursor:pointer;font-size:0.72rem;",
                onclick: move |_| props.on_event.call(UiEvent::DismissError),
                "Dismiss"
            }
        }
    }
}

/// Props for a destructive-action confirmation strip.
#[derive(Props, Clone, PartialEq)]
pub struct ConfirmPanelProps {
    pub message: String,
    pub confirm_label: String,
    pub on_confirm: EventHandler<()>,
    pub on_cancel: EventHandler<()>,
    #[props(default)]
    pub secondary_confirm_label: Option<String>,
    #[props(default)]
    pub on_secondary_confirm: Option<EventHandler<()>>,
}

/// Inline confirmation for destructive ops (delete branch / remove worktree / drop stash).
#[component]
pub fn ConfirmPanel(props: ConfirmPanelProps) -> Element {
    rsx! {
        div {
            style: "padding:0.55rem 0.65rem;border:1px solid var(--gb-danger-border);background:var(--gb-danger-bg);\
                    border-radius:var(--gb-radius);display:flex;flex-direction:column;gap:0.4rem;",
            p {
                style: "margin:0;font-size:0.85rem;",
                "{props.message}"
            }
            div {
                style: "display:flex;gap:0.4rem;flex-wrap:wrap;",
                button {
                    style: "border:0;background:var(--gb-danger-strong);color:white;border-radius:var(--gb-radius);\
                            padding:0.3rem 0.65rem;cursor:pointer;font-size:0.78rem;",
                    onclick: move |_| props.on_confirm.call(()),
                    "{props.confirm_label}"
                }
                if let Some(label) = props.secondary_confirm_label.clone() {
                    if let Some(on_secondary) = props.on_secondary_confirm {
                        button {
                            style: "border:1px solid var(--gb-danger-border);background:transparent;color:var(--gb-danger);\
                                    border-radius:var(--gb-radius);padding:0.3rem 0.55rem;cursor:pointer;font-size:0.78rem;",
                            onclick: move |_| on_secondary.call(()),
                            "{label}"
                        }
                    }
                }
                button {
                    style: "border:1px solid var(--gb-border-strong);background:transparent;color:var(--gb-text-muted);\
                            border-radius:var(--gb-radius);padding:0.3rem 0.55rem;cursor:pointer;font-size:0.78rem;",
                    onclick: move |_| props.on_cancel.call(()),
                    "Cancel"
                }
            }
        }
    }
}
