#!/bin/bash

set -euo pipefail

echo "==> Kyro setup (Unix/macOS)"

if ! command -v node >/dev/null 2>&1; then
  echo "[ERROR] Node.js is required (20+ recommended)."
  exit 1
fi

if ! command -v rustup >/dev/null 2>&1; then
  echo "[ERROR] rustup is required. Install from https://rustup.rs"
  exit 1
fi

if ! command -v bun >/dev/null 2>&1; then
  echo "[ERROR] Bun is required. Install from https://bun.sh"
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "==> Initializing Rust toolchain"
  rustup default stable
fi

if [[ "$(uname -s)" == "Linux" ]]; then
  echo "==> Linux dependency hint"
  echo "Install Tauri deps if not already installed:"
  echo "  sudo apt install -y build-essential libgtk-3-dev libwebkit2gtk-4.1-dev librsvg2-dev libssl-dev libayatana-appindicator3-dev patchelf"
fi

if [[ "$(uname -s)" == "Darwin" ]]; then
  if ! xcode-select -p >/dev/null 2>&1; then
    echo "[ERROR] Xcode Command Line Tools are required: xcode-select --install"
    exit 1
  fi
fi

echo "==> Installing project dependencies"
bun install

echo "==> Running kyro doctor"
bun run doctor

echo ""
echo "Setup complete. Next steps:"
echo "  1) Run checks:  ./scripts/check-all.ps1 (Windows) or run individual Bun/Cargo checks"
echo "  2) Start dev:   bun run tauri:dev"
echo "  3) Build app:   bun run tauri:build"
