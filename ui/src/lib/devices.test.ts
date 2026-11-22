import { describe, expect, it } from "vitest";
import {
  matchLabel,
  deviceBoardGroups,
  mayBeBoard,
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
  otherBoards: [],
  rank: 0,
  buildMatch: null,
  who: "the Alhambra II",
  driver: null,
};

const outcome = (action: string | null, devices: UsbDevice[] = []): TaskOutcome => ({
  ok: false, diagnostics: [], testbenches: [], report: null,
  cancelled: false, seconds: 1, action, devices, driverDevice: null, sameIds: null, checkedFiles: [],
});

describe("devices", () => {
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
      note: "This may be an FPGA board, the iCESugar v1.5: it has its USB IDs. Add a udev rule for it, so it can be programmed and its name read.",
      rank: 0,
      driver: driver({ needed: "udev-rule", interface: null, current: null, tag: "no udev rule yet" }),
      boards: [{ board: "icesugar-1-5", level: "weak" as const }],
    };
    expect(lockedTag(locked)).toBe("no udev rule yet");
    expect(lockedTag({ ...locked, driver: null })).toBe("can't be opened");
    expect(unreachableNote(locked, null, () => "iCESugar v1.5")).toBe(locked.note);
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
      note: "This may be your Sipeed Tang Nano 9k: it has the same USB IDs. It needs its driver first, so it can be programmed.",
      rank: 0,
      driver: driver(),
      boards: [{ board: "alhambra-ii", level: "weak" as const }, { board: "tangnano9k", level: "weak" as const }],
    };
    const name = (id: string) => ({ "alhambra-ii": "Alhambra II", tangnano9k: "Sipeed Tang Nano 9k" })[id] ?? id;
    const text = devicesReport({ usb: [{ ...alhambra, rank: 1 }], unreachable: [locked] }, "tangnano9k", name, {
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

  it("lists the devices in Logix Engine's order, one list", () => {
    const mouse = { ...alhambra, id: "mouse", label: "USB Mouse", boards: [], likelyBoards: [], rank: 2 };
    const other = { ...alhambra, id: "other", rank: 1 };
    const locked = {
      id: "locked", label: "USB device (0403:6010)", vid: "0403", pid: "6010",
      action: "install_drivers", who: "USB device (0403:6010)", fix: "It needs its driver first, so it can be programmed.",
      note: "This may be your Alhambra II: it has the same USB IDs. It needs its driver first, so it can be programmed.",
      rank: 0,
      driver: driver(),
      boards: [{ board: "alhambra-ii", level: "weak" as const }, { board: "alchitry-au", level: "weak" as const }],
    };
    expect(orderedDevices({ usb: [mouse, other], unreachable: [locked] }).map((d) => d.device.id)).toEqual([
      "locked",
      "other",
      "mouse",
    ]);
    // The engine's note, saying which board that is in Logix IDE.
    const name = (id: string) => ({ "alhambra-ii": "Alhambra II", "alchitry-au": "Alchitry Au" })[id] ?? id;
    expect(unreachableNote(locked, "alhambra-ii", name)).toBe(
      "This may be your Alhambra II (the board selected in the toolbar): it has the same USB IDs. It needs its driver first, so it can be programmed.",
    );
    expect(unreachableNote(locked, "icestick", name)).toBe(locked.note);
  });

  it("tells whether a device may be a board", () => {
    const generic = { ...alhambra, boards: [{ board: "alchitry-au", level: "weak" as const }] };
    expect(mayBeBoard(generic, "alchitry-au")).toBe(true);
    expect(mayBeBoard(generic, "icestick")).toBe(false);
    expect(mayBeBoard(generic, null)).toBe(false);
  });

  it("groups a device's boards as Logix Engine names them", () => {
    const d = { ...alhambra, likelyBoards: ["arty", "basys3"], otherBoards: ["generic-a"] };
    expect(deviceBoardGroups(d)).toEqual({ matches: ["arty", "basys3"], others: ["generic-a"] });
  });

  it("puts match levels in words", () => {
    expect(matchLabel("strong")).toBe("Strong match");
    expect(matchLabel("possible")).toBe("Possible match");
    expect(matchLabel("weak")).toBe("Weak match");
    expect(matchLabel(null)).toBe("");
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
