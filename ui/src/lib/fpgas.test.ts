import { describe, expect, it } from "vitest";
import type { Board, FpgaFamily } from "./backend";
import { count, fpgaGroups } from "./fpgas";

const families: FpgaFamily[] = [
  {
    id: "ice40",
    label: "Lattice iCE40",
    fpgas: [
      { id: "ice40hx4k-tq144-8k", partNumber: "ICE40HX4K-TQ144", size: "8k" },
      { id: "ice40up5k-sg48", partNumber: "ICE40UP5K-SG48", size: "5k" },
    ],
  },
  { id: "gw5a", label: "Gowin GW5A", fpgas: [{ id: "gw5a-25", partNumber: "GW5A-LV25UG324C2/I1", size: "25k" }] },
];

const board = (id: string, description: string, fpga: string): Board => ({
  id, description, family: "ice40", familyLabel: "Lattice iCE40", familyOrder: 0, pinExtension: "pcf",
  dimensions: [], configurations: [], custom: false, fpga: { id: fpga, partNumber: "", size: "" },
});
const boards = [
  board("kefir", "Kéfir I", "ice40hx4k-tq144-8k"),
  board("alhambra-ii", "Alhambra II", "ice40hx4k-tq144-8k"),
  board("icebreaker", "iCEBreaker", "ice40up5k-sg48"),
];

describe("fpgaGroups", () => {
  it("lists every family with its FPGAs and the boards that have them", () => {
    const groups = fpgaGroups(families, boards, "");
    expect(groups.map((g) => [g.label, g.fpgaCount, g.boardCount])).toEqual([
      ["Lattice iCE40", 2, 3],
      ["Gowin GW5A", 1, 0],
    ]);
    expect(groups[0].fpgas[0].boards).toEqual(["Alhambra II", "Kéfir I"]);
    expect(groups[1].fpgas[0].boards).toEqual([]);
  });

  it("searches part numbers, IDs and family names", () => {
    const show = (q: string) => fpgaGroups(families, boards, q).map((g) => `${g.label}: ${g.fpgas.map((f) => f.id).join(",")}`);
    expect(show("up5k")).toEqual(["Lattice iCE40: ice40up5k-sg48"]);
    expect(show("GW5A-25")).toEqual(["Gowin GW5A: gw5a-25"]);
    expect(show("gowin")).toEqual(["Gowin GW5A: gw5a-25"]);
    expect(show("zzz")).toEqual([]);
    // The family's counts stay its own, whatever matches.
    expect(fpgaGroups(families, boards, "up5k")[0]).toMatchObject({ fpgaCount: 2, boardCount: 3 });
  });

  it("counts in words", () => {
    expect([count(1, "FPGA"), count(48, "FPGA"), count(0, "board")]).toEqual(["1 FPGA", "48 FPGAs", "0 boards"]);
  });
});
