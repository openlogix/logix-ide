import { describe, expect, it } from "vitest";
import { archLabel, boardGroups, groupByArch } from "./boards";

const boards = [
  { id: "alhambra-ii", description: "Alhambra II", arch: "ice40" },
  { id: "arty", description: "Arty A7 (Xilinx, Artix7)", arch: "xilinx" },
  { id: "colorlight", description: "ColorLight 5A-75B", arch: "ecp5" },
  { id: "icebreaker", description: "iCEBreaker", arch: "ice40" },
  { id: "mystery", description: "Mystery Board", arch: "" },
  { id: "tang", description: "Tang Nano 9K", arch: "gowin" },
];

const summary = (connected: string[], q: string) =>
  boardGroups(boards, connected, q).map(
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

  it("lists connected boards first, and in their group too", () => {
    expect(summary(["tang", "unknown-board"], "")[0]).toBe("Connected: tang");
    expect(summary(["tang"], "")).toContain("Gowin: tang");
  });

  it("searches names, ids and architectures", () => {
    expect(summary([], "ICE40")).toEqual(["Lattice iCE40: alhambra-ii,icebreaker"]);
    expect(summary([], "gowin")).toEqual(["Gowin: tang"]);
    expect(summary(["alhambra-ii"], "tang")).toEqual(["Gowin: tang"]);
    expect(summary([], "zzz")).toEqual([]);
  });
});

describe("groupByArch", () => {
  it("keeps the fixed order and puts unknown architectures last", () => {
    const items = [
      { n: "a", arch: "zeta" },
      { n: "b", arch: "xilinx" },
      { n: "c", arch: "ice40" },
      { n: "d", arch: "alpha" },
    ];
    const groups = groupByArch(items, (i) => i.arch).map(
      (g) => `${g.label}:${g.items.map((i) => i.n).join("")}`,
    );
    expect(groups).toEqual(["Lattice iCE40:c", "Xilinx:b", "ALPHA:d", "ZETA:a"]);
  });

  it("marks Connected as not collapsible", () => {
    const [connected, ice40] = boardGroups(boards, ["tang"], "");
    expect(connected.collapsible).toBe(false);
    expect(ice40.collapsible).toBe(true);
  });
});

describe("archLabel", () => {
  it("names known architectures and keeps others recognizable", () => {
    expect(archLabel("ecp5")).toBe("Lattice ECP5");
    expect(archLabel("nexus")).toBe("NEXUS");
    expect(archLabel("")).toBe("Other");
  });
});
