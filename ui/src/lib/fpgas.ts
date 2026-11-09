import type { Board, FpgaFamily } from "./backend";

/** One FPGA in Tools > Supported FPGAs, with the boards that have it. */
export interface FpgaRow {
  id: string;
  partNumber: string;
  size: string;
  /** The boards offered (the catalog's and the project's) with this FPGA, by name. */
  boards: string[];
}

/** A family in Tools > Supported FPGAs. */
export interface FpgaGroup {
  id: string;
  label: string;
  /** Its FPGAs that match the search (all without one). */
  fpgas: FpgaRow[];
  /** How many FPGAs the family has, matching or not. */
  fpgaCount: number;
  /** How many of the boards offered have one of the family's FPGAs. */
  boardCount: number;
}

/**
 * The families and their FPGAs, with the boards that have each. With a
 * search query, only the FPGAs whose part number or ID, or whose family's
 * name, contains it; families without a match are left out.
 */
export function fpgaGroups(families: FpgaFamily[], boards: Board[], query: string): FpgaGroup[] {
  const byFpga = new Map<string, string[]>();
  for (const b of boards) {
    if (!b.fpga) continue;
    byFpga.set(b.fpga.id, [...(byFpga.get(b.fpga.id) ?? []), b.description]);
  }
  const q = query.trim().toLowerCase();
  const has = (t: string) => t.toLowerCase().includes(q);
  return families
    .map((f) => {
      const all = f.fpgas.map((x) => ({ ...x, boards: (byFpga.get(x.id) ?? []).sort((a, b) => a.localeCompare(b)) }));
      const fpgas = q === "" || has(f.label) ? all : all.filter((x) => has(x.partNumber) || has(x.id));
      return {
        id: f.id,
        label: f.label,
        fpgas,
        fpgaCount: all.length,
        boardCount: all.reduce((n, x) => n + x.boards.length, 0),
      };
    })
    .filter((g) => q === "" || g.fpgas.length > 0);
}

/** "1 FPGA", "48 FPGAs", "0 boards" and so on. */
export function count(n: number, what: string): string {
  return `${n} ${what}${n === 1 ? "" : "s"}`;
}
