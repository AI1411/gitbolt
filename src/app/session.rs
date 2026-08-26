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
use crate::cache::{CacheKey, RepoCaches};
use crate::task::{Outcome, Priority, TaskRunner};
use crate::watcher::{RepoWatcher, WatchEvent};

/// Number of worker threads for Git / IO work.
const WORKERS: usize = 4;

/// Owns application state and the background task pool.
pub struct AppSession {
    pub state: AppState,
    runner: TaskRunner<AppMessage>,
    rx: Receiver<Outcome<AppMessage>>,
    /// Path of the currently opened repository (for follow-up commands).
    repo_path: Option<PathBuf>,
    /// Live filesystem watcher for the open repository (issue #12 / #7).
    watcher: Option<RepoWatcher>,
    watch_rx: Option<Receiver<WatchEvent>>,
    /// Diff/blame caches for the open repository (issue #13).
    caches: RepoCaches,
    /// Next auto-fetch deadline (issue #19).
    next_auto_fetch: Option<std::time::Instant>,
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
            watcher: None,
            watch_rx: None,
            caches: RepoCaches::new(),
            next_auto_fetch: None,
        }
    }

    /// Applies a UI event, persists Recent on open, and submits commands.
    pub fn dispatch_event(&mut self, event: UiEvent) {
        let opening = matches!(event, UiEvent::OpenRepository(_));
        let closing = matches!(event, UiEvent::CloseRepository);
        let is_fetch = matches!(event, UiEvent::Fetch);
        let commands = reduce(&mut self.state, event);
        if opening {
            let _ = save_recent(&self.state.repository.recent);
            if let Some(path) = self.state.repository.path.clone() {
                self.repo_path = Some(path);
            }
            self.stop_watcher();
            self.caches = RepoCaches::new();
            self.schedule_auto_fetch();
        }
        if closing || matches!(self.state.repository.status, RepositoryStatus::NotOpened) {
            self.repo_path = None;
            self.stop_watcher();
            self.caches = RepoCaches::new();
            self.next_auto_fetch = None;
        }
        if is_fetch {
            self.schedule_auto_fetch();
        }
        self.submit_commands(commands);
    }

    /// Opens `path` if provided (CLI / drag-drop bootstrap).
    pub fn open_if_present(&mut self, path: Option<PathBuf>) {
        if let Some(path) = path {
            self.dispatch_event(UiEvent::OpenRepository(path));
        }
    }

    /// Drains completed worker messages and watch events. Returns true if state changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok(outcome) = self.rx.try_recv() {
            self.cache_diff_result(&outcome.message);
            if matches!(
                &outcome.message,
                AppMessage::CheckoutCompleted { result: Ok(_), .. }
                    | AppMessage::CommitCompleted { result: Ok(_), .. }
                    | AppMessage::RemoteCompleted {
                        op: crate::app::message::RemoteOp::Pull,
                        result: Ok(_),
                        ..
                    }
            ) {
                self.caches.on_head_change();
            }
            if matches!(
                &outcome.message,
                AppMessage::RemoteCompleted {
                    op: crate::app::message::RemoteOp::Fetch,
                    result: Ok(_),
                    ..
                }
            ) {
                self.schedule_auto_fetch();
            }
            let follow = apply(&mut self.state, outcome.message);
            self.submit_commands(follow);
            changed = true;
        }
        if let Some(path) = self.state.ui.pending_open_worktree.take() {
            self.dispatch_event(UiEvent::OpenRepository(path));
            changed = true;
        }
        self.ensure_watcher();
        if self.poll_watch() {
            changed = true;
        }
        if self.poll_auto_fetch() {
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
                self.cache_diff_result(&outcome.message);
                if matches!(
                    &outcome.message,
                    AppMessage::CheckoutCompleted { result: Ok(_), .. }
                        | AppMessage::CommitCompleted { result: Ok(_), .. }
                        | AppMessage::RemoteCompleted {
                            op: crate::app::message::RemoteOp::Pull,
                            result: Ok(_),
                            ..
                        }
                ) {
                    self.caches.on_head_change();
                }
                if matches!(
                    &outcome.message,
                    AppMessage::RemoteCompleted {
                        op: crate::app::message::RemoteOp::Fetch,
                        result: Ok(_),
                        ..
                    } | AppMessage::RepositoryOpened { result: Ok(_), .. }
                ) {
                    self.schedule_auto_fetch();
                }
                let follow = apply(&mut self.state, outcome.message);
                self.submit_commands(follow);
                let _ = self.poll();
                true
            }
            Err(_) => false,
        }
    }

    fn cache_diff_result(&mut self, message: &AppMessage) {
        match message {
            AppMessage::DiffLoaded {
                result: Ok(content),
                ..
            } => {
                if let Some(head) = self.state.repository.head.oid.clone() {
                    let key =
                        CacheKey::new(head, content.target.path.clone(), content.target.staged);
                    self.caches.diff.insert(key, content.clone());
                }
            }
            AppMessage::BlameEnriched {
                target, origins, ..
            } if !origins.is_empty() => {
                if let Some(head) = self.state.repository.head.oid.clone() {
                    let key = CacheKey::new(head.clone(), target.path.clone(), target.staged);
                    let mut map = (*self
                        .caches
                        .blame
                        .get(&key)
                        .unwrap_or_else(|| std::sync::Arc::new(std::collections::HashMap::new())))
                    .clone();
                    for (line, summary) in origins {
                        map.insert(
                            *line,
                            crate::git::CommitInfo {
                                oid: summary.oid.0.clone(),
                                summary: summary.summary.clone(),
                                author: summary.author.clone(),
                                time: summary.timestamp,
                            },
                        );
                    }
                    self.caches.blame.insert(key, map);
                }
            }
            _ => {}
        }
    }

    fn ensure_watcher(&mut self) {
        if !self.state.is_ready() {
            self.stop_watcher();
            return;
        }
        if self.watcher.is_some() {
            return;
        }
        let Some(path) = self.repo_path.clone() else {
            return;
        };
        if let Ok((watcher, rx)) = RepoWatcher::start(&path, Duration::from_millis(200)) {
            self.watcher = Some(watcher);
            self.watch_rx = Some(rx);
        }
        // Watcher is best-effort; status still refreshes after Git ops.
    }

    fn stop_watcher(&mut self) {
        self.watcher = None;
        self.watch_rx = None;
    }

    fn schedule_auto_fetch(&mut self) {
        let secs = self.state.ui.auto_fetch_secs;
        self.next_auto_fetch = if secs == 0 || !self.state.is_ready() {
            None
        } else {
            Some(std::time::Instant::now() + Duration::from_secs(secs))
        };
    }

    fn poll_auto_fetch(&mut self) -> bool {
        let Some(deadline) = self.next_auto_fetch else {
            return false;
        };
        if std::time::Instant::now() < deadline {
            return false;
        }
        if !self.state.is_ready() || self.state.background.remote_label.is_some() {
            self.schedule_auto_fetch();
            return false;
        }
        self.state.background.remote_label = Some("fetching…".into());
        self.state.background.inflight = self.state.background.inflight.saturating_add(1);
        let gen = self.state.generation;
        self.schedule_auto_fetch();
        self.submit_commands(vec![Command::AutoFetch { generation: gen }]);
        true
    }

    fn poll_watch(&mut self) -> bool {
        let Some(rx) = &self.watch_rx else {
            return false;
        };
        let mut saw_head = false;
        let mut saw_any = false;
        let mut wt_paths = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            saw_any = true;
            match ev {
                WatchEvent::Head => saw_head = true,
                WatchEvent::WorkingTree(paths) => wt_paths.extend(paths),
            }
        }
        if !saw_any {
            return false;
        }
        if saw_head {
            self.caches.on_head_change();
        }
        if !wt_paths.is_empty() {
            self.caches.on_working_tree_change(wt_paths);
        }
        let gen = self.state.generation;
        let mut cmds = vec![Command::LoadStatus { generation: gen }];
        if saw_head {
            cmds.push(Command::LoadBranches { generation: gen });
            cmds.push(Command::LoadWorktrees { generation: gen });
        }
        self.submit_commands(cmds);
        true
    }

    fn submit_commands(&mut self, commands: Vec<Command>) {
        self.runner.set_generation(self.state.generation);
        for cmd in commands {
            if let Command::LoadDiff { target, generation } = &cmd {
                if let Some(head) = self.state.repository.head.oid.clone() {
                    let key = CacheKey::new(head, target.path.clone(), target.staged);
                    if let Some(cached) = self.caches.diff.get(&key) {
                        let follow = apply(
                            &mut self.state,
                            AppMessage::DiffLoaded {
                                generation: *generation,
                                result: Ok((*cached).clone()),
                            },
                        );
                        // Avoid re-entering cache insert with same content via poll path:
                        // apply only; do not call cache_diff_result here (already cached).
                        self.submit_commands(follow);
                        continue;
                    }
                }
            }
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
        | Command::Checkout { .. }
        | Command::Fetch { .. }
        | Command::Pull { .. }
        | Command::Push { .. } => Priority::P0,
        Command::LoadDiff { .. } | Command::EnrichBlame { .. } => Priority::P1,
        Command::LoadStatus { .. }
        | Command::LoadHistoryPage { .. }
        | Command::LoadBranches { .. }
        | Command::LoadDivergence { .. }
        | Command::LoadWorktrees { .. }
        | Command::SetUpstream { .. } => Priority::P2,
        Command::EnrichBranchHealth { .. }
        | Command::CreateBranch { .. }
        | Command::DeleteBranch { .. }
        | Command::AutoFetch { .. }
        | Command::CreateWorktree { .. }
        | Command::RemoveWorktree { .. } => Priority::P3,
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
