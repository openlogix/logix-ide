import { describe, expect, it, vi } from "vitest";

// lsp.ts imports the backend, which needs the app window; stub Tauri.
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), Channel: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const { fileUri, hoverMarkdown, tidyVeribleHover } = await import("./lsp");

describe("fileUri", () => {
  it("uses the real path of a saved project's file", () => {
    expect(fileUri("/Users/ann/Documents/Logix/my blinky", "main.v")).toBe(
      "file:///Users/ann/Documents/Logix/my%20blinky/main.v",
    );
  });
  it("uses a placeholder for an unsaved project", () => {
    expect(fileUri(null, "main_tb.v")).toBe("file:///logix-unsaved/main_tb.v");
  });
});

describe("hoverMarkdown", () => {
  it("drops empty answers", () => {
    expect(hoverMarkdown(null)).toEqual([]);
    expect(hoverMarkdown("")).toEqual([]);
    expect(hoverMarkdown({ kind: "markdown", value: "  " })).toEqual([]);
    expect(hoverMarkdown(["", { language: "verilog", value: "" }])).toEqual([]);
  });
  it("keeps the rest as Markdown", () => {
    expect(hoverMarkdown({ kind: "markdown", value: "wire x" })).toEqual(["wire x"]);
    expect(hoverMarkdown(["a", { language: "verilog", value: "reg r;" }])).toEqual([
      "a",
      "```verilog\nreg r;\n```",
    ]);
  });
});

describe("tidyVeribleHover", () => {
  // As Verible sends them (see the language server's textDocument/hover).
  it("keeps the name and the useful part, without Verible's category", () => {
    expect(
      tidyVeribleHover(
        "### data/net/var/instance counter\n\n---\n\nType: reg [23:0]\n\n---",
      ),
    ).toBe("**counter**\n\nType: reg [23:0]");
  });
  it("shows nothing when there's only the title", () => {
    expect(tidyVeribleHover("### data/net/var/instance sys_clk\n\n")).toBe("");
    expect(tidyVeribleHover("### module main\n\n")).toBe("");
    expect(hoverMarkdown({ kind: "markdown", value: "### module main\n\n" })).toEqual([]);
  });
  it("leaves other Markdown alone", () => {
    expect(tidyVeribleHover("Some *text*")).toBe("Some *text*");
  });
});
