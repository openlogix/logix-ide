// A fake of src/lib/backend.ts for the end-to-end UI tests (Playwright). In
// "e2e" mode, Vite replaces the real backend with this (see
// vite.config.ts), so the real UI runs in a plain browser without Tauri or
// Logix Engine. It behaves like a small, predictable Logix IDE backend:
// - the FPGA tools are ready right away, with Logix Engine's real boards,
//   Logix IDE's examples, and outputs replayed from real engine runs
//   (e2e/fixtures);
// - a build fails if main.v contains "oops" (with an error on that line);
// - Auto Format removes trailing spaces.
// Tests drive native-menu actions through `window.__fake.menu(id)`.

import type * as Real from "../src/lib/backend";
import type {
  Diagnostic,
  InfoDetail,
  MenuId,
  ProjectDoc,
  ProjectFile,
  TaskEvent,
  TaskOutcome,
  VersionEntry,
} from "../src/lib/backend";

// Everything except `backend` comes from the real module (types, toUserError).
export * from "../src/lib/backend";

// Real engine data and output (see e2e/fixtures/README.md).
import catalog from "./fixtures/catalog.json";
import fpgaFamilies from "./fixtures/fpgas.json";
import { configurationOf, resolve } from "../src/lib/variants";
import { boardsProblemLines } from "../src/lib/boards";
import verifyOutput from "./fixtures/verify.json";
import verifyErrorOutput from "./fixtures/verify-error.json";
import testOutput from "./fixtures/test.json";
import diagramOutput from "./fixtures/diagram.json";
import verifyReport from "./fixtures/verify-report.json";
// The iCESugar v1.5's Blinky and Bare Minimum, as Logix IDE ships them
// (the default board: the fixtures are captured for it).
import sugarBlinkyMain from "../../examples/icesugar-1-5/blinky/main.v?raw";
import sugarBlinkyTb from "../../examples/icesugar-1-5/blinky/main_tb.v?raw";
import sugarBlinkyPins from "../../examples/icesugar-1-5/blinky/pinout.pcf?raw";
import sugarBareMain from "../../examples/icesugar-1-5/bare-minimum/main.v?raw";
import sugarBarePins from "../../examples/icesugar-1-5/bare-minimum/pinout.pcf?raw";
import sugarBareTb from "../../examples/icesugar-1-5/bare-minimum/main_tb.v?raw";
import sugarSerialMain from "../../examples/icesugar-1-5/serial/main.v?raw";
import sugarSerialUart from "../../examples/icesugar-1-5/serial/uart.v?raw";
import sugarSerialTb from "../../examples/icesugar-1-5/serial/main_tb.v?raw";
import sugarSerialUartTb from "../../examples/icesugar-1-5/serial/uart_tb.v?raw";
import sugarSerialPins from "../../examples/icesugar-1-5/serial/pinout.pcf?raw";
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
      fpga: sameFpga ?? { id: b.fpga_id!, partNumber: b.fpga_id!.toUpperCase(), size: "", label: b.fpga_id!.toUpperCase() },
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
/** A console line, as the backend streams it (`TaskEvent` "line"). */
type Line = Extract<TaskEvent, { kind: "line" }>;
/** The console's events as `capture_fixtures` recorded them from Logix Engine. */
const outputLines = (events: unknown) => events as Line[];
/** A line of Logix IDE's or the engine's own. */
const plain = (line: string): Line => ({ kind: "line", line });

/**
 * The files Logix Engine reads for a task (its `checked_files`): the
 * design files, the testbenches for Run Tests and Simulate, and the pin
 * file for builds.
 */
function checkedFiles(files: ProjectFile[], task: string): string[] {
  const builds = task === "build" || task === "upload" || task === "report" || task === "debug";
  const tests = task === "test" || task === "simulate";
  return files
    .map((f) => f.name)
    .filter((n) =>
      /_tb\.s?v$/i.test(n) ? tests : /\.s?vh?$/i.test(n) ? true : /\.(pcf|lpf|cst|xdc)$/i.test(n) ? builds : false,
    );
}

/** The file formats, as Logix Engine gives them (`file_formats`). */
const FILE_FORMATS: Real.FileFormats = {
  source: ["v", "sv"],
  include: ["vh", "svh"],
  pins: FPGA_FAMILIES.flatMap((f) =>
    f.pinExtension && f.pinComment ? [{ extension: f.pinExtension, comment: f.pinComment }] : [],
  ).filter((p, i, all) => all.findIndex((q) => q.extension === p.extension) === i),
};

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

/** The default board: the iCESugar v1.5, as in Logix IDE. */
const DEFAULT_BOARD = "icesugar-1-5";

/** The blinky example for `board`, as an unsaved project: the iCESugar
 * v1.5's real one, else a small generic one. */
function blinkyDoc(board: string): ProjectDoc {
  const sugar = board === DEFAULT_BOARD;
  return {
    dir: null,
    templateDir: `/fake/examples/${board}/blinky`,
    name: "blinky",
    files: sugar
      ? [
          { name: "main.v", text: sugarBlinkyMain },
          { name: "main_tb.v", text: sugarBlinkyTb },
          { name: "pinout.pcf", text: sugarBlinkyPins },
        ]
      : [
          { name: "main.v", text: BLINKY_MAIN },
          { name: "main_tb.v", text: BLINKY_TB },
          { name: "pinout.pcf", text: "set_io sys_clk 49\nset_io led 45\n" },
        ],
    board,
    variant: {},
    serialBaud: null,
  };
}

/** The iCESugar v1.5's real serial example (its views left out). */
function serialDoc(board: string): ProjectDoc {
  return {
    dir: null,
    templateDir: `/fake/examples/${board}/serial`,
    name: "serial",
    files: [
      { name: "main.v", text: sugarSerialMain },
      { name: "uart.v", text: sugarSerialUart },
      { name: "main_tb.v", text: sugarSerialTb },
      { name: "uart_tb.v", text: sugarSerialUartTb },
      { name: "pinout.pcf", text: sugarSerialPins },
    ],
    board,
    variant: {},
    serialBaud: null,
  };
}

/** A board's Bare Minimum (also File > New Project). */
function bareDoc(board: string): ProjectDoc {
  return {
    dir: null,
    templateDir: null,
    name: "untitled",
    files:
      board === DEFAULT_BOARD
        ? [
            { name: "main.v", text: sugarBareMain },
            { name: "main_tb.v", text: sugarBareTb },
            { name: "pinout.pcf", text: sugarBarePins },
          ]
        : [
            { name: "main.v", text: "module main (\n    input  sys_clk,\n    output led\n);\n\n  assign led = 0;\n\nendmodule\n" },
            { name: "pinout.pcf", text: "set_io sys_clk 49\nset_io led 45\n" },
          ],
    board,
    variant: {},
    serialBaud: null,
  };
}

let preferences: Real.Preferences = {
  projectsDir: "/Users/you/Documents/OpenLogix/IDE",
  editorFontSize: 13,
  uiScale: null,
  theme: "system",
  lintOnVerify: true,
  liveChecking: true,
  hoverInfo: false,
  showOutput: "onCommand",
  autoSave: true,
};

const handlers: {
  menu?: (id: MenuId) => void;
  output?: (event: TaskEvent) => void;
  finishSetup?: () => void;
  finishUpdates?: () => void;
  waveView?: (view: { key: string; name: string; text: string }) => void;
  debugEnded?: () => void;
  waveOpenSource?: (at: { key: string; file: string; line: number }) => void;
  projectFilesChanged?: (change: { dir: string; names: string[] }) => void;
  diagramOpenSource?: (at: { key: string; file: string; line: number }) => void;
  serial?: (event: Real.SerialEvent) => void;
  listsChanged?: () => void;
} = {};

/** Test hooks, reachable from Playwright via `page.evaluate`. */
const fake = {
  menu: (id: MenuId) => handlers.menu?.(id),
  /** The Serial Monitor: the ports opened (with their speed) and what was sent. */
  serialOpened: [] as { port: string; baud: number; device: string | null }[],
  serialSent: [] as string[],
  /** Text from the board, as Logix Engine's read_serial would return it. */
  serialReceive: (text: string, events: string[] = []) =>
    handlers.serial?.({ kind: "read", text, events, open: events.length === 0, port: fake.serialOpened.at(-1)?.port ?? "" }),
  /** The engine's session ended (such as an engine restart). */
  serialEnd: (message: string) => handlers.serial?.({ kind: "ended", message }),
  /** The speed Save wrote (`saveProject`). */
  savedBaud: undefined as number | null | undefined,
  /** Connects a real Alhambra II, with its captured USB names, in place of
   * the default iCESugar v1.5: an FTDI board with two ports and a driver,
   * for the tests that need them. */
  useAlhambra: () => {
    fake.usb = [alhambraDevice];
    fake.uploadDevices = [alhambraDevice];
  },
  /** USB serial ports Logix Engine tied to no device (its
   * other_serial_ports), such as two lookalike CH340 adapters. */
  otherPorts: [] as { port: string; interface: number | null; description: string }[],
  /** The module the last editor right-click menu offered the diagram of. */
  editorMenuModule: null as string | null,
  /** The project and module the last Show Diagram opened its window on. */
  diagramShown: null as { key: string; module: string | null } | null,
  /** A click on a block in the Diagram window. */
  diagramClick: (key: string, file: string, line: number) =>
    handlers.diagramOpenSource?.({ key, file, line }),
  /** The start screen's recent projects (`recentProjects`); `?recents=a,b`
   * sets them from the start. */
  recents: (new URLSearchParams(location.search).get("recents") ?? "")
    .split(",")
    .filter(Boolean),
  /** The USB devices Tools > Connected Devices lists (`scanDevices`); one iCESugar v1.5 by default. */
  usb: null as Real.UsbDevice[] | null,
  /** A newer Logix IDE for the banner (`newerIdeNotice`); `?newerIde` sets one. */
  newerIde: (new URLSearchParams(location.search).has("newerIde")
    ? {
        version: "2026.12.01",
        current: "2026.11.22",
        url: "https://github.com/openlogix/logix-ide/releases/tag/2026-12-01",
      }
    : null) as Real.NewerIde | null,
  /** How many times the banner was dismissed (`dismissNewerIde`). */
  newerDismissed: 0,
  /** How many times the banner's Download opened the page (`openNewerIdePage`). */
  newerPageOpened: 0,
  /** The folder Show Project Folder showed last (`showProjectFolder`). */
  shownFolder: null as string | null,
  /** The window's title state last set (`setDocumentState`). */
  documentState: null as { name: string; dirty: boolean } | null,
  /** The last task's project (its build key), as the Waveforms window's events name it. */
  lastRun: null as { key: string; task?: string; target?: string | null } | null,
  /** The text System Info's Copy copied last (`copyText`). */
  copiedText: null as string | null,
  /** The path Copy Project Folder Path copied last (`copyProjectPath`). */
  copiedPath: null as string | null,
  /** The confirmation dialogs shown (`askConfirm`), oldest first. */
  confirms: [] as { title: string; okLabel: string; kind: string }[],
  /** What the user answers in confirmation dialogs (`askConfirm`). */
  confirmAnswer: true,
  /** When set, a Save into the project's folder fails with this error (`saveProject`). */
  saveError: null as Real.UserError | null,
  /** The projects "Save changes?" asked about (`askSaveChanges`), and its answer. */
  saveChangesAsked: [] as string[],
  saveChangesAnswer: "discard" as Real.SaveChoice,
  /** When set, Save As fails with this error (`saveProjectAsDialog`). */
  saveAsError: null as Real.UserError | null,
  /** Devices reported as plugged in, with their boards (`connectedDevices`). */
  connected: [] as Real.ConnectedDevice[],
  /** The devices Upload finds that fit the board; with several, Upload
   * asks which one (`select_device`). One Alhambra II by default. */
  uploadDevices: null as Real.UsbDevice[] | null,
  /** The device id the last Upload used. */
  uploadedDevice: null as string | null,
  /**
   * Windows: the driver Upload's device has (`driver` in list_devices); a
   * driver that isn't ready makes Upload ask to install WinUSB first.
   */
  uploadDriver: null as RawDriver | null,
  /**
   * When set, Logix Engine refuses to install a driver without
   * `allowSameIds`, with this message (other devices with the same USB IDs
   * are connected).
   */
  sameIdsMessage: null as string | null,
  /** An engine error the next driver change fails with (`{message, details}`). */
  driverError: null as { message: string; details: string } | null,
  /** Devices that can't be opened yet (`list_devices` `unreachable`). */
  unreachable: [] as (Omit<Real.UnreachableDevice, "driver" | "who" | "fix" | "note" | "rank"> & { driver: RawDriver | null; who?: string; fix?: string })[],
  /** While set, Install Driver waits for it, as for Windows' prompt. */
  driverHold: null as Promise<void> | null,
  /** The status texts Upload sent while installing a driver. */
  driverStatuses: [] as string[],
  /** The drivers installed (`install_driver`), oldest first. */
  driverInstalls: [] as { device: string; allowSameIds: boolean }[],
  /** The devices whose drivers were removed (`uninstall_driver`). */
  driverRemovals: [] as string[],
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
   * the text of its file, which the user edits outside Logix IDE (change it to
   * simulate an edit).
   */
  customBoardsOnDisk: {} as Record<string, Record<string, string>>,
  /** The design's nets, as Logix Engine's list_nets gives them (`listDebugNets`). */
  debugNets: [
    { path: "main.clk", width: 1, kind: "input" },
    { path: "main.count", width: 24, kind: "internal" },
    { path: "main.led", width: 1, kind: "output" },
  ] as Real.DebugNet[],
  /** Each choice of the net picker, as `updateDebugConfig` wrote it into debug.toml. */
  debugUpdates: [] as { clock: string; watch: string[]; drive: string[] }[],
  /** The debug session the last Debug opened, until Stop, Upload or the window closes. */
  debugSession: null as string | null,
  /** How often Stop ended a debug session (`stopDebug`). */
  debugStops: 0,
  /** The folder whose boards folder the last task run used (`runTask`). */
  taskBoardsFolder: null as string | null,
  /** The folder whose boards folder the last connected-devices scan used. */
  scannedBoardsFolder: null as string | null,
  /** The variant of the last task run (`runTask`). */
  taskVariant: null as Real.Variant | null,
  /** The board and variant last saved (`saveProject`, Save As). */
  saved: null as { board: string | null; variant: Real.Variant } | null,
  /** Each Save into the project's folder (`saveProject`): the files written and deleted. */
  saves: [] as { files: string[]; removed: string[] }[],
  /** Ends a setup held at the engine's package download (`/?engineSetup`). */
  finishSetup: () => handlers.finishSetup?.(),
  /** Check for Updates waits at 42% while this is set, until finishUpdates(). */
  holdUpdates: false,
  finishUpdates: () => handlers.finishUpdates?.(),
  /** Check for Updates brings a new example (Alhambra II's fresh-from-update). */
  updateBringsExample: false,
  /** The Waveforms window saved its view (as the real backend's "wave-view" event). */
  waveView: (view: { key: string; name: string; text: string }) => handlers.waveView?.(view),
  /** The Debugger window was closed (the backend's "debug-ended"). */
  debugEnded: () => handlers.debugEnded?.(),
  /** A source line asked for in the Waveforms window ("wave-open-source"). */
  waveOpenSource: (at: { key: string; file: string; line: number }) => handlers.waveOpenSource?.(at),
  /** The folder Logix IDE watches (`watchProject`), or null. */
  watchedDir: null as string | null,
  /** Another program changes the watched folder: each file's new text, or null for deleted; a board for logix-ide.toml. */
  diskChange: (
    files: { name: string; text: string | null }[],
    settings: Real.ProjectChanges["settings"] = null,
  ) => {
    if (!fake.watchedDir) return;
    pendingDisk = { files, settings };
    const names = files.map((f) => f.name);
    if (settings) names.push("logix-ide.toml");
    handlers.projectFilesChanged?.({ dir: fake.watchedDir, names });
  },
};

/** What `readProjectChanges` returns next (`fake.diskChange`). */
let pendingDisk: Real.ProjectChanges = { files: [], settings: null };
(window as unknown as { __fake: typeof fake }).__fake = fake;

/** The engine's logix-catalog in System Info; Check for Updates moves it. */
let catalogRelease = "2026.11.14";
/** Examples Check for Updates brought (`fake.updateBringsExample`). */
const updateExamples: Real.Example[] = [];

/** The board files in `folder`'s boards folder on the fake disk (null without it). */
function boardsOnDisk(folder: string | null): Record<string, string> | null {
  return folder === null ? null : (fake.customBoardsOnDisk[folder] ?? null);
}


// ---- Logix Engine's driver texts --------------------------------------
// What Logix Engine adds to a device and its driver for people (its
// src/driver_texts.rs, who in list_devices): the fake's copy, so tests can
// set up devices with the driver's raw fields, as captured from real
// boards, and see what the engine sends.

/** A driver's raw fields, as tests set them up. */
type RawDriver = { needed: string; interface: number | null; current: string | null; ready: boolean };

function windowsName(service: string): string {
  return (
    { winusb: "WinUSB", libusbk: "libusbK", libusb0: "libusb0", ftdibus: "FTDI's driver", usbser: "the serial port driver" } as Record<string, string>
  )[service] ?? service;
}

/** A device's name for people, as Logix Engine's who. */
function engineWho(label: string, boards: string[], chosen: string | null): string {
  const name = (id: string) => BOARDS.find((b) => b.id === id)?.description ?? id;
  if (chosen !== null && boards.includes(chosen)) return `the ${name(chosen)}`;
  return boards.length === 1 ? `the ${name(boards[0])}` : label;
}

/** A driver with Logix Engine's texts and questions, for the device `who`. */
function engineDriver(d: RawDriver, who: string): Real.Driver {
  const udev = d.needed === "udev-rule";
  const currentName = !udev && d.current ? windowsName(d.current) : null;
  const canAdd = !d.ready;
  const canRemove = udev ? d.current !== null : d.ready;
  const state = udev
    ? !d.ready
      ? "Access: Linux needs a udev rule before the board can be programmed."
      : d.current
        ? `Access: ready to program the board, with Logix Engine's udev rule (${d.current}).`
        : "Access: ready to program the board."
    : d.ready
      ? `Driver: ${currentName ?? windowsName(d.needed)}, ready to program the board.`
      : `Driver: needs WinUSB on USB interface ${d.interface ?? 0} to program the board (${currentName ? `it has ${currentName} now` : "it has no driver now"}).`;
  const add: Real.Consent = udev
    ? {
        title: "Let your user account program the board?",
        lead: `To program ${who}, Linux needs a udev rule that lets your user account use it.`,
        steps: [
          "Linux asks for your password. Enter it to allow the change.",
          "The rule is only for this kind of board (its USB IDs): other connected boards of the same kind can be programmed too.",
          "Nothing else on your computer changes.",
        ],
        note: "You can remove the rule later.",
        confirm: "Add udev Rule",
        busy: "Adding the board's udev rule…",
        cancelled: "Cancelled: the udev rule wasn't added.",
      }
    : {
        title: "Install the board's USB driver?",
        lead: `To program ${who}, Windows needs to use the WinUSB driver for it.${currentName ? ` Right now Windows uses ${currentName} for it.` : ""}`,
        steps: [
          "Windows asks whether Logix Engine may make changes to your device. Choose Yes.",
          "It takes a minute or two.",
          "Only the part of the board that programs the FPGA changes. Its other parts, such as a serial port, keep working as before.",
          "Other connected boards of the same kind get the driver too.",
        ],
        note: "You can remove the driver later.",
        confirm: "Install Driver",
        busy: "Installing the board's driver, which may take a minute or two…",
        cancelled: "Cancelled: the driver wasn't installed.",
      };
  const remove: Real.Consent = udev
    ? {
        title: "Remove the board's udev rule?",
        lead: `This removes Logix Engine's udev rule for ${who}.`,
        steps: [
          "Linux asks for your password. Enter it to allow the change.",
          "Then your user account can't program it, unless another rule allows it.",
          "To program the board again, add the rule again.",
        ],
        note: null,
        confirm: "Remove udev Rule",
        busy: "Removing the board's udev rule…",
        cancelled: "Cancelled: the udev rule wasn't removed.",
      }
    : {
        title: "Remove the board's USB driver?",
        lead: `This restores the original Windows driver for ${who}, if it had one, so the board maker's own tools can use it again.`,
        steps: [
          "Windows asks whether Logix Engine may make changes to your device. Choose Yes.",
          "Other connected boards of the same kind get their original driver back too.",
          "To program the board again, install WinUSB again.",
        ],
        note: null,
        confirm: "Remove Driver",
        busy: "Removing the board's driver…",
        cancelled: "Cancelled: the driver wasn't removed.",
      };
  return {
    needed: d.needed, interface: d.interface, current: d.current, ready: d.ready,
    label: udev ? "a udev rule" : "the WinUSB driver",
    state,
    tag: d.ready ? null : udev ? "no udev rule yet" : "no driver yet",
    canAdd, canRemove,
    add: canAdd ? add : null,
    remove: canRemove ? remove : null,
  };
}

/** A device as Logix Engine lists it: its who, and its driver's texts. */
function engineDevice<T extends { label: string; boards: { board: string }[]; driver: RawDriver | null }>(
  d: T,
  chosen: string | null,
): Omit<T, "driver"> & { who: string; driver: Real.Driver | null } {
  const who = engineWho(d.label, d.boards.map((b) => b.board), chosen);
  return { ...d, who, driver: d.driver ? engineDriver(d.driver, who) : null };
}

/**
 * Logix Engine's words and order for a device list asked about `board`
 * (as its list_devices gives them): each device's `rank` (those that may be
 * the board, then those that may be other boards, then the rest, devices
 * before unreachable ones within each), `other_boards` (with a strong
 * match, only the board when the device may be it) and an unreachable
 * device's `note`.
 */
function engineList<U extends Omit<Real.UsbDevice, "otherBoards" | "rank">, L extends Omit<Real.UnreachableDevice, "note" | "rank">>(
  usb: U[],
  unreachable: L[],
  board: string | null,
): Real.Devices {
  const name = (id: string) => BOARDS.find((b) => b.id === id)?.description ?? id;
  const ids = (d: { boards: { board: string }[] }) => d.boards.map((b) => b.board);
  const group = (d: { boards: { board: string }[] }) =>
    board !== null && ids(d).includes(board) ? 0 : ids(d).length > 0 ? 1 : 2;
  const all = [...usb.map((d, i) => ({ d, locked: 0, i })), ...unreachable.map((d, i) => ({ d, locked: 1, i }))];
  const order = [...all].sort((a, b) => group(a.d) - group(b.d) || a.locked - b.locked || a.i - b.i);
  const rank = (d: object) => order.findIndex((x) => x.d === d);
  return {
    usb: usb.map((d) => {
      const rest = ids(d).filter((b) => !d.likelyBoards.includes(b));
      const strong = d.boards.some((b) => b.level === "strong");
      return { ...d, otherBoards: strong ? rest.filter((b) => b === board) : rest, rank: rank(d) };
    }),
    unreachable: unreachable.map((u) => {
      const boards = ids(u);
      const note =
        board !== null && boards.includes(board)
          ? `This may be your ${name(board)}: it has the same USB IDs. ${u.fix}`
          : boards.length === 1
            ? `This may be an FPGA board, the ${name(boards[0])}: it has its USB IDs. ${u.fix}`
            : `This may be an FPGA board: ${boards.length} boards have its USB IDs. ${u.fix}`;
      return { ...u, note, rank: rank(u) };
    }),
  };
}

/** Logix Engine's question when other devices have the same USB IDs (its data.consent). */
function sameIdsConsent(lead: string): Real.Consent {
  return {
    title: "Other devices of the same kind are connected",
    lead,
    steps: [],
    note: "Unplug them and try again, or continue: they get the same driver too.",
    confirm: "Continue for All",
    busy: "Installing the board's driver, which may take a minute or two…",
    cancelled: "Cancelled: the driver wasn't installed.",
  };
}

/** The fix for people Logix Engine gives an unreachable device, on the platform its driver says. */
function engineFix(driver: RawDriver | null): string {
  return driver?.needed === "winusb"
    ? "It needs its driver first, so it can be programmed."
    : "Add a udev rule for it, so it can be programmed and its name read.";
}

/** An iCESugar v1.5, as Logix Engine lists it: captured from a real board
 * (2026-11-22, Logix Engine 2026.11.21 on macOS), its serial number's last
 * digits zeroed. Its programmer, a DAPLink, names itself; its UART is USB
 * interface 2. */
const icesugarDevice: Real.UsbDevice = {
  id: "usb:1-1.4.2:1d50:602b:07000001004300413100001056313032a5a5a5a500000000",
  label: "DAPLink CMSIS-DAP (MuseLab, 1d50:602b)",
  vid: "1d50",
  pid: "602b",
  manufacturer: "MuseLab",
  product: "DAPLink CMSIS-DAP",
  serialNumber: "07000001004300413100001056313032a5a5a5a500000000",
  release: "0100",
  boards: [{ board: "icesugar-1-5", level: "strong" }],
  likelyBoards: ["icesugar-1-5"],
  otherBoards: [],
  rank: 0,
  buildMatch: "strong", driver: null, who: "the iCESugar v1.5",
  // The DAPLink's one serial port, the board's UART.
  serialPorts: [
    { port: "/dev/cu.usbmodem114202", interface: 2, role: "uart", description: "The iCESugar v1.5's serial port" },
  ],
  serialPort: "/dev/cu.usbmodem114202",
};


/** A real Alhambra II, as Logix Engine lists it (captured from the
 * board): for the tests of connected devices, `__fake.useAlhambra()`. */
const alhambraDevice: Real.UsbDevice = {
  id: "usb:1-1:0403:6010:FTATANKC",
  label: "Alhambra II v1.0A (AlhambraBits, 0403:6010)",
  vid: "0403",
  pid: "6010",
  manufacturer: "AlhambraBits",
  product: "Alhambra II v1.0A",
  serialNumber: "FTATANKC",
  // Not captured from the board.
  release: "",
  boards: [
    { board: "alhambra-ii", level: "strong" },
    { board: "alchitry-au", level: "weak" },
  ],
  likelyBoards: ["alhambra-ii"],
  otherBoards: [],
  rank: 0,
  buildMatch: "strong", driver: null, who: "USB device (0403:6010)",
  // The FTDI chip's two channels: programming, then the UART.
  serialPorts: [
    { port: "/dev/cu.usbserial-FTATANKC0", interface: 0, role: "programmer", description: "USB device (0403:6010)'s programming port" },
    { port: "/dev/cu.usbserial-FTATANKC1", interface: 1, role: "uart", description: "USB device (0403:6010)'s serial port" },
  ],
  serialPort: "/dev/cu.usbserial-FTATANKC1",
};

const unlisten = async () => () => {};
const pause = (ms: number) => new Promise((r) => setTimeout(r, ms));

function buildOutcome(files: ProjectFile[], board: string): {
  lines: Line[];
  outcome: TaskOutcome;
} {
  const main = files.find((f) => f.name === "main.v")?.text ?? "";
  // The iCESugar blinky with the ";" missing after "reg [N-1:0] counter =
  // 0": what the engine really reports (lint's message, at the next line's
  // "always").
  if (/reg \[N-1:0\] counter = 0\s*$/m.test(main)) {
    const lines = main.split("\n");
    const at = lines.findIndex((l) => /^\s*always\b/.test(l));
    return {
      lines: outputLines(verifyErrorOutput),
      outcome: {
        ok: false,
        diagnostics: [
          {
            severity: "error",
            file: "main.v",
            line: at + 1,
            column: lines[at].indexOf("always") + 1,
            message: "syntax error, unexpected always, expecting ',' or ';'",
          },
        ],
        testbenches: [],
        report: null,
        cancelled: false,
        seconds: 0.5,
        action: null,
        devices: [], driverDevice: null, sameIds: null, checkedFiles: [],
      },
    };
  }
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
       
        seconds: 0.5,
        action: null,
        devices: [], driverDevice: null, sameIds: null, checkedFiles: [],
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
        plain("\x1b[2m$ verilator_bin --lint-only --quiet main.v\x1b[0m"),
        plain("\x1b[2m$ yosys -q -p 'synth_ice40 -top main' -DSYNTHESIZE main.v\x1b[0m"),
        // Logix Engine gives the place the line names, for a link.
        {
          kind: "line",
          line: `\x1b[31mmain.v:${bad + 1}: ERROR: syntax error\x1b[0m`,
          tool: "yosys",
          location: { file: "main.v", line: bad + 1 },
        },
      ],
      outcome: {
        ok: false,
        diagnostics: [diag],
        testbenches: [],
        report: null,
        cancelled: false,
       
        seconds: 0.4,
        action: null,
        devices: [], driverDevice: null, sameIds: null, checkedFiles: [],
      },
    };
  }
  return {
    lines: outputLines(verifyOutput),
    outcome: {
      ok: true,
      diagnostics: [],
      testbenches: [],
      // Real Logix Engine output for the iCESugar v1.5's Blinky.
      report: verifyReport as Real.BuildReport,
      cancelled: false,
       
      seconds: 1.2,
      action: null,
      devices: [], driverDevice: null, sameIds: null, checkedFiles: [],
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
    // With `/?developerSettings`, developer settings are in effect: Logix
    // IDE's first, then the engine's once it runs, as the real setup sends
    // them.
    if (new URLSearchParams(location.search).has("developerSettings")) {
      onEvent({ kind: "developerSettings", names: ["LOGIX_DEV_ENGINE_BINARY"] });
      onEvent({
        kind: "developerSettings",
        names: ["LOGIX_DEV_ENGINE_BINARY", "LOGIX_ENGINE_CHANNEL"],
      });
    }
    // With `/?engineSetup`, setup stops in the engine's package download
    // until a test calls `__fake.finishSetup()`.
    if (new URLSearchParams(location.search).has("engineSetup")) {
      // As the real setup: the boards and examples before the FPGA tools.
      onEvent({ kind: "catalogReady" });
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
  fileFormats: async () => FILE_FORMATS,
  getFpgaFamilies: async () => FPGA_FAMILIES,

  getExamples: async () => ({
    examples: [...catalog.examples, ...updateExamples],
    hidden: catalog.hidden,
  }),

  openBareMinimum: async (board) => bareDoc(board),
  // The iCESugar v1.5's serial example as it is; every other example is blinky.
  openExample: async (board, name) =>
    board === DEFAULT_BOARD && name === "serial" ? serialDoc(board) : blinkyDoc(board),

  runTask: async (
    task,
    target,
    key,
    sourceDir,
    _projectDir,
    files,
    board,
    variant,
    device,
    onEvent,
    driver = { install: false, allowSameIds: false },
  ) => {
    fake.taskVariant = variant;
    fake.taskBoardsFolder = sourceDir;
    fake.lastRun = { key, task, target };
    // As the backend: Upload and Debug end the open debug session.
    if (task === "upload" || task === "debug") fake.debugSession = null;
    // As the IDE's backend: a file with problems stops the task, its
    // problems in the console at line and column.
    const own = fakeProjectBoards(boardsOnDisk(sourceDir), sourceDir, _projectDir != null);
    if (own.problem) {
      for (const line of boardsProblemLines(own.problem)) onEvent({ kind: "line", line });
      return {
        ok: false, diagnostics: own.problem.diagnostics, testbenches: [], report: null,
        cancelled: false, seconds: 0.1, action: null, devices: [], driverDevice: null, sameIds: null,
        checkedFiles: [],
      };
    }
    const onLine = (line: string) => onEvent(plain(line));
    const { lines: buildLines, outcome } = buildOutcome(files, board);
    outcome.checkedFiles = checkedFiles(files, task);
    let lines = buildLines;
    if (outcome.ok && (task === "test" || task === "simulate")) {
      lines = outputLines(testOutput);
      outcome.testbenches = [{ name: "main_tb.v", passed: true }];
      outcome.report = null;
    }
    if (outcome.ok && task === "diagram") {
      // The backend shows the engine's page in the Diagram window, which
      // the fake has none of: it records the module the window opens on.
      lines = outputLines(diagramOutput);
      fake.diagramShown = { key: key, module: target };
      outcome.report = null;
    }
    // Logix Engine reports each task's phases.
    const phase = {
      build: "Synthesizing",
      upload: "Synthesizing",
      debug: "Synthesizing",
      test: "Running main_tb.v",
      simulate: "Running main_tb.v",
      diagram: "Preparing the diagram",
      report: "Synthesizing",
    }[task];
    const phases = task === "build" || task === "upload" || task === "report" || task === "debug";
    // Debug programs the board as Upload does.
    const programs = task === "upload" || task === "debug";
    onEvent({ kind: "progress", message: phase, percent: null });
    for (const line of lines) {
      onEvent(line);
      await pause(5);
    }
    const fits = fake.uploadDevices ?? [icesugarDevice];
    if (phases && programs && outcome.ok && !device && fits.length > 1) {
      // The engine's list_devices for the build found several devices:
      // its status says so.
      const name = BOARDS.find((b) => b.id === board)?.description ?? board;
      const message = `${fits.length} connected devices could be the ${name}. Choose the one to program.`;
      return {
        ...outcome,
        ok: false,
        report: null,
        action: "select_device",
        devices: fits,
        diagnostics: [{ severity: "error", file: null, line: null, column: null, message }],
      };
    }
    // Windows: the board needs WinUSB first. Upload asks the user, then
    // runs again with the driver to install (Logix Engine's install_driver).
    let uploadId = device ?? fits[0].id;
    const needsDriver = fake.uploadDriver && !fake.uploadDriver.ready ? fake.uploadDriver : null;
    if (phases && programs && outcome.ok && needsDriver) {
      const picked = fits.find((d) => d.id === uploadId) ?? fits[0];
      const label = picked.label;
      const who = engineWho(label, picked.boards.map((b) => b.board), board);
      const driverDevice = { id: uploadId, label, who, driver: engineDriver(needsDriver, who) };
      if (!driver.install) {
        return { ...outcome, ok: false, report: null, action: "install_drivers", driverDevice };
      }
      if (fake.sameIdsMessage && !driver.allowSameIds) {
        return {
          ...outcome, ok: false, report: null, action: "confirm_same_ids", driverDevice,
          sameIds: sameIdsConsent(fake.sameIdsMessage),
        };
      }
      // As the backend says it (engine/task.rs): the engine's busy line,
      // then its summary.
      const udev = needsDriver.needed === "udev-rule";
      const statusText = driverDevice.driver.add?.busy ?? "";
      onEvent({ kind: "status", text: statusText });
      onLine(statusText);
      fake.driverStatuses.push(statusText);
      fake.driverInstalls.push({ device: uploadId, allowSameIds: driver.allowSameIds });
      fake.uploadDriver = udev
        ? { ...needsDriver, current: "70-logix-engine-0403-6010.rules", ready: true }
        : { ...needsDriver, current: "winusb", ready: true };
      // The engine reads the serial number once the driver lets it.
      uploadId = `${uploadId}FT1`;
      onLine(udev ? `Added a udev rule for ${who}.` : `Installed the WinUSB driver for ${who}.`);
      onEvent({ kind: "status", text: null });
    }
    if (phases && programs && outcome.ok) {
      fake.uploadedDevice = uploadId;
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
      onLine(
        `Programmed the ${memory} of the ${name}: the design ${
          memory === "RAM" ? "is lost" : "stays"
        } when the board is powered off.`,
      );
    }
    if (task === "debug" && outcome.ok) {
      // The backend opens the session and shows its panel (open_debug,
      // open_debug_panel) in the Debugger window.
      onLine("Opened the debug session on the board's UART.");
      fake.debugSession = "fake-session";
      outcome.debugSession = fake.debugSession;
    }
    return outcome;
  },

  openLastProject: async () => null,

  formatFile: async (name, text) => {
    handlers.output?.(
      plain(`\x1b[2m$ verible-verilog-format --nofailsafe_success --inplace ${name}\x1b[0m`),
    );
    return {
      ok: true,
      text: text.replace(/[ \t]+$/gm, ""),
      diagnostics: [],
      checkedFiles: [name],
    };
  },

  newProject: async (board) => bareDoc(board ?? DEFAULT_BOARD),
  lastBoard: async () => DEFAULT_BOARD,
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
  saveProject: async (_dir, files, removed, board, variant, serialBaud) => {
    if (fake.saveError) throw fake.saveError;
    fake.saves.push({ files: files.map((f) => f.name), removed });
    fake.saved = { board, variant };
    fake.savedBaud = serialBaud;
  },

  saveProjectAsDialog: async (_dir, _source, suggestedName, files, _removed, board, variant, serialBaud) => {
    if (fake.saveAsError) throw fake.saveAsError;
    fake.saved = { board, variant };
    fake.savedBaud = serialBaud;
    return {
      dir: `/fake/projects/${suggestedName}`,
      templateDir: null,
      name: suggestedName,
      files,
      board,
      variant,
      serialBaud,
    };
  },

  // The Serial Monitor: the connected devices' ports, the selected board's
  // UART first, as Logix Engine marks it; the board echoes what's sent.
  serialPorts: async (_folder, _saved, board) =>
    (fake.usb ?? [icesugarDevice]).flatMap((d) =>
      d.serialPorts.map((p): Real.SerialChoice => ({
        device: d.id,
        who: d.who,
        label: d.label,
        port: p.port,
        interface: p.interface,
        boardUart: !!board && d.boards.some((b) => b.board === board) && d.serialPort === p.port,
        role: p.role,
        description: p.description,
        other: false,
      })),
    )
      .concat(
        fake.otherPorts.map((p) => {
          const name = p.description.replace(/^A serial port of /, "");
          return { device: null, who: name, label: name, port: p.port, interface: p.interface,
                   boardUart: false, role: null, description: p.description, other: true };
        }),
      )
      .sort(
        (a, b) =>
          Number(b.boardUart) - Number(a.boardUart) ||
          Number(a.other) - Number(b.other) ||
          Number(a.role === "programmer") - Number(b.role === "programmer"),
      ),
  serialOpen: async (port, baud, device) => {
    fake.serialOpened.push({ port, baud, device });
    return `serial-${fake.serialOpened.length}`;
  },
  serialWrite: async (text) => {
    fake.serialSent.push(text);
    setTimeout(() => fake.serialReceive(text), 10);
  },
  serialClose: async () => {},
  onSerial: async (handler) => {
    handlers.serial = handler;
    return () => {};
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
  newerIdeNotice: async () => fake.newerIde,
  dismissNewerIde: async () => {
    fake.newerDismissed++;
    // As the real one: it stays away for a week.
    fake.newerIde = null;
  },
  openNewerIdePage: async () => {
    fake.newerPageOpened++;
  },
  checkForUpdates: async (onProgress) => {
    // As the real one: the engine's progress, a console line per package
    // and its percentage for the status bar, then what changed. With
    // `__fake.holdUpdates`, it waits at 42% until `__fake.finishUpdates()`.
    const message = "Downloading logix-catalog 2026.11.30";
    handlers.output?.(plain(`${message}...`));
    for (const percent of [0, 42]) {
      onProgress({ kind: "progress", message, percent });
      await pause(5);
    }
    if (fake.holdUpdates) await new Promise<void>((resolve) => (handlers.finishUpdates = resolve));
    onProgress({ kind: "progress", message, percent: 100 });
    catalogRelease = "2026.11.30";
    // As the real one: with new examples (`__fake.updateBringsExample`),
    // the backend tells the UI to fetch the lists again.
    if (fake.updateBringsExample) {
      updateExamples.push({ board: "alhambra-ii", name: "fresh-from-update", summary: "An example the update brought", notes: [] });
      handlers.listsChanged?.();
    }
    return ["Updated logix-catalog to 2026.11.30."];
  },
  systemInfo: async () => {
    // As the real one (system_info.rs): Logix IDE's entry with the
    // engine's own tree and the examples under it.
    const leaf = (name: string, version: string, kind: VersionEntry["kind"], notes: string[] = [], details: InfoDetail[] = []): VersionEntry =>
      ({ name, version, kind, notes, details, children: [] });
    const versions: VersionEntry = {
      ...leaf("Logix IDE", packageJson.version, "program", ["channel prod, commit 973dd78, remote config 001"],
        [{ label: "Repo", value: "openlogix/logix-ide" }]),
      channel: "prod",
      children: [
        {
          ...leaf("Logix Engine", "2026.11.14", "program", ["channel prod, commit a1b2c3d, remote config 001"],
            [{ label: "Pinned in", value: "Logix IDE's remote config 001, prod-channel.jsonc" }]),
          channel: "prod",
          children: [
            {
              ...leaf("logix-core", "2026.11.14", "package"),
              children: [
                leaf("OSS CAD Suite", "2026-11-04", "third_party", ["from YosysHQ/oss-cad-suite-build"],
                  [{ label: "Uses", value: "yosys, nextpnr-ice40, openFPGALoader" }]),
              ],
            },
            leaf("logix-catalog", catalogRelease, "package"),
          ],
        },
        leaf("logix-ide-examples", "2026.11.15", "package"),
      ],
    };
    return {
      facts: [
        { label: "Operating system", value: "macOS 26.5 (darwin-arm64)" },
        { label: "Log file", value: "/Users/you/.logix/ide/logs/20261115-171205-4412/ide.log" },
      ],
      developerSettings: [],
      versions,
      text: [
        `Logix IDE ${packageJson.version}`,
        "Operating system: macOS 26.5 (darwin-arm64)",
        "Log file: /Users/you/.logix/ide/logs/20261115-171205-4412/ide.log",
        "",
        "Versions:",
        `Logix IDE ${packageJson.version}`,
        "│    channel prod, commit 973dd78, remote config 001",
        "├─ Logix Engine 2026.11.14",
        "│  │    channel prod, commit a1b2c3d, remote config 001",
        "│  ├─ logix-core 2026.11.14",
        "│  │  └─ OSS CAD Suite 2026-11-04",
        "│  │          from YosysHQ/oss-cad-suite-build",
        "│  └─ logix-catalog 2026.11.14",
        "└─ logix-ide-examples 2026.11.15",
      ].join("\n"),
    };
  },
  copyText: async (text) => {
    fake.copiedText = text;
  },
  copyProjectPath: async (dir) => {
    fake.copiedPath = dir;
    return `'${dir}'`; // as the real one quotes a path with a space
  },
  cancelTask: async () => false,

  // debug.toml's text, in memory like the other project files: the fake
  // writes and reads only its clock and single-net entries.
  debugConfig: async (text) => {
    if (text == null) return { exists: false, clock: "", watch: [], drive: [], hasNets: false };
    const clock = /^clock = "(.*)"$/m.exec(text)?.[1] ?? "";
    const nets = (kind: string) => [...text.matchAll(new RegExp(`\\[\\[${kind}\\]\\]\\nnet = "(.*)"`, "g"))].map((m) => m[1]);
    const watch = nets("watch");
    const drive = nets("drive");
    return { exists: true, clock, watch, drive, hasNets: watch.length + drive.length > 0 };
  },
  updateDebugConfig: async (_text, clock, watch, drive) => {
    fake.debugUpdates.push({ clock, watch, drive });
    const entries = (kind: string, nets: string[]) => nets.map((n) => `\n[[${kind}]]\nnet = "${n}"\n`).join("");
    return `#:schema https://www.openlogix.org/schemas/logix-debug-v1.json\nclock = "${clock}"\n${entries("watch", watch)}${entries("drive", drive)}`;
  },
  listDebugNets: async () => fake.debugNets,
  stopDebug: async () => {
    const was = fake.debugSession != null;
    fake.debugSession = null;
    if (was) fake.debugStops++;
    return was;
  },
  onDebugEnded: async (handler) => {
    handlers.debugEnded = () => {
      fake.debugSession = null;
      handler();
    };
    return () => {};
  },

  // No language server in the fake: it never answers, so the UI quietly
  // runs without live errors in these tests and screenshots.
  lspStart: async () => {},
  lspSend: async () => {},
  lspStop: async () => {},
  onLspMessage: unlisten,
  onLspClosed: unlisten,
  onListsChanged: async (handler) => {
    handlers.listsChanged = handler;
    return () => {};
  },
  onWaveView: async (handler) => {
    handlers.waveView = handler;
    return () => {};
  },
  onWaveOpenSource: async (handler) => {
    handlers.waveOpenSource = handler;
    return () => {};
  },

  watchProject: async (dir) => {
    fake.watchedDir = dir;
  },
  onProjectFilesChanged: async (handler) => {
    handlers.projectFilesChanged = handler;
    return () => {};
  },
  readProjectChanges: async () => pendingDisk,

  onDiagramOpenSource: async (handler) => {
    handlers.diagramOpenSource = handler;
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

  scanDevices: async (_folder, _saved, board) =>
    engineList(
      (fake.usb ?? [icesugarDevice]).map((d) => engineDevice(d as typeof d & { driver: RawDriver | null }, board)),
      fake.unreachable.map((u) => ({
        ...engineDevice(u as typeof u & { driver: RawDriver | null }, board),
        fix: u.fix || engineFix(u.driver),
      })),
      board,
    ),
  installDeviceDriver: async (_folder, _saved, device, allowSameIds, board, onEvent) => {
    const listed = [...(fake.usb ?? []), ...fake.unreachable].find((d) => d.id === device);
    const who = listed ? engineWho(listed.label, listed.boards.map((b) => b.board), board) : device;
    if (fake.sameIdsMessage && !allowSameIds) return { installed: false, summary: [], sameIds: sameIdsConsent(fake.sameIdsMessage) };
    if (fake.driverError) throw fake.driverError;
    // Logix Engine's phases and output, as install_driver sends them.
    onEvent({ kind: "progress", message: "Waiting for permission", percent: null });
    onEvent({ kind: "line", line: "Installing WinUSB for the Alhambra II (USB interface 0)." });
    if (fake.driverHold) await fake.driverHold;
    onEvent({ kind: "progress", message: "Installing driver", percent: null });
    onEvent({ kind: "line", line: "\x1b[2m$ wdi-simple.exe --vid 0x0403 --pid 0x6010\x1b[0m" });
    onEvent({ kind: "line", line: "Success", tool: "wdi-simple" });
    fake.driverInstalls.push({ device, allowSameIds });
    fake.usb = (fake.usb ?? []).map((d) =>
      d.id === device && d.driver ? { ...d, driver: { ...d.driver, current: "winusb", ready: true } } : d,
    );
    fake.unreachable = fake.unreachable.filter((u) => u.id !== device);
    return { installed: true, summary: [`Installed the WinUSB driver for ${who}.`], sameIds: null };
  },
  uninstallDeviceDriver: async (_folder, _saved, device, board, onEvent) => {
    const listed = (fake.usb ?? []).find((d) => d.id === device);
    const who = listed ? engineWho(listed.label, listed.boards.map((b) => b.board), board) : device;
    onEvent({ kind: "progress", message: "Removing driver", percent: null });
    onEvent({ kind: "line", line: "\x1b[2m$ pnputil /delete-driver oem12.inf /uninstall /force\x1b[0m" });
    fake.driverRemovals.push(device);
    fake.usb = (fake.usb ?? []).map((d) =>
      d.id === device && d.driver ? { ...d, driver: { ...d.driver, current: "ftdibus", ready: false } } : d,
    );
    return [`Removed the WinUSB driver for ${who}.`, "Windows uses its original driver again."];
  },
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
  askSaveChanges: async (name: string) => {
    fake.saveChangesAsked.push(name);
    return fake.saveChangesAnswer;
  },
  setDocumentState: async (name: string, dirty: boolean) => {
    fake.documentState = { name, dirty };
  },
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
  showEditorMenu: async (module) => {
    fake.editorMenu = fake.menuState && { ...fake.menuState };
    fake.editorMenuModule = module;
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
