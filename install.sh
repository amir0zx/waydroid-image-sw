#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_SRC="${ROOT_DIR}/target/release/waydroid-switch"
BIN_DST="/usr/bin/waydroid-switch"

if [ ! -x "$BIN_SRC" ]; then
  echo "Binary not found: $BIN_SRC" >&2
  echo "Build it first:" >&2
  echo "  cargo build --release" >&2
  exit 1
fi

echo "Installing $BIN_SRC -> $BIN_DST"
sudo install -m 0755 "$BIN_SRC" "$BIN_DST"

echo "Installed. Run: waydroid-switch"
