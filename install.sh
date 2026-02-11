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

banner() {
  printf "%b" "${C_CYAN}${C_BOLD}"
  cat <<'BANNER'
    __      __           __           _     _                 
   / /___  / /___ ______/ /__________(_)___(_)___  ____ ______
  / / __ \/ / __ \`/ ___/ __/ ___/ __/ / __/ / __ \/ __ \`/ ___/
 / / /_/ / / /_/ / /__/ /_/ /  / /_/ / /_/ / / / / /_/ / /    
/_/\____/_/\__,_/\___/\__/__/   \__/_/\__/_/_/ /_/\__,_/_/     
BANNER
  printf "%b" "${C_RESET}"
}

prompt_path() {
  local label="$1"
  local out_var="$2"
  local path=""
  while true; do
    printf "%b%s%b" "${C_YELLOW}" "$label" "${C_RESET}"
    read -r path
    if [ -f "$path" ]; then
      eval "$out_var=\"$path\""
      return 0
    fi
    printf "%bInvalid path. Try again.%b\n" "${C_RED}" "${C_RESET}"
  done
}

prompt_optional() {
  local label="$1"
  local out_var="$2"
  local value=""
  printf "%b%s%b" "${C_YELLOW}" "$label" "${C_RESET}"
  read -r value || true
  eval "$out_var=\"$value\""
}

confirm() {
  local msg="$1"
  local ans=""
  while true; do
    printf "%b%s [y/n]: %b" "${C_BLUE}" "$msg" "${C_RESET}"
    read -r ans
    case "$ans" in
      y|Y) return 0 ;;
      n|N) return 1 ;;
      *) printf "%bPlease enter y or n.%b\n" "${C_RED}" "${C_RESET}" ;;
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
    printf "%bNo sha256 tool found. Skipping checksum.%b\n" "${C_YELLOW}" "${C_RESET}"
    return 0
  fi
  local actual
  actual=$(hash_file "$file" || true)
  if [ -z "$actual" ]; then
    printf "%bFailed to compute checksum for %s%b\n" "${C_RED}" "$file" "${C_RESET}"
    return 1
  fi
  if [ "$actual" != "$expected" ]; then
    printf "%bChecksum mismatch for %s%b\n" "${C_RED}" "$file" "${C_RESET}"
    printf "  expected: %s\n  actual:   %s\n" "$expected" "$actual"
    return 1
  fi
  printf "%bChecksum OK:%b %s\n" "${C_GREEN}" "${C_RESET}" "$file"
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

  printf "%bDownloading %s...%b\n" "${C_BLUE}" "$label" "${C_RESET}"
  if command -v curl >/dev/null 2>&1; then
    curl -L --progress-bar -o "$dest" "$url"
  elif command -v wget >/dev/null 2>&1; then
    wget -O "$dest" "$url"
  else
    printf "%bNeither curl nor wget found.%b\n" "${C_RED}" "${C_RESET}"
    exit 1
  fi

  eval "$out_var=\"$dest\""
}

main() {
  banner
  printf "%bWaydroid Image Switcher Installer%b\n\n" "${C_GREEN}${C_BOLD}" "${C_RESET}"

  printf "%bThis will move images into:%b ~/waydroid-images/{tv,a13}\n\n" "${C_BLUE}" "${C_RESET}"

  printf "%bOptional download URLs (press Enter to skip):%b\n" "${C_BOLD}" "${C_RESET}"
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

  printf "\n%bOptional SHA256 checksums (press Enter to skip):%b\n" "${C_BOLD}" "${C_RESET}"
  prompt_optional "TV system.img SHA256: " tv_system_sha
  prompt_optional "TV vendor.img SHA256: " tv_vendor_sha
  prompt_optional "A13 system.img SHA256: " a13_system_sha
  prompt_optional "A13 vendor.img SHA256: " a13_vendor_sha

  verify_checksum "$tv_system" "$tv_system_sha"
  verify_checksum "$tv_vendor" "$tv_vendor_sha"
  verify_checksum "$a13_system" "$a13_system_sha"
  verify_checksum "$a13_vendor" "$a13_vendor_sha"

  printf "\n%bReview:%b\n" "${C_BOLD}" "${C_RESET}"
  printf "  TV  system: %s\n" "$tv_system"
  printf "  TV  vendor: %s\n" "$tv_vendor"
  printf "  A13 system: %s\n" "$a13_system"
  printf "  A13 vendor: %s\n" "$a13_vendor"

  if ! confirm "Proceed to move these files?"; then
    printf "%bCancelled.%b\n" "${C_RED}" "${C_RESET}"
    exit 1
  fi

  base="$HOME/waydroid-images"
  mkdir -p "$base/tv" "$base/a13"

  mv -v "$tv_system" "$base/tv/system.img"
  mv -v "$tv_vendor" "$base/tv/vendor.img"
  mv -v "$a13_system" "$base/a13/system.img"
  mv -v "$a13_vendor" "$base/a13/vendor.img"

  printf "\n%bDone!%b Images are ready.\n" "${C_GREEN}${C_BOLD}" "${C_RESET}"
  printf "Now run: %b./waydroid-switch tv%b or %b./waydroid-switch a13%b\n" "${C_CYAN}" "${C_RESET}" "${C_CYAN}" "${C_RESET}"
}

main "$@"
