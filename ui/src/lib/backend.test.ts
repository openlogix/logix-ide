import { describe, expect, it, vi } from "vitest";

// backend.ts imports Tauri's IPC module, which needs the app window; stub it.
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), Channel: vi.fn() }));

const { toUserError } = await import("./backend");

describe("toUserError", () => {
  it("passes through backend errors", () => {
    expect(toUserError({ message: "Short.", details: "Short.: long" })).toEqual(
      { message: "Short.", details: "Short.: long" },
    );
  });
  it("wraps plain strings and other values", () => {
    expect(toUserError("boom")).toEqual({ message: "boom", details: "boom" });
    expect(toUserError(new Error("bad"))).toEqual({
      message: "bad",
      details: "bad",
    });
  });
});
