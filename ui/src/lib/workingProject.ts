// The project being edited, held in memory. Creating, renaming and deleting
// files only take effect on disk when the project is saved.
import type { ProjectDoc, Variant } from "./backend";

/** The main file of new projects. */
export const MAIN_FILE = "main.v";

export interface OpenFile {
  name: string;
  text: string;
  /** Text as last loaded or saved, to detect edits. */
  savedText: string;
  /** Whether a file with this name exists in the project folder. */
  onDisk: boolean;
}

export interface WorkingProject {
  /** Project folder, or null for a new, never saved project. */
  dir: string | null;
  /** Example folder whose other files are copied on the first save. */
  templateDir: string | null;
  name: string;
  board: string | null;
  /** Board as last loaded or saved. */
  savedBoard: string | null;
  /** The board's variant ({} for its default), as stored in the project. */
  variant: Variant;
  /** Variant as last loaded or saved. */
  savedVariant: Variant;
  /** The main file (main.v or main.sv) first. */
  files: OpenFile[];
  /** Files in the folder that were deleted or renamed since the last save. */
  removed: string[];
  /** Files were added, renamed or deleted since the last save. */
  structureChanged: boolean;
}

export function fromDoc(doc: ProjectDoc): WorkingProject {
  return {
    dir: doc.dir,
    templateDir: doc.templateDir,
    name: doc.name,
    board: doc.board,
    savedBoard: doc.board,
    variant: { ...doc.variant },
    savedVariant: { ...doc.variant },
    files: doc.files.map((f) => ({
      name: f.name,
      text: f.text,
      savedText: f.text,
      onDisk: doc.dir !== null,
    })),
    removed: [],
    structureChanged: false,
  };
}

export function isFileDirty(p: WorkingProject, f: OpenFile): boolean {
  return f.text !== f.savedText || (p.dir !== null && !f.onDisk);
}

export function isDirty(p: WorkingProject): boolean {
  return (
    p.structureChanged ||
    p.board !== p.savedBoard ||
    !sameVariant(p.variant, p.savedVariant) ||
    p.files.some((f) => f.text !== f.savedText)
  );
}

/** Whether two variants have the same values. */
export function sameVariant(a: Variant, b: Variant): boolean {
  const ka = Object.keys(a);
  return ka.length === Object.keys(b).length && ka.every((k) => a[k] === b[k]);
}

/**
 * The modules the diagram can show: those declared in the project's
 * Verilog files, except testbenches, with "main" first.
 */
export function projectModules(p: WorkingProject): string[] {
  const names = new Set<string>();
  for (const f of p.files) {
    if (!isHdlSource(f.name) || isTestbench(f.name)) continue;
    for (const m of f.text.matchAll(/^\s*module\s+([A-Za-z_][\w$]*)/gm)) {
      names.add(m[1]);
    }
  }
  const rest = [...names].filter((n) => n !== "main").sort();
  return names.has("main") ? ["main", ...rest] : rest;
}

/** Identifies the project for its build folder (see build_dir.rs). */
export function buildKey(p: WorkingProject): string {
  return p.dir ?? p.templateDir ?? "untitled";
}

/** Folder whose other contents "Save As" copies along. */
export function copySource(p: WorkingProject): string | null {
  return p.dir ?? p.templateDir;
}

/** The project's main file name (main.v or main.sv): always the first. */
export function mainFile(p: WorkingProject): string {
  return p.files[0]?.name ?? MAIN_FILE;
}

export function findFile(p: WorkingProject, name: string): OpenFile | undefined {
  return p.files.find((f) => f.name === name);
}

/** Whether a file is Verilog or SystemVerilog source (what Auto Format handles). */
export function isHdlSource(name: string): boolean {
  return /\.s?vh?$/i.test(name);
}

/** Whether a file is a testbench (`*_tb.v` or `*_tb.sv`), as Logix IDE names them. */
export function isTestbench(name: string): boolean {
  return /_tb\.s?v$/i.test(name);
}

/** What a project file is for, as shown on its tab (icon and tooltip). */
export type FileRole =
  | "main"
  | "design"
  | "include"
  | "testbench"
  | "pins"
  | "otherPins";

/**
 * The project's own boards: a folder in the project, a board file each in
 * Logix Catalog's format, named by board ID (`boards/my-board.json`),
 * edited outside Logix IDE (not tabs). Logix Engine reads it as
 * `custom_boards_folder`.
 */
export const CUSTOM_BOARDS_FOLDER = "boards";

/** How messages name the project's custom boards folder. */
export const BOARDS_FOLDER_NAME = "boards folder";

/** Whether `file` (as a diagnostic names it) is one of the project's board files. */
export function isBoardFile(file: string): boolean {
  return file.endsWith(".json") && (file.startsWith(`${CUSTOM_BOARDS_FOLDER}/`) || file.includes(`/${CUSTOM_BOARDS_FOLDER}/`));
}

/** Pin constraint file extensions (as `CONSTRAINT_EXTENSIONS` in `build_dir.rs`). */
const PIN_FILE = /\.(pcf|lpf|cst|xdc)$/i;

/**
 * The role of the project file `name`; `main` is the project's main file
 * and `pinExtension` the board's pin file extension (e.g. "pcf"; null if
 * unknown). A pin file of another kind (e.g. .lpf on an iCE40 board) isn't
 * used with the board: `otherPins`.
 */
export function fileRole(
  name: string,
  main: string,
  pinExtension: string | null = null,
): FileRole {
  if (name === main) return "main";
  if (isTestbench(name)) return "testbench";
  if (PIN_FILE.test(name)) {
    const ext = name.slice(name.lastIndexOf(".") + 1).toLowerCase();
    return pinExtension === null || ext === pinExtension.toLowerCase()
      ? "pins"
      : "otherPins";
  }
  if (/\.s?vh$/i.test(name)) return "include";
  return "design";
}

/**
 * A tab's tooltip: what the file is for, and for others than the main
 * file, how to rename it. `pinExtension` as in `fileRole`.
 */
export function roleTooltip(role: FileRole, pinExtension: string | null = null): string {
  const what: Record<FileRole, string> = {
    main: "Main file: the design's top module, named main",
    design: "Design file: part of your design",
    include: "Include file: text other Verilog files include",
    testbench: "Testbench: check your design with Run Tests or Simulate",
    pins: "Pin file: connects your design's signals to the board's pins",
    otherPins: `Pin file for another kind of board: not used with this board${
      pinExtension ? `, whose pin file is a .${pinExtension} file` : ""
    }`,
  };
  return role === "main"
    ? `${what.main}.`
    : `${what[role]}. Double-click to rename; right-click for more.`;
}

/** The project's testbench files. */
export function testbenches(p: WorkingProject): string[] {
  return p.files.map((f) => f.name).filter(isTestbench);
}

/**
 * Starting text for a new file: a testbench skeleton for `*_tb.v`, an
 * empty module for other Verilog files, and nothing otherwise.
 */
export function starterText(name: string): string {
  const match = /^([A-Za-z_][A-Za-z0-9_$]*)\.(s?v)$/i.exec(name);
  if (!match) return "";
  const module = match[1];
  if (isTestbench(name)) return testbenchTemplate(module);
  return `module ${module} (\n);\n\nendmodule\n`;
}

function testbenchTemplate(module: string): string {
  return `// Testbench. Run it with Project > Run Tests, or view its signals
// with Project > Simulate.

\`timescale 1ns / 1ps

module ${module};

  initial begin
    $dumpvars(0, ${module});

    // Your test steps go here. To fail the test when a check fails:
    //   if (actual !== expected)
    //     $error("expected %h, got %h", expected, actual);
    // Run Tests stops at the first error; Simulate runs to the end and
    // shows the signals in GTKWave.

    #100;
    $finish;
  end

endmodule
`;
}

export function addFile(p: WorkingProject, name: string, text: string) {
  p.files.push({ name, text, savedText: text, onDisk: false });
  p.structureChanged = true;
}

export function renameFile(p: WorkingProject, oldName: string, newName: string) {
  const f = findFile(p, oldName);
  if (!f || oldName === mainFile(p)) return;
  if (f.onDisk) p.removed.push(oldName);
  f.name = newName;
  f.onDisk = false;
  p.structureChanged = true;
}

export function deleteFile(p: WorkingProject, name: string) {
  const i = p.files.findIndex((f) => f.name === name);
  if (i < 0 || name === mainFile(p)) return;
  if (p.files[i].onDisk) p.removed.push(name);
  p.files.splice(i, 1);
  p.structureChanged = true;
}

/**
 * After a successful save of `saved` (from `filesForSave`), `board` and
 * `variant` into `dir`, which may be new after Save As. Changes made while
 * saving stay unsaved.
 */
export function markSaved(
  p: WorkingProject,
  dir: string,
  name: string,
  saved: { name: string; text: string }[],
  board: string | null,
  variant: Variant,
) {
  p.dir = dir;
  p.templateDir = null;
  p.name = name;
  p.savedBoard = board;
  p.savedVariant = { ...variant };
  for (const f of p.files) {
    const s = saved.find((x) => x.name === f.name);
    if (s) {
      f.savedText = s.text;
      f.onDisk = true;
    }
  }
  p.removed = [];
  p.structureChanged = false;
}

/** The files as the backend wants them for saving. */
export function filesForSave(p: WorkingProject) {
  return p.files.map((f) => ({ name: f.name, text: f.text }));
}
