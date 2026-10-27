import { describe, expect, it } from "vitest";
import type { BuildReport } from "./backend";
import { percent, reportSummary, usages } from "./buildReport";

const ice40: BuildReport = {
  arch: "ice40",
  resources: [
    { name: "ICESTORM_LC", used: 27, available: 7680 },
    { name: "ICESTORM_RAM", used: 0, available: 32 },
    { name: "SB_GB", used: 1, available: 8 },
    { name: "SB_IO", used: 9, available: 107 },
  ],
  clocks: [{ name: "sys_clk", fmaxMhz: 194.3634 }],
};

describe("build report", () => {
  it("lists the main resource and the used ones in plain words", () => {
    expect(usages(ice40).map((u) => u.label)).toEqual(["logic cells", "I/O pins"]);
  });

  it("always includes the main resource, even when unused", () => {
    const empty = { ...ice40, resources: [{ name: "ICESTORM_LC", used: 0, available: 7680 }] };
    expect(usages(empty)).toEqual([{ label: "logic cells", used: 0, available: 7680 }]);
  });

  it("adds up resources with several names", () => {
    const xilinx: BuildReport = {
      arch: "xilinx",
      resources: [
        { name: "SLICE_LUTX", used: 56, available: 65200 },
        { name: "RAMB36E1", used: 1, available: 75 },
        { name: "RAMB18E1", used: 2, available: 150 },
      ],
      clocks: [],
    };
    expect(usages(xilinx)).toEqual([
      { label: "LUTs", used: 56, available: 65200 },
      { label: "RAM blocks", used: 3, available: 225 },
    ]);
  });

  it("uses Apio's names for unknown families", () => {
    const other: BuildReport = {
      arch: "newfpga",
      resources: [
        { name: "CELL", used: 5, available: 100 },
        { name: "RAM", used: 0, available: 4 },
      ],
      clocks: [],
    };
    expect(usages(other)).toEqual([{ label: "CELL", used: 5, available: 100 }]);
    expect(reportSummary(other)).toBe("5% of CELL");
  });

  it("summarizes for the status bar", () => {
    const twoClocks = {
      ...ice40,
      clocks: [
        { name: "a", fmaxMhz: 200 },
        { name: "b", fmaxMhz: 90.04 },
      ],
    };
    expect(reportSummary(twoClocks)).toBe("0.4% of logic cells, max clock 90.0 MHz");
  });

  it("formats percentages", () => {
    expect(percent(0, 10)).toBe("0%");
    expect(percent(1, 300)).toBe("0.3%");
    expect(percent(1, 3)).toBe("33%");
    expect(percent(1, 0)).toBe("?%");
  });
});
