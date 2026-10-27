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
  | "downloadingApio"
  | "installingPackages"
  | "ready";

/** Progress events sent while setting up Apio (see `SetupEvent` in Rust). */
export type SetupEvent =
  | { kind: "stage"; stage: SetupStage }
  | { kind: "download"; received: number; total: number | null }
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
  /** A new project with starter content (from File > New). */
  starter: boolean;
}

export type SaveChoice = "save" | "discard" | "cancel";

export interface Board {
  /** Apio board id, e.g. "alhambra-ii". */
  id: string;
  /** e.g. "Alhambra II (Lattice, iCE40)". */
  description: string;
  /** FPGA architecture, e.g. "ice40", "ecp5", "gowin", "xilinx" ("" if unknown). */
  arch: string;
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

/** Resource usage and clocks after a successful build (see apio/report.rs). */
export interface BuildReport {
  /** FPGA family, e.g. "ice40", "ecp5", "gowin", "xilinx". */
  arch: string;
  /** With Apio's family-specific names, e.g. "ICESTORM_LC". */
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
}

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
  /** E.g. "FT2232H". */
  deviceType: string;
}

export interface SerialDevice {
  /** E.g. "/dev/cu.usbserial-FTATANKC0". */
  port: string;
  vid: string;
  pid: string;
  manufacturer: string;
  product: string;
}

/** Connected devices, from `apio api scan-devices`. */
export interface Devices {
  usb: UsbDevice[];
  serial: SerialDevice[];
}

/** A board that matches a connected USB device. */
export interface ConnectedBoard {
  board: string;
  /** Its product name matched too, not just the USB chip's ids. */
  exact: boolean;
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
}

export interface ExampleList {
  examples: Example[];
  /** Examples left out because Logix doesn't support them yet. */
  hidden: number;
}

/** Ids of native menu items the UI handles (see `menu.rs`). */
export type MenuId =
  | "new"
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
  | "clear-output";

export const backend = {
  appInfo: (): Promise<AppInfo> => invoke<AppInfo>("app_info"),

  /**
   * Makes sure Apio and its packages are installed. Resolves when ready,
   * rejects with a `UserError` on failure. Safe to call again to retry.
   */
  setupApio: (onEvent: (event: SetupEvent) => void): Promise<void> => {
    const channel = new Channel<SetupEvent>();
    channel.onmessage = onEvent;
    return invoke<void>("setup_apio", { onEvent: channel });
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
   * A board's BareMinimum (Examples): its pins and a design that does
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
   * receives GTKWave layouts saved since the last run. Apio's output lines go to
   * `onLine`. Resolves with the outcome (including failures); rejects only
   * if Apio couldn't run.
   */
  runTask: (
    task: Task,
    testbench: string | null,
    key: string,
    sourceDir: string | null,
    projectDir: string | null,
    files: ProjectFile[],
    board: string,
    onLine: (line: string) => void,
  ): Promise<TaskOutcome> => {
    const channel = new Channel<string>();
    channel.onmessage = onLine;
    return invoke<TaskOutcome>("run_task", {
      task,
      testbench,
      key,
      sourceDir,
      projectDir,
      files,
      board,
      onLine: channel,
    });
  },

  /** Reopens the project last opened or saved. Null if there isn't one. */
  openLastProject: (): Promise<ProjectDoc | null> =>
    invoke<ProjectDoc | null>("open_last_project"),

  /**
   * Formats a Verilog/SystemVerilog file's text with `apio format`, without
   * touching the project folder. `board` may be null (any board works).
   * Apio's output comes through `onApioOutput`.
   */
  formatFile: (
    name: string,
    text: string,
    board: string | null,
  ): Promise<FormatResult> =>
    invoke<FormatResult>("format_file", { name, text, board }),

  /**
   * A new, unsaved project for `board` (or else the last chosen board),
   * started from the board's basic example when possible, so it builds
   * right away.
   */
  newProject: (board: string | null): Promise<ProjectDoc> =>
    invoke<ProjectDoc>("new_project", { board }),

  /** Remembers the board the user chose, for new projects. */
  setLastBoard: (board: string): Promise<void> =>
    invoke<void>("set_last_board", { board }),

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
  ): Promise<void> =>
    invoke<void>("save_project", { dir, files, removed, board }),

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
  ): Promise<ProjectDoc | null> =>
    invoke<ProjectDoc | null>("save_project_as_dialog", {
      sourceDir,
      suggestedName,
      files,
      removed,
      board,
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

  /** The connected USB devices and serial ports. */
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

  showEditorMenu: (canGoToDefinition: boolean): Promise<void> =>
    invoke<void>("show_editor_menu", { canGoToDefinition }),

  /** Native menu items (New, Open, Save, Save As). */
  onMenu: (handler: (id: MenuId) => void): Promise<UnlistenFn> =>
    listen<MenuId>("menu", (e) => handler(e.payload)),

  /**
   * Apio output that isn't part of a build, upload, test or setup (which
   * report their own lines): board and example lists, opening an example,
   * Auto Format. Every Apio run is shown in the console.
   */
  onApioOutput: (handler: (line: string) => void): Promise<UnlistenFn> =>
    listen<string>("apio-output", (e) => handler(e.payload)),

  /** The user tried to close the window while there are unsaved changes. */
  onCloseRequested: (handler: () => void): Promise<UnlistenFn> =>
    listen("close-requested", () => handler()),
};
