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

/// Paginated commit list with a linear graph column.
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
            style: "display:flex;flex-direction:column;gap:var(--gb-space-2);font-size:var(--gb-size-body);",
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
                        style: "font-size:var(--gb-size-hint);color:var(--gb-text-muted);",
                        "{filter_label(filter)}"
                    }
                    button {
                        style: "padding:0.2rem 0.55rem;border:1px solid var(--gb-border-strong);border-radius:var(--gb-radius);\
                                cursor:pointer;background:transparent;color:var(--gb-text-muted);font-size:var(--gb-size-hint);",
                        onclick: move |_| props.on_event.call(UiEvent::ClearHistoryFilter),
                        "All commits"
                    }
                }
            }

            if commits.is_empty() && loading {
                p { style: "margin:0;color:var(--gb-text-muted);", "Loading history…" }
            } else if commits.is_empty() {
                p { style: "margin:0;color:var(--gb-text-muted);", "No commits yet." }
            } else {
                ul {
                    style: "list-style:none;margin:0;padding:0;display:flex;flex-direction:column;",
                    for (i, c) in commits.iter().enumerate() {
                        {
                            let oid = Oid(c.oid.0.clone());
                            let is_sel = selected.as_ref() == Some(&oid);
                            let short = if c.oid.0.len() > 7 {
                                c.oid.0[..7].to_string()
                            } else {
                                c.oid.0.clone()
                            };
                            let rel = relative_time(c.timestamp, now);
                            let has_next = i + 1 < commits.len();
                            rsx! {
                                li {
                                    key: "{c.oid.0}",
                                    button {
                                        class: "gb-selectable",
                                        style: format!(
                                            "width:100%;display:grid;grid-template-columns:14px 1fr;\
                                             gap:0.65rem;text-align:left;border:0;{};\
                                             color:var(--gb-text);cursor:pointer;padding:0.4rem 0.5rem;\
                                             border-radius:var(--gb-radius);min-height:var(--gb-row);",
                                            crate::ui::theme::row_style(is_sel)
                                        ),
                                        onclick: move |_| {
                                            props.on_event.call(UiEvent::SelectCommit(oid.clone()));
                                        },
                                        CommitGraphMark { has_next: has_next, selected: is_sel }
                                        div {
                                            style: "display:flex;flex-direction:column;gap:0.12rem;min-width:0;",
                                            div {
                                                style: "display:flex;gap:0.5rem;align-items:baseline;min-width:0;",
                                                span {
                                                    style: "font-family:var(--gb-mono);font-size:var(--gb-size-hint);\
                                                            color:var(--gb-text-faint);flex:0 0 auto;",
                                                    "{short}"
                                                }
                                                span {
                                                    style: "font-weight:var(--gb-weight-semibold);font-size:var(--gb-size-body);\
                                                            overflow:hidden;text-overflow:ellipsis;white-space:nowrap;",
                                                    "{c.summary}"
                                                }
                                            }
                                            div {
                                                style: "font-size:var(--gb-size-label);color:var(--gb-text-muted);",
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

#[component]
fn CommitGraphMark(has_next: bool, selected: bool) -> Element {
    let stroke = if selected {
        "var(--gb-accent)"
    } else {
        "var(--gb-link)"
    };
    let fill = if selected {
        "var(--gb-accent)"
    } else {
        "var(--gb-link)"
    };
    let line = if has_next {
        r#"<line x1="7" y1="16" x2="7" y2="36" stroke="var(--gb-border-strong)" stroke-width="1.5"/>"#
    } else {
        ""
    };
    let svg = format!(
        r#"<svg class="gb-graph" viewBox="0 0 14 36" width="14" height="36" aria-hidden="true">
  <line x1="7" y1="0" x2="7" y2="8" stroke="var(--gb-border-strong)" stroke-width="1.5"/>
  {line}
  <circle cx="7" cy="12" r="3.4" fill="{fill}" stroke="{stroke}" stroke-width="1"/>
</svg>"#
    );
    rsx! {
        span {
            style: "display:flex;align-items:flex-start;justify-content:center;padding-top:0.05rem;",
            dangerous_inner_html: "{svg}",
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
