import { describe, expect, it } from "vitest";
import { deviceSummaryLines, isBoardNotFound, usbLabel } from "./devices";
import type { TaskOutcome } from "./backend";

const alhambra = {
  vid: "0403",
  pid: "6010",
  manufacturer: "AlhambraBits",
  product: "Alhambra II v1.0A",
  serialNumber: "FTATANKC",
  deviceType: "FT2232H",
};

describe("devices", () => {
  it("describes a device in plain words", () => {
    expect(usbLabel(alhambra)).toBe("Alhambra II v1.0A (AlhambraBits, 0403:6010)");
    expect(usbLabel({ ...alhambra, product: "", manufacturer: "" })).toBe(
      "Unknown USB device (0403:6010)",
    );
  });

  it("recognizes a board-not-found upload error", () => {
    const outcome = (action: string | null): TaskOutcome => ({
      ok: false, diagnostics: [], testbenches: [], report: null,
      cancelled: false, graphSvg: null, seconds: 1, action,
    });
    expect(isBoardNotFound(outcome("connect_board"))).toBe(true);
    expect(isBoardNotFound(outcome(null))).toBe(false);
  });

  it("lists connected devices or gives tips", () => {
    expect(deviceSummaryLines({ usb: [alhambra], serial: [] })[1]).toBe(
      "  • Alhambra II v1.0A (AlhambraBits, 0403:6010)",
    );
    expect(deviceSummaryLines({ usb: [], serial: [] })[0]).toBe(
      "No USB devices are connected.",
    );
  });
});
