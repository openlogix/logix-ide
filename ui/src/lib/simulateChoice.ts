// Which testbench Simulate runs (the user's decision, 2026-11-22: with
// several testbenches and a design file open, Simulate was greyed out with
// no reason given). Pure functions, unit-tested; App.svelte keeps the
// testbench simulated last and shows the picker.

import { isTestbench, testbenchOf } from "./workingProject";

/** What Simulate does: run this testbench, ask which one, or nothing. */
export type SimulateChoice =
  | { kind: "run"; testbench: string }
  | { kind: "ask"; testbenches: string[] }
  | { kind: "none" };

/**
 * Simulate's testbench, in this order: the open tab when it's a testbench;
 * the open file's own testbench (`main.v`: `main_tb.v`, as renames pair
 * them); the project's only testbench; the one simulated last in this
 * project; else ask which one. `files` are the project's file names.
 */
export function simulateChoice(
  files: string[],
  active: string | null,
  last: string | null,
): SimulateChoice {
  const all = files.filter(isTestbench);
  if (all.length === 0) return { kind: "none" };
  if (active && isTestbench(active) && all.includes(active)) return { kind: "run", testbench: active };
  const own = active ? testbenchOf(active) : null;
  if (own && all.includes(own)) return { kind: "run", testbench: own };
  if (all.length === 1) return { kind: "run", testbench: all[0] };
  if (last && all.includes(last)) return { kind: "run", testbench: last };
  return { kind: "ask", testbenches: all };
}

/** The Simulate button's hover text: what it will do. */
export function simulateTitle(choice: SimulateChoice): string {
  switch (choice.kind) {
    case "run":
      return `Simulate ${choice.testbench}: run it and show its signals`;
    case "ask":
      return "Simulate: asks which testbench to run, then shows its signals";
    case "none":
      return "Simulate: add a testbench (a file ending in _tb.v) to simulate";
  }
}

// ---- The testbench simulated last, per project and per user, in the web
// view's storage (as the file history): it isn't in the project file, so
// sharing a project doesn't share it.

const KEY = "logix-ide.simulated";
/** How many projects' choices are kept (the oldest are dropped). */
const PROJECTS = 30;

type Stored = { dir: string; testbench: string }[];

function readAll(): Stored {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) ?? "[]");
    return Array.isArray(v) ? v : [];
  } catch {
    return [];
  }
}

/** The testbench last simulated in the project in folder `dir`. */
export function loadSimulated(dir: string): string | null {
  return readAll().find((p) => p.dir === dir)?.testbench ?? null;
}

/** Keeps the testbench last simulated in the project in folder `dir`. */
export function saveSimulated(dir: string, testbench: string) {
  const all = [{ dir, testbench }, ...readAll().filter((p) => p.dir !== dir)].slice(0, PROJECTS);
  try {
    localStorage.setItem(KEY, JSON.stringify(all));
  } catch {
    // Storage may be unavailable: the choice then lasts for this run.
  }
}
