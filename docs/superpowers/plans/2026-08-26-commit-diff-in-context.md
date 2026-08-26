# Commit file diff in Context panel

> **For agentic workers:** Move commit file diffs from History content to the right Context pane.

**Goal:** When a changed file is clicked in Commit Detail, show its unified diff in the right Context panel—not above the History list.

**Architecture:** `CommitFileDiffPreview` renders from `state.context.file_diff` / `selected_file` inside `CommitDetailBody`. History content stays a commit list only.

**Tech Stack:** Dioxus UI, existing `SelectCommitFile` / `ClearCommitFileDiff` events

## Global Constraints

- No new events or loaders; reuse `context.file_diff`.
- Left History pane must not show commit file diffs.
- Escape / Close still clears via `ClearCommitFileDiff`.

---

### Task 1: Move preview to Context

**Files:**
- Modify: `src/ui/history.rs`
- Modify: `src/ui/context.rs`

- [ ] Remove `CommitFileDiffPreview` from `HistoryView`
- [ ] Render the same preview under Changed files in `CommitDetailBody`
- [ ] `cargo fmt`, `clippy -D warnings`, `cargo test -p gitbolt --lib`
