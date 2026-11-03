import { describe, expect, it } from "vitest";
import {
  clampConsoleHeight,
  COLLAPSE_CONSOLE_HEIGHT,
  dragHidesConsole,
  MIN_CONSOLE_HEIGHT,
  MIN_EDITOR_HEIGHT,
} from "./layout";

describe("clampConsoleHeight", () => {
  it("keeps heights that fit", () => {
    expect(clampConsoleHeight(200, 600, 6)).toBe(200);
  });
  it("enforces the console minimum", () => {
    expect(clampConsoleHeight(10, 600, 6)).toBe(MIN_CONSOLE_HEIGHT);
  });
  it("leaves room for the editor", () => {
    expect(clampConsoleHeight(1000, 600, 6)).toBe(600 - 6 - MIN_EDITOR_HEIGHT);
  });
  it("never goes below the console minimum in tiny windows", () => {
    expect(clampConsoleHeight(100, 50, 6)).toBe(MIN_CONSOLE_HEIGHT);
  });
});

describe("dragHidesConsole", () => {
  it("hides the output when dragged near the bottom", () => {
    expect(dragHidesConsole(0)).toBe(true);
    expect(dragHidesConsole(-50)).toBe(true);
    expect(dragHidesConsole(COLLAPSE_CONSOLE_HEIGHT - 1)).toBe(true);
  });
  it("keeps it at the minimum height above that", () => {
    expect(dragHidesConsole(COLLAPSE_CONSOLE_HEIGHT)).toBe(false);
    expect(dragHidesConsole(MIN_CONSOLE_HEIGHT)).toBe(false);
  });
});
