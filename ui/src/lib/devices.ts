import type { Devices, UsbDevice } from "./backend";

/** A device in plain words, e.g. "Alhambra II v1.0A (AlhambraBits, 0403:6010)". */
export function usbLabel(d: UsbDevice): string {
  const name = d.product || "Unknown USB device";
  const details = [d.manufacturer, `${d.vid}:${d.pid}`].filter(Boolean);
  return `${name} (${details.join(", ")})`;
}

/** True if an Upload failure message means the board wasn't found on USB. */
export function isBoardNotFound(message: string): boolean {
  return /^No matching USB device/i.test(message);
}

/** Console lines listing what's connected, with tips when nothing is. */
export function deviceSummaryLines(devices: Devices): string[] {
  if (devices.usb.length === 0) {
    return [
      "No USB devices are connected.",
      "Tips: check that the board is plugged in and powered, and try another cable",
      "(some USB cables only charge and don't carry data).",
    ];
  }
  return [
    "Connected USB devices:",
    ...devices.usb.map((d) => `  • ${usbLabel(d)}`),
    "If your board isn't listed, check the cable and power.",
    "If it's listed, check that the board selected in the toolbar matches it.",
  ];
}
