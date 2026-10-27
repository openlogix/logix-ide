import { describe, expect, it } from "vitest";
import { addLinks, parseAnsi } from "./ansi";

describe("parseAnsi", () => {
  it("passes plain text through", () => {
    expect(parseAnsi("hello")).toEqual([{ text: "hello", style: "" }]);
    expect(parseAnsi("")).toEqual([]);
  });

  it("colors text and resets", () => {
    expect(parseAnsi("[\x1b[32mSUCCESS\x1b[0m] Took 1 s")).toEqual([
      { text: "[", style: "" },
      { text: "SUCCESS", style: "color:#23d18b" },
      { text: "] Took 1 s", style: "" },
    ]);
  });

  it("handles bold, bright, 256 and true colors", () => {
    expect(parseAnsi("\x1b[1;91mx")[0].style).toBe(
      "color:#ff6b6b;font-weight:bold",
    );
    expect(parseAnsi("\x1b[38;5;196mx")[0].style).toBe("color:rgb(255,0,0)");
    expect(parseAnsi("\x1b[38;2;1;2;3mx")[0].style).toBe("color:rgb(1,2,3)");
    expect(parseAnsi("\x1b[38;5;244mx")[0].style).toBe(
      "color:rgb(128,128,128)",
    );
  });

  it("drops other escape sequences", () => {
    expect(parseAnsi("\x1b[2K\x1b[?25lline")).toEqual([
      { text: "line", style: "" },
    ]);
    expect(parseAnsi("\x1b]8;;http://x\x07link\x1b]8;;\x07")).toEqual([
      { text: "link", style: "" },
    ]);
  });

  it("merges neighbors with the same style", () => {
    expect(parseAnsi("\x1b[33ma\x1b[33mb")).toEqual([
      { text: "ab", style: "color:#e5c07b" },
    ]);
  });
});

describe("addLinks", () => {
  const links = (line: string) =>
    addLinks(parseAnsi(line))
      .filter((s) => s.link)
      .map((s) => `${s.link!.file}:${s.link!.line}`);

  it("finds file:line references to project-style files", () => {
    expect(links("main.v:7: ERROR: syntax error")).toEqual(["main.v:7"]);
    expect(links("%Warning-WIDTH: main.v:8:15: Operator")).toEqual(["main.v:8"]);
    expect(links("*** ASSERTION FAILED at main_tb.v:52: expected")).toEqual([
      "main_tb.v:52",
    ]);
    expect(links("bad.sv: bad.sv:4:1-3: syntax error")).toEqual(["bad.sv:4"]);
  });

  it("ignores full paths and other files", () => {
    expect(links("/opt/yosys/share/cells_sim.v:10: x")).toEqual([]);
    expect(links("Took 1.25 seconds")).toEqual([]);
    expect(links("notes.txt:3")).toEqual([]);
  });

  it("keeps the text and colors around a link", () => {
    const segs = addLinks(parseAnsi("\x1b[31mmain.v:7: bad\x1b[0m"));
    expect(segs.map((s) => s.text).join("")).toBe("main.v:7: bad");
    expect(segs[0]).toEqual({
      text: "main.v:7",
      style: "color:#f14c4c",
      link: { file: "main.v", line: 7 },
    });
  });
});
