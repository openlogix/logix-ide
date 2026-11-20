import { type Consent, type Devices, type MatchLevel, type TaskOutcome, type UnreachableDevice, type UsbDevice } from "./backend";

/** True if Upload failed because the board wasn't found on USB. */
export function isBoardNotFound(outcome: TaskOutcome): boolean {
  return outcome.action === "connect_board";
}

/** True if Upload found several devices that fit and the user must pick one. */
export function needsDeviceChoice(outcome: TaskOutcome): boolean {
  return outcome.action === "select_device" && outcome.devices.length > 1;
}

/** Logix Engine's action code when other devices have the same USB IDs and would get the driver too. */
export const CONFIRM_SAME_IDS = "confirm_same_ids";

/**
 * True if Upload stopped because the board needs a driver (Windows) or a
 * udev rule (Linux) first:
 * the user agrees to install it ("install_drivers"), or to install it while
 * other devices with the same USB IDs are connected ("confirm_same_ids").
 */
export function needsDriver(outcome: TaskOutcome): boolean {
  return (
    outcome.driverDevice !== null &&
    (outcome.action === "install_drivers" || outcome.action === CONFIRM_SAME_IDS)
  );
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

/**
 * A question shown in Logix IDE's own confirmation dialog (`ConfirmDialog`):
 * a title, a lead sentence, what happens as a short list, an optional note,
 * and the button that agrees (Cancel is always there).
 */
export interface ConfirmContent {
  title: string;
  lead: string;
  steps: string[];
  note: string | null;
  okLabel: string;
  /** The other button's label: Cancel when not given. */
  cancelLabel?: string;
}

/** Where to undo a driver or udev rule change in Logix IDE, after Logix Engine's own note. */
export const UNDO_WHERE = "It's in Tools > Connected Devices.";

/**
 * One of Logix Engine's questions (a driver's `add` or `remove`, or the
 * same-USB-IDs refusal's `consent`), as Logix IDE's dialog shows it: the
 * engine's words (the same in every frontend, naming none, moved there
 * from Logix IDE, 2026-11-18), with `where` after its note, such as where
 * to undo it.
 */
export function confirmContent(c: Consent, where?: string): ConfirmContent {
  const note = [c.note, c.note ? where : undefined].filter(Boolean).join(" ");
  return { title: c.title, lead: c.lead, steps: c.steps, note: note || null, okLabel: c.confirm };
}

/**
 * A driver install or removal running (or just ended) in Tools > Connected
 * Devices: what it does, Logix Engine's phase, its output lines, and how it
 * ended.
 */
export interface DriverWork {
  title: string;
  step: string | null;
  lines: string[];
  /** How it ended: done ("ok"), failed or cancelled by the user ("error", red; `cancelled` then, nothing changed). */
  result: { text: string; kind: "ok" | "error"; cancelled?: boolean } | null;
  /** When it started (ms since the epoch), for how long it took. */
  started: number;
}

/**
 * How long something took, for people: "48 s" under a minute, else
 * "1 min 12 s" (a driver change takes a minute or two, mostly Windows).
 */
export function durationText(ms: number): string {
  const s = Math.max(0, Math.round(ms / 1000));
  if (s < 60) return `${s} s`;
  return s % 60 === 0 ? `${s / 60} min` : `${Math.floor(s / 60)} min ${s % 60} s`;
}

/**
 * The log's last lines when a driver change ends: after a success, how long
 * it took, then each sentence of the result on a line of its own (the
 * user's requests, 2026-11-15); after a failure, the error.
 */
export function driverWorkEnd(lines: string[], kind: "ok" | "error", ms: number): string[] {
  return kind === "ok" ? [`Done in ${durationText(ms)}.`, ...lines] : [`Failed: ${lines.join(" ")}`];
}

/**
 * A device in Tools > Connected Devices: one that can be opened (`usb`), or
 * one that can't be until it has its driver or permissions
 * (`unreachable`, whose USB IDs fit boards).
 */
export type ListedDevice =
  | { kind: "usb"; device: UsbDevice }
  | { kind: "unreachable"; device: UnreachableDevice };

/** The boards a listed device may be, by ID. */
function listedBoards(d: ListedDevice): string[] {
  return d.device.boards.map((b) => b.board);
}

/**
 * Every device in one list, the likely FPGA boards first (the user's
 * request, 2026-11-14: a board with no driver yet was listed last, after
 * the keyboard and mouse): first those that may be the selected board, then
 * those that may be other boards, then the rest. The engine's order is
 * kept within each group.
 */
export function orderedDevices(devices: Devices, selected: string | null): ListedDevice[] {
  const all: ListedDevice[] = [
    ...devices.usb.map((device) => ({ kind: "usb" as const, device })),
    ...devices.unreachable.map((device) => ({ kind: "unreachable" as const, device })),
  ];
  const rank = (d: ListedDevice) => {
    const boards = listedBoards(d);
    if (selected !== null && boards.includes(selected)) return 0;
    return boards.length > 0 ? 1 : 2;
  };
  return all
    .map((d, i) => ({ d, i }))
    .sort((a, b) => rank(a.d) - rank(b.d) || a.i - b.i)
    .map(({ d }) => d);
}

/**
 * What Connected Devices says about a device it can't open yet (its names
 * can't be read): that it may be an FPGA board, the selected one when its
 * USB IDs fit, and what to do.
 */
export function unreachableNote(
  u: UnreachableDevice,
  selected: string | null,
  boardName: (id: string) => string,
): string {
  const boards = u.boards.map((b) => b.board);
  const fix = u.fix;
  if (selected !== null && boards.includes(selected))
    return `This may be your ${boardName(selected)} (the board selected in the toolbar): it has the same USB IDs. ${fix}`;
  if (boards.length === 1)
    return `This may be an FPGA board, the ${boardName(boards[0])}: it has its USB IDs. ${fix}`;
  return `This may be an FPGA board: ${boards.length} boards have its USB IDs. ${fix}`;
}

/** What a device that can't be opened lacks, after its name: Logix Engine's tag ("no udev rule yet", "no driver yet"). */
export function lockedTag(u: UnreachableDevice): string {
  return u.driver?.tag ?? "can't be opened";
}

/** Console lines listing what's connected, with tips when nothing is. */
export function deviceSummaryLines(devices: Devices): string[] {
  const locked = devices.unreachable.map((u) => `  • ${u.label} can't be opened. ${u.fix}`);
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

/**
 * Tools > Connected Devices as plain text, for a problem report (its Copy
 * button; the user's request, 2026-11-14: parts such as a serial number
 * were hard to copy): every device in the dialog's order, with its ID,
 * serial number, driver and boards, then the last driver change and its
 * log.
 */
export function devicesReport(
  devices: Devices,
  selected: string | null,
  boardName: (id: string) => string,
  work: DriverWork | null,
): string {
  const names = (ids: string[]) => ids.map(boardName).join(", ");
  const out = [
    "Connected Devices",
    `Selected board: ${selected === null ? "none" : `${boardName(selected)} (${selected})`}`,
    "",
  ];
  const listed = orderedDevices(devices, selected);
  if (listed.length === 0) out.push("No USB devices.");
  for (const [i, item] of listed.entries()) {
    const d = item.device;
    // A blank line between devices (the user's request, 2026-11-14).
    if (i > 0) out.push("");
    out.push(item.kind === "usb" ? d.label : `${d.label}, ${lockedTag(item.device)}`);
    out.push(`  ID: ${d.id}`);
    if (item.kind === "usb" && item.device.serialNumber) out.push(`  Serial: ${item.device.serialNumber}`);
    if (d.driver) out.push(`  ${d.driver.state}`);
    if (item.kind === "usb") {
      const { matches, others } = deviceBoardGroups(item.device, selected);
      if (matches.length > 0) out.push(`  Matching boards: ${names(matches)}`);
      if (others.length > 0) out.push(`  Other possible boards: ${names(others)}`);
      if (matches.length + others.length === 0) out.push("  Not a board Logix IDE knows.");
    } else {
      out.push(`  ${item.device.fix}`);
      const ids = item.device.boards.map((b) => b.board);
      if (ids.length > 0) out.push(`  Boards with these USB IDs (${ids.length}): ${names(ids)}`);
    }
  }
  if (work) {
    out.push("", `Last driver change: ${work.title}`);
    const ended = work.result
      ? work.result.cancelled
        ? work.result.text
        : `${work.result.kind === "ok" ? "Done" : "Failed"}: ${work.result.text}`
      : `Running${work.step ? `: ${work.step}` : ""}`;
    out.push(`  ${ended}`);
    out.push(...work.lines.map((l) => `  | ${l}`));
  }
  return out.join("\n");
}
