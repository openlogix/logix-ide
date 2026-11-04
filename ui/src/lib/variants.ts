// A board's variants: its dimensions (e.g. Connection, Memory) and the
// configurations that exist. Pure functions, unit-tested in variants.test.ts.
//
// A project stores the values the user chose (`[variant]` in
// logix-ide.toml), only when they aren't the board's default. As in the
// engine, the first configuration with those values is used, and values the
// board doesn't have are ignored (so an old project still opens).

import type {
  Board,
  Configuration,
  ConnectedBoard,
  Variant,
  VariantMenu,
} from "./backend";

const matches = (c: Configuration, variant: Variant) =>
  Object.entries(variant).every(([k, v]) => c.when[k] === v);

/** The variant without values the board doesn't have. */
function known(board: Board, variant: Variant): Variant {
  const out: Variant = {};
  for (const d of board.dimensions) {
    const v = variant[d.id];
    if (v !== undefined && d.values.some((x) => x.id === v)) out[d.id] = v;
  }
  return out;
}

/**
 * The configuration a variant picks: the first with its values, or else
 * the board's default. Null only for a board without configurations.
 */
export function configurationOf(board: Board, variant: Variant): Configuration | null {
  const wanted = known(board, variant);
  return (
    board.configurations.find((c) => matches(c, wanted)) ??
    board.configurations[0] ??
    null
  );
}

/** Every dimension's value for the variant (what the engine will use). */
export function resolve(board: Board, variant: Variant): Variant {
  return { ...(configurationOf(board, variant)?.when ?? {}) };
}

/** What to store for a variant: {} for the board's default configuration. */
export function toStore(board: Board, variant: Variant): Variant {
  const c = configurationOf(board, variant);
  return c && c !== board.configurations[0] ? { ...c.when } : {};
}

/** The label of a dimension's value, e.g. "RAM" (the id if unknown). */
export function valueLabel(board: Board, dim: string, value: string): string {
  const d = board.dimensions.find((x) => x.id === dim);
  return d?.values.find((v) => v.id === value)?.label ?? value;
}

/**
 * The toolbar's summary: the value of every dimension, e.g.
 * "FT232H adapter · RAM". Empty for a board without dimensions.
 */
export function summary(board: Board, variant: Variant): string {
  const when = resolve(board, variant);
  return board.dimensions
    .map((d) => valueLabel(board, d.id, when[d.id]))
    .join(" · ");
}

/** Whether `value` exists with the variant's other current values. */
export function isAvailable(
  board: Board,
  variant: Variant,
  dim: string,
  value: string,
): boolean {
  const when = { ...resolve(board, variant), [dim]: value };
  return board.configurations.some((c) => matches(c, when));
}

/**
 * Chooses `value` for `dim`. If it doesn't exist with the other current
 * values, the first configuration that has it is used, and `changed` names
 * the other dimensions that changed, e.g. ["Connection: FT232H adapter"].
 * Returns the variant to store.
 */
export function choose(
  board: Board,
  variant: Variant,
  dim: string,
  value: string,
): { variant: Variant; changed: string[] } {
  const current = resolve(board, variant);
  const wanted = { ...current, [dim]: value };
  const c =
    board.configurations.find((x) => matches(x, wanted)) ??
    board.configurations.find((x) => x.when[dim] === value);
  if (!c) return { variant: toStore(board, variant), changed: [] };
  const changed = board.dimensions
    .filter((d) => d.id !== dim && c.when[d.id] !== current[d.id])
    .map((d) => `${d.label}: ${valueLabel(board, d.id, c.when[d.id])}`);
  return { variant: toStore(board, c.when), changed };
}

/** The Tools submenus for the board's dimensions (none for most boards). */
export function menus(board: Board | null, variant: Variant): VariantMenu[] {
  if (!board) return [];
  const when = resolve(board, variant);
  return board.dimensions.map((d) => ({
    id: d.id,
    label: d.label,
    values: d.values.map((v) => ({
      id: v.id,
      label: v.label,
      checked: when[d.id] === v.id,
      enabled: isAvailable(board, variant, d.id, v.id),
    })),
  }));
}

/** Whether the board is connected the way its variant programs it. */
export function isConnected(
  board: Board | null,
  boardId: string | null,
  variant: Variant,
  connected: ConnectedBoard[],
): boolean {
  if (!boardId) return false;
  const found = connected.find((c) => c.board === boardId);
  if (!found) return false;
  if (!board || board.dimensions.length === 0) return true;
  const when = resolve(board, variant);
  return found.variants.some((v) =>
    Object.entries(when).every(([k, x]) => v[k] === x),
  );
}

/**
 * Parses a Tools submenu id, "variant:<dimension>:<value>". Null for other
 * menu ids.
 */
export function parseMenuId(id: string): { dim: string; value: string } | null {
  const m = /^variant:([^:]+):(.+)$/.exec(id);
  return m ? { dim: m[1], value: m[2] } : null;
}
