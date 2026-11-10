// Turns the build report into a short status bar summary, e.g. "0.4% of
// logic cells, max clock 194.4 MHz", and console lines, as Arduino's
// "Sketch uses ...". Logix Engine names the resources in plain words
// (`resources_summary`), so the IDE keeps no FPGA family names of its own.
import type { BuildReport } from "./backend";

export interface Usage {
  label: string;
  used: number;
  available: number;
}

/**
 * The resources worth showing, from the engine's summary: its main one
 * always, others if used.
 */
export function usages(report: BuildReport): Usage[] {
  return report.summary
    .filter((r, i) => i === 0 || r.used > 0)
    .map((r) => ({ label: r.name, used: r.used, available: r.available }));
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

/**
 * Console lines after a build, e.g. "The design uses 27 of 7680 logic
 * cells (0.4%)." and "Maximum clock speed: 194.4 MHz (sys_clk).".
 */
export function reportLines(report: BuildReport): string[] {
  const lines = usages(report).map(
    (u) =>
      `The design uses ${u.used} of ${u.available} ${u.label} (${percent(u.used, u.available)}).`,
  );
  for (const c of report.clocks) {
    lines.push(`Maximum clock speed: ${mhz(c.fmaxMhz)} (${c.name}).`);
  }
  return lines;
}
