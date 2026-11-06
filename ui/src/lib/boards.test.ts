import { describe, expect, it } from "vitest";
import { boardGroups, deviceGroups, groupByArch, withProjectBoards, type DeviceGroup } from "./boards";
import type { ConnectedDevice } from "./backend";

const boards = [
  { id: "alhambra-ii", description: "Alhambra II", arch: "ice40", archLabel: "Lattice iCE40", archOrder: 0, pinExtension: null, dimensions: [], configurations: [], custom: false },
  { id: "arty", description: "Arty A7 (Xilinx, Artix7)", arch: "xilinx", archLabel: "Xilinx", archOrder: 3, pinExtension: null, dimensions: [], configurations: [], custom: false },
  { id: "colorlight", description: "ColorLight 5A-75B", arch: "ecp5", archLabel: "Lattice ECP5", archOrder: 1, pinExtension: null, dimensions: [], configurations: [], custom: false },
  { id: "icebreaker", description: "iCEBreaker", arch: "ice40", archLabel: "Lattice iCE40", archOrder: 0, pinExtension: null, dimensions: [], configurations: [], custom: false },
  { id: "mystery", description: "Mystery Board", arch: "", archLabel: "Other", archOrder: 99, pinExtension: null, dimensions: [], configurations: [], custom: false },
  { id: "tang", description: "Tang Nano 9K", arch: "gowin", archLabel: "Gowin", archOrder: 2, pinExtension: null, dimensions: [], configurations: [], custom: false },
];

/** One device that may be `ids`. */
const dev = (ids: string[], key = "usb:1"): DeviceGroup => ({
  key,
  label: `Device ${key}`,
  boards: ids,
});

const summary = (devices: DeviceGroup[], q: string) =>
  boardGroups(boards, devices, q).map(
    (g) => `${g.label}: ${g.boards.map((b) => b.id).join(",")}`,
  );

describe("boardGroups", () => {
  it("groups by architecture, in a fixed order", () => {
    expect(summary([], "")).toEqual([
      "Lattice iCE40: alhambra-ii,icebreaker",
      "Lattice ECP5: colorlight",
      "Gowin: tang",
      "Xilinx: arty",
      "Other: mystery",
    ]);
  });

  it("lists each connected device's boards first, and in their group too", () => {
    expect(summary([dev(["tang", "unknown-board"])], "")[0]).toBe("Device usb:1: tang");
    expect(summary([dev(["tang"])], "")).toContain("Gowin: tang");
  });

  it("keeps two devices apart, even with the same boards", () => {
    const groups = boardGroups(boards, [dev(["arty"], "usb:1"), dev(["arty"], "usb:2")], "");
    expect(groups.slice(0, 2).map((g) => g.key)).toEqual(["usb:1", "usb:2"]);
  });

  it("lists the project's own boards first, replacing catalog boards with their IDs", () => {
    const mine = { ...boards[0], id: "mine", description: "Mine", custom: true };
    const over = { ...boards[1], description: "My Arty", custom: true };
    const all = withProjectBoards(boards, [mine, over]);
    expect(all.slice(0, 2)).toEqual([mine, over]);
    expect(all.filter((b) => b.id === "arty")).toEqual([over]);
    expect(withProjectBoards(boards, [])).toBe(boards);
    const groups = boardGroups(all, [], "").map((g) => `${g.label}: ${g.boards.map((b) => b.id).join(",")}`);
    expect(groups[0]).toBe("This project's boards: mine,arty");
    expect(groups).not.toContain("Xilinx: arty");
  });

  it("searches names, ids and architectures", () => {
    expect(summary([], "ICE40")).toEqual(["Lattice iCE40: alhambra-ii,icebreaker"]);
    expect(summary([], "gowin")).toEqual(["Gowin: tang"]);
    expect(summary([dev(["alhambra-ii"])], "tang")).toEqual(["Gowin: tang"]);
    expect(summary([], "zzz")).toEqual([]);
  });
});

describe("groupByArch", () => {
  it("keeps the engine's order and puts unknown architectures last", () => {
    const arch = (arch: string, archLabel: string, archOrder: number) => ({ arch, archLabel, archOrder });
    const items = [
      { n: "a", ...arch("zeta", "ZETA", 99) },
      { n: "b", ...arch("xilinx", "Xilinx", 3) },
      { n: "c", ...arch("ice40", "Lattice iCE40", 0) },
      { n: "d", ...arch("alpha", "ALPHA", 99) },
    ];
    const groups = groupByArch(items, (i) => i).map(
      (g) => `${g.label}:${g.items.map((i) => i.n).join("")}`,
    );
    expect(groups).toEqual(["Lattice iCE40:c", "Xilinx:b", "ALPHA:d", "ZETA:a"]);
  });

  it("marks device groups", () => {
    const [device, ice40] = boardGroups(boards, [dev(["tang"])], "");
    expect(device.device).toBe(true);
    expect(ice40.device).toBe(false);
  });
});


describe("deviceGroups", () => {
  const digilent: ConnectedDevice = {
    id: "usb:1-3:0403:6010:210319AFEF8F",
    name: "Digilent USB Device",
    serial: "210319AFEF8F",
    boards: [
      { board: "arty-a7-35t", level: "strong", variants: [{}] },
      { board: "basys3", level: "strong", variants: [{}] },
      { board: "arty-a7-100t", level: "weak", variants: [{}] },
    ],
  };

  it("lists a device's boards that aren't weak, and the selected one when it fits", () => {
    expect(deviceGroups([digilent], "alhambra-ii")).toEqual([
      {
        key: digilent.id,
        label: "Connected: Digilent USB Device (210319AFEF8F)",
        boards: ["arty-a7-35t", "basys3"],
      },
    ]);
    // An Arty A7-100T selected with an Arty A7-35T plugged in: its filter
    // has only the USB ids, so it comes last.
    expect(deviceGroups([digilent], "arty-a7-100t")[0].boards).toEqual([
      "arty-a7-35t",
      "basys3",
      "arty-a7-100t",
    ]);
  });

  it("leaves out devices with nothing to list, and names without a serial", () => {
    const generic: ConnectedDevice = {
      id: "usb:1-4:0403:6010:",
      name: "Dual RS232-HS",
      serial: "",
      boards: [{ board: "a", level: "weak", variants: [{}] }],
    };
    expect(deviceGroups([generic], null)).toEqual([]);
    expect(deviceGroups([generic], "a")[0].label).toBe("Connected: Dual RS232-HS");
  });
});
