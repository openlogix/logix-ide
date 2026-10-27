import type { Board } from "./backend";

/** A titled group of boards in the board picker. */
export interface BoardGroup {
  label: string;
  boards: Board[];
  /** Architecture groups can be collapsed; "Connected" can't. */
  collapsible: boolean;
}

/** FPGA architectures (Apio's ids), in the picker's order, with names. */
const ARCHITECTURES: [string, string][] = [
  ["ice40", "Lattice iCE40"],
  ["ecp5", "Lattice ECP5"],
  ["gowin", "Gowin"],
  ["xilinx", "Xilinx"],
];

/** A plain name for an FPGA architecture id, e.g. "Lattice iCE40". */
export function archLabel(arch: string): string {
  const known = ARCHITECTURES.find(([id]) => id === arch);
  if (known) return known[1];
  return arch ? arch.toUpperCase() : "Other";
}

/**
 * Splits `items` by FPGA architecture (`arch` gives an item's), in the
 * fixed order (iCE40, ECP5, Gowin, Xilinx), then others by name.
 */
export function groupByArch<T>(
  items: T[],
  arch: (item: T) => string,
): { label: string; items: T[] }[] {
  const order = ARCHITECTURES.map(([id]) => id);
  const rank = (a: string) => {
    const i = order.indexOf(a);
    return i < 0 ? order.length : i;
  };
  const archs = [...new Set(items.map(arch))].sort(
    (a, b) => rank(a) - rank(b) || archLabel(a).localeCompare(archLabel(b)),
  );
  return archs.map((a) => ({
    label: archLabel(a),
    items: items.filter((item) => arch(item) === a),
  }));
}

/**
 * The board picker's groups: the connected boards first, then all boards
 * by FPGA architecture. With a search query, only boards whose name, id or
 * architecture contains it; empty groups are left out.
 */
export function boardGroups(
  boards: Board[],
  connected: string[],
  query: string,
): BoardGroup[] {
  const q = query.trim().toLowerCase();
  const matches = (b: Board) =>
    !q ||
    b.description.toLowerCase().includes(q) ||
    b.id.toLowerCase().includes(q) ||
    archLabel(b.arch).toLowerCase().includes(q);
  const shown = boards.filter(matches);

  const groups: BoardGroup[] = [];
  const plugged = connected
    .map((id) => shown.find((b) => b.id === id))
    .filter((b): b is Board => b !== undefined);
  if (plugged.length > 0)
    groups.push({ label: "Connected", boards: plugged, collapsible: false });
  for (const g of groupByArch(shown, (b) => b.arch)) {
    groups.push({ label: g.label, boards: g.items, collapsible: true });
  }
  return groups;
}
