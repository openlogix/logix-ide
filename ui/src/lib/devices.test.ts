import { describe, expect, it } from "vitest";
import {
  matchLabel,
  deviceBoardGroups,
  deviceSummaryLines,
  isBoardNotFound,
  needsDeviceChoice,
  needsDriver,
  confirmContent,
  orderedDevices,
  unreachableNote,
  devicesReport,
  deviceDetails,
  durationText,
  driverWorkEnd,
  lockedTag,
} from "./devices";
import type { Consent, Driver, TaskOutcome, UsbDevice } from "./backend";

/** A driver as Logix Engine sends it, with its texts (`over` replaces fields). */
const driver = (over: Partial<Driver> = {}): Driver => ({
  needed: "winusb", interface: 0, current: "ftdibus", ready: false,
  label: "the WinUSB driver",
  state: "Driver: needs WinUSB on USB interface 0 to program the board (it has FTDI's driver now).",
  tag: "no driver yet", canAdd: true, canRemove: false, add: null, remove: null,
  ...over,
});

const alhambra: UsbDevice = {
  id: "usb:1-1:0403:6010:FTATANKC",
  label: "Alhambra II v1.0A (AlhambraBits, 0403:6010)",
  vid: "0403",
  pid: "6010",
  manufacturer: "AlhambraBits",
  product: "Alhambra II v1.0A",
  serialNumber: "FTATANKC",
  release: "0700",
  boards: [
    { board: "alhambra-ii", level: "strong" as const },
    { board: "alchitry-au", level: "weak" as const },
  ],
  serialPorts: [],
  serialPort: null,
  likelyBoards: ["alhambra-ii"],
  buildMatch: null,
  who: "the Alhambra II",
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
    const needs = { id: "usb:1-2:0403:6010:", label: "Alhambra II", who: "the Alhambra II", driver: driver() };
    const same: Consent = { title: "T", lead: "L", steps: [], note: null, confirm: "Continue for All", busy: "B", cancelled: "C" };
    expect(needsDriver({ ...outcome("install_drivers"), driverDevice: needs })).toBe(true);
    expect(needsDriver({ ...outcome("confirm_same_ids"), driverDevice: needs, sameIds: same })).toBe(true);
    // program_board's own disconnect_devices isn't a driver offer.
    expect(needsDriver({ ...outcome("disconnect_devices"), driverDevice: needs })).toBe(false);
    expect(needsDriver(outcome("install_drivers"))).toBe(false);
    expect(needsDriver({ ...outcome("connect_board"), driverDevice: needs })).toBe(false);
  });

  // The questions are Logix Engine's (its tests check their words); Logix
  // IDE shows them as they are, adding where to undo the change.
  it("shows Logix Engine's questions, with where to undo them", () => {
    const ask: Consent = {
      title: "Let your user account program the board?",
      lead: "To program the iCESugar v1.5, Linux needs a udev rule that lets your user account use it.",
      steps: ["Linux asks for your password. Enter it to allow the change."],
      note: "You can remove the rule later.",
      confirm: "Add udev Rule", busy: "Adding the board's udev rule…", cancelled: "Cancelled: the udev rule wasn't added.",
    };
    expect(confirmContent(ask, "It's in Tools > Connected Devices.")).toEqual({
      title: ask.title, lead: ask.lead, steps: ask.steps,
      note: "You can remove the rule later. It's in Tools > Connected Devices.",
      okLabel: "Add udev Rule",
    });
    // No note of its own: none added.
    expect(confirmContent({ ...ask, note: null }, "It's in Tools > Connected Devices.").note).toBeNull();
  });

  it("names a locked device by Logix Engine's tag and fix", () => {
    const locked = {
      id: "usb:1-2:1d50:602b:", label: "USB device (1d50:602b)", vid: "1d50", pid: "602b",
      action: "fix_device_permissions", who: "the iCESugar v1.5",
      fix: "Add a udev rule for it, so it can be programmed and its name read.",
      driver: driver({ needed: "udev-rule", interface: null, current: null, tag: "no udev rule yet" }),
      boards: [{ board: "icesugar-1-5", level: "weak" as const }],
    };
    expect(lockedTag(locked)).toBe("no udev rule yet");
    expect(lockedTag({ ...locked, driver: null })).toBe("can't be opened");
    expect(unreachableNote(locked, null, () => "iCESugar v1.5")).toContain(locked.fix);
    expect(deviceSummaryLines({ usb: [], unreachable: [locked] })[1]).toBe(
      `  • USB device (1d50:602b) can't be opened. ${locked.fix}`,
    );
  });

  it("says how long a driver change took", () => {
    expect(durationText(400)).toBe("0 s");
    expect(durationText(48_400)).toBe("48 s");
    expect(durationText(60_000)).toBe("1 min");
    expect(durationText(72_300)).toBe("1 min 12 s");
    expect(driverWorkEnd(["Removed.", "Windows uses its original driver again."], "ok", 72_300)).toEqual([
      "Done in 1 min 12 s.",
      "Removed.",
      "Windows uses its original driver again.",
    ]);
    expect(driverWorkEnd(["Couldn't install."], "error", 5_000)).toEqual(["Failed: Couldn't install."]);
  });

  it("copies Connected Devices as text for a problem report", () => {
    const locked = {
      id: "usb:1-2:0403:6010", label: "USB device (0403:6010)", vid: "0403", pid: "6010",
      action: "install_drivers", who: "USB device (0403:6010)", fix: "It needs its driver first, so it can be programmed.",
      driver: driver(),
      boards: [{ board: "alhambra-ii", level: "weak" as const }, { board: "tangnano9k", level: "weak" as const }],
    };
    const name = (id: string) => ({ "alhambra-ii": "Alhambra II", tangnano9k: "Sipeed Tang Nano 9k" })[id] ?? id;
    const text = devicesReport({ usb: [alhambra], unreachable: [locked] }, "tangnano9k", name, {
      title: "Installing the WinUSB driver…", step: null, lines: ["Permission given."], started: 0,
      result: { text: "Installed.", kind: "ok" },
    });
    expect(text.split("\n")).toEqual([
      "Connected Devices",
      "Selected board: Sipeed Tang Nano 9k (tangnano9k)",
      "",
      "USB device (0403:6010), no driver yet",
      "  ID: usb:1-2:0403:6010",
      "  Driver: needs WinUSB on USB interface 0 to program the board (it has FTDI's driver now).",
      "  It needs its driver first, so it can be programmed.",
      "  Boards with these USB IDs (2): Alhambra II, Sipeed Tang Nano 9k",
      "",
      "Alhambra II v1.0A (AlhambraBits, 0403:6010)",
      "  ID: usb:1-1:0403:6010:FTATANKC",
      "  Serial: FTATANKC",
      "  USB release: 0700",
      "  Matching boards: Alhambra II",
      "",
      "Last driver change: Installing the WinUSB driver…",
      "  Done: Installed.",
      "  | Permission given.",
    ]);
    expect(devicesReport({ usb: [], unreachable: [] }, null, name, null)).toContain("No USB devices.");
  });

  it("lists the likely FPGA boards first, the selected board's first of all", () => {
    const mouse = { ...alhambra, id: "mouse", label: "USB Mouse", boards: [], likelyBoards: [] };
    const other = { ...alhambra, id: "other", boards: [{ board: "icestick", level: "weak" as const }] };
    const locked = {
      id: "locked", label: "USB device (0403:6010)", vid: "0403", pid: "6010",
      action: "install_drivers", who: "USB device (0403:6010)", fix: "It needs its driver first, so it can be programmed.",
      driver: driver(),
      boards: [{ board: "alhambra-ii", level: "weak" as const }, { board: "alchitry-au", level: "weak" as const }],
    };
    const ids = (selected: string | null) =>
      orderedDevices({ usb: [mouse, other], unreachable: [locked] }, selected).map((d) => d.device.id);
    expect(ids("alhambra-ii")).toEqual(["locked", "other", "mouse"]);
    expect(ids("icestick")).toEqual(["other", "locked", "mouse"]);
    expect(ids(null)).toEqual(["other", "locked", "mouse"]);
    const name = (id: string) => ({ "alhambra-ii": "Alhambra II", "alchitry-au": "Alchitry Au" })[id] ?? id;
    expect(unreachableNote(locked, "alhambra-ii", name)).toBe(
      "This may be your Alhambra II (the board selected in the toolbar): it has the same USB IDs. It needs its driver first, so it can be programmed.",
    );
    expect(unreachableNote(locked, "icestick", name)).toContain("This may be an FPGA board: 2 boards have its USB IDs.");
    expect(unreachableNote({ ...locked, boards: [locked.boards[0]] }, null, name)).toContain("the Alhambra II: it has its USB IDs");
    // Elsewhere (Linux permissions), the engine's fix too.
    expect(unreachableNote({ ...locked, driver: null, fix: "Fix the permissions." }, null, name)).toContain("Fix the permissions.");
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
      unreachable: [{ id: "usb:1-3:0403:6014:", driver: null, label: "USB device (0403:6014)", vid: "0403", pid: "6014", action: "fix_device_permissions", who: "USB device (0403:6014)", fix: "Allow it.", boards: [] }],
    });
    expect(none[0]).toBe("No USB devices are connected.");
    expect(none[1]).toBe("  • USB device (0403:6014) can't be opened. Allow it.");
  });
});

describe("deviceDetails", () => {
  it("gives the serial number, the USB release and each serial port, for Copy", () => {
    const d = {
      ...alhambra,
      serialPorts: [
        { port: "/dev/cu.usbserial-FTATANKC1", interface: 1, role: "uart" as const, description: "The Alhambra II's serial port" },
        { port: "/dev/cu.usbserial-X", interface: null, role: null, description: "" },
      ],
    };
    expect(deviceDetails(d)).toEqual([
      "Serial: FTATANKC",
      "USB release: 0700",
      "Serial port: /dev/cu.usbserial-FTATANKC1 (USB interface 1), The Alhambra II's serial port",
      "Serial port: /dev/cu.usbserial-X",
    ]);
    expect(deviceDetails({ ...d, serialNumber: "", release: "", serialPorts: [] })).toEqual([]);
  });
});
