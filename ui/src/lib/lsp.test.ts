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

describe("ProjectWorkspace", async () => {
  const { ProjectWorkspace, lspProblems } = await import("./lsp");
  const { Text } = await import("@codemirror/state");

  function fakeClient() {
    const sent: string[] = [];
    const client = {
      didOpen: (f: { uri: string; version: number }) => sent.push(`open ${f.uri} v${f.version}`),
      didClose: (uri: string) => sent.push(`close ${uri}`),
      sync: () => {
        for (const u of workspace.syncFiles()) {
          sent.push(`change ${u.file.uri} v${u.file.version}: ${u.file.doc.toString()}`);
        }
      },
    };
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const workspace = new ProjectWorkspace(client as any);
    return { workspace, sent };
  }
  const file = (uri: string, text: string) => ({ uri, languageId: "verilog", doc: Text.of([text]) });

  it("opens every file, shown or not, and sends changes to the ones not shown", () => {
    const { workspace, sent } = fakeClient();
    workspace.setFiles([file("a.v", "module a;"), file("b.v", "module b;")]);
    expect(sent).toEqual(["open a.v v0", "open b.v v0"]);
    sent.length = 0;
    // Unchanged text: nothing sent.
    workspace.setFiles([file("a.v", "module a;"), file("b.v", "module b;")]);
    expect(sent).toEqual([]);
    workspace.setFiles([file("a.v", "module a; endmodule"), file("b.v", "module b;")]);
    expect(sent).toEqual(["change a.v v1: module a; endmodule"]);
  });

  it("closes files that are gone", () => {
    const { workspace, sent } = fakeClient();
    workspace.setFiles([file("a.v", "x"), file("b.v", "y")]);
    sent.length = 0;
    workspace.setFiles([file("a.v", "x")]);
    expect(sent).toEqual(["close b.v"]);
    expect(workspace.files.map((f) => f.uri)).toEqual(["a.v"]);
  });

  it("keeps a file open when its tab is no longer shown", () => {
    const { workspace, sent } = fakeClient();
    workspace.setFiles([file("a.v", "x")]);
    const view = { state: { doc: Text.of(["x"]) } };
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    workspace.openFile("a.v", "verilog", view as any);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    workspace.closeFile("a.v", view as any);
    expect(sent).toEqual(["open a.v v0"]);
    expect(workspace.getFile("a.v")?.getView()).toBe(null);
  });

  it("reads LSP problems as 1-based lines and columns", () => {
    const range = { start: { line: 2, character: 4 } };
    expect(lspProblems([{ range, severity: 1, message: "syntax error" }, { range, severity: 2, message: "w" }])).toEqual([
      { severity: "error", file: null, line: 3, column: 5, message: "syntax error" },
      { severity: "warning", file: null, line: 3, column: 5, message: "w" },
    ]);
  });
});
