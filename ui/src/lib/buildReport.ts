// The Build Report window's table (`BuildReport.svelte`): the build's
// resources as Logix Engine gives them (the toolchain's names, with a short
// description and a checked share), the used ones by default. The status
// bar and output lines are the engine's own text (`summaryLine`, `text`),
// so the IDE keeps no FPGA knowledge or wording of its own for them.
import type { BuildReport } from "./backend";

export type ReportResource = BuildReport["resources"][number];

/**
 * The rows to show: the used resources, or all with `showUnused`, those
 * matching `filter` (in the name or the description, ignoring case), in the
 * engine's order (by name).
 */
export function reportRows(
  report: BuildReport,
  showUnused: boolean,
  filter: string,
): ReportResource[] {
  const f = filter.trim().toLowerCase();
  return report.resources.filter(
    (r) =>
      (showUnused || r.used > 0) &&
      (f === "" ||
        r.name.toLowerCase().includes(f) ||
        (r.description ?? "").toLowerCase().includes(f)),
  );
}

/** How many resources the design doesn't use (hidden by default). */
export function unusedCount(report: BuildReport): number {
  return report.resources.filter((r) => r.used === 0).length;
}

/**
 * A share for the table: empty for an unused resource (so the used ones
 * stand out), "n/a" when the engine found nextpnr's numbers invalid,
 * "<0.1%" for something used but less than that, one decimal under 1%,
 * whole numbers otherwise.
 */
export function percentText(r: ReportResource): string {
  if (r.used === 0) return "";
  if (r.percent === null) return "n/a";
  if (r.used > 0 && r.percent < 0.1) return "<0.1%";
  if (r.percent > 0 && r.percent < 1) return `${r.percent.toFixed(1)}%`;
  return `${Math.round(r.percent)}%`;
}

export function mhzText(mhz: number): string {
  return `${mhz.toFixed(1)} MHz`;
}

/** Text columns, padded; the columns in `right` aligned right (numbers). */
function columns(rows: string[][], right: number[]): string {
  const widths = rows[0].map((_, i) => Math.max(...rows.map((r) => r[i].length)));
  return rows
    .map((r) =>
      r
        .map((cell, i) => (right.includes(i) ? cell.padStart(widths[i]) : cell.padEnd(widths[i])))
        .join("  ")
        .trimEnd(),
    )
    .join("\n");
}

/**
 * The report as plain text for Copy: a title line, the rows shown, then the
 * clocks, in columns.
 */
export function reportText(title: string, rows: ReportResource[], report: BuildReport): string {
  const resources = columns(
    [
      ["Resource", "Description", "Used", "Available", "%"],
      ...rows.map((r) => [
        r.name,
        r.description ?? "",
        r.used === 0 ? "" : String(r.used),
        String(r.available),
        percentText(r),
      ]),
    ],
    [2, 3, 4],
  );
  const clocks =
    report.clocks.length === 0
      ? ""
      : "\n\n" +
        columns(
          [
            ["Clock", "Max speed", "Net"],
            ...report.clocks.map((c) => [c.name, mhzText(c.fmaxMhz), c.net]),
          ],
          [1],
        );
  return `${title}\n\n${resources}${clocks}\n`;
}
