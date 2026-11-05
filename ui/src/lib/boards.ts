import type { Board, ConnectedDevice } from "./backend";

/** A titled group of boards in the board picker. */
export interface BoardGroup {
  /** Unique among the groups (a device's id, or the label). */
  key: string;
  label: string;
  boards: Board[];
  /**
   * A connected device's group: starts expanded, and its state isn't kept
   * (device ids are short-lived). Architecture groups start collapsed and
   * are remembered. Both can be collapsed.
   */
  device: boolean;
}

/** A connected device's group in the board picker: the boards it may be. */
export interface DeviceGroup {
  /** The device's id. */
  key: string;
  /** E.g. "Connected: Digilent USB Device (210319AFEF8F)". */
  label: string;
  /** The boards it may be, most likely first (the engine's confidence). */
  boards: string[];
}

/** FPGA architectures (the engine's ids), in the picker's order, with names. */
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
 * The board picker's groups: one per connected device, with the boards it
 * may be (one device is one board), then all boards by FPGA architecture.
 * With a search query, only boards whose name, id or architecture contains
 * it; empty groups are left out.
 */
export function boardGroups(
  boards: Board[],
  devices: DeviceGroup[],
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
  for (const d of devices) {
    const its = d.boards
      .map((id) => shown.find((b) => b.id === id))
      .filter((b): b is Board => b !== undefined);
    if (its.length > 0)
      groups.push({ key: d.key, label: d.label, boards: its, device: true });
  }
  for (const g of groupByArch(shown, (b) => b.arch)) {
    groups.push({ key: g.label, label: g.label, boards: g.items, device: false });
  }
  return groups;
}

/**
 * The board picker's device groups from the connected devices: per device,
 * its suggested boards, most likely first (the engine's order: by
 * confidence), plus `selected` when the device fits it, so the toolbar's
 * status and the list agree. No board is marked: every board of a device
 * is a candidate. Devices with none are left out.
 */
export function deviceGroups(devices: ConnectedDevice[], selected: string | null): DeviceGroup[] {
  return devices
    .map((d) => {
      const listed = d.boards.filter((b) => b.suggested || b.board === selected);
      return {
        key: d.id,
        label: `Connected: ${d.serial ? `${d.name} (${d.serial})` : d.name}`,
        boards: listed.map((b) => b.board),
      };
    })
    .filter((g) => g.boards.length > 0);
}
