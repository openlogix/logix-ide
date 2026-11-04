import { describe, expect, it } from "vitest";
import type { Board } from "./backend";
import {
  choose,
  configurationOf,
  isAvailable,
  isConnected,
  menus,
  parseMenuId,
  resolve,
  summary,
  toStore,
} from "./variants";

const plain: Board = {
  id: "alhambra-ii",
  description: "Alhambra II",
  arch: "ice40",
  pinExtension: "pcf",
  // Every board has both standard dimensions; one value each is no choice.
  dimensions: [
    { id: "connection", label: "Connection", values: [{ id: "usb", label: "USB" }] },
    { id: "memory", label: "Memory", values: [{ id: "flash", label: "Flash" }] },
  ],
  configurations: [{ when: { connection: "usb", memory: "flash" }, tested: true }],
};

// Like the Arty A7: its own USB port, RAM only.
const arty: Board = {
  ...plain,
  id: "arty",
  dimensions: [
    { id: "connection", label: "Connection", values: [{ id: "usb", label: "USB" }] },
    { id: "memory", label: "Memory", values: [{ id: "ram", label: "RAM" }] },
  ],
  configurations: [{ when: { connection: "usb", memory: "ram" }, tested: true }],
};

// Like ButterStick: the bootloader writes flash only, the adapter both.
const stick: Board = {
  id: "stick",
  description: "Stick",
  arch: "ecp5",
  pinExtension: "lpf",
  dimensions: [
    {
      id: "connection",
      label: "Connection",
      values: [
        { id: "bootloader", label: "USB bootloader" },
        { id: "ft232h", label: "FT232H adapter" },
      ],
    },
    {
      id: "memory",
      label: "Memory",
      values: [
        { id: "flash", label: "Flash" },
        { id: "ram", label: "RAM", help: "Lost at power-off." },
      ],
    },
  ],
  configurations: [
    { when: { connection: "bootloader", memory: "flash" }, tested: true },
    { when: { connection: "ft232h", memory: "flash" }, tested: true },
    { when: { connection: "ft232h", memory: "ram" }, tested: false },
  ],
};

describe("variants", () => {
  it("defaults to the first configuration", () => {
    expect(resolve(stick, {})).toEqual({ connection: "bootloader", memory: "flash" });
    expect(summary(stick, {})).toBe("USB bootloader · Flash");
    expect(summary(plain, {})).toBe("USB · Flash");
    expect(configurationOf(plain, {})).toBe(plain.configurations[0]);
  });

  it("shows every dimension, even with one value", () => {
    expect(summary(arty, {})).toBe("USB · RAM");
    expect(menus(arty, {}).map((m) => m.values.map((v) => [v.label, v.checked, v.enabled]))).toEqual([
      [["USB", true, true]],
      [["RAM", true, true]],
    ]);
  });

  it("picks the first configuration with the given values", () => {
    expect(resolve(stick, { memory: "ram" })).toEqual({ connection: "ft232h", memory: "ram" });
    expect(configurationOf(stick, { memory: "ram" })?.tested).toBe(false);
    // Values the board doesn't have are ignored.
    expect(resolve(stick, { size: "85k", memory: "ram" })).toEqual({
      connection: "ft232h",
      memory: "ram",
    });
    expect(resolve(stick, { connection: "jtag" })).toEqual({ connection: "bootloader", memory: "flash" });
  });

  it("stores nothing for the default", () => {
    expect(toStore(stick, { connection: "bootloader" })).toEqual({});
    expect(toStore(stick, { memory: "ram" })).toEqual({ connection: "ft232h", memory: "ram" });
  });

  it("greys out values that don't exist with the others", () => {
    expect(isAvailable(stick, {}, "memory", "ram")).toBe(false);
    expect(isAvailable(stick, { connection: "ft232h" }, "memory", "ram")).toBe(true);
    expect(isAvailable(stick, { memory: "ram" }, "connection", "bootloader")).toBe(false);
  });

  it("choosing a greyed value switches the other dimensions too", () => {
    const ram = choose(stick, {}, "memory", "ram");
    expect(ram.variant).toEqual({ connection: "ft232h", memory: "ram" });
    expect(ram.changed).toEqual(["Connection: FT232H adapter"]);
    const back = choose(stick, ram.variant, "connection", "bootloader");
    expect(back.variant).toEqual({});
    expect(back.changed).toEqual(["Memory: Flash"]);
    const adapter = choose(stick, {}, "connection", "ft232h");
    expect(adapter.variant).toEqual({ connection: "ft232h", memory: "flash" });
    expect(adapter.changed).toEqual([]);
  });

  it("makes one Tools submenu per dimension", () => {
    expect(menus(null, {})).toEqual([]);
    const [connection, memory] = menus(stick, { connection: "ft232h" });
    expect(connection.label).toBe("Connection");
    expect(connection.values.map((v) => v.checked)).toEqual([false, true]);
    expect(memory.values).toEqual([
      { id: "flash", label: "Flash", checked: true, enabled: true },
      { id: "ram", label: "RAM", checked: false, enabled: true },
    ]);
    expect(menus(stick, {})[1].values[1].enabled).toBe(false);
    expect(parseMenuId("variant:memory:ram")).toEqual({ dim: "memory", value: "ram" });
    expect(parseMenuId("verify")).toBeNull();
  });

  it("is connected only the way the variant programs it", () => {
    const viaCable = [{ board: "stick", exact: false, suggested: true, variants: [{ connection: "ft232h", memory: "flash" }] }];
    expect(isConnected(stick, "stick", {}, viaCable)).toBe(false);
    expect(isConnected(stick, "stick", { connection: "ft232h" }, viaCable)).toBe(true);
    // The adapter reaches RAM too: same device.
    const both = [{ board: "stick", exact: false, suggested: true, variants: [
      { connection: "ft232h", memory: "flash" }, { connection: "ft232h", memory: "ram" }] }];
    expect(isConnected(stick, "stick", { memory: "ram" }, both)).toBe(true);
    expect(isConnected(plain, "alhambra-ii", {}, [{ board: "alhambra-ii", exact: true, suggested: true, variants: [{}] }])).toBe(true);
    expect(isConnected(plain, "alhambra-ii", {}, [])).toBe(false);
  });
});
