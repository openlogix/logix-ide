import { describe, expect, it } from "vitest";
import {
  confidenceLabel,
  deviceSummaryLines,
  isBoardNotFound,
  likelyBoards,
  mayBeBoard,
  needsDeviceChoice,
  usbLabel,
} from "./devices";
import type { TaskOutcome, UsbDevice } from "./backend";

const alhambra: UsbDevice = {
  id: "usb:1-1:0403:6010:FTATANKC",
  vid: "0403",
  pid: "6010",
  manufacturer: "AlhambraBits",
  product: "Alhambra II v1.0A",
  serialNumber: "FTATANKC",
  boards: [
    { board: "alhambra-ii", confidence: 3 },
    { board: "alchitry-au", confidence: 1 },
  ],
  buildConfidence: null,
};

const outcome = (action: string | null, devices: UsbDevice[] = []): TaskOutcome => ({
  ok: false, diagnostics: [], testbenches: [], report: null,
  cancelled: false, graphSvg: null, seconds: 1, action, devices,
});

describe("devices", () => {
  it("describes a device in plain words", () => {
    expect(usbLabel(alhambra)).toBe("Alhambra II v1.0A (AlhambraBits, 0403:6010)");
    expect(usbLabel({ ...alhambra, product: "", manufacturer: "" })).toBe(
      "Unknown USB device (0403:6010)",
    );
  });

  it("recognizes a board-not-found upload error", () => {
    expect(isBoardNotFound(outcome("connect_board"))).toBe(true);
    expect(isBoardNotFound(outcome(null))).toBe(false);
  });

  it("asks for a device only when several fit", () => {
    expect(needsDeviceChoice(outcome("select_device", [alhambra, alhambra]))).toBe(true);
    expect(needsDeviceChoice(outcome("select_device", [alhambra]))).toBe(false);
    expect(needsDeviceChoice(outcome("connect_board"))).toBe(false);
  });

  it("names the likely boards, only for name matches", () => {
    expect(likelyBoards(alhambra)).toEqual(["alhambra-ii"]);
    const generic = { ...alhambra, boards: [{ board: "alchitry-au", confidence: 1 }] };
    expect(likelyBoards(generic)).toEqual([]);
    expect(mayBeBoard(generic, "alchitry-au")).toBe(true);
    expect(mayBeBoard(generic, "icestick")).toBe(false);
    expect(mayBeBoard(generic, null)).toBe(false);
  });

  it("puts confidence in words, by comparing", () => {
    expect(confidenceLabel(3)).toBe("Matches the board's name");
    expect(confidenceLabel(2)).toBe("Likely");
    expect(confidenceLabel(1)).toBe("Possible");
    expect(confidenceLabel(null)).toBe("");
  });

  it("lists connected devices or gives tips", () => {
    expect(deviceSummaryLines({ usb: [alhambra], unreachable: [] })[1]).toBe(
      "  • Alhambra II v1.0A (AlhambraBits, 0403:6010)",
    );
    const none = deviceSummaryLines({
      usb: [],
      unreachable: [{ vid: "0403", pid: "6014", action: "fix_device_permissions", actionDescription: "Allow it." }],
    });
    expect(none[0]).toBe("No USB devices are connected.");
    expect(none[1]).toBe("  • A device (0403:6014) can't be opened. Allow it.");
  });
});
