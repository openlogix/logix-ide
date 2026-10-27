import type { Diagnostic, Task, TaskOutcome } from "./backend";
import { reportSummary } from "./buildReport";

const VERB: Record<Task, { running: string; done: string; noun: string }> = {
  build: { running: "Building…", done: "Build succeeded", noun: "Build" },
  upload: { running: "Uploading…", done: "Upload succeeded", noun: "Upload" },
  test: { running: "Running tests…", done: "Tests passed", noun: "Test" },
  simulate: {
    running: "Simulating…",
    done: "Simulation done",
    noun: "Simulation",
  },
  graph: {
    running: "Drawing the diagram…",
    done: "Diagram ready",
    noun: "Diagram",
  },
};

export function runningText(task: Task): string {
  return VERB[task].running;
}

/** Where a diagnostic points, e.g. "main.v:12", or "" if nowhere. */
export function location(d: Diagnostic): string {
  if (!d.file) return "";
  return d.line ? `${d.file}:${d.line}` : d.file;
}

/** The status line after a build or upload. */
export function outcomeText(task: Task, outcome: TaskOutcome): string {
  const errors = outcome.diagnostics.filter((d) => d.severity === "error");
  const warnings = outcome.diagnostics.length - errors.length;
  const time = `${outcome.seconds.toFixed(1)} s`;
  if (outcome.cancelled) return `${VERB[task].noun} stopped.`;
  if (outcome.ok) {
    const w = warnings > 0 ? `, ${plural(warnings, "warning")}` : "";
    if (task === "test") {
      const n = plural(outcome.testbenches.length, "testbench", "testbenches");
      return `${VERB.test.done} (${n}) in ${time}${w}.`;
    }
    if (task === "simulate") {
      return `${VERB.simulate.done} in ${time}. GTKWave shows the signals.`;
    }
    if (task === "graph") return `${VERB.graph.done} in ${time}.`;
    const summary = outcome.report ? reportSummary(outcome.report) : "";
    return `${VERB[task].done} in ${time}${w}.${summary ? ` Uses ${summary}.` : ""}`;
  }
  const first = errors[0];
  if (!first) return `${VERB[task].noun} failed. See the output below.`;
  const where = location(first);
  return `${VERB[task].noun} failed: ${where ? `${where}: ` : ""}${friendly(first.message)}`;
}

/** The first error (or else warning) that points into a file. */
export function firstLocated(diags: Diagnostic[]): Diagnostic | null {
  const located = diags.filter((d) => d.file && d.line);
  return (
    located.find((d) => d.severity === "error") ?? located[0] ?? null
  );
}

/** Diagnostics grouped by file (ones without a file are left out). */
export function byFile(diags: Diagnostic[]): Map<string, Diagnostic[]> {
  const map = new Map<string, Diagnostic[]>();
  for (const d of diags) {
    if (!d.file) continue;
    const list = map.get(d.file) ?? [];
    list.push(d);
    map.set(d.file, list);
  }
  return map;
}

/** Plain explanations for Apio messages beginners may not understand. */
export function friendly(message: string): string {
  if (/^No matching USB device/i.test(message)) {
    return (
      "Logix couldn't find the board on USB. Check that it's connected, " +
      "then try again. Tools > Connected Devices shows what's connected."
    );
  }
  if (/^No constraint file/i.test(message)) {
    return (
      "The project has no pin file (e.g. pinout.pcf) for the board. " +
      "Start a new project with the board selected, or open one of the board's examples."
    );
  }
  return message;
}

/** Console lines listing each testbench's result after a test run. */
export function testbenchLines(outcome: TaskOutcome): string[] {
  return outcome.testbenches.map(
    (t) => `${t.passed ? "✓" : "✗"} ${t.name} ${t.passed ? "passed" : "failed"}`,
  );
}

function plural(n: number, word: string, many = `${word}s`): string {
  return `${n} ${n === 1 ? word : many}`;
}
