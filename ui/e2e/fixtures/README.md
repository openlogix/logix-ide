Real tool output used by the fake backend (`../fakeBackend.ts`), so the
end-to-end tests and the documentation screenshots show what Logix IDE really
shows. Captured for the fake backend's blinky design (`BLINKY_MAIN`,
`BLINKY_TB`, pins for the Alhambra II), with scratch paths replaced by
Logix IDE's usual locations.

- `verify.txt`: Verify with Logix Engine `2026-11-03` (`lint`, then `build`), as the console shows its live output: command lines dimmed with `$`, errors red, warnings yellow.
- `verify-error.txt`: the same with the `;` missing at the end of `assign led = counter[23];`.
- `test.txt`: Run Tests (the engine's `test`).
- `diagram.txt`, `diagram.html`: Show Diagram (the engine's `diagram`): its output and the page the Diagram window shows (also the docs' diagram screenshot).
- `fpgas.json`: the FPGAs as `get_fpga_families` gives them (Logix Engine's `get_catalog` `fpgas` and `families`: each family's `id`, `label` and `fpgas`, by part number, in the engine's order). Update it with the engine's catalog, like `catalog.json`.
- `catalog.json`: the boards as `get_boards` gives them (Logix Engine's `get_catalog`, with each FPGA's family, pin file kind and `fpga` (`id`, `partNumber`, `size`), and each board's variants as `dimensions` and `configurations` (`when`, `label`, `tested`, `userNotes`), sorted by description) and the examples as `get_examples` gives them (from `examples/*/*/logix-ide.toml`, sorted by board and name). `capture_fixtures` doesn't write it: update it by hand (or a short script) when the engine's boards or the examples change.

To recapture the engine's output and diagram (all but `catalog.json`), from `src-tauri/`:
`LOGIX_DEV_ENGINE_BINARY=~/.logix/ide/engine-release/logix-engine LOGIX_IDE_TEST_ENGINE_FOLDER=~/.logix/ide/engine LOGIX_IDE_TEST_FIXTURES_OUT=../ui/e2e/fixtures cargo test capture_fixtures -- --ignored`
(`capture_fixtures` in `src-tauri/src/engine/task.rs`; it uses the same blinky).
