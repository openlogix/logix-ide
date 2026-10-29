// A fake of src/lib/backend.ts for the end-to-end UI tests (Playwright). In
// "e2e" mode, Vite replaces the real backend with this (see
// vite.config.ts), so the real UI runs in a plain browser without Tauri or
// Apio. It behaves like a small, predictable Logix backend:
// - the FPGA tools are ready right away, with Apio's real boards and
//   examples, and outputs replayed from real Apio runs (e2e/fixtures);
// - a build fails if main.v contains "oops" (with an error on that line);
// - Auto Format removes trailing spaces.
// Tests drive native-menu actions through `window.__fake.menu(id)`.

import type * as Real from "../src/lib/backend";
import type {
  Diagnostic,
  MenuId,
  ProjectDoc,
  ProjectFile,
  TaskOutcome,
} from "../src/lib/backend";

// Everything except `backend` comes from the real module (types, toUserError).
export * from "../src/lib/backend";

// Real Apio data and output (see e2e/fixtures/README.md).
import catalog from "./fixtures/catalog.json";
import verifyOutput from "./fixtures/verify.txt?raw";
import verifyErrorOutput from "./fixtures/verify-error.txt?raw";
import testOutput from "./fixtures/test.txt?raw";
import graphSvg from "./fixtures/graph.svg?raw";
import packageJson from "../../package.json";

const BOARDS = catalog.boards;
const outputLines = (text: string) => text.replace(/\n$/, "").split("\n");

const BLINKY_MAIN = `module main (
    input sys_clk,
    output led
);
  reg [23:0] counter = 0;
  always @(posedge sys_clk) counter <= counter + 1;
  assign led = counter[23];
endmodule
`;

const BLINKY_TB = `\`timescale 1ns / 1ps
module main_tb;
  initial begin
    $dumpvars(0, main_tb);
    #100;
    $finish;
  end
endmodule
`;

function starterDoc(board: string | null): ProjectDoc {
  if (!board) {
    return {
      dir: null,
      templateDir: null,
      name: "untitled",
      files: [{ name: "main.v", text: "module main ();\nendmodule\n" }],
      board: null,
      starter: true,
    };
  }
  return {
    dir: null,
    templateDir: `/fake/examples/${board}/blinky`,
    name: "untitled",
    files: [
      { name: "main.v", text: BLINKY_MAIN },
      { name: "main_tb.v", text: BLINKY_TB },
      { name: "pinout.pcf", text: "set_io sys_clk 49\nset_io led 45\n" },
    ],
    board,
    starter: true,
  };
}

let preferences: Real.Preferences = {
  projectsDir: "/Users/you/Documents/Logix",
  editorFontSize: 13,
  uiScale: null,
  theme: "system",
  lintOnVerify: true,
  liveChecking: true,
  hoverInfo: false,
};

const handlers: {
  menu?: (id: MenuId) => void;
  output?: (line: string) => void;
} = {};

/** Test hooks, reachable from Playwright via `page.evaluate`. */
const fake = {
  menu: (id: MenuId) => handlers.menu?.(id),
  /** Boards reported as plugged in (`connectedBoards`). */
  connected: [] as Real.ConnectedBoard[],
  /** The editor's right-click menu shown last (its Go to Definition state). */
  editorMenu: null as { canGoToDefinition: boolean } | null,
  /** How often the output pane's right-click menu was shown. */
  outputMenu: 0,
};
(window as unknown as { __fake: typeof fake }).__fake = fake;

const unlisten = async () => () => {};
const pause = (ms: number) => new Promise((r) => setTimeout(r, ms));

function buildOutcome(files: ProjectFile[], board: string): {
  lines: string[];
  outcome: TaskOutcome;
} {
  const main = files.find((f) => f.name === "main.v")?.text ?? "";
  // The blinky with the ";" missing after "assign led = counter[23]":
  // replay what Apio really reports (lint's message, on the next line).
  if (/assign led = counter\[23\]\s*$/m.test(main)) {
    const line = main.split("\n").findIndex((l) => /^\s*endmodule/.test(l)) + 1;
    return {
      lines: outputLines(verifyErrorOutput),
      outcome: {
        ok: false,
        diagnostics: [
          {
            severity: "error",
            file: "main.v",
            line,
            column: 1,
            message: "syntax error, unexpected endmodule, expecting ',' or ';'",
          },
        ],
        testbenches: [],
        report: null,
        cancelled: false,
        graphSvg: null,
        seconds: 0.5,
      },
    };
  }
  const bad = main.split("\n").findIndex((l) => l.includes("oops"));
  if (bad >= 0) {
    const diag: Diagnostic = {
      severity: "error",
      file: "main.v",
      line: bad + 1,
      column: null,
      message: "syntax error, unexpected identifier",
    };
    return {
      lines: ["$ apio lint", "$ apio build", `main.v:${bad + 1}: ERROR: syntax error`],
      outcome: {
        ok: false,
        diagnostics: [diag],
        testbenches: [],
        report: null,
        cancelled: false,
        graphSvg: null,
        seconds: 0.4,
      },
    };
  }
  return {
    lines: outputLines(verifyOutput),
    outcome: {
      ok: true,
      diagnostics: [],
      testbenches: [],
      report: {
        arch: "ice40",
        resources: [
          { name: "ICESTORM_LC", used: 27, available: 7680 },
          { name: "SB_GB", used: 1, available: 8 },
          { name: "SB_IO", used: 2, available: 107 },
        ],
        clocks: [{ name: "sys_clk", fmaxMhz: 194.36 }],
      },
      cancelled: false,
        graphSvg: null,
      seconds: 1.2,
    },
  };
}

export const backend: typeof Real.backend = {
  appInfo: async () => ({
    name: "Logix",
    version: packageJson.version,
    platform: "macos-arm64",
    platformSupported: true,
  }),

  setupApio: async (onEvent) => {
    onEvent({ kind: "stage", stage: "checking" });
    onEvent({
      kind: "log",
      line: "Checking FPGA tools (the first run can take a few minutes)...",
    });
    await pause(50);
    onEvent({ kind: "stage", stage: "ready" });
  },

  getBoards: async () => BOARDS,

  getExamples: async () => ({
    examples: catalog.examples,
    hidden: catalog.hidden,
  }),

  openBareMinimum: async (board) => ({
    dir: null,
    templateDir: null,
    name: "untitled",
    files: [
      { name: "main.v", text: "module main (\n    input  sys_clk,\n    output led\n);\n\n  assign led = 0;\n\nendmodule\n" },
      { name: "pinout.pcf", text: "set_io sys_clk 49\nset_io led 45\n" },
    ],
    board,
    starter: false,
  }),
  openExample: async (board) => ({ ...starterDoc(board), starter: false, name: "blinky" }),

  runTask: async (task, target, _key, _sourceDir, _projectDir, files, board, onLine) => {
    const { lines: buildLines, outcome } = buildOutcome(files, board);
    let lines = buildLines;
    if (outcome.ok && task === "test") {
      lines = outputLines(testOutput);
      outcome.testbenches = [{ name: "main_tb.v", passed: true }];
      outcome.report = null;
    }
    if (outcome.ok && task === "graph") {
      lines = [`\x1b[2m$ apio graph --no-viewer${target ? ` -t ${target}` : ""}\x1b[0m`];
      outcome.graphSvg = graphSvg;
      outcome.report = null;
    }
    for (const line of lines) {
      onLine(line);
      await pause(5);
    }
    return outcome;
  },

  openLastProject: async () => null,

  formatFile: async (name, text) => {
    handlers.output?.(`$ apio format ${name}`);
    return {
      ok: true,
      text: text.replace(/[ \t]+$/gm, ""),
      diagnostics: [],
    };
  },

  newProject: async (board) => starterDoc(board ?? "alhambra-ii"),
  setLastBoard: async () => {},
  openProjectDialog: async () => null,
  saveProject: async () => {},

  saveProjectAsDialog: async (_source, suggestedName, files, _removed, board) => ({
    dir: `/fake/projects/${suggestedName}`,
    templateDir: null,
    name: suggestedName,
    files,
    board,
    starter: false,
  }),

  openProjectAt: async () => {
    throw { message: "Not available in the fake backend.", details: "" };
  },
  showProjectFolder: async () => {},
  saveGtkwaveLayouts: async () => [],
  cancelTask: async () => false,

  // No language server in the fake: it never answers, so the UI quietly
  // runs without live errors in these tests and screenshots.
  lspStart: async () => {},
  lspSend: async () => {},
  lspStop: async () => {},
  onLspMessage: unlisten,
  onLspClosed: unlisten,

  getPreferences: async () => ({ ...preferences }),
  setPreferences: async (p) => {
    if (p.editorFontSize < 8 || p.editorFontSize > 32) {
      throw {
        message: "The font size must be between 8 and 32.",
        details: "The font size must be between 8 and 32.",
      };
    }
    preferences = { ...p };
    return { ...preferences };
  },
  chooseFolderDialog: async () => "/Users/you/fpga",

  scanDevices: async () => ({
    usb: [
      {
        vid: "0403",
        pid: "6010",
        manufacturer: "AlhambraBits",
        product: "Alhambra II v1.0A",
        serialNumber: "FTATANKC",
        deviceType: "FT2232H",
      },
    ],
    serial: [
      {
        port: "/dev/cu.usbserial-FTATANKC0",
        vid: "0403",
        pid: "6010",
        manufacturer: "AlhambraBits",
        product: "Alhambra II v1.0A",
      },
    ],
  }),
  onOpenRecent: unlisten,

  checkFileName: async (name, existing) => {
    const n = name.trim();
    if (!n) return "Enter a file name.";
    if (/\s/.test(n)) return "A file name can't contain spaces.";
    if (!/\.(s?vh?|pcf|lpf|cst|xdc|pdc|ccf)$/i.test(n)) {
      return "Use one of these extensions: .v .sv .vh .svh .pcf .lpf .cst .xdc .pdc .ccf.";
    }
    if (existing.some((e) => e.toLowerCase() === n.toLowerCase())) {
      return `There's already a file named "${n}".`;
    }
    return null;
  },

  askConfirm: async () => true,
  askSaveChanges: async () => "discard",
  setDocumentState: async () => {},
  closeWindow: async () => {},
  logOutput: async () => {},
  connectedBoards: async () => fake.connected,
  showOutputMenu: async () => {
    fake.outputMenu += 1;
  },
  showEditorMenu: async (canGoToDefinition: boolean) => {
    fake.editorMenu = { canGoToDefinition };
  },

  onMenu: async (handler) => {
    handlers.menu = handler;
    return () => {};
  },
  onApioOutput: async (handler) => {
    handlers.output = handler;
    return () => {};
  },
  onCloseRequested: unlisten,
};
