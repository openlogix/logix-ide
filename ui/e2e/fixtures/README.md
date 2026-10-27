Real Apio output used by the fake backend (`../fakeBackend.ts`), so the
end-to-end tests and the documentation screenshots show what Logix really
shows. Captured with Apio `2026-10-22` for the fake backend's blinky design
(`BLINKY_MAIN`, `BLINKY_TB`, pins for the Alhambra II), with scratch paths
replaced by Logix's usual locations.

- `verify.txt`: `apio lint`, `apio build` and `apio report`, as Verify shows them.
- `test.txt`: `apio test`.
- `graph.svg`: `apio graph`.
- `catalog.json`: the boards and examples (the ones Logix supports) from `apio api get-boards` / `get-examples`.
- `verify-error.txt`: Verify with the `;` missing at the end of `assign led = counter[23];`.
