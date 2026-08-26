//! Welcome / Open Repository screen.
//!
//! Paths: dialog Open, Recent list, Drag&Drop (issue #9 / #87).

use std::path::PathBuf;

use dioxus::html::HasFileData;
use dioxus::prelude::*;

use crate::app::recent::display_name;

/// Props for the open screen.
#[derive(Props, Clone, PartialEq)]
pub struct OpenScreenProps {
    pub recent: Vec<PathBuf>,
    pub error: Option<String>,
    pub opening: bool,
    pub on_open: EventHandler<PathBuf>,
    pub on_remove_recent: EventHandler<PathBuf>,
    pub on_pin_recent: EventHandler<PathBuf>,
    pub on_prune_recent: EventHandler<()>,
}

/// Renders the repository open / welcome surface.
#[component]
pub fn OpenScreen(props: OpenScreenProps) -> Element {
    let mut drop_hover = use_signal(|| false);

    rsx! {
        div {
            class: "open-screen",
            style: "display:flex;flex-direction:column;align-items:center;justify-content:center;\
                    width:100%;height:100%;gap:1.25rem;font-family:var(--gb-font);\
                    background:linear-gradient(160deg,var(--gb-bg) 0%,var(--gb-surface-mid) 55%,var(--gb-border) 100%);color:var(--gb-text);",
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
                style: "padding:0.65rem 1.4rem;border:0;border-radius:var(--gb-radius-lg);cursor:pointer;\
                        background:var(--gb-accent);color:white;font-size:0.95rem;font-weight:600;",
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
                     border-radius:var(--gb-radius-xl);display:flex;align-items:center;justify-content:center;\
                     padding:1rem;opacity:0.9;transition:border-color 120ms ease;",
                    if drop_hover() { "var(--gb-accent)" } else { "var(--gb-drop-idle)" }
                ),
                "Drop a repository folder here"
            }

            if !props.recent.is_empty() {
                div {
                    style: "min-width:min(28rem,90vw);",
                    div {
                        style: "display:flex;align-items:center;justify-content:space-between;gap:0.5rem;\
                                margin:0 0 0.5rem;",
                        h2 { style: "font-size:0.85rem;opacity:0.7;margin:0;font-weight:600;\
                                     text-transform:uppercase;letter-spacing:0.06em;",
                            "Recent"
                        }
                        button {
                            r#type: "button",
                            style: "border:0;background:transparent;color:var(--gb-text-faint);cursor:pointer;font-size:0.72rem;",
                            onclick: move |_| props.on_prune_recent.call(()),
                            "Remove missing"
                        }
                    }
                    ul {
                        style: "list-style:none;margin:0;padding:0;display:flex;flex-direction:column;gap:0.35rem;",
                        for path in props.recent.iter().cloned() {
                            {
                                let name = display_name(&path);
                                let missing = !path.exists();
                                let path_label = path.display().to_string();
                                let open_path = path.clone();
                                let pin_path = path.clone();
                                let remove_path = path.clone();
                                rsx! {
                                    li {
                                        key: "{path_label}",
                                        div {
                                            style: "display:flex;align-items:stretch;gap:0.35rem;",
                                            button {
                                                style: format!(
                                                    "flex:1;text-align:left;padding:0.45rem 0.75rem;\
                                                     border:1px solid var(--gb-border-strong);border-radius:var(--gb-radius-lg);background:var(--gb-surface);\
                                                     color:var(--gb-text);cursor:pointer;opacity:{};",
                                                    if missing { "0.55" } else { "1" }
                                                ),
                                                disabled: props.opening || missing,
                                                onclick: move |_| props.on_open.call(open_path.clone()),
                                                div {
                                                    style: "font-size:0.95rem;font-weight:600;",
                                                    "{name}"
                                                }
                                                div {
                                                    style: "font-size:0.72rem;opacity:0.55;font-family:var(--gb-mono);\
                                                            overflow:hidden;text-overflow:ellipsis;",
                                                    if missing { "Missing · {path_label}" } else { "{path_label}" }
                                                }
                                            }
                                            button {
                                                r#type: "button",
                                                title: "Pin to top",
                                                style: "border:1px solid var(--gb-border-strong);background:transparent;color:var(--gb-text-muted);\
                                                        border-radius:var(--gb-radius-lg);padding:0 0.55rem;cursor:pointer;font-size:0.75rem;",
                                                onclick: move |_| props.on_pin_recent.call(pin_path.clone()),
                                                "Pin"
                                            }
                                            button {
                                                r#type: "button",
                                                title: "Remove from Recent",
                                                style: "border:1px solid var(--gb-border-strong);background:transparent;color:var(--gb-danger);\
                                                        border-radius:var(--gb-radius-lg);padding:0 0.55rem;cursor:pointer;font-size:0.75rem;",
                                                onclick: move |_| props.on_remove_recent.call(remove_path.clone()),
                                                "×"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(err) = props.error.as_ref() {
                p {
                    style: "margin:0;color:var(--gb-danger);max-width:28rem;text-align:center;font-size:0.9rem;",
                    "{err}"
                }
            }
        }
    }
}
