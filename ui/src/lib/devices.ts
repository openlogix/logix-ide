import type { Devices, MatchLevel, TaskOutcome, UsbDevice } from "./backend";

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

/** Whether a device may be the given board, at any level. */
export function mayBeBoard(d: UsbDevice, board: string | null): boolean {
  return board !== null && d.boards.some((b) => b.board === board);
}

/**
 * The boards a device may be, as Tools > Connected Devices lists them, in
 * two collapsible groups: `matches`, the boards it most likely is (the
 * engine's `likely_boards`); and `others`, the rest. With a strong match, the weaker ones would only be noise
 * (the user's decision), so `others` then holds only the selected board,
 * when the device fits it.
 */
export function deviceBoardGroups(
  d: UsbDevice,
  selected: string | null,
): { matches: string[]; others: string[] } {
  const matches = d.likelyBoards;
  const rest = d.boards.map((b) => b.board).filter((b) => !matches.includes(b));
  const strong = d.boards.some((b) => b.level === "strong");
  const others = strong ? rest.filter((b) => b === selected) : rest;
  return { matches, others };
}

/** Console lines listing what's connected, with tips when nothing is. */
export function deviceSummaryLines(devices: Devices): string[] {
  const locked = devices.unreachable.map(
    (u) => `  • ${u.label} can't be opened. ${u.actionDescription}`,
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
    ...devices.usb.map((d) => `  • ${d.label}`),
    ...locked,
    "If your board isn't listed, check the cable and power.",
    "If it's listed, check that the board selected in the toolbar matches it.",
  ];
}
