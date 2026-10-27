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
    expect(outcomeText("build", { ok: true, diagnostics: [], report: null, cancelled: false, graphSvg: null, testbenches: [], seconds: 1.62 })).toBe(
      "Build succeeded in 1.6 s.",
    );
    expect(
      outcomeText("build", { ok: true, diagnostics: [warn("a.pcf", 3)], report: null, cancelled: false, graphSvg: null, testbenches: [], seconds: 2 }),
    ).toBe("Build succeeded in 2.0 s, 1 warning.");
  });

  it("reports the first error on failure", () => {
    expect(
      outcomeText("build", {
        ok: false,
        diagnostics: [warn("pinout.pcf", 6), err("main.v", 25, "syntax error")],
        report: null, cancelled: false, graphSvg: null, testbenches: [],
        seconds: 0.4,
      }),
    ).toBe("Build failed: main.v:25: syntax error");
    expect(
      outcomeText("upload", {
        ok: false,
        diagnostics: [err(null, null, "No matching USB device.")],
        report: null, cancelled: false, graphSvg: null, testbenches: [],
        seconds: 1,
      }),
    ).toMatch(/^Upload failed: Logix couldn't find the board/);
    expect(outcomeText("upload", { ok: false, diagnostics: [], report: null, cancelled: false, graphSvg: null, testbenches: [], seconds: 1 })).toBe(
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
      outcomeText("test", { ok: true, diagnostics: [], report: null, cancelled: false, graphSvg: null, testbenches: [benches[0]], seconds: 4.3 }),
    ).toBe("Tests passed (1 testbench) in 4.3 s.");
  });
  it("names the first failure", () => {
    expect(
      outcomeText("test", {
        ok: false,
        diagnostics: [err("alu_tb.v", 52, "Assertion failed: expected 'h01, actual 'h00")],
        report: null, cancelled: false, graphSvg: null, testbenches: benches,
        seconds: 1,
      }),
    ).toBe("Test failed: alu_tb.v:52: Assertion failed: expected 'h01, actual 'h00");
  });
  it("lists each testbench", () => {
    expect(
      testbenchLines({ ok: false, diagnostics: [], report: null, cancelled: false, graphSvg: null, testbenches: benches, seconds: 1 }),
    ).toEqual(["✓ main_tb.v passed", "✗ alu_tb.v failed"]);
  });
  it("mentions GTKWave after simulating", () => {
    expect(
      outcomeText("simulate", { ok: true, diagnostics: [], report: null, cancelled: false, graphSvg: null, testbenches: [], seconds: 0.3 }),
    ).toBe("Simulation done in 0.3 s. GTKWave shows the signals.");
  });
});

describe("build report in the status", () => {
  it("adds a usage summary after a successful build", () => {
    expect(
      outcomeText("build", {
        ok: true,
        diagnostics: [],
        testbenches: [],
        cancelled: false, graphSvg: null,
        report: {
          arch: "ice40",
          resources: [{ name: "ICESTORM_LC", used: 27, available: 7680 }],
          clocks: [{ name: "sys_clk", fmaxMhz: 194.36 }],
        },
        seconds: 2.4,
      }),
    ).toBe("Build succeeded in 2.4 s. Uses 0.4% of logic cells, max clock 194.4 MHz.");
  });
});

describe("friendly messages", () => {
  it("explains a missing pin file", () => {
    expect(
      outcomeText("build", {
        ok: false,
        diagnostics: [err(null, null, "No constraint file '*.pcf' found.")],
        testbenches: [],
        report: null,
        cancelled: false, graphSvg: null,
        seconds: 0.2,
      }),
    ).toMatch(/^Build failed: The project has no pin file/);
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
        cancelled: true, graphSvg: null,
        seconds: 3,
      }),
    ).toBe("Upload stopped.");
  });
});

describe("board not found", () => {
  it("explains it in plain words", () => {
    expect(
      outcomeText("upload", {
        ok: false,
        diagnostics: [err(null, null, "No matching USB device.")],
        testbenches: [],
        report: null,
        cancelled: false,
        graphSvg: null,
        seconds: 1,
      }),
    ).toMatch(/^Upload failed: Logix couldn't find the board on USB/);
  });
});
