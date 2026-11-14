import { describe, expect, it } from "vitest";
import { appendCapped } from "./format";

describe("appendCapped", () => {
  it("keeps only the newest items", () => {
    expect(appendCapped([1, 2], [3], 5)).toEqual([1, 2, 3]);
    expect(appendCapped([1, 2, 3], [4, 5], 3)).toEqual([3, 4, 5]);
  });
});
