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
 * only boards whose name, id or architecture contains it; empty groups are
 * left out.
 */
export function boardGroups(boards: Board[], recent: string[], query: string): BoardGroup[] {
  const q = query.trim().toLowerCase();
  const matches = (b: Board) =>
    !q ||
    b.description.toLowerCase().includes(q) ||
    b.id.toLowerCase().includes(q) ||
    b.archLabel.toLowerCase().includes(q);
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
