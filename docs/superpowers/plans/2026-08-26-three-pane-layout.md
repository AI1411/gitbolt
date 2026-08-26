# Three-Pane Layout & Navigation (Issue #10) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Ready placeholder with a Single Window 3-pane shell (Navigation / Content / Context) driven by `NavigationState`, with resizable panes and ⌘I context toggle.

**Architecture:** Keep `App` as the session/open gate. When `RepositoryStatus::Ready`, render `Shell` which owns pane widths as local UI signals and dispatches `UiEvent::SelectView` / `ToggleContextPanel`. View modules stay thin placeholders that show progressive text (never a blank Loading wipe).

**Tech Stack:** Dioxus 0.7 Desktop, existing `AppState` / `View` / `UiEvent`

## Global Constraints

- No modals / tabs / route transitions — single window only
- Progressive Disclosure: keep prior content while background loads
- Navigation labels: Changes / History / Branches / Worktrees / Stashes
- ⌘I toggles context panel (`NavigationState.context_panel_open`)
- `cargo fmt` / `clippy -D warnings` / `cargo test` green

---

## File Structure

| Path | Responsibility |
|------|----------------|
| `src/ui/shell.rs` | 3-pane layout, resize handles, keyboard ⌘I |
| `src/ui/nav.rs` | Navigation list bound to `View` |
| `src/ui/context.rs` | Context panel chrome (placeholder detail) |
| `src/ui/changes.rs` etc. | Minimal Ready-state placeholders |
| `src/ui/mod.rs` | Wire Ready → `Shell` |
| `src/ui/layout_model.rs` | Pure helpers: `nav_label`, `content_title` (unit-tested) |

---

### Task 1: Layout helpers (TDD)

**Files:**
- Create: `src/ui/layout_model.rs`
- Modify: `src/ui/mod.rs`

**Interfaces:**
- `pub fn nav_items() -> &'static [(View, &'static str)]`
- `pub fn content_heading(view: View) -> &'static str`

- [ ] **Step 1: Failing tests** for labels covering all five views
- [ ] **Step 2: Implement**
- [ ] **Step 3: Commit** `feat(ui): navigation label helpers (#10)`

---

### Task 2: Shell + Nav + Context components

**Files:**
- Create: `src/ui/shell.rs`, `src/ui/nav.rs`, `src/ui/context.rs`
- Modify: view modules to export `#[component] fn XView(state: AppState) -> Element`
- Modify: `src/ui/mod.rs` — Ready renders `Shell`

**Interfaces:**
- `Shell { state, on_event: EventHandler<UiEvent> }`
- Resize: local `nav_width` / `context_width` signals; drag on 4px gutters
- Content switches on `state.navigation.active_view`
- Context pane omitted from DOM when `!context_panel_open` (or width 0)
- `onkeydown`: meta/ctrl + KeyI → `ToggleContextPanel`

- [ ] **Step 1: Implement components**
- [ ] **Step 2: Wire dispatch from App**
- [ ] **Step 3: `cargo test` + clippy**
- [ ] **Step 4: Commit** `feat(ui): single-window 3-pane shell and navigation (#10)`

---

### Task 3: PR / merge / close

- [ ] Push, open PR, wait CI, merge, close #10

## Self-Review

1. Spec: 3-pane ✓, nav items ✓, content switch ✓, ⌘I ✓, NavigationState ✓, progressive placeholders ✓
2. No placeholders / TBD
3. Uses existing `View` / `UiEvent` names

**Proceeding with Inline Execution.**
