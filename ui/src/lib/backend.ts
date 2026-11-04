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
  | { kind: "log"; line: string };

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

export type SaveChoice = "save" | "discard" | "cancel";

export interface Board {
  /** Board id, e.g. "alhambra-ii". */
  id: string;
  /** e.g. "Alhambra II (Lattice, iCE40)". */
  description: string;
  /** FPGA architecture, e.g. "ice40", "ecp5", "gowin", "xilinx" ("" if unknown). */
  arch: string;
  /** The extension of the board's pin files, e.g. "pcf" (null if unknown). */
  pinExtension: string | null;
  /** The board's choices for programming it (none for most boards). */
  dimensions: Dimension[];
  /** The combinations that exist; the first is the default. */
  configurations: Configuration[];
}

export interface Example {
  board: string;
  name: string;
  description: string;
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
  /** Logix Engine's suggested action when it failed, e.g. "connect_board". */
  action: string | null;
}

/** Streamed while a task runs: console lines, and the current phase. */
export type TaskEvent =
  | { kind: "line"; line: string }
  | { kind: "progress"; message: string; percent: number | null }
  // Upload's build is done; programming the board starts.
  | { kind: "programming" };

/** The result of Auto Format. */
export interface FormatResult {
  ok: boolean;
  /** The formatted text, if formatting succeeded. */
  text: string | null;
  diagnostics: Diagnostic[];
}

export interface UsbDevice {
  /** USB vendor and product ids, as 4 hex digits. */
  vid: string;
  pid: string;
  manufacturer: string;
  product: string;
  serialNumber: string;
}

/** Connected devices, from Logix Engine. */
export interface Devices {
  usb: UsbDevice[];
}

/** A board that matches a connected USB device. */
export interface ConnectedBoard {
  board: string;
  /** Its product name matched too, not just the USB chip's ids. */
  exact: boolean;
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
  /** The open tab is Verilog: Auto Format. */
  verilog: boolean;
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
  | "stop"
  | "show-diagram"
  | "devices"
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
   * receives GTKWave layouts saved since the last run. The tools' output
   * and the engine's phases go to `onEvent`. Resolves with the outcome
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
      onEvent: channel,
    });
  },

  /** Reopens the project last opened or saved. Null if there isn't one. */
  openLastProject: (): Promise<ProjectDoc | null> =>
    invoke<ProjectDoc | null>("open_last_project"),

  /**
   * Formats a Verilog/SystemVerilog file's text with Logix Engine's
   * `format` (Verible), without touching the project folder. Verible's
   * output comes through `onApioOutput`.
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
  recentProjectsForBoard: (board: string): Promise<string[]> =>
    invoke<string[]>("recent_projects_for_board", { board }),

  /** The variant of the last chosen board ({} for its default). */
  lastVariant: (): Promise<Variant> => invoke<Variant>("last_variant"),

  /** Remembers the board and variant the user chose, for new projects. */
  setLastBoard: (board: string, variant: Variant): Promise<void> =>
    invoke<void>("set_last_board", { board, variant }),

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
    sourceDir: string | null,
    suggestedName: string,
    files: ProjectFile[],
    removed: string[],
    board: string | null,
    variant: Variant,
  ): Promise<ProjectDoc | null> =>
    invoke<ProjectDoc | null>("save_project_as_dialog", {
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

  /** The language server stopped. */
  onLspClosed: (handler: () => void): Promise<UnlistenFn> =>
    listen("lsp-closed", () => handler()),

  /** The connected USB devices. */
  scanDevices: (): Promise<Devices> => invoke<Devices>("scan_devices"),

  /** The boards that match the connected USB devices (silent). */
  connectedBoards: (): Promise<ConnectedBoard[]> =>
    invoke<ConnectedBoard[]>("connected_boards"),

  /** Stops the running build, upload, test or simulation. */
  cancelTask: (): Promise<boolean> => invoke<boolean>("cancel_task"),

  /** File > Open Recent: the folder of the project to open. */
  onOpenRecent: (handler: (dir: string) => void): Promise<UnlistenFn> =>
    listen<string>("open-recent", (e) => handler(e.payload)),

  /** Checks a new file name. Returns a message if it can't be used. */
  checkFileName: (name: string, existing: string[]): Promise<string | null> =>
    invoke<string | null>("check_file_name", { name, existing }),

  /** Asks to confirm something destructive. True if confirmed. */
  askConfirm: (
    title: string,
    message: string,
    okLabel: string,
  ): Promise<boolean> =>
    invoke<boolean>("ask_confirm", { title, message, okLabel }),

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
  onApioOutput: (handler: (line: string) => void): Promise<UnlistenFn> =>
    listen<string>("apio-output", (e) => handler(e.payload)),

  /** The user tried to close the window while there are unsaved changes. */
  onCloseRequested: (handler: () => void): Promise<UnlistenFn> =>
    listen("close-requested", () => handler()),
};
