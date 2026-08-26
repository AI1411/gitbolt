# Changes View (Issue #12) Implementation Plan

> **For agentic workers:** Inline execution.

**Goal:** Complete the Changes view: STAGED / UNSTAGED / CONFLICTED sections with status marks, selection that drives Diff, keyboard j/k navigation, and watcher-driven status refresh.

**Architecture:** Enhance `ChangesView` + `SelectFile { path, staged }`. Flat ordered list for keyboard focus. Wire `RepoWatcher` into `AppSession` so working-tree/HEAD events dispatch `LoadStatus` (and branch reload on HEAD).

**Tech Stack:** Existing gix status, Dioxus Changes UI, notify watcher

## Tasks

- [ ] CONFLICTED section + selection highlight + staged-aware SelectFile
- [ ] `NavigateChanges(i32)` for j/k / arrows
- [ ] AppSession starts/stops RepoWatcher; poll watch events → LoadStatus
- [ ] Tests for selection order / staged flag
- [ ] PR / merge / close #12

**Proceeding with Inline Execution.**
