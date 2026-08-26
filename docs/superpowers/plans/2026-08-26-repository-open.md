# Repository Open (Issue #9) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire every Repository Open path (dialog, Recent, Drag&Drop, CLI) so the app reaches Ready with minimal HEAD metadata within the first paint budget, then loads Changes/Branches/History in the background.

**Architecture:** Keep the existing `UiEvent → reduce → Command → worker → AppMessage → apply` loop. Add a Command executor that runs `GixService::open` + `head` off the UI thread via `TaskRunner`, persist Recent to disk, and expose an Open screen + CLI entry that all dispatch `UiEvent::OpenRepository`.

**Tech Stack:** Rust, Dioxus 0.7 Desktop, gix, TaskRunner, rfd (folder dialog), serde_json (Recent persistence)

## Global Constraints

- Dioxus 0.7 desktop
- MVP OS: macOS / Apple Silicon arm64 (Linux CI OK for unit tests)
- UI never calls Git directly; only via `Command` / `GitService`
- Ready after Minimal Metadata (HEAD / branch); Changes/Branch/History start in background
- Japanese user-facing error strings via `GitError::user_message`
- `cargo fmt`, `clippy -D warnings`, `cargo test` must pass

---

## File Structure

| Path | Responsibility |
|------|----------------|
| `src/app/recent.rs` | Load/save recent repository paths (JSON, max 10) |
| `src/app/executor.rs` | `execute(Command, &Path) -> AppMessage` for Open (and stubs for later) |
| `src/app/session.rs` | Owns `AppState` + `TaskRunner` dispatch/poll helpers used by UI |
| `src/ui/open.rs` | Welcome / Open screen (Open button, Recent list, drop zone, inline error) |
| `src/ui/mod.rs` | Root `App` wires session, CLI path, Open vs Ready shell |
| `src/main.rs` | Parse `gitbolt [path]`, launch Desktop |
| `Cargo.toml` | Add `serde`, `serde_json`, `dirs`, `rfd` |

---

### Task 1: Recent repositories persistence

**Files:**
- Create: `src/app/recent.rs`
- Modify: `src/app/mod.rs`
- Test: unit tests inside `src/app/recent.rs`

**Interfaces:**
- Consumes: `std::path::PathBuf`
- Produces:
  - `pub fn recent_store_path() -> PathBuf`
  - `pub fn load_recent() -> Vec<PathBuf>`
  - `pub fn save_recent(paths: &[PathBuf]) -> std::io::Result<()>`
  - `pub fn push_recent(paths: &mut Vec<PathBuf>, path: PathBuf)` (dedupe, front, truncate 10)

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn push_recent_dedupes_and_caps_at_ten() {
    let mut recent = Vec::new();
    for i in 0..12 {
        push_recent(&mut recent, PathBuf::from(format!("/r{i}")));
    }
    assert_eq!(recent.len(), 10);
    assert_eq!(recent[0], PathBuf::from("/r11"));
    push_recent(&mut recent, PathBuf::from("/r5"));
    assert_eq!(recent[0], PathBuf::from("/r5"));
    assert_eq!(recent.iter().filter(|p| *p == Path::new("/r5")).count(), 1);
}

#[test]
fn save_and_load_roundtrip(tmp: PathBuf) {
    // override store path via env GITBOLT_RECENT_PATH in test
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitbolt app::recent -- --nocapture`
Expected: FAIL (module missing)

- [ ] **Step 3: Write minimal implementation**

```rust
use std::path::{Path, PathBuf};

pub fn push_recent(recent: &mut Vec<PathBuf>, path: PathBuf) {
    recent.retain(|p| p != &path);
    recent.insert(0, path);
    recent.truncate(10);
}

pub fn recent_store_path() -> PathBuf {
    if let Ok(p) = std::env::var("GITBOLT_RECENT_PATH") {
        return PathBuf::from(p);
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gitbolt")
        .join("recent.json")
}

pub fn load_recent() -> Vec<PathBuf> {
    let path = recent_store_path();
    let Ok(bytes) = std::fs::read(&path) else {
        return Vec::new();
    };
    serde_json::from_slice::<Vec<PathBuf>>(&bytes).unwrap_or_default()
}

pub fn save_recent(paths: &[PathBuf]) -> std::io::Result<()> {
    let path = recent_store_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec_pretty(paths)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitbolt app::recent -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/app/recent.rs src/app/mod.rs
git commit -m "feat(app): persist recent repositories (#9)"
```

---

### Task 2: Command executor for OpenRepository

**Files:**
- Create: `src/app/executor.rs`
- Modify: `src/app/mod.rs`
- Test: `src/app/executor.rs` tests

**Interfaces:**
- Consumes: `Command`, open repo `Path` (from command), `GixService`
- Produces: `pub fn execute(cmd: &Command, repo_path: Option<&Path>) -> AppMessage`

Open path:

```rust
Command::OpenRepository { path, generation } => {
    match GixService::open(path).and_then(|svc| svc.head().map(|h| (svc, h))) {
        Ok((_svc, head)) => AppMessage::RepositoryOpened {
            generation: *generation,
            result: Ok(RepositoryData {
                head: HeadInfo {
                    branch: head.branch,
                    oid: head.oid.map(Oid),
                    detached: head.detached,
                },
            }),
        },
        Err(err) => AppMessage::RepositoryOpened {
            generation: *generation,
            result: Err(err.user_message()),
        },
    }
}
```

Unsupported commands return a failed message with `GitError::unsupported(...).user_message()` until later issues.

- [ ] **Step 1: Write failing tests** for open success / not-a-repo
- [ ] **Step 2: Run → FAIL**
- [ ] **Step 3: Implement `execute`**
- [ ] **Step 4: Run → PASS**
- [ ] **Step 5: Commit** `feat(app): execute OpenRepository via GixService (#9)`

---

### Task 3: Session dispatch (TaskRunner bridge)

**Files:**
- Create: `src/app/session.rs`
- Modify: `src/app/mod.rs`

**Interfaces:**
- Consumes: `AppState`, `Command`, `TaskRunner<AppMessage>`, `execute`
- Produces:
  - `pub struct AppSession { pub state: AppState, runner: TaskRunner<AppMessage>, rx: Receiver<Outcome<AppMessage>>, repo_path: Option<PathBuf> }`
  - `AppSession::new() -> Self`
  - `AppSession::dispatch_event(&mut self, UiEvent) -> ()` — reduce, persist recent on open, submit commands
  - `AppSession::poll(&mut self) -> bool` — apply pending outcomes, return whether state changed
  - Priority: OpenRepository = P0

```rust
fn submit_commands(&mut self, commands: Vec<Command>) {
    for cmd in commands {
        let gen = cmd.generation();
        self.runner.set_generation(self.state.generation);
        let path = match &cmd {
            Command::OpenRepository { path, .. } => Some(path.clone()),
            _ => self.repo_path.clone(),
        };
        self.runner.submit(Priority::P0, gen, move || {
            execute(&cmd, path.as_deref())
        });
    }
}
```

On successful open / open event, update `repo_path` and call `save_recent`.

- [ ] **Step 1: Unit test** open via session against TempRepo ends Ready with head branch
- [ ] **Step 2–4: TDD cycle**
- [ ] **Step 5: Commit** `feat(app): session bridge for open commands (#9)`

---

### Task 4: Open UI + CLI launch

**Files:**
- Create: `src/ui/open.rs`
- Modify: `src/ui/mod.rs`, `src/main.rs`, `Cargo.toml` (`rfd`)

**Interfaces:**
- Consumes: `Signal<AppSession>` or callbacks `on_open: EventHandler<PathBuf>`
- Produces: `OpenScreen` component; root `App` shows Open when `NotOpened|Error|Opening`, otherwise a minimal Ready placeholder (Pulse/layout arrive in #10–#11)

CLI:

```rust
fn main() {
    let initial = std::env::args().nth(1).map(PathBuf::from);
    dioxus::LaunchBuilder::desktop()
        .with_context(initial)
        .launch(App);
}
```

Open screen:
- "Open Repository" button → `rfd::FileDialog::new().pick_folder()`
- Recent list buttons
- Drop zone (`ondragover` prevent_default, `ondrop` → `file.path()`)
- Inline error from `RepositoryStatus::Error` / `ui.error_banner`

- [ ] **Step 1: Manual/unit** — CLI helper `parse_cli_path(args) -> Option<PathBuf>` tested
- [ ] **Step 2: Implement UI + wire session poll via `use_future` / interval**
- [ ] **Step 3: `cargo test` + `cargo clippy -D warnings`**
- [ ] **Step 4: Commit** `feat(ui): repository open screen, drag-drop, CLI (#9)`

---

### Task 5: Wire reducer recent persistence + close issue

**Files:**
- Modify: `src/app/reducer.rs` — optionally call `push_recent` helper from `recent` module (DRY with `merge_recent`)
- Ensure Opening state shows non-blocking UI (no blank wipe of recent)

- [ ] **Step 1: Replace local `merge_recent` with `crate::app::recent::push_recent`**
- [ ] **Step 2: Full test suite green**
- [ ] **Step 3: Commit** `refactor(app): share recent list helper (#9)`
- [ ] **Step 4: Open PR, merge, close issue #9**

---

## Self-Review

1. **Spec coverage:** Dialog Open ✓, Recent persistence ✓, Drag&Drop ✓, CLI ✓, Minimal Metadata Ready ✓, inline error ✓
2. **Placeholders:** none — executor stubs only return explicit unsupported messages for non-open commands
3. **Types:** `HeadInfo` / `Oid` / `RepositoryData` match existing `message.rs`

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-08-26-repository-open.md`.

**Proceeding with Inline Execution** (user requested sequential implement → PR → merge for all Phase 1 issues).
