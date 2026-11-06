// A fake of src/lib/backend.ts for the end-to-end UI tests (Playwright). In
// "e2e" mode, Vite replaces the real backend with this (see
// vite.config.ts), so the real UI runs in a plain browser without Tauri or
// Logix Engine. It behaves like a small, predictable Logix backend:
// - the FPGA tools are ready right away, with Logix Engine's real boards,
//   Logix's examples, and outputs replayed from real engine runs
//   (e2e/fixtures);
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

// Real engine data and output (see e2e/fixtures/README.md).
import catalog from "./fixtures/catalog.json";
import { configurationOf, resolve } from "../src/lib/variants";
import verifyOutput from "./fixtures/verify.txt?raw";
import verifyErrorOutput from "./fixtures/verify-error.txt?raw";
import testOutput from "./fixtures/test.txt?raw";
import graphOutput from "./fixtures/graph.txt?raw";
import graphSvg from "./fixtures/graph.svg?raw";
import packageJson from "../../package.json";

const BOARDS: Real.Board[] = catalog.boards.map((b) => ({ ...b, custom: false }));

/**
 * The project's own boards as the fake engine reads them from the text of
 * custom-boards.json: enough of Logix Engine's rules to test the UI (valid
 * JSON, an object, board IDs, an FPGA, variants with a connection and
 * memory), with each problem at the line of the board's key (line 1 for
 * invalid JSON), and the FPGA's family taken from its ID.
 */
function fakeProjectBoards(text: string | null): Real.ProjectBoards {
  if (text === null || text.trim() === "") return { boards: [], problem: null };
  const problem = (message: string, line = 1): Real.ProjectBoards => ({
    boards: [],
    problem: {
      message: `The custom boards file has problems. custom-boards.json:${line}:1: ${message}`,
      diagnostics: [
        { severity: "error", file: "custom-boards.json", line, column: 1, message },
      ],
    },
  });
  let boards: unknown;
  try {
    boards = JSON.parse(text);
  } catch {
    return problem("Expected value.");
  }
  if (typeof boards !== "object" || boards === null || Array.isArray(boards)) {
    return problem("Invalid type: expected a map.");
  }
  const lineOf = (id: string) => text.split("\n").findIndex((l) => l.includes(`"${id}"`)) + 1 || 1;
  try {
    return { boards: fakeBoards(boards as Record<string, unknown>), problem: null };
  } catch (e) {
    const { message, id } = e as { message: string; id: string };
    return problem(message, lineOf(id));
  }
}

function fakeBoards(boards: Record<string, unknown>): Real.Board[] {
  let current = "";
  const fail = (message: string): never => {
    throw { message, id: current };
  };
  const family = (fpga: string) =>
    fpga.startsWith("ice40") ? "ice40" : fpga.startsWith("lfe5") ? "ecp5" : fpga.startsWith("gw") ? "gowin" : "xilinx";
  // Value labels, as the catalog's boards have them.
  const labels = new Map<string, Real.Dimension["values"][number]>();
  for (const b of BOARDS) for (const d of b.dimensions) for (const v of d.values) labels.set(`${d.id}:${v.id}`, v);
  return Object.entries(boards).map(([id, item]) => {
    current = id;
    const b = { ...(item as object), id } as { id: string; description?: string; fpga_id?: string; variants?: { connection: string; memory: string; tested: boolean }[] };
    if (!/^[a-z0-9-]+$/.test(b.id)) fail(`${JSON.stringify(b.id)} isn't a board ID: use lowercase letters, digits and hyphens.`);
    if (!b.fpga_id) fail(`Board ${JSON.stringify(b.id)}: give exactly one of fpga_id and fpga.`);
    if (!b.variants?.length) fail("A board needs at least one variant.");
    const arch = family(b.fpga_id!);
    const like = BOARDS.find((x) => x.arch === arch)!;
    const dims = ["connection", "memory"].map((id) => ({
      id,
      label: id === "connection" ? "Connection" : "Memory",
      values: [...new Set(b.variants!.map((v) => (id === "connection" ? v.connection : v.memory)))].map(
        (v) => labels.get(`${id}:${v}`) ?? { id: v, label: v },
      ),
    }));
    return {
      ...like,
      id: b.id!,
      description: b.description ?? b.id!,
      dimensions: dims,
      configurations: b.variants!.map((v) => ({
        when: { connection: v.connection, memory: v.memory },
        tested: v.tested,
      })),
      custom: true,
    };
  });
}
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

/** The blinky example for `board`, as an unsaved project. */
function blinkyDoc(board: string): ProjectDoc {
  return {
    dir: null,
    templateDir: `/fake/examples/${board}/blinky`,
    name: "blinky",
    files: [
      { name: "main.v", text: BLINKY_MAIN },
      { name: "main_tb.v", text: BLINKY_TB },
      { name: "pinout.pcf", text: "set_io sys_clk 49\nset_io led 45\n" },
    ],
    board,
    variant: {},
  };
}

/** A board's Bare Minimum (also File > New Project > Empty Project). */
function bareDoc(board: string): ProjectDoc {
  return {
    dir: null,
    templateDir: null,
    name: "untitled",
    files: [
      { name: "main.v", text: "module main (\n    input  sys_clk,\n    output led\n);\n\n  assign led = 0;\n\nendmodule\n" },
      { name: "pinout.pcf", text: "set_io sys_clk 49\nset_io led 45\n" },
    ],
    board,
    variant: {},
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
  showOutput: "onCommand",
};

const handlers: {
  menu?: (id: MenuId) => void;
  output?: (line: string) => void;
  finishSetup?: () => void;
} = {};

/** Test hooks, reachable from Playwright via `page.evaluate`. */
const fake = {
  menu: (id: MenuId) => handlers.menu?.(id),
  /** The start screen's recent projects (`recentProjects`); `?recents=a,b`
   * sets them from the start. */
  recents: (new URLSearchParams(location.search).get("recents") ?? "")
    .split(",")
    .filter(Boolean),
  /** The USB devices Tools > Connected Devices lists (`scanDevices`); one Alhambra II by default. */
  usb: null as Real.UsbDevice[] | null,
  /** When set, Save As fails with this error (`saveProjectAsDialog`). */
  saveAsError: null as Real.UserError | null,
  /** Devices reported as plugged in, with their boards (`connectedDevices`). */
  connected: [] as Real.ConnectedDevice[],
  /** The devices Upload finds that fit the board; with several, Upload
   * asks which one (`select_device`). One Alhambra II by default. */
  uploadDevices: null as Real.UsbDevice[] | null,
  /** The device id the last Upload used. */
  uploadedDevice: null as string | null,
  /** The menu state the UI reported last (`setMenuState`). */
  menuState: null as Real.MenuState | null,
  /** The menu state when the editor's right-click menu was shown last. */
  editorMenu: null as Real.MenuState | null,
  /** How often the output pane's right-click menu was shown. */
  outputMenu: 0,
  /** More files for the projects `openProjectAt` opens, such as custom-boards.json. */
  projectFiles: [] as ProjectFile[],
  /** The custom boards of the last task run (`runTask`). */
  taskCustomBoards: null as string | null,
  /** The custom boards of the last connected-devices scan. */
  scannedCustomBoards: null as string | null,
  /** The variant of the last task run (`runTask`). */
  taskVariant: null as Real.Variant | null,
  /** The board and variant last saved (`saveProject`, Save As). */
  saved: null as { board: string | null; variant: Real.Variant } | null,
  /** Ends a setup held at the engine's package install (`/?engineSetup`). */
  finishSetup: () => handlers.finishSetup?.(),
};
(window as unknown as { __fake: typeof fake }).__fake = fake;

/** A real Alhambra II, as Logix Engine lists it. */
const alhambraDevice: Real.UsbDevice = {
  id: "usb:1-1:0403:6010:FTATANKC",
  vid: "0403",
  pid: "6010",
  manufacturer: "AlhambraBits",
  product: "Alhambra II v1.0A",
  serialNumber: "FTATANKC",
  boards: [
    { board: "alhambra-ii", level: "strong" },
    { board: "alchitry-au", level: "weak" },
  ],
  buildMatch: "strong",
};

const unlisten = async () => () => {};
const pause = (ms: number) => new Promise((r) => setTimeout(r, ms));

function buildOutcome(files: ProjectFile[], board: string): {
  lines: string[];
  outcome: TaskOutcome;
} {
  const main = files.find((f) => f.name === "main.v")?.text ?? "";
  // The blinky with the ";" missing after "assign led = counter[23]":
  // replay what the engine really reports (lint's message, on the next line).
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
        action: null,
        devices: [],
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
      lines: [
        "\x1b[2m$ verilator_bin --lint-only --quiet main.v\x1b[0m",
        "\x1b[2m$ yosys -q -p 'synth_ice40 -top main' -DSYNTHESIZE main.v\x1b[0m",
        `\x1b[31mmain.v:${bad + 1}: ERROR: syntax error\x1b[0m`,
      ],
      outcome: {
        ok: false,
        diagnostics: [diag],
        testbenches: [],
        report: null,
        cancelled: false,
        graphSvg: null,
        seconds: 0.4,
        action: null,
        devices: [],
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
        summary: [
          { name: "logic cells", used: 27, available: 7680 },
          { name: "I/O pins", used: 2, available: 107 },
        ],
        clocks: [{ name: "sys_clk", fmaxMhz: 194.36 }],
      },
      cancelled: false,
        graphSvg: null,
      seconds: 1.2,
      action: null,
      devices: [],
    },
  };
}

export const backend: typeof Real.backend = {
  appInfo: async () => ({
    name: "Logix IDE",
    version: packageJson.version,
    platform: "macos-arm64",
    platformSupported: true,
  }),

  setup: async (onEvent) => {
    onEvent({ kind: "stage", stage: "checking" });
    // With `/?devEngine`, setup uses a local engine (LOGIX_DEV_ENGINE_BINARY).
    if (new URLSearchParams(location.search).has("devEngine")) {
      onEvent({ kind: "devEngine", path: "/Users/you/logix-engine/target/debug/logix-engine" });
    }
    // With `/?engineSetup`, setup stops in the engine's package install
    // until a test calls `__fake.finishSetup()`.
    if (new URLSearchParams(location.search).has("engineSetup")) {
      onEvent({ kind: "stage", stage: "installingEnginePackages" });
      onEvent({ kind: "log", line: "Installing oss-cad-suite..." });
      onEvent({
        kind: "progress",
        message: "Installing oss-cad-suite",
        percent: 42,
      });
      await new Promise<void>((resolve) => (handlers.finishSetup = resolve));
    }
    await pause(50);
    onEvent({ kind: "stage", stage: "ready" });
  },

  getBoards: async () => BOARDS,

  getExamples: async () => ({
    examples: catalog.examples,
    hidden: catalog.hidden,
  }),

  openBareMinimum: async (board) => bareDoc(board),
  openExample: async (board) => blinkyDoc(board),

  runTask: async (task, target, _key, _sourceDir, _projectDir, files, board, variant, device, onEvent) => {
    fake.taskVariant = variant;
    const customBoards = files.find((f) => f.name === "custom-boards.json")?.text ?? null;
    fake.taskCustomBoards = customBoards;
    // As the engine: a file with problems fails the task, at its lines.
    const own = fakeProjectBoards(customBoards);
    if (own.problem) {
      onEvent({ kind: "line", line: `\x1b[31m${own.problem.message}\x1b[0m` });
      return {
        ok: false, diagnostics: own.problem.diagnostics, testbenches: [], report: null,
        cancelled: false, graphSvg: null, seconds: 0.1, action: null, devices: [],
      };
    }
    const onLine = (line: string) => onEvent({ kind: "line", line });
    const { lines: buildLines, outcome } = buildOutcome(files, board);
    let lines = buildLines;
    if (outcome.ok && task === "test") {
      lines = outputLines(testOutput);
      outcome.testbenches = [{ name: "main_tb.v", passed: true }];
      outcome.report = null;
    }
    if (outcome.ok && task === "graph") {
      // The engine's yosys command names the module drawn.
      lines = outputLines(graphOutput).map((l) =>
        l.replace(/ main' /, ` ${target ?? "main"}' `),
      );
      outcome.graphSvg = graphSvg;
      outcome.report = null;
    }
    // Logix Engine reports each task's phases.
    const phase = {
      build: "Synthesizing",
      upload: "Synthesizing",
      test: "Running main_tb.v",
      simulate: "Running main_tb.v",
      graph: "Drawing the graph",
    }[task];
    const phases = task === "build" || task === "upload";
    onEvent({ kind: "progress", message: phase, percent: null });
    for (const line of lines) {
      onLine(line);
      await pause(5);
    }
    const fits = fake.uploadDevices ?? [alhambraDevice];
    if (phases && task === "upload" && outcome.ok && !device && fits.length > 1) {
      // The engine's list_devices for the build found several devices.
      const message = `${fits.length} connected devices could be this board. Choose one to upload to.`;
      onLine(`\x1b[31m${message}\x1b[0m`);
      return {
        ...outcome,
        ok: false,
        report: null,
        action: "select_device",
        devices: fits,
        diagnostics: [{ severity: "error", file: null, line: null, column: null, message }],
      };
    }
    if (phases && task === "upload" && outcome.ok) {
      fake.uploadedDevice = device ?? fits[0].id;
      // The engine's messages (program_board), for the board's memory.
      const info = [...own.boards, ...BOARDS].find((b) => b.id === board);
      const memory = info && resolve(info, variant).memory === "ram" ? "RAM" : "flash";
      const name = info?.description ?? board;
      const untested = info && configurationOf(info, variant)?.tested === false ? " (untested)" : "";
      onLine(`Programming the ${memory} of the ${name} through its USB port${untested}.`);
      onEvent({ kind: "progress", message: `Writing ${memory}`, percent: 42 });
      onLine("Programmed Alhambra II v1.0A.");
      onLine(
        `Programmed the ${memory} of the ${name}: the design ${
          memory === "RAM" ? "is lost" : "stays"
        } when the board is powered off.`,
      );
    }
    return outcome;
  },

  openLastProject: async () => null,

  formatFile: async (name, text) => {
    handlers.output?.(
      `\x1b[2m$ verible-verilog-format --nofailsafe_success --inplace ${name}\x1b[0m`,
    );
    return {
      ok: true,
      text: text.replace(/[ \t]+$/gm, ""),
      diagnostics: [],
    };
  },

  newProject: async (board) => bareDoc(board ?? "alhambra-ii"),
  lastBoard: async () => "alhambra-ii",
  lastVariant: async () => ({}),
  recentProjects: async () => fake.recents,
  // The fake's recent projects are all UPduino v2 projects (openProjectAt).
  recentProjectsForBoard: async (board) => (board === "upduino2" ? fake.recents : []),
  setLastBoard: async () => {},
  closeProject: async () => {},
  openProjectDialog: async () => null,
  saveProject: async (_dir, _files, _removed, board, variant) => {
    fake.saved = { board, variant };
  },

  saveProjectAsDialog: async (_dir, _source, suggestedName, files, _removed, board, variant) => {
    if (fake.saveAsError) throw fake.saveAsError;
    fake.saved = { board, variant };
    return {
      dir: `/fake/projects/${suggestedName}`,
      templateDir: null,
      name: suggestedName,
      files,
      board,
      variant,
    };
  },

  // A saved project for the UPduino v2 (any folder), so tests can check that
  // opening it brings its own board.
  openProjectAt: async (dir) => {
    const doc = bareDoc("upduino2");
    return {
      ...doc,
      files: [...doc.files, ...fake.projectFiles],
      dir,
      name: dir.split("/").pop() ?? dir,
    };
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

  scanDevices: async () => ({ usb: fake.usb ?? [alhambraDevice], unreachable: [] }),
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
  connectedDevices: async (customBoards) => {
    fake.scannedCustomBoards = customBoards;
    return fake.connected;
  },
  projectBoards: async (customBoards) => fakeProjectBoards(customBoards),
  showOutputMenu: async () => {
    fake.outputMenu += 1;
  },
  showEditorMenu: async () => {
    fake.editorMenu = fake.menuState && { ...fake.menuState };
  },
  setMenuState: async (menu) => {
    fake.menuState = menu;
  },

  onMenu: async (handler) => {
    handlers.menu = handler;
    return () => {};
  },
  onToolOutput: async (handler) => {
    handlers.output = handler;
    return () => {};
  },
  onCloseRequested: unlisten,
};
