// The only place the UI talks to the Rust backend. Every Tauri command the
// UI uses gets a typed wrapper here; components never call `invoke` directly.
import { Channel, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface AppInfo {
  name: string;
  version: string;
  /** Platform id, e.g. "macos-arm64", or "unknown". */
  platform: string;
  /** False when the IDE doesn't support this platform yet. */
  platformSupported: boolean;
}

/** Error returned by backend commands (see `UserError` in Rust). */
export interface UserError {
  /** Short plain-language message for the user. */
  message: string;
  /** Full technical details, for the console. */
  details: string;
}

/** Normalizes anything a backend call rejects with into a `UserError`. */
export function toUserError(e: unknown): UserError {
  if (
    typeof e === "object" &&
    e !== null &&
    typeof (e as UserError).message === "string"
  ) {
    const { message, details } = e as Partial<UserError>;
    return { message: message!, details: details ?? message! };
  }
  const text = String(e);
  return { message: text, details: text };
}

/**
 * The error's message with its reason, for a banner the user reads without
 * opening the output: "Couldn't save the project. There's a file there…".
 * The details hold the message, then each cause after ": ".
 */
export function withReason(err: UserError): string {
  const rest = err.details.startsWith(err.message)
    ? err.details.slice(err.message.length).replace(/^:\s*/, "").trim()
    : "";
  if (!rest) return err.message;
  const reason = rest.charAt(0).toUpperCase() + rest.slice(1);
  return `${err.message} ${/[.!?]$/.test(reason) ? reason : `${reason}.`}`;
}

export type SetupStage =
  | "checking"
  | "downloadingEngine"
  | "installingEnginePackages"
  | "ready";

/**
 * Progress events sent while setting up Logix Engine (see
 * `SetupEvent` in Rust). `progress` is the engine's package install: the
 * current phase (e.g. "Installing oss-cad-suite") and its percentage.
 */
export type SetupEvent =
  | { kind: "stage"; stage: SetupStage }
  | { kind: "download"; received: number; total: number | null }
  | { kind: "progress"; message: string; percent: number | null }
  | { kind: "log"; line: string }
  /** A local engine (LOGIX_DEV_ENGINE_BINARY) is used, for diagnosis. */
  | { kind: "devEngine"; path: string };

export interface ProjectFile {
  name: string;
  text: string;
}

/** A project as the UI sees it. `dir` is null for a new, unsaved project. */
export interface ProjectDoc {
  dir: string | null;
  /** For an unsaved project opened from an example: the example's folder. */
  templateDir: string | null;
  /** Display name (the folder name, or "untitled"). */
  name: string;
  /** The project's source files, main.v first. */
  files: ProjectFile[];
  board: string | null;
  /** The board's variant (`[variant]` in logix-ide.toml); {} for its default. */
  variant: Variant;
}

/**
 * A board's variant: values of its dimensions by dimension id, e.g.
 * `{ memory: "ram" }`. The engine uses the first configuration with these
 * values; {} means the board's first (default) configuration.
 */
export type Variant = Record<string, string>;

/** One kind of choice a board offers for programming it, e.g. "Memory". */
export interface Dimension {
  id: string;
  label: string;
  values: { id: string; label: string; help?: string }[];
}

/** One combination of dimension values that exists for a board. */
export interface Configuration {
  /** The value of every dimension. */
  when: Variant;
  /** Whether programming this way was tried on hardware. */
  tested: boolean;
  /** Warnings for users, e.g. to hold a button while plugging the board in. */
  userNotes?: string;
}

/**
 * The project's own boards as Logix Engine reads them from its
 * custom-boards.json (the file on disk), or the problems it found there:
 * its message, and a diagnostic for each problem at line and column, the
 * file named as the user knows it (its full path in a saved project,
 * `custom-boards.json` in one never saved).
 */
export interface ProjectBoards {
  boards: Board[];
  problem: { message: string; diagnostics: Diagnostic[] } | null;
}

/**
 * Two boards compared by Logix Engine's `compare_boards`: facts only (the
 * IDE words them, `boardChangeNote`).
 */
export interface BoardComparison {
  /** Whether a build for one board is valid for the other. */
  sameBuild: boolean;
  /**
   * Every aspect, in the engine's order: board, architecture, fpga, size,
   * constraint_format, pins (later engines may add more).
   */
  aspects: {
    aspect: string;
    same: "yes" | "no" | "unknown";
    /** The two values, when known: any JSON type, by aspect (text today). */
    from?: unknown;
    to?: unknown;
    /** For people, e.g. "FPGA: ICE40HX4K-TQ144 → ICE40UP5K-SG48.". */
    text: string;
  }[];
}

export type SaveChoice = "save" | "discard" | "cancel";

export interface Board {
  /** Board id, e.g. "alhambra-ii". */
  id: string;
  /** e.g. "Alhambra II (Lattice, iCE40)". */
  description: string;
  /** FPGA architecture, e.g. "ice40", "ecp5", "gowin", "xilinx" ("" if unknown). */
  arch: string;
  /** The architecture's name for people, from Logix Engine, e.g. "Lattice iCE40". */
  archLabel: string;
  /** The architecture's place in Logix Engine's order, for the board list. */
  archOrder: number;
  /** The extension of the board's pin files, from Logix Engine, e.g. "pcf" (null if unknown). */
  pinExtension: string | null;
  /** How the board is programmed: Connection and Memory, for every board. */
  dimensions: Dimension[];
  /** The combinations that exist; the first is the default. */
  configurations: Configuration[];
  /** One of the project's own boards (custom-boards.json), not a catalog board. */
  custom: boolean;
  /** The board's FPGA, from Logix Engine's catalog (null if it doesn't say). */
  fpga: Fpga | null;
}

/** A board's FPGA. */
export interface Fpga {
  /** The catalog's FPGA ID, e.g. "ice40hx4k-tq144-8k": boards with the same ID have the same FPGA. Null for a custom board's own FPGA definition. */
  id: string | null;
  /** The vendor's part number, e.g. "ICE40HX4K-TQ144". */
  partNumber: string;
  /** The device size, e.g. "8k". */
  size: string;
}

export interface Example {
  board: string;
  name: string;
  /** What it does, in one line. */
  summary: string;
  /** More about it, a line each (empty when none), e.g. "Not yet tried on the board." */
  notes: string[];
}

export type Task = "build" | "upload" | "test" | "simulate" | "graph";

export interface Diagnostic {
  severity: "error" | "warning";
  /** Project file name, if the message points into one. */
  file: string | null;
  /** 1-based line and column, if known. */
  line: number | null;
  column: number | null;
  message: string;
}

export interface TestbenchResult {
  name: string;
  passed: boolean;
}

/** Resource usage and clocks after a successful build. */
export interface BuildReport {
  /** FPGA family, e.g. "ice40", "ecp5", "gowin", "xilinx". */
  arch: string;
  /** With the toolchain's family-specific names, e.g. "ICESTORM_LC". */
  resources: { name: string; used: number; available: number }[];
  /**
   * Logix Engine's plain-language summary, main capacity measure first:
   * `name` is its label, e.g. "logic cells". Empty from an older engine.
   */
  summary: { name: string; used: number; available: number }[];
  /** Each clock with its plain name from the engine, e.g. "sys_clk". */
  clocks: { name: string; fmaxMhz: number }[];
}

export interface TaskOutcome {
  ok: boolean;
  diagnostics: Diagnostic[];
  /** For "test": each testbench that ran, in order. */
  testbenches: TestbenchResult[];
  /** For a successful "build" or "upload". */
  report: BuildReport | null;
  /** Stopped by the user. */
  cancelled: boolean;
  /** For a successful "graph": the diagram as SVG. */
  graphSvg: string | null;
  seconds: number;
  /**
   * Logix Engine's suggested action when it failed, e.g. "connect_board",
   * or "select_device" when Upload found several devices that fit the
   * board (in `devices`).
   */
  action: string | null;
  /** For Upload's "select_device": the devices that fit, best first. */
  devices: UsbDevice[];
}

/** Streamed while a task runs: console lines, and the current phase. */
export type TaskEvent =
  | {
      kind: "line";
      line: string;
      /** The program that printed it; absent for the IDE's and engine's own lines. */
      tool?: string;
      /** The tool prints over it: replace it with that tool's next line. */
      redraw?: boolean;
    }
  | { kind: "progress"; message: string; percent: number | null };

/** The result of Auto Format. */
export interface FormatResult {
  ok: boolean;
  /** The formatted text, if formatting succeeded. */
  text: string | null;
  diagnostics: Diagnostic[];
}

export interface UsbDevice {
  /** Logix Engine's device id, for Upload. Opaque and short-lived. */
  id: string;
  /** The engine's name for people, e.g. "Alhambra II v1.0A (AlhambraBits, 0403:6010)". */
  label: string;
  /** USB vendor and product ids, as 4 hex digits. */
  vid: string;
  pid: string;
  manufacturer: string;
  product: string;
  serialNumber: string;
  /** The boards it may be, strongest first, each with its match level. */
  boards: { board: string; level: MatchLevel }[];
  /** The boards it most likely is, by the engine (none when only weak matches). */
  likelyBoards: string[];
  /** For Upload's candidates: how well it matches the build's board. */
  buildMatch: MatchLevel | null;
}

/**
 * Logix Engine's match level: what a match means for the user, never its
 * cause (the engine's criteria are internal and change without notice).
 * "strong": very likely this board; "possible": a few other boards look the
 * same; "weak": many boards fit. Only show and compare levels.
 */
export type MatchLevel = "strong" | "possible" | "weak";

/** A device that may be a board but can't be opened yet. */
export interface UnreachableDevice {
  /** The engine's name for people, e.g. "USB device (0403:6014)". */
  label: string;
  vid: string;
  pid: string;
  /** The suggested fix, e.g. "fix_device_permissions". */
  action: string;
  actionDescription: string;
}

/** Connected devices, from Logix Engine. */
export interface Devices {
  usb: UsbDevice[];
  /** Devices that need a driver or permissions before they can be used. */
  unreachable: UnreachableDevice[];
}

/** A connected USB device that fits catalog boards. */
export interface ConnectedDevice {
  /** The engine's device id: unique among the connected devices. */
  id: string;
  /** Its name for people, e.g. "Digilent USB Device". */
  name: string;
  /** Its serial number, "" when it has none. */
  serial: string;
  /** The boards it may be (one device is one board), name matches first. */
  boards: ConnectedBoard[];
}

/** A board that a connected USB device may be. */
export interface ConnectedBoard {
  board: string;
  /** The engine's level for this board; weak ones aren't listed by name. */
  level: MatchLevel;
  /** The configurations whose USB filter matched ({} for most boards). */
  variants: Variant[];
}

export type Theme = "system" | "light" | "dark";

/** User preferences (File > Settings), defaults filled in. */
export interface Preferences {
  /** Where new projects go (Arduino's "sketchbook location"). */
  projectsDir: string;
  editorFontSize: number;
  /** Interface scale in percent; null for automatic. */
  uiScale: number | null;
  theme: Theme;
  /** Whether Verify runs lint for warnings. */
  lintOnVerify: boolean;
  /** Whether the editor checks the code while typing (Verible). */
  liveChecking: boolean;
  /** Whether hovering over a name shows info (when liveChecking). */
  hoverInfo: boolean;
  /** When the hidden output comes back by itself. */
  showOutput: ShowOutput;
}

/**
 * When the hidden output comes back by itself: when a command starts (and
 * on errors), only when one fails, or never (only by hand).
 */
export type ShowOutput = "onCommand" | "onError" | "never";

export interface ExampleList {
  examples: Example[];
  /** Examples left out because Logix doesn't support them yet. */
  hidden: number;
}

/** What the menu items can act on now (`MenuState` in `menu.rs`). */
export interface MenuState {
  /** A project is open (not the start screen). */
  project: boolean;
  /** The open tab can be formatted (Auto Format): Verilog. */
  formattable: boolean;
  /** The open tab has line comments (Verilog or a pin file): Comment/Uncomment. */
  comment: boolean;
  /** Go to Definition can answer (a Verilog tab, live checking running). */
  definition: boolean;
  /** The project has testbenches (Run Tests). */
  testbenches: boolean;
  /** There's a testbench to simulate (Simulate). */
  simulate: boolean;
  /** The open tab is a testbench (the editor menu's Run This Test, Simulate). */
  testbenchOpen: boolean;
  /** The toolbar has a board Logix knows (Tools > Board Info). */
  board: boolean;
  /** One Tools submenu per dimension of the board's variants. */
  variantMenus: VariantMenu[];
}

/** A Tools submenu for one dimension; its items send "variant:<dim>:<value>". */
export interface VariantMenu {
  id: string;
  label: string;
  values: { id: string; label: string; checked: boolean; enabled: boolean }[];
}

/** Ids of native menu items the UI handles (see `menu.rs`). */
export type MenuId =
  | "new"
  | "close-project"
  | "open"
  | "save"
  | "save-as"
  | "new-file"
  | "examples"
  | "verify"
  | "upload"
  | "run-tests"
  | "simulate"
  | "auto-format"
  | "toggle-output"
  | "show-folder"
  | "copy-path"
  | "stop"
  | "show-diagram"
  | "devices"
  | "board-info"
  | "settings"
  | "go-to-definition"
  | "toggle-comment"
  | "select-all"
  | "output-select-all"
  | "clear-output"
  | "run-this-test"
  | "simulate-this-test"
  // Not a menu item: the recent projects list changed (Clear Menu).
  | "recents-changed"
  // Tools > <dimension> > <value>: the board's variant.
  | `variant:${string}`;

export const backend = {
  appInfo: (): Promise<AppInfo> => invoke<AppInfo>("app_info"),

  /**
   * Makes sure Logix Engine and its packages (the FPGA tools) are
   * installed and starts it. Resolves when ready, rejects with a
   * `UserError` on failure. Safe to call again to retry.
   */
  setup: (onEvent: (event: SetupEvent) => void): Promise<void> => {
    const channel = new Channel<SetupEvent>();
    channel.onmessage = onEvent;
    return invoke<void>("setup", { onEvent: channel });
  },

  /** All boards, sorted by description. Needs the FPGA tools ready. */
  getBoards: (): Promise<Board[]> => invoke<Board[]>("get_boards"),

  /**
   * The examples Logix supports, sorted by board and name, and the number
   * hidden. Needs the FPGA tools ready.
   */
  getExamples: (): Promise<ExampleList> =>
    invoke<ExampleList>("get_examples"),

  /**
   * A board's Bare Minimum (Examples): its pins and a design that does
   * nothing yet, as a new, unsaved project.
   */
  openBareMinimum: (board: string): Promise<ProjectDoc> =>
    invoke<ProjectDoc>("open_bare_minimum", { board }),

  /** Opens an example as a new, unsaved project set to its board. */
  openExample: (board: string, name: string): Promise<ProjectDoc> =>
    invoke<ProjectDoc>("open_example", { board, name }),

  /**
   * Builds, uploads, tests, simulates or draws (graph) the given (possibly
   * unsaved) files. `testbench` picks one testbench for "test" (all if
   * null) and "simulate", or the module to draw for "graph" (main if null). `key` identifies the project; `sourceDir` is the folder
   * whose other files are copied along; `projectDir` (saved projects only)
   * receives GTKWave layouts saved since the last run. `device` is the
   * device Upload uses (a `UsbDevice` id the user chose), or null to let
   * Upload use the only one that fits. The tools' output and the engine's
   * phases go to `onEvent`. Resolves with the outcome
   * (including failures); rejects only if the engine couldn't run.
   */
  runTask: (
    task: Task,
    testbench: string | null,
    key: string,
    sourceDir: string | null,
    projectDir: string | null,
    files: ProjectFile[],
    board: string,
    variant: Variant,
    device: string | null,
    onEvent: (event: TaskEvent) => void,
  ): Promise<TaskOutcome> => {
    const channel = new Channel<TaskEvent>();
    channel.onmessage = onEvent;
    return invoke<TaskOutcome>("run_task", {
      task,
      testbench,
      key,
      sourceDir,
      projectDir,
      files,
      board,
      variant,
      device,
      onEvent: channel,
    });
  },

  /** Reopens the project last opened or saved. Null if there isn't one. */
  openLastProject: (): Promise<ProjectDoc | null> =>
    invoke<ProjectDoc | null>("open_last_project"),

  /**
   * Formats a Verilog/SystemVerilog file's text with Logix Engine's
   * `format` (Verible), without touching the project folder. Verible's
   * output comes through `onToolOutput`.
   */
  formatFile: (
    name: string,
    text: string,
  ): Promise<FormatResult> =>
    invoke<FormatResult>("format_file", { name, text }),

  /**
   * File > New Project > Empty Project: an unsaved project for `board`
   * (or else the last chosen board) with the board's pins and an empty
   * design (its Bare Minimum), so it builds right away.
   */
  newProject: (board: string | null): Promise<ProjectDoc> =>
    invoke<ProjectDoc>("new_project", { board }),

  /** The board for new projects: the last one chosen, or the default. */
  lastBoard: (): Promise<string> => invoke<string>("last_board"),

  /** The recent project folders, newest first (the start screen). */
  recentProjects: (): Promise<string[]> => invoke<string[]>("recent_projects"),

  /** The recent project folders made for `board`, newest first. */

  /** The variant of the last chosen board ({} for its default). */
  lastVariant: (): Promise<Variant> => invoke<Variant>("last_variant"),

  /**
   * Remembers the board and variant the user chose, for new projects and
   * "Recently used". Resolves with the recent boards, newest first.
   */
  setLastBoard: (board: string, variant: Variant): Promise<string[]> =>
    invoke<string[]>("set_last_board", { board, variant }),

  /** The boards chosen most recently, newest first ("Recently used"). */
  recentBoards: (): Promise<string[]> => invoke<string[]>("recent_boards"),

  /** Forgets the last project, so the next start shows the start screen. */
  closeProject: (): Promise<void> => invoke<void>("close_project"),

  /** Asks for a project folder and loads it. Null if cancelled. */
  openProjectDialog: (): Promise<ProjectDoc | null> =>
    invoke<ProjectDoc | null>("open_project_dialog"),

  /**
   * Saves the project in its folder. `removed` lists files deleted or
   * renamed since the last save.
   */
  saveProject: (
    dir: string,
    files: ProjectFile[],
    removed: string[],
    board: string | null,
    variant: Variant,
  ): Promise<void> =>
    invoke<void>("save_project", { dir, files, removed, board, variant }),

  /**
   * Asks for a name and location, then creates the project there (copying
   * the other contents of `sourceDir` except `removed`). Null if cancelled.
   */
  saveProjectAsDialog: (
    /** The project's own folder (null for a project never saved). */
    projectDir: string | null,
    /** The folder whose other files are copied (the project's, or an example's). */
    sourceDir: string | null,
    suggestedName: string,
    files: ProjectFile[],
    removed: string[],
    board: string | null,
    variant: Variant,
  ): Promise<ProjectDoc | null> =>
    invoke<ProjectDoc | null>("save_project_as_dialog", {
      projectDir,
      sourceDir,
      suggestedName,
      files,
      removed,
      board,
      variant,
    }),

  /** Opens the project in `dir` (File > Open Recent). */
  openProjectAt: (dir: string): Promise<ProjectDoc> =>
    invoke<ProjectDoc>("open_project_at", { dir }),

  /** Opens the project folder in Finder. */
  showProjectFolder: (dir: string): Promise<void> =>
    invoke<void>("show_project_folder", { dir }),

  /**
   * Puts the project folder's path on the clipboard, quoted for the shell
   * when it needs it (to paste after `cd`). Returns the text copied.
   */
  copyProjectPath: (dir: string): Promise<string> =>
    invoke<string>("copy_project_path", { dir }),

  /**
   * Copies GTKWave layouts saved from a simulation back to the project
   * folder. Returns console lines about what was saved.
   */
  saveGtkwaveLayouts: (key: string, projectDir: string): Promise<string[]> =>
    invoke<string[]>("save_gtkwave_layouts", { key, projectDir }),

  /** The current preferences. */
  getPreferences: (): Promise<Preferences> =>
    invoke<Preferences>("get_preferences"),

  /**
   * Saves the preferences (the backend applies the interface scale and the
   * window theme) and returns them as stored.
   */
  setPreferences: (preferences: Preferences): Promise<Preferences> =>
    invoke<Preferences>("set_preferences", { preferences }),

  /** Asks for a folder. Null if cancelled. */
  chooseFolderDialog: (start: string | null): Promise<string | null> =>
    invoke<string | null>("choose_folder_dialog", { start }),

  /** Starts the Verilog language server (live errors), if not running. */
  lspStart: (): Promise<void> => invoke<void>("lsp_start"),

  /** Stops the language server (live checking turned off). */
  lspStop: (): Promise<void> => invoke<void>("lsp_stop"),

  /** Sends one LSP message (JSON text) to the language server. */
  lspSend: (message: string): Promise<void> =>
    invoke<void>("lsp_send", { message }),

  /** Messages (JSON text) from the language server. */
  onLspMessage: (handler: (message: string) => void): Promise<UnlistenFn> =>
    listen<string>("lsp-message", (e) => handler(e.payload)),

  /**
   * After Simulate: GTKWave's window appeared and was brought to the front
   * (true), or didn't appear within 30 s (false).
   */
  onGtkwaveOpened: (handler: (found: boolean) => void): Promise<UnlistenFn> =>
    listen<boolean>("gtkwave-opened", (e) => handler(e.payload)),

  /** The language server stopped. */
  onLspClosed: (handler: () => void): Promise<UnlistenFn> =>
    listen("lsp-closed", () => handler()),

  /**
   * The connected USB devices, matched to the catalog's boards and those of
   * the custom-boards.json in `folder` (the project's folder, `copySource`;
   * null for none). `saved`: whether the project is saved, which decides
   * how messages name the file (its full path, or `custom-boards.json`).
   */
  scanDevices: (folder: string | null, saved: boolean): Promise<Devices> =>
    invoke<Devices>("scan_devices", { folder, saved }),

  /**
   * The connected USB devices that fit boards (the catalog's and those of
   * the custom-boards.json in `folder`), with their boards (silent).
   */
  connectedDevices: (folder: string | null, saved: boolean): Promise<ConnectedDevice[]> =>
    invoke<ConnectedDevice[]>("connected_devices", { folder, saved }),

  /**
   * The project's own boards, as Logix Engine reads them from the
   * custom-boards.json in `folder` (the file on disk, edited outside
   * Logix; none without it), or the problems it found in it, the file
   * named as the user knows it: its full path when the project is `saved`,
   * `custom-boards.json` when it isn't.
   */
  compareBoards: (
    from: string,
    to: string,
    folder: string | null,
    saved: boolean,
  ): Promise<BoardComparison> =>
    invoke<BoardComparison>("compare_boards", { from, to, folder, saved }),

  projectBoards: (folder: string | null, saved: boolean): Promise<ProjectBoards> =>
    invoke<ProjectBoards>("project_boards", { folder, saved }),

  /**
   * A value that changes whenever the custom-boards.json in `folder` is
   * added, changed or removed on disk (null without it), polled to notice
   * edits made outside Logix.
   */
  customBoardsStamp: (folder: string | null): Promise<string | null> =>
    invoke<string | null>("custom_boards_stamp", { folder }),

  /** Stops the running build, upload, test or simulation. */
  cancelTask: (): Promise<boolean> => invoke<boolean>("cancel_task"),

  /** File > Open Recent: the folder of the project to open. */
  onOpenRecent: (handler: (dir: string) => void): Promise<UnlistenFn> =>
    listen<string>("open-recent", (e) => handler(e.payload)),

  /** Checks a new file name. Returns a message if it can't be used. */
  checkFileName: (name: string, existing: string[]): Promise<string | null> =>
    invoke<string | null>("check_file_name", { name, existing }),

  /**
   * Asks to confirm an action. True if confirmed. A warning (something
   * destructive) unless `kind` is "info" (an offer, e.g. to save first).
   */
  askConfirm: (
    title: string,
    message: string,
    okLabel: string,
    kind: "warning" | "info" = "warning",
  ): Promise<boolean> =>
    invoke<boolean>("ask_confirm", { title, message, okLabel, kind }),

  /** Asks what to do with unsaved changes. */
  askSaveChanges: (name: string): Promise<SaveChoice> =>
    invoke<SaveChoice>("ask_save_changes", { name }),

  /** Updates the window title and the unsaved-changes flag. */
  setDocumentState: (name: string, dirty: boolean): Promise<void> =>
    invoke<void>("set_document_state", { name, dirty }),

  /** Closes the window (after unsaved changes were handled). */
  closeWindow: (): Promise<void> => invoke<void>("close_window"),

  /** Appends lines the output pane showed to the Logix log file. */
  logOutput: (lines: string[]): Promise<void> =>
    invoke<void>("log_output", { lines }),

  /**
   * Shows the editor's right-click menu at the mouse. Its items come back
   * as `menu` events.
   */
  showOutputMenu: (): Promise<void> => invoke<void>("show_output_menu"),

  /** Shows the editor's right-click menu, its items as in `setMenuState`. */
  showEditorMenu: (): Promise<void> => invoke<void>("show_editor_menu"),

  /**
   * What the menu items can act on now; the others are greyed out, in the
   * menu bar and the editor's right-click menu.
   */
  setMenuState: (menu: MenuState): Promise<void> =>
    invoke<void>("set_menu_state", { menu }),

  /** Native menu items (New, Open, Save, Save As). */
  onMenu: (handler: (id: MenuId) => void): Promise<UnlistenFn> =>
    listen<MenuId>("menu", (e) => handler(e.payload)),

  /**
   * Tool output that isn't part of a task or setup (which report their
   * own lines): Auto Format's.
   */
  onToolOutput: (handler: (line: string) => void): Promise<UnlistenFn> =>
    listen<string>("tool-output", (e) => handler(e.payload)),

  /** The user tried to close the window while there are unsaved changes. */
  onCloseRequested: (handler: () => void): Promise<UnlistenFn> =>
    listen("close-requested", () => handler()),
};
