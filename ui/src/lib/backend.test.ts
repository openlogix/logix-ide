import { describe, expect, it, vi } from "vitest";

// backend.ts imports Tauri's IPC module, which needs the app window; stub it.
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), Channel: vi.fn() }));

const { toUserError, withReason } = await import("./backend");

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

describe("withReason", () => {
  it("adds the cause to the message, for a banner", () => {
    expect(
      withReason({
        message: "Couldn't save the project.",
        details:
          'Couldn\'t save the project.: "blinky" there is a file, not a project. Choose another name.',
      }),
    ).toBe('Couldn\'t save the project. "blinky" there is a file, not a project. Choose another name.');
    expect(
      withReason({ message: "Couldn't open it.", details: "Couldn't open it.: permission denied (os error 13)" }),
    ).toBe("Couldn't open it. Permission denied (os error 13).");
    expect(withReason({ message: "Short.", details: "Short." })).toBe("Short.");
    expect(withReason({ message: "Short.", details: "something else" })).toBe("Short.");
  });
});
