# Squashed Branch Detection (Issue #79) Implementation Plan

> **For agentic workers:** Inline execution.

**Goal:** Detect squash-merged local branches via `git cherry` patch equivalence; fold into cleanup candidates; allow session exclude for false positives.

**Architecture:** `git cherry <base> <branch>` — if every commit line is `-` (equivalent patch already in base) and the branch is not an ancestor of base, mark as squashed. Extend `CleanupReason` with `Squashed`. Session exclude list on `BranchState`.

Closes #79
