#!/usr/bin/env bash
set -euo pipefail

if [ ! -t 1 ]; then
  echo "This installer is TUI-based. Run it directly:" >&2
  echo "  curl -fsSL https://raw.githubusercontent.com/amir0zx/waydroid-image-sw/main/install.sh -o /tmp/waydroid-image-sw-installer" >&2
  echo "  chmod +x /tmp/waydroid-image-sw-installer" >&2
  echo "  /tmp/waydroid-image-sw-installer" >&2
  exit 1
fi

REPO="https://github.com/amir0zx/waydroid-image-sw"
TMP_DIR="${TMPDIR:-/tmp}/waydroid-image-sw"
BIN="$TMP_DIR/waydroid-image-sw-installer"

mkdir -p "$TMP_DIR"

if command -v curl >/dev/null 2>&1; then
  curl -L --progress-bar -o "$BIN" "$REPO/releases/latest/download/waydroid-image-sw-installer"
elif command -v wget >/dev/null 2>&1; then
  wget -O "$BIN" "$REPO/releases/latest/download/waydroid-image-sw-installer"
else
  echo "curl or wget is required." >&2
  exit 1
fi

chmod +x "$BIN"
exec "$BIN"
