// Converts terminal color codes (ANSI escape sequences) in a line of tool
// output into styled text segments for the console. Other escape sequences
// (cursor movement, line erasing, links) are dropped.

export interface Segment {
  text: string;
  /** Inline CSS for the segment, or "" for plain text. */
  style: string;
  /** A "file:line" reference the console can jump to. */
  link?: { file: string; line: number };
}

/** The 16 basic colors, chosen to read well on the dark console. */
const BASIC = [
  "#666666", // black (lifted so it's visible on the dark background)
  "#f14c4c", // red
  "#23d18b", // green
  "#e5c07b", // yellow
  "#3b8eea", // blue
  "#d670d6", // magenta
  "#29b8db", // cyan
  "#d4d4d4", // white
  "#8a8a8a", // bright black
  "#ff6b6b", // bright red
  "#5af78e", // bright green
  "#f5f543", // bright yellow
  "#6cb6ff", // bright blue
  "#ff77ff", // bright magenta
  "#5ce1e6", // bright cyan
  "#ffffff", // bright white
];

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
 * "file.v:12" style references to source and pin files: a bare file name
 * (not part of a longer path), a colon, and a line number.
 */
const FILE_LINE =
  /(?<![\w/.\\-])([\w.-]+\.(?:s?vh?|pcf|lpf|cst|xdc|pdc|ccf)):(\d+)/gi;

/** Splits segments further so "file:line" references become links. */
export function addLinks(segments: Segment[]): Segment[] {
  const result: Segment[] = [];
  for (const seg of segments) {
    let pos = 0;
    for (const m of seg.text.matchAll(FILE_LINE)) {
      if (m.index! > pos) {
        result.push({ text: seg.text.slice(pos, m.index), style: seg.style });
      }
      result.push({
        text: m[0],
        style: seg.style,
        link: { file: m[1], line: Number(m[2]) },
      });
      pos = m.index! + m[0].length;
    }
    if (pos < seg.text.length) {
      result.push({ text: seg.text.slice(pos), style: seg.style });
    }
  }
  return result;
}
