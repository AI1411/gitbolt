//! In-view list search bar for `/` (issue #85).

use dioxus::prelude::*;

use crate::app::event::UiEvent;
use crate::app::state::AppState;

/// Props for the list search bar.
#[derive(Props, Clone, PartialEq)]
pub struct ListSearchBarProps {
    pub state: AppState,
    pub on_event: EventHandler<UiEvent>,
    pub placeholder: String,
}

/// Compact search field shown when list search is active.
#[component]
pub fn ListSearchBar(props: ListSearchBarProps) -> Element {
    if !props.state.ui.searching {
        return rsx! { Fragment {} };
    }
    let token = props.state.ui.search_focus_token;
    let query = props.state.ui.search_query.clone();
    rsx! {
        div {
            style: "display:flex;align-items:center;gap:0.4rem;margin-bottom:0.5rem;",
            input {
                key: "{token}",
                autofocus: true,
                style: "flex:1;padding:0.35rem 0.5rem;border-radius:var(--gb-radius);border:1px solid var(--gb-border-strong);\
                        background:var(--gb-bg);color:var(--gb-text);font-size:0.85rem;",
                placeholder: "{props.placeholder}",
                value: "{query}",
                onfocus: move |_| props.on_event.call(UiEvent::SetTyping(true)),
                onblur: move |_| props.on_event.call(UiEvent::SetTyping(false)),
                oninput: move |evt| props.on_event.call(UiEvent::Search(evt.value())),
                onkeydown: move |evt| {
                    if matches!(evt.data().key(), Key::Escape) {
                        evt.prevent_default();
                        props.on_event.call(UiEvent::Escape);
                    }
                },
            }
            span { style: "font-size:0.7rem;opacity:0.5;", "Esc" }
        }
    }
}

/// Case-insensitive substring match helper.
#[must_use]
pub fn matches_query(haystack: &str, query: &str) -> bool {
    let q = query.trim();
    if q.is_empty() {
        return true;
    }
    haystack
        .to_ascii_lowercase()
        .contains(&q.to_ascii_lowercase())
}
