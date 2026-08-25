# 10. 主要ユーザーフロー

## Repository Open

```
OpenRepository → Discover .git → Minimal Metadata → Ready → Changes/Branch/History/Watcher並列開始
```

## Diff

```
SelectFile → Cache Hitなら即表示 / MissならBackground Diff → Cache → 表示
```

## Stage

```
Space → Optimistic UI → Git Index Update → Success維持 / Error Rollback → Refresh
```

## Commit

```
Validate → Commit → HEAD/Changes即更新 → History/AheadBehind background更新
```

## Branch Switch

```
Preflight → Checkout → HEAD更新 → Diff/Blame/History/Branch Health invalidate → Minimal Reload
```

## Instant Worktree

```
Select Branch → W → Default Path → Validate → Create → State Update → Optional Open
```

## History / Blame

- History: recent 100 → scrollで追加100
- Blame: Current line → Visible lines → Nearby lines → Rest of file
