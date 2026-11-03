#!/usr/bin/env python3
"""Imports Apio's examples into Logix's own examples (examples/).

Run once to bring them in, or again to pick up more from a newer Apio:
    python3 scripts/import-apio-examples.py <apio examples folder> <catalog.json>

<apio examples folder> is Apio's definitions package's examples folder (one
folder per board, one subfolder per example). <catalog.json> is a file with
Logix Engine's boards ({"boards": [{"id": ...}]}, such as
ui/e2e/fixtures/catalog.json): examples for other boards are skipped.

Only examples Logix can open as they are are kept: a top-level main.v or
main.sv, no subfolders, and an apio.ini with one env for the example's
board, top module main, and no settings other than board, top-module and
default-testbench. Each becomes a Logix project: its files without Apio's
apio.ini, info and hidden files, plus a logix-ide.toml with the board and
the description (from Apio's info file). The examples folder is replaced.
"""

import json
import shutil
import sys
from pathlib import Path

SUPPORTED_KEYS = {"board", "top-module", "default-testbench"}
MAIN_FILES = ("main.v", "main.sv")
SKIPPED = {"apio.ini", "info"}

# The check macro of Apio's example testbenches, and what replaces it: a
# one-line macro that reports with $error, as Logix Engine recommends
# (iverilog reports wrong line numbers after a multi-line macro). The
# engine stops Run Tests at the first error and lets Simulate run on, so
# the testbench doesn't check the mode (`APIO_SIM).
APIO_CHECK = """// Checks that 'actual_val' equals 'expected_val'. On a mismatch, prints
// the values and location, sets assertion_err, and stops the simulation.
// Under 'apio sim' the simulation continues so the signals can be viewed.
`define EXPECT_EQ(actual_val, expected_val) \\
  if ((actual_val) !== (expected_val)) begin \\
    $display("*** ASSERTION FAILED at %s:%0d: expected 'h%h, actual 'h%h", \\
             `__FILE__, `__LINE__, (expected_val), (actual_val)); \\
    assertion_err = 1; \\
    if (!`APIO_SIM) $fatal; \\
  end
"""
LOGIX_CHECK = """// Checks that 'actual_val' equals 'expected_val'. On a mismatch, reports
// an error with both values. Run Tests stops at the first error; Simulate
// runs to the end so the signals can be viewed.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected 'h%h, actual 'h%h", (expected_val), (actual_val));
"""
APIO_FLAG = """  // Set to 1 when an assertion fails.
  reg assertion_err = 0;

"""

# The flag's trace in the examples' GTKWave layouts.
APIO_FLAG_TRACE = "@28\n[color] 1\nmain_tb.assertion_err\n"

# Apio's commands in comments, as Logix names them.
WORDING = [
    ("'apio test'", "Run Tests"),
    ("'apio sim'", "Simulate"),
    ("`apio test`", "Run Tests"),
    ("`apio sim`", "Simulate"),
    ("apio test", "Run Tests"),
    ("apio sim", "Simulate"),
]


def ini_is_supported(text: str, board: str) -> bool:
    envs = 0
    for raw in text.splitlines():
        if raw.startswith((" ", "\t")):
            continue  # continues a multi-line value
        line = raw.strip()
        if not line or line.startswith((";", "#")):
            continue
        if line.startswith("[") and line.endswith("]"):
            section = line[1:-1].strip()
            if section.startswith("env:"):
                envs += 1
            elif section != "common":
                return False
            continue
        if "=" not in line:
            return False
        key, value = (s.strip() for s in line.split("=", 1))
        if (
            key not in SUPPORTED_KEYS
            or (key == "board" and value != board)
            or (key == "top-module" and value != "main")
        ):
            return False
    return envs == 1


def is_supported(folder: Path, board: str) -> bool:
    if not any((folder / m).is_file() for m in MAIN_FILES):
        return False
    for entry in folder.iterdir():
        if entry.is_dir() and not entry.name.startswith("."):
            return False
    ini = folder / "apio.ini"
    return not ini.exists() or ini_is_supported(ini.read_text(), board)


def main() -> None:
    source, catalog = Path(sys.argv[1]), Path(sys.argv[2])
    boards = {b["id"] for b in json.loads(catalog.read_text())["boards"]}
    target = Path(__file__).resolve().parent.parent / "examples"
    for old in target.glob("*"):
        if old.is_dir():
            shutil.rmtree(old)
    kept = skipped = 0
    for board_dir in sorted(p for p in source.iterdir() if p.is_dir()):
        board = board_dir.name
        for example in sorted(p for p in board_dir.iterdir() if p.is_dir()):
            if board not in boards or not is_supported(example, board):
                skipped += 1
                continue
            out = target / board / example.name
            out.mkdir(parents=True)
            for f in sorted(example.iterdir()):
                if f.is_dir() or f.name.startswith(".") or f.name in SKIPPED:
                    continue
                data = f.read_bytes()
                try:
                    text = data.decode("utf-8")
                    if APIO_CHECK in text:
                        text = text.replace(APIO_CHECK, LOGIX_CHECK)
                        text = text.replace(APIO_FLAG, "")
                        text = text.replace(APIO_FLAG.replace("reg ", "logic "), "")
                    text = text.replace(APIO_FLAG_TRACE, "")
                    for old, new in WORDING:
                        text = text.replace(old, new)
                    (out / f.name).write_text(text)
                except UnicodeDecodeError:
                    (out / f.name).write_bytes(data)
            info = example / "info"
            description = info.read_text().strip() if info.exists() else ""
            # TOML basic strings take JSON's escapes.
            (out / "logix-ide.toml").write_text(
                "# Logix IDE project settings.\n"
                f"board = {json.dumps(board)}\n"
                f"description = {json.dumps(description, ensure_ascii=False)}\n"
            )
            kept += 1
    print(f"{kept} examples imported, {skipped} skipped")


if __name__ == "__main__":
    main()
