//! Branch list with create / checkout / delete, Recent, and Quick Open (issues #17 / #30).

use dioxus::prelude::*;

use crate::app::branch_health::format_badge;
use crate::app::event::UiEvent;
use crate::app::model::BranchInfo;
use crate::app::state::AppState;
use crate::ui::divergence::DivergenceView;
use crate::ui::error_banner::ConfirmPanel;
use crate::ui::list_search::ListSearchBar;

/// Props for the branches pane.
#[derive(Props, Clone, PartialEq)]
pub struct BranchesViewProps {
    pub state: AppState,
    pub on_event: EventHandler<UiEvent>,
}

/// Branches list + optional Divergence panel.
#[component]
pub fn BranchesView(props: BranchesViewProps) -> Element {
    let mut upstream_draft = use_signal(String::new);
    let mut upstream_target = use_signal(|| None::<String>);
    let mut create_draft = use_signal(String::new);
    // Local filter avoids controlled-input races with AppState round-trips
    // (Quick Open must keep matching rows visible while typing).
    let mut filter_text = use_signal(|| props.state.branch.filter.clone());

    let current = props
        .state
        .branch
        .current
        .clone()
        .or_else(|| props.state.repository.head.branch.clone())
        .unwrap_or_else(|| "(none)".into());
    let show_div = props.state.divergence.left.is_some() || props.state.divergence.loading;
    let needle = if props.state.ui.searching && !props.state.ui.search_query.is_empty() {
        props.state.ui.search_query.clone()
    } else {
        filter_text()
    };
    let filtered: Vec<BranchInfo> = filter_branches(&props.state.branch.branches, &needle)
        .into_iter()
        .cloned()
        .collect();
    let loaded = props.state.branch.loaded;
    let total = props.state.branch.branches.len();
    let pending_delete = props.state.ui.confirm_delete_branch.clone();
    let cleanup = props.state.ui.branch_cleanup.clone();
    let current_opt = props
        .state
        .branch
        .current
        .clone()
        .or_else(|| props.state.repository.head.branch.clone());
    let cleanup_candidates = crate::app::branch_cleanup::candidates(
        &props.state.branch.branches,
        current_opt.as_deref(),
        &props.state.branch.merged_into_base,
        &props.state.branch.squashed_into_base,
        &props.state.branch.cleanup_excluded,
    );

    rsx! {
        div {
            style: "font-size:0.9rem;opacity:0.95;display:flex;flex-direction:column;gap:0.75rem;",
            p { style: "margin:0;opacity:0.75;", "Current: {current}" }

            ListSearchBar {
                state: props.state.clone(),
                on_event: props.on_event,
                placeholder: "Filter branches…".to_string(),
            }

            div {
                style: "display:flex;gap:0.4rem;flex-wrap:wrap;align-items:center;",
                button {
                    r#type: "button",
                    style: "border:1px solid var(--gb-border-strong);background:var(--gb-chip);color:var(--gb-chip-strong);border-radius:var(--gb-radius);\
                            padding:0.3rem 0.65rem;cursor:pointer;font-size:0.78rem;",
                    disabled: cleanup_candidates.is_empty(),
                    onclick: move |_| props.on_event.call(UiEvent::OpenBranchCleanup),
                    "Cleanup outdated…"
                }
                span {
                    style: "font-size:0.75rem;opacity:0.55;",
                    "{cleanup_candidates.len()} candidate(s)"
                }
            }

            if let Some(cleanup_state) = cleanup.clone() {
                CleanupPanel {
                    candidates: cleanup_candidates.clone(),
                    selected: cleanup_state.selected.clone(),
                    on_event: props.on_event,
                }
            }

            // Create branch
            div {
                style: "display:flex;gap:0.4rem;align-items:center;flex-wrap:wrap;",
                input {
                    style: "flex:1;min-width:8rem;padding:0.35rem 0.5rem;border-radius:var(--gb-radius);\
                            border:1px solid var(--gb-border-strong);background:var(--gb-bg);color:var(--gb-text);font-size:0.85rem;",
                    placeholder: "New branch name",
                    value: "{create_draft()}",
                    oninput: move |evt| create_draft.set(evt.value()),
                    onkeydown: move |evt| {
                        if evt.data().key() == Key::Enter {
                            let name = create_draft().trim().to_string();
                            if !name.is_empty() {
                                props.on_event.call(UiEvent::CreateBranch(name));
                                create_draft.set(String::new());
                            }
                        }
                    },
                }
                button {
                    style: "border:0;background:var(--gb-accent);color:white;border-radius:var(--gb-radius);\
                            padding:0.35rem 0.7rem;cursor:pointer;font-size:0.8rem;",
                    onclick: move |_| {
                        let name = create_draft().trim().to_string();
                        if !name.is_empty() {
                            props.on_event.call(UiEvent::CreateBranch(name));
                            create_draft.set(String::new());
                        }
                    },
                    "Create"
                }
            }

            // Quick Open filter for branches
            input {
                style: "width:100%;box-sizing:border-box;padding:0.4rem 0.55rem;border-radius:var(--gb-radius);\
                        border:1px solid var(--gb-border-strong);background:var(--gb-bg);color:var(--gb-text);font-size:0.85rem;",
                placeholder: "Quick Open branches (/)",
                value: "{filter_text()}",
                oninput: move |evt| {
                    let value = evt.value();
                    filter_text.set(value.clone());
                    props.on_event.call(UiEvent::SetBranchFilter(value));
                },
            }

            if !props.state.branch.recent.is_empty() {
                div {
                    h3 {
                        style: "margin:0 0 0.35rem;font-size:0.75rem;letter-spacing:0.06em;\
                                text-transform:uppercase;opacity:0.6;",
                        "Recent"
                    }
                    ul {
                        style: "list-style:none;margin:0;padding:0;display:flex;flex-wrap:wrap;gap:0.35rem;",
                        for name in props.state.branch.recent.iter().cloned() {
                            {
                                let n = name.clone();
                                let checkout = name.clone();
                                rsx! {
                                    li {
                                        button {
                                            style: "border:1px solid var(--gb-border-strong);background:var(--gb-surface-raised);color:var(--gb-chip-text);\
                                                    border-radius:var(--gb-radius);padding:0.2rem 0.5rem;cursor:pointer;\
                                                    font-family:var(--gb-mono);font-size:0.75rem;",
                                            title: "Checkout",
                                            onclick: move |_| {
                                                props.on_event.call(UiEvent::CheckoutBranch(checkout.clone()));
                                            },
                                            oncontextmenu: move |evt| {
                                                evt.prevent_default();
                                                props.on_event.call(UiEvent::SelectBranch(n.clone()));
                                            },
                                            "{name}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(name) = pending_delete.clone() {
                ConfirmPanel {
                    message: format!(
                        "Delete local branch {name}? Unmerged branches will be refused."
                    ),
                    confirm_label: String::from("Delete"),
                    on_confirm: move |()| props.on_event.call(UiEvent::ConfirmDeleteBranch),
                    on_cancel: move |()| props.on_event.call(UiEvent::CancelDeleteBranch),
                }
            }

            if !loaded {
                p { style: "margin:0;opacity:0.6;", "Loading branches…" }
            } else if filtered.is_empty() {
                p {
                    style: "margin:0;opacity:0.6;",
                    if total == 0 {
                        "No branches"
                    } else {
                        "No branches match the filter"
                    }
                }
            } else {
                ul {
                    style: "list-style:none;margin:0;padding:0;display:flex;flex-direction:column;gap:0.35rem;",
                    for b in filtered.iter().cloned() {
                        {
                            let key = b.name.clone();
                            rsx! {
                                BranchRow {
                                    key: "{key}",
                                    branch: b,
                                    current: current.clone(),
                                    on_event: props.on_event,
                                    on_track: move |name| {
                                        upstream_target.set(Some(name));
                                        upstream_draft.set(String::new());
                                    },
                                }
                            }
                        }
                    }
                }
            }

            if let Some(branch) = upstream_target() {
                div {
                    style: "display:flex;gap:0.4rem;align-items:center;flex-wrap:wrap;",
                    span { style: "font-size:0.8rem;opacity:0.7;", "Track for {branch}:" }
                    input {
                        style: "flex:1;min-width:8rem;padding:0.3rem 0.45rem;border-radius:var(--gb-radius);\
                                border:1px solid var(--gb-border-strong);background:var(--gb-bg);color:var(--gb-text);font-size:0.8rem;",
                        placeholder: "origin/main",
                        value: "{upstream_draft()}",
                        oninput: move |evt| upstream_draft.set(evt.value()),
                    }
                    button {
                        style: "border:0;background:var(--gb-accent);color:white;border-radius:var(--gb-radius);\
                                padding:0.3rem 0.65rem;cursor:pointer;font-size:0.78rem;",
                        onclick: move |_| {
                            let upstream = upstream_draft().trim().to_string();
                            if !upstream.is_empty() {
                                props.on_event.call(UiEvent::SetUpstream {
                                    branch: branch.clone(),
                                    upstream,
                                });
                                upstream_target.set(None);
                            }
                        },
                        "Set upstream"
                    }
                    button {
                        style: "border:1px solid var(--gb-border-strong);background:transparent;color:var(--gb-text-muted);\
                                border-radius:var(--gb-radius);padding:0.3rem 0.55rem;cursor:pointer;font-size:0.78rem;",
                        onclick: move |_| upstream_target.set(None),
                        "Cancel"
                    }
                }
            }

            if show_div {
                DivergenceView {
                    state: props.state.divergence.clone(),
                    on_event: props.on_event,
                }
            }
        }
    }
}

#[component]
fn BranchRow(
    branch: BranchInfo,
    current: String,
    on_event: EventHandler<UiEvent>,
    on_track: EventHandler<String>,
) -> Element {
    let badge = format_badge(
        branch.health,
        branch.ahead,
        branch.behind,
        branch.stale_days,
    );
    let name = branch.name.clone();
    let checkout_name = branch.name.clone();
    let other = branch.name.clone();
    let track_name = branch.name.clone();
    let delete_name = branch.name.clone();
    let is_remote = branch.is_remote;
    let is_current = !is_remote && branch.name == current;
    let last = branch.last_commit.as_ref().map_or_else(String::new, |c| {
        let short = if c.oid.0.len() > 7 {
            c.oid.0[..7].to_string()
        } else {
            c.oid.0.clone()
        };
        format!("{short} · {}", c.summary)
    });
    let upstream = if is_remote {
        "remote".into()
    } else {
        branch
            .upstream
            .clone()
            .unwrap_or_else(|| "no upstream".into())
    };

    rsx! {
        li {
            style: "display:flex;flex-direction:column;gap:0.15rem;padding:0.3rem 0.35rem;\
                    border-radius:var(--gb-radius);border:1px solid var(--gb-chip);",
            div {
                style: "display:flex;align-items:center;gap:0.5rem;flex-wrap:wrap;",
                button {
                    style: "flex:1;min-width:6rem;text-align:left;border:0;background:transparent;color:var(--gb-text);\
                            cursor:pointer;font-family:var(--gb-mono);font-size:0.85rem;",
                    onclick: move |_| on_event.call(UiEvent::SelectBranch(name.clone())),
                    if is_remote {
                        span { style: "opacity:0.55;margin-right:0.25rem;", "remote" }
                    }
                    "{branch.name}"
                    if is_current {
                        span { style: "opacity:0.55;margin-left:0.35rem;", "(current)" }
                    }
                }
                if !is_remote {
                    span { style: "opacity:0.7;font-size:0.8rem;min-width:3.5rem;", "{badge}" }
                }
                if !is_current {
                    button {
                        style: "border:1px solid var(--gb-border-strong);background:transparent;color:var(--gb-text-muted);\
                                border-radius:var(--gb-radius);padding:0.15rem 0.45rem;cursor:pointer;font-size:0.72rem;",
                        onclick: move |_| {
                            on_event.call(UiEvent::CheckoutBranch(checkout_name.clone()));
                        },
                        "Checkout"
                    }
                }
                if !is_remote {
                    button {
                        style: "border:1px solid var(--gb-border-strong);background:transparent;color:var(--gb-text-muted);\
                                border-radius:var(--gb-radius);padding:0.15rem 0.45rem;cursor:pointer;font-size:0.72rem;",
                        title: "Instant Worktree (W)",
                        onclick: move |_| {
                            on_event.call(UiEvent::InstantWorktree {
                                branch: branch.name.clone(),
                            });
                        },
                        "W"
                    }
                    button {
                        style: "border:1px solid var(--gb-border-strong);background:transparent;color:var(--gb-text-muted);\
                                border-radius:var(--gb-radius);padding:0.15rem 0.45rem;cursor:pointer;font-size:0.72rem;",
                        onclick: move |_| on_track.call(track_name.clone()),
                        "Track"
                    }
                    button {
                        style: "border:1px solid var(--gb-border-strong);background:transparent;color:var(--gb-text-muted);\
                                border-radius:var(--gb-radius);padding:0.15rem 0.45rem;cursor:pointer;font-size:0.72rem;",
                        onclick: move |_| {
                            on_event.call(UiEvent::ShowDivergence {
                                other: other.clone(),
                            });
                        },
                        "Divergence"
                    }
                    if !is_current {
                        button {
                            style: "border:1px solid var(--gb-danger-border);background:transparent;color:var(--gb-danger);\
                                    border-radius:var(--gb-radius);padding:0.15rem 0.45rem;cursor:pointer;font-size:0.72rem;",
                            onclick: move |_| {
                                on_event.call(UiEvent::RequestDeleteBranch(delete_name.clone()));
                            },
                            "Delete"
                        }
                    }
                }
            }
            div {
                style: "font-size:0.72rem;opacity:0.55;font-family:var(--gb-mono);",
                "{upstream}"
                if !last.is_empty() {
                    span { " · {last}" }
                }
            }
        }
    }
}

/// Returns branches whose names contain `needle` (case-insensitive).
#[must_use]
pub fn filter_branches<'a>(branches: &'a [BranchInfo], needle: &str) -> Vec<&'a BranchInfo> {
    let needle = needle.to_ascii_lowercase();
    branches
        .iter()
        .filter(|b| needle.is_empty() || b.name.to_ascii_lowercase().contains(&needle))
        .collect()
}

#[component]
fn CleanupPanel(
    candidates: Vec<crate::app::branch_cleanup::CleanupCandidate>,
    selected: Vec<String>,
    on_event: EventHandler<UiEvent>,
) -> Element {
    let selected_n = selected.len();
    rsx! {
        div {
            style: "border:1px solid var(--gb-danger-border);background:var(--gb-danger-bg);border-radius:var(--gb-radius-lg);padding:0.65rem;\
                    display:flex;flex-direction:column;gap:0.45rem;",
            p {
                style: "margin:0;font-size:0.85rem;font-weight:600;",
                "Delete outdated branches?"
            }
            p {
                style: "margin:0;font-size:0.75rem;opacity:0.7;",
                "Merged, squash-merged, and/or stale (≥30d). Current and protected branches are excluded. Uses safe delete (-d)."
            }
            ul {
                style: "list-style:none;margin:0;padding:0;display:flex;flex-direction:column;gap:0.25rem;max-height:12rem;overflow:auto;",
                for c in candidates {
                    {
                        let name = c.name.clone();
                        let name_cb = c.name.clone();
                        let name_ex = c.name.clone();
                        let checked = selected.iter().any(|s| s == &c.name);
                        let reason = crate::app::branch_cleanup::reason_label(c.reason);
                        rsx! {
                            li {
                                key: "{name}",
                                div {
                                    style: "display:flex;align-items:center;gap:0.4rem;font-size:0.8rem;",
                                    label {
                                        style: "display:flex;align-items:center;gap:0.4rem;cursor:pointer;flex:1;",
                                        input {
                                            r#type: "checkbox",
                                            checked: checked,
                                            onchange: move |_| {
                                                on_event.call(UiEvent::ToggleCleanupBranch(name_cb.clone()));
                                            },
                                        }
                                        span { style: "font-family:var(--gb-mono);", "{name}" }
                                        span { style: "opacity:0.55;", "({reason})" }
                                    }
                                    button {
                                        r#type: "button",
                                        title: "Exclude as false positive",
                                        style: "border:0;background:transparent;color:var(--gb-text-faint);cursor:pointer;font-size:0.7rem;",
                                        onclick: move |_| {
                                            on_event.call(UiEvent::ExcludeCleanupBranch(name_ex.clone()));
                                        },
                                        "Exclude"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div {
                style: "display:flex;gap:0.4rem;align-items:center;",
                button {
                    r#type: "button",
                    style: "border:0;background:var(--gb-danger-strong);color:white;border-radius:var(--gb-radius);\
                            padding:0.3rem 0.7rem;cursor:pointer;font-size:0.78rem;font-weight:600;",
                    disabled: selected_n == 0,
                    onclick: move |_| on_event.call(UiEvent::ConfirmBranchCleanup),
                    "Delete {selected_n}"
                }
                button {
                    r#type: "button",
                    style: "border:1px solid var(--gb-border-strong);background:transparent;color:var(--gb-chip-text);border-radius:var(--gb-radius);\
                            padding:0.3rem 0.7rem;cursor:pointer;font-size:0.78rem;",
                    onclick: move |_| on_event.call(UiEvent::CancelBranchCleanup),
                    "Cancel"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::model::BranchHealth;

    fn sample(name: &str) -> BranchInfo {
        BranchInfo {
            name: name.into(),
            upstream: None,
            health: BranchHealth::Local,
            ahead: 0,
            behind: 0,
            last_commit: None,
            is_remote: false,
            stale_days: None,
        }
    }

    #[test]
    fn filter_branches_matches_substring_case_insensitive() {
        let all = vec![sample("main"), sample("feature"), sample("wip")];
        let hit = filter_branches(&all, "FeAt");
        assert_eq!(hit.len(), 1);
        assert_eq!(hit[0].name, "feature");
        assert_eq!(filter_branches(&all, "").len(), 3);
        assert!(filter_branches(&all, "zzz").is_empty());
    }
}
