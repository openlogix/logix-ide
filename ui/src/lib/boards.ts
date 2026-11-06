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

/**
 * What changing a project from board `from` to board `to` means, as short
 * points for the "change the board" choice (empty when nothing needs
 * saying): facts, and the risky consequences, which the dialog shows in
 * red. `pinFile` is the project's pin file, such as "pinout.pcf". From
 * the catalog's facts only: the same FPGA keeps the design, another FPGA
 * of the family needs this board's pins, and another family needs another
 * kind of pin file and probably significant changes.
 */
export function boardChangeNote(from: Board | null, to: Board, pinFile: string | null): NotePoint[] {
  if (!from || from.id === to.id) return [];
  const fact = (text: string): NotePoint => ({ text, risky: false });
  const risk = (text: string): NotePoint => ({ text, risky: true });
  const pins = pinFile ?? "the pin file";
  if (from.arch !== to.arch && from.arch && to.arch) {
    const ext = to.pinExtension ? `a .${to.pinExtension} pin file` : "another kind of pin file";
    return [
      fact(`Another FPGA family: ${from.archLabel} → ${to.archLabel}.`),
      risk(`Needs ${ext}${pinFile ? ` instead of ${pinFile}` : ""}.`),
      risk(`Probably significant changes: ${from.archLabel} parts, such as I/O cells and PLLs, won't build.`),
    ];
  }
  if (from.fpga?.id && from.fpga.id === to.fpga?.id) {
    return [fact(`Same FPGA: ${to.fpga.partNumber}.`), risk(`Check that the pins in ${pins} match this board.`)];
  }
  if (from.fpga && to.fpga) {
    return [
      fact(`Another ${to.archLabel} FPGA: ${from.fpga.partNumber} → ${to.fpga.partNumber}.`),
      risk(`${pinFile ?? "The pin file"} needs this board's pins.`),
    ];
  }
  return [];
}
