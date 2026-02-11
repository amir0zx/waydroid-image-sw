# waydroid-image-sw

Simple script to switch Waydroid images between Android TV and Android 13 image sets.

## Usage

```bash
./waydroid-switch tv
./waydroid-switch a13
```

## Notes

- Expects images to live in `~/waydroid-images/tv` and `~/waydroid-images/a13`.
- Edits `/var/lib/waydroid/waydroid.cfg` and restarts the session.
- Requires `sudo` for Waydroid stop/start and config update.
