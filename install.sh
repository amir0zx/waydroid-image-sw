#!/usr/bin/env bash
set -euo pipefail

REPO="amir0zx/waydroid-image-sw"
API="https://api.github.com/repos/${REPO}/releases/latest"
BIN_DIR="${HOME}/.local/bin"
BIN_PATH="${BIN_DIR}/waydroid-image-sw"
TMP_DIR="${TMPDIR:-/tmp}/waydroid-image-sw"
TMP_BIN="${TMP_DIR}/waydroid-image-sw"

mkdir -p "$TMP_DIR" "$BIN_DIR"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "Missing required command: $1" >&2
    exit 1
  }
}

need_cmd curl
need_cmd sed
need_cmd grep

latest_json="$(curl -fsSL "$API")"
latest_tag="$(printf '%s' "$latest_json" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)"

if [ -z "$latest_tag" ]; then
  echo "Failed to detect latest release tag." >&2
  exit 1
fi

local_ver=""
if [ -x "$BIN_PATH" ]; then
  local_ver="$($BIN_PATH --version 2>/dev/null | awk '{print $2}' || true)"
fi

if [ -x "$BIN_PATH" ] && [ "$local_ver" = "$latest_tag" ]; then
  echo "waydroid-image-sw is installed and up to date (${local_ver})."
  exec "$BIN_PATH"
fi

asset_url=""
for name in "waydroid-image-sw" "waydroid-image-sw-installer"; do
  url="https://github.com/${REPO}/releases/download/${latest_tag}/${name}"
  if curl -fsI "$url" >/dev/null 2>&1; then
    asset_url="$url"
    break
  fi
done

if [ -z "$asset_url" ]; then
  echo "No compatible Linux x86_64 binary asset found in release ${latest_tag}." >&2
  exit 1
fi

if [ -x "$BIN_PATH" ] && [ -n "$local_ver" ] && [ "$local_ver" != "$latest_tag" ]; then
  printf "Update available: %s -> %s. Update now? [y/N]: " "$local_ver" "$latest_tag"
  read -r ans
  case "$ans" in
    y|Y|yes|YES) ;;
    *)
      echo "Keeping current version (${local_ver}). Starting it..."
      exec "$BIN_PATH"
      ;;
  esac
fi

echo "Downloading waydroid-image-sw ${latest_tag}..."
curl -fL --progress-bar -o "$TMP_BIN" "$asset_url"
chmod +x "$TMP_BIN"
cp "$TMP_BIN" "$BIN_PATH"

echo "Installed ${latest_tag} to ${BIN_PATH}"
exec "$BIN_PATH"
