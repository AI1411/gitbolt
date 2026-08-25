# Cargo プロジェクト初期化 (Dioxus Desktop) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dioxus Desktop 0.7 ベースの Cargo プロジェクトを作成し、設計書 15 章のモジュール骨組みと CI (fmt + clippy + build) を整備する。

**Architecture:** `gitbolt` クレートは `src/lib.rs` をモジュールルートとし、`main.rs` から空ウィンドウの `App` コンポーネントを起動する。`app` / `ui` / `git` / `cache` / `watcher` / `task` / `platform` は Phase 0 ではプレースホルダ型のみを持ち、後続 issue で実装する。

**Tech Stack:** Rust stable, Dioxus 0.7 (desktop feature), GitHub Actions (macos-latest)

## Global Constraints

- Dioxus 0.7 系を使用
- MVP OS: macOS / Apple Silicon arm64
- 設計書 `docs/design/05-architecture.md` 15 章のモジュール構成に従う
- CI: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release`
- `.gitignore`: `target/`, `.DS_Store` 等

---

## File Structure

| Path | Responsibility |
|------|----------------|
| `Cargo.toml` | パッケージ定義、Dioxus 0.7 desktop 依存、clippy lint |
| `Dioxus.toml` | `dx serve --desktop` 用設定 |
| `rust-toolchain.toml` | stable + rustfmt + clippy |
| `rustfmt.toml` | フォーマット設定 |
| `src/main.rs` | エントリポイント (`dioxus::launch`) |
| `src/lib.rs` | モジュールルート |
| `src/app/` | state / event / command / message |
| `src/ui/` | 各ビュー + `App` コンポーネント |
| `src/git/` | Git サービス層 |
| `src/cache/` | キャッシュ層 |
| `src/watcher/` | ファイル監視 |
| `src/task/` | バックグラウンドタスク |
| `src/platform/` | プラットフォーム判定 |
| `.github/workflows/ci.yml` | CI パイプライン |
| `.gitignore` | ビルド成果物・OS 固有ファイル |

---

### Task 1: Cargo プロジェクトと Dioxus 依存

**Files:**
- Create: `Cargo.toml`
- Create: `Dioxus.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`

**Interfaces:**
- Consumes: なし
- Produces: `gitbolt::ui::App` コンポーネント、`dioxus::launch(App)` エントリ

- [x] **Step 1:** `Cargo.toml` に `dioxus = { version = "0.7", features = ["desktop"] }` を追加
- [x] **Step 2:** `main.rs` で `dioxus::launch(App)` を呼び出す
- [x] **Step 3:** `cargo build` が通ることを確認

---

### Task 2: モジュール骨組み

**Files:**
- Create: `src/app/{mod,state,event,command,message}.rs`
- Create: `src/ui/{mod,pulse,changes,diff,history,branches,worktrees,blame}.rs`
- Create: `src/git/{mod,repository,status,diff,commit,history,blame,branch,remote,worktree}.rs`
- Create: `src/cache/mod.rs`, `src/watcher/mod.rs`, `src/task/mod.rs`, `src/platform/mod.rs`

**Interfaces:**
- Consumes: `dioxus::prelude::*`
- Produces: 各モジュールのプレースホルダ型、`pub fn App() -> Element`

- [x] **Step 1:** 設計書 15 章に従い全サブモジュールを作成
- [x] **Step 2:** `ui/mod.rs` に空ウィンドウ (`100vw` × `100vh`) の `App` コンポーネントを定義
- [x] **Step 3:** `cargo clippy --all-targets -- -D warnings` が通ることを確認

---

### Task 3: ツールチェーン設定

**Files:**
- Create: `rust-toolchain.toml`
- Create: `rustfmt.toml`
- Modify: `.gitignore`

- [x] **Step 1:** `rust-toolchain.toml` に stable + rustfmt + clippy を指定
- [x] **Step 2:** `rustfmt.toml` に edition 2021, max_width 100 を設定
- [x] **Step 3:** `.gitignore` に `target/`, `.DS_Store`, `.dioxus/` を追加

---

### Task 4: CI ワークフロー

**Files:**
- Create: `.github/workflows/ci.yml`

- [x] **Step 1:** `macos-latest` ランナーで fmt / clippy / build を実行
- [x] **Step 2:** `Swatinem/rust-cache@v2` でキャッシュ

---

### Task 5: ドキュメント更新

**Files:**
- Modify: `README.md`

- [x] **Step 1:** 開発手順 (`dx serve --desktop`, `cargo run`) とチェックコマンドを記載

---

## Self-Review

1. **Spec coverage:** Issue #1 の全タスク (cargo init, モジュール雛形, .gitignore, rustfmt/clippy, CI) をカバー済み。macOS arm64 での `dx serve` / `cargo run` は CI (macos-latest) と README で対応。
2. **Placeholder scan:** プレースホルダ型は意図的。TBD/TODO なし。
3. **Type consistency:** `App` コンポーネントは `ui/mod.rs` に一元定義、`main.rs` から参照。
