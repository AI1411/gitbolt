# Divergence View (Issue #29) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Visualize commits unique to each of two branches since their merge-base, with entry from the Branches / Branch Health UI.

**Architecture:** CLI `git merge-base` + `git log A..B` via `GitCli`. App holds `DivergenceState`; Branches view selects the comparison tip vs HEAD (or upstream) and loads both sides. Branch list is filled so Health-style ahead/behind badges can link into Divergence.

**Tech Stack:** Rust, GitCli, Dioxus Branches UI

## Global Constraints

- CLI for merge-base / rev-list (reliable across repos)
- No modals — Divergence renders in Content pane under Branches
- Progressive disclosure while loading
- fmt / clippy / test green

---

## File Structure

| Path | Responsibility |
|------|----------------|
| `src/git/branch.rs` | `list_branches`, `merge_base`, `commits_not_in` |
| `src/git/service.rs` | Trait methods |
| `src/app/state.rs` | `DivergenceState` |
| `src/app/{event,command,message,reducer,executor}.rs` | LoadDivergence |
| `src/ui/branches.rs` | Branch list + Divergence dual-column UI |
| `src/ui/divergence.rs` | Presentational dual list |

---

### Task 1: Git merge-base + side commits (TDD)

```rust
pub fn merge_base(repo: &Path, a: &str, b: &str) -> Result<String, GitError>;
pub fn commits_not_in(repo: &Path, tip: &str, base: &str, limit: usize) -> Result<Vec<CommitInfo>, GitError>;
pub fn list_branches(repo: &Path) -> Result<Vec<BranchRef>, GitError>;
```

- [ ] TempRepo with main + feature branch diverged → assert merge-base and one commit each side
- [ ] Wire `GitService` + `GixService` + executor `LoadBranches`

### Task 2: App + UI

- [ ] `UiEvent::ShowDivergence { left, right }`, `Command::LoadDivergence`
- [ ] Branches UI: click branch → show ↑/↓ + “Divergence” dual columns
- [ ] PR / merge / close #29

**Proceeding with Inline Execution.**
