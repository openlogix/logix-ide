import { describe, expect, it } from "vitest";
import { BARE_MINIMUM, examplesSummary, groupExamples } from "./examples";

const boards = [
  { id: "alhambra-ii", description: "Alhambra II (Lattice, iCE40)", family: "ice40", familyLabel: "Lattice iCE40", familyOrder: 0, pinExtension: null, dimensions: [], configurations: [], custom: false, fpga: null },
  { id: "icebreaker", description: "iCEBreaker (Lattice, iCE40)", family: "ice40", familyLabel: "Lattice iCE40", familyOrder: 0, pinExtension: null, dimensions: [], configurations: [], custom: false, fpga: null },
  { id: "no-examples", description: "Lonely Board", family: "", familyLabel: "Other", familyOrder: 99, pinExtension: null, dimensions: [], configurations: [], custom: false, fpga: null },
];
const examples = [
  { board: "alhambra-ii", name: "blinky", summary: "Blinks an LED", notes: [] },
  { board: "alhambra-ii", name: "ledon", summary: "Turns on an LED", notes: [] },
  { board: "icebreaker", name: "pll", summary: "Clock from a PLL", notes: ["Not yet tried on the board."] },
  { board: "mystery", name: "blinky", summary: "Blinks", notes: [] },
];

const summary = (q: string) =>
  groupExamples(boards, examples, q).map(
    (g) =>
      `${g.board.id}:${g.examples
        .map((e) => (e.name === BARE_MINIMUM ? "BM" : e.name))
        .join(",")}`,
  );

describe("groupExamples", () => {
  it("lists all boards in board order, each with Bare Minimum first", () => {
    expect(summary("")).toEqual([
      "alhambra-ii:BM,blinky,ledon",
      "icebreaker:BM,pll",
      "no-examples:BM",
      "mystery:BM,blinky",
    ]);
  });

  it("counts only real examples, not Bare Minimum", () => {
    const counts = groupExamples(boards, examples, "").map((g) => g.count);
    expect(counts).toEqual([2, 1, 0, 1]);
  });

  it("gives every board's Bare Minimum the same summary", () => {
    const groups = groupExamples(boards, examples, "");
    for (const g of groups) {
      expect(g.examples[0].bare).toBe(true);
      expect(g.examples[0].summary).toContain("board's pins");
    }
  });

  it("shows all entries of a matching board", () => {
    expect(summary("alhambra")).toEqual(["alhambra-ii:BM,blinky,ledon"]);
  });

  it("finds bare-minimum by name, with a hyphen or a space", () => {
    const all = ["alhambra-ii:BM", "icebreaker:BM", "no-examples:BM", "mystery:BM"];
    expect(summary("minimum")).toEqual(all);
    expect(summary("bare-minimum")).toEqual(all);
    expect(summary("Bare Minimum")).toEqual(all);
    expect(BARE_MINIMUM).toBe("bare-minimum");
  });

  it("shows only matching examples otherwise", () => {
    expect(summary("BLINK")).toEqual(["alhambra-ii:blinky", "mystery:blinky"]);
    expect(summary("clock")).toEqual(["icebreaker:pll"]);
    // Notes count too.
    expect(summary("not yet tried")).toEqual(["icebreaker:pll"]);
  });

  it("returns nothing when nothing matches", () => {
    expect(summary("zzz")).toEqual([]);
  });
});

describe("examplesSummary", () => {
  it("mentions hidden examples only when there are some", () => {
    expect(examplesSummary(146, 18)).toBe(
      "146 examples available (18 hidden: not supported by Logix yet).",
    );
    expect(examplesSummary(1, 0)).toBe("1 example available.");
  });
});
