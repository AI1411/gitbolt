# Visual polish (Issues #107–#118) Implementation Plan

> **For agentic workers:** Inline execution. One issue per PR; merge then continue.

**Goal:** Raise visual scan speed without breaking Keyboard First / Fast First.

**Status:** Done. Child PRs #119–#129 landed; this tracker can close.

## Order

1. **#108 tokens** — CSS variables on the app root; replace UI hex with `var(--gb-*)`. Keep heatmap hex (data colors + tests).
2. **#109 Changes layout** — #83 already split panes; polish split surfaces / sticky section headers.
3. **#110 status colors** — `ChangeKind` → token colors.
4. **#111 typography** — font stacks + size scale on tokens.
5. **#112 Pulse hierarchy** — branch primary, divergence badges, actions grouped.
6. **#113 selection vs focus** — selected background vs focus outline.
7. **#114 Diff readability** — line numbers, hunk header, split gutter; heatmap restrained.
8. **#115 Nav** — left accent bar on active item.
9. **#116 resize handles** — hover highlight.
10. **#117 Open ↔ Shell tone** — shared surface depth.
11. **#118 light theme** — second token set + toggle (deferred until tokens exist).
12. **#107 tracker** — close when children are done.

Closes each issue from its PR body (`Closes #N`).
