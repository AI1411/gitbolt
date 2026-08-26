# Line-Unit Stage (Issue #28) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the user select individual diff lines and stage/unstage only those lines via a partial patch applied with `git apply --cached`, with optimistic UI and rollback.

**Architecture:** Extend `GitService` with `diff` (CLI unified) and `stage_lines` / `unstage_lines` (CLI `git apply --cached` / `--reverse`). A pure `git::patch` module builds a minimal unified patch from selected `+`/`-` lines. App layer adds `UiEvent::ToggleDiffLine` / `StageSelectedLines` / `UnstageSelectedLines`, `Command::StageLines`, and Diff UI selection. Prerequisite Phase-1 pieces (file diff load + file stage via CLI) are included only as far as Line Stage needs them.

**Tech Stack:** Rust, gix (open/status), `GitCli` (`diff` / `apply --cached`), Dioxus Diff view

## Global Constraints

- Line/hunk stage uses **CLI fallback** (`docs/design/09-git-backend.md`)
- Optimistic UI + reload status on failure (existing StageCompleted pattern)
- Selected lines only change index; remaining hunks stay unstaged
- `cargo fmt` / `clippy -D warnings` / `cargo test` green

---

## File Structure

| Path | Responsibility |
|------|----------------|
| `src/git/patch.rs` | Pure partial-patch builder from unified diff + selected indices |
| `src/git/diff.rs` | Parse unified diff → `DiffText` / app `DiffContent` |
| `src/git/stage.rs` | `stage_file` / `stage_lines` via CLI |
| `src/git/service.rs` | Trait methods `diff`, `stage`, `stage_lines`, `unstage_lines` |
| `src/app/{event,command,message,state,reducer,executor}.rs` | Line selection + StageLines command |
| `src/ui/diff.rs` | Clickable lines, Stage selected button |

---

### Task 1: Partial patch builder (TDD)

**Files:** Create `src/git/patch.rs`, export from `git/mod.rs`

**Interfaces:**
```rust
pub fn build_partial_patch(
    unified_diff: &str,
    selected: &[usize], // 0-based indices into diff body lines (non-header)
    reverse: bool,      // true for unstage
) -> Result<String, GitError>;
```

- [ ] **Step 1: Write failing tests** — select one `+` line from a 2-line addition; assert patch contains only that line and valid `@@` header
- [ ] **Step 2: Implement minimal builder** (single-hunk files first)
- [ ] **Step 3: Commit** `feat(git): partial unified patch builder for line stage (#28)`

---

### Task 2: GitService diff + stage_lines

**Files:** `src/git/diff.rs`, `src/git/stage.rs`, `service.rs`, `cli.rs` helpers

**Interfaces:**
```rust
fn diff(&self, path: &Path, staged: bool) -> Result<DiffText, GitError>;
fn stage(&self, path: &Path) -> Result<(), GitError>; // git add -- path
fn stage_lines(&self, path: &Path, staged: bool, selected: &[usize]) -> Result<(), GitError>;
```

`stage_lines`: load full unified diff → `build_partial_patch` → `git apply --cached` (or `--cached --reverse` when unstaging from index diff).

- [ ] **Step 1: Integration test** on TempRepo — modify file with 3 lines, stage only line 2, assert status still has unstaged remainder
- [ ] **Step 2: Implement**
- [ ] **Step 3: Commit** `feat(git): CLI diff and line stage/unstage (#28)`

---

### Task 3: App wiring + Diff UI

**Files:** event/command/message/state/reducer/executor/session, `ui/diff.rs`

**Interfaces:**
- `DiffState.selected_lines: BTreeSet<usize>`
- `UiEvent::ToggleDiffLine(usize)`, `StageSelectedLines`, `UnstageSelectedLines`
- `Command::StageLines { path, staged, lines, generation }`
- Optimistic: clear selection on dispatch; on error reload status + diff

- [ ] **Step 1: Reducer tests** for toggle + command emit
- [ ] **Step 2: Executor + Diff view UI**
- [ ] **Step 3: Full test + clippy**
- [ ] **Step 4: Commit / PR / merge / close #28**

## Self-Review

1. Spec: line select ✓, partial patch ✓, optimistic/rollback ✓, only selected staged ✓
2. No TBD placeholders
3. Types align with existing `DiffLine` / `DiffHunk` / `DiffContent`

**Proceeding with Inline Execution** (user requested sequential implement → PR → merge for all Phase 2).
