#!/usr/bin/env python3
"""Packs Logix IDE's examples as a release file, logix-ide-examples-<yyyymmdd>.tgz.

    scripts/pack-examples.py <output .tgz>

The file holds one top-level folder, logix-ide-examples/, with the repo's
examples/ folder (examples/<board>/<name>/, its README.md) and the licenses.
Logix IDE downloads it as the dependency its remote config names
(logix-ide-examples). Reproducible: the same files give the same bytes
(sorted entries, fixed times, owners and modes, no gzip time stamp), so a
day's pre-release rebuilt from the same commit keeps its SHA-256 and
installed Logix IDEs don't download it again.
"""

import gzip
import io
import sys
import tarfile
from pathlib import Path

TOP = "logix-ide-examples"


def entries(repo: Path):
    """(path in the archive, file on disk), sorted, folders included."""
    out = []
    examples = repo / "examples"
    for p in sorted(examples.rglob("*")):
        if p.name == ".DS_Store":
            continue
        out.append((f"{TOP}/{p.relative_to(examples).as_posix()}", p))
    for name in ("LICENSE-MIT", "LICENSE-APACHE"):
        out.append((f"{TOP}/{name}", repo / name))
    return [(f"{TOP}", examples)] + sorted(out)


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    repo = Path(__file__).resolve().parent.parent
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w", format=tarfile.PAX_FORMAT) as tar:
        for arcname, path in entries(repo):
            info = tarfile.TarInfo(arcname)
            info.mtime = 0
            info.uid = info.gid = 0
            info.uname = info.gname = ""
            if path.is_dir():
                info.type = tarfile.DIRTYPE
                info.mode = 0o755
                tar.addfile(info)
            else:
                data = path.read_bytes()
                info.mode = 0o644
                info.size = len(data)
                tar.addfile(info, io.BytesIO(data))
    out = Path(sys.argv[1])
    with open(out, "wb") as f:
        with gzip.GzipFile(filename="", mode="wb", fileobj=f, mtime=0, compresslevel=9) as gz:
            gz.write(buf.getvalue())
    print(f"{out}: {len(entries(repo))} entries")


if __name__ == "__main__":
    main()
