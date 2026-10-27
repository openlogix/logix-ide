import { describe, expect, it } from "vitest";
import { BARE_MINIMUM, examplesSummary, groupExamples } from "./examples";

const boards = [
  { id: "alhambra-ii", description: "Alhambra II (Lattice, iCE40)", arch: "ice40" },
  { id: "icebreaker", description: "iCEBreaker (Lattice, iCE40)", arch: "ice40" },
  { id: "no-examples", description: "Lonely Board", arch: "" },
];
const examples = [
  { board: "alhambra-ii", name: "blinky", description: "Blinks an LED" },
  { board: "alhambra-ii", name: "ledon", description: "Turns on an LED" },
  { board: "icebreaker", name: "pll", description: "Clock from a PLL" },
  { board: "mystery", name: "blinky", description: "Blinks" },
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

  it("counts only Apio's examples", () => {
    const counts = groupExamples(boards, examples, "").map((g) => g.count);
    expect(counts).toEqual([2, 1, 0, 1]);
  });

  it("says when Bare Minimum has no pins", () => {
    const [alhambra, icebreaker] = groupExamples(boards, examples, "");
    expect(alhambra.examples[0].bare).toBe(true);
    expect(alhambra.examples[0].description).toContain("board's pins");
    expect(icebreaker.examples[0].description).toContain("without pins");
  });

  it("shows all entries of a matching board", () => {
    expect(summary("alhambra")).toEqual(["alhambra-ii:BM,blinky,ledon"]);
  });

  it("finds Bare Minimum by name", () => {
    expect(summary("minimum")).toEqual([
      "alhambra-ii:BM",
      "icebreaker:BM",
      "no-examples:BM",
      "mystery:BM",
    ]);
  });

  it("shows only matching examples otherwise", () => {
    expect(summary("BLINK")).toEqual(["alhambra-ii:blinky", "mystery:blinky"]);
    expect(summary("clock")).toEqual(["icebreaker:pll"]);
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
