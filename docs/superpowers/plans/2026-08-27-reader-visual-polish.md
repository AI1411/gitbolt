# Reader visual polish (Diff / History / Context)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Diff and History feel like reading surfaces — dual line numbers, sticky hunk headers, stronger +/- bands, a real commit graph, and clearer Commit Detail hierarchy — on top of existing `--gb-*` tokens.

**Architecture:** Extend `DiffLine` with `new_line`; polish renderers in `diff.rs` / `history.rs` / `context.rs`; add sticky/gutter helpers in `theme.rs` GLOBAL_CSS. No new views or events.

**Tech Stack:** Dioxus desktop, existing CSS variables, unified-diff parser

## Global Constraints

- Keep dark slate + blue accent; no purple/glow/card dashboards.
- Preserve Keyboard First and existing `SelectCommit` / stage-line behavior.
- Heatmap gutter remains data-driven hex (not forced into tokens).

---

### Task 1: Dual line numbers in the parser

**Files:**
- Modify: `src/app/model.rs`
- Modify: `src/app/diff_parse.rs`
- Modify: `src/app/heatmap.rs` (test fixtures)
- Test: `src/app/diff_parse.rs`

**Interfaces:**
- Produces: `DiffLine { new_line: Option<u32>, … }` alongside `old_line`

- [ ] Add `new_line` and parse `@@ -old +new @@`
- [ ] Update tests
- [ ] Commit

### Task 2: Diff reader chrome

**Files:**
- Modify: `src/ui/diff.rs`
- Modify: `src/ui/theme.rs`

- [ ] Sticky hunk headers (`.gb-hunk-header`)
- [ ] Dual gutters `old | new`
- [ ] Left accent bar on +/- rows
- [ ] Commit

### Task 3: History graph + Context hierarchy

**Files:**
- Modify: `src/ui/history.rs`
- Modify: `src/ui/context.rs`

- [ ] SVG/CSS graph column replacing ASCII `●│`
- [ ] Commit Detail: actions toolbar vs meta separation
- [ ] Commit file diffs use same +/- bands + line numbers
- [ ] Commit

### Task 4: Verify

- [ ] `cargo fmt`, `clippy -D warnings`, `cargo test -p gitbolt --lib`
- [ ] Manual History + Changes diff screenshot/video
- [ ] PR + merge
