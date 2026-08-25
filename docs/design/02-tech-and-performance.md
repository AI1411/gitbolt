# 2. 技術方針

| 要素 | 方針 |
|------|------|
| Language | Rust |
| GUI | Dioxus Desktop(確定) |
| Git Engine | gix (gitoxide) |
| File Watcher | notify |
| 並列処理 | rayon等 |
| MVP OS | macOS / Apple Silicon arm64 |
| 将来 | Windows / Linux |

GUIはDioxus Desktopに確定(egui / eframeは不採用)。

Git操作は可能な限りgixで直接扱い、必要な操作のみgit CLIへfallbackする。UI層からGitを直接操作せずGit Serviceへ集約する。

# 3. パフォーマンス要件

| 項目 | 目標 |
|------|------|
| 初期操作可能時間 | 100ms以内を目標 |
| 通常利用時メモリ | 100MB未満、理想50MB前後 |
| UI反応 | 16ms以内を理想 |
| History | 100件程度から段階ロード |
| Blame | 選択行・可視範囲優先 |

- UI threadをGit処理でblockしない
- 古い非同期結果はgeneration/versionで破棄
- 起動時に全履歴・全Blameを読まない
- 既存コンテンツを消してLoading画面にしない
