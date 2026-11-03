import { describe, expect, it } from "vitest";
import { pinCommentToken, pinLanguage } from "./pinFiles";

describe("pin file comments", () => {
  it("uses each format's line comment", () => {
    expect(pinCommentToken("pinout.pcf")).toBe("#");
    expect(pinCommentToken("Pins.LPF")).toBe("#");
    expect(pinCommentToken("arty.xdc")).toBe("#");
    expect(pinCommentToken("tang.cst")).toBe("//");
  });
  it("leaves other files alone", () => {
    expect(pinCommentToken("main.v")).toBeNull();
    expect(pinCommentToken("notes.txt")).toBeNull();
    expect(pinCommentToken("pcf")).toBeNull();
    expect(pinLanguage("main.v")).toBeNull();
  });
  it("shares one language per comment kind", () => {
    expect(pinLanguage("a.pcf")).toBe(pinLanguage("b.lpf"));
    expect(pinLanguage("a.cst")).not.toBe(pinLanguage("a.pcf"));
  });
});
