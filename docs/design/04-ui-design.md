# 6. 画面設計

## 基本レイアウト

```
┌──────────────────────────────────────────────────────────────┐
│ GitBolt / main ↑2 ↓1 · 4 changes · 2 staged · 3 worktrees    │
├────────────┬────────────────────────────┬────────────────────┤
│ Navigation │ Content                    │ Context            │
│ Changes    │ Diff / History / Branch    │ Commit / Blame     │
│ History    │                            │ Branch Context     │
│ Branches   │                            │                    │
│ Worktrees  │                            │                    │
│ Stashes    │                            │                    │
└────────────┴────────────────────────────┴────────────────────┘
```

Single Window / 3ペイン。モーダル・タブ・画面遷移を極力減らす。

## Repository Pulse

```
GitBolt / feature/auth ↑3 ↓1 · 7 changes · 4 staged · 3 worktrees
```

## Changes

```
STAGED (2)
M main.rs
A cache.rs

UNSTAGED (4)
M status.rs
M ui.rs
? test.rs
D old.rs
```

SpaceでStage/Unstage。hunk選択時はHunk Stage。

## Smart Blame

| 状態 | 表示 |
|------|------|
| 通常 | `Akira · 3d` |
| Hover | `Akira · 3 days ago · a182df · Optimize repository loading` |
| Click | Context PanelにCommit Detail |

## Branch Health

```
● main            ✓
  feature/auth    ↑3
  fix/cache       ↓1
  feature/payment ↑2↓3
  experiment      ◌ 42d
```

## Worktree First

```
● main          ~/dev/app
  feature/auth  ~/dev/app-worktrees/auth

Branches without Worktree
  feature/payment
```

# 7. Keyboard First

| キー | 操作 |
|------|------|
| j / ↓ | Next |
| k / ↑ | Previous |
| Space | Stage / Unstage |
| [ / ] | Previous / Next Hunk |
| J / K | Next / Previous File |
| C | Commit |
| B | Branches |
| H | History |
| W | Worktrees |
| F | Fetch |
| / | Search |
| ⌘K | Command Palette |
| ⌘P | Quick Open |
| ⌘I | Context Panel |
| ⌘Enter | Commit / Confirm |
| Esc | Back / Close |
