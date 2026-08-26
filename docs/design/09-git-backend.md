# 9. Git バックエンド: gix / git CLI fallback 対応表 (issue #3)

設計方針 (`docs/design/02-tech-and-performance.md` 2 章) に基づき、MVP 機能
(`docs/design/03-features.md` 4 章) の各 Git 操作を **gix (gitoxide) で実装するか、
git CLI へ fallback するか**を確定する。実装は Git Service (issue #5) に集約し、UI 層から
Git を直接触らない。

> **前提バージョン**: gitoxide ≈ 0.56 系 / `gix-blame` 0.17 系 / `gix-stash` (2026-08 時点)。
> gitoxide は活発に開発中のため、実装 (#5 以降) の着手時に
> [`crate-status.md`](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md)
> で最新状況を再確認する。本表の「対応状況」はその時点の調査結果。

## gix 全体の対応状況 (2026-08 時点、上流公表)

| 機能 | gix | 備考 |
|------|-----|------|
| clone / fetch | ✅ | ネットワーク I/O・認証は本表で別途方針化 |
| **push** | ❌ **未対応** | CLI fallback 必須 |
| status | ✅ | `gix-status` / `gix-dir` |
| blob & tree diff | ✅ | `gix-diff` |
| merge (blob/tree/commit) | 🟡 実装済だが新しい | `gix-merge` |
| commit (hooks 含む) | ✅ | オブジェクト生成 + ref 更新 |
| blame | ✅ (plumbing) | `gix-blame`、incremental / commit-graph Bloom 対応 |
| worktree checkout / stream | ✅ | `gix-worktree-state` (書き出し), `gix-worktree-stream` |
| reset | ❌ 未対応 | CLI fallback |
| stash (push/pop/list) | 🟡 MVP | `gix-stash` (feature gated)、apply/drop は未提供 |
| index (`.git/index`) 読み書き | ✅ | staging の土台。ただし filter 適用等の porcelain は自前 |
| refs 読み書き | ✅ | branch の list/create/delete の土台 |
| config / ignore / attributes / pathspec / revspec | ✅ | — |

凡例: ✅ 安定して利用可 / 🟡 実験的・新しい・機能限定 / ❌ 未対応

---

## MVP 操作別 実装方針 (確定表)

| 領域 | 操作 | 方針 | gix 対応 | 根拠・補足 |
|------|------|------|----------|------------|
| Repository | open / discover | **gix** | ✅ | `gix::open` / `gix::discover`。起動時は最小メタデータのみ |
| Repository | recent / drag&drop / CLI 起動 | アプリ側 | — | Git 操作ではなくパス管理 |
| Changes | status (staged/unstaged/untracked/conflict) | **gix** | ✅ | `gix-status`。大量ファイルでも段階取得 |
| Diff | tree / blob diff (Unified/Split) | **gix** | ✅ | `gix-diff`。syntax highlight はアプリ側 |
| Diff | worktree の未追跡/巨大バイナリ | gix + 判定 | 🟡 | バイナリ判定・サイズ上限で表示抑制 |
| Stage | file / all | **gix** (index 書換) | ✅ | index に blob を追記し書き出す |
| Stage | **hunk 単位** | **CLI fallback** | 🟡 | `git apply --cached` 相当が確実。porcelain の filter/patch 整合を CLI に委譲 |
| Stage | line 単位 | CLI fallback (MVP+) | 🟡 | issue #28 (MVP+)。方針は hunk と同じ |
| Commit | commit (message / HEAD 即更新) | **gix** | ✅ | index→tree 書出し→commit オブジェクト→ref 更新。hook は gix 対応 |
| History | log / graph / lazy load | **gix** | ✅ | `gix::revision::walk` (+ commit-graph)。recent 100 段階ロード |
| Branch | list | **gix** | ✅ | `gix-ref` |
| Branch | create / delete | **gix** | ✅ | ref transaction。削除は preflight (未 merge 警告) |
| Branch | **checkout (switch)** | **CLI fallback** | 🟡 | worktree 書換自体は gix 可だが、ローカル変更の保護・部分更新等の porcelain 安全性を `git switch/checkout` に委譲 |
| Remote | fetch / auto fetch | **CLI fallback** (当面) | ✅(gixも可) | gix でも可能だが、**認証**を確実にするため MVP は CLI。将来 gix へ移行検討 |
| Remote | pull | **CLI fallback** | 🟡 | fetch + merge。merge は gix 新しめ、当面 `git pull` |
| Remote | **push** | **CLI fallback** 必須 | ❌ | gix 未対応 |
| Worktree | list | **gix** | ✅ | linked worktree 情報の読取 |
| Worktree | **create / remove / open** | **CLI fallback** | 🟡 | linked worktree の生成/削除は `git worktree add/remove` が確実 |
| Blame | Inline / Smart / File / Line | **gix** | ✅ | `gix-blame` incremental。可視行優先の段階ロード |
| Stash | list | **gix** (or CLI) | 🟡 | `gix-stash::list` |
| Stash | save(push) / pop | **gix** (feature gated) | 🟡 | `gix-stash` push/pop。untracked 取込は options |
| Stash | **apply / drop** | **CLI fallback** | ❌ | `gix-stash` MVP に未提供 → `git stash apply/drop` |

---

## 重点方針の詳細

### 1. push と認証 (最重要)

- **push は gix 未対応**のため CLI fallback 固定。
- fetch/pull も MVP では CLI fallback とし、**認証をユーザー環境に委譲**する:
  - HTTPS: git の credential helper (osxkeychain 等) をそのまま利用。
  - SSH: システムの `ssh` / ssh-agent / `~/.ssh/config` を利用。
  - GUI 側で認証情報を保持・実装しない (攻撃面とメンテを増やさない)。
  - 認証待ちで UI を block しない。パスワード/パスフレーズ要求は将来
    `GIT_ASKPASS` 経由でダイアログ化を検討 (MVP は端末非依存で失敗時にエラー表示)。
- 実装: `CliFallback` ラッパーで `git push`/`git fetch` を実行し、`--porcelain` /
  終了コード / stderr を解析して `GitError` に変換。

### 2. worktree

- 一覧取得 (Worktree First view / Branch Health) は **gix で読取**。
- 生成・削除は porcelain の副作用 (ディレクトリ作成、`.git/worktrees/*` 管理、
  ロック等) が多いため **`git worktree add/remove` に委譲**。
- Instant Worktree (issue #21) はデフォルトパス算出 → validate → `git worktree add`
  → State 更新、の順。

### 3. stash

- `gix-stash` の push/pop/list を優先利用 (feature gated)。ただし **apply / drop は
  未提供**なので CLI fallback。
- untracked を含む stash、conflict 時の pop 挙動 (`git stash pop` 準拠) に注意。
- MVP では「gix: save/pop/list、CLI: apply/drop」の混在で開始し、gix 側の充足に応じて
  CLI 依存を減らす。

### 4. checkout / reset

- branch switch は安全性重視で `git switch` / `git checkout` に委譲 (reset は gix 未対応)。
- switch 後は Diff / Blame / History HEAD / Branch Health / AheadBehind を invalidate
  (`docs/design/07-runtime.md` 11 章、issue #7)。

---

## CLI fallback ラッパーの要件 (issue #5 で実装)

- `git` バイナリの検出 (PATH、`xcode-select` 提供の git 等) とバージョン確認。
- 実行は Background Task (issue #6) 上で行い UI thread を block しない。
- 作業ディレクトリは対象 repository に固定 (`git -C <path>`)。
- 出力は可能な限り機械可読フラグ (`--porcelain`, `-z`, `--null`) を使う。
- 終了コード + stderr を `GitError` に変換 (下記)。

## エラー変換方針

`GitError` (issue #5 で定義) の想定バリアントと利用者向けメッセージの方針:

| バリアント | 発生源 | ユーザー向け表示例 |
|------------|--------|--------------------|
| `NotARepository` | open/discover 失敗 | 「Git リポジトリではありません」 |
| `GitBinaryNotFound` | CLI fallback で git 不在 | 「git コマンドが見つかりません」 |
| `Auth` | fetch/pull/push 認証失敗 | 「認証に失敗しました。資格情報を確認してください」 |
| `Conflict` | merge/pop の conflict | 「コンフリクトが発生しました」 (インライン表示) |
| `Io` / `Backend` | gix/CLI の内部エラー | 原因を要約し、詳細はログへ |

- 通常エラーはインライン表示、成功は原則無通知、破壊的操作のみ確認 UI
  (`docs/design/07-runtime.md` 13 章)。
- Repository 異常でもアプリ全体を落とさない。

## 実装ロードマップ対応

- issue #5: `GitService` trait / `GixService` (open, status) / `CliFallback` / `GitError`。
- 以降の MVP issue が本表の方針に従って各操作を追加する。
