# Outdated Branch Cleanup (Issue #78) Implementation Plan

> **For agentic workers:** Inline execution.

**Goal:** Detect merged-into-base and stale local branches; present a bulk cleanup confirm UI that excludes the current and protected branches.

**Architecture:** `git branch --merged <base>` plus existing Stale health. Pure `branch_cleanup` filters candidates. UI on Branches view opens a confirm panel; Confirm issues sequential `DeleteBranch` commands (`git branch -d`).

**Tech Stack:** GitCli, existing ConfirmPanel patterns, BranchInfo

## Global Constraints

- Protected: `main`, `master`, `develop`, `trunk`, `release`
- Never include current HEAD branch
- Use safe delete (`-d`), not force
- Confirm before any delete

Closes #78
