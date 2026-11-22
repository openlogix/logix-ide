import { beforeEach, describe, expect, it } from "vitest";
import { loadSimulated, saveSimulated, simulateChoice, simulateTitle } from "./simulateChoice";

const serial = ["main.v", "uart.v", "main_tb.v", "uart_tb.v", "pinout.pcf"];

describe("simulateChoice", () => {
  it("runs the open testbench", () => {
    expect(simulateChoice(serial, "uart_tb.v", "main_tb.v")).toEqual({ kind: "run", testbench: "uart_tb.v" });
  });
  it("runs the open file's own testbench", () => {
    expect(simulateChoice(serial, "main.v", null)).toEqual({ kind: "run", testbench: "main_tb.v" });
    expect(simulateChoice(serial, "uart.v", "main_tb.v")).toEqual({ kind: "run", testbench: "uart_tb.v" });
  });
  it("runs the only testbench", () => {
    expect(simulateChoice(["main.v", "main_tb.v", "pinout.pcf"], "pinout.pcf", null)).toEqual({
      kind: "run",
      testbench: "main_tb.v",
    });
  });
  it("runs the testbench simulated last, else asks", () => {
    expect(simulateChoice(serial, "pinout.pcf", "uart_tb.v")).toEqual({ kind: "run", testbench: "uart_tb.v" });
    expect(simulateChoice(serial, "pinout.pcf", null)).toEqual({
      kind: "ask",
      testbenches: ["main_tb.v", "uart_tb.v"],
    });
    // One renamed or deleted since: ask.
    expect(simulateChoice(serial, "pinout.pcf", "gone_tb.v").kind).toBe("ask");
  });
  it("a design file without its own testbench falls through", () => {
    expect(simulateChoice([...serial, "debug.v"], "debug.v", null).kind).toBe("ask");
  });
  it("has nothing without testbenches", () => {
    expect(simulateChoice(["main.v", "pinout.pcf"], "main.v", "main_tb.v")).toEqual({ kind: "none" });
    expect(simulateChoice(serial, null, null).kind).toBe("ask");
  });
});

describe("simulateTitle", () => {
  it("says what Simulate will do", () => {
    expect(simulateTitle({ kind: "run", testbench: "main_tb.v" })).toBe(
      "Simulate main_tb.v: run it and show its signals",
    );
    expect(simulateTitle({ kind: "ask", testbenches: [] })).toMatch(/^Simulate: asks which testbench/);
    expect(simulateTitle({ kind: "none" })).toMatch(/add a testbench \(a file ending in _tb\.v\)/);
  });
});

describe("the testbench simulated last", () => {
  // The tests run outside a browser: a stand-in for the web view's storage.
  beforeEach(() => {
    const data = new Map<string, string>();
    (globalThis as { localStorage?: unknown }).localStorage = {
      getItem: (k: string) => data.get(k) ?? null,
      setItem: (k: string, v: string) => void data.set(k, v),
    };
  });
  it("is kept per project", () => {
    expect(loadSimulated("/p/a")).toBeNull();
    saveSimulated("/p/a", "uart_tb.v");
    saveSimulated("/p/b", "main_tb.v");
    saveSimulated("/p/a", "main_tb.v");
    expect(loadSimulated("/p/a")).toBe("main_tb.v");
    expect(loadSimulated("/p/b")).toBe("main_tb.v");
  });
});
