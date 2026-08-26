//! Application session: bridges UI events to the `TaskRunner` and applies results.
//!
//! Owns [`AppState`] and dispatches [`Command`]s produced by the reducer onto
//! background workers (see `docs/design/05-architecture.md` section 9).

use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use super::command::Command;
use super::event::UiEvent;
use super::executor;
use super::message::AppMessage;
use super::recent::{load_recent, save_recent};
use super::reducer::{apply, reduce};
use super::state::{AppState, RepositoryStatus};
use crate::task::{Outcome, Priority, TaskRunner};

/// Number of worker threads for Git / IO work.
const WORKERS: usize = 4;

/// Owns application state and the background task pool.
pub struct AppSession {
    pub state: AppState,
    runner: TaskRunner<AppMessage>,
    rx: Receiver<Outcome<AppMessage>>,
    /// Path of the currently opened repository (for follow-up commands).
    repo_path: Option<PathBuf>,
}

impl AppSession {
    /// Creates a session with Recent repositories loaded from disk.
    #[must_use]
    pub fn new() -> Self {
        let (runner, rx) = TaskRunner::new(WORKERS);
        let mut state = AppState::new();
        state.repository.recent = load_recent();
        Self {
            state,
            runner,
            rx,
            repo_path: None,
        }
    }

    /// Applies a UI event, persists Recent on open, and submits commands.
    pub fn dispatch_event(&mut self, event: UiEvent) {
        let opening = matches!(event, UiEvent::OpenRepository(_));
        let commands = reduce(&mut self.state, event);
        if opening {
            let _ = save_recent(&self.state.repository.recent);
            if let Some(path) = self.state.repository.path.clone() {
                self.repo_path = Some(path);
            }
        }
        if matches!(self.state.repository.status, RepositoryStatus::NotOpened) {
            self.repo_path = None;
        }
        self.submit_commands(commands);
    }

    /// Opens `path` if provided (CLI / drag-drop bootstrap).
    pub fn open_if_present(&mut self, path: Option<PathBuf>) {
        if let Some(path) = path {
            self.dispatch_event(UiEvent::OpenRepository(path));
        }
    }

    /// Drains completed worker messages. Returns true if state changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok(outcome) = self.rx.try_recv() {
            let follow = apply(&mut self.state, outcome.message);
            self.submit_commands(follow);
            changed = true;
        }
        changed
    }

    /// Blocks briefly waiting for at least one outcome (tests / sync paths).
    pub fn poll_wait(&mut self, timeout: Duration) -> bool {
        if self.poll() {
            return true;
        }
        match self.rx.recv_timeout(timeout) {
            Ok(outcome) => {
                let follow = apply(&mut self.state, outcome.message);
                self.submit_commands(follow);
                let _ = self.poll();
                true
            }
            Err(_) => false,
        }
    }

    fn submit_commands(&mut self, commands: Vec<Command>) {
        self.runner.set_generation(self.state.generation);
        for cmd in commands {
            let generation = cmd.generation();
            let priority = command_priority(&cmd);
            let path = match &cmd {
                Command::OpenRepository { path, .. } => Some(path.clone()),
                _ => self.repo_path.clone(),
            };
            self.runner.submit(priority, generation, move || {
                executor::execute(&cmd, path.as_deref())
            });
        }
    }
}

impl Default for AppSession {
    fn default() -> Self {
        Self::new()
    }
}

fn command_priority(cmd: &Command) -> Priority {
    match cmd {
        Command::OpenRepository { .. }
        | Command::Stage { .. }
        | Command::Unstage { .. }
        | Command::StageLines { .. }
        | Command::StageAll { .. }
        | Command::UnstageAll { .. }
        | Command::Commit { .. }
        | Command::Checkout { .. } => Priority::P0,
        Command::LoadDiff { .. } => Priority::P1,
        Command::LoadStatus { .. }
        | Command::LoadHistoryPage { .. }
        | Command::LoadBranches { .. }
        | Command::LoadDivergence { .. }
        | Command::LoadWorktrees { .. } => Priority::P2,
        Command::CreateBranch { .. }
        | Command::DeleteBranch { .. }
        | Command::Fetch { .. }
        | Command::Pull { .. }
        | Command::Push { .. }
        | Command::CreateWorktree { .. } => Priority::P3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::RepositoryStatus;
    use crate::git::fixture::TempRepo;

    #[test]
    fn open_repository_reaches_ready_with_head() {
        let repo = TempRepo::init();
        repo.write("README.md", "# hello\n");
        repo.stage("README.md");
        repo.commit("initial");

        let mut session = AppSession::new();
        session.dispatch_event(UiEvent::OpenRepository(repo.path().to_path_buf()));

        let mut ready = false;
        for _ in 0..200 {
            if session.poll_wait(Duration::from_millis(10))
                && session.state.repository.status == RepositoryStatus::Ready
            {
                ready = true;
                break;
            }
        }
        assert!(
            ready,
            "expected Ready, got {:?}",
            session.state.repository.status
        );
        assert_eq!(
            session.state.repository.head.branch.as_deref(),
            Some("main")
        );
        assert!(session
            .state
            .repository
            .recent
            .iter()
            .any(|p| p == repo.path()));
    }

    #[test]
    fn open_non_repo_sets_error_status() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut session = AppSession::new();
        session.dispatch_event(UiEvent::OpenRepository(dir.path().to_path_buf()));

        let mut errored = false;
        for _ in 0..200 {
            let _ = session.poll_wait(Duration::from_millis(10));
            if matches!(session.state.repository.status, RepositoryStatus::Error(_)) {
                errored = true;
                break;
            }
        }
        assert!(errored, "expected Error status");
        assert!(session.state.ui.error_banner.is_some());
    }
}
