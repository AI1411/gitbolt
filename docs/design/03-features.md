# 4. MVP機能

| 領域 | 機能 |
|------|------|
| Repository | Open / Recent / Drag&Drop / CLI起動 |
| Changes | status / staged / unstaged / untracked / conflict |
| Diff | Unified / Split / syntax highlight |
| Stage | file / all / hunk、lineはMVP+ |
| Commit | commit |
| History | commit list / graph / lazy load |
| Branch | list / create / checkout / delete |
| Remote | fetch / pull / push / auto fetch |
| Worktree | list / create / remove / open |
| Blame | Inline Blame / File History / Line History |
| Stash | stash / list / apply / pop / drop |

# 5. GitToolBox系機能

## 採用

| 機能 | 優先度 |
|------|--------|
| Inline Blame | S |
| File History | S |
| Ahead / Behind | S |
| Blame Details | A |
| File Blame | A |
| Line History | A |
| Divergence View | A |
| Auto Fetch | A |
| Recent Branches | A |
| Branch Last Commit | A |
| Branch Tracking | A |
| Copy Git Info | B |

## 保留 (再評価済み・見送り)

Phase 1/2 完了後の再評価結果は [deferred-reevaluation.md](deferred-reevaluation.md) を参照。

- Gitmoji — 需要が出るまで見送り (Conventional Commits を優先)
- Author Avatar — オフライン / プライバシー方針が固まるまで見送り

## Phase 3+ バックログ (旧・保留から採用)

| 機能 | 優先度 | Issue |
|------|--------|-------|
| Remote Commit / File / Line Link | A | #75 |
| Issue / PR Link Detection | A | #76 |
| Conventional Commit Completion | B | #77 |
| Outdated Branch Cleanup | B | #78 |
| Squashed Branch Detection | C | #79 |

## 不採用

- IDE固有連携
- 成功通知の常時表示
- 大量の設定画面

## GitBolt独自拡張

| 機能 | 優先度 | 概要 |
|------|--------|------|
| Smart Blame | S | minimal → hover → detail |
| Instant Commit Context | S | 画面遷移なしでCommit文脈表示 |
| Branch Health | S | Synced/Ahead/Behind/Diverged/Stale |
| Worktree First | S | Worktreeを第一級機能化 |
| Instant Worktree | S | branchから即生成 |
| Repository Pulse | S | repository状態を1行に集約 |
| Change Origin | A | Working Tree変更の元Commit |
| Commit Navigation | A | Back / Forward |
| Blame Heatmap | C | 変更頻度可視化 |
