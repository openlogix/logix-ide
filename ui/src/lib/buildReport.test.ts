import { describe, expect, it } from "vitest";
import type { BuildReport } from "./backend";
import { percentText, reportRows, reportText, unusedCount } from "./buildReport";

// As Logix Engine's build gives them (resources sorted by name).
const ice40: BuildReport = {
  family: "ice40",
  resources: [
    { name: "ICESTORM_LC", description: "Logic cell: a 4-input LUT, a flip-flop and carry logic", used: 27, available: 7680, percent: 0.4 },
    { name: "ICESTORM_RAM", description: "Block RAM, 4 Kbit", used: 0, available: 32, percent: 0 },
    { name: "NEW_THING", description: null, used: 3, available: 0, percent: null },
    { name: "SB_IO", description: "I/O pin", used: 9, available: 107, percent: 8.4 },
  ],
  clocks: [{ name: "sys_clk", net: "sys_clk$SB_IO_IN_$glb_clk", fmaxMhz: 194.3634 }],
  text: "The design uses 27 of 7680 logic cells (0.4%).",
  summaryLine: "Uses 0.4% of logic cells, max clock 194.4 MHz.",
};

describe("build report window", () => {
  it("shows the used resources by default, all on request", () => {
    expect(reportRows(ice40, false, "").map((r) => r.name)).toEqual(["ICESTORM_LC", "NEW_THING", "SB_IO"]);
    expect(reportRows(ice40, true, "")).toHaveLength(4);
    expect(unusedCount(ice40)).toBe(1);
  });

  it("filters by name or description, ignoring case", () => {
    expect(reportRows(ice40, true, "ram").map((r) => r.name)).toEqual(["ICESTORM_RAM"]);
    expect(reportRows(ice40, true, "icestorm_lc").map((r) => r.name)).toEqual(["ICESTORM_LC"]);
    expect(reportRows(ice40, true, "pin").map((r) => r.name)).toEqual(["SB_IO"]);
    // Unused rows stay hidden while filtering, unless shown.
    expect(reportRows(ice40, false, "ram")).toEqual([]);
  });

  it("formats shares, n/a for numbers the engine found invalid", () => {
    const r = (used: number, percent: number | null) => ({
      name: "X", description: null, used, available: 100, percent,
    });
    expect(percentText(r(0, 0))).toBe("");
    expect(percentText(r(0, null))).toBe("");
    expect(percentText(r(1, 0))).toBe("<0.1%");
    expect(percentText(r(1, 0.4))).toBe("0.4%");
    expect(percentText(r(1, 33.3))).toBe("33%");
    expect(percentText(r(5, null))).toBe("n/a");
  });

  it("copies as text columns", () => {
    const rows = reportRows(ice40, false, "");
    expect(reportText("Board Utilization: blinky", rows, ice40)).toBe(
      [
        "Board Utilization: blinky",
        "",
        "Resource     Description                                             Used  Available     %",
        "ICESTORM_LC  Logic cell: a 4-input LUT, a flip-flop and carry logic    27       7680  0.4%",
        "NEW_THING                                                               3          0   n/a",
        "SB_IO        I/O pin                                                    9        107    8%",
        "",
        "Clock    Max speed  Net",
        "sys_clk  194.4 MHz  sys_clk$SB_IO_IN_$glb_clk",
        "",
      ].join("\n"),
    );
  });
});
