import { beforeAll, describe, expect, it } from "vitest";
import { pinComment, setFileFormats } from "./formats";
import { pinLanguage } from "./pinFiles";
import { ENGINE_FORMATS } from "./testFormats";

beforeAll(() => setFileFormats(ENGINE_FORMATS));

describe("pin file comments", () => {
  it("uses each format's line comment, as Logix Engine gives it", () => {
    expect(pinComment("pinout.pcf")).toBe("#");
    expect(pinComment("Pins.LPF")).toBe("#");
    expect(pinComment("arty.xdc")).toBe("#");
    expect(pinComment("tang.cst")).toBe("//");
  });
  it("leaves other files alone", () => {
    expect(pinComment("main.v")).toBeNull();
    expect(pinComment("notes.txt")).toBeNull();
    expect(pinComment("pcf")).toBeNull();
    expect(pinLanguage("main.v")).toBeNull();
  });
  it("shares one language per comment kind", () => {
    expect(pinLanguage("a.pcf")).toBe(pinLanguage("b.lpf"));
    expect(pinLanguage("a.cst")).not.toBe(pinLanguage("a.pcf"));
  });
});
