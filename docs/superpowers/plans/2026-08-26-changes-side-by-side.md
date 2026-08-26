# Changes side-by-side layout (Issue #83) Implementation Plan

> **For agentic workers:** Inline execution.

**Goal:** Place STAGED/UNSTAGED file list beside the diff (not stacked), and scroll per pane instead of nested `max-height:50vh`.

**Architecture:** `ContentBody` Changes uses a horizontal flex row: left `ChangesView` (~38%), right `DiffView` (flex 1). Both panes `overflow:auto;height:100%`. Remove Diff inner `max-height:50vh`.

Closes #83
