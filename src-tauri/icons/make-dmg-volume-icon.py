#!/usr/bin/env python3
"""Makes dmg-volume.icns, the mounted disk image's icon, from
dmg-volume.svg: option C, the Logix IDE icon with a small drive badge (the
user's choice, 2026-11-21), so the disk on the desktop isn't taken for the
app. Run it on a Mac (sips, iconutil) when the SVG changes, and commit the
result; the build only copies it (scripts/set-dmg-volume-icon.sh).
The drawing fills 824 of 1024 px, Apple's icon grid, as the app icon."""

import subprocess
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent


def main() -> None:
    svg = (HERE / "dmg-volume.svg").read_text()
    inner = svg.split(">", 1)[1].rsplit("</svg>", 1)[0]
    with tempfile.TemporaryDirectory() as t:
        tmp = Path(t)
        mac = tmp / "mac.svg"
        mac.write_text('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">'
                       f'<g transform="translate(100 100) scale({824 / 64})">{inner}</g></svg>\n')
        iconset = tmp / "icon.iconset"
        iconset.mkdir()
        for size in [16, 32, 128, 256, 512]:
            for scale, suffix in [(1, ""), (2, "@2x")]:
                out = iconset / f"icon_{size}x{size}{suffix}.png"
                subprocess.run(["sips", "-s", "format", "png", "-z", str(size * scale), str(size * scale),
                                str(mac), "--out", str(out)], check=True, capture_output=True)
        subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(HERE / "dmg-volume.icns")], check=True)
    print("dmg-volume.icns")


if __name__ == "__main__":
    main()
