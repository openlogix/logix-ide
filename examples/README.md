# Examples

The examples Logix IDE offers in File > Examples. They reach users as the
`logix-ide-examples` package: each pre-release packs this folder
(`scripts/pack-examples.py`), and Logix IDE downloads the release its remote
config names (`remote-config/logix-ide/`), so a fixed or new example reaches
installed Logix IDEs without a new Logix IDE release. Each
folder `<board>/<name>/` is a Logix IDE project: its files and a `logix-ide.toml`
with the board and a one-line description.

Every board's `blinky` and `bare-minimum` are standard examples, written
by an AI from the spec in [`spec/examples/`](../spec/examples/README.md) and
the board's entry in Logix Engine's catalog: change the spec or the catalog,
not these files. The other examples are maintained here, edited in
place. They're licensed under
MIT or the Apache License 2.0, at your option, like the rest of this
repository.
