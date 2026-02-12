# waydroid-switch

Universal Waydroid image switcher (single TUI binary).

`waydroid-switch` auto-scans `~/waydroid-images` for any profile folders containing `system.img` and `vendor.img`, shows the current active `images_path`, and switches profiles safely.

## Features

- Single TUI binary: `waydroid-switch`
- Auto-searches `~/waydroid-images` recursively
- Supports linked images (symlinks)
- Shows current active `images_path`
- Full profile switch: image + userdata
- Manual add submenu for custom image paths
- Universal switching (not limited to TV/A13)

## Build

```bash
cargo build --release
```

## Install

`install.sh` only installs the local built binary to `/usr/bin`.

```bash
./install.sh
```

## Run

```bash
waydroid-switch
```

## TUI Keys

- `Up/Down`: move
- `Enter`: switch selected profile
  - Also switches Waydroid userdata to a profile-specific directory
- `a`: manual add submenu
- `r`: refresh auto-scan list
- `q`: quit

## Requirements

- Waydroid installed
- `sudo` access (for updating `/var/lib/waydroid/waydroid.cfg` and stopping/starting session)
- Image profiles under `~/waydroid-images`

## Data Isolation

On switch, the app links live Waydroid userdata to a profile-specific path:

- Live path: `~/.local/share/waydroid/data`
- Profile store: `~/.local/share/waydroid/profiles/<profile-id>/data`

This prevents app/theme leftovers from one image profile bleeding into another.

## License

MIT
