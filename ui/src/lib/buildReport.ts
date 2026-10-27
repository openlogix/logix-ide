// Turns Apio's build report into a short status bar summary, e.g. "0.4% of
// logic cells, max clock 194.4 MHz". (The full tables come from
// `apio report` in the console.) Apio names resources per FPGA family; this
// maps the main ones to plain names.
import type { BuildReport } from "./backend";

interface Kind {
  /** Plain name, plural. */
  label: string;
  /** Apio's names for it (their counts are added up). */
  names: string[];
}

/**
 * Per FPGA family: the main capacity measure first (always shown), then
 * other resources (shown when used).
 */
const KINDS: Record<string, Kind[]> = {
  ice40: [
    { label: "logic cells", names: ["ICESTORM_LC"] },
    { label: "RAM blocks", names: ["ICESTORM_RAM"] },
    { label: "multipliers", names: ["ICESTORM_DSP"] },
    { label: "PLLs", names: ["ICESTORM_PLL"] },
    { label: "I/O pins", names: ["SB_IO"] },
  ],
  ecp5: [
    { label: "LUTs", names: ["TRELLIS_COMB"] },
    { label: "flip-flops", names: ["TRELLIS_FF"] },
    { label: "RAM blocks", names: ["DP16KD"] },
    { label: "multipliers", names: ["MULT18X18D"] },
    { label: "PLLs", names: ["EHXPLLL"] },
    { label: "I/O pins", names: ["TRELLIS_IO"] },
  ],
  gowin: [
    { label: "LUTs", names: ["LUT4"] },
    { label: "flip-flops", names: ["DFF"] },
    { label: "RAM blocks", names: ["BSRAM"] },
    { label: "multipliers", names: ["MULT18X18"] },
    { label: "PLLs", names: ["rPLL", "PLLVR"] },
    { label: "I/O pins", names: ["IOB"] },
  ],
  xilinx: [
    { label: "LUTs", names: ["SLICE_LUTX"] },
    { label: "flip-flops", names: ["SLICE_FFX"] },
    { label: "RAM blocks", names: ["RAMB36E1", "RAMB18E1"] },
    { label: "multipliers", names: ["DSP48E1"] },
    { label: "PLLs", names: ["PLLE2_ADV", "MMCME2_ADV"] },
    { label: "I/O pins", names: ["PAD"] },
  ],
};

export interface Usage {
  label: string;
  used: number;
  available: number;
}

/** The resources worth showing: the main one always, others if used. */
export function usages(report: BuildReport): Usage[] {
  const byName = new Map(report.resources.map((r) => [r.name, r]));
  const kinds = KINDS[report.arch];
  if (!kinds) {
    // Unknown family: Apio's own names, for what's used.
    return report.resources
      .filter((r) => r.used > 0)
      .map((r) => ({ label: r.name, used: r.used, available: r.available }));
  }
  const result: Usage[] = [];
  kinds.forEach((kind, i) => {
    let used = 0;
    let available = 0;
    let found = false;
    for (const name of kind.names) {
      const r = byName.get(name);
      if (!r) continue;
      found = true;
      used += r.used;
      available += r.available;
    }
    if (found && (i === 0 || used > 0)) {
      result.push({ label: kind.label, used, available });
    }
  });
  return result;
}

export function percent(used: number, available: number): string {
  if (available <= 0) return "?%";
  const p = (100 * used) / available;
  if (p > 0 && p < 1) return `${p.toFixed(1)}%`;
  return `${Math.round(p)}%`;
}

function mhz(value: number): string {
  return `${value.toFixed(1)} MHz`;
}

/** A short summary for the status bar, e.g. "0.4% of logic cells, max clock 194.4 MHz". */
export function reportSummary(report: BuildReport): string {
  const parts: string[] = [];
  const main = usages(report)[0];
  if (main) parts.push(`${percent(main.used, main.available)} of ${main.label}`);
  if (report.clocks.length > 0) {
    const slowest = Math.min(...report.clocks.map((c) => c.fmaxMhz));
    parts.push(`max clock ${mhz(slowest)}`);
  }
  return parts.join(", ");
}
