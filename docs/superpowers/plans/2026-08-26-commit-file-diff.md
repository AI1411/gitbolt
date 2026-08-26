# Commit File Diff Implementation Plan

> **For agentic workers:** Inline execution.

**Goal:** From History commit detail, click a changed file to view that file's unified diff for the commit in the History content pane.

## Design

1. `git show <oid> -- <path>` → `commit_detail::show_file_diff`
2. `ContextState` gains `selected_file` + `file_diff: Loadable<DiffContent>`
3. `UiEvent::SelectCommitFile` / `ClearCommitFileDiff` + Command/Message/executor
4. Clickable file rows in Context Panel; History view shows read-only diff preview (stash-style)
5. Selecting another commit clears the file diff

**Proceeding with Inline Execution.**
