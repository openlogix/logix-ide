#!/bin/bash
# Gives a built .dmg its own volume icon (src-tauri/icons/dmg-volume.icns:
# the app icon with a drive badge, the user's choice, 2026-11-21), so the
# mounted disk isn't taken for the app. Tauri's bundler always uses the
# app icon and has no setting for this, so the release build swaps it
# after bundling: a writable copy, the icon replaced, compressed again.
#
#     scripts/set-dmg-volume-icon.sh <the .dmg> [<icon.icns>]
#
# Used by the release workflow only (.github/workflows/build-darwin-arm64.yaml);
# installers are never made on a developer machine.
set -euo pipefail
dmg="$1"
icon="${2:-$(cd "$(dirname "$0")/.." && pwd)/src-tauri/icons/dmg-volume.icns}"
work="$(mktemp -d)"
trap 'hdiutil detach "$work/mnt" -force >/dev/null 2>&1 || true; rm -rf "$work"' EXIT
hdiutil convert "$dmg" -format UDRW -o "$work/rw.dmg" >/dev/null
mkdir "$work/mnt"
hdiutil attach "$work/rw.dmg" -nobrowse -mountpoint "$work/mnt" >/dev/null
cp "$icon" "$work/mnt/.VolumeIcon.icns"
# The volume's "has a custom icon" flag; the bundler sets it already.
if command -v SetFile >/dev/null; then SetFile -a C "$work/mnt"; fi
hdiutil detach "$work/mnt" >/dev/null
hdiutil convert "$work/rw.dmg" -format UDZO -imagekey zlib-level=9 -o "$work/out.dmg" >/dev/null
mv "$work/out.dmg" "$dmg"
echo "Set the volume icon of $dmg"
