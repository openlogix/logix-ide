import type { Devices, Driver, MatchLevel, TaskOutcome, UnreachableDevice, UsbDevice } from "./backend";

/** True if Upload failed because the board wasn't found on USB. */
export function isBoardNotFound(outcome: TaskOutcome): boolean {
  return outcome.action === "connect_board";
}

/** True if Upload found several devices that fit and the user must pick one. */
export function needsDeviceChoice(outcome: TaskOutcome): boolean {
  return outcome.action === "select_device" && outcome.devices.length > 1;
}

/**
 * True if Upload stopped because the board needs a driver first (Windows):
 * the user agrees to install it ("install_drivers"), or to install it while
 * other devices with the same USB IDs are connected ("disconnect_devices").
 */
export function needsDriver(outcome: TaskOutcome): boolean {
  return (
    outcome.driverDevice !== null &&
    (outcome.action === "install_drivers" || outcome.action === "disconnect_devices")
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

/** Console lines listing what's connected, with tips when nothing is. */
/** What the driver a device's board needs looks like now (Windows), for Connected Devices. */
export function driverStateText(d: Driver): string {
  if (d.ready) return `Driver: ${driverName(d.current ?? d.needed)}, ready to program the board.`;
  const now = d.current ? `it has ${driverName(d.current)} now` : "it has no driver now";
  return `Driver: needs WinUSB on USB interface ${d.interface} to program the board (${now}).`;
}

/** A driver's Windows service name for people. */
function driverName(service: string): string {
  return (
    { winusb: "WinUSB", libusbk: "libusbK", libusb0: "libusb0", ftdibus: "FTDI's driver", usbser: "the serial port driver" }[
      service
    ] ?? service
  );
}

/**
 * The question before installing WinUSB for a board (Upload or Connected
 * Devices). `board` names it for people: the board's name when known, such
 * as "the Alhambra II", else the device's label.
 */
export function driverOfferText(board: string, current: string | null): string {
  const now = current ? ` Windows uses ${driverName(current)} for it now, which Logix IDE can't program the board with.` : "";
  return [
    `Logix IDE programs ${board} through the WinUSB driver.${now}`,
    "What happens:",
    "• Windows asks whether Logix Engine may make changes to your device. Choose Yes.",
    "• Installing takes a minute or two.",
    "• Only the part of the board that programs the FPGA gets WinUSB. Its other parts, such as a serial port, keep working as before.",
    "• Other connected devices of the same kind (with the same USB IDs) get WinUSB too.",
    "You can undo it later in Tools > Connected Devices (Remove Driver…).",
  ].join("\n\n");
}

/** The question when other connected devices have the same USB IDs (Logix Engine's message names them). */
export function sameIdsText(engineMessage: string): string {
  return `${engineMessage}\n\nUnplug them and try again, or continue: they get the WinUSB driver too.`;
}

/** The question before removing WinUSB from a board. `board` as for `driverOfferText`. */
export function driverRemoveText(board: string): string {
  return [
    `This removes the WinUSB driver Logix IDE installed for ${board}. Windows then uses the driver it had before (such as FTDI's), so the board maker's own tools can use the board again.`,
    "What happens:",
    "• Windows asks whether Logix Engine may make changes to your device. Choose Yes.",
    "• Other connected devices of the same kind (with the same USB IDs) go back to their driver too.",
    "• Logix IDE can't program the board until WinUSB is installed again. Upload offers to install it when it's needed.",
  ].join("\n\n");
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
  result: { text: string; kind: "ok" | "error" } | null;
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
  const fix = u.driver && !u.driver.ready
    ? "Install its driver so Logix IDE can read its name and program it."
    : u.actionDescription;
  if (selected !== null && boards.includes(selected))
    return `This may be your ${boardName(selected)} (the board selected in the toolbar): it has the same USB IDs. ${fix}`;
  if (boards.length === 1)
    return `This may be an FPGA board, the ${boardName(boards[0])}: it has its USB IDs. ${fix}`;
  return `This may be an FPGA board: ${boards.length} boards have its USB IDs. ${fix}`;
}

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
