//! Left navigation pane (issue #10).

use dioxus::prelude::*;

use crate::app::event::UiEvent;
use crate::app::model::View;
use crate::ui::layout_model::nav_items;

/// Props for the navigation list.
#[derive(Props, Clone, PartialEq)]
pub struct NavPaneProps {
    pub active: View,
    pub on_event: EventHandler<UiEvent>,
}

/// Renders Changes / History / Branches / Worktrees / Stashes.
#[component]
pub fn NavPane(props: NavPaneProps) -> Element {
    rsx! {
        nav {
            class: "nav-pane",
            style: "display:flex;flex-direction:column;gap:0.25rem;padding:0.75rem 0.5rem;\
                    height:100%;box-sizing:border-box;overflow:auto;",
            "aria-label": "Primary",
            for (view, label) in nav_items().iter().copied() {
                {
                    let selected = props.active == view;
                    rsx! {
                        button {
                            key: "{label}",
                            class: if selected { "nav-item active" } else { "nav-item" },
                            style: format!(
                                "text-align:left;border:0;border-radius:4px;padding:0.45rem 0.65rem;\
                                 cursor:pointer;font-size:0.9rem;background:{};color:{};font-weight:{};",
                                if selected { "#1e3a5f" } else { "transparent" },
                                if selected { "#e8eef7" } else { "#9fb0c7" },
                                if selected { "600" } else { "500" },
                            ),
                            onclick: move |_| props.on_event.call(UiEvent::SelectView(view)),
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}
