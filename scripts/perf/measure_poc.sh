#!/usr/bin/env bash
# Measure the Dioxus Desktop performance PoC (issue #2).
#
# Builds the `poc_status` example in release mode and runs it in benchmark mode
# a few times, collecting time-to-first-render, resident memory, and a
# signal-update -> re-render latency proxy. Results are printed as JSON lines.
#
# Requirements: a display. On a headless machine start one first, e.g.
#   bash .cursor/start.sh && export DISPLAY=:99
#
# Usage:
#   scripts/perf/measure_poc.sh [REPO_PATH] [RUNS]
set -euo pipefail

REPO_PATH="${1:-$(pwd)}"
RUNS="${2:-5}"
export POC_REPO="$REPO_PATH"
export POC_BENCH=1
export POC_ITERS="${POC_ITERS:-200}"

if [ -z "${DISPLAY:-}" ]; then
    echo "warning: DISPLAY is not set; the desktop window needs an X server." >&2
    echo "         run 'bash .cursor/start.sh && export DISPLAY=:99' first." >&2
fi

echo "Building release example poc_status..." >&2
cargo build --release --example poc_status >&2

BIN="target/release/examples/poc_status"

echo "Running $RUNS benchmark iterations against: $REPO_PATH" >&2
for i in $(seq 1 "$RUNS"); do
    printf '{"run":%s,' "$i"
    # Strip the leading brace from the app's JSON so we can prepend "run".
    "$BIN" | sed 's/^{//'
done
