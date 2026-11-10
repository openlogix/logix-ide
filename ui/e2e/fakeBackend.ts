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
import fpgaFamilies from "./fixtures/fpgas.json";
import { configurationOf, resolve } from "../src/lib/variants";
import { boardsProblemLines } from "../src/lib/boards";
import verifyOutput from "./fixtures/verify.txt?raw";
import verifyErrorOutput from "./fixtures/verify-error.txt?raw";
import testOutput from "./fixtures/test.txt?raw";
import graphOutput from "./fixtures/graph.txt?raw";
import graphSvg from "./fixtures/graph.svg?raw";
import packageJson from "../../package.json";

const BOARDS: Real.Board[] = catalog.boards.map((b) => ({ ...b, custom: false }) as unknown as Real.Board);
const FPGA_FAMILIES: Real.FpgaFamily[] = fpgaFamilies;

/**
 * Logix Engine's compare_boards, from the boards' facts: each aspect the
 * same, different or unknown, with the engine's sentences.
 */
function fakeCompare(a: Real.Board | undefined, b: Real.Board | undefined): Real.BoardComparison {
  if (!a || !b) throw { message: "Unknown board." };
  const aspect = (name: string, label: string, sameLabel: string, x: string, y: string, same: boolean) => ({
    aspect: name,
    same: same ? ("yes" as const) : ("no" as const),
    from: x,
    to: y,
    text: same ? `${sameLabel}: ${x}.` : `${label}: ${x} → ${y}.`,
  });
  const ext = (x: Real.Board) => `.${x.pinExtension ?? "?"}`;
  const sameFpga = !!a.fpga?.id && a.fpga.id === b.fpga?.id;
  return {
    sameBuild: sameFpga,
    aspects: [
      aspect("board", "Board", "Same board", a.description, b.description, a.id === b.id),
      aspect("family", "FPGA family", "Same FPGA family", a.familyLabel, b.familyLabel, a.family === b.family),
      aspect("fpga", "FPGA", "Same FPGA", a.fpga?.partNumber ?? "?", b.fpga?.partNumber ?? "?", sameFpga),
      aspect("size", "Size", "Same size", a.fpga?.size ?? "?", b.fpga?.size ?? "?", a.fpga?.size === b.fpga?.size),
      aspect("constraint_format", "Pin file format", "Same pin file format", ext(a), ext(b), ext(a) === ext(b)),
      a.id === b.id
        ? { aspect: "pins", same: "yes" as const, text: "Same pins: the same board." }
        : { aspect: "pins", same: "unknown" as const, text: "Pins: not known whether they match." },
    ],
  };
}

/**
 * The project's own boards as the fake engine reads them from the board
 * files in its boards folder (on the fake disk, `fake.customBoardsOnDisk`,
 * board ID to file text): enough of Logix Engine's rules to test the UI
 * (valid JSON, an object, board IDs, an FPGA, variants with a selector),
 * with each problem at line 1 of its file, and the FPGA's family taken from
 * its ID. Files are named as the IDE asks the engine to (display_paths):
 * the folder's full path when the project is `saved`, `boards` when it
 * isn't, then the file name.
 */
function fakeProjectBoards(
  files: Record<string, string> | null,
  folder: string | null,
  saved: boolean,
): Real.ProjectBoards {
  if (files === null) return { boards: [], problem: null };
  const base = saved ? `${folder}/boards` : "boards";
  const problem = (message: string, id: string): Real.ProjectBoards => {
    const file = `${base}/${id}.json`;
    return {
      boards: [],
      problem: {
        message: `The custom boards folder has problems. ${file}:1:1: ${message}`,
        diagnostics: [{ severity: "error", file, line: 1, column: 1, message }],
      },
    };
  };
  const boards: Record<string, unknown> = {};
  for (const [id, text] of Object.entries(files)) {
    let board: unknown;
    try {
      board = JSON.parse(text);
    } catch {
      return problem("Expected value.", id);
    }
    if (typeof board !== "object" || board === null || Array.isArray(board)) {
      return problem("Expected a board: an object with name, fpga_id and variants.", id);
    }
    boards[id] = board;
  }
  try {
    // As the engine's get_catalog merges them: the file's boards with the
    // catalog's, one with a catalog board's ID replacing it, by description.
    const own = fakeBoards(boards as Record<string, unknown>);
    const ids = new Set(own.map((b) => b.id));
    const merged = [...own, ...BOARDS.filter((b) => !ids.has(b.id))].sort((x, y) =>
      x.description.toLowerCase().localeCompare(y.description.toLowerCase()),
    );
    return { boards: merged, problem: null };
  } catch (e) {
    const { message, id } = e as { message: string; id: string };
    return problem(message, id);
  }
}

function fakeBoards(boards: Record<string, unknown>): Real.Board[] {
  let current = "";
  const fail = (message: string): never => {
    throw { message, id: current };
  };
  // The family of an FPGA ID, by the start of its part number.
  const prefixes: [string, string][] = [
    ["ice40", "ice40"], ["ice5", "ice40"], ["lfe5", "ecp5"], ["gw1n", "gw1n"], ["gw2a", "gw2a"],
    ["gw5a", "gw5a"], ["xc7a", "artix7"], ["xc7s", "spartan7"], ["xc7z", "zynq7000"],
  ];
  const family = (fpga: string) => prefixes.find(([p]) => fpga.startsWith(p))?.[1] ?? "";
  // Value labels, as the catalog's boards have them.
  const labels = new Map<string, Real.Dimension["values"][number]>();
  for (const b of BOARDS) for (const d of b.dimensions) for (const v of d.values) labels.set(`${d.id}:${v.id}`, v);
  return Object.entries(boards).map(([id, item]) => {
    current = id;
    const b = { ...(item as object), id } as { id: string; name?: string; fpga_id?: string; variants?: { selector: { connection: string; memory: string }; tested: boolean }[] };
    if (!/^[a-z0-9-]+$/.test(b.id)) fail(`${JSON.stringify(b.id)} isn't a board ID: use lowercase letters, digits and hyphens.`);
    if (!b.fpga_id) fail("Missing field `fpga_id`.");
    if (!b.variants?.length) fail("A board needs at least one variant.");
    if (b.variants!.some((v) => !v.selector)) fail("Missing field `selector`.");
    const fam = family(b.fpga_id!);
    const like = BOARDS.find((x) => x.family === fam)!;
    const dims = ["connection", "memory"].map((id) => ({
      id,
      label: id === "connection" ? "Connection" : "Memory",
      values: [...new Set(b.variants!.map((v) => (id === "connection" ? v.selector.connection : v.selector.memory)))].map(
        (v) => labels.get(`${id}:${v}`) ?? { id: v, label: v },
      ),
    }));
    const sameFpga = BOARDS.find((x) => x.fpga?.id === b.fpga_id)?.fpga;
    return {
      ...like,
      id: b.id!,
      description: b.name ?? b.id!,
      fpga: sameFpga ?? { id: b.fpga_id!, partNumber: b.fpga_id!.toUpperCase(), size: "" },
      dimensions: dims,
      configurations: b.variants!.map((v) => ({
        when: { connection: v.selector.connection, memory: v.selector.memory },
        // As the engine labels it, from the dimension labels.
        label: [
          dims[0]?.values.find((x) => x.id === v.selector.connection)?.label ?? v.selector.connection,
          dims[1]?.values.find((x) => x.id === v.selector.memory)?.label ?? v.selector.memory,
        ].join(" · "),
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

/** A board's Bare Minimum (also File > New Project). */
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
  gtkwaveOpened?: (found: boolean) => void;
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
  /** The folder Show Project Folder showed last (`showProjectFolder`). */
  shownFolder: null as string | null,
  /** The text System Info's Copy copied last (`copyText`). */
  copiedText: null as string | null,
  /** The path Copy Project Folder Path copied last (`copyProjectPath`). */
  copiedPath: null as string | null,
  /** The confirmation dialogs shown (`askConfirm`), oldest first. */
  confirms: [] as { title: string; okLabel: string; kind: string }[],
  /** What the user answers in confirmation dialogs (`askConfirm`). */
  confirmAnswer: true,
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
  /** The boards chosen most recently, newest first (`setLastBoard`). */
  recentBoards: [] as string[],
  /**
   * The boards folders on the fake disk, by project folder: board ID to
   * the text of its file, which the user edits outside Logix (change it to
   * simulate an edit).
   */
  customBoardsOnDisk: {} as Record<string, Record<string, string>>,
  /** The folder whose boards folder the last task run used (`runTask`). */
  taskBoardsFolder: null as string | null,
  /** The folder whose boards folder the last connected-devices scan used. */
  scannedBoardsFolder: null as string | null,
  /** The variant of the last task run (`runTask`). */
  taskVariant: null as Real.Variant | null,
  /** The board and variant last saved (`saveProject`, Save As). */
  saved: null as { board: string | null; variant: Real.Variant } | null,
  /** Ends a setup held at the engine's package download (`/?engineSetup`). */
  finishSetup: () => handlers.finishSetup?.(),
  /** GTKWave's window appeared after Simulate (true), or not in time (false). */
  gtkwaveOpened: (found: boolean) => handlers.gtkwaveOpened?.(found),
};
(window as unknown as { __fake: typeof fake }).__fake = fake;

/** The board files in `folder`'s boards folder on the fake disk (null without it). */
function boardsOnDisk(folder: string | null): Record<string, string> | null {
  return folder === null ? null : (fake.customBoardsOnDisk[folder] ?? null);
}

/** A real Alhambra II, as Logix Engine lists it. */
const alhambraDevice: Real.UsbDevice = {
  id: "usb:1-1:0403:6010:FTATANKC",
  label: "Alhambra II v1.0A (AlhambraBits, 0403:6010)",
  vid: "0403",
  pid: "6010",
  manufacturer: "AlhambraBits",
  product: "Alhambra II v1.0A",
  serialNumber: "FTATANKC",
  boards: [
    { board: "alhambra-ii", level: "strong" },
    { board: "alchitry-au", level: "weak" },
  ],
  likelyBoards: ["alhambra-ii"],
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
      // Real Logix Engine output for this blinky on the Alhambra II.
      report: {
        family: "ice40",
        resources: [
          { name: "ICESTORM_LC", description: "Logic cell: a 4-input LUT, a flip-flop and carry logic", used: 27, available: 7680, percent: 0.4, displayPercent: "1%" },
          { name: "ICESTORM_PLL", description: "PLL (clock generator)", used: 0, available: 2, percent: 0.0, displayPercent: "0%" },
          { name: "ICESTORM_RAM", description: "Block RAM, 4 Kbit", used: 0, available: 32, percent: 0.0, displayPercent: "0%" },
          { name: "SB_GB", description: "Global buffer, for clocks and other high-fanout signals", used: 1, available: 8, percent: 12.5, displayPercent: "13%" },
          { name: "SB_IO", description: "I/O pin", used: 2, available: 107, percent: 1.9, displayPercent: "2%" },
          { name: "SB_WARMBOOT", description: "Warm boot: loads another bitstream from flash", used: 0, available: 1, percent: 0.0, displayPercent: "0%" },
        ],
        clocks: [{ name: "sys_clk", net: "sys_clk$SB_IO_IN_$glb_clk", fmaxMhz: 194.36 }],
        text: "The design uses 27 of 7680 logic cells (1%).\nThe design uses 2 of 107 I/O pins (2%).\nMaximum clock speed: 194.4 MHz (sys_clk).",
        summaryLine: "Uses 1% of logic cells, max clock 194.4 MHz.",
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
    // With `/?engineSetup`, setup stops in the engine's package download
    // until a test calls `__fake.finishSetup()`.
    if (new URLSearchParams(location.search).has("engineSetup")) {
      onEvent({ kind: "stage", stage: "downloadingEnginePackages" });
      onEvent({ kind: "log", line: "Downloading oss-cad-suite..." });
      onEvent({
        kind: "progress",
        message: "Downloading oss-cad-suite",
        percent: 42,
      });
      await new Promise<void>((resolve) => (handlers.finishSetup = resolve));
    }
    await pause(50);
    onEvent({ kind: "stage", stage: "ready" });
  },

  getBoards: async () => BOARDS,
  getFpgaFamilies: async () => FPGA_FAMILIES,

  getExamples: async () => ({
    examples: catalog.examples,
    hidden: catalog.hidden,
  }),

  openBareMinimum: async (board) => bareDoc(board),
  openExample: async (board) => blinkyDoc(board),

  runTask: async (task, target, _key, sourceDir, _projectDir, files, board, variant, device, onEvent) => {
    fake.taskVariant = variant;
    fake.taskBoardsFolder = sourceDir;
    // As the IDE's backend: a file with problems stops the task, its
    // problems in the console at line and column.
    const own = fakeProjectBoards(boardsOnDisk(sourceDir), sourceDir, _projectDir != null);
    if (own.problem) {
      for (const line of boardsProblemLines(own.problem)) onEvent({ kind: "line", line });
      return {
        ok: false, diagnostics: own.problem.diagnostics, testbenches: [], report: null,
        cancelled: false, graphSvg: null, seconds: 0.1, action: null, devices: [],
      };
    }
    const onLine = (line: string) => onEvent({ kind: "line", line });
    const { lines: buildLines, outcome } = buildOutcome(files, board);
    let lines = buildLines;
    if (outcome.ok && (task === "test" || task === "simulate")) {
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
      report: "Synthesizing",
    }[task];
    const phases = task === "build" || task === "upload" || task === "report";
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
      // The programmer's progress, printed over itself (the engine's
      // redraw flag), as iceprog prints it.
      const prog = (line: string, redraw = false) =>
        onEvent({ kind: "line", line, tool: "iceprog", redraw });
      prog("programming..");
      prog("addr 0x000000   0%", true);
      await pause(5);
      prog("addr 0x000100  50%", true);
      await pause(500);
      prog("done.");
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
  // The fake's recent projects are all UPduino v3.1 projects (openProjectAt).
  setLastBoard: async (board) => {
    fake.recentBoards = [board, ...fake.recentBoards.filter((b) => b !== board)].slice(0, 5);
    return fake.recentBoards;
  },
  recentBoards: async () => fake.recentBoards,
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

  // A saved project for the UPduino v3.1 (any folder), so tests can check that
  // opening it brings its own board.
  openProjectAt: async (dir) => {
    const doc = bareDoc("upduino31");
    return {
      ...doc,
      files: doc.files,
      dir,
      name: dir.split("/").pop() ?? dir,
    };
  },
  showProjectFolder: async (dir) => {
    fake.shownFolder = dir;
  },
  systemInfo: async () =>
    [
      "Logix IDE " + packageJson.version,
      "Operating system: macOS 26.5 (macos-arm64)",
      "Log file: /Users/you/.logix/logs/logix.log",
      "Logix Engine used: release 2026-11-08",
      "",
      "Logix Engine 2026-11-08",
      "Platform: darwin-arm64",
      "Boards schema: https://www.openlogix.org/schemas/logix-board-v1.json",
    ].join("\n"),
  copyText: async (text) => {
    fake.copiedText = text;
  },
  copyProjectPath: async (dir) => {
    fake.copiedPath = dir;
    return `'${dir}'`; // as the real one quotes a path with a space
  },
  saveGtkwaveLayouts: async () => [],
  cancelTask: async () => false,

  // No language server in the fake: it never answers, so the UI quietly
  // runs without live errors in these tests and screenshots.
  lspStart: async () => {},
  lspSend: async () => {},
  lspStop: async () => {},
  onLspMessage: unlisten,
  onLspClosed: unlisten,
  onGtkwaveOpened: async (handler) => {
    handlers.gtkwaveOpened = handler;
    return () => {};
  },

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
    if (!/\.(s?vh?|pcf|lpf|cst|xdc)$/i.test(n)) {
      return "Use one of these extensions: .v .sv .vh .svh .pcf .lpf .cst .xdc.";
    }
    if (existing.some((e) => e.toLowerCase() === n.toLowerCase())) {
      return `There's already a file named "${n}".`;
    }
    return null;
  },

  askConfirm: async (title, _message, okLabel, kind = "warning") => {
    fake.confirms.push({ title, okLabel, kind });
    return fake.confirmAnswer;
  },
  askSaveChanges: async () => "discard",
  setDocumentState: async () => {},
  closeWindow: async () => {},
  logOutput: async () => {},
  connectedDevices: async (folder) => {
    fake.scannedBoardsFolder = folder;
    return fake.connected;
  },
  projectBoards: async (folder, saved) => fakeProjectBoards(boardsOnDisk(folder), folder, saved),
  compareBoards: async (from, to, folder, saved) => {
    const own = fakeProjectBoards(boardsOnDisk(folder), folder, saved).boards;
    const find = (id: string) => own.find((b) => b.id === id) ?? BOARDS.find((b) => b.id === id);
    return fakeCompare(find(from), find(to));
  },
  // The text itself: it changes whenever the file does.
  customBoardsStamp: async (folder) => {
    const files = boardsOnDisk(folder);
    return files === null ? null : JSON.stringify(files);
  },
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
