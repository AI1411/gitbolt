# Conventional Commit Completion (Issue #77) Implementation Plan

> **For agentic workers:** Inline execution.

**Goal:** Suggest Conventional Commits `type(scope): subject` while typing in CommitBox; scope from changed paths; palette inserts types.

**Architecture:** Pure helpers in `src/app/conventional.rs`. CommitBox shows type chips when the message has no type yet, and scope chips after `type(` or `type`. Palette entries insert `feat: ` / `fix: ` prefixes.

**Tech Stack:** Rust, existing CommitBox / Palette

## Global Constraints

- Types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert
- No network
- Don't force conventional format — suggestions only
- Tests + clippy green

---

### Task 1: Helpers + CommitBox chips + palette

- `suggest_types(prefix) -> Vec<&str>`
- `suggest_scopes(message, paths) -> Vec<String>` from first path segment / file stem
- `apply_type(message, ty) -> String`
- `apply_scope(message, scope) -> String`
- UI chips under textarea
- Palette: "Commit type: feat" etc. via `SetCommitMessage` prefix

Closes #77
