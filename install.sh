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
  printf "%b\n" "${C_CYAN}${C_BOLD}"
  printf "    __      __           __           _     _                 \n"
  printf "   / /___  / /___ ______/ /__________(_)___(_)___  ____ ______\n"
  printf "  / / __ \/ / __ \\`/ ___/ __/ ___/ __/ / __/ / __ \/ __ \\`/ ___/\n"
  printf " / / /_/ / / /_/ / /__/ /_/ /  / /_/ / /_/ / / / / /_/ / /    \n"
  printf "/_/\____/_/\__,_/\___/\__/__/   \__/_/\__/_/_/ /_/\__,_/_/     \n"
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

main() {
  banner
  printf "%bWaydroid Image Switcher Installer%b\n\n" "${C_GREEN}${C_BOLD}" "${C_RESET}"

  printf "%bThis will move images into:%b ~/waydroid-images/{tv,a13}\n\n" "${C_BLUE}" "${C_RESET}"

  prompt_path "Path to TV system.img: " tv_system
  prompt_path "Path to TV vendor.img: " tv_vendor
  prompt_path "Path to A13 system.img: " a13_system
  prompt_path "Path to A13 vendor.img: " a13_vendor

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
