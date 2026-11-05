import type { Devices, TaskOutcome, UsbDevice } from "./backend";

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

/**
 * Logix Engine's confidence (1 to 3, higher is surer) in words. Logix only
 * compares the number; the words just say "how sure".
 */
export function confidenceLabel(confidence: number | null): string {
  if (confidence === null) return "";
  if (confidence >= 3) return "Matches the board's name";
  if (confidence >= 2) return "Likely";
  return "Possible";
}

/**
 * The boards a device most likely is: those with the best confidence,
 * when it's 2 or more (a name match). Chip-only matches fit dozens of
 * boards, so they aren't worth naming.
 */
export function likelyBoards(d: UsbDevice): string[] {
  const best = Math.max(0, ...d.boards.map((b) => b.confidence));
  if (best < 2) return [];
  return d.boards.filter((b) => b.confidence === best).map((b) => b.board);
}

/** Whether a device may be the given board, at any confidence. */
export function mayBeBoard(d: UsbDevice, board: string | null): boolean {
  return board !== null && d.boards.some((b) => b.board === board);
}

/** A piece of a device's boards line: text, or a board to show as a link. */
export type LinePart = string | { board: string };

/**
 * One line on which boards a device may be, from the selected board's point
 * of view (null when there's nothing worth saying): the boards it looks like
 * by name, with the selected board first when it's one of them, or a note
 * that it may be the selected board when only the USB chip fits. Boards
 * other than the selected one are `{ board }` parts, which the Connected
 * Devices dialog shows as links that select the board.
 */
export function deviceBoardsParts(
  d: UsbDevice,
  selected: string | null,
  boardName: (id: string) => string,
): LinePart[] | null {
  const likely = likelyBoards(d);
  const link = (id: string): LinePart => ({ board: id });
  // "this board: A" or "one of these boards: A, B or C".
  const boards = (ids: string[]): LinePart[] => {
    if (ids.length === 1) return ["this board: ", link(ids[0])];
    const out: LinePart[] = ["one of these boards: "];
    ids.forEach((id, i) => {
      if (i > 0) out.push(i === ids.length - 1 ? " or " : ", ");
      out.push(link(id));
    });
    return out;
  };
  if (selected && likely.includes(selected)) {
    const others = likely.filter((b) => b !== selected);
    const or: LinePart[] = others.length > 0 ? [", or ", ...boards(others)] : [];
    return [`Looks like the selected board, ${boardName(selected)}`, ...or, "."];
  }
  const fits = mayBeBoard(d, selected);
  if (likely.length > 0) {
    return ["Looks like ", ...boards(likely), fits ? ". It may also be the selected board." : "."];
  }
  if (fits && selected) return [`May be the selected board, ${boardName(selected)}.`];
  return null;
}

/** The same line as plain text. */
export function deviceBoardsLine(
  d: UsbDevice,
  selected: string | null,
  boardName: (id: string) => string,
): string | null {
  const parts = deviceBoardsParts(d, selected, boardName);
  return parts && parts.map((p) => (typeof p === "string" ? p : boardName(p.board))).join("");
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
