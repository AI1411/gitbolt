# 8. アプリケーション状態

```
AppState
├── RepositoryState
├── ChangesState
├── DiffState
├── HistoryState
├── BranchState
├── WorktreeState
├── SelectionState
├── NavigationState
├── BackgroundTaskState
└── UiState
```

Git由来データとUI状態を分離し、巨大データはArc / Cache / Lazy Dataで保持する。

# 9. イベント・Commandモデル

```
UI → UiEvent → State/Command → Worker/Git Service → AppMessage → State → Dioxus Re-render
```

UIからGitを直接変更しない。副作用はCommandとして実行し、結果MessageでStateを更新する。

# 15. 想定モジュール構成

```
src/
├── app/       # state / event / command / message
├── ui/        # pulse / changes / diff / history / branches / worktrees / blame
├── git/       # repository / status / diff / commit / history / blame / branch / remote / worktree
├── cache/
├── watcher/
├── task/
└── platform/
```
