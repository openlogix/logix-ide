// The problems shown in the editor and on the tabs. Plain code, no Svelte.
//
// Two kinds, kept apart:
// - live problems: what a check that runs while typing found (Verible for
//   Verilog files). Always current:
//   each check replaces its file's list.
// - command problems: what the last command found (Verify, Upload, Run
//   Tests, Simulate, Show Diagram, Auto Format). They stay until the file
//   is edited, or a later command that reads the file reports again: Logix
//   Engine says which files each command read (`checked_files`), such as
//   the pin file only for builds.
// The editor and the tabs show both together.
import type { Diagnostic } from "./backend";

/** Problems by file, from the checks that run while typing. */
export type LiveProblems = Map<string, Diagnostic[]>;

/**
 * The command problems after a command's result: `found` replaces the
 * earlier problems of the files the command read (`checked`, Logix Engine's
 * `checked_files`); the other files keep theirs. Problems without a file
 * always go.
 */
export function afterCommand(
  earlier: Diagnostic[],
  found: Diagnostic[],
  checked: string[],
): Diagnostic[] {
  const kept = earlier.filter((d) => d.file !== null && !checked.includes(d.file));
  return [...kept, ...found];
}

/** The command problems after `file` was edited: its own are dropped. */
export function afterEdit(earlier: Diagnostic[], file: string): Diagnostic[] {
  return earlier.some((d) => d.file === file)
    ? earlier.filter((d) => d.file !== file)
    : earlier;
}

/** Live and command problems together, for the editor. */
export function allProblems(command: Diagnostic[], live: LiveProblems): Diagnostic[] {
  return [...command, ...[...live.values()].flat()];
}

/** Each file's mark for its tab: "error" if it has one, else "warning". */
export function tabMarks(diags: Diagnostic[]): Map<string, "error" | "warning"> {
  const marks = new Map<string, "error" | "warning">();
  for (const d of diags) {
    if (!d.file) continue;
    if (d.severity === "error") marks.set(d.file, "error");
    else if (!marks.has(d.file)) marks.set(d.file, "warning");
  }
  return marks;
}


/** A file's problems in words, for its tab's tooltip, e.g. "2 errors, 1 warning". */
export function problemCount(diags: Diagnostic[], file: string): string {
  const mine = diags.filter((d) => d.file === file);
  const errors = mine.filter((d) => d.severity === "error").length;
  const warnings = mine.length - errors;
  const part = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;
  return [errors > 0 ? part(errors, "error") : "", warnings > 0 ? part(warnings, "warning") : ""]
    .filter(Boolean)
    .join(", ");
}
