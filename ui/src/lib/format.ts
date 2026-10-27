/** Formats a byte count for people, e.g. "18.6 MB". */
export function formatBytes(bytes: number): string {
  if (bytes < 1000) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let value = bytes;
  let unit = "B";
  for (const u of units) {
    if (value < 1000) break;
    value /= 1000;
    unit = u;
  }
  return `${value.toFixed(1)} ${unit}`;
}

/** Download progress as a 0..1 fraction, or null if the total is unknown. */
export function progressFraction(
  received: number,
  total: number | null,
): number | null {
  if (!total || total <= 0) return null;
  return Math.min(1, Math.max(0, received / total));
}

/** Appends lines, keeping at most `max` of the newest ones. */
export function appendCapped<T>(list: T[], items: T[], max: number): T[] {
  const next = list.concat(items);
  return next.length > max ? next.slice(next.length - max) : next;
}
