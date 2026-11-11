import { describe, expect, it } from "vitest";
import type { Diagnostic } from "./backend";
import { byFile, firstLocated, outcomeText, testbenchLines } from "./tasks";

const err = (file: string | null, line: number | null, message: string): Diagnostic => ({
  severity: "error",
  file,
  line,
  column: null,
  message,
});
const warn = (file: string, line: number): Diagnostic => ({
  ...err(file, line, "w"),
  severity: "warning",
});

describe("outcomeText", () => {
  it("reports success with time and warnings", () => {
    expect(outcomeText("build", { ok: true, diagnostics: [], report: null, cancelled: false, testbenches: [], seconds: 1.62, action: null, devices: [] })).toBe(
      "Build succeeded in 1.6 s.",
    );
    expect(
      outcomeText("build", { ok: true, diagnostics: [warn("a.pcf", 3)], report: null, cancelled: false, testbenches: [], seconds: 2, action: null, devices: [] }),
    ).toBe("Build succeeded in 2.0 s, 1 warning.");
  });

  it("reports the first error on failure", () => {
    expect(
      outcomeText("build", {
        ok: false,
        diagnostics: [warn("pinout.pcf", 6), err("main.v", 25, "syntax error")],
        report: null, cancelled: false, testbenches: [],
        seconds: 0.4,
        action: null,
        devices: [],
      }),
    ).toBe("Build failed.");
    expect(
      outcomeText("upload", {
        ok: false,
        diagnostics: [err(null, null, "No Alhambra II is connected. Connect the Alhambra II by USB, then retry.")],
        report: null, cancelled: false, testbenches: [],
        seconds: 1,
        action: "connect_board",
        devices: [],
      }),
    ).toBe("Upload failed: No Alhambra II is connected. Connect the Alhambra II by USB, then retry.");
    // A board file: the output lists its problems, so only the folder.
    expect(
      outcomeText("build", {
        ok: false,
        diagnostics: [err("boards/mine.json", 4, "Expected `,` or `}`.")],
        report: null, cancelled: false, testbenches: [],
        seconds: 0,
        action: null,
        devices: [],
      }),
    ).toBe("Build failed: the project's boards folder has problems.");
    // In a saved project, the engine names it by its full path.
    expect(
      outcomeText("build", {
        ok: false,
        diagnostics: [err("/Users/you/blinky 3/boards/mine.json", 4, "Expected `,` or `}`.")],
        report: null, cancelled: false, testbenches: [],
        seconds: 0,
        action: null,
        devices: [],
      }),
    ).toBe("Build failed: the project's boards folder has problems.");
    expect(outcomeText("upload", { ok: false, diagnostics: [], report: null, cancelled: false, testbenches: [], seconds: 1, action: null, devices: [] })).toBe(
      "Upload failed. See the output below.",
    );
  });
});

describe("firstLocated / byFile", () => {
  const diags = [err(null, null, "x"), warn("pinout.pcf", 6), err("main.v", 3, "y")];
  it("prefers errors that point into a file", () => {
    expect(firstLocated(diags)?.file).toBe("main.v");
    expect(firstLocated([warn("p.pcf", 1)])?.file).toBe("p.pcf");
    expect(firstLocated([err(null, null, "x")])).toBeNull();
  });
  it("groups by file", () => {
    const map = byFile(diags);
    expect([...map.keys()]).toEqual(["pinout.pcf", "main.v"]);
  });
});

describe("test results", () => {
  const benches = [
    { name: "main_tb.v", passed: true },
    { name: "alu_tb.v", passed: false },
  ];
  it("summarizes passing tests", () => {
    expect(
      outcomeText("test", { ok: true, diagnostics: [], report: null, cancelled: false, testbenches: [benches[0]], seconds: 4.3, action: null, devices: [] }),
    ).toBe("Tests passed (1 testbench) in 4.3 s.");
  });
  it("names the first failure", () => {
    expect(
      outcomeText("test", {
        ok: false,
        diagnostics: [err("alu_tb.v", 52, "Assertion failed: expected 'h01, actual 'h00")],
        report: null, cancelled: false, testbenches: benches,
        seconds: 1,
        action: null,
        devices: [],
      }),
    ).toBe("Test failed.");
  });
  it("lists each testbench", () => {
    expect(
      testbenchLines({ ok: false, diagnostics: [], report: null, cancelled: false, testbenches: benches, seconds: 1, action: null, devices: [] }),
    ).toEqual(["✓ main_tb.v passed", "✗ alu_tb.v failed"]);
  });
  it("mentions GTKWave after simulating", () => {
    expect(
      outcomeText("simulate", { ok: true, diagnostics: [], report: null, cancelled: false, testbenches: [], seconds: 0.3, action: null, devices: [] }),
    ).toBe("Simulation done in 0.3 s. GTKWave shows the signals.");
  });
  it("says GTKWave shows a failing testbench too", () => {
    const failure = { severity: "error" as const, file: "main_tb.v", line: 52, column: null, message: "Assertion failed" };
    const outcome = { ok: false, diagnostics: [failure], report: null, cancelled: false, seconds: 0.3, action: null, devices: [] };
    expect(
      outcomeText("simulate", { ...outcome, testbenches: [{ name: "main_tb.v", passed: false }] }),
    ).toBe("The testbench failed. GTKWave shows the signals.");
    // A compile error: nothing was simulated.
    expect(outcomeText("simulate", { ...outcome, testbenches: [] })).toBe("Simulation failed.");
    // An error without a location is shown only here, so it stays.
    const unlocated = { ...failure, file: null, line: null, message: "vvp stopped." };
    expect(
      outcomeText("simulate", { ...outcome, diagnostics: [unlocated], testbenches: [{ name: "main_tb.v", passed: false }] }),
    ).toBe("The testbench failed (GTKWave shows the signals): vvp stopped.");
  });
});

describe("build report in the status", () => {
  it("adds a usage summary after a successful build", () => {
    expect(
      outcomeText("build", {
        ok: true,
        diagnostics: [],
        testbenches: [],
        cancelled: false,
        report: {
          family: "ice40",
          resources: [],
          clocks: [],
          text: "",
          // The engine's one-line report, shown as it words it.
          summaryLine: "Uses 0.4% of logic cells, max clock 194.4 MHz.",
        },
        seconds: 2.4,
        action: null,
        devices: [],
      }),
    ).toBe("Build succeeded in 2.4 s. Uses 0.4% of logic cells, max clock 194.4 MHz.");
  });
});

describe("stopped tasks", () => {
  it("says the task was stopped", () => {
    expect(
      outcomeText("upload", {
        ok: false,
        diagnostics: [err(null, null, "x")],
        testbenches: [],
        report: null,
        cancelled: true,
        seconds: 3,
        action: null,
        devices: [],
      }),
    ).toBe("Upload stopped.");
  });
});
