# Performance Benchmarks (Issue #34) Implementation Plan

> **For agentic workers:** Inline execution.

**Goal:** Measurable open/status/diff/blame/history timings, configurable worker count, memory sample, CI soft regression warnings.

## Design

1. `src/perf/` — thresholds, timing helpers, synthetic scale fixture, suite runner, worker sweep.
2. `GITBOLT_WORKERS` env (default 4, clamp 1–16) wired into `AppSession`.
3. `gitbolt-bench` binary: JSON report; `--check-thresholds` / `--warn-only` for CI.
4. CI step after tests: release bench with `--warn-only` (prints `::warning::`, does not fail the job).

**Proceeding with Inline Execution.**
