import type { Devices, Driver, MatchLevel, TaskOutcome, UsbDevice } from "./backend";

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

/** The question before installing WinUSB for a device (Upload or Connected Devices). */
export function driverOfferText(label: string, iface: number, current: string | null): string {
  const now = current ? ` It has ${driverName(current)} there now.` : "";
  return [
    `To program the board, ${label} needs the WinUSB driver on its USB interface ${iface}.${now} Its other interfaces, such as a serial port, keep their drivers.`,
    "Windows will ask whether Logix Engine may make changes to this device: choose Yes. Other devices with the same USB IDs use the same driver. Tools > Connected Devices can remove it later.",
  ].join("\n\n");
}

/** The question when other connected devices have the same USB IDs (Logix Engine's message names them). */
export function sameIdsText(engineMessage: string): string {
  return `${engineMessage}\n\nUnplug them and try again, or continue: they get the WinUSB driver too.`;
}

/** The question before removing WinUSB from a device's board. */
export function driverRemoveText(label: string): string {
  return [
    `Windows gives back its own driver (such as FTDI's) to ${label} and to every device with the same USB IDs, so other programs, such as the board maker's tools, can use it again.`,
    "Logix IDE can't program the board until WinUSB is installed again: Upload offers it. Windows will ask whether Logix Engine may make changes: choose Yes.",
  ].join("\n\n");
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
