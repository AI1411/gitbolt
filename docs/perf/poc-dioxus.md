# Dioxus Desktop 最小 PoC パフォーマンス計測 (issue #2)

設計書 `docs/design/02-tech-and-performance.md` 3 章のパフォーマンス目標が Dioxus
Desktop で達成可能かを、最小アプリで実測した記録。

## 対象

- 実装: [`examples/poc_status.rs`](../../examples/poc_status.rs)
  「リポジトリを開いて `git status` 一覧を表示するだけ」の最小 Dioxus Desktop アプリ。
- 計測スクリプト: [`scripts/perf/measure_poc.sh`](../../scripts/perf/measure_poc.sh)

## 計測項目と手法

| 項目 | 手法 |
|------|------|
| 初期操作可能時間 (time to first render) | `main` 冒頭で `Instant::now()` を記録し、初回レンダー後に走る `use_effect` で経過ミリ秒を測定 |
| メモリ (RSS / peak) | レンダー後に `/proc/self/status` の `VmRSS` / `VmHWM` を読む (Linux) |
| UI 反応 (update latency) | signal を更新 → 次レンダー確定 (`use_effect` 再実行) までのマイクロ秒を 200 回計測し min/median/max/avg を集計 |

`POC_BENCH=1` で自動計測モードになり、結果を JSON 1 行で stdout に出力して終了する。

再現手順:

```bash
bash .cursor/start.sh && export DISPLAY=:99   # 表示先 (ヘッドレス環境のみ)
scripts/perf/measure_poc.sh "$(pwd)" 6
```

## 計測結果

計測環境: **Ubuntu 24.04 x86_64 / Xvfb (ソフトウェアレンダリング, llvmpipe) / WebKitGTK 2.52 / release ビルド**。
6 回実行、対象リポジトリはこの repo 自身 (status 3 件)。

| 項目 | 目標 | 実測 (代表値) | 判定 |
|------|------|----------------|------|
| 初期操作可能時間 | 100ms 以内 | 中央値 **≈224ms** (219–243ms) | ⚠️ 未達 |
| 通常利用時メモリ (RSS) | 100MB 未満 (理想 50MB) | **≈160MB** (162–165MB) | ⚠️ 未達 |
| UI 反応 (state→再描画) | 16ms 以内 | 中央値 **≈0–1µs** / 最大 **≈7µs** | ✅ 達成 (桁違いに高速) |

生ログ: 各 run の JSON は `scripts/perf/measure_poc.sh` 実行時に stdout へ出力される。

```
{"time_to_first_render_ms":224,"rss_kib":162616,"peak_rss_kib":162616,"status_entries":3,"latency_iters":200,"update_latency_us":{"min":0,"median":0,"max":5,"avg":0}}
```

## 分析

### UI 反応: 十分に速い

state 更新から再描画確定までの Dioxus リアクティブコストは µs オーダーで、16ms 目標に対し
桁違いに余裕がある。小〜中規模の DOM では Dioxus の VirtualDOM diff がボトルネックにならない
ことを確認できた。大きな diff / 長い history では別途 virtualization が必要になる想定。

### 起動時間・メモリ: WebView 初期化が支配的

- **起動 (~224ms)** と **RSS (~160MB)** はいずれも目標未達。
- 最小アプリ (status 3 件・DOM ごく僅か) でこの値なので、コストの大半はアプリ側ではなく
  **WebView (WebKitGTK) の初期化とベースラインメモリ**に起因する。DOM 構築やアプリロジックは
  誤差レベル。

### 計測環境の重要な注意

MVP のターゲットは **macOS / Apple Silicon arm64** (設計書 2 章) だが、本計測は
**Linux + Xvfb + ソフトウェアレンダリング**上のもので、以下の理由から**そのままの数値を
ターゲット性能とみなせない**:

1. WebView バックエンドが異なる (Linux: WebKitGTK / macOS: WKWebView)。ベースラインメモリと
   初期化時間の特性が大きく異なる。
2. GPU が無く llvmpipe による CPU レンダリング。実機の GPU 合成より不利。
3. Xvfb 仮想ディスプレイのオーバーヘッド。

したがって本結果は「**Dioxus/アプリ側ロジックは目標に対して軽い**」「**WebView 初期化コストが
起動・メモリの主因**」という**相対的な結論**の根拠として扱い、絶対値は macOS 実機での再計測で
確定する。

## ボトルネックと対策案

| ボトルネック | 対策案 |
|--------------|--------|
| WebView 初期化による起動遅延 | ウィンドウ生成と Git メタデータ取得を並列化 (設計書 10 章「Repository Open」)。初期描画は最小 shell のみ、status 等は background 取得後に流し込み、体感初期操作可能時間を短縮する |
| WebView ベースラインメモリ (~150MB) | 単一 WebView に集約 (Single Window / 3 ペイン、設計書 6 章) しプロセス増殖を避ける。巨大データは `Arc` / Lazy 保持 (設計書 8 章) しコピーを作らない |
| 大規模 diff / history での DOM 肥大 | 可視範囲のみレンダーする virtualization。History は 100 件段階ロード、Blame は可視行優先 (設計書 6・10 章) |
| 計測の非代表性 | macOS arm64 実機で `scripts/perf/measure_poc.sh` を再実行し、本ドキュメントに実機値を追記する (フォローアップ) |

## 以降の設計判断への反映

- **レンダリング戦略**: Dioxus のリアクティブ更新は高速なので、UI 側は素直に signal 駆動で実装し、
  重い処理は Git Service / Background Task (issue #5・#6) に逃がす方針で問題ない。
- **起動戦略**: 「最小 shell を即描画 → 実データは非同期流し込み」を前提に Repository Open フローを
  設計する (既存コンテンツを消して Loading にしない、設計書 3 章)。
- **メモリ戦略**: WebView は 1 つに集約し、Git 由来の巨大データは共有参照 / キャッシュで持つ。
- **未確定事項 (フォローアップ)**: macOS arm64 実機での絶対値確定。目標未達が実機でも続く場合は
  起動時の遅延描画・メモリ上限監視を MVP のパフォーマンス受け入れ基準に組み込む。
