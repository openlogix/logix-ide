import type { Board } from "./backend";

/** A titled group of boards in the board picker. */
export interface BoardGroup {
  /** Unique among the groups: its label. */
  key: string;
  label: string;
  boards: Board[];
  /**
   * The "Recently used" group: starts expanded. Architecture groups start
   * collapsed, and their state is remembered. Both can be collapsed.
   */
  recent: boolean;
}

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
 * The board picker's groups: the boards chosen most recently (`recent`,
 * newest first, the ones still offered), then every board by FPGA
 * architecture, the project's own boards included. With a search query,
 * only boards whose name, id, architecture or FPGA (part number or ID)
 * contains it; empty groups are left out.
 */
export function boardGroups(boards: Board[], recent: string[], query: string): BoardGroup[] {
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
  const recentBoards = recent
    .map((id) => shown.find((b) => b.id === id))
    .filter((b): b is Board => b !== undefined);
  if (recentBoards.length > 0)
    groups.push({ key: RECENT_GROUP, label: RECENT_GROUP, boards: recentBoards, recent: true });
  for (const g of groupByArch(shown, (b) => b)) {
    groups.push({ key: g.label, label: g.label, boards: g.items, recent: false });
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

/** One point of a board-change note: `risky` ones are shown in red. */
export interface NotePoint {
  text: string;
  risky: boolean;
}

/** One level of a board-change note: its title and its points (2 to 4). */
export interface NoteGroup {
  /** E.g. "FPGA change: ICE40HX4K-TQ144 → ICE40UP5K-SG48". */
  title: string;
  points: NotePoint[];
}

/**
 * What changing a project from board `from` to board `to` means, by level
 * (the user's design): "Board change" for any other board, then "FPGA
 * change" when the FPGA differs, then "Architecture change" when the FPGA
 * family differs, each crossed level adding its group. Empty for the same
 * board. `pinFile` is the project's pin file, such as "pinout.pcf". The
 * board-change points are things to check (not red); the FPGA and
 * architecture changes' are risky (red). From the catalog's facts only.
 */
export function boardChangeNote(from: Board | null, to: Board, pinFile: string | null): NoteGroup[] {
  if (!from || from.id === to.id) return [];
  const check = (text: string): NotePoint => ({ text, risky: false });
  const risk = (text: string): NotePoint => ({ text, risky: true });
  const pins = pinFile ?? "the pin file";
  const groups: NoteGroup[] = [
    {
      title: "Board change",
      points: [
        check(`The pins may differ: check ${pins}.`),
        check("The clock frequency may differ: check rates such as blinking and baud rates."),
        check("LEDs and buttons may be active-low instead of active-high, or the other way."),
      ],
    },
  ];
  const otherArch = !!from.arch && !!to.arch && from.arch !== to.arch;
  const otherFpga = otherArch || (!!from.fpga && !!to.fpga && from.fpga.id !== to.fpga.id);
  if (otherFpga) {
    const parts =
      from.fpga && to.fpga ? `: ${from.fpga.partNumber} → ${to.fpga.partNumber}` : "";
    groups.push({
      title: `FPGA change${parts}`,
      points: [
        risk("FPGA-specific parts, such as PLLs and special cells, may need changes."),
        risk("The design may not fit, or may not reach its clock speed."),
      ],
    });
  }
  if (otherArch) {
    const ext = to.pinExtension ? `a .${to.pinExtension} pin file` : "another kind of pin file";
    groups.push({
      title: `Architecture change: ${from.archLabel} → ${to.archLabel}`,
      points: [
        risk(`Needs ${ext}${pinFile ? ` instead of ${pinFile}` : ""}.`),
        risk(`Probably significant changes: ${from.archLabel} parts, such as I/O cells and PLLs, won't build.`),
      ],
    });
  }
  return groups;
}
