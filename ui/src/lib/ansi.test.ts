import { describe, expect, it } from "vitest";
import { addLink, parseAnsi } from "./ansi";

describe("parseAnsi", () => {
  it("passes plain text through", () => {
    expect(parseAnsi("hello")).toEqual([{ text: "hello", style: "" }]);
    expect(parseAnsi("")).toEqual([]);
  });

  it("colors text and resets", () => {
    expect(parseAnsi("[\x1b[32mSUCCESS\x1b[0m] Took 1 s")).toEqual([
      { text: "[", style: "" },
      { text: "SUCCESS", style: "color:var(--ansi-2)" },
      { text: "] Took 1 s", style: "" },
    ]);
  });

  it("handles bold, bright, 256 and true colors", () => {
    expect(parseAnsi("\x1b[1;91mx")[0].style).toBe(
      "color:var(--ansi-9);font-weight:bold",
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
      { text: "ab", style: "color:var(--ansi-3)" },
    ]);
  });
});

describe("addLink", () => {
  const links = (line: string, location?: { file: string; line: number }) =>
    addLink(parseAnsi(line), location)
      .filter((s) => s.link)
      .map((s) => `${s.text} -> ${s.link!.file}:${s.link!.line}`);

  it("links the place Logix Engine gives", () => {
    expect(links("main.v:7: ERROR: syntax error", { file: "main.v", line: 7 })).toEqual([
      "main.v:7 -> main.v:7",
    ]);
    expect(
      links("%Warning-WIDTH: rtl/uart.v:8:15: Operator", { file: "rtl/uart.v", line: 8 }),
    ).toEqual(["rtl/uart.v:8 -> rtl/uart.v:8"]);
  });

  it("links the whole line when its text doesn't show the place", () => {
    expect(links("ERROR at line 7", { file: "main.v", line: 7 })).toEqual([
      "ERROR at line 7 -> main.v:7",
    ]);
  });

  it("reads no paths out of lines without a location", () => {
    expect(links("main.v:7: ERROR: syntax error")).toEqual([]);
  });

  it("keeps the text and colors around a link", () => {
    const segs = addLink(parseAnsi("\x1b[31mmain.v:7: bad\x1b[0m"), { file: "main.v", line: 7 });
    expect(segs.map((s) => s.text).join("")).toBe("main.v:7: bad");
    expect(segs[0]).toEqual({
      text: "main.v:7",
      style: "color:var(--ansi-1)",
      link: { file: "main.v", line: 7 },
    });
    expect(segs[1]).toEqual({ text: ": bad", style: "color:var(--ansi-1)" });
  });
});
