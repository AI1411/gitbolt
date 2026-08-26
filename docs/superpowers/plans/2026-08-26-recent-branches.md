# Recent Branches / Last Commit / Tracking (Issue #30) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enrich the branch list with reflog-ordered Recent Branches, last-commit summary/time per branch, and upstream set/change; expose a lightweight Quick Open filter for branches.

**Architecture:** Extend `git::branch` with reflog recent tips, `%(objectname:short) %(contents:subject) %(committerdate:unix)` formats, and `git branch -u`. App maps into `BranchInfo.last_commit` + new `recent_branches` / `UiEvent::SetUpstream` / `QuickOpenBranches`.

**Tech Stack:** GitCli, existing Branches UI

## Global Constraints

- CLI for reflog / upstream (porcelain)
- Quick Open here is branch-only (full palette is #26)
- fmt / clippy / test green

---

### Task 1: Git helpers (TDD)
- `recent_branches(repo, limit) -> Vec<String>` from `git reflog`
- `branch_last_commit(repo, name) -> Option<CommitInfo>`
- `set_upstream(repo, branch, upstream) -> Result<()>`

### Task 2: App + UI
- Fill `BranchInfo.last_commit` in `load_branches`
- Recent section at top of Branches view
- Set upstream inline (prompt/input) + Quick Open filter (`/`)
- PR / merge / close #30

**Proceeding with Inline Execution.**
