// The All files list (the tab bar's ▾, when the tabs don't fit) and Go to
// File (⌘P / Ctrl+P), one list for both (the user's decision, 2026-11-20):
// the project's files in groups, the ones switched to most recently first,
// narrowed by what's typed. Plain functions, tested; FileList.svelte shows
// them.

import { fileRole, type FileRole } from "./workingProject";

/** How many recent files the list shows. */
export const RECENT_COUNT = 5;
/** How many files a project's history keeps (one more than shown, since
    the file shown now isn't listed). */
const HISTORY = RECENT_COUNT + 1;

/** A group of the list, with its title. */
export type FileGroup = { title: string; files: string[] };

/**
 * The project's history after switching to `file`: the files switched to,
 * newest first, each once.
 */
export function visit(history: string[], file: string): string[] {
  return [file, ...history.filter((f) => f !== file)].slice(0, HISTORY);
}

/** The history after a file's rename (`to`) or deletion (`to` null). */
export function renamed(
  history: string[],
  from: string,
  to: string | null,
): string[] {
  return history.flatMap((f) => (f !== from ? [f] : to === null ? [] : [to]));
}

/** Whether `file` matches what's typed: its name contains it, any case. */
export function matches(file: string, query: string): boolean {
  return file.toLowerCase().includes(query.trim().toLowerCase());
}

const GROUP_OF: Record<FileRole, string> = {
  main: "Design",
  design: "Design",
  include: "Design",
  testbench: "Testbenches",
  pins: "Pin files",
  otherPins: "Pin files",
};
const GROUP_ORDER = ["Design", "Testbenches", "Pin files"];

/**
 * The list's groups: Recent (the files switched to most recently, newest
 * first, without `current`, the one shown), then the others by kind:
 * Design, Testbenches, Pin files, each in `files`' order (the tabs').
 * Each file appears once; empty groups are left out. With a query, only
 * the files whose names contain it.
 */
export function fileGroups(
  files: string[],
  history: string[],
  current: string,
  main: string,
  pinExtension: string | null,
  query: string,
): FileGroup[] {
  const shown = files.filter((f) => matches(f, query));
  const recent = history
    .filter((f) => f !== current && shown.includes(f))
    .slice(0, RECENT_COUNT);
  const groups: FileGroup[] = [{ title: "Recent", files: recent }];
  for (const title of GROUP_ORDER) {
    groups.push({
      title,
      files: shown.filter(
        (f) =>
          !recent.includes(f) &&
          GROUP_OF[fileRole(f, main, pinExtension)] === title,
      ),
    });
  }
  return groups.filter((g) => g.files.length > 0);
}

// ---- Each project's history, kept per user in the web view's storage
// (as the Serial Monitor's line ending): reopening a project brings it back,
// and it isn't in the project file, so sharing a project doesn't share it.

const KEY = "logix-ide.recentFiles";
/** How many projects' histories are kept (the oldest are dropped). */
const PROJECTS = 30;

type Stored = { dir: string; files: string[] }[];

function readAll(): Stored {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) ?? "[]");
    return Array.isArray(v) ? v : [];
  } catch {
    return [];
  }
}

/** The history of the project in folder `dir`. */
export function loadHistory(dir: string): string[] {
  return readAll().find((p) => p.dir === dir)?.files ?? [];
}

/** Keeps the history of the project in folder `dir`. */
export function saveHistory(dir: string, files: string[]) {
  const all = [{ dir, files }, ...readAll().filter((p) => p.dir !== dir)].slice(
    0,
    PROJECTS,
  );
  try {
    localStorage.setItem(KEY, JSON.stringify(all));
  } catch {
    // Storage may be unavailable: the history then lasts for this run.
  }
}

/** The shortcut's name on this computer: ⌘P on macOS, Ctrl+P elsewhere. */
export function shortcutLabel(platform = navigator.platform): string {
  return /^Mac/i.test(platform) ? "⌘P" : "Ctrl+P";
}
