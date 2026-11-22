Real Logix Engine output and data used by the fake backend (`../fakeBackend.ts`),
so the end-to-end tests and the documentation screenshots show what Logix IDE
really shows. Captured for the iCESugar v1.5's Blinky, as Logix IDE ships it
(`examples/icesugar-1-5/blinky`, the fake backend's default board and
project), with scratch paths replaced by Logix IDE's usual locations.

The console's output is kept as the backend streams it: a JSON list of
`TaskEvent` lines (`{"kind": "line", "line", "tool"?, "redraw"?, "location"?}`),
command lines dimmed with `$`, errors red, warnings yellow, and each tool line
with the place in a project file Logix Engine found in it (`location`), which
the console makes a link.

- `verify.json`: Verify (`build` with `lint`).
- `verify-report.json`: that build's report, as Board Utilization shows it.
- `verify-error.json`: the same with the `;` missing at the end of `reg [N-1:0] counter = 0;`.
- `test.json`: Run Tests (the engine's `test`).
- `diagram.json`, `diagram.html`: Show Diagram (the engine's `diagram`): its output and the page the Diagram window shows (also the docs' diagram screenshot).
- `simulate.json`, `waveforms.html`: Simulate (the engine's `simulate` on `main_tb.v`): its output and the waveform viewer's page the Waveforms window shows (also the docs' waveforms screenshot).
- `catalog.json`: the boards as `get_boards` gives them (Logix Engine's `get_catalog`: each board's FPGA family, pin file kind and comment, `fpga` with its `label`, variants as `dimensions` and `configurations`, `resources` with the engine's `type_label`, `pins_text` and `notes_text`, `requires` with their `text`, sorted by description) and the examples as `get_examples` gives them (from `examples/*/*/logix-ide.toml`, sorted by board and name).
- `fpgas.json`: the FPGAs as `get_fpga_families` gives them (Logix Engine's `get_catalog` `fpgas` and `families`: each family's `id`, `label`, `pinExtension`, `pinComment` and `fpgas`, by part number, in the engine's order).

To recapture them all, from `src-tauri/`:
`LOGIX_DEV_ENGINE_BINARY=<logix-engine> LOGIX_IDE_TEST_ENGINE_FOLDER=<engine folder with packages> LOGIX_IDE_TEST_FIXTURES_OUT=../ui/e2e/fixtures cargo test capture_fixtures -- --ignored`
(`capture_fixtures` in `src-tauri/src/engine/task.rs`; it uses the same blinky).
Recapture them when the engine's output, its catalog or the examples change.
