# GitBolt 超高速・軽量 Git GUI 設計書 v0.1

A blazing-fast, lightweight Git GUI built with Rust.

これまでの要件定義、GitToolBox系機能分類、画面設計、MVPユーザーフロー、状態遷移、技術選定を統合した初期設計書。

## 目次

| # | ファイル | 内容 |
|---|----------|------|
| 1 | [01-overview.md](01-overview.md) | プロダクト概要・設計原則・ターゲット |
| 2 | [02-tech-and-performance.md](02-tech-and-performance.md) | 技術方針・パフォーマンス要件 |
| 3 | [03-features.md](03-features.md) | MVP機能・GitToolBox系機能・GitBolt独自拡張 |
| 4 | [04-ui-design.md](04-ui-design.md) | 画面設計・Keyboard First |
| 5 | [05-architecture.md](05-architecture.md) | アプリケーション状態・イベント/Commandモデル・モジュール構成 |
| 6 | [06-user-flows.md](06-user-flows.md) | 主要ユーザーフロー |
| 7 | [07-runtime.md](07-runtime.md) | キャッシュ・更新・Background Task・エラー/安全設計 |
| 8 | [08-scope-and-roadmap.md](08-scope-and-roadmap.md) | MVP対象外・CLI/完成体験・次フェーズ |

> 元ファイル: `docs/GitBolt_Design_v0.1.docx`
