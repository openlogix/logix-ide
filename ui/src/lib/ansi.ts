// Converts terminal color codes (ANSI escape sequences) in a line of tool
// output into styled text segments for the console. Other escape sequences
// (cursor movement, line erasing, links) are dropped.

export interface Segment {
  text: string;
  /** Inline CSS for the segment, or "" for plain text. */
  style: string;
  /** The place in a project file the line names (Logix Engine's `location`), for the console to jump to. */
  link?: { file: string; line: number };
}

/** The 16 basic colors: theme tokens (app.css, `--ansi-0` to `--ansi-15`),
 * so each reads well on the console in the light and the dark theme. */
const BASIC = Array.from({ length: 16 }, (_, i) => `var(--ansi-${i})`);

/** One of the 256 extended colors. */
function color256(n: number): string {
  if (n < 16) return BASIC[n];
  if (n < 232) {
    const i = n - 16;
    const level = (v: number) => (v === 0 ? 0 : 55 + v * 40);
    const r = level(Math.floor(i / 36));
    const g = level(Math.floor(i / 6) % 6);
    const b = level(i % 6);
    return `rgb(${r},${g},${b})`;
  }
  const gray = 8 + (n - 232) * 10;
  return `rgb(${gray},${gray},${gray})`;
}

interface State {
  fg: string | null;
  bg: string | null;
  bold: boolean;
  dim: boolean;
  italic: boolean;
  underline: boolean;
}

const plain = (): State => ({
  fg: null,
  bg: null,
  bold: false,
  dim: false,
  italic: false,
  underline: false,
});

function css(s: State): string {
  const parts: string[] = [];
  if (s.fg) parts.push(`color:${s.fg}`);
  if (s.bg) parts.push(`background-color:${s.bg}`);
  if (s.bold) parts.push("font-weight:bold");
  if (s.dim) parts.push("opacity:0.7");
  if (s.italic) parts.push("font-style:italic");
  if (s.underline) parts.push("text-decoration:underline");
  return parts.join(";");
}

/** Applies the parameters of one "set graphics" (SGR) sequence. */
function applySgr(state: State, params: number[]) {
  if (params.length === 0) params = [0];
  for (let i = 0; i < params.length; i++) {
    const p = params[i];
    if (p === 0) Object.assign(state, plain());
    else if (p === 1) state.bold = true;
    else if (p === 2) state.dim = true;
    else if (p === 3) state.italic = true;
    else if (p === 4) state.underline = true;
    else if (p === 22) state.bold = state.dim = false;
    else if (p === 23) state.italic = false;
    else if (p === 24) state.underline = false;
    else if (p >= 30 && p <= 37) state.fg = BASIC[p - 30];
    else if (p >= 90 && p <= 97) state.fg = BASIC[p - 90 + 8];
    else if (p === 39) state.fg = null;
    else if (p >= 40 && p <= 47) state.bg = BASIC[p - 40];
    else if (p >= 100 && p <= 107) state.bg = BASIC[p - 100 + 8];
    else if (p === 49) state.bg = null;
    else if (p === 38 || p === 48) {
      // Extended color: 5;n (256 colors) or 2;r;g;b (true color).
      let value: string | null = null;
      if (params[i + 1] === 5 && params[i + 2] !== undefined) {
        value = color256(params[i + 2]);
        i += 2;
      } else if (params[i + 1] === 2 && params[i + 4] !== undefined) {
        value = `rgb(${params[i + 2]},${params[i + 3]},${params[i + 4]})`;
        i += 4;
      }
      if (value) {
        if (p === 38) state.fg = value;
        else state.bg = value;
      }
    }
  }
}

// CSI sequences (ESC [ ... final), OSC sequences (ESC ] ... BEL or ESC \),
// and two-character escapes.
const ESCAPE =
  /\x1b(?:\[([0-?]*)[ -/]*([@-~])|\][^\x07\x1b]*(?:\x07|\x1b\\)|[@-Z\\-_])/g;

/** Splits a line into styled segments. */
export function parseAnsi(line: string): Segment[] {
  const segments: Segment[] = [];
  const state = plain();
  const push = (text: string) => {
    if (!text) return;
    const style = css(state);
    const last = segments[segments.length - 1];
    if (last && last.style === style) last.text += text;
    else segments.push({ text, style });
  };
  let pos = 0;
  for (const m of line.matchAll(ESCAPE)) {
    push(line.slice(pos, m.index));
    pos = m.index! + m[0].length;
    if (m[2] === "m") {
      const params = (m[1] ?? "")
        .split(";")
        .filter((x) => x !== "")
        .map(Number);
      applySgr(state, params);
    }
  }
  push(line.slice(pos));
  return segments;
}

/**
 * Makes the place a tool's line names, as Logix Engine gives it (the
 * `location` of a `logix/output` line), a link: the "file:line" text in the
 * line, or else the whole line. Logix IDE never reads paths out of the
 * text itself; lines without a location get no link.
 */
export function addLink(segments: Segment[], location?: { file: string; line: number }): Segment[] {
  if (!location) return segments;
  const link = { file: location.file, line: location.line };
  const text = segments.map((s) => s.text).join("");
  const at = text.indexOf(`${location.file}:${location.line}`);
  const [start, end] = at < 0 ? [0, text.length] : [at, at + `${location.file}:${location.line}`.length];
  const result: Segment[] = [];
  let pos = 0;
  for (const seg of segments) {
    const from = pos;
    const to = pos + seg.text.length;
    pos = to;
    // The parts of this segment before, inside and after the link's span.
    const cuts = [from, Math.min(Math.max(start, from), to), Math.min(Math.max(end, from), to), to];
    for (let k = 0; k < 3; k++) {
      const part = seg.text.slice(cuts[k] - from, cuts[k + 1] - from);
      if (part) result.push(k === 1 ? { ...seg, text: part, link } : { ...seg, text: part });
    }
  }
  return result;
}
