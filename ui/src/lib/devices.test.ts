import { describe, expect, it } from "vitest";
import { deviceSummaryLines, isBoardNotFound, usbLabel } from "./devices";

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
    expect(isBoardNotFound("No matching USB device.")).toBe(true);
    expect(isBoardNotFound("syntax error")).toBe(false);
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
