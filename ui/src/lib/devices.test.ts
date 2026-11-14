import { describe, expect, it } from "vitest";
import {
  matchLabel,
  deviceBoardGroups,
  deviceSummaryLines,
  isBoardNotFound,
  mayBeBoard,
  needsDeviceChoice,
  needsDriver,
  driverStateText,
  driverOfferText,
  driverRemoveText,
  sameIdsText,
  orderedDevices,
  unreachableNote,
} from "./devices";
import type { TaskOutcome, UsbDevice } from "./backend";

const alhambra: UsbDevice = {
  id: "usb:1-1:0403:6010:FTATANKC",
  label: "Alhambra II v1.0A (AlhambraBits, 0403:6010)",
  vid: "0403",
  pid: "6010",
  manufacturer: "AlhambraBits",
  product: "Alhambra II v1.0A",
  serialNumber: "FTATANKC",
  boards: [
    { board: "alhambra-ii", level: "strong" as const },
    { board: "alchitry-au", level: "weak" as const },
  ],
  likelyBoards: ["alhambra-ii"],
  buildMatch: null,
  driver: null,
};

const outcome = (action: string | null, devices: UsbDevice[] = []): TaskOutcome => ({
  ok: false, diagnostics: [], testbenches: [], report: null,
  cancelled: false, seconds: 1, action, devices, driverDevice: null, sameIds: null,
});

describe("devices", () => {
  it("recognizes a board-not-found upload error", () => {
    expect(isBoardNotFound(outcome("connect_board"))).toBe(true);
    expect(isBoardNotFound(outcome(null))).toBe(false);
  });

  it("asks for a device only when several fit", () => {
    expect(needsDeviceChoice(outcome("select_device", [alhambra, alhambra]))).toBe(true);
    expect(needsDeviceChoice(outcome("select_device", [alhambra]))).toBe(false);
    expect(needsDeviceChoice(outcome("connect_board"))).toBe(false);
  });

  it("stops Upload for a driver only with the device that needs it", () => {
    const needs = { id: "usb:1-2:0403:6010:", label: "Alhambra II", interface: 0, current: "ftdibus" };
    expect(needsDriver({ ...outcome("install_drivers"), driverDevice: needs })).toBe(true);
    expect(needsDriver({ ...outcome("disconnect_devices"), driverDevice: needs, sameIds: "x" })).toBe(true);
    expect(needsDriver(outcome("install_drivers"))).toBe(false);
    expect(needsDriver({ ...outcome("connect_board"), driverDevice: needs })).toBe(false);
  });

  it("says what a driver looks like and what installing or removing it does", () => {
    expect(driverStateText({ needed: "winusb", interface: 0, current: "winusb", ready: true })).toBe(
      "Driver: WinUSB, ready to program the board.",
    );
    expect(driverStateText({ needed: "winusb", interface: 0, current: "ftdibus", ready: false })).toBe(
      "Driver: needs WinUSB on USB interface 0 to program the board (it has FTDI's driver now).",
    );
    expect(driverStateText({ needed: "winusb", interface: 2, current: null, ready: false })).toContain(
      "interface 2 to program the board (it has no driver now)",
    );
    const offer = driverOfferText("the Alhambra II", "ftdibus");
    expect(offer).toContain("Logix IDE programs the Alhambra II through the WinUSB driver. Windows uses FTDI's driver for it now");
    expect(offer).toContain("Windows asks whether Logix Engine may make changes to your device. Choose Yes.");
    expect(offer).toContain("a minute or two");
    expect(offer).toContain("such as a serial port, keep working");
    expect(offer).not.toContain("interface");
    expect(offer).not.toMatch(/\bLogix\b(?! (IDE|Engine))/);
    expect(driverOfferText("the Alhambra II", null)).not.toContain("Windows uses");
    expect(sameIdsText("Another device (0403:6010) is connected.")).toContain("they get the WinUSB driver too");
    const remove = driverRemoveText("the Alhambra II");
    expect(remove).toContain("removes the WinUSB driver Logix IDE installed for the Alhambra II");
    expect(remove).toContain("the board maker's own tools");
    expect(remove).toContain("same USB IDs");
    expect(remove).not.toMatch(/\bLogix\b(?! (IDE|Engine))/);
  });

  it("lists the likely FPGA boards first, the selected board's first of all", () => {
    const mouse = { ...alhambra, id: "mouse", label: "USB Mouse", boards: [], likelyBoards: [] };
    const other = { ...alhambra, id: "other", boards: [{ board: "icestick", level: "weak" as const }] };
    const locked = {
      id: "locked", label: "USB device (0403:6010)", vid: "0403", pid: "6010",
      action: "install_drivers", actionDescription: "Install the driver.",
      driver: { needed: "winusb", interface: 0, current: "ftdibus", ready: false },
      boards: [{ board: "alhambra-ii", level: "weak" as const }, { board: "alchitry-au", level: "weak" as const }],
    };
    const ids = (selected: string | null) =>
      orderedDevices({ usb: [mouse, other], unreachable: [locked] }, selected).map((d) => d.device.id);
    expect(ids("alhambra-ii")).toEqual(["locked", "other", "mouse"]);
    expect(ids("icestick")).toEqual(["other", "locked", "mouse"]);
    expect(ids(null)).toEqual(["other", "locked", "mouse"]);
    const name = (id: string) => ({ "alhambra-ii": "Alhambra II", "alchitry-au": "Alchitry Au" })[id] ?? id;
    expect(unreachableNote(locked, "alhambra-ii", name)).toBe(
      "This may be your Alhambra II (the board selected in the toolbar): it has the same USB IDs. Install its driver so Logix IDE can read its name and program it.",
    );
    expect(unreachableNote(locked, "icestick", name)).toContain("This may be an FPGA board: 2 boards have its USB IDs.");
    expect(unreachableNote({ ...locked, boards: [locked.boards[0]] }, null, name)).toContain("the Alhambra II: it has its USB IDs");
    // Elsewhere (Linux permissions), the engine's fix.
    expect(unreachableNote({ ...locked, driver: null, actionDescription: "Fix the permissions." }, null, name)).toContain("Fix the permissions.");
  });

  it("tells whether a device may be a board", () => {
    const generic = { ...alhambra, boards: [{ board: "alchitry-au", level: "weak" as const }] };
    expect(mayBeBoard(generic, "alchitry-au")).toBe(true);
    expect(mayBeBoard(generic, "icestick")).toBe(false);
    expect(mayBeBoard(generic, null)).toBe(false);
  });

  it("splits a device's boards into the engine's likely boards and others", () => {
    const d = {
      ...alhambra,
      likelyBoards: ["arty", "basys3"],
      boards: [
        { board: "arty", level: "possible" as const },
        { board: "basys3", level: "possible" as const },
        { board: "generic-a", level: "weak" as const },
        { board: "generic-b", level: "weak" as const },
      ],
    };
    expect(deviceBoardGroups(d, null)).toEqual({
      matches: ["arty", "basys3"],
      others: ["generic-a", "generic-b"],
    });
    // With a strong match, the weaker ones are left out, except the
    // selected board when the device fits it.
    const strong = {
      ...d,
      likelyBoards: ["arty"],
      boards: [{ board: "arty", level: "strong" as const }, ...d.boards.slice(1)],
    };
    expect(deviceBoardGroups(strong, null)).toEqual({ matches: ["arty"], others: [] });
    expect(deviceBoardGroups(strong, "generic-b")).toEqual({ matches: ["arty"], others: ["generic-b"] });
    expect(deviceBoardGroups(strong, "alhambra-ii")).toEqual({ matches: ["arty"], others: [] });
    // Only weak matches: no likely board, all others.
    const generic = { ...alhambra, likelyBoards: [], boards: [{ board: "generic-a", level: "weak" as const }] };
    expect(deviceBoardGroups(generic, null)).toEqual({ matches: [], others: ["generic-a"] });
    expect(deviceBoardGroups({ ...alhambra, likelyBoards: [], boards: [] }, null)).toEqual({ matches: [], others: [] });
  });

  it("puts match levels in words", () => {
    expect(matchLabel("strong")).toBe("Strong match");
    expect(matchLabel("possible")).toBe("Possible match");
    expect(matchLabel("weak")).toBe("Weak match");
    expect(matchLabel(null)).toBe("");
  });

  it("lists connected devices or gives tips", () => {
    expect(deviceSummaryLines({ usb: [alhambra], unreachable: [] })[1]).toBe(
      "  • Alhambra II v1.0A (AlhambraBits, 0403:6010)",
    );
    const none = deviceSummaryLines({
      usb: [],
      unreachable: [{ id: "usb:1-3:0403:6014:", driver: null, label: "USB device (0403:6014)", vid: "0403", pid: "6014", action: "fix_device_permissions", actionDescription: "Allow it.", boards: [] }],
    });
    expect(none[0]).toBe("No USB devices are connected.");
    expect(none[1]).toBe("  • USB device (0403:6014) can't be opened. Allow it.");
  });
});
