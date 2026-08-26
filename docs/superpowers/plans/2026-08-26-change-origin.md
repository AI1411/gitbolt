# Change Origin (Issue #31) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Show which HEAD commit last authored each deleted/context line in a working-tree (or staged) diff — the “Change Origin” of the edit — inside Diff view, with click → `SelectCommit` / context panel.

**Architecture:** CLI `git blame --line-porcelain HEAD -- path` → per-line `CommitInfo`. Diff parse tracks old-side line numbers from `@@` headers. `load_diff` joins blame onto `DiffLine.change_origin`. UI renders a compact origin chip; click emits `SelectCommit` and opens the context panel.

**Tech Stack:** Rust, `GitCli` blame, existing Diff / AppState / Dioxus Diff view

## Global Constraints

- Blame uses **CLI** (gix-blame deferred; Phase 1 #22)
- Origins only for lines with an old-side number (`-` / context); pure `+` lines stay empty
- Missing HEAD blob (new file) → no origins, no error
- `cargo fmt` / `clippy -D warnings` / `cargo test` green

---

## File Structure

| Path | Responsibility |
|------|----------------|
| `src/git/blame.rs` | `blame_at_head(repo, path) -> Vec<CommitInfo>` (1-based line index via vec) |
| `src/git/service.rs` / `repository.rs` | Wire `blame` |
| `src/app/model.rs` | `DiffLine.old_line`, `DiffLine.change_origin` |
| `src/app/diff_parse.rs` | Parse old line numbers from hunk headers |
| `src/app/executor.rs` | Enrich diff with blame origins |
| `src/ui/diff.rs` | Show / click origin |
| `src/app/reducer.rs` | `SelectCommit` opens context panel |

---

### Task 1: Blame at HEAD (TDD)

- [ ] Tests on TempRepo: edit existing line → blame line maps to prior commit
- [ ] Implement porcelain parser
- [ ] Commit

### Task 2: Diff parse + join

- [ ] Track `old_line` on `DiffLine`
- [ ] `attach_change_origins(content, blame)`
- [ ] `load_diff` calls blame for existing paths

### Task 3: Diff UI + context panel

- [ ] Render short oid · summary on lines with origin
- [ ] Click → `SelectCommit` + ensure `context_panel_open`
- [ ] Minimal context strip when commit selected (if no Context Panel yet)
