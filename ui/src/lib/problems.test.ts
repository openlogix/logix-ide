import { describe, expect, it } from "vitest";
import type { Diagnostic } from "./backend";
import { afterCommand, afterEdit, allProblems, problemCount, tabMarks } from "./problems";

const d = (file: string | null, severity: "error" | "warning" = "error"): Diagnostic => ({
  severity,
  file,
  line: 1,
  column: null,
  message: `${severity} in ${file}`,
});

describe("afterCommand", () => {
  it("replaces the problems of the files the command read, keeps the others", () => {
    // Run Tests reads main.v and main_tb.v (Logix Engine's checked_files), never the pin file.
    const earlier = [d("pins.pcf"), d("main.v"), d(null)];
    const found = [d("main.v", "warning")];
    const after = afterCommand(earlier, found, ["main.v", "main_tb.v"]);
    expect(after).toEqual([d("pins.pcf"), d("main.v", "warning")]);
  });
  it("a successful command clears the files it read", () => {
    expect(afterCommand([d("pins.pcf")], [], ["main.v", "pins.pcf"])).toEqual([]);
  });
  it("a command that read nothing keeps every file's problems", () => {
    expect(afterCommand([d("main.v")], [], [])).toEqual([d("main.v")]);
  });
});

describe("afterEdit", () => {
  it("drops only the edited file's problems", () => {
    expect(afterEdit([d("main.v"), d("pins.pcf")], "pins.pcf")).toEqual([d("main.v")]);
  });
  it("returns the same list when the file had none (no redraw needed)", () => {
    const list = [d("main.v")];
    expect(afterEdit(list, "pins.pcf")).toBe(list);
  });
});

describe("tab marks", () => {
  it("combine live and command problems, an error over a warning", () => {
    const live = new Map([["main.v", [d("main.v", "warning")]]]);
    const all = allProblems([d("main.v"), d("top.v", "warning"), d(null)], live);
    expect(tabMarks(all)).toEqual(
      new Map([
        ["main.v", "error"],
        ["top.v", "warning"],
      ]),
    );
  });
});

describe("problemCount", () => {
  it("says how many errors and warnings a file has", () => {
    const all = [d("main.v"), d("main.v"), d("main.v", "warning"), d("top.v", "warning")];
    expect(problemCount(all, "main.v")).toBe("2 errors, 1 warning");
    expect(problemCount(all, "top.v")).toBe("1 warning");
    expect(problemCount(all, "x.v")).toBe("");
  });
});
