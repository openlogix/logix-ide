import { describe, expect, it } from "vitest";
import { appendCapped, formatBytes, progressFraction } from "./format";

describe("formatBytes", () => {
  it("formats small and large sizes", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(999)).toBe("999 B");
    expect(formatBytes(18_614_612)).toBe("18.6 MB");
    expect(formatBytes(2_500_000_000)).toBe("2.5 GB");
  });
});

describe("progressFraction", () => {
  it("returns null when total is unknown", () => {
    expect(progressFraction(10, null)).toBeNull();
    expect(progressFraction(10, 0)).toBeNull();
  });
  it("clamps to 0..1", () => {
    expect(progressFraction(50, 100)).toBe(0.5);
    expect(progressFraction(150, 100)).toBe(1);
  });
});

describe("appendCapped", () => {
  it("keeps only the newest items", () => {
    expect(appendCapped([1, 2], [3], 5)).toEqual([1, 2, 3]);
    expect(appendCapped([1, 2, 3], [4, 5], 3)).toEqual([3, 4, 5]);
  });
});
