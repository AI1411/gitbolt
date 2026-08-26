//! Left navigation pane (issue #10 / #88).

use dioxus::prelude::*;

use crate::app::event::UiEvent;
use crate::app::model::View;
use crate::app::state::AppState;
use crate::ui::layout_model::nav_items;

/// Props for the navigation list.
#[derive(Props, Clone, PartialEq)]
pub struct NavPaneProps {
    pub state: AppState,
    pub on_event: EventHandler<UiEvent>,
}

/// Renders Changes / History / Branches / Worktrees / Stashes with counts.
#[component]
pub fn NavPane(props: NavPaneProps) -> Element {
    let active = props.state.navigation.active_view;
    rsx! {
        nav {
            class: "nav-pane",
            style: "display:flex;flex-direction:column;gap:0.25rem;padding:0.75rem 0.5rem;\
                    height:100%;box-sizing:border-box;overflow:auto;",
            "aria-label": "Primary",
            for (view, label) in nav_items().iter().copied() {
                {
                    let selected = active == view;
                    let badge = nav_badge(&props.state, view);
                    rsx! {
                        button {
                            key: "{label}",
                            class: if selected { "nav-item active" } else { "nav-item" },
                            style: format!(
                                "text-align:left;border:0;border-radius:4px;padding:0.45rem 0.65rem;\
                                 cursor:pointer;font-size:0.9rem;background:{};color:{};font-weight:{};\
                                 display:flex;align-items:center;justify-content:space-between;gap:0.4rem;",
                                if selected { "#1e3a5f" } else { "transparent" },
                                if selected { "#e8eef7" } else { "#9fb0c7" },
                                if selected { "600" } else { "500" },
                            ),
                            onclick: move |_| props.on_event.call(UiEvent::SelectView(view)),
                            span { "{label}" }
                            if let Some(b) = badge {
                                span {
                                    style: format!(
                                        "font-size:0.72rem;font-weight:600;padding:0.05rem 0.35rem;\
                                         border-radius:999px;background:{};color:{};",
                                        b.bg, b.fg
                                    ),
                                    "{b.text}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

struct Badge {
    text: String,
    bg: &'static str,
    fg: &'static str,
}

fn nav_badge(state: &AppState, view: View) -> Option<Badge> {
    match view {
        View::Changes => {
            let n = state.changes.staged.len()
                + state.changes.unstaged.len()
                + state.changes.untracked.len()
                + state.changes.conflicted.len();
            if !state.changes.conflicted.is_empty() {
                Some(Badge {
                    text: format!("!{n}"),
                    bg: "#7f1d1d",
                    fg: "#fecaca",
                })
            } else if n > 0 {
                Some(Badge {
                    text: n.to_string(),
                    bg: "#1e3a5f",
                    fg: "#93c5fd",
                })
            } else {
                None
            }
        }
        View::Stashes => {
            let n = state.stash.entries.len();
            (n > 0).then(|| Badge {
                text: n.to_string(),
                bg: "#1e293b",
                fg: "#cbd5e1",
            })
        }
        View::Worktrees => {
            let n = state.worktree.worktrees.len();
            (n > 1).then(|| Badge {
                text: n.to_string(),
                bg: "#1e293b",
                fg: "#cbd5e1",
            })
        }
        View::Branches | View::History => None,
    }
}
