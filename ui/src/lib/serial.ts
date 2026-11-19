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
 * Adds `text` from the board, or a `note` on a line of its own. A line feed
 * ends the line and starts the next at once, empty, as a terminal moves its
 * cursor down (so after Enter the new line shows before anything is typed
 * on it; the user's report, 2026-11-19). A carriage return without a line
 * feed is dropped, as it would only move a terminal's cursor to the line's
 * start. `now` timestamps each line when its first text arrives (null: no
 * timestamps). Returns the new list.
 */
export function serialLines(
  lines: SerialLine[],
  text: string | null,
  now: Date | null,
  note?: string,
): SerialLine[] {
  const out = lines.slice();
  if (note !== undefined) {
    const entry: SerialLine = { kind: "note", text: note, time: null, done: true };
    // Before the empty line a line feed started, which stays last.
    const last = out[out.length - 1];
    if (last && last.kind === "rx" && !last.done && last.text === "") out.splice(out.length - 1, 0, entry);
    else out.push(entry);
  } else if (text) {
    const parts = text.replace(/\r\n/g, "\n").replace(/\r/g, "").split("\n");
    parts.forEach((part, i) => {
      const last = out[out.length - 1];
      if (last && last.kind === "rx" && !last.done) {
        const time = last.time ?? (part !== "" && now ? clock(now) : null);
        out[out.length - 1] = { ...last, text: last.text + part, time };
      } else if (part !== "" || i < parts.length - 1) {
        out.push({ kind: "rx", text: part, time: part !== "" && now ? clock(now) : null, done: false });
      }
      // A line feed: the line ends, and the next starts, empty.
      if (i < parts.length - 1) {
        out[out.length - 1] = { ...out[out.length - 1], done: true };
        out.push({ kind: "rx", text: "", time: null, done: false });
      }
    });
  }
  return out.length > MAX_LINES ? out.slice(out.length - MAX_LINES) : out;
}
