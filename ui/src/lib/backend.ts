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
/** A newer Logix IDE, for the banner under the toolbar (`newer.rs`). */
export interface NewerIde {
  /** The newer release's version, yyyy.mm.dd. */
  version: string;
  /** This build's version. */
  current: string;
  /** The release's page, where its installers and install steps are. */
  url: string;
}

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

/** One label and value: a System Info fact or a version entry's detail. */
export interface InfoDetail {
  label: string;
  value: string;
}

/**
 * One entry of System Info's version tree: a program, a package or a
 * third-party release, with its own dependencies under it.
 */
export interface VersionEntry {
  name: string;
  version: string;
  kind: "program" | "package" | "third_party";
  /** A program's release channel, such as prod. */
  channel?: string | null;
  /** The developer setting that replaces this entry. */
  overriddenBy?: string | null;
  /** Lines under the entry in the text (what Copy gives). */
  notes: string[];
  /** Why the entry is incomplete, shown on its row. */
  problem?: string | null;
  /** The rest, shown on hover. */
  details: InfoDetail[];
  children: VersionEntry[];
}

/** A developer setting in effect: an environment variable and its value. */
export interface DeveloperSetting {
  name: string;
  value: string;
}

/** Help > System Info's report. */
export interface SystemInfo {
  /** Logix IDE's facts: the operating system, the log file. */
  facts: InfoDetail[];
  developerSettings: DeveloperSetting[];
  versions: VersionEntry;
  /** The whole report as plain text, what Copy gives. */
  text: string;
}

export type SetupStage =
  | "checking"
  | "downloadingEngine"
  | "downloadingEnginePackages"
  | "ready";

/**
 * Progress events sent while setting up Logix Engine (see
 * `SetupEvent` in Rust). `progress` is the engine's package download: the
 * current phase (e.g. "Downloading oss-cad-suite") and its percentage.
 */
export type SetupEvent =
  | { kind: "stage"; stage: SetupStage }
  | { kind: "progress"; message: string; percent: number | null }
  | { kind: "log"; line: string }
  /**
   * Developer settings are in effect (such as LOGIX_DEV_ENGINE_BINARY or
   * LOGIX_IDE_CHANNEL): their names, Logix IDE's and then the engine's.
   */
  | { kind: "developerSettings"; names: string[] }
  /** The board and example lists are ready, before the FPGA tools are. */
  | { kind: "catalogReady" };

export interface ProjectFile {
  name: string;
  text: string;
}

/** A project's files and settings as read after a change on disk (`readProjectChanges`). */
export interface ProjectChanges {
  /** Each file's text, or null when it's gone (or isn't text). */
  files: { name: string; text: string | null }[];
  /** The project file's values, when it changed. */
  settings: { board: string | null; variant: Variant; serialBaud: number | null } | null;
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
  /** The Serial Monitor's speed (`[serial] baud`); null for the default, 115200. */
  serialBaud: number | null;
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
  /** The engine's label for people, e.g. "USB · Flash". */
  label: string;
  /** Whether programming this way was tried on hardware. */
  tested: boolean;
  /** Warnings for users, e.g. to hold a button while plugging the board in. */
  userNotes?: string;
}

/**
 * The project's own boards as Logix Engine reads them from its
 * boards folder (the file on disk), or the problems it found there:
 * its message, and a diagnostic for each problem at line and column, the
 * file named as the user knows it (its full path in a saved project,
 * `boards folder` in one never saved).
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
   * Every aspect, in the engine's order: board, family, fpga, size,
   * constraint_format, pins, clock (MHz lists), leds (counts); later
   * engines may add more, and older ones lack clock and leds.
   */
  aspects: {
    aspect: string;
    same: "yes" | "no" | "unknown";
    /** The two values, when known: any JSON type, by aspect. */
    from?: unknown;
    to?: unknown;
    /** For people, e.g. "FPGA: ICE40HX4K-TQ144 → ICE40UP5K-SG48.". */
    text: string;
  }[];
}

export type SaveChoice = "save" | "discard" | "cancel";

/** An FPGA family and its FPGAs, from Logix Engine's catalog. */
export interface FpgaFamily {
  /** e.g. "gw1n", as `Board.family` names it. */
  id: string;
  /** e.g. "Gowin GW1N". */
  label: string;
  /** Its pin files' extension, e.g. "cst" (null if unknown). */
  pinExtension: string | null;
  fpgas: { id: string; partNumber: string; size: string }[];
}

export interface Board {
  /** Board id, e.g. "alhambra-ii". */
  id: string;
  /** e.g. "Alhambra II (Lattice, iCE40)". */
  description: string;
  /** FPGA family, e.g. "ice40", "ecp5", "gw1n", "artix7" ("" if unknown). */
  family: string;
  /** The family's name for people, from Logix Engine, e.g. "Lattice iCE40". */
  familyLabel: string;
  /** The family's place in Logix Engine's order, for the board list. */
  familyOrder: number;
  /** The extension of the board's pin files, from Logix Engine, e.g. "pcf" (null if unknown). */
  pinExtension: string | null;
  /** How the board is programmed: Connection and Memory, for every board. */
  dimensions: Dimension[];
  /** The combinations that exist; the first is the default. */
  configurations: Configuration[];
  /** One of the project's own boards (boards folder), not a catalog board. */
  custom: boolean;
  /** The board's FPGA, from Logix Engine's catalog (null if it doesn't say). */
  fpga: Fpga | null;
  /**
   * What the board wires to FPGA pins, by port name, as Logix Engine gives
   * it: each with its `type` (clock, leds, uart…) and that type's fields.
   */
  resources?: Record<string, Record<string, unknown>>;
  /** What every design for the board must do (drives and setups), as the engine gives it. */
  requires?: Record<string, unknown>[];
  /** The board's own hints for AI agents, and people, writing designs for it (its resources have their own `ai_hints`). */
  aiHints?: string[];
}

/** A board's FPGA. */
export interface Fpga {
  /** The catalog's FPGA ID, e.g. "ice40hx4k-tq144-8k": boards with the same ID have the same FPGA (every board's FPGA is a catalog FPGA). */
  id: string;
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

/** "report" is Board Utilization: a build without lint, then the report. */
export type Task = "build" | "upload" | "test" | "simulate" | "diagram" | "report";

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
  /** FPGA family, e.g. "ice40", "ecp5", "gw1n", "artix7". */
  family: string;
  /**
   * Every resource, from Logix Engine's `resources`, sorted by name: the
   * toolchain's name (e.g. "ICESTORM_LC"), a short description (null when
   * the engine has none), used, available and the share in percent (null
   * when nextpnr's numbers aren't valid).
   */
  resources: {
    name: string;
    description: string | null;
    used: number;
    available: number;
    percent: number | null;
    /** The engine's share for people, e.g. "1%" (null when percent is). */
    displayPercent: string | null;
  }[];
  /** Each clock, from the engine's `clocks_summary`: plain name, net, speed. */
  clocks: { name: string; net: string; fmaxMhz: number }[];
  /** The engine's report for people (`report`), a line each. */
  text: string;
  /** The engine's one-line report (`report_summary`), for the status bar. */
  summaryLine: string;
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
  seconds: number;
  /**
   * Logix Engine's suggested action when it failed, e.g. "connect_board",
   * or "select_device" when Upload found several devices that fit the
   * board (in `devices`).
   */
  action: string | null;
  /** For Upload's "select_device": the devices that fit, best first. */
  devices: UsbDevice[];
  /**
   * For Upload's "install_drivers": the device that needs the WinUSB driver
   * (Windows) or a udev rule (Linux) first, for the user to agree; also
   * for "confirm_same_ids", when Logix Engine refused because other
   * connected devices have the same USB IDs (its question in `sameIds`).
   */
  driverDevice: DriverDevice | null;
  sameIds: Consent | null;
}

/** For Upload (Windows and Linux): install the board's driver or udev rule first, which the user agreed to. */
export interface UploadDriver {
  install: boolean;
  /** Also while other devices with the same USB IDs are connected. */
  allowSameIds: boolean;
}
export const NO_DRIVER: UploadDriver = { install: false, allowSameIds: false };

/** A device that needs a driver before Upload can program it. */
export interface DriverDevice {
  /** Logix Engine's device id, for Upload with `installDriver`. */
  id: string;
  /** The engine's name for people. */
  label: string;
  /** Logix Engine's name for it in a sentence, such as "the Alhambra II". */
  who: string;
  /** Logix Engine's `driver` for it, with its texts and the question to ask. */
  driver: Driver;
}

/**
 * What a device needs for its board's programmer (Logix Engine's `driver`).
 * On Windows, for a board whose programmer needs one: the driver it needs
 * (`needed` "winusb") on which USB interface, the one Windows has on it,
 * and whether the programmer can use it. On Linux, for every board: a udev
 * rule (`needed` "udev-rule", no interface), Logix Engine's rule file when
 * there's one (`current`), and whether the board can be opened (`ready`).
 */
export interface Driver {
  needed: string;
  interface: number | null;
  current: string | null;
  ready: boolean;
  /**
   * Logix Engine's texts for people, the same in every frontend and naming
   * none (moved from Logix IDE, 2026-11-18): what it needs ("a udev rule",
   * "the WinUSB driver"), its state, and a short tag while not ready.
   */
  label: string;
  state: string;
  tag: string | null;
  /** Whether Install/Add and Remove apply, by the engine. */
  canAdd: boolean;
  canRemove: boolean;
  /** The engine's questions before each. */
  add: Consent | null;
  remove: Consent | null;
}

/**
 * One of Logix Engine's questions before a driver or udev rule change, with
 * its status line while it runs (`busy`) and the line when the user
 * declines (`cancelled`).
 */
export interface Consent {
  title: string;
  lead: string;
  steps: string[];
  note: string | null;
  /** The agreeing button's label, such as "Install Driver". */
  confirm: string;
  busy: string;
  cancelled: string;
}

/** What Install Driver did (Tools > Connected Devices). */
export interface DriverInstalled {
  /** False when the driver was ready already. */
  installed: boolean;
  /** What was done, for people (Logix Engine's `summary`, a sentence each). */
  summary: string[];
  /**
   * Set when Logix Engine refused because other connected devices have the
   * same USB IDs: its question, naming them.
   */
  sameIds: Consent | null;
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
  | { kind: "progress"; message: string; percent: number | null }
  /**
   * What the task does now, when it isn't what its name says (Upload
   * installing the board's driver first); null: back to the task's own text.
   */
  | { kind: "status"; text: string | null };

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
  /** Its USB release (bcdDevice, four hex digits, such as "0100"); empty when it reports none. */
  release: string;
  /** The boards it may be, strongest first, each with its match level. */
  boards: { board: string; level: MatchLevel }[];
  /** The boards it most likely is, by the engine (none when only weak matches). */
  likelyBoards: string[];
  /** For Upload's candidates: how well it matches the build's board. */
  buildMatch: MatchLevel | null;
  /** Logix Engine's name for it in a sentence, such as "the Alhambra II". */
  who: string;
  /** On Windows and Linux, the driver or udev rule its board needs and has; null elsewhere. */
  driver: Driver | null;
  /** Its serial ports, each with its USB interface when known, and what it's for, by the engine. */
  serialPorts: { port: string; interface: number | null; role: "uart" | "programmer" | null; description: string }[];
  /** Its board's UART port, by the engine; null when it can't tell. */
  serialPort: string | null;
}

/** A serial port the Serial Monitor can open. */
export interface SerialChoice {
  /** The device it's on (Logix Engine's id). */
  device: string;
  /** The device for people, such as "the Alhambra II". */
  who: string;
  /** The device's full name, such as "Alhambra II v1.0A (AlhambraBits, 0403:6010)". */
  label: string;
  /** Such as COM5, /dev/cu.usbserial-A10 or /dev/ttyUSB1. */
  port: string;
  interface: number | null;
  /** The selected board's UART, by the engine: the port to open by itself. */
  boardUart: boolean;
  /** What it's for, by the engine: the board's UART, its programmer's interface, or not known. */
  role: "uart" | "programmer" | null;
  /** The engine's phrase for people, such as "The Alhambra II's programming port". */
  description: string;
}

/** What the Serial Monitor's read loop sends. */
export type SerialEvent =
  | { kind: "read"; text: string; events: string[]; open: boolean; port: string }
  | { kind: "ended"; message: string };

/**
 * Logix Engine's match level: what a match means for the user, never its
 * cause (the engine's criteria are internal and change without notice).
 * "strong": very likely this board; "possible": a few other boards look the
 * same; "weak": many boards fit. Only show and compare levels.
 */
export type MatchLevel = "strong" | "possible" | "weak";

/** A device that may be a board but can't be opened yet. */
export interface UnreachableDevice {
  /** Logix Engine's device id, for Install Driver (not for Upload). */
  id: string;
  /** The engine's name for people, e.g. "USB device (0403:6014)". */
  label: string;
  vid: string;
  pid: string;
  /** The suggested fix, e.g. "fix_device_permissions". */
  action: string;
  /** Logix Engine's name for it in a sentence. */
  who: string;
  /** What to do, for people, by Logix Engine. */
  fix: string;
  /** On Windows, the driver its board needs and has. */
  driver: Driver | null;
  /** The boards whose USB IDs it has (all weak: its names can't be read yet). */
  boards: { board: string; level: MatchLevel }[];
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
  /** Whether a saved project's changes are saved by themselves. */
  autoSave: boolean;
}

/**
 * When the hidden output comes back by itself: when a command starts (and
 * on errors), only when one fails, or never (only by hand).
 */
export type ShowOutput = "onCommand" | "onError" | "never";

export interface ExampleList {
  examples: Example[];
  /** Examples left out because Logix IDE doesn't support them yet. */
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
  /** The open tab is a testbench (the editor menu's Run Test, Simulate Test). */
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
  | "copy-path"
  | "stop"
  | "show-diagram"
  | "build-report"
  | "devices"
  | "serial-monitor"
  | "board-explorer"
  | "settings"
  | "go-to-definition"
  | "go-to-file"
  | "toggle-comment"
  | "select-all"
  | "output-select-all"
  | "clear-output"
  | "run-this-test"
  | "show-module-diagram"
  | "simulate-this-test"
  | "system-info"
  | "check-for-updates"
  // Not a menu item: the recent projects list changed (Clear Menu).
  | "recents-changed"
  // Tools > <dimension> > <value>: the board's variant.
  | `variant:${string}`;

export const backend = {
  appInfo: (): Promise<AppInfo> => invoke<AppInfo>("app_info"),

  /**
   * Help > System Info: the report for bug reports: Logix IDE's facts, the
   * developer settings, and the version tree with the engine's own under
   * it (`system_info.rs`); `text` is what Copy gives.
   */
  systemInfo: (): Promise<SystemInfo> => invoke<SystemInfo>("system_info"),

  /**
   * Help > Check for Updates: reads Logix IDE's remote config now, switches
   * to a new Logix Engine release if it names one, and has the engine check
   * its own packages. Progress goes to the console (`tool-output`); returns
   * what changed, a line each, or a line saying everything is up to date.
   */
  /**
   * Help > Check for Updates: the lines saying what changed. `onProgress`
   * gets what downloads meanwhile, with its percentage (`progress` events),
   * for the status bar.
   */
  checkForUpdates: (onProgress: (event: TaskEvent) => void): Promise<string[]> => {
    const channel = new Channel<TaskEvent>();
    channel.onmessage = onProgress;
    return invoke<string[]>("check_for_updates", { onProgress: channel });
  },

  /**
   * The newer-version banner: a newer Logix IDE on this build's channel,
   * or null (none, or the banner was dismissed in the last week). Looks at
   * most once a day; never fails.
   */
  newerIdeNotice: (): Promise<NewerIde | null> => invoke<NewerIde | null>("newer_ide_notice"),

  /** The banner's Dismiss: it stays away for a week. */
  dismissNewerIde: (): Promise<void> => invoke<void>("dismiss_newer_ide"),

  /** The banner's Download: the newer release's page, in the browser. */
  openNewerIdePage: (): Promise<void> => invoke<void>("open_newer_ide_page"),

  /** Puts text on the clipboard. */
  copyText: (text: string): Promise<void> => invoke<void>("copy_text", { text }),

  /**
   * Makes sure Logix Engine and its packages (the FPGA tools) are
   * downloaded and starts it. Resolves when ready, rejects with a
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
   * Every FPGA Logix Engine supports, by family in the engine's order, each
   * family's FPGAs by part number (the Board Explorer). Needs the FPGA
   * tools ready.
   */
  getFpgaFamilies: (): Promise<FpgaFamily[]> => invoke<FpgaFamily[]>("get_fpga_families"),

  /**
   * The examples Logix IDE supports, sorted by board and name, and the number
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
   * Builds, uploads, tests, simulates or draws (diagram) the given (possibly
   * unsaved) files. `testbench` picks one testbench for "test" (all if
   * null) and "simulate", or the module the Diagram window opens on for "diagram" (main if null). `key` identifies the project; `sourceDir` is the folder
   * whose other files are copied along; `projectDir` is the saved
   * project's folder (null for a project never saved). `device` is the
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
    driver: UploadDriver = NO_DRIVER,
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
      installDriver: driver.install,
      allowSameIds: driver.allowSameIds,
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
   * File > New Project: an unsaved project for `board`
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
    serialBaud: number | null,
  ): Promise<void> =>
    invoke<void>("save_project", { dir, files, removed, board, variant, serialBaud }),

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
    serialBaud: number | null,
  ): Promise<ProjectDoc | null> =>
    invoke<ProjectDoc | null>("save_project_as_dialog", {
      projectDir,
      sourceDir,
      suggestedName,
      files,
      removed,
      board,
      variant,
      serialBaud,
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

  /**
   * The connected devices' serial ports, the selected board's UART first
   * (Logix Engine finds it from the board's data).
   */
  serialPorts: (folder: string | null, saved: boolean, board: string | null): Promise<SerialChoice[]> =>
    invoke<SerialChoice[]>("serial_ports", { folder, saved, board }),
  /**
   * Opens the Serial Monitor's port (closing one already open); what arrives
   * comes through `onSerial`. `device` lets Logix Engine pause it during
   * Upload and reopen it after an unplug.
   */
  serialOpen: (port: string, baud: number, device: string | null): Promise<string> =>
    invoke<string>("serial_open", { port, baud, device }),
  /** Sends text as it is (add the line ending). */
  serialWrite: (text: string): Promise<void> => invoke<void>("serial_write", { text }),
  serialClose: (): Promise<void> => invoke<void>("serial_close"),
  onSerial: (handler: (event: SerialEvent) => void): Promise<UnlistenFn> =>
    listen<SerialEvent>("serial", (e) => handler(e.payload)),

  /** Messages (JSON text) from the language server. */
  onLspMessage: (handler: (message: string) => void): Promise<UnlistenFn> =>
    listen<string>("lsp-message", (e) => handler(e.payload)),

  /**
   * The waveform window's page saved its view (the signals shown, their
   * formats, the zoom): the project (its build key), the view file's name
   * (`main_tb.view`) and its text, a project file taken in as an edit.
   */
  onWaveView: (
    handler: (view: { key: string; name: string; text: string }) => void,
  ): Promise<UnlistenFn> =>
    listen<{ key: string; name: string; text: string }>("wave-view", (e) => handler(e.payload)),

  /**
   * A signal's name or a failure clicked in the waveform window: the
   * project (its build key) and the file and line to show. The backend
   * sends only the project's files.
   */
  onWaveOpenSource: (
    handler: (at: { key: string; file: string; line: number }) => void,
  ): Promise<UnlistenFn> =>
    listen<{ key: string; file: string; line: number }>("wave-open-source", (e) =>
      handler(e.payload),
    ),

  /** Tools > Add AI Agent Guide: writes AGENTS.md and CLAUDE.md unless they exist; the files written. */

  /** Watches the open project's folder for changes other programs make (null: stops). */
  watchProject: (dir: string | null): Promise<void> => invoke("watch_project", { dir }),
  /** Files that changed in the watched project folder (names only; read them with readProjectChanges). */
  onProjectFilesChanged: (
    handler: (change: { dir: string; names: string[] }) => void,
  ): Promise<UnlistenFn> =>
    listen<{ dir: string; names: string[] }>("project-files-changed", (e) => handler(e.payload)),
  /** The files `names` of the project in `dir` as they are on disk now. */
  readProjectChanges: (dir: string, names: string[]): Promise<ProjectChanges> =>
    invoke<ProjectChanges>("read_project_changes", { dir, names }),

  /**
   * A block clicked in a Diagram window: the project (its build key) and
   * the file and line to show. The backend sends only the project's files.
   */
  onDiagramOpenSource: (
    handler: (at: { key: string; file: string; line: number }) => void,
  ): Promise<UnlistenFn> =>
    listen<{ key: string; file: string; line: number }>("diagram-open-source", (e) =>
      handler(e.payload),
    ),

  /** The language server stopped. */
  onLspClosed: (handler: () => void): Promise<UnlistenFn> =>
    listen("lsp-closed", () => handler()),

  /**
   * The connected USB devices, matched to the catalog's boards and those of
   * the boards folder in `folder` (the project's folder, `copySource`;
   * null for none). `saved`: whether the project is saved, which decides
   * how messages name the file (its full path, or `boards folder`).
   */
  scanDevices: (folder: string | null, saved: boolean, board: string | null): Promise<Devices> =>
    invoke<Devices>("scan_devices", { folder, saved, board }),

  /**
   * Installs the WinUSB driver a device's board needs (Windows), after the
   * user agreed; Windows asks for permission for Logix Engine. With
   * `allowSameIds`, also while other devices with the same USB IDs are
   * connected (they get it too).
   */
  installDeviceDriver: (
    folder: string | null,
    saved: boolean,
    device: string,
    allowSameIds: boolean,
    board: string | null,
    onEvent: (event: TaskEvent) => void,
  ): Promise<DriverInstalled> => {
    const channel = new Channel<TaskEvent>();
    channel.onmessage = onEvent;
    return invoke<DriverInstalled>("install_device_driver", { folder, saved, device, allowSameIds, board, onEvent: channel });
  },

  /**
   * Removes the WinUSB driver Logix Engine installed for a device's board
   * (Windows), so Windows restores its original driver, for every device with the
   * same USB IDs. Resolves with what was done, for people (Logix Engine's
   * `summary`).
   */
  uninstallDeviceDriver: (
    folder: string | null,
    saved: boolean,
    device: string,
    board: string | null,
    onEvent: (event: TaskEvent) => void,
  ): Promise<string[]> => {
    const channel = new Channel<TaskEvent>();
    channel.onmessage = onEvent;
    return invoke<string[]>("uninstall_device_driver", { folder, saved, device, board, onEvent: channel });
  },

  /**
   * The connected USB devices that fit boards (the catalog's and those of
   * the boards folder in `folder`), with their boards (silent).
   */
  connectedDevices: (folder: string | null, saved: boolean): Promise<ConnectedDevice[]> =>
    invoke<ConnectedDevice[]>("connected_devices", { folder, saved }),

  /**
   * The project's own boards, as Logix Engine reads them from the
   * boards folder in `folder` (the file on disk, edited outside
   * Logix IDE; none without it), or the problems it found in it, the file
   * named as the user knows it: its full path when the project is `saved`,
   * `boards folder` when it isn't.
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
   * A value that changes whenever the boards folder in `folder` is
   * added, changed or removed on disk (null without it), polled to notice
   * edits made outside Logix IDE.
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

  /** Appends lines the output pane showed to the Logix IDE log file. */
  logOutput: (lines: string[]): Promise<void> =>
    invoke<void>("log_output", { lines }),

  /**
   * Shows the editor's right-click menu at the mouse. Its items come back
   * as `menu` events.
   */
  showOutputMenu: (): Promise<void> => invoke<void>("show_output_menu"),

  /** Shows the editor's right-click menu, its items as in `setMenuState`. */
  /**
   * Shows the editor's right-click menu; with a module, it offers Show
   * Diagram of that module.
   */
  showEditorMenu: (module: string | null): Promise<void> =>
    invoke<void>("show_editor_menu", { module }),

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
