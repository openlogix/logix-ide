# Standard examples: the AI spec

Every catalog board gets the same standard examples, such as `bare-minimum`
and `blinky`. They aren't written by hand or by a code generator: an AI
agent writes them from this spec and the board's definition in Logix
Engine's catalog. Each file here says what one example is; the AI applies
it to each board.

| File | What it holds |
| --- | --- |
| `README.md` | This file: how examples are made, checked and kept up to date |
| `COMMON.md` | What every standard example shares: files, board data, pin files, style, testbenches |
| `<example>.md` | One standard example each, named like its folder: [`bare-minimum.md`](bare-minimum.md), [`blinky.md`](blinky.md) |
| `CATALOG.json` | The boards the examples were last written from (see When to run) |

Shared files have capital names, examples lowercase ones. Since macOS and
Windows ignore case in file names, no example may be named `readme`, `common`
or `catalog`.

## Inputs: the spec and the board, nothing else

For each board, the AI reads only:

1. This folder: `COMMON.md` and the example's file.
2. The board's entry in Logix Engine's catalog, with its FPGA and FPGA
   family: `scripts/board-info.sh <board id>` prints them (the engine's
   `get_catalog`; Open Logix's tools read the catalog only through the engine).
   This includes the board's and its resources' `ai_hints`.
3. General knowledge of FPGAs, Verilog and the toolchains, as an engineer
   with the FPGA's datasheet would have.

Never the existing examples (generated or hand-written, of any board), old
versions of them, or code that made them. When the spec and the board's
data aren't enough to write a correct example, stop and say what's
missing: the fix goes into the catalog (a field, or an `ai_hints` line,
following Logix Catalog's `docs/model.md`) or into this spec, never into
one board's example.

## Output

`examples/<board id>/<example>/`, replaced as a whole: delete the folder,
then write it anew. A board an example doesn't apply to (`blinky` on a
board without LEDs) gets no folder for it, and an old one is deleted.
Hand-written examples are never touched, including one with a standard
example's name on a board that example doesn't apply to (the Tang Primer
20K Lite's `blinky`, on a PMOD, since the board has no LED).

## Checks

Every example must build and pass its testbench on the real engine:

```
LOGIX_DEV_ENGINE_BINARY=<logix-engine> LOGIX_IDE_TEST_ENGINE_FOLDER=<engine folder with packages> \
  cargo test --manifest-path src-tauri/Cargo.toml real_examples_build -- --ignored --nocapture
```

Fix failures in the example (or, when the cause is the spec or the board's
data, there), then check again. Review the diff before committing: a
change in a board that wasn't meant to change is a mistake.

## When to run

- **A board's definition changed** (a new catalog in the engine pin): that
  board's examples. [`CATALOG.json`](CATALOG.json), here, is the
  `board-info.sh` output for every board the examples were written from,
  with the catalog version (`get_engine_info` `catalog`), so a later run
  can tell which boards changed: compare it with a new `board-info.sh`
  output, then write it anew. (Not in `examples/`, which the app embeds.)
- **A file here changed:** that example on every board (`COMMON.md`: all).
- **A new board:** all its examples.

## Prompt

Run in Claude Code (or another agent) in this repo, with a local engine:

> Write the standard examples `<examples, or all>` for the boards
> `<board IDs, or all>`, following `spec/examples/README.md`. Read only
> what it lists as inputs. Get each board with `scripts/board-info.sh`
> (`LOGIX_DEV_ENGINE_BINARY=<engine>`). Then run the checks, fix, and
> update `spec/examples/CATALOG.json`.
