# waydroid-image-sw

Universal Waydroid image switcher for Linux.

`waydroid-image-sw` is a terminal UI + CLI tool to switch active Waydroid images safely, manage multiple image profiles, and quickly view the current selected profile.

## Features

- Universal profile switching (not limited to TV/A13)
- Detects profiles from `~/waydroid-images/*`
- Shows current active `images_path` status
- Interactive Ratatui TUI selector
- CLI commands for `list`, `status`, and direct switching
- Bootstrap installer/updater script with version check

## Install / Update / Launch

```bash
curl -fsSL https://raw.githubusercontent.com/amir0zx/waydroid-image-sw/main/install.sh | bash
```

What this does:

1. Checks whether `~/.local/bin/waydroid-image-sw` exists.
2. Compares local version vs latest GitHub release.
3. Installs if missing.
4. Prompts to update if outdated.
5. Launches immediately if up to date.

## TUI Usage

```bash
waydroid-image-sw
```

Keys:

- `Up/Down` to select profile
- `Enter` to switch
- `q` to quit

## CLI Usage

```bash
waydroid-switch list
waydroid-switch status
waydroid-switch <profile>
```

Profiles are directories under `~/waydroid-images` containing:

- `system.img`
- `vendor.img`

## Build

```bash
cargo build --release
```

## SEO Keywords

Waydroid image switcher, Waydroid profile manager, Linux Waydroid tool, Android TV Waydroid, Waydroid A13, Waydroid image path switch.

## License

MIT
