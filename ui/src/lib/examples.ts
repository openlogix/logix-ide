import type { Board, Example } from "./backend";

/**
 * Every board's first entry, as Arduino's File > Examples > 01.Basics >
 * BareMinimum: the board's pins and a design that does nothing yet. Named
 * like the examples (the user's decision, 2026-11-07; was "Bare Minimum").
 */
export const BARE_MINIMUM = "bare-minimum";

/** An entry in the Examples dialog: one of the examples, or Bare Minimum. */
export interface ExampleEntry extends Example {
  bare?: boolean;
}

export interface BoardGroup {
  board: Board;
  /** Bare Minimum (if it matches the search) first, then the examples. */
  examples: ExampleEntry[];
  /** How many examples the board has. */
  count: number;
}

/**
 * A board's Bare Minimum entry. Every catalog board has one, generated
 * with its blinky (`src-tauri/src/generate.rs`); the backend opens an
 * empty design for any other board.
 */
function bareMinimum(board: string): ExampleEntry {
  return {
    board,
    name: BARE_MINIMUM,
    summary: "The board's pins and an empty design, to start from scratch",
    notes: [],
    bare: true,
  };
}

/**
 * Groups examples by board (in board order). Every board has Bare Minimum
 * first. With a search query, a board matches by name or id and then
 * shows all its entries; otherwise only its matching entries are shown.
 */
export function groupExamples(
  boards: Board[],
  examples: Example[],
  query: string,
): BoardGroup[] {
  const q = query.trim().toLowerCase();
  const byBoard = new Map<string, Example[]>();
  for (const e of examples) {
    const list = byBoard.get(e.board) ?? [];
    list.push(e);
    byBoard.set(e.board, list);
  }
  // Examples for boards missing from the board list still show up.
  const known = new Set(boards.map((b) => b.id));
  const allBoards = boards.concat(
    [...byBoard.keys()]
      .filter((id) => !known.has(id))
      .map((id) => ({
        id,
        description: id,
        family: "",
        familyLabel: "Other",
        familyOrder: Number.MAX_SAFE_INTEGER,
        pinExtension: null,
        dimensions: [],
        configurations: [],
        custom: false,
        fpga: null,
      })),
  );

  const groups: BoardGroup[] = [];
  for (const board of allBoards) {
    const list = byBoard.get(board.id) ?? [];
    const all: ExampleEntry[] = [bareMinimum(board.id), ...list];
    const boardMatches =
      !q ||
      board.description.toLowerCase().includes(q) ||
      board.id.toLowerCase().includes(q);
    const shown = boardMatches
      ? all
      : all.filter(
          (e) =>
            // "bare minimum" finds bare-minimum too.
            [e.name, e.bare ? "bare minimum" : "", e.summary, ...e.notes].some((t) =>
              t.toLowerCase().includes(q),
            ),
        );
    if (shown.length > 0)
      groups.push({ board, examples: shown, count: list.length });
  }
  return groups;
}

/** Console line about the example list, e.g. "146 examples (18 hidden ...)". */
export function examplesSummary(shown: number, hidden: number): string {
  const base = `${shown} example${shown === 1 ? "" : "s"} available`;
  return hidden > 0
    ? `${base} (${hidden} hidden: not supported by Logix IDE yet).`
    : `${base}.`;
}
