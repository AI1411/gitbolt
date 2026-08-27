//! Command Palette actions and filtering (issue #26).

use crate::app::event::UiEvent;
use crate::app::model::View;

/// A searchable command for the palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteCommand {
    pub id: &'static str,
    pub label: &'static str,
    pub keys: &'static str,
    pub event: PaletteAction,
}

/// Actions the palette can trigger (mapped to [`UiEvent`] at run time).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteAction {
    GoChanges,
    GoHistory,
    GoBranches,
    InstantWorktree,
    Fetch,
    Pull,
    Push,
    StageAll,
    UnstageAll,
    FocusCommit,
    Commit,
    ToggleContext,
    StashSave,
    ToggleQuickOpen,
    CommitBack,
    CommitForward,
    ToggleHeatmap,
    /// Insert a conventional commit type prefix (issue #77).
    ConventionalType(&'static str),
    /// Return to the welcome / open screen (issue #87).
    SwitchRepository,
}

/// Built-in palette entries.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn all_commands() -> Vec<PaletteCommand> {
    vec![
        PaletteCommand {
            id: "go.changes",
            label: "Go to Changes",
            keys: "",
            event: PaletteAction::GoChanges,
        },
        PaletteCommand {
            id: "go.history",
            label: "Go to History",
            keys: "H",
            event: PaletteAction::GoHistory,
        },
        PaletteCommand {
            id: "go.branches",
            label: "Go to Branches",
            keys: "B",
            event: PaletteAction::GoBranches,
        },
        PaletteCommand {
            id: "worktree.instant",
            label: "Instant Worktree",
            keys: "W",
            event: PaletteAction::InstantWorktree,
        },
        PaletteCommand {
            id: "remote.fetch",
            label: "Fetch",
            keys: "F",
            event: PaletteAction::Fetch,
        },
        PaletteCommand {
            id: "remote.pull",
            label: "Pull",
            keys: "",
            event: PaletteAction::Pull,
        },
        PaletteCommand {
            id: "remote.push",
            label: "Push",
            keys: "",
            event: PaletteAction::Push,
        },
        PaletteCommand {
            id: "stage.all",
            label: "Stage All",
            keys: "",
            event: PaletteAction::StageAll,
        },
        PaletteCommand {
            id: "unstage.all",
            label: "Unstage All",
            keys: "",
            event: PaletteAction::UnstageAll,
        },
        PaletteCommand {
            id: "commit.focus",
            label: "Focus Commit Message",
            keys: "C",
            event: PaletteAction::FocusCommit,
        },
        PaletteCommand {
            id: "commit.run",
            label: "Commit",
            keys: "⌘Enter",
            event: PaletteAction::Commit,
        },
        PaletteCommand {
            id: "panel.toggle",
            label: "Toggle Context Panel",
            keys: "⌘I",
            event: PaletteAction::ToggleContext,
        },
        PaletteCommand {
            id: "stash.save",
            label: "Stash Changes",
            keys: "",
            event: PaletteAction::StashSave,
        },
        PaletteCommand {
            id: "quick.open",
            label: "Quick Open",
            keys: "⌘P",
            event: PaletteAction::ToggleQuickOpen,
        },
        PaletteCommand {
            id: "commit.back",
            label: "Commit Back",
            keys: "⌘[",
            event: PaletteAction::CommitBack,
        },
        PaletteCommand {
            id: "commit.forward",
            label: "Commit Forward",
            keys: "⌘]",
            event: PaletteAction::CommitForward,
        },
        PaletteCommand {
            id: "diff.heatmap",
            label: "Toggle Blame Heatmap",
            keys: "",
            event: PaletteAction::ToggleHeatmap,
        },
        PaletteCommand {
            id: "commit.type.feat",
            label: "Commit type: feat",
            keys: "",
            event: PaletteAction::ConventionalType("feat"),
        },
        PaletteCommand {
            id: "commit.type.fix",
            label: "Commit type: fix",
            keys: "",
            event: PaletteAction::ConventionalType("fix"),
        },
        PaletteCommand {
            id: "commit.type.docs",
            label: "Commit type: docs",
            keys: "",
            event: PaletteAction::ConventionalType("docs"),
        },
        PaletteCommand {
            id: "commit.type.chore",
            label: "Commit type: chore",
            keys: "",
            event: PaletteAction::ConventionalType("chore"),
        },
        PaletteCommand {
            id: "commit.type.refactor",
            label: "Commit type: refactor",
            keys: "",
            event: PaletteAction::ConventionalType("refactor"),
        },
        PaletteCommand {
            id: "commit.type.test",
            label: "Commit type: test",
            keys: "",
            event: PaletteAction::ConventionalType("test"),
        },
        PaletteCommand {
            id: "repo.switch",
            label: "Switch Repository",
            keys: "",
            event: PaletteAction::SwitchRepository,
        },
    ]
}

/// Case-insensitive substring filter on label/id/keys.
#[must_use]
pub fn filter_commands(query: &str) -> Vec<PaletteCommand> {
    let needle = query.trim().to_lowercase();
    all_commands()
        .into_iter()
        .filter(|c| {
            if needle.is_empty() {
                return true;
            }
            c.label.to_lowercase().contains(&needle)
                || c.id.to_lowercase().contains(&needle)
                || c.keys.to_lowercase().contains(&needle)
        })
        .collect()
}

/// Maps a palette action to a [`UiEvent`].
#[must_use]
pub fn action_to_event(action: PaletteAction) -> UiEvent {
    match action {
        PaletteAction::GoChanges => UiEvent::SelectView(View::Changes),
        PaletteAction::GoHistory => UiEvent::SelectView(View::History),
        PaletteAction::GoBranches => UiEvent::SelectView(View::Branches),
        PaletteAction::InstantWorktree => UiEvent::InstantWorktree {
            branch: String::new(),
        },
        PaletteAction::Fetch => UiEvent::Fetch,
        PaletteAction::Pull => UiEvent::Pull,
        PaletteAction::Push => UiEvent::Push,
        PaletteAction::StageAll => UiEvent::StageAll,
        PaletteAction::UnstageAll => UiEvent::UnstageAll,
        PaletteAction::FocusCommit => UiEvent::FocusCommitInput,
        PaletteAction::Commit => UiEvent::Commit,
        PaletteAction::ToggleContext => UiEvent::ToggleContextPanel,
        PaletteAction::StashSave => UiEvent::StashSave { message: None },
        PaletteAction::ToggleQuickOpen => UiEvent::OpenQuickOpen,
        PaletteAction::CommitBack => UiEvent::NavigateCommit { delta: -1 },
        PaletteAction::CommitForward => UiEvent::NavigateCommit { delta: 1 },
        PaletteAction::ToggleHeatmap => UiEvent::ToggleHeatmap,
        PaletteAction::ConventionalType(ty) => {
            // Placeholder — confirm_overlay applies against current message.
            UiEvent::SetCommitMessage(format!("{ty}: "))
        }
        PaletteAction::SwitchRepository => UiEvent::CloseRepository,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_matches_label_substring() {
        let hits = filter_commands("fetch");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "remote.fetch");
    }

    #[test]
    fn empty_query_returns_all() {
        assert_eq!(filter_commands("").len(), all_commands().len());
    }
}
