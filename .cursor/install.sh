#!/usr/bin/env bash
# Idempotent dependency setup for the gitbolt Dioxus Desktop app.
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

# System libraries required to build and run the Dioxus Desktop (wry / WebKitGTK)
# application, plus Xvfb so the GUI can be launched headlessly in an agent VM.
sudo apt-get update -qq
sudo apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    libwebkit2gtk-4.1-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    libxdo-dev \
    xvfb

# Dioxus CLI (`dx`) powers the hot-reload workflow documented in the README
# (`dx serve --desktop`). Skip the (re)build when it is already installed.
if ! command -v dx >/dev/null 2>&1; then
    cargo install dioxus-cli --locked
fi

# Fetch crates and compile so the workspace is ready to run immediately.
cargo build --locked
