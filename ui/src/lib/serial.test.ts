import { describe, expect, it } from "vitest";
import { MAX_LINES, serialLines, type SerialLine } from "./serial";

const texts = (lines: SerialLine[]) => lines.map((l) => `${l.kind}:${l.text}`);

describe("serial lines", () => {
  it("splits on line feeds and joins text that arrives in pieces", () => {
    let l = serialLines([], "Hello from ", null);
    l = serialLines(l, "your FPGA board\r\nA", null);
    expect(texts(l)).toEqual(["rx:Hello from your FPGA board", "rx:A"]);
    expect(l[0].done).toBe(true);
    expect(l[1].done).toBe(false);
    l = serialLines(l, "B\n", null);
    expect(texts(l)).toEqual(["rx:Hello from your FPGA board", "rx:AB"]);
  });

  it("drops a lone carriage return, and keeps blank lines", () => {
    const l = serialLines([], "a\rb\n\nc", null);
    expect(texts(l)).toEqual(["rx:ab", "rx:", "rx:c"]);
  });

  it("puts notes on lines of their own", () => {
    let l = serialLines([], "abc", null);
    l = serialLines(l, null, null, "Opened COM5.");
    l = serialLines(l, "d", null);
    expect(texts(l)).toEqual(["rx:abc", "note:Opened COM5.", "rx:d"]);
  });

  it("timestamps each new line", () => {
    const l = serialLines([], "x\ny", new Date(2026, 10, 19, 9, 5, 7, 42));
    expect(l.map((x) => x.time)).toEqual(["09:05:07.042", "09:05:07.042"]);
  });

  it("keeps the newest lines", () => {
    const l = serialLines([], "x\n".repeat(MAX_LINES + 10), null);
    expect(l.length).toBe(MAX_LINES);
  });
});
