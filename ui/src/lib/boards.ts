import type { Board, BoardComparison, ProjectBoards } from "./backend";

/** A titled group of boards in the board picker. */
export interface BoardGroup {
  /** Unique among the groups: its label. */
  key: string;
  label: string;
  boards: Board[];
  /**
   * "custom" (the project's custom-boards.json) and "recent" (Recently
   * used) start expanded; architecture groups start collapsed, and their
   * state is remembered. All can be collapsed.
   */
  kind: "custom" | "recent" | "arch";
}

/** The board picker's group of the project's own boards (custom-boards.json). */
export const CUSTOM_GROUP = "Custom boards";
/** The board picker's group of the boards chosen most recently. */
export const RECENT_GROUP = "Recently used";

/** What groups a board by FPGA architecture, as Logix Engine describes it. */
type Arch = Pick<Board, "arch" | "archLabel" | "archOrder">;

/**
 * Splits `items` by FPGA architecture (`board` gives an item's board), in
 * Logix Engine's order (its catalog's `architectures`), then others by name.
 */
export function groupByArch<T>(
  items: T[],
  board: (item: T) => Arch,
): { label: string; items: T[] }[] {
  const archs = new Map<string, Arch>();
  for (const item of items) {
    const b = board(item);
    if (!archs.has(b.arch)) archs.set(b.arch, b);
  }
  return [...archs.values()]
    .sort((a, b) => a.archOrder - b.archOrder || a.archLabel.localeCompare(b.archLabel))
    .map((a) => ({
      label: a.archLabel,
      items: items.filter((item) => board(item).arch === a.arch),
    }));
}

/**
 * The console lines for problems Logix Engine found in the project's
 * custom-boards.json, in red: one per problem at line and column, the
 * file named as the engine was asked (`/Users/you/blinky/custom-boards.json:4:3:
 * error: Trailing comma.` in a saved project), like build errors, so the
 * user can find them in the editor they edit it with; the engine's message
 * when it gives no locations.
 */
export function boardsProblemLines(problem: NonNullable<ProjectBoards["problem"]>): string[] {
  const red = (text: string) => `\x1b[31m${text}\x1b[0m`;
  if (problem.diagnostics.length === 0) return [red(problem.message)];
  return problem.diagnostics.map((d) => {
    const at = [d.file ?? "", d.line, d.column].filter((x) => x != null).join(":");
    return red(`${at}: ${d.severity === "error" ? "error" : "warning"}: ${d.message}`);
  });
}

/**
 * The boards to offer: the project's own boards (custom-boards.json) first,
 * then the catalog's; one with a catalog board's ID replaces it, as in
 * Logix Engine.
 */
export function withProjectBoards(catalog: Board[], own: Board[]): Board[] {
  if (own.length === 0) return catalog;
  const ids = new Set(own.map((b) => b.id));
  return [...own, ...catalog.filter((b) => !ids.has(b.id))];
}

/**
 * The board picker's groups: "Custom boards", the project's own boards
 * (custom-boards.json; "custom" rather than "project", since a project's
 * catalog board is its board too: the user's naming, 2026-11-07), then the
 * boards chosen most recently (`recent`, newest first, the ones still
 * offered), then every board by FPGA architecture, the project's own
 * boards included. With a search query, only boards whose name, id,
 * architecture or FPGA (part number or ID) contains it; empty groups are
 * left out, except "Custom boards" when `customProblem` (its file has
 * problems, so the group says so instead of listing boards).
 */
export function boardGroups(
  boards: Board[],
  recent: string[],
  query: string,
  customProblem = false,
): BoardGroup[] {
  const q = query.trim().toLowerCase();
  // The FPGA's part number and ID count too, though the list doesn't show
  // them, so "up5k" or "xc7a35t" finds the boards with that FPGA.
  const matches = (b: Board) =>
    !q ||
    [b.description, b.id, b.archLabel, b.fpga?.partNumber ?? "", b.fpga?.id ?? ""].some((t) =>
      t.toLowerCase().includes(q),
    );
  const shown = boards.filter(matches);

  const groups: BoardGroup[] = [];
  // The project's own boards first (also in their architecture's group);
  // with problems in its file, the group stays, empty, to say so.
  const custom = shown.filter((b) => b.custom);
  if (custom.length > 0 || customProblem)
    groups.push({ key: CUSTOM_GROUP, label: CUSTOM_GROUP, boards: custom, kind: "custom" });
  const recentBoards = recent
    .map((id) => shown.find((b) => b.id === id))
    .filter((b): b is Board => b !== undefined);
  if (recentBoards.length > 0)
    groups.push({ key: RECENT_GROUP, label: RECENT_GROUP, boards: recentBoards, kind: "recent" });
  for (const g of groupByArch(shown, (b) => b)) {
    groups.push({ key: g.label, label: g.label, boards: g.items, kind: "arch" });
  }
  return groups;
}

/** A board's FPGA for people, e.g. "Lattice iCE40 ICE40HX4K-TQ144 (8k)"; "" if unknown. */
export function fpgaLabel(b: Board): string {
  if (!b.fpga) return "";
  return `${b.archLabel} ${b.fpga.partNumber}${b.fpga.size ? ` (${b.fpga.size})` : ""}`;
}

/**
 * Every board with the same FPGA as `b` (the catalog's FPGA ID), `b`
 * included, by name: the same list for each of them, so Board Info's list
 * stays put while browsing it. Just `b` when its FPGA has no ID (a custom
 * board's own FPGA definition).
 */
export function boardsWithFpga(boards: Board[], b: Board): Board[] {
  const id = b.fpga?.id;
  const same = id ? boards.filter((x) => x.fpga?.id === id) : [];
  if (!same.some((x) => x.id === b.id)) same.push(b);
  return same.sort((x, y) => x.description.localeCompare(y.description));
}

/**
 * One point of a board-change note: the change itself (the first point),
 * or one of its implications, all risky (shown in red: the user's
 * decision, since every implication is a possible problem).
 */
export interface NotePoint {
  text: string;
  kind: "change" | "risky";
}

/**
 * What moving a project from one board to another means for a Logix
 * project, which keeps its sources as they are: 2 to 4 points (the user's
 * design), the first naming the change at its highest level
 * (architecture, FPGA or board), the rest the risks of keeping the
 * sources. Empty for the same board. The facts come from Logix Engine's
 * `compare_boards` (`c`); the wording is the IDE's, since what a change
 * means depends on the project model. `pinFile` is the project's pin file,
 * such as "pinout.pcf".
 */
export function boardChangeNote(c: BoardComparison | null, pinFile: string | null): NotePoint[] {
  if (!c) return [];
  const aspect = (name: string) => c.aspects.find((a) => a.aspect === name);
  const differs = (name: string) => aspect(name)?.same === "no";
  const change = (text: string): NotePoint => ({ text, kind: "change" });
  const risk = (text: string): NotePoint => ({ text, kind: "risky" });
  const pins = pinFile ?? "the pin file";
  const arch = aspect("architecture");
  if (arch && differs("architecture")) {
    const format = aspect("constraint_format")?.to as string | undefined;
    const needs = format ? `a ${format} pin file` : "another kind of pin file";
    return [
      change(arch.text),
      risk(`Needs ${needs}${pinFile ? ` instead of ${pinFile}` : ""}, with this board's pins.`),
      risk(`Probably significant changes: ${String(arch.from)} parts, such as I/O cells and PLLs, won't build.`),
      risk("The clock frequency and the LED and button wiring may differ: check the design."),
    ];
  }
  const fpga = aspect("fpga");
  if (fpga && differs("fpga")) {
    return [
      change(fpga.text),
      risk("FPGA-specific parts, such as PLLs and special cells, may need changes."),
      risk("The design may not fit, or may not reach its clock speed."),
      risk(`The pins, clock frequency and LED and button wiring may differ: check ${pins} and the design.`),
    ];
  }
  const board = aspect("board");
  if (!board || !differs("board")) return [];
  return [
    change([board.text, fpga?.text].filter(Boolean).join(" ")),
    ...(aspect("pins")?.same === "yes" ? [] : [risk(`The pins may differ: check ${pins}.`)]),
    risk("The clock frequency and the LED and button wiring may differ: check the design."),
  ];
}
