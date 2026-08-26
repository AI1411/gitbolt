# Repository Pulse (Issue #11) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Always-visible one-line repository summary in the shell header: branch, ahead/behind, change counts, staged count, worktree count — with clickable segments that navigate to the matching view. Updates whenever `AppState` changes (stage / status / branches / worktrees).

**Architecture:** Pure `pulse::summary(&AppState) -> PulseSnapshot` derived from existing Changes / Branch / Worktree / Head state. Replace the static shell header with `PulseHeader`. Add minimal `git worktree list` so worktree count is real (prerequisite slice of #20).

**Tech Stack:** Rust, existing AppState, Dioxus shell header, GitCli for worktree list

## Global Constraints

- No new background polling beyond existing status/branch loads
- Detached HEAD and missing upstream have explicit copy
- fmt / clippy / test green

---

### Task 1: Pulse snapshot (TDD)

```rust
pub struct PulseSnapshot {
  pub branch_label: String, // "feature/auth" | "detached HEAD" | oid short
  pub ahead: Option<u32>,
  pub behind: Option<u32>,
  pub changes: u32,  // unstaged + untracked + conflicted (+ staged? design says "7 changes · 4 staged")
  pub staged: u32,
  pub worktrees: u32,
  pub has_upstream: bool,
}
```

- [ ] Unit tests for detached / no upstream / counts
- [ ] Implement `summary`

### Task 2: Worktree list (minimal)

- [ ] `git::worktree::list` via `git worktree list --porcelain`
- [ ] Wire `LoadWorktrees` in executor
- [ ] Reload on repository open (existing lazy load)

### Task 3: PulseHeader UI

- [ ] Clickable segments → `SelectView(Branches|Changes|Worktrees)`
- [ ] Replace shell header brand line
- [ ] PR / merge / close #11

**Proceeding with Inline Execution.**
