//! UI components for each major view.
//!
//! See `docs/design/05-architecture.md` section 15.

pub mod blame;
pub mod branches;
pub mod changes;
pub mod diff;
pub mod history;
pub mod open;
pub mod pulse;
pub mod worktrees;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use dioxus::prelude::*;

use crate::app::event::UiEvent;
use crate::app::session::AppSession;
use crate::app::state::{AppState, RepositoryStatus};

use open::OpenScreen;

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

    rsx! {
        div {
            width: "100vw",
            height: "100vh",
            match state.repository.status {
                RepositoryStatus::Ready => {
                    rsx! { ReadyPlaceholder { state: state.clone() } }
                }
                RepositoryStatus::Opening => {
                    rsx! {
                        OpenScreen {
                            recent: state.repository.recent.clone(),
                            error: None,
                            opening: true,
                            on_open: move |path| {
                                dispatch(&open_session, UiEvent::OpenRepository(path), &mut snapshot);
                            },
                        }
                    }
                }
                RepositoryStatus::NotOpened | RepositoryStatus::Error(_) => {
                    let error = match &state.repository.status {
                        RepositoryStatus::Error(msg) => Some(msg.clone()),
                        _ => state.ui.error_banner.clone(),
                    };
                    rsx! {
                        OpenScreen {
                            recent: state.repository.recent.clone(),
                            error: error,
                            opening: false,
                            on_open: move |path| {
                                dispatch(&open_session, UiEvent::OpenRepository(path), &mut snapshot);
                            },
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ReadyPlaceholder(state: AppState) -> Element {
    let branch = state.repository.head.branch.clone().unwrap_or_else(|| {
        if state.repository.head.detached {
            "detached HEAD".into()
        } else {
            "(unknown)".into()
        }
    });
    let path = state
        .repository
        .path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:0.5rem;padding:1.25rem;\
                    font-family:ui-sans-serif,system-ui,sans-serif;height:100%;\
                    background:#0f1419;color:#e8eef7;",
            div { style: "font-weight:600;font-size:1.1rem;", "GitBolt / {branch}" }
            div { style: "opacity:0.7;font-family:ui-monospace,monospace;font-size:0.85rem;", "{path}" }
            div { style: "opacity:0.55;font-size:0.85rem;margin-top:0.75rem;",
                "Repository ready — layout arrives in the next MVP issues."
            }
        }
    }
}

fn dispatch(session: &SharedSession, event: UiEvent, snapshot: &mut Signal<AppState>) {
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
}
