# 1. プロダクト概要

GitBoltはRustで実装する高速・軽量なデスクトップGit GUI。Gitリポジトリの「今」と「過去」を瞬時に理解し、CLIと同じ速度感で操作できる体験を目指す。

## 設計原則

- Fast First
- Keyboard First
- Single Window
- Context First
- Progressive Disclosure
- Lazy Everything
- Zero Waiting
- No Unnecessary Runtime

## ターゲット

- CLIでGitを日常利用するエンジニア
- 既存Git GUIを重いと感じるユーザー
- diff / stage / commitを高速に行いたいユーザー
- branch / worktreeを多用するユーザー
- 大規模repositoryを扱うユーザー
