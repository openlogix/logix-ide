import { describe, expect, it } from "vitest";
import type { ProjectDoc } from "./backend";
import {
  fileRole,
  roleTooltip,
  addFile,
  copySource,
  deleteFile,
  filesForSave,
  fromDoc,
  isDirty,
  isHdlSource,
  moduleAt,
  isFileDirty,
  mainFile,
  markSaved,
  renameFile,
  starterText,
  layoutOf,
  testbenchOf,
  changedFiles,
  applyDiskChanges,
} from "./workingProject";

const savedDoc: ProjectDoc = {
  dir: "/p/blinky",
  templateDir: null,
  name: "blinky",
  board: null,
  variant: {},
  serialBaud: null,
  files: [
    { name: "main.v", text: "m" },
    { name: "pinout.pcf", text: "p" },
  ],
};

describe("workingProject", () => {
  it("has no special case for board files, which aren't tabs", () => {
    expect(fileRole("mine.json", "main.v")).toBe("design");
    expect(starterText("mine.json")).toBe("");
  });

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

  it("names a testbench's layout and a design file's testbench", () => {
    expect(layoutOf("adder_tb.v")).toBe("adder_tb.view");
    expect(layoutOf("adder_tb.sv")).toBe("adder_tb.view");
    expect(layoutOf("adder.v")).toBeNull();
    expect(testbenchOf("adder.v")).toBe("adder_tb.v");
    expect(testbenchOf("adder.sv")).toBe("adder_tb.sv");
    expect(testbenchOf("adder_tb.v")).toBeNull();
    expect(testbenchOf("defs.vh")).toBeNull();
    expect(testbenchOf("pinout.pcf")).toBeNull();
  });

  it("renames and deletes a testbench's layout with it", () => {
    const p = fromDoc({
      ...savedDoc,
      files: [...savedDoc.files, { name: "main_tb.v", text: "t" }, { name: "main_tb.view", text: "g" }],
    });
    renameFile(p, "main_tb.v", "top_tb.v");
    expect(p.files.map((f) => f.name)).toContain("top_tb.view");
    expect(p.removed).toEqual(["main_tb.v", "main_tb.view"]);
    deleteFile(p, "top_tb.v");
    expect(p.files.map((f) => f.name)).toEqual(["main.v", "pinout.pcf"]);
  });

  it("leaves another file's layout alone when the name is taken", () => {
    const p = fromDoc({
      ...savedDoc,
      files: [
        ...savedDoc.files,
        { name: "a_tb.v", text: "" },
        { name: "a_tb.view", text: "a" },
        { name: "b_tb.view", text: "b" },
      ],
    });
    renameFile(p, "a_tb.v", "b_tb.v");
    const names = p.files.map((f) => f.name);
    expect(names).toContain("a_tb.view");
    expect(p.files.find((f) => f.name === "b_tb.view")?.text).toBe("b");
  });

  it("saves only the files that changed, and keeps changes made while saving", () => {
    const p = fromDoc(savedDoc);
    expect(changedFiles(p)).toEqual([]);
    p.files[0].text = "m2";
    addFile(p, "a.v", "a");
    renameFile(p, "pinout.pcf", "pins.pcf");
    const files = changedFiles(p);
    expect(files.map((f) => f.name)).toEqual(["main.v", "pins.pcf", "a.v"]);
    const removed = [...p.removed];
    // While the save runs: an edit and a deletion.
    p.files[0].text = "m3";
    deleteFile(p, "pins.pcf");
    markSaved(p, "/p/blinky", "blinky", files, null, removed);
    expect(isFileDirty(p, p.files[0])).toBe(true);
    expect(p.removed).toEqual(["pins.pcf"]);
    expect(isDirty(p)).toBe(true);
    expect(changedFiles(p).map((f) => f.name)).toEqual(["main.v"]);
  });

  it("takes changes made on disk by other programs, without asking", () => {
    const p = fromDoc({
      ...savedDoc,
      files: [...savedDoc.files, { name: "a.v", text: "a" }, { name: "b.v", text: "b" }],
    });
    p.files[3].text = "b edited"; // Unsaved edits in b.v.
    addFile(p, "new.v", "n"); // Not saved yet.
    const changes = applyDiskChanges(p, [
      { name: "a.v", text: "a by the agent" }, // Changed: reloaded.
      { name: "b.v", text: "b by the agent" }, // Changed over unsaved edits: the disk wins.
      { name: "pinout.pcf", text: "p" }, // As last saved (Logix IDE's own save): nothing.
      { name: "c.v", text: "c" }, // New on disk: added.
      { name: "new.v", text: null }, // Not saved yet: stays.
      { name: "main.v", text: null }, // The main file can't go: saved again.
    ]);
    expect(changes.reloaded).toEqual([
      { name: "a.v", text: "a by the agent", hadEdits: false },
      { name: "b.v", text: "b by the agent", hadEdits: true },
    ]);
    expect(changes.added).toEqual([{ name: "c.v", text: "c" }]);
    expect(changes.removed).toEqual([]);
    expect(p.files.map((f) => f.name)).toEqual(["main.v", "pinout.pcf", "a.v", "b.v", "new.v", "c.v"]);
    expect(isFileDirty(p, p.files[2])).toBe(false);
    expect(isFileDirty(p, p.files[0])).toBe(true); // main.v is written again.
    // A file deleted on disk goes; one deleted in Logix IDE, not saved yet, stays deleted.
    deleteFile(p, "c.v");
    const gone = applyDiskChanges(p, [
      { name: "a.v", text: null },
      { name: "c.v", text: "c again" },
    ]);
    expect(gone.removed).toEqual(["a.v"]);
    expect(gone.added).toEqual([]);
    expect(p.files.map((f) => f.name)).not.toContain("a.v");
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

  it("doesn't treat a variant change as unsaved", () => {
    const p = fromDoc({ ...savedDoc, board: "icesugar-pro" });
    p.variant = { memory: "ram" };
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


describe("file roles", () => {
  it("tells what each file is for", () => {
    expect(fileRole("main.v", "main.v")).toBe("main");
    expect(fileRole("main.sv", "main.sv")).toBe("main");
    expect(fileRole("uart.v", "main.v")).toBe("design");
    expect(fileRole("defs.vh", "main.v")).toBe("include");
    expect(fileRole("main_tb.v", "main.v")).toBe("testbench");
    expect(fileRole("uart_tb.sv", "main.v")).toBe("testbench");
    expect(fileRole("pinout.pcf", "main.v")).toBe("pins");
    expect(fileRole("Pins.LPF", "main.v")).toBe("pins");
  });
  it("marks only the board's kind of pin file", () => {
    expect(fileRole("pinout.pcf", "main.v", "pcf")).toBe("pins");
    expect(fileRole("Pinout.PCF", "main.v", "pcf")).toBe("pins");
    expect(fileRole("pinout.lpf", "main.v", "pcf")).toBe("otherPins");
    expect(fileRole("pinout.lpf", "main.v", "lpf")).toBe("pins");
    expect(roleTooltip("otherPins", "pcf")).toBe(
      "Pin file for another kind of board: not used with this board, whose pin file is a .pcf file. Double-click to rename; right-click for more.",
    );
  });
  it("explains the roles in the tooltips", () => {
    expect(roleTooltip("main")).toBe("Main file: the design's top module, named main.");
    expect(roleTooltip("testbench")).toMatch(/^Testbench: .* Double-click to rename/);
  });
});

describe("moduleAt", () => {
  const text = "// top\nmodule main(input a);\nendmodule\n\nmodule alu #(parameter W = 8) (input b);\nendmodule\n";
  it("is the module the cursor is in", () => {
    expect(moduleAt(text, text.indexOf("input a"))).toBe("main");
    expect(moduleAt(text, text.indexOf("input b"))).toBe("alu");
    expect(moduleAt(text, text.length)).toBe("alu");
  });
  it("is the first module before any, and none without", () => {
    expect(moduleAt(text, 0)).toBe("main");
    expect(moduleAt("// nothing", 3)).toBeNull();
  });
});
