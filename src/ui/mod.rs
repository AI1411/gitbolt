//! UI components for each major view.
//!
//! See `docs/design/05-architecture.md` section 15.

pub mod blame;
pub mod branches;
pub mod changes;
pub mod context;
pub mod diff;
pub mod divergence;
pub mod error_banner;
pub mod history;
pub mod layout_model;
pub mod list_search;
pub mod nav;
pub mod open;
pub mod overlay;
pub mod pulse;
pub mod shell;
pub mod stashes;
pub mod theme;
pub mod worktrees;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use dioxus::prelude::*;

use crate::app::event::UiEvent;
use crate::app::session::AppSession;
use crate::app::state::{AppState, RepositoryStatus};

use open::OpenScreen;
use shell::Shell;

/// Optional path passed from the CLI (`gitbolt <path>`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CliLaunchPath(pub Option<PathBuf>);

/// Shared session guarded for the UI poll loop.
type SharedSession = Arc<Mutex<AppSession>>;

/// Root application shell — Open screen until a repository is Ready.
#[component]
pub fn App() -> Element {
    let cli = try_consume_context::<CliLaunchPath>().unwrap_or_default();
    let session: SharedSession = use_hook(|| {
        let mut session = AppSession::new();
        session.open_if_present(cli.0.clone());
        Arc::new(Mutex::new(session))
    });

    let mut snapshot = use_signal(|| {
        session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .state
            .clone()
    });

    // Poll background worker outcomes onto the UI signal.
    let session_poll = session.clone();
    use_future(move || {
        let session_poll = session_poll.clone();
        let mut snapshot = snapshot;
        async move {
            loop {
                tokio::time::sleep(Duration::from_millis(16)).await;
                let changed = session_poll
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .poll();
                if changed {
                    let state = session_poll
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .state
                        .clone();
                    snapshot.set(state);
                }
            }
        }
    });

    let state = snapshot();
    let open_session = session.clone();
    let ready_session = session.clone();

    rsx! {
        div {
            class: "gb-root",
            style: crate::ui::theme::root_style(state.ui.color_scheme),
            style { dangerous_inner_html: crate::ui::theme::GLOBAL_CSS }
            match state.repository.status {
                RepositoryStatus::Ready => {
                    rsx! {
                        Shell {
                            state: state.clone(),
                            on_event: move |event| {
                                dispatch(&ready_session, event, &mut snapshot);
                            },
                        }
                    }
                }
                RepositoryStatus::Opening => {
                    let session_open = open_session.clone();
                    let session_remove = open_session.clone();
                    let session_pin = open_session.clone();
                    let session_prune = open_session.clone();
                    let session_theme = open_session.clone();
                    rsx! {
                        OpenScreen {
                            recent: state.repository.recent.clone(),
                            error: None,
                            opening: true,
                            on_open: move |path| {
                                dispatch(&session_open, UiEvent::OpenRepository(path), &mut snapshot);
                            },
                            on_remove_recent: move |path| {
                                dispatch(&session_remove, UiEvent::RemoveRecent(path), &mut snapshot);
                            },
                            on_pin_recent: move |path| {
                                dispatch(&session_pin, UiEvent::PinRecent(path), &mut snapshot);
                            },
                            on_prune_recent: move |()| {
                                dispatch(&session_prune, UiEvent::PruneRecent, &mut snapshot);
                            },
                            color_scheme: state.ui.color_scheme,
                            on_toggle_theme: move |()| {
                                dispatch(&session_theme, UiEvent::ToggleColorScheme, &mut snapshot);
                            },
                        }
                    }
                }
                RepositoryStatus::NotOpened | RepositoryStatus::Error(_) => {
                    let error = match &state.repository.status {
                        RepositoryStatus::Error(msg) => Some(msg.clone()),
                        _ => state.ui.error_banner.clone(),
                    };
                    let session_open = open_session.clone();
                    let session_remove = open_session.clone();
                    let session_pin = open_session.clone();
                    let session_prune = open_session.clone();
                    let session_theme = open_session.clone();
                    rsx! {
                        OpenScreen {
                            recent: state.repository.recent.clone(),
                            error: error,
                            opening: false,
                            on_open: move |path| {
                                dispatch(&session_open, UiEvent::OpenRepository(path), &mut snapshot);
                            },
                            on_remove_recent: move |path| {
                                dispatch(&session_remove, UiEvent::RemoveRecent(path), &mut snapshot);
                            },
                            on_pin_recent: move |path| {
                                dispatch(&session_pin, UiEvent::PinRecent(path), &mut snapshot);
                            },
                            on_prune_recent: move |()| {
                                dispatch(&session_prune, UiEvent::PruneRecent, &mut snapshot);
                            },
                            color_scheme: state.ui.color_scheme,
                            on_toggle_theme: move |()| {
                                dispatch(&session_theme, UiEvent::ToggleColorScheme, &mut snapshot);
                            },
                        }
                    }
                }
            }
        }
    }
}

fn dispatch(session: &SharedSession, event: UiEvent, snapshot: &mut Signal<AppState>) {
    if let UiEvent::CopyText(text) = &event {
        let _ = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text.clone()));
    }
    if let UiEvent::OpenUrl(url) = &event {
        let _ = ::open::that(url);
    }
    let mut guard = session
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.dispatch_event(event);
    snapshot.set(guard.state.clone());
}

/// Parses CLI arguments into an optional repository path.
///
/// Accepts `gitbolt`, `gitbolt .`, or `gitbolt <path>`. Ignores flags for now.
#[must_use]
pub fn parse_cli_path<I, S>(args: I) -> Option<PathBuf>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut args = args.into_iter();
    let _exe = args.next();
    for arg in args {
        let arg = PathBuf::from(arg.as_ref());
        let s = arg.to_string_lossy();
        if s.starts_with('-') {
            continue;
        }
        return Some(arg);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::event::UiEvent;
    use crate::app::model::View;
    use crate::app::reducer::reduce;
    use crate::app::state::AppState;

    #[test]
    fn parse_cli_path_reads_positional() {
        assert_eq!(
            parse_cli_path(["gitbolt", "/tmp/repo"]),
            Some(PathBuf::from("/tmp/repo"))
        );
        assert_eq!(parse_cli_path(["gitbolt", "."]), Some(PathBuf::from(".")));
        assert_eq!(parse_cli_path(["gitbolt"]), None);
        assert_eq!(parse_cli_path(["gitbolt", "--help"]), None);
    }

    #[test]
    fn select_view_updates_navigation_state() {
        let mut state = AppState::new();
        assert_eq!(state.navigation.active_view, View::Changes);
        let _ = reduce(&mut state, UiEvent::SelectView(View::History));
        assert_eq!(state.navigation.active_view, View::History);
        let _ = reduce(&mut state, UiEvent::ToggleContextPanel);
        assert!(!state.navigation.context_panel_open);
    }
}
