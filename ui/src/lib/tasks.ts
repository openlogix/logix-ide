import type { Diagnostic, Task, TaskOutcome } from "./backend";
import { reportSummary } from "./buildReport";
import { CUSTOM_BOARDS_FILE } from "./workingProject";

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
  // The output above already lists the file's problems at its full path.
  // No count: the engine may stop before finding them all.
  // Named `custom-boards.json`, or by its full path in a saved project.
  const file = first.file ?? "";
  if (file === CUSTOM_BOARDS_FILE || file.endsWith(`/${CUSTOM_BOARDS_FILE}`)) {
    return `${VERB[task].noun} failed: the project's ${CUSTOM_BOARDS_FILE} has problems.`;
  }
  // A located error is the tool's own line, already in the output just
  // above: repeated here it read twice (the user's choice, 2026-11-07).
  // One without a location (such as "No Alhambra II is connected. …")
  // shows only here.
  const located = Boolean(first.file && first.line);
  // A failing testbench is still simulated: its waveform shows the problem.
  if (task === "simulate" && outcome.testbenches.length > 0) {
    return located
      ? "The testbench failed. GTKWave shows the signals."
      : `The testbench failed (GTKWave shows the signals): ${first.message}`;
  }
  return located
    ? `${VERB[task].noun} failed.`
    : `${VERB[task].noun} failed: ${first.message}`;
}

/**
 * The status line after a successful simulation whose GTKWave window
 * didn't appear in time (a slow start): it may still open, behind Logix.
 */
export function gtkwaveSlowText(outcome: TaskOutcome): string {
  return `${VERB.simulate.done} in ${outcome.seconds.toFixed(1)} s. GTKWave is taking a while to start; it may open behind this window.`;
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

/** Console lines listing each testbench's result after a test run. */
export function testbenchLines(outcome: TaskOutcome): string[] {
  return outcome.testbenches.map(
    (t) => `${t.passed ? "✓" : "✗"} ${t.name} ${t.passed ? "passed" : "failed"}`,
  );
}

function plural(n: number, word: string, many = `${word}s`): string {
  return `${n} ${n === 1 ? word : many}`;
}
