import { describe, expect, it } from "vitest";
import { scopeColumns } from "./scopeLines";

/** The indentation of each line of `text` (null when blank). */
const indents = (text: string) =>
  text.split("\n").map((l) => (l.trim() === "" ? null : l.length - l.trimStart().length));

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

  it("keeps a block's lines through its blank lines", () => {
    const text = ["module main;", "  reg a;", "", "  reg b;", "", "endmodule"].join("\n");
    expect(scopeColumns(indents(text))).toEqual([[], [0], [0], [0], [], []]);
  });

  it("doesn't depend on the syntax", () => {
    // The `end` closing the `if` is missing: the lines follow the indentation.
    const text = ["  always begin", "    if (a) begin", "      b <= 1;", "  end"].join("\n");
    expect(scopeColumns(indents(text))).toEqual([[], [2], [2, 4], []]);
  });

  it("draws nothing for unindented code", () => {
    expect(scopeColumns(indents("module main;\nreg a;\nendmodule"))).toEqual([[], [], []]);
  });
});
