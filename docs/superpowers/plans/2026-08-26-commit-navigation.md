# Commit Navigation (Issue #32) Implementation Plan

> **For agentic workers:** Inline execution.

**Goal:** Browser-like Back / Forward through commits visited in Context Panel / History; Esc acts as Back for commit trail; UI buttons + keybinds.

## Design

1. Extend `NavigationState` with `commit_back: Vec<Oid>` and `commit_forward: Vec<Oid>`.
2. On user `SelectCommit`, push previous selection onto `commit_back` and clear `commit_forward`.
3. Add `UiEvent::NavigateCommit { delta }` (−1 Back / +1 Forward) that moves between stacks without re-recording.
4. Esc: after overlays/confirms/errors/search, if a commit is selected → Back if stack non-empty, else clear selection; then existing view `back_stack`.
5. Keys: ⌘[ / ⌘] (Ctrl on non-mac); buttons on commit detail; Command Palette entries.

**Proceeding with Inline Execution.**
