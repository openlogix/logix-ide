import { describe, expect, it } from "vitest";
import { Redraws } from "./redraw";

describe("Redraws", () => {
  it("replaces a tool's redraw line with its next line", () => {
    // iceprog's progress, as the engine sends it.
    const r = new Redraws();
    expect(r.toolLine("programming..", "iceprog", false)).toEqual({
      replace: false,
      final: ["programming.."],
    });
    expect(r.toolLine("addr 0x000000   0%", "iceprog", true)).toEqual({
      replace: false,
      final: [],
    });
    expect(r.toolLine("addr 0x000100   1%", "iceprog", true)).toEqual({
      replace: true,
      final: [],
    });
    // The last state is printed over too, as in a terminal.
    expect(r.toolLine("done.", "iceprog", false)).toEqual({
      replace: true,
      final: ["done."],
    });
  });

  it("keeps a redraw line when something else comes next", () => {
    const r = new Redraws();
    r.toolLine("Writing: [===] 40%", "openFPGALoader", true);
    expect(r.toolLine("x", "yosys", false)).toEqual({
      replace: false,
      final: ["Writing: [===] 40%", "x"],
    });
    r.toolLine("addr 0x000100   1%", "iceprog", true);
    expect(r.end()).toEqual(["addr 0x000100   1%"]);
    // After the end, the next line of the same tool is a new line.
    expect(r.toolLine("done.", "iceprog", false).replace).toBe(false);
  });
});
