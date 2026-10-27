// Which architecture groups are expanded in the board lists (the toolbar's
// board picker and the Examples window share it). Collapsed by default;
// remembered per viewer in localStorage.
import { SvelteSet } from "svelte/reactivity";

const KEY = "logix.expandedGroups";

function load(): string[] {
  try {
    const value = JSON.parse(localStorage.getItem(KEY) ?? "[]");
    return Array.isArray(value) ? value.filter((v) => typeof v === "string") : [];
  } catch {
    return [];
  }
}

function save() {
  try {
    localStorage.setItem(KEY, JSON.stringify([...expanded]));
  } catch {
    // Not essential.
  }
}

/** Labels of the expanded groups, e.g. "Lattice iCE40". */
export const expanded = new SvelteSet<string>(load());

export function toggleGroup(label: string) {
  if (expanded.has(label)) expanded.delete(label);
  else expanded.add(label);
  save();
}

export function expandGroup(label: string) {
  if (expanded.has(label)) return;
  expanded.add(label);
  save();
}
