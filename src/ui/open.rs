//! Welcome / Open Repository screen.
//!
//! Paths: dialog Open, Recent list, Drag&Drop (issue #9).

use std::path::PathBuf;

use dioxus::html::HasFileData;
use dioxus::prelude::*;

/// Props for the open screen.
#[derive(Props, Clone, PartialEq)]
pub struct OpenScreenProps {
    pub recent: Vec<PathBuf>,
    pub error: Option<String>,
    pub opening: bool,
    pub on_open: EventHandler<PathBuf>,
}

/// Renders the repository open / welcome surface.
#[component]
pub fn OpenScreen(props: OpenScreenProps) -> Element {
    let mut drop_hover = use_signal(|| false);

    rsx! {
        div {
            class: "open-screen",
            style: "display:flex;flex-direction:column;align-items:center;justify-content:center;\
                    width:100%;height:100%;gap:1.25rem;font-family:ui-sans-serif,system-ui,sans-serif;\
                    background:linear-gradient(160deg,#0f1419 0%,#1a2332 55%,#243044 100%);color:#e8eef7;",
            ondragover: move |evt| {
                evt.prevent_default();
                drop_hover.set(true);
            },
            ondragleave: move |_| drop_hover.set(false),
            ondrop: move |evt| {
                evt.prevent_default();
                drop_hover.set(false);
                if let Some(file) = evt.files().into_iter().next() {
                    let path = file.path();
                    if !path.as_os_str().is_empty() {
                        props.on_open.call(path);
                    }
                }
            },

            h1 { style: "margin:0;font-size:2rem;letter-spacing:0.04em;font-weight:600;",
                "GitBolt"
            }
            p { style: "margin:0;opacity:0.75;font-size:0.95rem;",
                "Open a Git repository to get started"
            }

            button {
                style: "padding:0.65rem 1.4rem;border:0;border-radius:6px;cursor:pointer;\
                        background:#3d8bfd;color:white;font-size:0.95rem;font-weight:600;",
                disabled: props.opening,
                onclick: move |_| {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        props.on_open.call(path);
                    }
                },
                if props.opening { "Opening…" } else { "Open Repository…" }
            }

            div {
                style: format!(
                    "min-width:min(28rem,90vw);min-height:7rem;border:2px dashed {};\
                     border-radius:8px;display:flex;align-items:center;justify-content:center;\
                     padding:1rem;opacity:0.9;transition:border-color 120ms ease;",
                    if drop_hover() { "#3d8bfd" } else { "#4a5568" }
                ),
                "Drop a repository folder here"
            }

            if !props.recent.is_empty() {
                div {
                    style: "min-width:min(28rem,90vw);",
                    h2 { style: "font-size:0.85rem;opacity:0.7;margin:0 0 0.5rem;font-weight:600;\
                                 text-transform:uppercase;letter-spacing:0.06em;",
                        "Recent"
                    }
                    ul {
                        style: "list-style:none;margin:0;padding:0;display:flex;flex-direction:column;gap:0.35rem;",
                        for path in props.recent.iter().cloned() {
                            li {
                                button {
                                    style: "width:100%;text-align:left;padding:0.5rem 0.75rem;\
                                            border:1px solid #334155;border-radius:6px;background:#111827;\
                                            color:#e8eef7;cursor:pointer;font-family:ui-monospace,monospace;\
                                            font-size:0.85rem;",
                                    disabled: props.opening,
                                    onclick: move |_| props.on_open.call(path.clone()),
                                    "{path.display()}"
                                }
                            }
                        }
                    }
                }
            }

            if let Some(err) = props.error.as_ref() {
                p {
                    style: "margin:0;color:#fca5a5;max-width:28rem;text-align:center;font-size:0.9rem;",
                    "{err}"
                }
            }
        }
    }
}
