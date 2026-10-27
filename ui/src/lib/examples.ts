import type { Board, Example } from "./backend";

/**
 * Every board's first entry, as Arduino's File > Examples > 01.Basics >
 * Bare Minimum: the board's pins and a design that does nothing yet.
 */
export const BARE_MINIMUM = "Bare Minimum";

/** An entry in the Examples dialog: one of Apio's examples, or Bare Minimum. */
export interface ExampleEntry extends Example {
  bare?: boolean;
}

export interface BoardGroup {
  board: Board;
  /** Bare Minimum (if it matches the search) first, then Apio's examples. */
  examples: ExampleEntry[];
  /** How many of Apio's examples the board has. */
  count: number;
}

/**
 * Examples a board's pins are taken from, for new projects and
 * Bare Minimum. Keep in sync with `STARTER_EXAMPLES` in `catalog.rs`.
 */
const STARTER_EXAMPLES = ["blinky", "ledon"];

function bareMinimum(board: string, examples: Example[]): ExampleEntry {
  const hasPins = examples.some((e) => STARTER_EXAMPLES.includes(e.name));
  return {
    board,
    name: BARE_MINIMUM,
    description: hasPins
      ? "The board's pins and an empty design, to start from scratch"
      : "An empty design, without pins (no example to take them from)",
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
      .map((id) => ({ id, description: id, arch: "" })),
  );

  const groups: BoardGroup[] = [];
  for (const board of allBoards) {
    const list = byBoard.get(board.id) ?? [];
    const all = [bareMinimum(board.id, list), ...list];
    const boardMatches =
      !q ||
      board.description.toLowerCase().includes(q) ||
      board.id.toLowerCase().includes(q);
    const shown = boardMatches
      ? all
      : all.filter(
          (e) =>
            e.name.toLowerCase().includes(q) ||
            e.description.toLowerCase().includes(q),
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
    ? `${base} (${hidden} hidden: not supported by Logix yet).`
    : `${base}.`;
}
