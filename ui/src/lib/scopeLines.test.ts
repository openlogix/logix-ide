import { describe, expect, it } from "vitest";
import { indentOf, scopeColumns } from "./scopeLines";

/** The indentation of each line of `text` (null when blank). */
const indents = (text: string) => text.split("\n").map((l) => indentOf(l));

describe("scopeColumns", () => {
  it("draws a line under each block's opener", () => {
    const text = [
      "module main;",
      "  always @(posedge clk) begin",
      "    if (a) begin",
      "      b <= 1;",
      "    end",
      "  end",
      "endmodule",
    ].join("\n");
    expect(scopeColumns(indents(text))).toEqual([[], [0], [0, 2], [0, 2, 4], [0, 2], [0], []]);
  });

  it("adds no line for continuation lines indented further", () => {
    // The user's report, 2026-11-21: a long `if` condition continued on
    // deeper lines drew a line at their column, between `if` and `uart`.
    const text = [
      "generate",
      "  if (a",
      "      || b",
      "      || c) begin : bad",
      "    uart_too_far_off check ();",
      "  end",
      "endgenerate",
    ].join("\n");
    expect(scopeColumns(indents(text))).toEqual([[], [0], [0, 2], [0, 2], [0, 2], [0], []]);
  });

  it("runs a block's line through blank lines up to its closer", () => {
    const text = ["module main;", "  reg a;", "", "  reg b;", "", "endmodule"].join("\n");
    expect(scopeColumns(indents(text))).toEqual([[], [0], [0], [0], [0], []]);
  });

  it("doesn't depend on the syntax", () => {
    // The `end` closing the `if` is missing: the lines follow the indentation.
    const text = ["  always begin", "    if (a) begin", "      b <= 1;", "  end"].join("\n");
    expect(scopeColumns(indents(text))).toEqual([[], [2], [2, 4], []]);
  });

  it("draws nothing for unindented code", () => {
    expect(scopeColumns(indents("module main;\nreg a;\nendmodule"))).toEqual([[], [], []]);
  });

  it("runs through compiler directives, wherever they're written", () => {
    // The user's report, 2026-11-21: an `ifdef at column 0 inside a module
    // cut the module's line.
    const text = [
      "module main;",
      "  wire a;",
      "`ifdef SYNTHESIZE",
      "  SB_RGBA_DRV #(",
      "    .CURRENT_MODE(1)",
      "  ) driver ();",
      "`else",
      "  assign a = 0;",
      "`endif",
      "",
      "endmodule",
    ].join("\n");
    expect(scopeColumns(indents(text))).toEqual([
      [], [0], [0], [0], [0, 2], [0], [0], [0], [0], [0], [],
    ]);
  });

  it("counts tabs by the tab size", () => {
    expect([indentOf("\tx"), indentOf("  \tx"), indentOf("\t\tx", 2)]).toEqual([4, 4, 4]);
    expect([indentOf("   "), indentOf("  `endif"), indentOf("  `MY_MACRO(x)")]).toEqual([null, null, 2]);
  });

  it("reaches the next case item and endmodule across blank lines", () => {
    // The user's report, 2026-11-21.
    const text = [
      "module m;",
      "  always @(posedge clk)",
      "    case (step)",
      "      STOP:",
      "        if (bad) begin",
      "          step <= WAIT;",
      "        end",
      "",
      "      default:",
      "        step <= START;",
      "    endcase",
      "",
      "endmodule",
    ].join("\n");
    const columns = scopeColumns(indents(text));
    expect(columns[7]).toEqual([0, 2, 4, 6]); // up to default:
    expect(columns[11]).toEqual([0]); // up to endmodule
  });

  it("doesn't join siblings across a blank line", () => {
    const text = ["module m;", "  reg a;", "", "  reg b;", "endmodule"].join("\n");
    expect(scopeColumns(indents(text))[2]).toEqual([0]);
  });
});
