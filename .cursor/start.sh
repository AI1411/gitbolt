#!/usr/bin/env bash
# Per-boot runtime setup: provide a virtual X display so the Dioxus Desktop
# window can render without a physical monitor. Run `DISPLAY=:99 cargo run`
# (or `DISPLAY=:99 dx serve --desktop`) to launch the app.
set -euo pipefail

DISPLAY_NUM=":99"

if ! pgrep -x Xvfb >/dev/null 2>&1; then
    Xvfb "$DISPLAY_NUM" -screen 0 1280x800x24 >/tmp/xvfb.log 2>&1 &
    # Wait for the X socket to appear so downstream clients can connect.
    for _ in $(seq 1 20); do
        if [ -S "/tmp/.X11-unix/X${DISPLAY_NUM#:}" ]; then
            break
        fi
        sleep 0.5
    done
fi
