# Blame Heatmap (Issue #33) Implementation Plan

> **For agentic workers:** Inline execution.

**Goal:** Per-line heat gutter from blame (recency + OID frequency), shown in Diff view, toggleable.

## Design

1. `app/heatmap.rs`: from `DiffContent` blame origins, score each line `0.0..=1.0` (`0.65` recency over ~1y + `0.35` relative OID frequency); map to cool→warm color.
2. `DiffState.heatmap_enabled` (default `false`) + `UiEvent::ToggleHeatmap`.
3. Diff toolbar toggle; 4px gutter on unified/split lines when enabled and origin exists.
4. Command Palette entry; unit tests for intensity/color.

**Proceeding with Inline Execution.**
