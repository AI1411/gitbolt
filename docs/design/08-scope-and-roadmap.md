# 14. MVP対象外

- GitHub API / Pull Request / Issue / Actions
- 高度なMerge Conflict Editor
- Interactive Rebase
- Advanced Cherry-pick
- Git LFS GUI
- Submodule管理
- Plugin System
- AI機能

# 16. CLI・完成体験

```
$ gitbolt .
→ Changes
→ file select
→ diff
→ Space
→ C
→ message
→ ⌘Enter
```

単なるGit操作のGUI化ではなく、変更理由・変更者・履歴・branch/remote差分・worktreeの所在までを一瞬で理解して操作する。

# 17. 次フェーズ

- Dioxus vs eguiの小規模PoCと実測比較
- gix / git CLI fallback対応表
- AppState / UiEvent / Command / AppMessageのRust型確定
- Git Service trait設計
- Cache invalidation詳細
- MVP実装タスク分解
