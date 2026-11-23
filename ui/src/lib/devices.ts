import { type Consent, type Devices, type MatchLevel, type TaskOutcome, type UnreachableDevice, type UsbDevice } from "./backend";

// The helpers that turn Logix Engine's device lists into what Logix IDE
// shows. The engine words and orders them (`status`, `note`, `rank`,
// `likely_boards`, `other_boards`, the driver texts); these only lay them out.

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

/**
 * The boards a device may be, as Tools > Connected Devices lists them, in
 * two collapsible groups, both Logix Engine's (for the board the scan
 * named): `matches`, the boards it most likely is (`likely_boards`); and
 * `others`, the other boards worth naming (`other_boards`: with a strong
 * match, only the selected board when the device fits it, since dozens of
 * boards sharing a USB chip would only be noise, the user's decision).
 */
export function deviceBoardGroups(d: UsbDevice): { matches: string[]; others: string[] } {
  return { matches: d.likelyBoards, others: d.otherBoards };
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

/**
 * Every device in one list, in Logix Engine's order (`rank`): the likely
 * FPGA boards first (the user's request, 2026-11-14: a board with no
 * driver yet was listed last, after the keyboard and mouse), those that may
 * be the selected board, then those that may be other boards, then the
 * rest.
 */
export function orderedDevices(devices: Devices): ListedDevice[] {
  const all: ListedDevice[] = [
    ...devices.usb.map((device) => ({ kind: "usb" as const, device })),
    ...devices.unreachable.map((device) => ({ kind: "unreachable" as const, device })),
  ];
  return all.sort((a, b) => a.device.rank - b.device.rank);
}

/**
 * What Connected Devices says about a device it can't open yet (its names
 * can't be read): Logix Engine's `note` ("This may be your Alhambra II: it
 * has the same USB IDs. …"), saying which board that is in Logix IDE when
 * it's the one selected in the toolbar.
 */
export function unreachableNote(
  u: UnreachableDevice,
  selected: string | null,
  boardName: (id: string) => string,
): string {
  if (selected === null || !u.boards.some((b) => b.board === selected)) return u.note;
  const yours = `your ${boardName(selected)}`;
  return u.note.replace(yours, `${yours} (the board selected in the toolbar)`);
}

/** What a device that can't be opened lacks, after its name: Logix Engine's tag ("no udev rule yet", "no driver yet"). */
export function lockedTag(u: UnreachableDevice): string {
  return u.driver?.tag ?? "can't be opened";
}

/**
 * Tools > Connected Devices as plain text, for a problem report (its Copy
 * button; the user's request, 2026-11-14: parts such as a serial number
 * were hard to copy): every device in the dialog's order, with its ID,
 * serial number, driver and boards, then the last driver change and its
 * log.
 */
/** What else a device reports, a line each, for the dialog and Copy: its
 * serial number, USB release and serial ports, so a pasted Copy says what
 * a bug report or a board request to Logix Catalog needs (Milestone 6,
 * the user's decision, 2026-11-22). */
export function deviceDetails(d: UsbDevice): string[] {
  const out: string[] = [];
  if (d.serialNumber) out.push(`Serial: ${d.serialNumber}`);
  if (d.release) out.push(`USB release: ${d.release}`);
  for (const p of d.serialPorts ?? []) {
    const where = p.interface === null ? "" : ` (USB interface ${p.interface})`;
    out.push(`Serial port: ${p.port}${where}${p.description ? `, ${p.description}` : ""}`);
  }
  return out;
}

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
  const listed = orderedDevices(devices);
  if (listed.length === 0) out.push("No USB devices.");
  for (const [i, item] of listed.entries()) {
    const d = item.device;
    // A blank line between devices (the user's request, 2026-11-14).
    if (i > 0) out.push("");
    out.push(item.kind === "usb" ? d.label : `${d.label}, ${lockedTag(item.device)}`);
    out.push(`  ID: ${d.id}`);
    if (item.kind === "usb") out.push(...deviceDetails(item.device).map((l) => `  ${l}`));
    if (d.driver) out.push(`  ${d.driver.state}`);
    if (item.kind === "usb") {
      const { matches, others } = deviceBoardGroups(item.device);
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
