import { describe, expect, it } from "vitest";
import { boardGroups, groupByArch, withProjectBoards } from "./boards";
import type { Board } from "./backend";

const boards: Board[] = [
  { id: "alhambra-ii", description: "Alhambra II", arch: "ice40", archLabel: "Lattice iCE40", archOrder: 0, pinExtension: null, dimensions: [], configurations: [], custom: false },
  { id: "arty", description: "Arty A7 (Xilinx, Artix7)", arch: "xilinx", archLabel: "Xilinx", archOrder: 3, pinExtension: null, dimensions: [], configurations: [], custom: false },
  { id: "colorlight", description: "ColorLight 5A-75B", arch: "ecp5", archLabel: "Lattice ECP5", archOrder: 1, pinExtension: null, dimensions: [], configurations: [], custom: false },
  { id: "icebreaker", description: "iCEBreaker", arch: "ice40", archLabel: "Lattice iCE40", archOrder: 0, pinExtension: null, dimensions: [], configurations: [], custom: false },
  { id: "mystery", description: "Mystery Board", arch: "", archLabel: "Other", archOrder: 99, pinExtension: null, dimensions: [], configurations: [], custom: false },
  { id: "tang", description: "Tang Nano 9K", arch: "gowin", archLabel: "Gowin", archOrder: 2, pinExtension: null, dimensions: [], configurations: [], custom: false },
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
