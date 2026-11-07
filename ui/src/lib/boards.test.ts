import { describe, expect, it } from "vitest";
import { boardChangeNote, boardGroups, boardsProblemLines, fpgaLabel, groupByArch, boardsWithFpga } from "./boards";
import type { Board, BoardComparison } from "./backend";

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
    expect(recent.kind).toBe("recent");
    expect(ice40.kind).toBe("arch");
  });

  it("lists the project's own boards first, as Custom boards, and in their group too", () => {
    const mine = { ...boards[0], id: "mine", description: "Mine", custom: true };
    // As the engine merges them: by description, not custom first.
    const all = [...boards.slice(0, 4), mine, ...boards.slice(4)];
    const groups = summary(["tang"], "", all);
    expect(groups.slice(0, 3)).toEqual([
      "Custom boards: mine",
      "Recently used: tang",
      "Lattice iCE40: mine,alhambra-ii,icebreaker",
    ]);
    expect(boardGroups(all, [], "")[0].kind).toBe("custom");
    // No custom boards: no group, unless the file has problems.
    expect(summary([], "")[0]).toBe("Lattice iCE40: alhambra-ii,icebreaker");
    expect(boardGroups(boards, [], "", true)[0]).toMatchObject({ kind: "custom", boards: [] });
    // Searching keeps only matches.
    expect(summary([], "icebreaker", all)).toEqual(["Lattice iCE40: icebreaker"]);
  });

  it("puts the project's own boards in their architecture's group", () => {
    const mine = { ...boards[0], id: "mine", description: "Mine", custom: true };
    // The engine's merged list: "My Arty" replaced the catalog's Arty.
    const over = { ...boards[1], description: "My Arty", custom: true };
    const all = [boards[0], boards[2], boards[3], mine, boards[4], over, boards[5]];
    const groups = summary([], "", all);
    expect(groups).toContain("Lattice iCE40: mine,alhambra-ii,icebreaker");
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
  // Logix Engine's compare_boards answers, as the engine words each aspect.
  type Same = "yes" | "no" | "unknown";
  const a = (aspect: string, same: Same, from: string | null, to: string | null, text: string) => ({
    aspect, same, text, ...(from === null ? {} : { from }), ...(to === null ? {} : { to }),
  });
  const cmp = (...aspects: ReturnType<typeof a>[]): BoardComparison => ({ sameBuild: false, aspects });
  const kefir = cmp(
    a("board", "no", "Alhambra II", "Kéfir I", "Board: Alhambra II → Kéfir I."),
    a("architecture", "yes", "Lattice iCE40", "Lattice iCE40", "Same architecture: Lattice iCE40."),
    a("fpga", "yes", "ICE40HX4K-TQ144", "ICE40HX4K-TQ144", "Same FPGA: ICE40HX4K-TQ144."),
    a("pins", "unknown", null, null, "Pins: not known whether they match."),
  );
  const icebreaker = cmp(
    a("board", "no", "Alhambra II", "iCEBreaker", "Board: Alhambra II → iCEBreaker."),
    a("architecture", "yes", "Lattice iCE40", "Lattice iCE40", "Same architecture: Lattice iCE40."),
    a("fpga", "no", "ICE40HX4K-TQ144", "ICE40UP5K-SG48", "FPGA: ICE40HX4K-TQ144 → ICE40UP5K-SG48."),
  );
  const colorlight = cmp(
    a("board", "no", "Alhambra II", "ColorLight 5A-75B", "Board: Alhambra II → ColorLight 5A-75B."),
    a("architecture", "no", "Lattice iCE40", "Lattice ECP5", "Architecture: Lattice iCE40 → Lattice ECP5."),
    a("fpga", "no", "ICE40HX4K-TQ144", "LFE5U-25F-6BG256C", "FPGA: ICE40HX4K-TQ144 → LFE5U-25F-6BG256C."),
    a("constraint_format", "no", ".pcf", ".lpf", "Pin file format: .pcf → .lpf."),
  );
  const same = cmp(a("board", "yes", "Alhambra II", "Alhambra II", "Same board: Alhambra II."));

  it("names the change at its highest level, then what it risks for the sources", () => {
    const show = (points: { text: string; kind: string }[]) =>
      points.map((p) => `${p.kind === "risky" ? "! " : p.kind === "change" ? "> " : ""}${p.text}`);
    expect(show(boardChangeNote(kefir, "pinout.pcf"))).toEqual([
      "> Board: Alhambra II → Kéfir I. Same FPGA: ICE40HX4K-TQ144.",
      "! The pins may differ: check pinout.pcf.",
      "! The clock frequency and the LED and button wiring may differ: check the design.",
    ]);
    expect(show(boardChangeNote(icebreaker, "pinout.pcf"))).toEqual([
      "> FPGA: ICE40HX4K-TQ144 → ICE40UP5K-SG48.",
      "! FPGA-specific parts, such as PLLs and special cells, may need changes.",
      "! The design may not fit, or may not reach its clock speed.",
      "! The pins, clock frequency and LED and button wiring may differ: check pinout.pcf and the design.",
    ]);
    expect(show(boardChangeNote(colorlight, "pinout.pcf"))).toEqual([
      "> Architecture: Lattice iCE40 → Lattice ECP5.",
      "! Needs a .lpf pin file instead of pinout.pcf, with this board's pins.",
      "! Probably significant changes: Lattice iCE40 parts, such as I/O cells and PLLs, won't build.",
      "! The clock frequency and the LED and button wiring may differ: check the design.",
    ]);
    expect(boardChangeNote(colorlight, null)[1].text).toBe("Needs a .lpf pin file, with this board's pins.");
    // Nothing to say: no comparison, or the same board.
    expect(boardChangeNote(null, "pinout.pcf")).toEqual([]);
    expect(boardChangeNote(same, "pinout.pcf")).toEqual([]);
  });
});

describe("boardsProblemLines", () => {
  it("gives one red line per problem, at line and column, like build errors", () => {
    expect(
      // The file as the engine named it (here, a saved project's).
      boardsProblemLines({
        message: "The custom boards file has problems.",
        diagnostics: [
          { severity: "error", file: "/Users/you/blinky 3/custom-boards.json", line: 4, column: 3, message: "Trailing comma." },
          { severity: "warning", file: "/Users/you/blinky 3/custom-boards.json", line: 9, column: null, message: "Odd." },
        ],
      }),
    ).toEqual([
      "\x1b[31m/Users/you/blinky 3/custom-boards.json:4:3: error: Trailing comma.\x1b[0m",
      "\x1b[31m/Users/you/blinky 3/custom-boards.json:9: warning: Odd.\x1b[0m",
    ]);
  });
  it("gives the message when there are no locations", () => {
    expect(boardsProblemLines({ message: "Can't read it.", diagnostics: [] })).toEqual([
      "\x1b[31mCan't read it.\x1b[0m",
    ]);
  });
});
