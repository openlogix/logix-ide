import { beforeAll, describe, expect, it } from "vitest";
import { setFileFormats } from "./formats";
import { ENGINE_FORMATS } from "./testFormats";
import {
  fileGroups,
  matches,
  renamed,
  RECENT_COUNT,
  shortcutLabel,
  visit,
} from "./fileList";

// Which files are pin files, as Logix Engine says (formats.ts).
beforeAll(() => setFileFormats(ENGINE_FORMATS));

const FILES = [
  "main.v",
  "adder.v",
  "adder_tb.v",
  "counter.v",
  "counter_tb.v",
  "main_tb.v",
  "pinout.pcf",
];

describe("visit", () => {
  it("keeps the files switched to newest first, each once", () => {
    let h: string[] = [];
    for (const f of ["main.v", "adder.v", "counter.v", "adder.v"])
      h = visit(h, f);
    expect(h).toEqual(["adder.v", "counter.v", "main.v"]);
  });
  it("keeps one more than the list shows", () => {
    let h: string[] = [];
    for (let i = 0; i < 20; i++) h = visit(h, `f${i}.v`);
    expect(h).toHaveLength(RECENT_COUNT + 1);
    expect(h[0]).toBe("f19.v");
  });
});

describe("renamed", () => {
  it("follows a rename and drops a deleted file", () => {
    expect(renamed(["a.v", "b.v"], "a.v", "c.v")).toEqual(["c.v", "b.v"]);
    expect(renamed(["a.v", "b.v"], "a.v", null)).toEqual(["b.v"]);
  });
});

describe("matches", () => {
  it("finds a part of the name, any case", () => {
    expect(matches("Clock_Divider.v", "clock_d")).toBe(true);
    expect(matches("adder.v", " TB ")).toBe(false);
    expect(matches("adder_tb.v", "tb")).toBe(true);
  });
});

describe("fileGroups", () => {
  it("lists every file by kind when nothing was switched to", () => {
    expect(fileGroups(FILES, [], "main.v", "main.v", "pcf", "")).toEqual([
      { title: "Design", files: ["main.v", "adder.v", "counter.v"] },
      {
        title: "Testbenches",
        files: ["adder_tb.v", "counter_tb.v", "main_tb.v"],
      },
      { title: "Pin files", files: ["pinout.pcf"] },
    ]);
  });
  it("puts the recent files first, without the one shown, each once", () => {
    const h = ["counter_tb.v", "adder.v", "main.v"];
    const groups = fileGroups(FILES, h, "counter_tb.v", "main.v", "pcf", "");
    expect(groups[0]).toEqual({
      title: "Recent",
      files: ["adder.v", "main.v"],
    });
    expect(groups[1]).toEqual({ title: "Design", files: ["counter.v"] });
    expect(groups.flatMap((g) => g.files).sort()).toEqual([...FILES].sort());
  });
  it("narrows every group to the files matching the query", () => {
    const groups = fileGroups(
      FILES,
      ["adder.v", "main.v"],
      "main.v",
      "main.v",
      "pcf",
      "add",
    );
    expect(groups).toEqual([
      { title: "Recent", files: ["adder.v"] },
      { title: "Testbenches", files: ["adder_tb.v"] },
    ]);
  });
  it("leaves out files that are gone", () => {
    const groups = fileGroups(
      FILES,
      ["old.v", "adder.v"],
      "main.v",
      "main.v",
      "pcf",
      "",
    );
    expect(groups[0]).toEqual({ title: "Recent", files: ["adder.v"] });
  });
});

describe("shortcutLabel", () => {
  it("names the key as the menu does", () => {
    expect(shortcutLabel("MacIntel")).toBe("⌘P");
    expect(shortcutLabel("Win32")).toBe("Ctrl+P");
    expect(shortcutLabel("Linux x86_64")).toBe("Ctrl+P");
  });
});
