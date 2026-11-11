import { describe, expect, it } from "vitest";
import type { Diagnostic } from "./backend";
import { afterCommand, afterEdit, allProblems, problemCount, tabMarks, taskReads } from "./problems";

const d = (file: string | null, severity: "error" | "warning" = "error"): Diagnostic => ({
  severity,
  file,
  line: 1,
  column: null,
  message: `${severity} in ${file}`,
});

describe("taskReads", () => {
  it("pin files are read only by Verify and Upload", () => {
    expect(taskReads("build", "pins")).toBe(true);
    expect(taskReads("upload", "otherPins")).toBe(true);
    expect(taskReads("test", "pins")).toBe(false);
    expect(taskReads("diagram", "pins")).toBe(false);
  });
  it("testbenches only by Run Tests and Simulate", () => {
    expect(taskReads("test", "testbench")).toBe(true);
    expect(taskReads("simulate", "testbench")).toBe(true);
    expect(taskReads("build", "testbench")).toBe(false);
  });
  it("the rest by every task", () => {
    for (const task of ["build", "upload", "test", "simulate", "diagram"] as const) {
      expect(taskReads(task, "main")).toBe(true);
      expect(taskReads(task, "design")).toBe(true);
    }
  });
});

describe("afterCommand", () => {
  it("replaces the problems of the files the command read, keeps the others", () => {
    const earlier = [d("pins.pcf"), d("main.v"), d(null)];
    const found = [d("main.v", "warning")];
    const after = afterCommand(earlier, found, (f) => f !== "pins.pcf");
    expect(after).toEqual([d("pins.pcf"), d("main.v", "warning")]);
  });
  it("a successful command clears the files it read", () => {
    expect(afterCommand([d("pins.pcf")], [], () => true)).toEqual([]);
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
