//! The reducer: the single place where state transitions happen.
//!
//! Flow (see `docs/design/05-architecture.md` section 9):
//!
//! ```text
//! UI -> UiEvent -> [reduce] -> (State mutation + Command[]) -> Worker
//!    -> AppMessage -> [apply] -> (State mutation + Command[]) -> re-render
//! ```
//!
//! [`reduce`] handles user events (optimistic updates + follow-up commands).
//! [`apply`] handles worker results, discarding stale ones via [`Generation`].

use std::path::Path;

use super::command::Command;
use super::event::UiEvent;
use super::message::{AppMessage, RemoteOp};
use super::model::{
    ChangeKind, CommitSummary, DiffContent, DiffTarget, FileChange, Generation, Loadable, View,
};
use super::state::{AppState, HistoryFilter, RepositoryState, RepositoryStatus};

/// Number of commits fetched per history page.
pub const HISTORY_PAGE: usize = 100;

/// Applies a UI event to the state and returns commands to execute.
#[allow(clippy::too_many_lines)]
pub fn reduce(state: &mut AppState, event: UiEvent) -> Vec<Command> {
    let gen = state.generation;
    match event {
        UiEvent::OpenRepository(path) => {
            let recent = merge_recent(&state.repository, &path);
            let generation = gen.next();
            *state = AppState::new();
            state.generation = generation;
            state.repository.recent = recent;
            state.repository.path = Some(path.clone());
            state.repository.status = RepositoryStatus::Opening;
            issue(state, vec![Command::OpenRepository { path, generation }])
        }
        UiEvent::CloseRepository => {
            let recent = std::mem::take(&mut state.repository.recent);
            let generation = gen.next();
            *state = AppState::new();
            state.generation = generation;
            state.repository.recent = recent;
            Vec::new()
        }
        UiEvent::SelectView(view) => {
            if state.navigation.active_view != view {
                state
                    .navigation
                    .back_stack
                    .push(state.navigation.active_view);
                state.navigation.active_view = view;
            }
            lazy_load_view(state, view)
        }
        UiEvent::ToggleContextPanel => {
            state.navigation.context_panel_open = !state.navigation.context_panel_open;
            Vec::new()
        }
        UiEvent::SelectFile { path, staged } => {
            state.selection.file = Some(path.clone());
            let target = DiffTarget { path, staged };
            state.diff.target = Some(target.clone());
            state.diff.content = Loadable::Loading;
            state.diff.selected_lines.clear();
            state.diff.focused_hunk = 0;
            issue(
                state,
                vec![Command::LoadDiff {
                    target,
                    generation: gen,
                }],
            )
        }
        UiEvent::NavigateChanges { delta } => {
            let entries = changes_entries(state);
            if entries.is_empty() {
                return Vec::new();
            }
            let current = state.diff.target.as_ref().and_then(|t| {
                entries
                    .iter()
                    .position(|(p, staged)| p == &t.path && *staged == t.staged)
            });
            let len = i64::try_from(entries.len()).unwrap_or(1);
            let next = match current {
                Some(i) => {
                    let i = i64::try_from(i).unwrap_or(0);
                    let idx = (i + i64::from(delta)).rem_euclid(len);
                    usize::try_from(idx).unwrap_or(0)
                }
                None => {
                    if delta >= 0 {
                        0
                    } else {
                        entries.len() - 1
                    }
                }
            };
            let (path, staged) = entries[next].clone();
            reduce(state, UiEvent::SelectFile { path, staged })
        }
        UiEvent::SelectCommit(oid) => {
            state.selection.commit = Some(oid.clone());
            state.navigation.context_panel_open = true;
            state.context.commit = Loadable::Loading;
            issue(
                state,
                vec![Command::LoadCommitDetail {
                    oid,
                    generation: gen,
                }],
            )
        }
        UiEvent::SelectBranch(name) => {
            state.selection.branch = Some(name);
            state.selection.commit = None;
            state.context.commit = Loadable::Idle;
            state.navigation.context_panel_open = true;
            Vec::new()
        }
        UiEvent::SetBranchFilter(filter) => {
            state.branch.filter = filter;
            Vec::new()
        }
        UiEvent::SetUpstream { branch, upstream } => issue(
            state,
            vec![Command::SetUpstream {
                branch,
                upstream,
                generation: gen,
            }],
        ),
        UiEvent::ShowDivergence { other } => {
            let left = state
                .repository
                .head
                .branch
                .clone()
                .or_else(|| state.branch.current.clone())
                .unwrap_or_else(|| "HEAD".into());
            state.divergence.left = Some(left.clone());
            state.divergence.right = Some(other.clone());
            state.divergence.loading = true;
            state.divergence.left_only.clear();
            state.divergence.right_only.clear();
            state.divergence.merge_base = None;
            state.navigation.active_view = View::Branches;
            issue(
                state,
                vec![Command::LoadDivergence {
                    left,
                    right: other,
                    generation: gen,
                }],
            )
        }
        UiEvent::ClearDivergence => {
            state.divergence = crate::app::state::DivergenceState::default();
            Vec::new()
        }
        UiEvent::SetDiffView(view) => {
            state.diff.view = view;
            Vec::new()
        }
        UiEvent::NavigateHunk { delta } => {
            let n = state.diff.content.ready().map_or(0, |c| c.hunks.len());
            if n == 0 {
                return Vec::new();
            }
            let len = i64::try_from(n).unwrap_or(1);
            let cur = i64::try_from(state.diff.focused_hunk).unwrap_or(0);
            let next = (cur + i64::from(delta)).rem_euclid(len);
            state.diff.focused_hunk = usize::try_from(next).unwrap_or(0);
            Vec::new()
        }
        UiEvent::ToggleDiffLine(index) => {
            if let Some(pos) = state.diff.selected_lines.iter().position(|i| *i == index) {
                state.diff.selected_lines.remove(pos);
            } else {
                state.diff.selected_lines.push(index);
                state.diff.selected_lines.sort_unstable();
            }
            Vec::new()
        }
        UiEvent::ClearDiffLineSelection => {
            state.diff.selected_lines.clear();
            Vec::new()
        }
        UiEvent::StageSelectedLines => stage_selected_lines(state, gen, false),
        UiEvent::UnstageSelectedLines => stage_selected_lines(state, gen, true),
        UiEvent::StageFile(path) => {
            stage_one(state, &path);
            issue(
                state,
                vec![Command::Stage {
                    path,
                    generation: gen,
                }],
            )
        }
        UiEvent::UnstageFile(path) => {
            unstage_one(state, &path);
            issue(
                state,
                vec![Command::Unstage {
                    path,
                    generation: gen,
                }],
            )
        }
        UiEvent::StageAll => {
            stage_all(state);
            issue(state, vec![Command::StageAll { generation: gen }])
        }
        UiEvent::UnstageAll => {
            unstage_all(state);
            issue(state, vec![Command::UnstageAll { generation: gen }])
        }
        UiEvent::ToggleStageSelection => {
            let Some(target) = state.diff.target.clone() else {
                return Vec::new();
            };
            if target.staged {
                reduce(state, UiEvent::UnstageFile(target.path))
            } else {
                reduce(state, UiEvent::StageFile(target.path))
            }
        }
        UiEvent::StageFocusedHunk => {
            let Some(content) = state.diff.content.ready().cloned() else {
                return Vec::new();
            };
            let Some(hunk) = content.hunks.get(state.diff.focused_hunk) else {
                return Vec::new();
            };
            let lines: Vec<usize> = hunk
                .lines
                .iter()
                .filter(|l| l.origin == '+' || l.origin == '-')
                .map(|l| l.body_index)
                .collect();
            if lines.is_empty() {
                return Vec::new();
            }
            state.diff.selected_lines = lines;
            if content.target.staged {
                stage_selected_lines(state, gen, true)
            } else {
                stage_selected_lines(state, gen, false)
            }
        }
        UiEvent::SetCommitMessage(message) => {
            state.ui.commit_message = message;
            Vec::new()
        }
        UiEvent::FocusCommitInput => {
            state.navigation.context_panel_open = true;
            state.ui.commit_focus_token = state.ui.commit_focus_token.wrapping_add(1);
            Vec::new()
        }
        UiEvent::Commit => reduce_commit(state, gen),
        UiEvent::CreateBranch(name) => {
            let name = name.trim().to_string();
            if name.is_empty() {
                set_error(state, "ブランチ名を入力してください".into());
                return Vec::new();
            }
            state.ui.new_branch_name.clear();
            issue(
                state,
                vec![Command::CreateBranch {
                    name,
                    generation: gen,
                }],
            )
        }
        UiEvent::CheckoutBranch(name) => issue(
            state,
            vec![Command::Checkout {
                name,
                generation: gen,
            }],
        ),
        UiEvent::RequestDeleteBranch(name) => {
            if name.trim().is_empty() {
                return Vec::new();
            }
            state.ui.confirm_delete_branch = Some(name);
            Vec::new()
        }
        UiEvent::ConfirmDeleteBranch => {
            let Some(name) = state.ui.confirm_delete_branch.take() else {
                return Vec::new();
            };
            issue(
                state,
                vec![Command::DeleteBranch {
                    name,
                    generation: gen,
                }],
            )
        }
        UiEvent::CancelDeleteBranch => {
            state.ui.confirm_delete_branch = None;
            Vec::new()
        }
        UiEvent::Fetch => {
            state.background.remote_label = Some("fetching…".into());
            issue(state, vec![Command::Fetch { generation: gen }])
        }
        UiEvent::Pull => {
            state.background.remote_label = Some("pulling…".into());
            issue(state, vec![Command::Pull { generation: gen }])
        }
        UiEvent::Push => {
            state.background.remote_label = Some("pushing…".into());
            issue(state, vec![Command::Push { generation: gen }])
        }
        UiEvent::SetOpenAfterInstantWorktree(enabled) => {
            state.ui.open_after_instant_worktree = enabled;
            Vec::new()
        }
        UiEvent::CreateWorktree { branch, path } => {
            if branch.trim().is_empty() {
                set_error(state, "ブランチ名を入力してください".into());
                return Vec::new();
            }
            issue(
                state,
                vec![Command::CreateWorktree {
                    branch,
                    path,
                    generation: gen,
                }],
            )
        }
        UiEvent::InstantWorktree { branch } => reduce_instant_worktree(state, gen, &branch),
        UiEvent::RequestRemoveWorktree(path) => {
            state.ui.confirm_remove_worktree = Some(path);
            Vec::new()
        }
        UiEvent::ConfirmRemoveWorktree => {
            let Some(path) = state.ui.confirm_remove_worktree.take() else {
                return Vec::new();
            };
            issue(
                state,
                vec![Command::RemoveWorktree {
                    path,
                    generation: gen,
                }],
            )
        }
        UiEvent::CancelRemoveWorktree => {
            state.ui.confirm_remove_worktree = None;
            Vec::new()
        }
        UiEvent::StashSave { message } => issue(
            state,
            vec![Command::StashSave {
                message,
                generation: gen,
            }],
        ),
        UiEvent::SelectStash(index) => {
            state.stash.selected = Some(index);
            state.stash.diff = Loadable::Loading;
            issue(
                state,
                vec![Command::LoadStashDiff {
                    index,
                    generation: gen,
                }],
            )
        }
        UiEvent::StashApply(index) => issue(
            state,
            vec![Command::StashApply {
                index,
                generation: gen,
            }],
        ),
        UiEvent::StashPop(index) => issue(
            state,
            vec![Command::StashPop {
                index,
                generation: gen,
            }],
        ),
        UiEvent::RequestDropStash(index) => {
            state.ui.confirm_drop_stash = Some(index);
            Vec::new()
        }
        UiEvent::ConfirmDropStash => {
            let Some(index) = state.ui.confirm_drop_stash.take() else {
                return Vec::new();
            };
            issue(
                state,
                vec![Command::StashDrop {
                    index,
                    generation: gen,
                }],
            )
        }
        UiEvent::CancelDropStash => {
            state.ui.confirm_drop_stash = None;
            Vec::new()
        }
        UiEvent::LoadMoreHistory => {
            if state.history.loading || !state.history.has_more {
                return Vec::new();
            }
            state.history.loading = true;
            let offset = state.history.commits.len();
            issue(
                state,
                vec![Command::LoadHistoryPage {
                    filter: state.history.filter.clone(),
                    offset,
                    generation: gen,
                }],
            )
        }
        UiEvent::ShowFileHistory { path } => {
            state.selection.file = Some(path.clone());
            state.navigation.active_view = View::History;
            state.navigation.context_panel_open = true;
            reset_history(state, HistoryFilter::File { path: path.clone() });
            issue(
                state,
                vec![Command::LoadHistoryPage {
                    filter: HistoryFilter::File { path },
                    offset: 0,
                    generation: gen,
                }],
            )
        }
        UiEvent::ShowLineHistory { path, line } => {
            state.selection.file = Some(path.clone());
            state.navigation.active_view = View::History;
            state.navigation.context_panel_open = true;
            reset_history(
                state,
                HistoryFilter::Line {
                    path: path.clone(),
                    line,
                },
            );
            issue(
                state,
                vec![Command::LoadHistoryPage {
                    filter: HistoryFilter::Line { path, line },
                    offset: 0,
                    generation: gen,
                }],
            )
        }
        UiEvent::ClearHistoryFilter => {
            reset_history(state, HistoryFilter::All);
            issue(
                state,
                vec![Command::LoadHistoryPage {
                    filter: HistoryFilter::All,
                    offset: 0,
                    generation: gen,
                }],
            )
        }
        UiEvent::Search(query) => {
            state.ui.searching = !query.is_empty();
            state.ui.search_query = query;
            Vec::new()
        }
        UiEvent::CopyText(_text) => {
            state.ui.copy_feedback = Some("Copied to clipboard".into());
            Vec::new()
        }
        UiEvent::DismissError => {
            state.ui.error_banner = None;
            state.background.last_error = None;
            Vec::new()
        }
    }
}

/// Applies a worker message to the state and returns follow-up commands.
#[allow(clippy::too_many_lines)]
pub fn apply(state: &mut AppState, message: AppMessage) -> Vec<Command> {
    // Every message is terminal: it completes exactly one in-flight command.
    state.background.inflight = state.background.inflight.saturating_sub(1);

    // Discard results from a superseded repository context.
    if message.generation() != state.generation {
        return Vec::new();
    }
    let gen = state.generation;

    match message {
        AppMessage::RepositoryOpened { result, .. } => match result {
            Ok(data) => {
                state.repository.status = RepositoryStatus::Ready;
                state.repository.head = data.head;
                issue(
                    state,
                    vec![
                        Command::LoadStatus { generation: gen },
                        Command::LoadBranches { generation: gen },
                        Command::LoadWorktrees { generation: gen },
                        Command::LoadHistoryPage {
                            filter: state.history.filter.clone(),
                            offset: 0,
                            generation: gen,
                        },
                    ],
                )
            }
            Err(err) => {
                state.repository.status = RepositoryStatus::Error(err.clone());
                set_error(state, err);
                Vec::new()
            }
        },
        AppMessage::StatusLoaded { result, .. } => {
            match result {
                Ok(status) => {
                    state.changes.staged = status.staged.into();
                    state.changes.unstaged = status.unstaged.into();
                    state.changes.untracked = status.untracked.into();
                    state.changes.conflicted = status.conflicted.into();
                    state.changes.loaded = true;
                }
                Err(err) => set_error(state, err),
            }
            Vec::new()
        }
        AppMessage::DiffLoaded { result, .. } => {
            match result {
                Ok(content) => {
                    let mut cmds = Vec::new();
                    if state.diff.target.as_ref() == Some(&content.target) {
                        let lines = old_lines_from_diff(&content);
                        let target = content.target.clone();
                        state.diff.content = Loadable::Ready(content);
                        if !lines.is_empty() {
                            // Phase 1: first line (current) + small nearby window, then rest.
                            let (priority, remaining) = split_blame_phases(&lines);
                            cmds.push(Command::EnrichBlame {
                                target,
                                lines: priority,
                                remaining,
                                generation: gen,
                            });
                        }
                    }
                    issue(state, cmds)
                }
                Err(err) => {
                    if state.diff.content.is_loading() {
                        state.diff.content = Loadable::Failed(err);
                    }
                    Vec::new()
                }
            }
        }
        AppMessage::BlameEnriched {
            target,
            origins,
            remaining,
            ..
        } => {
            if state.diff.target.as_ref() == Some(&target) {
                if let Loadable::Ready(content) = &mut state.diff.content {
                    apply_blame_origins(content, &origins);
                }
            }
            if remaining.is_empty() {
                Vec::new()
            } else {
                let (priority, rest) = split_blame_phases(&remaining);
                issue(
                    state,
                    vec![Command::EnrichBlame {
                        target,
                        lines: priority,
                        remaining: rest,
                        generation: gen,
                    }],
                )
            }
        }
        AppMessage::HistoryPageLoaded {
            offset,
            filter,
            result,
            ..
        } => {
            state.history.loading = false;
            state.history.filter = filter;
            match result {
                Ok(commits) => {
                    state.history.has_more = commits.len() >= HISTORY_PAGE;
                    if offset == 0 {
                        state.history.commits = commits;
                    } else {
                        state.history.commits.extend(commits);
                    }
                }
                Err(err) => set_error(state, err),
            }
            Vec::new()
        }
        AppMessage::BranchesLoaded { result, .. } => match result {
            Ok(data) => {
                let pending = data.pending_health;
                state.branch.branches = data.branches.into();
                state.branch.current = data.current;
                state.branch.recent = data.recent;
                state.branch.loaded = true;
                if pending.is_empty() {
                    Vec::new()
                } else {
                    issue(
                        state,
                        vec![Command::EnrichBranchHealth {
                            names: pending,
                            generation: gen,
                        }],
                    )
                }
            }
            Err(err) => {
                set_error(state, err);
                Vec::new()
            }
        },
        AppMessage::BranchHealthEnriched { result, .. } => {
            match result {
                Ok(updates) => {
                    let mut branches = state.branch.branches.to_vec();
                    for upd in updates {
                        if let Some(b) = branches.iter_mut().find(|b| b.name == upd.name) {
                            b.ahead = upd.ahead;
                            b.behind = upd.behind;
                            b.health = upd.health;
                        }
                    }
                    state.branch.branches = branches.into();
                }
                Err(err) => set_error(state, err),
            }
            Vec::new()
        }
        AppMessage::DivergenceLoaded {
            left,
            right,
            result,
            ..
        } => {
            state.divergence.loading = false;
            state.divergence.left = Some(left);
            state.divergence.right = Some(right);
            match result {
                Ok(data) => {
                    state.divergence.merge_base = data.merge_base;
                    state.divergence.left_only = data.left_only;
                    state.divergence.right_only = data.right_only;
                }
                Err(err) => set_error(state, err),
            }
            Vec::new()
        }
        AppMessage::WorktreesLoaded { result, .. } => {
            match result {
                Ok(worktrees) => {
                    state.worktree.worktrees = worktrees.into();
                    state.worktree.loaded = true;
                }
                Err(err) => set_error(state, err),
            }
            Vec::new()
        }
        AppMessage::StageCompleted {
            result: Err(err), ..
        }
        | AppMessage::UnstageCompleted {
            result: Err(err), ..
        } => {
            // Roll back the optimistic move by re-reading status.
            set_error(state, err);
            issue(state, vec![Command::LoadStatus { generation: gen }])
        }
        AppMessage::StageCompleted { result: Ok(()), .. }
        | AppMessage::UnstageCompleted { result: Ok(()), .. } => {
            let mut cmds = vec![Command::LoadStatus { generation: gen }];
            if let Some(target) = state.diff.target.clone() {
                // Keep showing prior diff while refresh runs (progressive disclosure).
                cmds.push(Command::LoadDiff {
                    target,
                    generation: gen,
                });
            }
            issue(state, cmds)
        }
        AppMessage::CommitCompleted { result, .. } => match result {
            Ok(oid) => {
                state.ui.commit_message.clear();
                state.repository.head.oid = Some(oid);
                bump_after_head_change(state);
                let gen = state.generation;
                issue(
                    state,
                    vec![
                        Command::LoadStatus { generation: gen },
                        Command::LoadHistoryPage {
                            filter: state.history.filter.clone(),
                            offset: 0,
                            generation: gen,
                        },
                        Command::LoadBranches { generation: gen },
                    ],
                )
            }
            Err(err) => {
                // Preserve the commit message for retry.
                set_error(state, err);
                Vec::new()
            }
        },
        AppMessage::CheckoutCompleted { result, .. } => match result {
            Ok(head) => {
                state.repository.head = head;
                state.selection.file = None;
                state.diff = super::state::DiffState::default();
                bump_after_head_change(state);
                let gen = state.generation;
                issue(
                    state,
                    vec![
                        Command::LoadStatus { generation: gen },
                        Command::LoadBranches { generation: gen },
                        Command::LoadHistoryPage {
                            filter: state.history.filter.clone(),
                            offset: 0,
                            generation: gen,
                        },
                    ],
                )
            }
            Err(err) => {
                set_error(state, err);
                Vec::new()
            }
        },
        AppMessage::BranchCreated { result, .. }
        | AppMessage::BranchDeleted { result, .. }
        | AppMessage::UpstreamSet { result, .. } => match result {
            Ok(()) => {
                state.ui.confirm_delete_branch = None;
                issue(state, vec![Command::LoadBranches { generation: gen }])
            }
            Err(err) => {
                set_error(state, err);
                Vec::new()
            }
        },
        AppMessage::WorktreeCreated { result, .. } => match result {
            Ok(info) => {
                state.ui.confirm_remove_worktree = None;
                if state.ui.open_after_instant_worktree {
                    state.ui.pending_open_worktree = Some(info.path.clone());
                }
                issue(state, vec![Command::LoadWorktrees { generation: gen }])
            }
            Err(err) => {
                set_error(state, err);
                Vec::new()
            }
        },
        AppMessage::WorktreeRemoved { result, .. } => match result {
            Ok(()) => {
                state.ui.confirm_remove_worktree = None;
                issue(state, vec![Command::LoadWorktrees { generation: gen }])
            }
            Err(err) => {
                set_error(state, err);
                Vec::new()
            }
        },
        AppMessage::StashesLoaded { result, .. } => {
            match result {
                Ok(entries) => {
                    state.stash.entries = entries.into();
                    state.stash.loaded = true;
                    if let Some(selected) = state.stash.selected {
                        if !state.stash.entries.iter().any(|e| e.index == selected) {
                            state.stash.selected = None;
                            state.stash.diff = Loadable::Idle;
                        }
                    }
                }
                Err(err) => set_error(state, err),
            }
            Vec::new()
        }
        AppMessage::StashDiffLoaded { index, result, .. } => {
            if state.stash.selected == Some(index) {
                match result {
                    Ok(content) => state.stash.diff = Loadable::Ready(content),
                    Err(err) => state.stash.diff = Loadable::Failed(err),
                }
            }
            Vec::new()
        }
        AppMessage::StashSaved { result, .. }
        | AppMessage::StashApplied { result, .. }
        | AppMessage::StashPopped { result, .. } => match result {
            Ok(()) => {
                state.stash.selected = None;
                state.stash.diff = Loadable::Idle;
                reload_after_stash_mutation(state, gen)
            }
            Err(err) => {
                set_error(state, err);
                Vec::new()
            }
        },
        AppMessage::StashDropped { result, .. } => match result {
            Ok(()) => {
                state.ui.confirm_drop_stash = None;
                state.stash.selected = None;
                state.stash.diff = Loadable::Idle;
                reload_after_stash_mutation(state, gen)
            }
            Err(err) => {
                set_error(state, err);
                Vec::new()
            }
        },
        AppMessage::CommitDetailLoaded { oid, result, .. } => {
            if state.selection.commit.as_ref() == Some(&oid) {
                match result {
                    Ok(detail) => state.context.commit = Loadable::Ready(detail),
                    Err(err) => state.context.commit = Loadable::Failed(err),
                }
            }
            Vec::new()
        }
        AppMessage::RemoteCompleted { op, result, .. } => {
            state.background.remote_label = None;
            match result {
                Ok(head) => {
                    let mut cmds = vec![Command::LoadBranches { generation: gen }];
                    if matches!(op, RemoteOp::Fetch | RemoteOp::Pull) {
                        cmds.push(Command::LoadStatus { generation: gen });
                    }
                    if matches!(op, RemoteOp::Pull) {
                        if let Some(head) = head {
                            state.repository.head = head;
                        }
                        bump_after_head_change(state);
                        let gen = state.generation;
                        cmds = vec![
                            Command::LoadStatus { generation: gen },
                            Command::LoadBranches { generation: gen },
                            Command::LoadHistoryPage {
                                filter: state.history.filter.clone(),
                                offset: 0,
                                generation: gen,
                            },
                        ];
                    }
                    issue(state, cmds)
                }
                Err(err) => {
                    set_error(state, err);
                    Vec::new()
                }
            }
        }
    }
}

// --- helpers ---------------------------------------------------------------

/// Registers the given number of commands as in-flight and returns them.
fn issue(state: &mut AppState, commands: Vec<Command>) -> Vec<Command> {
    let count = u32::try_from(commands.len()).unwrap_or(u32::MAX);
    state.background.inflight = state.background.inflight.saturating_add(count);
    commands
}

/// Sets both the error banner and the last background error.
fn set_error(state: &mut AppState, message: String) {
    state.ui.error_banner = Some(message.clone());
    state.background.last_error = Some(message);
}

/// Bumps the generation and invalidates HEAD-dependent views.
fn bump_after_head_change(state: &mut AppState) {
    state.generation = state.generation.next();
    state.diff.content = Loadable::Idle;
    state.history.commits.clear();
    state.history.has_more = true;
    state.history.loading = false;
}

fn reset_history(state: &mut AppState, filter: HistoryFilter) {
    state.history.filter = filter;
    state.history.commits.clear();
    state.history.has_more = true;
    state.history.loading = true;
}

/// Builds the recent-repositories list with `path` moved to the front.
fn merge_recent(repo: &RepositoryState, path: &Path) -> Vec<std::path::PathBuf> {
    let mut recent = repo.recent.clone();
    crate::app::recent::push_recent(&mut recent, path.to_path_buf());
    recent
}

/// Flat Changes list order: staged → conflicted → unstaged → untracked.
fn changes_entries(state: &AppState) -> Vec<(std::path::PathBuf, bool)> {
    let mut out = Vec::new();
    for f in state.changes.staged.iter() {
        out.push((f.path.clone(), true));
    }
    for f in state.changes.conflicted.iter() {
        out.push((f.path.clone(), false));
    }
    for f in state
        .changes
        .unstaged
        .iter()
        .chain(state.changes.untracked.iter())
    {
        out.push((f.path.clone(), false));
    }
    out
}

/// Dispatches line-level stage/unstage for the current selection.
fn stage_selected_lines(
    state: &mut AppState,
    generation: Generation,
    from_staged: bool,
) -> Vec<Command> {
    let Some(target) = state.diff.target.clone() else {
        set_error(state, "差分が選択されていません".into());
        return Vec::new();
    };
    if state.diff.selected_lines.is_empty() {
        set_error(state, "ステージする行が選択されていません".into());
        return Vec::new();
    }
    let lines = std::mem::take(&mut state.diff.selected_lines);
    issue(
        state,
        vec![Command::StageLines {
            path: target.path,
            from_staged,
            lines,
            generation,
        }],
    )
}

fn old_lines_from_diff(content: &DiffContent) -> Vec<u32> {
    let mut lines = Vec::new();
    for hunk in content.hunks.iter() {
        for line in &hunk.lines {
            if let Some(n) = line.old_line {
                lines.push(n);
            }
        }
    }
    lines.sort_unstable();
    lines.dedup();
    lines
}

/// First phase: up to 8 lines (current + nearby); rest deferred (issue #22).
fn split_blame_phases(lines: &[u32]) -> (Vec<u32>, Vec<u32>) {
    const FIRST: usize = 8;
    if lines.len() <= FIRST {
        return (lines.to_vec(), Vec::new());
    }
    (lines[..FIRST].to_vec(), lines[FIRST..].to_vec())
}

fn apply_blame_origins(
    content: &mut DiffContent,
    origins: &std::collections::HashMap<u32, CommitSummary>,
) {
    let hunks: Vec<_> = content
        .hunks
        .iter()
        .map(|h| {
            let mut hunk = h.clone();
            for line in &mut hunk.lines {
                if line.change_origin.is_none() {
                    if let Some(n) = line.old_line {
                        if let Some(c) = origins.get(&n) {
                            line.change_origin = Some(c.clone());
                        }
                    }
                }
            }
            hunk
        })
        .collect();
    content.hunks = hunks.into();
}

/// Validates and dispatches a commit.
fn reduce_commit(state: &mut AppState, generation: Generation) -> Vec<Command> {
    let message = state.ui.commit_message.trim().to_string();
    if message.is_empty() {
        set_error(state, "コミットメッセージが空です".to_string());
        return Vec::new();
    }
    if state.changes.staged.is_empty() {
        set_error(state, "ステージされた変更がありません".to_string());
        return Vec::new();
    }
    issue(
        state,
        vec![Command::Commit {
            message,
            generation,
        }],
    )
}

/// Instant Worktree: branch → default path → create (issue #21).
fn reduce_instant_worktree(
    state: &mut AppState,
    generation: Generation,
    branch: &str,
) -> Vec<Command> {
    let branch = branch.trim().to_string();
    if branch.is_empty() {
        set_error(
            state,
            "Instant Worktree にはブランチの選択が必要です".into(),
        );
        return Vec::new();
    }
    state.selection.branch = Some(branch.clone());
    if state
        .branch
        .branches
        .iter()
        .any(|b| b.name == branch && b.is_remote)
    {
        set_error(
            state,
            "リモート追跡ブランチからは Instant Worktree できません".into(),
        );
        return Vec::new();
    }
    let Some(repo) = state.repository.path.clone() else {
        set_error(state, "リポジトリが開かれていません".into());
        return Vec::new();
    };
    let path = crate::git::worktree::default_worktree_path(&repo, &branch);
    issue(
        state,
        vec![Command::CreateWorktree {
            branch,
            path,
            generation,
        }],
    )
}

/// Lazily loads data for a view the first time it is shown.
fn lazy_load_view(state: &mut AppState, view: View) -> Vec<Command> {
    let gen = state.generation;
    match view {
        View::History if state.history.commits.is_empty() && !state.history.loading => {
            state.history.loading = true;
            issue(
                state,
                vec![Command::LoadHistoryPage {
                    filter: state.history.filter.clone(),
                    offset: 0,
                    generation: gen,
                }],
            )
        }
        View::Branches if !state.branch.loaded => {
            issue(state, vec![Command::LoadBranches { generation: gen }])
        }
        View::Worktrees if !state.worktree.loaded => {
            issue(state, vec![Command::LoadWorktrees { generation: gen }])
        }
        View::Stashes if !state.stash.loaded => {
            issue(state, vec![Command::LoadStashes { generation: gen }])
        }
        _ => Vec::new(),
    }
}

fn reload_after_stash_mutation(state: &mut AppState, gen: Generation) -> Vec<Command> {
    issue(
        state,
        vec![
            Command::LoadStatus { generation: gen },
            Command::LoadStashes { generation: gen },
        ],
    )
}

fn stage_one(state: &mut AppState, path: &Path) {
    let mut unstaged = state.changes.unstaged.to_vec();
    let mut untracked = state.changes.untracked.to_vec();
    let mut staged = state.changes.staged.to_vec();
    if let Some(pos) = unstaged.iter().position(|f| f.path == path) {
        staged.push(unstaged.remove(pos));
    } else if let Some(pos) = untracked.iter().position(|f| f.path == path) {
        let mut change = untracked.remove(pos);
        change.kind = ChangeKind::Added;
        staged.push(change);
    } else {
        return;
    }
    state.changes.unstaged = unstaged.into();
    state.changes.untracked = untracked.into();
    state.changes.staged = staged.into();
}

fn unstage_one(state: &mut AppState, path: &Path) {
    let mut staged = state.changes.staged.to_vec();
    let Some(pos) = staged.iter().position(|f| f.path == path) else {
        return;
    };
    let change = staged.remove(pos);
    let mut unstaged = state.changes.unstaged.to_vec();
    unstaged.push(FileChange::new(change.path, ChangeKind::Modified));
    state.changes.staged = staged.into();
    state.changes.unstaged = unstaged.into();
}

fn stage_all(state: &mut AppState) {
    let mut staged = state.changes.staged.to_vec();
    staged.extend(state.changes.unstaged.iter().cloned());
    staged.extend(
        state
            .changes
            .untracked
            .iter()
            .map(|f| FileChange::new(f.path.clone(), ChangeKind::Added)),
    );
    state.changes.staged = staged.into();
    state.changes.unstaged = Vec::new().into();
    state.changes.untracked = Vec::new().into();
}

fn unstage_all(state: &mut AppState) {
    let mut unstaged = state.changes.unstaged.to_vec();
    unstaged.extend(
        state
            .changes
            .staged
            .iter()
            .map(|f| FileChange::new(f.path.clone(), ChangeKind::Modified)),
    );
    state.changes.unstaged = unstaged.into();
    state.changes.staged = Vec::new().into();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::message::RepositoryData;
    use crate::app::model::{CommitSummary, DiffContent, DiffHunk, HeadInfo, Oid};
    use crate::app::state::HistoryFilter;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    fn staged_state() -> AppState {
        let mut state = AppState::new();
        state.changes.unstaged = vec![FileChange::new("a.rs", ChangeKind::Modified)].into();
        state.changes.untracked = vec![FileChange::new("b.rs", ChangeKind::Untracked)].into();
        state
    }

    #[test]
    fn open_repository_bumps_generation_and_dispatches() {
        let mut state = AppState::new();
        let cmds = reduce(&mut state, UiEvent::OpenRepository(PathBuf::from("/repo")));
        assert_eq!(state.generation, Generation(1));
        assert_eq!(state.repository.status, RepositoryStatus::Opening);
        assert_eq!(state.repository.recent, vec![PathBuf::from("/repo")]);
        assert_eq!(state.background.inflight, 1);
        assert!(matches!(cmds.as_slice(), [Command::OpenRepository { .. }]));
    }

    #[test]
    fn repository_opened_fresh_triggers_loads_but_stale_is_dropped() {
        let mut state = AppState::new();
        reduce(&mut state, UiEvent::OpenRepository(PathBuf::from("/repo")));
        let gen = state.generation;

        // A stale message (older generation) is discarded.
        let dropped = apply(
            &mut state,
            AppMessage::RepositoryOpened {
                generation: Generation(0),
                result: Ok(RepositoryData {
                    head: HeadInfo::default(),
                }),
            },
        );
        assert!(dropped.is_empty());
        assert_ne!(state.repository.status, RepositoryStatus::Ready);

        // The fresh message is applied and fans out to load commands.
        let cmds = apply(
            &mut state,
            AppMessage::RepositoryOpened {
                generation: gen,
                result: Ok(RepositoryData {
                    head: HeadInfo {
                        branch: Some("main".into()),
                        oid: None,
                        detached: false,
                    },
                }),
            },
        );
        assert!(state.is_ready());
        assert_eq!(state.repository.head.branch.as_deref(), Some("main"));
        assert_eq!(cmds.len(), 4);
    }

    #[test]
    fn select_file_requests_diff_and_only_matching_diff_is_applied() {
        let mut state = AppState::new();
        reduce(
            &mut state,
            UiEvent::SelectFile {
                path: PathBuf::from("a.rs"),
                staged: false,
            },
        );
        assert!(state.diff.content.is_loading());
        let gen = state.generation;

        // Diff for a different file is ignored.
        let other = DiffContent {
            target: DiffTarget {
                path: "other.rs".into(),
                staged: false,
            },
            hunks: Arc::from([] as [DiffHunk; 0]),
            notice: None,
        };
        apply(
            &mut state,
            AppMessage::DiffLoaded {
                generation: gen,
                result: Ok(other),
            },
        );
        assert!(state.diff.content.is_loading());

        // Diff for the selected file is applied.
        let content = DiffContent {
            target: DiffTarget {
                path: "a.rs".into(),
                staged: false,
            },
            hunks: Arc::from([DiffHunk {
                header: "@@".into(),
                lines: vec![],
            }]),
            notice: None,
        };
        apply(
            &mut state,
            AppMessage::DiffLoaded {
                generation: gen,
                result: Ok(content),
            },
        );
        assert!(state.diff.content.ready().is_some());
    }

    #[test]
    fn optimistic_stage_moves_file_and_error_rolls_back_via_reload() {
        let mut state = staged_state();
        reduce(&mut state, UiEvent::StageFile(PathBuf::from("a.rs")));
        assert_eq!(state.changes.staged.len(), 1);
        assert_eq!(state.changes.unstaged.len(), 0);
        let gen = state.generation;

        let cmds = apply(
            &mut state,
            AppMessage::StageCompleted {
                generation: gen,
                path: PathBuf::from("a.rs"),
                result: Err("boom".into()),
            },
        );
        assert!(state.ui.error_banner.is_some());
        assert!(matches!(cmds.as_slice(), [Command::LoadStatus { .. }]));
    }

    #[test]
    fn stage_all_moves_everything() {
        let mut state = staged_state();
        reduce(&mut state, UiEvent::StageAll);
        assert_eq!(state.changes.staged.len(), 2);
        assert_eq!(state.changes.unstaged.len(), 0);
        assert_eq!(state.changes.untracked.len(), 0);
    }

    #[test]
    fn commit_validation_requires_message_and_staged_changes() {
        let mut state = AppState::new();
        // Empty message -> error, no command.
        let cmds = reduce(&mut state, UiEvent::Commit);
        assert!(cmds.is_empty());
        assert!(state.ui.error_banner.is_some());

        // Message but nothing staged -> error.
        reduce(&mut state, UiEvent::SetCommitMessage("feat: x".into()));
        let cmds = reduce(&mut state, UiEvent::Commit);
        assert!(cmds.is_empty());

        // Message + staged -> Commit command; message preserved until success.
        state.changes.staged = vec![FileChange::new("a.rs", ChangeKind::Modified)].into();
        let cmds = reduce(&mut state, UiEvent::Commit);
        assert!(matches!(cmds.as_slice(), [Command::Commit { .. }]));
        assert_eq!(state.ui.commit_message, "feat: x");
    }

    #[test]
    fn commit_failure_preserves_message_success_clears_and_bumps() {
        let mut state = AppState::new();
        state.ui.commit_message = "feat: x".into();
        let gen = state.generation;

        apply(
            &mut state,
            AppMessage::CommitCompleted {
                generation: gen,
                result: Err("locked".into()),
            },
        );
        assert_eq!(state.ui.commit_message, "feat: x");

        let cmds = apply(
            &mut state,
            AppMessage::CommitCompleted {
                generation: gen,
                result: Ok(Oid("abc".into())),
            },
        );
        assert_eq!(state.ui.commit_message, "");
        assert_eq!(state.generation, gen.next());
        assert_eq!(cmds.len(), 3);
    }

    #[test]
    fn checkout_completion_bumps_generation_and_invalidates_diff() {
        let mut state = AppState::new();
        state.diff.content = Loadable::Ready(DiffContent {
            target: DiffTarget {
                path: "a.rs".into(),
                staged: false,
            },
            hunks: Arc::from([] as [DiffHunk; 0]),
            notice: None,
        });
        let gen = state.generation;
        let cmds = apply(
            &mut state,
            AppMessage::CheckoutCompleted {
                generation: gen,
                result: Ok(HeadInfo {
                    branch: Some("dev".into()),
                    oid: None,
                    detached: false,
                }),
            },
        );
        assert_eq!(state.generation, gen.next());
        assert_eq!(state.diff.content, Loadable::Idle);
        assert_eq!(state.repository.head.branch.as_deref(), Some("dev"));
        assert_eq!(cmds.len(), 3);
    }

    #[test]
    fn instant_worktree_requires_branch_and_dispatches_create() {
        let mut state = AppState::new();
        let cmds = reduce(
            &mut state,
            UiEvent::InstantWorktree {
                branch: String::new(),
            },
        );
        assert!(cmds.is_empty());
        assert!(state.ui.error_banner.is_some());

        state.ui.error_banner = None;
        state.repository.path = Some(PathBuf::from("/tmp/app"));
        let cmds = reduce(
            &mut state,
            UiEvent::InstantWorktree {
                branch: "feature/auth".into(),
            },
        );
        assert_eq!(state.selection.branch.as_deref(), Some("feature/auth"));
        assert!(matches!(
            cmds.as_slice(),
            [Command::CreateWorktree { branch, path, .. }]
                if branch == "feature/auth"
                    && path.ends_with("app-worktrees/feature-auth")
        ));
    }

    #[test]
    fn delete_branch_requires_confirmation_then_dispatches() {
        let mut state = AppState::new();
        let cmds = reduce(&mut state, UiEvent::RequestDeleteBranch("feature".into()));
        assert!(cmds.is_empty());
        assert_eq!(state.ui.confirm_delete_branch.as_deref(), Some("feature"));

        reduce(&mut state, UiEvent::CancelDeleteBranch);
        assert!(state.ui.confirm_delete_branch.is_none());

        reduce(&mut state, UiEvent::RequestDeleteBranch("feature".into()));
        let cmds = reduce(&mut state, UiEvent::ConfirmDeleteBranch);
        assert!(state.ui.confirm_delete_branch.is_none());
        assert!(matches!(
            cmds.as_slice(),
            [Command::DeleteBranch { name, .. }] if name == "feature"
        ));
    }

    #[test]
    fn history_paging_appends_and_stops_when_short_page() {
        let mut state = AppState::new();
        state.history.has_more = true;
        let gen = state.generation;
        let full: Vec<CommitSummary> = (0..HISTORY_PAGE)
            .map(|i| CommitSummary {
                oid: Oid(format!("{i}")),
                summary: String::new(),
                author: String::new(),
                timestamp: 0,
            })
            .collect();
        apply(
            &mut state,
            AppMessage::HistoryPageLoaded {
                generation: gen,
                filter: HistoryFilter::All,
                offset: 0,
                result: Ok(full),
            },
        );
        assert_eq!(state.history.commits.len(), HISTORY_PAGE);
        assert!(state.history.has_more);

        // A short next page appends and clears has_more.
        state.history.loading = true;
        apply(
            &mut state,
            AppMessage::HistoryPageLoaded {
                generation: gen,
                filter: HistoryFilter::All,
                offset: HISTORY_PAGE,
                result: Ok(vec![CommitSummary {
                    oid: Oid("x".into()),
                    summary: String::new(),
                    author: String::new(),
                    timestamp: 0,
                }]),
            },
        );
        assert_eq!(state.history.commits.len(), HISTORY_PAGE + 1);
        assert!(!state.history.has_more);
    }

    #[test]
    fn inflight_tracks_dispatch_and_completion() {
        let mut state = AppState::new();
        reduce(&mut state, UiEvent::OpenRepository(PathBuf::from("/repo")));
        assert_eq!(state.background.inflight, 1);
        let gen = state.generation;
        apply(
            &mut state,
            AppMessage::RepositoryOpened {
                generation: gen,
                result: Ok(RepositoryData {
                    head: HeadInfo::default(),
                }),
            },
        );
        // -1 for the completed open, +4 for the fan-out loads.
        assert_eq!(state.background.inflight, 4);
    }

    #[test]
    fn toggle_diff_line_and_stage_selected_emits_stage_lines() {
        let mut state = AppState::new();
        reduce(
            &mut state,
            UiEvent::SelectFile {
                path: PathBuf::from("f.txt"),
                staged: false,
            },
        );
        reduce(&mut state, UiEvent::ToggleDiffLine(3));
        reduce(&mut state, UiEvent::ToggleDiffLine(5));
        assert_eq!(state.diff.selected_lines, vec![3, 5]);
        let cmds = reduce(&mut state, UiEvent::StageSelectedLines);
        assert!(state.diff.selected_lines.is_empty());
        assert!(matches!(
            cmds.as_slice(),
            [Command::StageLines {
                from_staged: false,
                lines,
                ..
            }] if lines == &[3, 5]
        ));
    }

    #[test]
    fn navigate_changes_walks_staged_then_unstaged() {
        use crate::app::model::{ChangeKind, FileChange};
        use std::sync::Arc;

        let mut state = AppState::new();
        state.changes.staged = Arc::from([FileChange::new("a.txt", ChangeKind::Modified)]);
        state.changes.unstaged = Arc::from([FileChange::new("b.txt", ChangeKind::Modified)]);
        state.changes.loaded = true;

        let cmds = reduce(&mut state, UiEvent::NavigateChanges { delta: 1 });
        assert_eq!(
            state
                .diff
                .target
                .as_ref()
                .map(|t| (t.path.as_path(), t.staged)),
            Some((Path::new("a.txt"), true))
        );
        assert!(matches!(cmds.as_slice(), [Command::LoadDiff { .. }]));

        let _ = reduce(&mut state, UiEvent::NavigateChanges { delta: 1 });
        assert_eq!(
            state
                .diff
                .target
                .as_ref()
                .map(|t| (t.path.as_path(), t.staged)),
            Some((Path::new("b.txt"), false))
        );
    }

    #[test]
    fn show_file_history_switches_view_and_loads() {
        let mut state = AppState::new();
        let cmds = reduce(
            &mut state,
            UiEvent::ShowFileHistory {
                path: PathBuf::from("src/main.rs"),
            },
        );
        assert_eq!(state.navigation.active_view, View::History);
        assert!(matches!(
            state.history.filter,
            HistoryFilter::File { ref path } if path == Path::new("src/main.rs")
        ));
        assert!(state.history.loading);
        assert!(matches!(
            cmds.as_slice(),
            [Command::LoadHistoryPage {
                filter: HistoryFilter::File { .. },
                offset: 0,
                ..
            }]
        ));
    }

    #[test]
    fn show_line_history_switches_view_and_loads() {
        let mut state = AppState::new();
        let cmds = reduce(
            &mut state,
            UiEvent::ShowLineHistory {
                path: PathBuf::from("lib.rs"),
                line: 42,
            },
        );
        assert_eq!(state.navigation.active_view, View::History);
        assert!(matches!(
            state.history.filter,
            HistoryFilter::Line { ref path, line: 42 } if path == Path::new("lib.rs")
        ));
        assert!(matches!(
            cmds.as_slice(),
            [Command::LoadHistoryPage {
                filter: HistoryFilter::Line { .. },
                offset: 0,
                ..
            }]
        ));
    }

    #[test]
    fn select_stash_loads_diff() {
        let mut state = AppState::new();
        let cmds = reduce(&mut state, UiEvent::SelectStash(0));
        assert_eq!(state.stash.selected, Some(0));
        assert!(state.stash.diff.is_loading());
        assert!(matches!(
            cmds.as_slice(),
            [Command::LoadStashDiff { index: 0, .. }]
        ));
    }

    #[test]
    fn select_commit_loads_detail_and_opens_context() {
        let mut state = AppState::new();
        let oid = Oid("abc123".into());
        let cmds = reduce(&mut state, UiEvent::SelectCommit(oid.clone()));
        assert_eq!(state.selection.commit, Some(oid));
        assert!(state.navigation.context_panel_open);
        assert!(state.context.commit.is_loading());
        assert!(matches!(
            cmds.as_slice(),
            [Command::LoadCommitDetail { .. }]
        ));
    }
}
