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
    const offer = driverOfferText("Alhambra II", 0, "ftdibus");
    expect(offer).toContain("Alhambra II needs the WinUSB driver on its USB interface 0. It has FTDI's driver there now.");
    expect(offer).toContain("Logix Engine");
    expect(offer).not.toMatch(/\bLogix\b(?! (IDE|Engine))/);
    expect(sameIdsText("Another device (0403:6010) is connected.")).toContain("they get the WinUSB driver too");
    expect(driverRemoveText("Alhambra II")).toContain("every device with the same USB IDs");
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
      unreachable: [{ id: "usb:1-3:0403:6014:", driver: null, label: "USB device (0403:6014)", vid: "0403", pid: "6014", action: "fix_device_permissions", actionDescription: "Allow it." }],
    });
    expect(none[0]).toBe("No USB devices are connected.");
    expect(none[1]).toBe("  • USB device (0403:6014) can't be opened. Allow it.");
  });
});
