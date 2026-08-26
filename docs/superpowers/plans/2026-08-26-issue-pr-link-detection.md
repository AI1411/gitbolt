# Issue / PR Link Detection (Issue #76) Implementation Plan

> **For agentic workers:** Inline execution (sequential issue pipeline).

**Goal:** Detect `#123` / `GH-123` / full host URLs in commit messages and branch names, and show clickable Open/Copy links in the Context Panel (offline: URL only; no API title fetch).

**Architecture:** Pure regex detectors in `src/app/issue_link.rs` produce `IssueRef` values. Resolve each ref against `RepositoryState.origin_web` into absolute URLs. Context Panel lists refs under commit detail and branch context.

**Tech Stack:** Rust regex (std or `regex` crate), existing `RemoteWeb` / `OpenUrl` / `CopyText`

## Global Constraints

- Offline-first: no `gh`/API title enrichment in this issue (optional task deferred)
- Unsupported remotes → show raw `#N` text without broken links
- Reuse #75 Open/Copy actions
- `cargo fmt` / `clippy -D warnings` / `cargo test` green

---

## File Structure

| Path | Responsibility |
|------|----------------|
| `src/app/issue_link.rs` | Detect + resolve issue/PR refs |
| `src/app/mod.rs` | `pub mod issue_link` |
| `src/ui/context.rs` | Render detected links |
| `Cargo.toml` | Add `regex` if needed |

### Task 1: Detector (TDD)

**Interfaces:**
- `pub struct IssueRef { pub kind: IssueKind, pub number: u32, pub raw: String }`
- `pub enum IssueKind { Issue, PullRequest }` — treat `#N` and `GH-N` as Issue; `/pull/N` or `/issues/N` URLs set kind
- `pub fn detect_issue_refs(text: &str) -> Vec<IssueRef>`
- `pub fn resolve_issue_url(web: &RemoteWeb, r: &IssueRef) -> Option<String>`

Patterns:
- `#123` (word-boundary; not part of oid)
- `GH-123` / `gh-123`
- `https://github.com/o/r/issues/123` / `.../pull/123`
- GitLab `.../-/issues/123` / `.../-/merge_requests/123`

GitHub URL: `{base}/issues/{n}` (issues and PRs share numbering on GitHub — use `/issues/` for `#N`; use `/pull/` when URL or `PR #N` / `!N` for GitLab MR)

- [ ] Tests for detect + resolve
- [ ] Implement
- [ ] Commit

### Task 2: Context Panel

- Commit detail: scan `summary` + `body`
- Branch context: scan branch name
- Render list of RemoteLinkActions (or compact link chips)

- [ ] Implement UI
- [ ] Tests green
- [ ] Commit + PR `Closes #76`
