//! Quick Open candidates: files, branches, commits (issue #26).

use crate::app::event::UiEvent;
use crate::app::model::Oid;
use crate::app::state::AppState;

/// Kind of Quick Open hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickOpenKind {
    File,
    Branch,
    Commit,
}

/// A searchable Quick Open entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickOpenItem {
    pub kind: QuickOpenKind,
    pub label: String,
    pub detail: String,
    pub event: UiEvent,
}

/// Builds Quick Open items from current app state.
#[must_use]
pub fn collect_items(state: &AppState) -> Vec<QuickOpenItem> {
    let mut items = Vec::new();

    for f in state.changes.staged.iter() {
        items.push(QuickOpenItem {
            kind: QuickOpenKind::File,
            label: f.path.display().to_string(),
            detail: "staged".into(),
            event: UiEvent::SelectFile {
                path: f.path.clone(),
                staged: true,
            },
        });
    }
    for f in state
        .changes
        .unstaged
        .iter()
        .chain(state.changes.untracked.iter())
        .chain(state.changes.conflicted.iter())
    {
        items.push(QuickOpenItem {
            kind: QuickOpenKind::File,
            label: f.path.display().to_string(),
            detail: "working tree".into(),
            event: UiEvent::SelectFile {
                path: f.path.clone(),
                staged: false,
            },
        });
    }

    for b in state.branch.branches.iter().filter(|b| !b.is_remote) {
        items.push(QuickOpenItem {
            kind: QuickOpenKind::Branch,
            label: b.name.clone(),
            detail: "branch".into(),
            event: UiEvent::SelectBranch(b.name.clone()),
        });
    }

    for c in state.history.commits.iter().take(50) {
        let short = if c.oid.0.len() > 7 {
            c.oid.0[..7].to_string()
        } else {
            c.oid.0.clone()
        };
        items.push(QuickOpenItem {
            kind: QuickOpenKind::Commit,
            label: format!("{short} {summary}", summary = c.summary),
            detail: c.author.clone(),
            event: UiEvent::SelectCommit(Oid(c.oid.0.clone())),
        });
    }

    items
}

/// Filters items by case-insensitive substring on label/detail.
#[must_use]
pub fn filter_items(items: &[QuickOpenItem], query: &str) -> Vec<QuickOpenItem> {
    let needle = query.trim().to_lowercase();
    items
        .iter()
        .filter(|item| {
            if needle.is_empty() {
                return true;
            }
            item.label.to_lowercase().contains(&needle)
                || item.detail.to_lowercase().contains(&needle)
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::model::{ChangeKind, FileChange};
    use std::sync::Arc;

    #[test]
    fn filter_files_by_path() {
        let mut state = AppState::new();
        state.changes.unstaged = Arc::from([FileChange::new("src/main.rs", ChangeKind::Modified)]);
        let items = collect_items(&state);
        let hits = filter_items(&items, "main");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].kind, QuickOpenKind::File);
    }
}
