Real tool output used by the fake backend (`../fakeBackend.ts`), so the
end-to-end tests and the documentation screenshots show what Logix really
shows. Captured for the fake backend's blinky design (`BLINKY_MAIN`,
`BLINKY_TB`, pins for the Alhambra II), with scratch paths replaced by
Logix's usual locations.

- `verify.txt`: Verify with Logix Engine `2026-11-03` (`lint`, then `build`), as the console shows its live output: command lines dimmed with `$`, errors red, warnings yellow.
- `verify-error.txt`: the same with the `;` missing at the end of `assign led = counter[23];`.
- `test.txt`: Run Tests (the engine's `test`).
- `graph.txt`, `graph.svg`: Show Diagram (the engine's `graph`): its output and the diagram.
- `catalog.json`: the boards (Logix Engine's `get_catalog`, with each FPGA's architecture) and the examples Logix supports (from `apio api get-examples`, only for those boards).

To recapture the engine's output and diagram (all but `catalog.json`), from `src-tauri/`:
`LOGIX_TEST_ENGINE_EXE=~/.logix/engine-bin/logix-engine LOGIX_TEST_ENGINE_FOLDER=~/.logix/engine LOGIX_FIXTURES_OUT=../ui/e2e/fixtures cargo test capture_fixtures -- --ignored`
(`capture_fixtures` in `src-tauri/src/engine/task.rs`; it uses the same blinky).
