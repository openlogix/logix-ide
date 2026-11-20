// The Serial Monitor's lines (SerialMonitor.svelte): what arrives, as a
// terminal shows it (a line feed starts a new line; after a carriage return
// the line is overwritten in place, and what's left of the old one is
// erased when the new one ends), and notes (such as "Opened COM5").

/** One line of the Serial Monitor. */
export interface SerialLine {
  /** "rx": text from the board; "note": a message from Logix IDE or Logix Engine. */
  kind: "rx" | "note";
  text: string;
  /** When the line was last written from its start (hh:mm:ss.mmm); null before any text. Shown with Timestamps on. */
  time: string | null;
  /** Whether the line has ended (a line feed came); the next text starts a new one. */
  done: boolean;
  /** Where the next character goes on the open line: its end, or from 0
   * after a carriage return, over the old text. */
  column: number;
  /** Whether characters were written since the last carriage return: then
   * the next carriage return or line feed erases what's past them. */
  written: boolean;
}

/** The most lines kept; older ones are dropped. */
export const MAX_LINES = 5000;

function clock(d: Date): string {
  const pad = (n: number, w = 2) => String(n).padStart(w, "0");
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}.${pad(d.getMilliseconds(), 3)}`;
}

/**
 * Adds `text` from the board, or a `note` on a line of its own, as a
 * terminal would show it:
 *   - a line feed ends the line and starts the next at once, empty, as a
 *     terminal moves its cursor down (the user's report, 2026-11-19);
 *   - after a carriage return, characters overwrite the line from its
 *     start, as in a terminal, so a value updates in place without the line
 *     going blank, however slowly they come (the user's request,
 *     2026-11-20);
 *   - when that new line ends (the next carriage return or line feed),
 *     whatever is past its last character is erased, so a shorter line
 *     leaves nothing behind (a strict terminal would show "count: 0x0021006";
 *     the user's report, 2026-11-20). With nothing written in between, as in
 *     a carriage return and a line feed, nothing is erased.
 * All by the characters' order, never their timing.
 * `now` timestamps a line when it's written from its start (null: no
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
    // A note gets its time too, so it lines up with the text around it.
    const entry: SerialLine = { kind: "note", text: note, time: now ? clock(now) : null, done: true, column: 0, written: false };
    // Before the empty line a line feed started, which stays last.
    const last = out[out.length - 1];
    if (last && last.kind === "rx" && !last.done && last.text === "") out.splice(out.length - 1, 0, entry);
    else out.push(entry);
  } else if (text) {
    // The open line: the last one, if it's text from the board still open.
    let line: SerialLine | null = null;
    const last = out[out.length - 1];
    if (last && last.kind === "rx" && !last.done) line = { ...last };
    else line = null;
    const close = () => {
      if (line) out[out.length - 1] = line;
    };
    // The end of a line written over an old one: erase what's past it.
    const trim = (l: SerialLine) => {
      if (l.written) l.text = [...l.text].slice(0, l.column).join("");
      l.written = false;
    };
    for (const c of text) {
      if (c === "\n") {
        if (!line) out.push((line = { kind: "rx", text: "", time: null, done: false, column: 0, written: false }));
        trim(line);
        line.done = true;
        close();
        out.push((line = { kind: "rx", text: "", time: null, done: false, column: 0, written: false }));
        continue;
      }
      if (!line) out.push((line = { kind: "rx", text: "", time: null, done: false, column: 0, written: false }));
      if (c === "\r") {
        trim(line);
        line.column = 0;
        continue;
      }
      // A character: written at the column, over what's there.
      if (line.column === 0 && now) line.time = clock(now);
      const chars = [...line.text];
      chars[line.column] = c;
      line.text = chars.join("");
      line.column += 1;
      line.written = true;
    }
    close();
  }
  return out.length > MAX_LINES ? out.slice(out.length - MAX_LINES) : out;
}
