Real tool output used by the fake backend (`../fakeBackend.ts`), so the
end-to-end tests and the documentation screenshots show what Logix really
shows. Captured for the fake backend's blinky design (`BLINKY_MAIN`,
`BLINKY_TB`, pins for the Alhambra II), with scratch paths replaced by
Logix's usual locations.

- `verify.txt`: Verify with Logix Engine `2026-11-03` (`lint`, then `build`), as the console shows its live output: command lines dimmed with `$`, errors red, warnings yellow.
- `verify-error.txt`: the same with the `;` missing at the end of `assign led = counter[23];`.
- `test.txt`: `apio test` (Apio `2026-10-22`).
- `graph.svg`: `apio graph`.
- `catalog.json`: the boards (Logix Engine's `get_catalog`, with each FPGA's architecture) and the examples Logix supports (from `apio api get-examples`, only for those boards).
