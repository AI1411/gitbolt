# Remote Commit / File / Line Link (Issue #75) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve `origin` remote URLs into GitHub/GitLab/Bitbucket web hosts and expose Open/Copy links for commit, file, and line from Context Panel / History.

**Architecture:** Pure URL parsing + link builders in `src/git/remote_link.rs` (no network). On repository open / status load, read `git remote get-url origin` and store a parsed `RemoteWeb` on `RepositoryState`. Context Panel and file/line history surfaces emit `CopyText` / `OpenUrl`; `OpenUrl` is handled in `ui/mod.rs` via the system opener (same pattern as clipboard for `CopyText`).

**Tech Stack:** Rust, existing `GitCli`, Dioxus Desktop, `open` crate for cross-platform URL open

## Global Constraints

- No GitHub/GitLab API calls — URL inference only
- Unsupported / local-path remotes → hide Open/Copy remote actions (no error spam)
- Prefer `origin`; if missing, no remote links
- `cargo fmt` / `clippy -D warnings` / `cargo test` green
- Commit messages reference `Closes #75` on the merge PR

---

## File Structure

| Path | Responsibility |
|------|----------------|
| `src/git/remote_link.rs` | Parse remote URL → `RemoteWeb`; build commit/file/line URLs |
| `src/git/mod.rs` | `pub mod remote_link` |
| `src/git/remote.rs` | `origin_url(repo) -> Result<String, GitError>` via `git remote get-url origin` |
| `src/git/service.rs` / `repository.rs` | Optional `origin_url` on trait / impl |
| `src/app/model.rs` or `state.rs` | `RemoteWeb` mirror / `RepositoryState.origin_web` |
| `src/app/executor.rs` | Populate `origin_web` when loading status |
| `src/app/event.rs` | `OpenUrl(String)` |
| `src/app/reducer.rs` | No-op / ignore for `OpenUrl` (side effect in UI) |
| `src/ui/mod.rs` | Handle `OpenUrl` with `open::that` |
| `src/ui/context.rs` | Open / Copy remote links on commit + file context |
| `Cargo.toml` | Add `open` dependency |

---

### Task 1: Parse remote URL + build links (TDD)

**Files:**
- Create: `src/git/remote_link.rs`
- Modify: `src/git/mod.rs`
- Test: inline in `remote_link.rs`

**Interfaces:**
- Produces:
  - `pub enum WebHostKind { GitHub, GitLab, Bitbucket, Unknown }`
  - `pub struct RemoteWeb { pub kind: WebHostKind, pub web_base: String }`
  - `pub fn parse_remote_url(url: &str) -> Option<RemoteWeb>`
  - `pub fn commit_url(web: &RemoteWeb, oid: &str) -> String`
  - `pub fn file_url(web: &RemoteWeb, rev: &str, path: &str) -> String`
  - `pub fn line_url(web: &RemoteWeb, rev: &str, path: &str, line: u32) -> String`

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_https_github() {
        let w = parse_remote_url("https://github.com/ai1411/gitbolt.git").unwrap();
        assert_eq!(w.kind, WebHostKind::GitHub);
        assert_eq!(w.web_base, "https://github.com/ai1411/gitbolt");
        assert_eq!(
            commit_url(&w, "abc123"),
            "https://github.com/ai1411/gitbolt/commit/abc123"
        );
        assert_eq!(
            file_url(&w, "abc123", "src/main.rs"),
            "https://github.com/ai1411/gitbolt/blob/abc123/src/main.rs"
        );
        assert_eq!(
            line_url(&w, "abc123", "src/main.rs", 10),
            "https://github.com/ai1411/gitbolt/blob/abc123/src/main.rs#L10"
        );
    }

    #[test]
    fn parses_ssh_github() {
        let w = parse_remote_url("git@github.com:ai1411/gitbolt.git").unwrap();
        assert_eq!(w.web_base, "https://github.com/ai1411/gitbolt");
    }

    #[test]
    fn parses_gitlab_https() {
        let w = parse_remote_url("https://gitlab.com/group/proj.git").unwrap();
        assert_eq!(w.kind, WebHostKind::GitLab);
        assert_eq!(
            commit_url(&w, "deadbeef"),
            "https://gitlab.com/group/proj/-/commit/deadbeef"
        );
        assert_eq!(
            line_url(&w, "main", "a.rs", 3),
            "https://gitlab.com/group/proj/-/blob/main/a.rs#L3"
        );
    }

    #[test]
    fn parses_bitbucket() {
        let w = parse_remote_url("https://bitbucket.org/team/repo.git").unwrap();
        assert_eq!(w.kind, WebHostKind::Bitbucket);
        assert_eq!(
            commit_url(&w, "abc"),
            "https://bitbucket.org/team/repo/commits/abc"
        );
        assert_eq!(
            line_url(&w, "abc", "f.rs", 7),
            "https://bitbucket.org/team/repo/src/abc/f.rs#lines-7"
        );
    }

    #[test]
    fn rejects_local_path() {
        assert!(parse_remote_url("/tmp/bare.git").is_none());
        assert!(parse_remote_url("file:///tmp/bare.git").is_none());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitbolt --lib remote_link -- --nocapture`
Expected: FAIL (module missing)

- [ ] **Step 3: Write minimal implementation**

Implement `parse_remote_url` supporting:
- `https://host/owner/repo(.git)`
- `http://…`
- `git@host:owner/repo(.git)`
- `ssh://git@host/owner/repo(.git)`
- nested GitLab groups (`group/sub/proj`)
- strip trailing `.git` and `/`
- host detection: `github.com` → GitHub, `gitlab.com` or host contains `gitlab` → GitLab, `bitbucket.org` → Bitbucket, else Unknown with generic GitHub-like paths (`/commit`, `/blob`)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitbolt --lib remote_link -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/git/remote_link.rs src/git/mod.rs
git commit -m "feat(git): parse remote URLs into web commit/file/line links"
```

---

### Task 2: Read origin URL into RepositoryState

**Files:**
- Modify: `src/git/remote.rs` — add `origin_url`
- Modify: `src/git/service.rs` — default / trait method optional OR call remote module from executor
- Modify: `src/app/state.rs` — `origin_web: Option<RemoteWeb>` (re-export or duplicate thin type in app)
- Modify: `src/app/executor.rs` — set on status load
- Modify: `src/app/reducer.rs` / `session.rs` — clear on close; set from StatusData

**Interfaces:**
- Consumes: `parse_remote_url`
- Produces: `StatusData.origin_web: Option<RemoteWeb>` populated each status refresh

Prefer storing `crate::git::remote_link::RemoteWeb` on `RepositoryState` (Clone, PartialEq, Eq).

```rust
// remote.rs
pub fn origin_url(repo: &Path) -> Result<String, GitError> {
    let cli = GitCli::new(repo)?;
    cli.run(&["remote", "get-url", "origin"])
}
```

In `load_status`, after opening service:

```rust
let origin_web = remote::origin_url(path)
    .ok()
    .and_then(|u| remote_link::parse_remote_url(&u));
```

Wire through `StatusData` → reducer → `state.repository.origin_web`.

- [ ] **Step 1: Unit test** TempRepo with `remote add origin https://github.com/o/r.git` → load_status yields parsed web_base
- [ ] **Step 2: Implement**
- [ ] **Step 3: Commit**

```bash
git commit -m "feat(app): load origin remote web host into RepositoryState"
```

---

### Task 3: OpenUrl event + Context Panel actions

**Files:**
- Modify: `Cargo.toml` — `open = "5"`
- Modify: `src/app/event.rs` — `OpenUrl(String)`
- Modify: `src/app/reducer.rs` — empty match arm (side effect in UI)
- Modify: `src/ui/mod.rs` — `open::that(url)` on `OpenUrl`
- Modify: `src/ui/context.rs` — remote link row on commit detail; file context with HEAD/file/line when available

**UI behavior:**
- Commit detail: buttons `Open commit` / `Copy link` when `origin_web` is `Some`
- File context (Changes): if file selected and HEAD oid known → file link at HEAD; if HistoryFilter::Line active → line link
- History filter context: same

Use existing `CopyButton`; add `LinkActions { open_label, copy_label, url }`.

```rust
#[component]
fn RemoteLinkActions(url: String, on_event: EventHandler<UiEvent>) -> Element {
    rsx! {
        div {
            style: "display:flex;gap:0.35rem;flex-wrap:wrap;",
            button {
                style: "border:1px solid #334155;background:transparent;color:#9fb0c7;\
                        border-radius:4px;padding:0.15rem 0.45rem;cursor:pointer;font-size:0.68rem;",
                onclick: move |_| on_event.call(UiEvent::OpenUrl(url.clone())),
                "Open"
            }
            CopyButton { label: "Copy link".into(), text: url.clone(), on_event: on_event }
        }
    }
}
```

- [ ] **Step 1: Implement OpenUrl dispatch**
- [ ] **Step 2: Wire Context Panel**
- [ ] **Step 3: `cargo test` / `clippy -D warnings` / `fmt`**
- [ ] **Step 4: Commit**

```bash
git commit -m "feat(ui): Open/Copy remote commit and file links in Context Panel"
```

---

### Task 4: PR

- Push branch `cursor/issue-75-remote-links-1efd`
- Open draft PR with body `Closes #75`
- Merge after CI (or when green locally if CI slow)
- Confirm issue closed

---

## Self-Review

1. Spec coverage: remote URL resolve ✅; Commit/File/Line links ✅; Context/History Open/Copy ✅
2. No placeholders
3. Types: `RemoteWeb` / `WebHostKind` consistent across tasks
