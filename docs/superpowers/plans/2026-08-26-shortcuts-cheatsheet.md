# Shortcuts consistency + cheatsheet (Issue #84) Implementation Plan

> **For agentic workers:** Inline execution.

**Goal:** Consistent view keys (`1`–`5`, `B`/`H`/`W`), `?` cheatsheet, OS-aware mod labels, ignore shell keys while typing.

**Architecture:** Extend `Overlay` with `CheatSheet`. `platform::mod_key_label()`. Shell: digits switch views; `H`/`W` always navigate (file history → `Shift+H`, Instant Worktree stays on Branches button/palette). `UiEvent::SetTyping(bool)` from inputs.

Closes #84
