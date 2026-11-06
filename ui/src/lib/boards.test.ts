import { describe, expect, it } from "vitest";
import { boardChangeNote, boardGroups, fpgaLabel, groupByArch, boardsWithFpga, withProjectBoards } from "./boards";
import type { Board } from "./backend";

const boards: Board[] = [
  { id: "alhambra-ii", description: "Alhambra II", arch: "ice40", archLabel: "Lattice iCE40", archOrder: 0, pinExtension: null, dimensions: [], configurations: [], custom: false, fpga: null },
  { id: "arty", description: "Arty A7 (Xilinx, Artix7)", arch: "xilinx", archLabel: "Xilinx", archOrder: 3, pinExtension: null, dimensions: [], configurations: [], custom: false, fpga: null },
  { id: "colorlight", description: "ColorLight 5A-75B", arch: "ecp5", archLabel: "Lattice ECP5", archOrder: 1, pinExtension: null, dimensions: [], configurations: [], custom: false, fpga: null },
  { id: "icebreaker", description: "iCEBreaker", arch: "ice40", archLabel: "Lattice iCE40", archOrder: 0, pinExtension: null, dimensions: [], configurations: [], custom: false, fpga: null },
  { id: "mystery", description: "Mystery Board", arch: "", archLabel: "Other", archOrder: 99, pinExtension: null, dimensions: [], configurations: [], custom: false, fpga: null },
  { id: "tang", description: "Tang Nano 9K", arch: "gowin", archLabel: "Gowin", archOrder: 2, pinExtension: null, dimensions: [], configurations: [], custom: false, fpga: null },
];

const summary = (recent: string[], q: string, list: Board[] = boards) =>
  boardGroups(list, recent, q).map((g) => `${g.label}: ${g.boards.map((b) => b.id).join(",")}`);

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

  it("lists the recently used boards first, newest first, and in their group too", () => {
    const groups = summary(["tang", "gone-board", "alhambra-ii"], "");
    expect(groups[0]).toBe("Recently used: tang,alhambra-ii");
    expect(groups).toContain("Gowin: tang");
    const [recent, ice40] = boardGroups(boards, ["tang"], "");
    expect(recent.recent).toBe(true);
    expect(ice40.recent).toBe(false);
  });

  it("puts the project's own boards in their architecture's group", () => {
    const mine = { ...boards[0], id: "mine", description: "Mine", custom: true };
    const over = { ...boards[1], description: "My Arty", custom: true };
    const all = withProjectBoards(boards, [mine, over]);
    expect(all.slice(0, 2)).toEqual([mine, over]);
    expect(all.filter((b) => b.id === "arty")).toEqual([over]);
    expect(withProjectBoards(boards, [])).toBe(boards);
    const groups = summary([], "", all);
    expect(groups[0]).toBe("Lattice iCE40: mine,alhambra-ii,icebreaker");
    expect(groups).toContain("Xilinx: arty");
  });

  it("searches names, ids and architectures", () => {
    expect(summary([], "ICE40")).toEqual(["Lattice iCE40: alhambra-ii,icebreaker"]);
    expect(summary([], "gowin")).toEqual(["Gowin: tang"]);
    expect(summary(["alhambra-ii"], "tang")).toEqual(["Gowin: tang"]);
    expect(summary([], "zzz")).toEqual([]);
  });
});

describe("FPGAs", () => {
  const hx4k = { id: "ice40hx4k-tq144-8k", partNumber: "ICE40HX4K-TQ144", size: "8k" };
  const withFpga: Board[] = [
    { ...boards[0], fpga: hx4k },
    { ...boards[3], fpga: { id: "ice40up5k-sg48", partNumber: "ICE40UP5K-SG48", size: "5k" } },
    { ...boards[0], id: "kefir", description: "Kefir", fpga: hx4k },
    ...boards.slice(4),
  ];

  it("finds boards by their FPGA's part number or ID, though it isn't shown", () => {
    expect(summary([], "up5k", withFpga)).toEqual(["Lattice iCE40: icebreaker"]);
    expect(summary([], "ICE40HX4K-TQ144", withFpga)).toEqual(["Lattice iCE40: alhambra-ii,kefir"]);
  });

  it("names the FPGA for people", () => {
    expect(fpgaLabel(withFpga[0])).toBe("Lattice iCE40 ICE40HX4K-TQ144 (8k)");
    expect(fpgaLabel(boards[4])).toBe("");
  });

  it("lists every board with the same FPGA, itself included, by name", () => {
    const ids = (b: Board) => boardsWithFpga(withFpga, b).map((x) => x.id);
    expect(ids(withFpga[0])).toEqual(["alhambra-ii", "kefir"]);
    // The same list from each of them.
    expect(ids(withFpga[2])).toEqual(["alhambra-ii", "kefir"]);
    expect(ids(withFpga[1])).toEqual(["icebreaker"]);
    // A custom board's own FPGA definition has no ID to compare: just it.
    const own = { ...withFpga[0], id: "mine", fpga: { ...hx4k, id: null } };
    expect(ids(own)).toEqual(["mine"]);
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
});

describe("boardChangeNote", () => {
  const hx4k = { id: "ice40hx4k-tq144-8k", partNumber: "ICE40HX4K-TQ144", size: "8k" };
  const alhambra: Board = { ...boards[0], pinExtension: "pcf", fpga: hx4k };
  const kefir: Board = { ...alhambra, id: "kefir", description: "Kéfir I" };
  const icebreaker: Board = {
    ...boards[3],
    pinExtension: "pcf",
    fpga: { id: "ice40up5k-sg48", partNumber: "ICE40UP5K-SG48", size: "5k" },
  };
  const colorlight: Board = {
    ...boards[2],
    pinExtension: "lpf",
    fpga: { id: "lfe5u-25f", partNumber: "LFE5U-25F-6BG256C", size: "25k" },
  };

  it("says what a board change means for the project, as short points", () => {
    expect(boardChangeNote(alhambra, kefir, "pinout.pcf")).toEqual([
      "Same FPGA: ICE40HX4K-TQ144.",
      "Check that the pins in pinout.pcf match this board.",
    ]);
    expect(boardChangeNote(alhambra, icebreaker, "pinout.pcf")).toEqual([
      "Another Lattice iCE40 FPGA: ICE40HX4K-TQ144 → ICE40UP5K-SG48.",
      "pinout.pcf needs this board's pins.",
    ]);
    expect(boardChangeNote(alhambra, colorlight, "pinout.pcf")).toEqual([
      "Another FPGA family: Lattice iCE40 → Lattice ECP5.",
      "Needs a .lpf pin file instead of pinout.pcf.",
      "Probably significant changes: Lattice iCE40 parts, such as I/O cells and PLLs, won't build.",
    ]);
    expect(boardChangeNote(alhambra, colorlight, null)[1]).toBe("Needs a .lpf pin file.");
    expect(boardChangeNote(null, colorlight, null)).toEqual([]);
    expect(boardChangeNote(alhambra, alhambra, "pinout.pcf")).toEqual([]);
  });
});
