//! History / commit list with lazy paging (issue #16).
//! Commit file diffs open in the Context Panel (changed-files list).

use dioxus::prelude::*;

use crate::app::event::UiEvent;
use crate::app::model::Oid;
use crate::app::state::{AppState, HistoryFilter};
use crate::ui::list_search::{matches_query, ListSearchBar};

/// Props for the history view.
#[derive(Props, Clone, PartialEq)]
pub struct HistoryViewProps {
    pub state: AppState,
    pub on_event: EventHandler<UiEvent>,
}

/// Paginated commit list with a simple linear graph column.
#[component]
pub fn HistoryView(props: HistoryViewProps) -> Element {
    let q = props.state.ui.search_query.clone();
    let commits: Vec<_> = props
        .state
        .history
        .commits
        .iter()
        .filter(|c| {
            matches_query(&c.summary, &q)
                || matches_query(&c.author, &q)
                || matches_query(&c.oid.0, &q)
        })
        .cloned()
        .collect();
    let loading = props.state.history.loading;
    let has_more = props.state.history.has_more;
    let filter = &props.state.history.filter;
    let selected = props.state.selection.commit.clone();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(0);

    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:0.5rem;font-size:0.9rem;",
            ListSearchBar {
                state: props.state.clone(),
                on_event: props.on_event,
                placeholder: "Filter commits…".to_string(),
            }
            if !matches!(filter, HistoryFilter::All) {
                div {
                    style: "display:flex;align-items:center;gap:0.5rem;flex-wrap:wrap;\
                            padding:0.35rem 0.55rem;border-radius:var(--gb-radius);background:var(--gb-chip);",
                    span {
                        style: "font-size:0.8rem;opacity:0.85;",
                        "{filter_label(filter)}"
                    }
                    button {
                        style: "padding:0.2rem 0.55rem;border:1px solid var(--gb-border-strong);border-radius:var(--gb-radius);\
                                cursor:pointer;background:transparent;color:var(--gb-text-muted);font-size:0.75rem;",
                        onclick: move |_| props.on_event.call(UiEvent::ClearHistoryFilter),
                        "All commits"
                    }
                }
            }

            if commits.is_empty() && loading {
                p { style: "margin:0;opacity:0.6;", "Loading history…" }
            } else if commits.is_empty() {
                p { style: "margin:0;opacity:0.6;", "No commits yet." }
            } else {
                ul {
                    style: "list-style:none;margin:0;padding:0;display:flex;flex-direction:column;",
                    for (i, c) in commits.iter().enumerate() {
                        {
                            let oid = Oid(c.oid.0.clone());
                            let is_sel = selected.as_ref() == Some(&oid);
                            let bg = if is_sel { "var(--gb-selected)" } else { "transparent" };
                            let short = if c.oid.0.len() > 7 {
                                c.oid.0[..7].to_string()
                            } else {
                                c.oid.0.clone()
                            };
                            let rel = relative_time(c.timestamp, now);
                            let graph = if i + 1 < commits.len() { "●\n│" } else { "●" };
                            rsx! {
                                li {
                                    key: "{c.oid.0}",
                                    button {
                                        style: format!(
                                            "width:100%;display:grid;grid-template-columns:1.2rem 1fr;\
                                             gap:0.55rem;text-align:left;border:0;background:{bg};\
                                             color:var(--gb-text);cursor:pointer;padding:0.35rem 0.45rem;\
                                             border-radius:var(--gb-radius);"
                                        ),
                                        onclick: move |_| {
                                            props.on_event.call(UiEvent::SelectCommit(oid.clone()));
                                        },
                                        pre {
                                            style: "margin:0;font-size:0.75rem;line-height:1.1;opacity:0.55;\
                                                    color:var(--gb-link);font-family:var(--gb-mono);",
                                            "{graph}"
                                        }
                                        div {
                                            style: "display:flex;flex-direction:column;gap:0.1rem;min-width:0;",
                                            div {
                                                style: "display:flex;gap:0.5rem;align-items:baseline;flex-wrap:wrap;",
                                                span {
                                                    style: "font-family:var(--gb-mono);font-size:0.75rem;opacity:0.6;",
                                                    "{short}"
                                                }
                                                span {
                                                    style: "font-weight:600;font-size:0.85rem;",
                                                    "{c.summary}"
                                                }
                                            }
                                            div {
                                                style: "font-size:0.72rem;opacity:0.55;",
                                                "{c.author} · {rel}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if has_more {
                button {
                    style: "align-self:flex-start;margin-top:0.35rem;padding:0.35rem 0.75rem;\
                            border:1px solid var(--gb-border-strong);border-radius:var(--gb-radius);cursor:pointer;\
                            background:transparent;color:var(--gb-text-muted);font-size:0.8rem;",
                    disabled: loading,
                    onclick: move |_| props.on_event.call(UiEvent::LoadMoreHistory),
                    if loading { "Loading…" } else { "Load more" }
                }
            }
        }
    }
}

fn filter_label(filter: &HistoryFilter) -> String {
    match filter {
        HistoryFilter::All => "All commits".into(),
        HistoryFilter::File { path } => format!("File history: {}", path.display()),
        HistoryFilter::Line { path, line } => {
            format!("Line {line} history: {}", path.display())
        }
    }
}

fn relative_time(ts: i64, now: i64) -> String {
    let delta = (now - ts).max(0);
    if delta < 60 {
        return format!("{delta}s ago");
    }
    if delta < 3600 {
        return format!("{}m ago", delta / 60);
    }
    if delta < 86400 {
        return format!("{}h ago", delta / 3600);
    }
    if delta < 86400 * 30 {
        return format!("{}d ago", delta / 86400);
    }
    format!("{}mo ago", delta / (86400 * 30))
}
