# waydroid-switch

Universal Waydroid image switcher (single TUI binary).

`waydroid-switch` auto-scans `~/waydroid-images` for any profile folders containing `system.img` and `vendor.img`, shows the current active `images_path`, and switches profiles safely.

## Features

- Single TUI binary: `waydroid-switch`
- Auto-searches `~/waydroid-images` recursively
- Supports linked images (symlinks)
- Shows current active `images_path`
- Full profile switch: image + userdata + overlay
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
  - Also switches Waydroid userdata and overlay to profile-specific directories
- `a`: manual add submenu
- `r`: refresh auto-scan list
- `q`: quit

## Requirements

- Waydroid installed
- `sudo` access (for updating `/var/lib/waydroid/waydroid.cfg` and stopping/starting session)
- Image profiles under `~/waydroid-images`

## Data Isolation

On switch, the app links live Waydroid userdata and overlay to profile-specific paths:

- Live path: `~/.local/share/waydroid/data`
- Profile store: `~/.local/share/waydroid/profiles/<profile-id>/data`
- Live overlays: `/var/lib/waydroid/overlay_rw` and `/var/lib/waydroid/overlay_work`
- Profile overlays: `~/.local/share/waydroid/profiles/<profile-id>/overlay_rw` and `.../overlay_work`

This prevents app/theme/root leftovers from one image profile bleeding into another.

## License

MIT
