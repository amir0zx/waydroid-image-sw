#!/usr/bin/env bash
set -euo pipefail

# Simple TUI installer for waydroid-image-sw

C_RESET="\033[0m"
C_RED="\033[31m"
C_GREEN="\033[32m"
C_YELLOW="\033[33m"
C_BLUE="\033[34m"
C_CYAN="\033[36m"
C_BOLD="\033[1m"

# Ensure interactive prompts work even when piped via curl | bash
TTY_IN="/dev/tty"
TTY_OUT="/dev/tty"
if [ ! -t 0 ] && [ -r /dev/tty ]; then
  exec </dev/tty
fi
if [ ! -t 1 ] || [ ! -w /dev/tty ]; then
  TTY_OUT="/dev/stdout"
fi

say() { printf "%b" "$*" >"$TTY_OUT"; }
sayln() { printf "%b\n" "$*" >"$TTY_OUT"; }
read_tty() { read -r "$@" <"$TTY_IN"; }

banner() {
  say "${C_CYAN}${C_BOLD}"
  cat >"$TTY_OUT" <<'BANNER'
    __      __           __           _     _                 
   / /___  / /___ ______/ /__________(_)___(_)___  ____ ______
  / / __ \/ / __ \`/ ___/ __/ ___/ __/ / __/ / __ \/ __ \`/ ___/
 / / /_/ / / /_/ / /__/ /_/ /  / /_/ / /_/ / / / / /_/ / /    
/_/\____/_/\__,_/\___/\__/__/   \__/_/\__/_/_/ /_/\__,_/_/     
BANNER
  say "${C_RESET}"
}

prompt_path() {
  local label="$1"
  local out_var="$2"
  local path=""
  while true; do
    say "${C_YELLOW}${label}${C_RESET}"
    read_tty path
    if [ -f "$path" ]; then
      eval "$out_var=\"$path\""
      return 0
    fi
    sayln "${C_RED}Invalid path. Try again.${C_RESET}"
  done
}

prompt_optional() {
  local label="$1"
  local out_var="$2"
  local value=""
  say "${C_YELLOW}${label}${C_RESET}"
  read_tty value || true
  eval "$out_var=\"$value\""
}

confirm() {
  local msg="$1"
  local ans=""
  while true; do
    say "${C_BLUE}${msg} [y/n]: ${C_RESET}"
    read_tty ans
    case "$ans" in
      y|Y) return 0 ;;
      n|N) return 1 ;;
      *) sayln "${C_RED}Please enter y or n.${C_RESET}" ;;
    esac
  done
}

hash_file() {
  local file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{print $1}'
  else
    return 1
  fi
}

verify_checksum() {
  local file="$1"
  local expected="$2"
  if [ -z "$expected" ]; then
    return 0
  fi
  if ! command -v sha256sum >/dev/null 2>&1 && ! command -v shasum >/dev/null 2>&1; then
    sayln "${C_YELLOW}No sha256 tool found. Skipping checksum.${C_RESET}"
    return 0
  fi
  local actual
  actual=$(hash_file "$file" || true)
  if [ -z "$actual" ]; then
    sayln "${C_RED}Failed to compute checksum for $file${C_RESET}"
    return 1
  fi
  if [ "$actual" != "$expected" ]; then
    sayln "${C_RED}Checksum mismatch for $file${C_RESET}"
    sayln "  expected: $expected"
    sayln "  actual:   $actual"
    return 1
  fi
  sayln "${C_GREEN}Checksum OK:${C_RESET} $file"
}

download_if_url() {
  local label="$1"
  local url="$2"
  local out_var="$3"
  local target_dir="$4"

  if [ -z "$url" ]; then
    eval "$out_var=\"\""
    return 0
  fi

  mkdir -p "$target_dir"
  local filename
  filename=$(basename "$url")
  local dest="$target_dir/$filename"

  sayln "${C_BLUE}Downloading ${label}...${C_RESET}"
  if command -v curl >/dev/null 2>&1; then
    curl -L --progress-bar -o "$dest" "$url"
  elif command -v wget >/dev/null 2>&1; then
    wget -O "$dest" "$url"
  else
    sayln "${C_RED}Neither curl nor wget found.${C_RESET}"
    exit 1
  fi

  eval "$out_var=\"$dest\""
}

main() {
  banner
  sayln "${C_GREEN}${C_BOLD}Waydroid Image Switcher Installer${C_RESET}\n"

  sayln "${C_BLUE}This will move images into:${C_RESET} ~/waydroid-images/{tv,a13}\n"

  sayln "${C_BOLD}Optional download URLs (press Enter to skip):${C_RESET}"
  prompt_optional "TV system.img URL: " tv_system_url
  prompt_optional "TV vendor.img URL: " tv_vendor_url
  prompt_optional "A13 system.img URL: " a13_system_url
  prompt_optional "A13 vendor.img URL: " a13_vendor_url

  dl_dir="$HOME/waydroid-images/downloads"
  download_if_url "TV system.img" "$tv_system_url" tv_system "$dl_dir"
  download_if_url "TV vendor.img" "$tv_vendor_url" tv_vendor "$dl_dir"
  download_if_url "A13 system.img" "$a13_system_url" a13_system "$dl_dir"
  download_if_url "A13 vendor.img" "$a13_vendor_url" a13_vendor "$dl_dir"

  if [ -z "${tv_system:-}" ]; then
    prompt_path "Path to TV system.img: " tv_system
  fi
  if [ -z "${tv_vendor:-}" ]; then
    prompt_path "Path to TV vendor.img: " tv_vendor
  fi
  if [ -z "${a13_system:-}" ]; then
    prompt_path "Path to A13 system.img: " a13_system
  fi
  if [ -z "${a13_vendor:-}" ]; then
    prompt_path "Path to A13 vendor.img: " a13_vendor
  fi

  sayln "\n${C_BOLD}Optional SHA256 checksums (press Enter to skip):${C_RESET}"
  prompt_optional "TV system.img SHA256: " tv_system_sha
  prompt_optional "TV vendor.img SHA256: " tv_vendor_sha
  prompt_optional "A13 system.img SHA256: " a13_system_sha
  prompt_optional "A13 vendor.img SHA256: " a13_vendor_sha

  verify_checksum "$tv_system" "$tv_system_sha"
  verify_checksum "$tv_vendor" "$tv_vendor_sha"
  verify_checksum "$a13_system" "$a13_system_sha"
  verify_checksum "$a13_vendor" "$a13_vendor_sha"

  sayln "\n${C_BOLD}Review:${C_RESET}"
  sayln "  TV  system: $tv_system"
  sayln "  TV  vendor: $tv_vendor"
  sayln "  A13 system: $a13_system"
  sayln "  A13 vendor: $a13_vendor"

  if ! confirm "Proceed to move these files?"; then
    sayln "${C_RED}Cancelled.${C_RESET}"
    exit 1
  fi

  base="$HOME/waydroid-images"
  mkdir -p "$base/tv" "$base/a13"

  mv -v "$tv_system" "$base/tv/system.img"
  mv -v "$tv_vendor" "$base/tv/vendor.img"
  mv -v "$a13_system" "$base/a13/system.img"
  mv -v "$a13_vendor" "$base/a13/vendor.img"

  sayln "\n${C_GREEN}${C_BOLD}Done!${C_RESET} Images are ready."
  sayln "Now run: ${C_CYAN}./waydroid-switch tv${C_RESET} or ${C_CYAN}./waydroid-switch a13${C_RESET}"
}

main "$@"
