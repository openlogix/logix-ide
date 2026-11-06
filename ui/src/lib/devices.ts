import type { Devices, MatchLevel, TaskOutcome, UsbDevice } from "./backend";

/** A device in plain words, e.g. "Alhambra II v1.0A (AlhambraBits, 0403:6010)". */
export function usbLabel(d: UsbDevice): string {
  const name = d.product || "Unknown USB device";
  const details = [d.manufacturer, `${d.vid}:${d.pid}`].filter(Boolean);
  return `${name} (${details.join(", ")})`;
}

/** True if Upload failed because the board wasn't found on USB. */
export function isBoardNotFound(outcome: TaskOutcome): boolean {
  return outcome.action === "connect_board";
}

/** True if Upload found several devices that fit and the user must pick one. */
export function needsDeviceChoice(outcome: TaskOutcome): boolean {
  return outcome.action === "select_device" && outcome.devices.length > 1;
}

/** The engine's match level in words, saying nothing about its cause. */
export function matchLabel(level: MatchLevel | null): string {
  if (level === null) return "";
  return { strong: "Strong match", possible: "Possible match", weak: "Weak match" }[level];
}

/** A level's rank, for comparing: weak < possible < strong. */
export function levelRank(level: MatchLevel): number {
  return { weak: 0, possible: 1, strong: 2 }[level];
}

/**
 * The boards a device most likely is: those at its strongest level, unless
 * that is weak (weak matches fit many boards, so they aren't named).
 */
export function likelyBoards(d: UsbDevice): string[] {
  const best = Math.max(-1, ...d.boards.map((b) => levelRank(b.level)));
  if (best <= levelRank("weak")) return [];
  return d.boards.filter((b) => levelRank(b.level) === best).map((b) => b.board);
}

/** Whether a device may be the given board, at any level. */
export function mayBeBoard(d: UsbDevice, board: string | null): boolean {
  return board !== null && d.boards.some((b) => b.board === board);
}

/**
 * The boards a device may be, as Tools > Connected Devices lists them, in
 * two collapsible groups: `matches`, the boards it most likely is (its
 * strongest matches, unless weak), in the engine's order; and `others`, the
 * weaker ones. With a strong match, the weaker ones would only be noise
 * (the user's decision), so `others` then holds only the selected board,
 * when the device fits it.
 */
export function deviceBoardGroups(
  d: UsbDevice,
  selected: string | null,
): { matches: string[]; others: string[] } {
  const matches = likelyBoards(d);
  const rest = d.boards.map((b) => b.board).filter((b) => !matches.includes(b));
  const strong = d.boards.some((b) => b.level === "strong");
  const others = strong ? rest.filter((b) => b === selected) : rest;
  return { matches, others };
}

/** Console lines listing what's connected, with tips when nothing is. */
export function deviceSummaryLines(devices: Devices): string[] {
  const locked = devices.unreachable.map(
    (u) => `  • A device (${u.vid}:${u.pid}) can't be opened. ${u.actionDescription}`,
  );
  if (devices.usb.length === 0) {
    return [
      "No USB devices are connected.",
      ...locked,
      "Tips: check that the board is plugged in and powered, and try another cable",
      "(some USB cables only charge and don't carry data).",
    ];
  }
  return [
    "Connected USB devices:",
    ...devices.usb.map((d) => `  • ${usbLabel(d)}`),
    ...locked,
    "If your board isn't listed, check the cable and power.",
    "If it's listed, check that the board selected in the toolbar matches it.",
  ];
}
