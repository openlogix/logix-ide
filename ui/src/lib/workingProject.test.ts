import { describe, expect, it } from "vitest";
import type { ProjectDoc } from "./backend";
import {
  addFile,
  copySource,
  deleteFile,
  filesForSave,
  fromDoc,
  isDirty,
  isHdlSource,
  isUntouchedStarter,
  projectModules,
  isFileDirty,
  mainFile,
  markSaved,
  renameFile,
  starterText,
} from "./workingProject";

const savedDoc: ProjectDoc = {
  dir: "/p/blinky",
  templateDir: null,
  starter: false,
  name: "blinky",
  board: null,
  files: [
    { name: "main.v", text: "m" },
    { name: "pinout.pcf", text: "p" },
  ],
};

describe("workingProject", () => {
  it("starts clean", () => {
    const p = fromDoc(savedDoc);
    expect(isDirty(p)).toBe(false);
    expect(p.files.every((f) => !isFileDirty(p, f))).toBe(true);
  });

  it("tracks text edits per file", () => {
    const p = fromDoc(savedDoc);
    p.files[1].text = "changed";
    expect(isDirty(p)).toBe(true);
    expect(isFileDirty(p, p.files[0])).toBe(false);
    expect(isFileDirty(p, p.files[1])).toBe(true);
  });

  it("records renames and deletes of files on disk", () => {
    const p = fromDoc(savedDoc);
    addFile(p, "a.v", "");
    renameFile(p, "pinout.pcf", "pins.pcf");
    deleteFile(p, "a.v");
    expect(p.removed).toEqual(["pinout.pcf"]);
    expect(p.files.map((f) => f.name)).toEqual(["main.v", "pins.pcf"]);
    expect(isDirty(p)).toBe(true);
  });

  it("never renames or deletes main.v", () => {
    const p = fromDoc(savedDoc);
    renameFile(p, "main.v", "x.v");
    deleteFile(p, "main.v");
    expect(p.files[0].name).toBe("main.v");
    expect(isDirty(p)).toBe(false);
  });

  it("protects main.sv like main.v", () => {
    const p = fromDoc({
      ...savedDoc,
      files: [
        { name: "main.sv", text: "m" },
        { name: "util.sv", text: "u" },
      ],
    });
    expect(mainFile(p)).toBe("main.sv");
    deleteFile(p, "main.sv");
    renameFile(p, "main.sv", "x.sv");
    expect(p.files.map((f) => f.name)).toEqual(["main.sv", "util.sv"]);
  });

  it("is clean again after saving", () => {
    const p = fromDoc(savedDoc);
    addFile(p, "b.v", "x");
    deleteFile(p, "pinout.pcf");
    expect(filesForSave(p)).toEqual([
      { name: "main.v", text: "m" },
      { name: "b.v", text: "x" },
    ]);
    markSaved(p, "/p/copy", "copy", filesForSave(p), p.board);
    expect(isDirty(p)).toBe(false);
    expect(p.removed).toEqual([]);
    expect(p.dir).toBe("/p/copy");
  });

  it("keeps edits made during a save as unsaved", () => {
    const p = fromDoc(savedDoc);
    p.files[0].text = "v1";
    const snapshot = filesForSave(p);
    p.files[0].text = "v2";
    markSaved(p, "/p/blinky", "blinky", snapshot, p.board);
    expect(isDirty(p)).toBe(true);
  });

  it("treats a board change as unsaved", () => {
    const p = fromDoc(savedDoc);
    p.board = "alhambra-ii";
    expect(isDirty(p)).toBe(true);
    markSaved(p, "/p/blinky", "blinky", filesForSave(p), "alhambra-ii");
    expect(isDirty(p)).toBe(false);
  });

  it("copies an example's files on the first save only", () => {
    const p = fromDoc({ ...savedDoc, dir: null, templateDir: "/tmp/ex" });
    expect(isDirty(p)).toBe(false);
    expect(copySource(p)).toBe("/tmp/ex");
    markSaved(p, "/p/blinky", "blinky", filesForSave(p), p.board);
    expect(copySource(p)).toBe("/p/blinky");
  });

  it("doesn't mark files of a never saved project as unsaved", () => {
    const p = fromDoc({ ...savedDoc, dir: null });
    expect(p.files.some((f) => isFileDirty(p, f))).toBe(false);
  });
});

describe("starterText", () => {
  it("makes a module for Verilog files", () => {
    expect(starterText("counter.v")).toBe("module counter (\n);\n\nendmodule\n");
    expect(starterText("alu.sv")).toContain("module alu");
  });
  it("makes a testbench skeleton for *_tb files", () => {
    const text = starterText("counter_tb.v");
    expect(text).toContain("module counter_tb;");
    expect(text).toContain("$dumpvars(0, counter_tb);");
    expect(text).toContain("$finish;");
  });
  it("leaves other files empty", () => {
    expect(starterText("pinout.pcf")).toBe("");
    expect(starterText("defs.vh")).toBe("");
    expect(starterText("2bad.v")).toBe("");
  });
});

describe("isHdlSource", () => {
  it("accepts Verilog and SystemVerilog files only", () => {
    for (const n of ["main.v", "alu.SV", "defs.vh", "pkg.svh"]) {
      expect(isHdlSource(n)).toBe(true);
    }
    for (const n of ["pinout.pcf", "notes.txt", "v"]) {
      expect(isHdlSource(n)).toBe(false);
    }
  });
});

describe("isUntouchedStarter", () => {
  const starterDoc = { ...savedDoc, dir: null, starter: true };
  it("is true for a new, unchanged project", () => {
    expect(isUntouchedStarter(fromDoc(starterDoc))).toBe(true);
  });
  it("is false once edited, saved, or for other projects", () => {
    const edited = fromDoc(starterDoc);
    edited.files[0].text = "x";
    expect(isUntouchedStarter(edited)).toBe(false);
    const saved = fromDoc(starterDoc);
    markSaved(saved, "/p/x", "x", filesForSave(saved), null);
    expect(isUntouchedStarter(saved)).toBe(false);
    expect(isUntouchedStarter(fromDoc(savedDoc))).toBe(false);
  });
});

describe("projectModules", () => {
  it("lists design modules, main first, without testbenches", () => {
    const p = fromDoc({
      ...savedDoc,
      files: [
        { name: "main.v", text: "module main (\n);\n  counter c();\nendmodule\n" },
        { name: "util.v", text: "// module fake\nmodule counter;\nendmodule\nmodule alu;\nendmodule\n" },
        { name: "main_tb.v", text: "module main_tb;\nendmodule\n" },
        { name: "pinout.pcf", text: "module nope" },
      ],
    });
    expect(projectModules(p)).toEqual(["main", "alu", "counter"]);
  });
});
