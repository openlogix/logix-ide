import { describe, expect, it } from "vitest";
import type { Board, Example, FpgaFamily } from "./backend";
import { buildModel, detailsOf, listGroups, resourceRows, requirementText, type Details } from "./explorer";

const families: FpgaFamily[] = [
  {
    id: "ice40",
    label: "Lattice iCE40",
    pinExtension: "pcf",
    fpgas: [
      { id: "ice40hx4k-tq144-8k", partNumber: "ICE40HX4K-TQ144", size: "8k" },
      { id: "ice40up5k-sg48", partNumber: "ICE40UP5K-SG48", size: "5k" },
      { id: "ice40lp1k-cm36", partNumber: "ICE40LP1K-CM36", size: "1k" },
    ],
  },
  { id: "gw5a", label: "Gowin GW5A", pinExtension: "cst", fpgas: [{ id: "gw5a-25", partNumber: "GW5A-25", size: "25k" }] },
];

const board = (id: string, description: string, fpga: string, extra: Partial<Board> = {}): Board => ({
  id,
  description,
  family: "ice40",
  familyLabel: "Lattice iCE40",
  familyOrder: 0,
  pinExtension: "pcf",
  dimensions: [],
  configurations: [{ when: {}, label: "USB · Flash", tested: true }],
  custom: false,
  fpga: { id: fpga, partNumber: fpga.toUpperCase(), size: "8k" },
  ...extra,
});

const boards = [
  board("alhambra-ii", "Alhambra II", "ice40hx4k-tq144-8k", {
    resources: {
      clk: { type: "clock", pin: "49", mhz: 12 },
      leds: { type: "leds", pins: ["45", "44"], active: "high" },
      uart: { type: "uart", tx: "62", rx: "61" },
    },
  }),
  board("kefir", "Kéfir I", "ice40hx4k-tq144-8k"),
  board("icebreaker", "iCEBreaker", "ice40up5k-sg48", {
    requires: [{ type: "drive", name: "flash_cs", pins: ["16"], level: "high" }],
  }),
  board("mine", "My Board", "ice40up5k-sg48", { custom: true }),
];

const examples: Example[] = [
  { board: "alhambra-ii", name: "blinky", summary: "Blinks the LEDs", notes: [] },
  { board: "alhambra-ii", name: "counter", summary: "Counts", notes: ["Not yet tried on the board."] },
  { board: "icebreaker", name: "blinky", summary: "Blinks the LED", notes: [] },
];

const model = buildModel(families, boards, examples);
const show = (kind: "family" | "fpga" | "board" | "example", q = "") =>
  listGroups(model, kind, q).map((g) => `${g.label ?? "-"}: ${g.items.map((i) => `${i.label}${i.hint ? ` (${i.hint})` : ""}`).join(", ")}`);

describe("listGroups", () => {
  it("lists every kind by family, families as one flat list", () => {
    expect(show("family")).toEqual(["-: Lattice iCE40 (4 boards), Gowin GW5A (0 boards)"]);
    expect(show("fpga")).toEqual([
      "Lattice iCE40: ICE40HX4K-TQ144 (2 boards), ICE40LP1K-CM36, ICE40UP5K-SG48 (2 boards)",
      "Gowin GW5A: GW5A-25",
    ]);
    expect(show("board")[0]).toBe(
      "Lattice iCE40: Alhambra II (ICE40HX4K-TQ144-8K), iCEBreaker (ICE40UP5K-SG48), Kéfir I (ICE40HX4K-TQ144-8K), My Board (ICE40UP5K-SG48)",
    );
    expect(show("example")).toEqual([
      "Lattice iCE40: blinky (Alhambra II), blinky (iCEBreaker), counter (Alhambra II)",
    ]);
  });

  it("searches each kind by its names and IDs; a family's name keeps all its items", () => {
    expect(show("fpga", "up5k")).toEqual(["Lattice iCE40: ICE40UP5K-SG48 (2 boards)"]);
    expect(show("fpga", "gowin")).toEqual(["Gowin GW5A: GW5A-25"]);
    expect(show("board", "up5k")).toEqual(["Lattice iCE40: iCEBreaker (ICE40UP5K-SG48), My Board (ICE40UP5K-SG48)"]);
    expect(show("example", "icebreaker")).toEqual(["Lattice iCE40: blinky (iCEBreaker)"]);
    expect(show("family", "gw5a")).toEqual(["-: Gowin GW5A (0 boards)"]);
    expect(show("board", "zzz")).toEqual([]);
  });

  it("lists boards and examples the engine's families don't cover under Other", () => {
    const m = buildModel(families, [board("odd", "Odd", "x", { family: "newfamily" })], [
      { board: "gone", name: "e", summary: "", notes: [] },
    ]);
    expect(listGroups(m, "board", "").map((g) => g.label)).toEqual(["Other"]);
    expect(listGroups(m, "example", "").map((g) => g.label)).toEqual(["Other"]);
  });
});

describe("detailsOf", () => {
  const text = (d: Details | null) =>
    d && {
      title: `${d.kindLabel}: ${d.title}`,
      facts: d.facts.map((f) => `${f.label}: ${f.value}${f.link ? ` → ${f.link.kind}/${f.link.id}` : ""}`),
      sections: d.sections.map((s) =>
        s.kind === "links"
          ? `${s.title}: ${s.links.map((l) => `${l.ref.kind}/${l.ref.id}`).join(", ") || s.empty}`
          : s.kind === "lines"
            ? `${s.title}: ${s.lines.join(" | ")}`
            : `${s.title}: ${s.rows.map((r) => r.name).join(", ") || s.empty}`,
      ),
    };

  it("links a family to its boards and FPGAs", () => {
    expect(text(detailsOf(model, { kind: "family", id: "ice40" }))).toEqual({
      title: "FPGA family: Lattice iCE40",
      facts: ["Family ID: ice40", "Pin file: .pcf", "FPGAs: 3", "Boards: 4", "Examples: 3"],
      sections: [
        "Boards: board/alhambra-ii, board/icebreaker, board/kefir, board/mine",
        "FPGAs: fpga/ice40hx4k-tq144-8k, fpga/ice40lp1k-cm36, fpga/ice40up5k-sg48",
      ],
    });
    expect(text(detailsOf(model, { kind: "family", id: "gw5a" }))?.sections[0]).toBe("Boards: No boards yet.");
  });

  it("links an FPGA to its family, boards and their examples", () => {
    expect(text(detailsOf(model, { kind: "fpga", id: "ice40hx4k-tq144-8k" }))).toEqual({
      title: "FPGA: ICE40HX4K-TQ144",
      facts: ["Family: Lattice iCE40 → family/ice40", "Size: 8k", "FPGA ID: ice40hx4k-tq144-8k"],
      sections: [
        "Boards: board/alhambra-ii, board/kefir",
        "Examples: example/alhambra-ii/blinky, example/alhambra-ii/counter",
      ],
    });
  });

  it("links a board to its family, FPGA, examples and the boards with the same FPGA", () => {
    expect(text(detailsOf(model, { kind: "board", id: "alhambra-ii" }))).toEqual({
      title: "Board: Alhambra II",
      facts: [
        "Family: Lattice iCE40 → family/ice40",
        "FPGA: ICE40HX4K-TQ144-8K → fpga/ice40hx4k-tq144-8k",
        "Size: 8k",
        "Pin file: .pcf",
        "Board ID: alhambra-ii",
      ],
      sections: [
        "Ways to program it: USB · Flash (default)",
        "Resources: clk, leds, uart",
        "Examples: example/alhambra-ii/blinky, example/alhambra-ii/counter",
        "Boards with the same FPGA: board/kefir",
      ],
    });
    const ice = text(detailsOf(model, { kind: "board", id: "icebreaker" }))!;
    expect(ice.sections[1]).toBe("Every design must: Drive flash_cs (16) high");
    expect(ice.sections[2]).toBe("Resources: Not described yet.");
    expect(text(detailsOf(model, { kind: "board", id: "mine" }))!.facts).toContain("From: This project's boards folder");
    // Marked as the project's own, in the lists, the details and the links.
    expect(detailsOf(model, { kind: "board", id: "mine" })!.kindLabel).toBe("Custom board");
    expect(listGroups(model, "board", "mine")[0].items[0].custom).toBe(true);
    const links = detailsOf(model, { kind: "fpga", id: "ice40up5k-sg48" })!.sections[0];
    expect(links.kind === "links" && links.links.map((l) => [l.label, !!l.custom])).toEqual([
      ["iCEBreaker", false],
      ["My Board", true],
    ]);
  });

  it("links an example to its board, FPGA and family", () => {
    expect(text(detailsOf(model, { kind: "example", id: "alhambra-ii/counter" }))).toEqual({
      title: "Example: counter",
      facts: [
        "Board: Alhambra II → board/alhambra-ii",
        "FPGA: ICE40HX4K-TQ144-8K → fpga/ice40hx4k-tq144-8k",
        "Family: Lattice iCE40 → family/ice40",
      ],
      sections: ["About: Counts | Not yet tried on the board."],
    });
    expect(detailsOf(model, { kind: "board", id: "nope" })).toBeNull();
  });
});

describe("resources", () => {
  it("words each type, its pins and notes", () => {
    const rows = resourceRows({
      resources: {
        clk: { type: "clock", pin: "35", mhz: 12, requires: [{ type: "setup", text: "Close J1." }] },
        led: { type: "leds", pins: ["11"], active: "low" },
        leds: { type: "leds", pins: ["11", "37"], active: "low", io_standard: "LVCMOS33" },
        rgb: { type: "rgb_led", red: "41", green: "39", blue: "40", active: "low" },
        uart: { type: "uart", tx: "9", rx: "6" },
        j1: { type: "header", pins: ["1", null, "3"] },
        m: { type: "led_matrix", leds: ["a", "b"], colors: { red: "r", blue: "b" }, user_notes: "Seven LEDs." },
        odd: { type: "newtype", pins: ["7"] },
      },
    });
    expect(rows.map((r) => [r.name, r.type, r.pins, r.notes])).toEqual([
      ["clk", "Clock", "35", "12 MHz. Needs: Close J1."],
      ["j1", "Header", "1, –, 3", ""],
      ["led", "LED", "11", "active low"],
      ["leds", "LEDs", "11, 37", "active low. LVCMOS33"],
      ["m", "LED matrix", "a, b; R r, B b", "Seven LEDs."],
      ["odd", "newtype", "7", ""],
      ["rgb", "RGB LED", "R 41, G 39, B 40", "active low"],
      ["uart", "UART", "TX 9, RX 6", ""],
    ]);
    expect(resourceRows({})).toEqual([]);
  });

  it("words requirements", () => {
    expect(requirementText({ type: "drive", name: "wifi_gpio0", pins: ["L2"], level: "high", why: "Or the ESP32 resets the board." })).toBe(
      "Drive wifi_gpio0 (L2) high: Or the ESP32 resets the board.",
    );
    expect(requirementText({ type: "setup", text: "Close J1." })).toBe("Close J1.");
  });
});
