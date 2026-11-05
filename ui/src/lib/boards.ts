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
  /** The boards it may be, strongest first (the engine's match levels). */
  boards: string[];
}

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
    b.archLabel.toLowerCase().includes(q);
  const shown = boards.filter(matches);

  const groups: BoardGroup[] = [];
  for (const d of devices) {
    const its = d.boards
      .map((id) => shown.find((b) => b.id === id))
      .filter((b): b is Board => b !== undefined);
    if (its.length > 0)
      groups.push({ key: d.key, label: d.label, boards: its, device: true });
  }
  for (const g of groupByArch(shown, (b) => b)) {
    groups.push({ key: g.label, label: g.label, boards: g.items, device: false });
  }
  return groups;
}

/**
 * The board picker's device groups from the connected devices: per device,
 * the boards it may be that aren't weak matches (weak ones fit many boards),
 * strongest first, plus `selected` when the device fits it, so the
 * toolbar's status and the list agree. No board is marked: every board of
 * a device is a candidate. Devices with none are left out.
 */
export function deviceGroups(devices: ConnectedDevice[], selected: string | null): DeviceGroup[] {
  return devices
    .map((d) => {
      const listed = d.boards.filter((b) => b.level !== "weak" || b.board === selected);
      return {
        key: d.id,
        label: `Connected: ${d.serial ? `${d.name} (${d.serial})` : d.name}`,
        boards: listed.map((b) => b.board),
      };
    })
    .filter((g) => g.boards.length > 0);
}
