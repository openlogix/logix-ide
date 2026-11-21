// Scope lines: a faint vertical line through each block of a Verilog file,
// under the first letter of the line that opens the block (the user's
// design, 2026-11-21). Drawn from the indentation alone, so a syntax error
// never moves them, and unindented code has none until Auto Format.
//
// A block is the run of lines indented deeper than the line just above it,
// its opener. Only openers get a line: continuation lines indented further
// (such as a long `if` condition) don't add one at their own column, as a
// line per indentation level would (the user's report, 2026-11-21).
import { RangeSetBuilder, type EditorState, type Extension } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
} from "@codemirror/view";

/**
 * The columns of each line's scope lines, from each line's indentation
 * (`null` for a blank line or a directive, see indentOf). A block's line
 * runs through blank lines up to the line that closes it, such as `end`,
 * `default:` or `endmodule` (the user's report, 2026-11-21: it stopped at
 * the block's last code line).
 */
export function scopeColumns(indents: (number | null)[]): number[][] {
  // The open blocks' columns, increasing; `opened` once a deeper line
  // followed, so the line is a block's opener and not just a sibling.
  const openers: { col: number; opened: boolean }[] = [];
  return indents.map((indent, i) => {
    if (indent === null) {
      const next = indents.slice(i + 1).find((n) => n !== null) ?? 0;
      return openers
        .filter((o) => o.col < next || (o.opened && o.col <= next))
        .map((o) => o.col);
    }
    while (openers.length && openers[openers.length - 1].col >= indent) openers.pop();
    const columns = openers.map((o) => o.col);
    if (openers.length) openers[openers.length - 1].opened = true;
    // Every line may open a block: it does if the next line is deeper.
    openers.push({ col: indent, opened: false });
    return columns;
  });
}

/**
 * Compiler directives such as `ifdef, `else, `endif and `define, usually
 * written at column 0 wherever they are. They're skipped like blank lines,
 * so a module's line runs through an `ifdef inside it (the user's report,
 * 2026-11-21).
 */
const DIRECTIVE =
  /^`(ifdef|ifndef|elsif|else|endif|define|undef|undefineall|include|timescale|default_nettype|resetall|celldefine|endcelldefine|line|pragma|begin_keywords|end_keywords|unconnected_drive|nounconnected_drive)\b/;

/**
 * A line's indentation in columns (tabs by the tab size), or null for a
 * line that doesn't count: blank, or a compiler directive.
 */
export function indentOf(text: string, tabSize = 4): number | null {
  const first = text.search(/\S/);
  if (first < 0 || DIRECTIVE.test(text.slice(first))) return null;
  let col = 0;
  for (const ch of text.slice(0, first)) col = ch === "\t" ? col + tabSize - (col % tabSize) : col + 1;
  return col;
}

function indents(state: EditorState): (number | null)[] {
  const result: (number | null)[] = [];
  for (let n = 1; n <= state.doc.lines; n++) result.push(indentOf(state.doc.line(n).text, state.tabSize));
  return result;
}

const lineDecos = new Map<string, Decoration>();
function lineDeco(columns: number[]): Decoration {
  const key = columns.join(",");
  let deco = lineDecos.get(key);
  if (!deco) {
    // One 1 px line per column, 1 px right of the column's start, so it sits
    // under the first letter of the opener.
    const layers = columns.map(
      (c) => `linear-gradient(var(--scope-line), var(--scope-line)) calc(${c}ch + 1px) 0 / 1px 100% no-repeat`,
    );
    deco = Decoration.line({
      class: "cm-scope-lines",
      attributes: { style: `--scope-lines: ${layers.join(", ")}` },
    });
    lineDecos.set(key, deco);
  }
  return deco;
}

class ScopeLines {
  columns: number[][];
  decorations: DecorationSet;
  constructor(view: EditorView) {
    this.columns = scopeColumns(indents(view.state));
    this.decorations = this.build(view);
  }
  update(update: ViewUpdate) {
    if (update.docChanged) this.columns = scopeColumns(indents(update.state));
    if (update.docChanged || update.viewportChanged) this.decorations = this.build(update.view);
  }
  /** Decorations for the lines on screen only. */
  build(view: EditorView): DecorationSet {
    const builder = new RangeSetBuilder<Decoration>();
    for (const { from, to } of view.visibleRanges) {
      for (let pos = from; pos <= to; ) {
        const line = view.state.doc.lineAt(pos);
        const columns = this.columns[line.number - 1];
        if (columns?.length) builder.add(line.from, line.from, lineDeco(columns));
        pos = line.to + 1;
      }
    }
    return builder.finish();
  }
}

/** The scope lines extension, for Verilog documents. */
export const scopeLines: Extension = [
  ViewPlugin.fromClass(ScopeLines, { decorations: (p) => p.decorations }),
  EditorView.baseTheme({
    ".cm-line.cm-scope-lines": { position: "relative" },
    // Drawn behind the text from where it starts (CodeMirror's lines have a
    // 6 px left padding), so the line's own background, such as the active
    // line's, stays.
    ".cm-scope-lines::before": {
      content: '""',
      position: "absolute",
      top: 0,
      bottom: 0,
      left: "6px",
      right: 0,
      background: "var(--scope-lines)",
      pointerEvents: "none",
      zIndex: "-1",
    },
  }),
];
