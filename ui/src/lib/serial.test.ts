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
    // The line feed starts the next line at once, empty.
    expect(texts(l)).toEqual(["rx:Hello from your FPGA board", "rx:AB", "rx:"]);
  });

  it("replaces the line with what follows a carriage return, and keeps blank lines", () => {
    const l = serialLines([], "abc\rX\n\nc", null);
    expect(texts(l)).toEqual(["rx:X", "rx:", "rx:c"]);
    // Opened mid-line: the end of a line, then whole lines, leave nothing behind.
    let mid = serialLines([], "0x0020006\r", null);
    mid = serialLines(mid, "count: 0x0021\r", null);
    expect(texts(mid)).toEqual(["rx:count: 0x0021"]);
    // Typed one key at a time: a, Enter (a line feed), b.
    let k = serialLines([], "a", null);
    k = serialLines(k, "\n", null);
    expect(texts(k)).toEqual(["rx:a", "rx:"]);
    k = serialLines(k, "b", null);
    expect(texts(k)).toEqual(["rx:a", "rx:b"]);
  });

  it("updates a value in place, line after line ending in a carriage return", () => {
    let l = serialLines([], "count:0x0001\r", new Date(2026, 10, 20, 9, 0, 0, 0));
    l = serialLines(l, "count:0x0002\r", new Date(2026, 10, 20, 9, 0, 1, 0));
    expect(texts(l)).toEqual(["rx:count:0x0002"]);
    // The time is the latest write's.
    expect(l[0].time).toBe("09:00:01.000");
    // A carriage return and a line feed just end the line.
    l = serialLines(l, "\r\nnext", null);
    expect(texts(l)).toEqual(["rx:count:0x0002", "rx:next"]);
  });

  it("puts notes on lines of their own", () => {
    let l = serialLines([], "abc", null);
    l = serialLines(l, null, null, "Opened COM5.");
    l = serialLines(l, "d", null);
    expect(texts(l)).toEqual(["rx:abc", "note:Opened COM5.", "rx:d"]);
    // After a line feed, a note goes before the empty line it started.
    let m = serialLines([], "x\n", null);
    m = serialLines(m, null, null, "Paused.");
    expect(texts(m)).toEqual(["rx:x", "note:Paused.", "rx:"]);
    // A note gets its time too, to line up with the board's lines.
    const n = serialLines([], null, new Date(2026, 10, 20, 9, 5, 7, 42), "Opened COM5.");
    expect(n[0].time).toBe("09:05:07.042");
  });

  it("timestamps each new line", () => {
    const l = serialLines([], "x\ny", new Date(2026, 10, 19, 9, 5, 7, 42));
    expect(l.map((x) => x.time)).toEqual(["09:05:07.042", "09:05:07.042"]);
    // An empty line gets its time with its first text.
    let e = serialLines([], "x\n", new Date(2026, 10, 19, 9, 5, 7, 42));
    expect(e[1].time).toBeNull();
    e = serialLines(e, "y", new Date(2026, 10, 19, 9, 5, 8, 0));
    expect(e[1].time).toBe("09:05:08.000");
  });

  it("keeps the newest lines", () => {
    const l = serialLines([], "x\n".repeat(MAX_LINES + 10), null);
    expect(l.length).toBe(MAX_LINES);
  });
});
