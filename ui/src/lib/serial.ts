// The Serial Monitor's lines (SerialMonitor.svelte): what arrives, split
// into lines as a terminal shows them, and notes (such as "Opened COM5").

/** One line of the Serial Monitor. */
export interface SerialLine {
  /** "rx": text from the board; "note": a message from Logix IDE or Logix Engine. */
  kind: "rx" | "note";
  text: string;
  /** When the line began (hh:mm:ss.mmm), with timestamps on; else null. */
  time: string | null;
  /** Whether the line has ended (a line feed came); the next text starts a new one. */
  done: boolean;
}

/** The most lines kept; older ones are dropped. */
export const MAX_LINES = 5000;

function clock(d: Date): string {
  const pad = (n: number, w = 2) => String(n).padStart(w, "0");
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}.${pad(d.getMilliseconds(), 3)}`;
}

/**
 * Adds `text` from the board (its line feeds end lines; a carriage return
 * without a line feed is dropped, as it would only move a terminal's
 * cursor), or a `note` on a line of its own. `now` timestamps each new
 * line (null: no timestamps). Returns the new list.
 */
export function serialLines(
  lines: SerialLine[],
  text: string | null,
  now: Date | null,
  note?: string,
): SerialLine[] {
  const out = lines.slice();
  if (note !== undefined) {
    out.push({ kind: "note", text: note, time: null, done: true });
  } else if (text) {
    const parts = text.replace(/\r\n/g, "\n").replace(/\r/g, "").split("\n");
    parts.forEach((part, i) => {
      const last = out[out.length - 1];
      if (last && last.kind === "rx" && !last.done) {
        out[out.length - 1] = { ...last, text: last.text + part };
      } else if (part !== "" || i < parts.length - 1) {
        out.push({ kind: "rx", text: part, time: now ? clock(now) : null, done: false });
      }
      // Every part but the last ended with a line feed.
      if (i < parts.length - 1) out[out.length - 1] = { ...out[out.length - 1], done: true };
    });
  }
  return out.length > MAX_LINES ? out.slice(out.length - MAX_LINES) : out;
}
