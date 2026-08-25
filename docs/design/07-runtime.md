# 11. キャッシュ・更新

- Filesystem watcher + debounce
- 変更ファイルに関係するcacheのみinvalidate
- HEAD変更時はDiff/Blame/History HEAD/Branch Health/AheadBehindをinvalidate
- Blame cache key: repository + HEAD OID + file path

# 12. Background Task

| 優先度 | 対象 |
|--------|------|
| P0 | Commit等のユーザー操作 |
| P1 | Selected File Diff等Visible UI |
| P2 | 次ページHistory等Near-visible data |
| P3 | 非表示branch metadata等 |

Taskを無制限spawnせず、worker数はbenchmarkで調整する。

# 13. エラー・安全設計

- 通常エラーはインライン表示
- 成功時は原則通知しない
- Commit失敗時はmessage保持
- Repository異常でもアプリ全体を落とさない
- 破壊的操作のみ確認UI
