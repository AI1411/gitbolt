# Deferred Features Re-evaluation (Issue #35)

MVP (Phase 1) and Phase 2 extras (#32–#34) are complete. This document records
the post-MVP decision for each item formerly listed under **保留** in
`docs/design/03-features.md`.

## Decision summary

| Feature | Decision | Priority | Follow-up |
|---------|----------|----------|-----------|
| Remote Commit / File / Line Link | **Adopt** | A | #75 |
| Issue / PR Link Detection | **Adopt** | A | #76 |
| Conventional Commit Completion | **Adopt** | B | #77 |
| Outdated Branch Cleanup | **Adopt** | B | #78 |
| Squashed Branch Detection | **Adopt** | C | #79 |
| Gitmoji | **Defer** | — | revisit on user demand |
| Author Avatar | **Defer** | — | revisit with privacy / offline story |

## Rationale

### Adopt — Remote Commit / File / Line Link (A) → #75
Sharing exact locations is a daily Git GUI need. Hosts can be inferred from
`remote.origin.url` without always calling an API. Fits Context Panel / History
copy actions already shipped in Phase 1.

### Adopt — Issue / PR Link Detection (A) → #76
Commit messages and branch names already carry `#123` references. Detection is
mostly local regex; title enrichment can be optional/`gh`-backed later.

### Adopt — Conventional Commit Completion (B) → #77
Low UI surface: enhance the existing CommitBox. Aligns with keyboard-first
workflows; no network required.

### Adopt — Outdated Branch Cleanup (B) → #78
Natural extension of Branch Health / Stale. Destructive — must reuse confirm UI
from issue #27.

### Adopt — Squashed Branch Detection (C) → #79
Harder heuristics (patch-id / tree equality). Schedule after Cleanup so the
candidate list has a home.

### Defer — Gitmoji
Emoji prefixes are niche and compete with Conventional Commits. Prefer not to
add picker chrome until users ask. Remains out of the default roadmap.

### Defer — Author Avatar
Needs Gravatar/GitHub (or similar) and a clear offline/privacy policy. Smart
Blame already surfaces author names; avatars are polish, not workflow-critical.

## Ordering suggestion

1. #75 Remote links  
2. #76 Issue / PR detection  
3. #77 Conventional commits  
4. #78 Outdated cleanup  
5. #79 Squash detection  

## Design doc updates

- `03-features.md`: move adopted items out of **保留** into a **Phase 3+ backlog**
  table; keep Gitmoji / Author Avatar under **保留 (再評価済み・見送り)**.
