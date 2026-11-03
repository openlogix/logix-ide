# Examples

The examples Logix IDE offers in File > Examples, built into the app. Each
folder `<board>/<name>/` is a Logix project: its files and a `logix-ide.toml`
with the board and a one-line description.

They come from [Apio](https://github.com/FPGAwars/apio)'s examples
(apio-definitions, release `2026-10-26`), imported with
`scripts/import-apio-examples.py` and adapted to Logix (no `apio.ini`;
testbenches report failures with `$error`). They're licensed under the
Apache License 2.0, like the rest of this repository.
