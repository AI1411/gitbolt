//! Diff view with Unified/Split, hunk nav, and light syntax tint (issue #13).

use std::path::PathBuf;

use dioxus::prelude::*;

use crate::app::blame_format::{format_hover, format_minimal};
use crate::app::event::UiEvent;
use crate::app::heatmap::{color_for, intensities_for_diff};
use crate::app::model::{DiffLine, DiffView as DiffMode, Loadable, Oid};
use crate::app::state::AppState;

/// Props for the interactive diff pane.
#[derive(Props, Clone, PartialEq)]
pub struct DiffViewProps {
    pub state: AppState,
    pub on_event: EventHandler<UiEvent>,
}

/// Diff content with view modes, hunk focus, and line-stage selection.
#[component]
pub fn DiffPane(props: DiffViewProps) -> Element {
    let selected = &props.state.diff.selected_lines;
    let staged = props.state.diff.target.as_ref().is_some_and(|t| t.staged);
    let mode = props.state.diff.view;
    let focused = props.state.diff.focused_hunk;
    let heatmap_on = props.state.diff.heatmap_enabled;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX));
    let heat = props
        .state
        .diff
        .content
        .ready()
        .filter(|_| heatmap_on)
        .map(|c| intensities_for_diff(c, now))
        .unwrap_or_default();

    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:0.5rem;min-height:0;height:100%;",

            div {
                style: "display:flex;gap:0.5rem;align-items:center;flex-wrap:wrap;",
                button {
                    style: mode_style(mode == DiffMode::Unified),
                    onclick: move |_| props.on_event.call(UiEvent::SetDiffView(DiffMode::Unified)),
                    "Unified"
                }
                button {
                    style: mode_style(mode == DiffMode::Split),
                    onclick: move |_| props.on_event.call(UiEvent::SetDiffView(DiffMode::Split)),
                    "Split"
                }
                button {
                    style: mode_style(heatmap_on),
                    title: "Blame heatmap (recency + frequency)",
                    onclick: move |_| props.on_event.call(UiEvent::ToggleHeatmap),
                    "Heatmap"
                }
                if let Some(target) = props.state.diff.target.as_ref() {
                    button {
                        style: "padding:0.35rem 0.75rem;border:1px solid var(--gb-border-strong);border-radius:var(--gb-radius);\
                                cursor:pointer;background:transparent;color:var(--gb-text-muted);font-size:0.8rem;",
                        onclick: {
                            let path = target.path.clone();
                            move |_| props.on_event.call(UiEvent::ShowFileHistory { path: path.clone() })
                        },
                        "File History"
                    }
                }
                span { style: "opacity:0.45;font-size:0.75rem;", "[ / ] hunks · H file history · Shift+click blame → line history" }
                if !selected.is_empty() {
                    button {
                        style: "padding:0.35rem 0.75rem;border:0;border-radius:var(--gb-radius);cursor:pointer;\
                                background:var(--gb-accent);color:white;font-size:0.8rem;font-weight:600;",
                        onclick: move |_| {
                            if staged {
                                props.on_event.call(UiEvent::UnstageSelectedLines);
                            } else {
                                props.on_event.call(UiEvent::StageSelectedLines);
                            }
                        },
                        if staged {
                            "Unstage {selected.len()} line(s)"
                        } else {
                            "Stage {selected.len()} line(s)"
                        }
                    }
                    button {
                        style: "padding:0.35rem 0.75rem;border:1px solid var(--gb-border-strong);border-radius:var(--gb-radius);\
                                cursor:pointer;background:transparent;color:var(--gb-text-muted);font-size:0.8rem;",
                        onclick: move |_| props.on_event.call(UiEvent::ClearDiffLineSelection),
                        "Clear"
                    }
                } else {
                    span {
                        style: "opacity:0.55;font-size:0.8rem;",
                        "Click + / − lines to select · Smart Blame · H file history"
                    }
                }
            }

            match &props.state.diff.content {
                Loadable::Ready(content) => rsx! {
                    if let Some(notice) = content.notice.as_ref() {
                        div {
                            style: "padding:0.4rem 0.65rem;border-radius:var(--gb-radius);background:var(--gb-chip);\
                                    color:var(--gb-warning);font-size:0.8rem;",
                            "{notice}"
                        }
                    }
                    div {
                        style: "padding:0.5rem 0;border:1px solid var(--gb-border);border-radius:var(--gb-radius-lg);\
                                background:var(--gb-surface-raised);font-family:var(--gb-mono);font-size:0.82rem;\
                                overflow:visible;",
                        for (hi, hunk) in content.hunks.iter().enumerate() {
                            {
                                let hunk_bg = if hi == focused { "var(--gb-selected-row)" } else { "transparent" };
                                rsx! {
                                    div {
                                        key: "{hi}",
                                        style: format!("background:{hunk_bg};"),
                                        div {
                                            style: "padding:0.25rem 0.65rem;opacity:0.55;background:var(--gb-bg);\
                                                    display:flex;align-items:center;justify-content:space-between;gap:0.5rem;",
                                            span { "{hunk.header}" }
                                            if hi == focused {
                                                button {
                                                    r#type: "button",
                                                    style: "border:1px solid var(--gb-border-strong);background:var(--gb-chip);color:var(--gb-chip-text);\
                                                            border-radius:var(--gb-radius-sm);padding:0.1rem 0.4rem;cursor:pointer;font-size:0.68rem;",
                                                    title: "Stage hunk (s)",
                                                    onclick: move |_| {
                                                        props.on_event.call(UiEvent::StageFocusedHunk);
                                                    },
                                                    "Stage hunk"
                                                }
                                            }
                                        }
                                        if mode == DiffMode::Split {
                                            SplitHunk {
                                                lines: hunk.lines.clone(),
                                                selected: selected.clone(),
                                                file_path: content.target.path.clone(),
                                                heat: heat.clone(),
                                                on_event: props.on_event,
                                            }
                                        } else {
                                            for line in hunk.lines.iter() {
                                                UnifiedLine {
                                                    line: line.clone(),
                                                    file_path: content.target.path.clone(),
                                                    selected: selected.contains(&line.body_index),
                                                    heat_color: heat.get(&line.body_index).copied().map(color_for),
                                                    on_event: props.on_event,
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                Loadable::Loading => rsx! {
                    div { style: "opacity:0.6;font-size:0.85rem;", "Loading diff…" }
                },
                Loadable::Failed(err) => rsx! {
                    div { style: "color:var(--gb-danger);font-size:0.85rem;", "Diff error: {err}" }
                },
                Loadable::Idle => rsx! {
                    div {
                        style: "opacity:0.7;font-size:0.85rem;",
                        if let Some(path) = props.state.selection.file.as_ref() {
                            "Selected: {path.display()}"
                        } else {
                            "Select a file to view its diff."
                        }
                    }
                },
            }
        }
    }
}

/// Back-compat name used by the shell.
#[component]
pub fn DiffView(props: DiffViewProps) -> Element {
    rsx! { DiffPane { state: props.state, on_event: props.on_event } }
}

#[component]
fn UnifiedLine(
    line: DiffLine,
    file_path: PathBuf,
    selected: bool,
    heat_color: Option<&'static str>,
    on_event: EventHandler<UiEvent>,
) -> Element {
    let idx = line.body_index;
    let stageable = line.origin == '+' || line.origin == '-';
    let bg = if selected {
        "var(--gb-selected)"
    } else if line.origin == '+' {
        "var(--gb-add-bg)"
    } else if line.origin == '-' {
        "var(--gb-del-bg)"
    } else {
        "transparent"
    };
    let color = match line.origin {
        '+' => "var(--gb-add)",
        '-' => "var(--gb-danger)",
        _ => "var(--gb-chip-text)",
    };
    let tinted = tint_line(&line.content);
    let origin = line.change_origin.clone();
    let gutter = heat_color.unwrap_or("transparent");

    rsx! {
        div {
            style: format!(
                "display:flex;align-items:stretch;gap:0.5rem;padding:0.05rem 0.65rem 0.05rem 0;\
                 white-space:pre;cursor:{};background:{};color:{};",
                if stageable { "pointer" } else { "default" },
                bg,
                color,
            ),
            onclick: move |_| {
                if stageable {
                    on_event.call(UiEvent::ToggleDiffLine(idx));
                }
            },
            span {
                style: format!(
                    "flex:0 0 4px;align-self:stretch;background:{gutter};border-radius:1px;"
                ),
                title: if heat_color.is_some() {
                    "Blame heatmap"
                } else {
                    ""
                },
            }
            span { style: "flex:0 0 1ch;opacity:0.7;align-self:baseline;", "{line.origin}" }
            span {
                style: "flex:1;min-width:0;align-self:baseline;",
                dangerous_inner_html: "{tinted}",
            }
            if let Some(origin) = origin {
                {
                    let oid = Oid(origin.oid.0.clone());
                    let blame_path = file_path.clone();
                    let blame_line = line.old_line;
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX));
                    let label = format_minimal(&origin, now);
                    let hover = format_hover(&origin, now);
                    rsx! {
                        button {
                            style: "flex:0 0 auto;max-width:12rem;overflow:hidden;text-overflow:ellipsis;\
                                    border:0;background:transparent;color:var(--gb-link);cursor:pointer;\
                                    font-size:0.7rem;font-family:var(--gb-mono);padding:0;\
                                    align-self:baseline;",
                            title: "{hover} · Shift+click for line history",
                            onclick: move |evt| {
                                evt.stop_propagation();
                                let mods = evt.data().modifiers();
                                if mods.contains(Modifiers::SHIFT) {
                                    if let Some(line_no) = blame_line {
                                        on_event.call(UiEvent::ShowLineHistory {
                                            path: blame_path.clone(),
                                            line: line_no,
                                        });
                                    }
                                } else {
                                    on_event.call(UiEvent::SelectCommit(oid.clone()));
                                }
                            },
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SplitHunk(
    lines: Vec<DiffLine>,
    file_path: PathBuf,
    selected: Vec<usize>,
    heat: std::collections::HashMap<usize, f32>,
    on_event: EventHandler<UiEvent>,
) -> Element {
    rsx! {
        div {
            style: "display:grid;grid-template-columns:1fr 1fr;gap:0;border-top:1px solid var(--gb-chip);",
            div {
                style: "border-right:1px solid var(--gb-chip);",
                for line in lines.iter().filter(|l| l.origin != '+') {
                    UnifiedLine {
                        line: line.clone(),
                        file_path: file_path.clone(),
                        selected: selected.contains(&line.body_index),
                        heat_color: heat.get(&line.body_index).copied().map(color_for),
                        on_event: on_event,
                    }
                }
            }
            div {
                for line in lines.iter().filter(|l| l.origin != '-') {
                    UnifiedLine {
                        line: line.clone(),
                        file_path: file_path.clone(),
                        selected: selected.contains(&line.body_index),
                        heat_color: heat.get(&line.body_index).copied().map(color_for),
                        on_event: on_event,
                    }
                }
            }
        }
    }
}

fn mode_style(active: bool) -> String {
    if active {
        "padding:0.25rem 0.55rem;border:0;border-radius:var(--gb-radius);cursor:pointer;\
         background:var(--gb-accent);color:white;font-size:0.75rem;font-weight:600;"
            .into()
    } else {
        "padding:0.25rem 0.55rem;border:1px solid var(--gb-border-strong);border-radius:var(--gb-radius);cursor:pointer;\
         background:transparent;color:var(--gb-text-muted);font-size:0.75rem;"
            .into()
    }
}

/// Very light syntax tint: comments / strings get muted spans (HTML-escaped).
pub(crate) fn tint_line(content: &str) -> String {
    let esc = html_escape(content);
    let trimmed = content.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*") {
        return format!("<span style=\"opacity:0.55\">{esc}</span>");
    }
    if let Some(rest) = esc.strip_prefix("&quot;") {
        return format!("<span style=\"color:var(--gb-string)\">&quot;{rest}</span>");
    }
    if let Some(rest) = esc.strip_prefix('\'') {
        return format!("<span style=\"color:var(--gb-string)\">'{rest}</span>");
    }
    esc
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tint_escapes_html() {
        assert!(tint_line("<script>").contains("&lt;script&gt;"));
    }
}
