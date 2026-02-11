# waydroid-image-sw

Switch Waydroid between **Android TV** and **Android 13** image sets with one command.

- Fast, minimal, and script-only
- Keeps your image sets organized
- Restarts Waydroid safely

## Quick Start

```bash
# Put your images here
~/waydroid-images/tv/system.img
~/waydroid-images/tv/vendor.img
~/waydroid-images/a13/system.img
~/waydroid-images/a13/vendor.img

# Run
./waydroid-switch tv
./waydroid-switch a13
```

## What It Does

1. Stops Waydroid session/container.
2. Updates `images_path` in `/var/lib/waydroid/waydroid.cfg`.
3. Starts Waydroid again under your user session.

## Requirements

- Waydroid installed
- `sudo` access for `waydroid` + config edit
- Image pairs for TV and A13

## Tips

- If your desktop session doesn’t expose DBus, set:

```bash
export DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$(id -u)/bus"
```

## License

MIT
